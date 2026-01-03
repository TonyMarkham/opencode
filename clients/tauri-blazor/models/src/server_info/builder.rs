use crate::error::model_error::ModelError;
use crate::{ErrorLocation, ServerInfo};

use std::panic::Location;

/// Builder for creating validated ServerInfo instances.
///
/// Provides a fluent API for constructing ServerInfo with automatic
/// type conversions (e.g., u16 port -> u32 for protobuf).
#[derive(Debug, Default)]
pub struct ServerInfoBuilder {
    pid: Option<u32>,
    port: Option<u16>,
    base_url: Option<String>,
    name: Option<String>,
    command: Option<String>,
    owned: Option<bool>,
}

impl ServerInfoBuilder {
    pub fn with_pid(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_command(mut self, cmd: impl Into<String>) -> Self {
        self.command = Some(cmd.into());
        self
    }

    pub fn with_owned(mut self, owned: bool) -> Self {
        self.owned = Some(owned);
        self
    }

    /// Build the ServerInfo with validation.
    #[track_caller]
    pub fn build(self) -> Result<ServerInfo, ModelError> {
        let pid = self.pid.ok_or_else(|| ModelError::Validation {
            message: String::from("PID is required"),
            location: ErrorLocation::from(Location::caller()),
        })?;

        if pid == 0 {
            return Err(ModelError::Validation {
                message: String::from("PID must be non-zero"),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        let port = self.port.ok_or_else(|| ModelError::Validation {
            message: String::from("Port is required"),
            location: ErrorLocation::from(Location::caller()),
        })?;

        let base_url = self.base_url.ok_or_else(|| ModelError::Validation {
            message: String::from("Base URL is required"),
            location: ErrorLocation::from(Location::caller()),
        })?;

        if base_url.is_empty() {
            return Err(ModelError::Validation {
                message: String::from("Base URL cannot be empty"),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return Err(ModelError::Validation {
                message: format!("Invalid base URL format: {base_url}"),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        let name = self.name.ok_or_else(|| ModelError::Validation {
            message: String::from("Server name is required"),
            location: ErrorLocation::from(Location::caller()),
        })?;

        if name.is_empty() {
            return Err(ModelError::Validation {
                message: String::from("Server name cannot be empty"),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        let command = self.command.ok_or_else(|| ModelError::Validation {
            message: String::from("Command is required"),
            location: ErrorLocation::from(Location::caller()),
        })?;

        if command.is_empty() {
            return Err(ModelError::Validation {
                message: String::from("Command cannot be empty"),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        let owned = self.owned.ok_or_else(|| ModelError::Validation {
            message: String::from("Owned is required"),
            location: ErrorLocation::from(Location::caller()),
        })?;

        Ok(ServerInfo {
            pid,
            port: port.into(),
            base_url,
            name,
            command,
            owned,
        })
    }
}
