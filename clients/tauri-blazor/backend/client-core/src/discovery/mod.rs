//! Server discovery and spawning utilities.
//!
//! This module provides functionality for:
//! - Discovering running OpenCode server processes
//! - Spawning new server instances when none are found
//! - Managing port overrides for development and testing
//!
//! # Port Override
//!
//! By default, discovery scans for running servers on any port. You can override
//! this behavior to target a specific port using [`set_override_port`].

pub mod process;
pub mod spawn;

use std::sync::Mutex;

static OVERRIDE_PORT: Mutex<Option<u16>> = Mutex::new(None);

/// Set a port override for server discovery and spawning.
///
/// When set, the discovery process will attempt to connect to this specific port
/// instead of scanning for running servers. The spawn process will also use this
/// port when starting a new server.
///
/// # Arguments
///
/// * `port` - The port number to use for server discovery and spawning
pub fn set_override_port(port: u16) {
    if let Ok(mut p) = OVERRIDE_PORT.lock() {
        *p = Some(port);
    }
}

/// Get the current port override, if set.
///
/// Returns `None` if no override is configured, otherwise returns the port number
/// that should be used for discovery and spawning.
///
/// # Returns
///
/// * `Some(port)` - If a port override is configured
/// * `None` - If no override is set
pub fn get_override_port() -> Option<u16> {
    OVERRIDE_PORT.lock().ok().and_then(|p| *p)
}
