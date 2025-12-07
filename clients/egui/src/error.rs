use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("failed to query system processes: {0}")]
    SystemQuery(String),
    #[error("failed to query network sockets: {0}")]
    NetworkQuery(String),
}

#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("failed to spawn opencode: {0}")]
    Spawn(String),
    #[error("failed to parse server url from output")] 
    Parse,
    #[error("server did not become ready within timeout")] 
    Timeout,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("invalid base url: {0}")]
    Url(String),
    #[error("http error: {0}")]
    Http(String),
    #[error("decode error: {0}")]
    Decode(String),
}

#[derive(Debug, Error)]
pub enum EventsError {
    #[error("http error: {0}")]
    Http(String),
}