//! Production-grade logging for OpenCode desktop application.
//!
//! Provides dual output (stdout with colors + file) with thread-safe initialization.

use crate::error::OpencodeError;

use models::ErrorLocation;

use std::io::stdout;
use std::path::Path;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use fern::Dispatch;
use fern::colors::Color::{Blue, Green, Magenta, Red, Yellow};
use fern::colors::ColoredLevelConfig;
use humantime::format_rfc3339;
use log::{LevelFilter, info, warn};

/// Thread-safe initialization guard.
static INIT_LOGGER_ONCE: Once = Once::new();

/// Tracks if logger initialization was already attempted.
static LOGGER_ALREADY_CALLED: AtomicBool = AtomicBool::new(false);

/// Log file name.
const LOG_FILE_NAME: &str = "opencode.log";

/// Message logged when logger is successfully initialized.
const LOGGER_INITIALIZED_MESSAGE_PREFIX: &str = "Logger initialized with level: ";

/// Warning message when logger is called multiple times.
const LOGGER_ALREADY_INITIALIZED_MESSAGE: &str = "Logger already initialized";

/// Default log level for debug builds.
#[cfg(debug_assertions)]
const LOG_LEVEL: LevelFilter = LevelFilter::Debug;

/// Default log level for release builds.
#[cfg(not(debug_assertions))]
const LOG_LEVEL: LevelFilter = LevelFilter::Info;

/// Initialize the logger with dual output (stdout + file).
///
/// This function is safe to call multiple times - subsequent calls will
/// log a warning and return Ok. The actual initialization runs exactly once.
///
/// # Arguments
///
/// * `log_dir` - Directory where the log file will be created
///
/// # Errors
///
/// Returns an error if:
/// - Log file cannot be created
/// - Logger dispatch configuration fails
pub fn initialize(log_dir: &Path) -> Result<(), OpencodeError> {
    if LOGGER_ALREADY_CALLED.swap(true, Ordering::SeqCst) {
        warn!("{LOGGER_ALREADY_INITIALIZED_MESSAGE}");
        return Ok(());
    }

    let mut result = Ok(());

    INIT_LOGGER_ONCE.call_once(|| {
        result = initialize_internal(log_dir);
        if result.is_ok() {
            info!("{LOGGER_INITIALIZED_MESSAGE_PREFIX}{LOG_LEVEL:?}");
        }
    });

    result
}

/// Internal logger initialization with dual dispatch.
#[track_caller]
fn initialize_internal(log_dir: &Path) -> Result<(), OpencodeError> {
    let log_file_path = log_dir.join(LOG_FILE_NAME);

    // Color configuration for stdout
    let color_configuration = ColoredLevelConfig::new()
        .debug(Blue)
        .info(Green)
        .warn(Yellow)
        .error(Red)
        .trace(Magenta);

    // Base dispatch with level filter
    let base_dispatch = Dispatch::new().level(LOG_LEVEL);

    // Stdout dispatch (colored)
    let stdout_dispatch = Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{date} - {level}] {message} [{file}:{line}]",
                date = format_rfc3339(SystemTime::now()),
                level = color_configuration.color(record.level()),
                message = message,
                file = record.file().unwrap_or("unknown"),
                line = record.line().unwrap_or(0),
            ))
        })
        .chain(stdout());

    // File dispatch (plain text, no colors)
    let file_dispatch = Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{date} - {level}] {message} [{file}:{line}]",
                date = format_rfc3339(SystemTime::now()),
                level = record.level(),
                message = message,
                file = record.file().unwrap_or("unknown"),
                line = record.line().unwrap_or(0)
            ))
        })
        .chain(
            fern::log_file(&log_file_path).map_err(|e| OpencodeError::Opencode {
                message: format!("Failed to create log file: {e}"),
                location: ErrorLocation::from(std::panic::Location::caller()),
            })?,
        );

    // Apply the configuration
    base_dispatch
        .chain(stdout_dispatch)
        .chain(file_dispatch)
        .apply()
        .map_err(|e| OpencodeError::Opencode {
            message: format!("Failed to initialize logger: {e}"),
            location: ErrorLocation::from(std::panic::Location::caller()),
        })?;

    Ok(())
}
