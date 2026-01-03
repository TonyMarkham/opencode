// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod logger;
mod state;

use crate::error::OpencodeError;
use crate::logger::initialize as LoggerInitialize;
use crate::state::AppState;

use models::ErrorLocation;

use std::fs::create_dir_all;
use std::panic::Location;

use log::info;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::server::discover_server,
            commands::server::spawn_server,
            commands::server::check_health,
            commands::server::stop_server,
        ])
        .setup(|app| {
            // Get app data directory for logs
            let log_dir = app
                .path()
                .app_log_dir()
                .map_err(|e| OpencodeError::Opencode {
                    message: format!("Failed to get log directory: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?;

            // Ensure log directory exists
            create_dir_all(&log_dir).map_err(|e| OpencodeError::Opencode {
                message: format!("Failed to create log directory: {}", e),
                location: ErrorLocation::from(Location::caller()),
            })?;

            // Initialize logger FIRST
            LoggerInitialize(&log_dir)?;

            info!("OpenCode Tauri application starting");
            info!("Log directory: {}", log_dir.display());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
