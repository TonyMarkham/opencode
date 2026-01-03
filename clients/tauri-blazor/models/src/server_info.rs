use serde::{Deserialize, Serialize};

/// Information about a discovered or spawned OpenCode server.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerInfo {
    /// Process ID of the server
    pub pid: u32,
    /// Port the server is listening on
    pub port: u16,
    /// Base URL for API requests (e.g., "http://127.0.0.1:4096")
    pub base_url: String,
    /// Process name (e.g., "bun", "node", "opencode")
    pub name: String,
    /// Full command line that started the process
    pub command: String,
    /// Whether this server was spawned by this client instance
    pub owned: bool,
}
