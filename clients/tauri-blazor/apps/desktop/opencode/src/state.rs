use models::ServerInfo;

use std::sync::Arc;

use log::{info, warn};
use tokio::sync::{Mutex, RwLock, mpsc};

/// Commands that mutate application state.
///
/// All state mutations go through the state actor via these commands.
/// This ensures serialized access and prevents race conditions.
#[derive(Debug, Clone)]
pub enum StateCommand {
    /// Set the current server info (from discovery or spawn)
    SetServer(ServerInfo),

    /// Clear the current server info (after stop)
    ClearServer,
}

/// Application state manager.
///
/// Uses an actor pattern to ensure all state mutations are serialized.
/// Commands send StateCommand messages which are processed sequentially
/// by a dedicated task.
///
/// Reads are lock-free using Arc<RwLock<T>> for optimal performance.
#[derive(Clone)]
pub struct AppState {
    /// Channel to send state mutation commands to the actor
    command_tx: Arc<Mutex<Option<mpsc::Sender<StateCommand>>>>,

    /// Shared read-only access to server info
    server: Arc<RwLock<Option<ServerInfo>>>,

    /// Track if actor has been initialized (NEW)
    actor_init: Arc<Mutex<bool>>,
}

impl AppState {
    /// Create a new state manager.
    ///
    /// The actor will be lazily spawned on first use within an async context.
    pub fn new() -> Self {
        Self {
            command_tx: Arc::new(Mutex::new(None)),
            server: Arc::new(RwLock::new(None)),
            actor_init: Arc::new(Mutex::new(false)),
        }
    }

    /// Send a state update command.
    ///
    /// Returns an error if the state actor has died (should never happen).
    pub async fn update(&self, cmd: StateCommand) -> Result<(), String> {
        self.ensure_actor().await; // ADD THIS LINE

        let tx_guard = self.command_tx.lock().await; // CHANGED: now needs lock
        let tx = tx_guard.as_ref().ok_or("Actor not initialized")?; // CHANGED
        tx.send(cmd)
            .await
            .map_err(|e| format!("State actor died: {}", e))
    }

    /// Get current server info (read-only).
    ///
    /// This is a lock-free read using RwLock, so it's fast and won't
    /// block on state mutations.
    pub async fn get_server(&self) -> Option<ServerInfo> {
        self.server.read().await.clone()
    }

    /// Ensure actor is spawned (called lazily from async context)
    async fn ensure_actor(&self) {
        let mut init_guard = self.actor_init.lock().await;
        if !*init_guard {
            let (tx, rx) = mpsc::channel(100);
            let server_clone = Arc::clone(&self.server);

            // Store tx BEFORE spawning to avoid race
            let mut tx_guard = self.command_tx.lock().await;
            *tx_guard = Some(tx);
            drop(tx_guard); // Release before spawn

            tokio::spawn(state_actor(rx, server_clone));
            *init_guard = true;
            info!("State actor spawned");
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// The state actor task.
///
/// Owns the mutable state and processes commands sequentially.
/// This ensures that all state mutations are serialized and
/// prevents race conditions between concurrent operations.
async fn state_actor(
    mut command_rx: mpsc::Receiver<StateCommand>,
    server: Arc<RwLock<Option<ServerInfo>>>,
) {
    info!("State actor started");

    while let Some(cmd) = command_rx.recv().await {
        match cmd {
            StateCommand::SetServer(new_server) => {
                let mut server_write = server.write().await;

                if let Some(ref existing) = *server_write {
                    warn!(
                        "Replacing existing server (PID {}, port {}) with new server (PID {}, port {})",
                        existing.pid, existing.port, new_server.pid, new_server.port
                    );
                } else {
                    info!(
                        "Setting server state: PID={}, port={}, owned={}",
                        new_server.pid, new_server.port, new_server.owned
                    );
                }

                *server_write = Some(new_server);
            }
            StateCommand::ClearServer => {
                let mut server_write = server.write().await;

                if let Some(ref old_server) = *server_write {
                    info!("Clearing server state: PID={}", old_server.pid);
                } else {
                    warn!("Clear server requested but no server was set");
                }

                *server_write = None;
            }
        }
    }

    warn!("State actor stopped - this should not happen during normal operation");
}
