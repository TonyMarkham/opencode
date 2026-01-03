use crate::discovery::get_override_port;
use crate::error::discovery::DiscoveryError;
use crate::{OPENCODE_BINARY, OPENCODE_SERVER_BASE_URL};

use models::error::error_location::ErrorLocation;
use models::{ServerInfo, ServerInfoBuilder};

use std::panic::Location;
use std::thread::sleep;
use std::time::Duration;

use backoff::{ExponentialBackoff, backoff::Backoff};
use log::{debug, trace};
use netstat2::{
    AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, SocketInfo, TcpState, get_sockets_info,
};
use reqwest::Client;
use sysinfo::{Pid, Process, ProcessesToUpdate, Signal, System};

const CHECK_HEALTH_DURATION: Duration = Duration::from_secs(3);
const HEALTH_CHECK_ENDPOINT: &str = "/doc";
const KILL_VERIFY_MAX_ELAPSED: Duration = Duration::from_secs(5);

#[track_caller]
fn query_tcp_sockets() -> Result<Vec<SocketInfo>, DiscoveryError> {
    get_sockets_info(
        AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6,
        ProtocolFlags::TCP,
    )
    .map_err(|e| DiscoveryError::NetworkQuery {
        message: format!("Failed to query network sockets: {e}"),
        location: ErrorLocation::from(Location::caller()),
        source: Box::new(e),
    })
}

#[track_caller]
fn discover_on_port(port: u16) -> Result<Option<ServerInfo>, DiscoveryError> {
    let sockets = query_tcp_sockets()?;

    for s in sockets {
        if let ProtocolSocketInfo::Tcp(tcp) = s.protocol_socket_info
            && tcp.state == TcpState::Listen
            && tcp.local_port == port
            && let Some(&pid) = s.associated_pids.first()
        {
            trace!("Found process {pid} listening on port {port}");

            let data = with_process(pid, |p| {
                (p.name().to_string_lossy().to_string(), format_command(p))
            });

            if let Some((name, command)) = data {
                let base_url = format!("{OPENCODE_SERVER_BASE_URL}:{port}");

                debug!("Discovered server: {name} (PID: {pid})");

                let server_info = ServerInfoBuilder::default()
                    .with_pid(pid)
                    .with_port(port)
                    .with_base_url(base_url)
                    .with_name(OPENCODE_BINARY)
                    .with_command(format!("{OPENCODE_BINARY} {command}"))
                    .with_owned(true)
                    .build()?;

                return Ok(Some(server_info));
            }

            trace!("Process {pid} disappeared before we could read its info");
        }
    }

    debug!("No process found listening on port {port}");
    Ok(None)
}

#[track_caller]
fn find_listening_port(pid: u32) -> Result<Option<u16>, DiscoveryError> {
    let sockets = query_tcp_sockets()?;

    for s in sockets {
        if let ProtocolSocketInfo::Tcp(tcp) = s.protocol_socket_info
            && tcp.state == TcpState::Listen
            && s.associated_pids.contains(&pid)
        {
            return Ok(Some(tcp.local_port));
        }
    }

    Ok(None)
}

#[track_caller]
fn discover_by_process_scan() -> Result<Option<ServerInfo>, DiscoveryError> {
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    trace!("Scanning {} processes", sys.processes().len());

    for (pid, p) in sys.processes() {
        let name = p.name().to_string_lossy().to_string();
        let command = format_command(p);

        let is_candidate =
            (name.contains("bun") || name.contains("node") || name.contains("opencode"))
                && (command.contains("opencode") || name.contains("opencode"));

        if !is_candidate {
            continue;
        }

        trace!("Found candidate process: {name} (PID: {pid})");

        let pid_u32 = pid.as_u32();
        if let Some(port) = find_listening_port(pid_u32)? {
            let base_url = format!("{OPENCODE_SERVER_BASE_URL}:{port}");

            debug!("Discovered server: {name} on port {port} (PID: {pid_u32})");

            let server_info = ServerInfoBuilder::default()
                .with_pid(pid_u32)
                .with_port(port)
                .with_base_url(base_url)
                .with_name(OPENCODE_BINARY)
                .with_command(format!("{OPENCODE_BINARY} {command}"))
                .with_owned(true)
                .build()?;

            return Ok(Some(server_info));
        }
    }

    debug!("No OpenCode server found");
    Ok(None)
}

#[track_caller]
pub(crate) fn with_process<F, R>(pid: u32, f: F) -> Option<R>
where
    F: FnOnce(&Process) -> R,
{
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    sys.process(Pid::from_u32(pid)).map(f)
}

pub(crate) fn format_command(process: &Process) -> String {
    let cmd_vec: Vec<String> = process
        .cmd()
        .iter()
        .map(|s| s.to_string_lossy().to_string())
        .collect();

    if cmd_vec.is_empty() {
        String::new()
    } else {
        cmd_vec.join(" ")
    }
}

/// Discover a running OpenCode server process.
///
/// Attempts to find an OpenCode server by:
/// 1. Checking for a port override (if set, looks for a process on that specific port)
/// 2. Scanning all processes for bun/node/opencode with "opencode" in the command line
/// 3. Mapping the process to its listening port via netstat
///
/// Note: Currently only discovers servers on localhost (127.0.0.1). This is intentional
/// for security - we only connect to servers on the local machine.
///
/// # Returns
///
/// * `Ok(Some(ServerInfo))` - If a server is found
/// * `Ok(None)` - If no server is running
/// * `Err(DiscoveryError)` - If process/network queries fail
#[track_caller]
pub fn discover() -> Result<Option<ServerInfo>, DiscoveryError> {
    debug!("Starting server discovery");

    if let Some(override_port) = get_override_port() {
        debug!("Port override set to {override_port}");
        return discover_on_port(override_port);
    }

    debug!("No port override - scanning for OpenCode processes");
    discover_by_process_scan()
}

/// Stop a server process by PID.
///
/// Attempts graceful termination (SIGTERM) first, falls back to force kill (SIGKILL).
/// Uses exponential backoff to verify the process has terminated, waiting up to 5 seconds.
///
/// # Arguments
///
/// * `pid` - Process ID to terminate
///
/// # Returns
///
/// * `true` - If the process was successfully terminated
/// * `false` - If the process doesn't exist or couldn't be killed
pub fn stop_pid(pid: u32) -> bool {
    let killed = with_process(pid, |p| {
        if let Some(sent) = p.kill_with(Signal::Term) {
            debug!("Sent SIGTERM to PID {pid}: success={sent}");
            sent
        } else {
            let killed = p.kill();
            debug!("Sent SIGKILL to PID {pid}: success={killed}");
            killed
        }
    })
    .unwrap_or_else(|| {
        debug!("Process {pid} not found");
        false
    });

    if !killed {
        return false;
    }

    // Wait with exponential backoff to verify termination
    let mut backoff = ExponentialBackoff {
        max_elapsed_time: Some(KILL_VERIFY_MAX_ELAPSED),
        ..Default::default()
    };

    loop {
        if with_process(pid, |_| true).is_none() {
            debug!("Process {pid} successfully terminated");
            return true;
        }

        match backoff.next_backoff() {
            Some(duration) => {
                trace!("Process {pid} still alive, retrying after {duration:?}");
                sleep(duration);
            }
            None => {
                debug!("Process {pid} still running after max backoff time");
                return false;
            }
        }
    }
}

/// Check if the server is healthy and responding.
///
/// Performs a lightweight GET request to {base_url}/doc with a 3-second timeout.
///
/// # Arguments
///
/// * `base_url` - Base URL of the server (e.g., "http://127.0.0.1:4096")
///
/// # Returns
///
/// * `true` - If server responds with HTTP 2xx
/// * `false` - If request fails or times out
pub async fn check_health(base_url: &str) -> bool {
    let url = format!("{base_url}{HEALTH_CHECK_ENDPOINT}");
    let client = Client::new();

    match client.get(&url).timeout(CHECK_HEALTH_DURATION).send().await {
        Ok(resp) if resp.status().is_success() => {
            debug!("Health check succeeded for {base_url}");
            true
        }
        Ok(resp) => {
            debug!(
                "Health check failed for {base_url}: status={}",
                resp.status()
            );
            false
        }
        Err(e) => {
            debug!("Health check failed for {base_url}: {e}");
            false
        }
    }
}
