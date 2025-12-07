use std::{process::Stdio, time::Duration};

use regex::Regex;
use thiserror::Error;

use crate::discovery::process::{check_health, ServerInfo};

#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("failed to spawn opencode: {0}")]
    Spawn(String),
    #[error("failed to parse server url from output")] 
    Parse,
    #[error("server did not become ready within timeout")] 
    Timeout,
}

/// Spawn `opencode serve --port 0 --hostname 127.0.0.1` and parse the printed URL line.
/// Then poll GET {base_url}/doc until success or timeout.
pub async fn spawn_and_wait() -> Result<ServerInfo, SpawnError> {
    let mut child = tokio::process::Command::new("opencode")
        .arg("serve")
        .arg("--port").arg("0")
        .arg("--hostname").arg("127.0.0.1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| SpawnError::Spawn(e.to_string()))?;

    let mut stdout = tokio::io::BufReader::new(child.stdout.take().expect("stdout")).lines();

    // Example line from server: "opencode server listening on http://127.0.0.1:4096"
    let re = Regex::new(r"http://([^\s:]+):(\d+)").unwrap();
    let (hostname, port): (String, u16);

    let mut found = None;
    // Read a few lines to find the URL
    for _ in 0..100 {
        if let Some(Ok(line)) = stdout.next_line().await {
            if let Some(cap) = re.captures(&line) {
                let host = cap.get(1).unwrap().as_str().to_string();
                let p: u16 = cap.get(2).unwrap().as_str().parse().unwrap_or(0);
                if p != 0 { found = Some((host, p)); break; }
            }
        } else {
            break;
        }
    }

    let (host, p) = found.ok_or(SpawnError::Parse)?;
    let base_url = format!("http://{host}:{p}");

    // Wait for readiness
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        if check_health(&base_url).await { 
            let pid = child.id().unwrap_or_default();
            return Ok(ServerInfo { pid, port: p, base_url, name: "opencode".into(), command: "opencode serve".into() }); 
        }
        if tokio::time::Instant::now() > deadline { return Err(SpawnError::Timeout); }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}
