use models::ErrorLocation;

use serde::Serialize;
use thiserror::Error;

/// Errors that can occur in Tauri commands.
///
/// These errors are converted to strings for IPC, but we maintain
/// structured error information and location tracking internally.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum OpencodeError {
    /// Error from this App
    #[error("Opencode Error: {message} {location}")]
    Opencode {
        message: String,
        location: ErrorLocation,
    },

    /// Error from client-core operations (discovery, spawn, etc.)
    #[error("Core Error: {message} {location}")]
    Core {
        message: String,
        location: ErrorLocation,
    },

    /// No server is currently connected
    #[error("No Server Error: {message} {location}")]
    NoServer {
        message: String,
        location: ErrorLocation,
    },

    /// Server failed to stop
    #[error("Stop Error: {message} {location}")]
    StopFailed {
        message: String,
        location: ErrorLocation,
    },
}
