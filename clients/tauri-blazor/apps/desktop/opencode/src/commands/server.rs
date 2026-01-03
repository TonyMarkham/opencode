use crate::error::OpencodeError;
use crate::state::{AppState, StateCommand};

use models::{ErrorLocation, ServerInfo};

use client_core::discovery::{process, spawn};

use std::panic::Location;

use log::{debug, error, info, warn};
use tauri::{State, command as TauriCommand};

/// Discover a running OpenCode server on localhost.
///
/// Attempts to find a running server by scanning processes and network ports.
/// If found, updates the app state with the server info.
///
/// # Returns
///
/// * `Ok(Some(ServerInfo))` - Server found and connected
/// * `Ok(None)` - No server running
/// * `Err(OpencodeError)` - Discovery failed (network error, permission denied, etc.)
#[TauriCommand]
pub async fn discover_server(
    state: State<'_, AppState>,
) -> Result<Option<ServerInfo>, OpencodeError> {
    debug!("Starting server discovery");

    let result = process::discover().map_err(|e| {
        error!("Server discovery failed: {}", e);
        OpencodeError::Core {
            message: e.to_string(),
            location: ErrorLocation::from(Location::caller()),
        }
    })?;

    if let Some(ref server_info) = result {
        info!(
            "Discovered server: PID={}, port={}, owned={}",
            server_info.pid, server_info.port, server_info.owned
        );

        state
            .update(StateCommand::SetServer(server_info.clone()))
            .await
            .map_err(|e| {
                error!("Failed to update state during discovery: {}", e);
                OpencodeError::Opencode {
                    message: e,
                    location: ErrorLocation::from(Location::caller()),
                }
            })?;

        debug!("Server info stored in state");
    } else {
        debug!("No server found");
    }

    Ok(result)
}

/// Spawn a new OpenCode server and wait for it to become healthy.
///
/// Starts `opencode serve` process, parses its output to find the listening port,
/// then polls the health endpoint until the server is ready.
///
/// Updates app state with the spawned server info.
///
/// # Returns
///
/// * `Ok(ServerInfo)` - Server spawned successfully and is healthy
/// * `Err(OpencodeError)` - Failed to spawn, parse output, or server didn't become healthy
#[TauriCommand]
pub async fn spawn_server(state: State<'_, AppState>) -> Result<ServerInfo, OpencodeError> {
    info!("Spawning new OpenCode server");

    let server_info = spawn::spawn_and_wait().await.map_err(|e| {
        error!("Failed to spawn server: {}", e);
        OpencodeError::Core {
            message: e.to_string(),
            location: ErrorLocation::from(Location::caller()),
        }
    })?;

    info!(
        "Server spawned successfully: PID={}, port={}, base_url={}",
        server_info.pid, server_info.port, server_info.base_url
    );

    state
        .update(StateCommand::SetServer(server_info.clone()))
        .await
        .map_err(|e| {
            error!("Failed to update state during spawn: {}", e);
            OpencodeError::Opencode {
                message: e,
                location: ErrorLocation::from(Location::caller()),
            }
        })?;

    debug!("Spawned server info stored in state");

    Ok(server_info)
}

/// Check if the currently connected server is healthy.
///
/// Performs an HTTP GET request to the server's health endpoint.
/// Requires a server to be connected (via discover or spawn).
///
/// # Returns
///
/// * `Ok(true)` - Server is healthy and responding
/// * `Ok(false)` - Server is not responding or returned error
/// * `Err(OpencodeError)` - No server is connected
#[TauriCommand]
pub async fn check_health(state: State<'_, AppState>) -> Result<bool, OpencodeError> {
    debug!("Checking server health");

    let server_info = state.get_server().await.ok_or_else(|| {
        warn!("Health check requested but no server connected");
        OpencodeError::NoServer {
            message: String::from(
                "No server connected - call discover_server or spawn_server first",
            ),
            location: ErrorLocation::from(Location::caller()),
        }
    })?;

    debug!("Checking health of server at {}", server_info.base_url);
    let healthy = process::check_health(&server_info.base_url).await;

    if healthy {
        debug!("Server is healthy");
    } else {
        warn!(
            "Server health check failed: PID={}, base_url={}",
            server_info.pid, server_info.base_url
        );
    }

    Ok(healthy)
}

/// Stop the currently connected server.
///
/// Sends SIGTERM (graceful) or SIGKILL (force) to the server process.
/// Clears the app state after stopping.
///
/// # Returns
///
/// * `Ok(())` - Server stopped successfully
/// * `Err(OpencodeError)` - No server connected or failed to stop
#[TauriCommand]
pub async fn stop_server(state: State<'_, AppState>) -> Result<(), OpencodeError> {
    debug!("Attempting to stop server");

    let server_info = state.get_server().await.ok_or_else(|| {
        warn!("Stop requested but no server connected");
        OpencodeError::NoServer {
            message: String::from("No server connected - nothing to stop"),
            location: ErrorLocation::from(Location::caller()),
        }
    })?;

    let pid = server_info.pid;

    info!("Stopping server with PID {}", pid);
    let stopped = process::stop_pid(pid);

    if !stopped {
        error!("Failed to stop server with PID {}", pid);
        return Err(OpencodeError::StopFailed {
            message: format!("Failed to stop server with PID {pid}"),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    info!("Server stopped successfully: PID {}", pid);

    state.update(StateCommand::ClearServer).await.map_err(|e| {
        error!("Failed to update state during cleanup: {}", e);
        OpencodeError::Opencode {
            message: e,
            location: ErrorLocation::from(Location::caller()),
        }
    })?;

    debug!("Server state cleared");

    Ok(())
}
