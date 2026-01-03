use crate::discovery::{get_override_port, process::check_health};
use crate::error::spawn::SpawnError;
use crate::{OPENCODE_BINARY, OPENCODE_SERVER_BASE_URL, OPENCODE_SERVER_HOSTNAME};

use models::ServerInfo;
use models::{ErrorLocation, ServerInfoBuilder};

use std::env::current_exe;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::mem::forget;
use std::panic::Location;
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use backoff::{ExponentialBackoff, backoff::Backoff};
use log::{debug, info, trace, warn};
use regex::Regex;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Child as TokioChild;
use tokio::process::Command as TokioCommand;
use tokio::spawn as TokioSpawn;
use tokio::time::sleep as TokioSleep;

const SERVE_COMMAND: &str = "serve";
const PORT_FLAG: &str = "--port";
const HOSTNAME_FLAG: &str = "--hostname";
const AUTO_SELECT_PORT: &str = "0";
const SPAWN_MAX_OUTPUT_LINES: usize = 100;
const HEALTH_CHECK_MAX_ELAPSED: Duration = Duration::from_secs(20);
const SERVER_URL_PATTERN: &str = r"http://(?P<host>[^\s:]+):(?P<port>\d+)";
const URL_CAPTURE_HOST: &str = "host";
const URL_CAPTURE_PORT: &str = "port";

static URL_REGEX: OnceLock<Regex> = OnceLock::new();

pub(crate) fn get_url_regex() -> &'static Regex {
    URL_REGEX.get_or_init(|| Regex::new(SERVER_URL_PATTERN).expect("valid regex pattern"))
}

pub(crate) fn build_spawn_command(port: &str) -> TokioCommand {
    let mut cmd = TokioCommand::new(OPENCODE_BINARY);
    cmd.arg(SERVE_COMMAND)
        .arg(PORT_FLAG)
        .arg(port)
        .arg(HOSTNAME_FLAG)
        .arg(OPENCODE_SERVER_HOSTNAME)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

/// Spawn an OpenCode server process and wait for it to become healthy.
///
/// Attempts to spawn `opencode serve` with the specified port (or auto-select if port override is not set).
/// Parses the server's stdout to find the listening URL, then polls the health endpoint until ready.
///
/// # Returns
///
/// * `Ok(ServerInfo)` - Server spawned and is healthy
/// * `Err(SpawnError)` - Failed to spawn, parse output, or server didn't become healthy
pub async fn spawn_and_wait() -> Result<ServerInfo, SpawnError> {
    let port_arg = get_override_port()
        .map(|p| p.to_string())
        .unwrap_or_else(|| AUTO_SELECT_PORT.to_string());

    info!("Spawning OpenCode server on port {port_arg}");

    let child = spawn_server_process(&port_arg).await?;
    let (mut child, base_url, port) = parse_server_url(child).await?;

    if let Err(e) = wait_for_health(&base_url).await {
        warn!(
            "Health check failed, killing spawned server (PID: {:?})",
            child.id()
        );
        let _ = child.kill().await;
        return Err(e);
    }

    let pid = child.id().unwrap_or_default();

    info!("OpenCode server ready at {base_url} (PID: {pid})");

    // Detach the child process - it will continue running as a daemon
    // The OS will clean it up when it exits
    forget(child);

    let server_info = ServerInfoBuilder::default()
        .with_pid(pid)
        .with_port(port)
        .with_base_url(base_url)
        .with_name(OPENCODE_BINARY)
        .with_command(format!("{OPENCODE_BINARY} {SERVE_COMMAND}"))
        .with_owned(true)
        .build()?;

    Ok(server_info)
}

async fn spawn_server_process(port: &str) -> Result<TokioChild, SpawnError> {
    debug!("Attempting to spawn {OPENCODE_BINARY} from PATH");

    match build_spawn_command(port).spawn() {
        Ok(child) => {
            info!(
                "Spawned {OPENCODE_BINARY} from PATH (PID: {:?})",
                child.id()
            );
            Ok(child)
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            debug!("{OPENCODE_BINARY} not in PATH, trying local binary");
            spawn_local_binary(port)
        }
        Err(err) => Err(SpawnError::Spawn {
            message: format!("Failed to spawn {OPENCODE_BINARY}: {err}"),
            location: ErrorLocation::from(Location::caller()),
            source: Box::new(err),
        }),
    }
}

fn spawn_local_binary(port: &str) -> Result<TokioChild, SpawnError> {
    let exe = current_exe().map_err(|e| SpawnError::Spawn {
        message: format!("Failed to get current executable path: {e}"),
        location: ErrorLocation::from(Location::caller()),
        source: Box::new(e),
    })?;

    let dir = exe.parent().ok_or_else(|| SpawnError::Spawn {
        message: format!("Executable has no parent directory: {}", exe.display()),
        location: ErrorLocation::from(Location::caller()),
        source: Box::new(IoError::new(ErrorKind::NotFound, "no parent dir")),
    })?;

    let local_path = dir.join(OPENCODE_BINARY);
    debug!("Attempting to spawn from {}", local_path.display());

    build_spawn_command(port)
        .current_dir(dir)
        .spawn()
        .map_err(|e| SpawnError::Spawn {
            message: format!(
                "Failed to spawn {OPENCODE_BINARY} from {}: {e}",
                local_path.display()
            ),
            location: ErrorLocation::from(Location::caller()),
            source: Box::new(e),
        })
}

async fn parse_server_url(mut child: TokioChild) -> Result<(TokioChild, String, u16), SpawnError> {
    let stdout = child.stdout.take().ok_or_else(|| SpawnError::Parse {
        message: "Child process has no stdout".to_string(),
        location: ErrorLocation::from(Location::caller()),
    })?;

    let stderr = child.stderr.take();

    // Spawn a task to capture stderr for debugging
    if let Some(stderr) = stderr {
        TokioSpawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                trace!("Server stderr: {line}");
            }
        });
    }

    let mut lines = BufReader::new(stdout).lines();
    let re = get_url_regex();

    for _ in 0..SPAWN_MAX_OUTPUT_LINES {
        match lines.next_line().await {
            Ok(Some(line)) => {
                trace!("Server output: {line}");

                if let Some(cap) = re.captures(&line) {
                    let host = cap
                        .name(URL_CAPTURE_HOST)
                        .ok_or_else(|| SpawnError::Parse {
                            message: format!(
                                "Regex matched but missing '{URL_CAPTURE_HOST}' capture group"
                            ),
                            location: ErrorLocation::from(Location::caller()),
                        })?
                        .as_str();

                    let port_str = cap
                        .name(URL_CAPTURE_PORT)
                        .ok_or_else(|| SpawnError::Parse {
                            message: format!(
                                "Regex matched but missing '{URL_CAPTURE_PORT}' capture group"
                            ),
                            location: ErrorLocation::from(Location::caller()),
                        })?
                        .as_str();

                    match port_str.parse::<u16>() {
                        Ok(port) => {
                            if host != OPENCODE_SERVER_HOSTNAME {
                                warn!(
                                    "Server reported unexpected hostname: {host}, expected {OPENCODE_SERVER_HOSTNAME}"
                                );
                            }

                            let base_url = format!("{OPENCODE_SERVER_BASE_URL}:{port}");
                            info!("Parsed server URL: {base_url}");
                            return Ok((child, base_url, port));
                        }
                        Err(e) => {
                            warn!("Failed to parse port '{port_str}': {e}");
                        }
                    }
                }
            }
            Ok(None) => {
                debug!("Server process ended before printing URL");
                break;
            }
            Err(e) => {
                return Err(SpawnError::Parse {
                    message: format!("Failed to read server output: {e}"),
                    location: ErrorLocation::from(Location::caller()),
                });
            }
        }
    }

    Err(SpawnError::Parse {
        message: format!("No server URL found in first {SPAWN_MAX_OUTPUT_LINES} lines of output"),
        location: ErrorLocation::from(Location::caller()),
    })
}

async fn wait_for_health(base_url: &str) -> Result<(), SpawnError> {
    let mut backoff = ExponentialBackoff {
        max_elapsed_time: Some(HEALTH_CHECK_MAX_ELAPSED),
        ..Default::default()
    };

    debug!("Waiting for server health at {base_url}");

    loop {
        if check_health(base_url).await {
            info!("Server is healthy at {base_url}");
            return Ok(());
        }

        match backoff.next_backoff() {
            Some(duration) => {
                trace!("Server not ready, retrying after {duration:?}");
                TokioSleep(duration).await;
            }
            None => {
                return Err(SpawnError::Timeout {
                    message: format!(
                        "Server at {base_url} did not become healthy within {HEALTH_CHECK_MAX_ELAPSED:?}"
                    ),
                    location: ErrorLocation::from(Location::caller()),
                });
            }
        }
    }
}
