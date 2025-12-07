use std::time::Duration;

use sysinfo::{ProcessExt, System, SystemExt};
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};
use crate::error::discovery::DiscoveryError;

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub pid: u32,
    pub port: u16,
    pub base_url: String,
    pub name: String,
    pub command: String,
}

    #[error("failed to query system processes: {0}")]
    SystemQuery(String),
    #[error("failed to query network sockets: {0}")]
    NetworkQuery(String),

fn find_listening_port(pid: u32) -> Result<Option<u16>, DiscoveryError> {
    let sockets = get_sockets_info(
        AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6,
        ProtocolFlags::TCP,
    ).map_err(|e| DiscoveryError::NetworkQuery(e.to_string()))?;

    for s in sockets {
        if let ProtocolSocketInfo::Tcp(tcp) = s.protocol_socket_info {
            if tcp.state == TcpState::Listen {
                if s.associated_pids.iter().any(|p| *p as u32 == pid) {
                    return Ok(Some(tcp.local_port));
                }
            }
        }
    }
    Ok(None)
}

/// Try to discover a running OpenCode server process and its listening port.
/// Strategy:
/// - Use sysinfo to enumerate processes, look for bun/node with command containing "opencode".
/// - Use netstat2 to resolve LISTENing port for that PID.
/// - Return first valid match with base_url = http://127.0.0.1:{port}.
pub fn discover() -> Result<Option<ServerInfo>, DiscoveryError> {
    let mut sys = System::new_all();
    // Refresh processes and network info (process list is enough here)
    sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true);

    for (pid, p) in sys.processes() {
        let name = p.name().to_string();
        let cmd_vec = p.cmd();
        let command = if cmd_vec.is_empty() {
            String::new()
        } else {
            cmd_vec.join(" ")
        };

        // Heuristic: bun/node running opencode
        let is_candidate = (name.contains("bun") || name.contains("node"))
            && (command.contains("opencode") || name.contains("opencode"));

        if !is_candidate { continue; }

        let pid_u32 = pid.as_u32();
        if let Some(port) = find_listening_port(pid_u32)? {
            let base_url = format!("http://127.0.0.1:{port}");
            return Ok(Some(ServerInfo {
                pid: pid_u32,
                port,
                base_url,
                name,
                command,
            }));
        }
    }

    Ok(None)
}

/// Lightweight readiness check against GET {base_url}/doc.
pub async fn check_health(base_url: &str) -> bool {
    let url = format!("{base_url}/doc");
    let client = reqwest::Client::new();
    match client.get(&url).timeout(Duration::from_secs(3)).send().await {
        Ok(resp) if resp.status().is_success() => true,
        _ => false,
    }
}
