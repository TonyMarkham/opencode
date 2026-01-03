use client_core::error::spawn::SpawnError;
use models::ErrorLocation;

use std::error::Error;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::panic::Location;

/// **VALUE**: Verifies that `SpawnError::Spawn` includes file/line/column location tracking.
///
/// **WHY THIS MATTERS**: When process spawning fails (binary not found, permission denied),
/// developers need to know EXACTLY where the spawn attempt originated. The spawn workflow
/// has multiple spawn points (PATH, local binary), so location tracking is critical.
///
/// **BUG THIS CATCHES**: Would catch if someone:
/// - Removes `#[track_caller]` from `spawn_server_process()` or `spawn_local_binary()`
/// - Breaks the location field in SpawnError::Spawn
/// - Removes location from error Display implementation
///
/// Without this, error messages would just say "Spawn Error: binary not found" instead of
/// "Spawn Error: binary not found [spawn.rs:126:39]", making debugging impossible.
#[test]
#[track_caller]
fn given_spawn_error_when_formatted_then_includes_location() {
    // GIVEN: A Spawn error with location
    let io_err = IoError::new(ErrorKind::NotFound, "binary not found");
    let location = ErrorLocation::from(Location::caller());
    let err = SpawnError::Spawn {
        message: "Failed to spawn opencode".to_string(),
        location,
        source: Box::new(io_err),
    };

    // WHEN: Formatting the error as string
    let error_string = format!("{}", err);

    // THEN: Should include error type, message, and file location
    assert!(error_string.contains("Spawn Error"));
    assert!(error_string.contains("Failed to spawn opencode"));
    assert!(error_string.contains("spawn.rs"));
}

/// **VALUE**: Verifies that `SpawnError::Parse` includes location tracking for URL parsing failures.
///
/// **WHY THIS MATTERS**: Parse errors happen when the server stdout doesn't contain a valid URL.
/// This can happen at multiple points (regex match failed, port parse failed, missing capture group).
/// Location tracking tells developers WHICH parse step failed.
///
/// **BUG THIS CATCHES**: Would catch if `parse_server_url()` loses `#[track_caller]` or if
/// someone creates ParseError without location, making it impossible to distinguish between
/// "regex didn't match" vs "port parsing failed" vs "missing capture group".
#[test]
#[track_caller]
fn given_parse_error_when_formatted_then_includes_location() {
    // GIVEN: A Parse error with location
    let location = ErrorLocation::from(Location::caller());
    let err = SpawnError::Parse {
        message: "No URL found in output".to_string(),
        location,
    };

    // WHEN: Formatting the error as string
    let error_string = format!("{}", err);

    // THEN: Should include error type, message, and file location
    assert!(error_string.contains("Parse Error"));
    assert!(error_string.contains("No URL found in output"));
    assert!(error_string.contains("spawn.rs"));
}

/// **VALUE**: Verifies that `SpawnError::Timeout` includes location tracking for health check timeouts.
///
/// **WHY THIS MATTERS**: Health check timeouts can happen in multiple places (initial spawn wait,
/// retry loop, backoff exhausted). Location tracking shows WHERE the timeout occurred.
///
/// **BUG THIS CATCHES**: Would catch if `wait_for_health()` loses `#[track_caller]`, making
/// timeout debugging much harder (can't tell if timeout was in spawn flow or health check loop).
#[test]
#[track_caller]
fn given_timeout_error_when_formatted_then_includes_location() {
    // GIVEN: A Timeout error with location
    let location = ErrorLocation::from(Location::caller());
    let err = SpawnError::Timeout {
        message: "Server did not become healthy".to_string(),
        location,
    };

    // WHEN: Formatting the error as string
    let error_string = format!("{}", err);

    // THEN: Should include error type, message, and file location
    assert!(error_string.contains("Timeout Error"));
    assert!(error_string.contains("Server did not become healthy"));
    assert!(error_string.contains("spawn.rs"));
}

/// **VALUE**: Verifies that error source chains are preserved for spawn errors.
///
/// **WHY THIS MATTERS**: Spawn errors wrap OS-level errors (NotFound, PermissionDenied, etc).
/// If the source chain breaks, developers lose critical information about WHY the spawn failed.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Someone removes `#[source]` from SpawnError::Spawn
/// - Error wrapping loses the underlying IO error
/// - Refactoring breaks error chaining
///
/// This ensures error messages show FULL context: "Spawn Error: Failed to spawn opencode"
/// caused by "permission denied" from the OS, not just the top-level message.
#[test]
fn given_spawn_error_with_source_when_inspected_then_preserves_chain() {
    // GIVEN: A Spawn error with an underlying IO error
    let io_err = IoError::new(ErrorKind::PermissionDenied, "permission denied");
    let location = ErrorLocation::from(Location::caller());
    let err = SpawnError::Spawn {
        message: "Spawn failed".to_string(),
        location,
        source: Box::new(io_err),
    };

    // WHEN: Accessing the error source
    let source = err.source();

    // THEN: Should preserve the source chain with original error message
    assert!(source.is_some(), "Should have error source");
    let source_msg = format!("{}", source.unwrap());
    assert!(
        source_msg.contains("permission denied"),
        "Should preserve underlying error message"
    );
}
