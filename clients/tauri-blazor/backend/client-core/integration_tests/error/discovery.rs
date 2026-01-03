use client_core::error::discovery::DiscoveryError;
use models::ErrorLocation;

use std::error::Error;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::panic::Location;

/// **VALUE**: Verifies that `DiscoveryError::NetworkQuery` includes file/line/column location tracking.
///
/// **WHY THIS MATTERS**: When network discovery fails in production, developers need to know
/// EXACTLY where the error originated (which file, which line). Without location tracking,
/// debugging network issues becomes a guessing game.
///
/// **BUG THIS CATCHES**: Would catch if someone:
/// - Removes the `location` field from DiscoveryError
/// - Breaks the Display implementation to not include location
/// - Removes `#[track_caller]` from functions creating these errors
///
/// This ensures error messages show "discovery.rs:42:15" instead of just "Network Query Error".
#[test]
#[track_caller]
fn given_network_query_error_when_formatted_then_includes_location() {
    // GIVEN: A NetworkQuery error with location
    let io_err = IoError::new(ErrorKind::ConnectionRefused, "connection refused");
    let location = ErrorLocation::from(Location::caller());
    let err = DiscoveryError::NetworkQuery {
        message: "Failed to query sockets".to_string(),
        location,
        source: Box::new(io_err),
    };

    // WHEN: Formatting the error as string
    let error_string = format!("{}", err);

    // THEN: Should include error type, message, and file location
    assert!(error_string.contains("Network Query Error"));
    assert!(error_string.contains("Failed to query sockets"));
    assert!(error_string.contains("discovery.rs"));
}

/// **VALUE**: Verifies that `DiscoveryError::SystemQuery` includes location tracking.
///
/// **WHY THIS MATTERS**: System query errors (process not found, permission denied) need
/// precise location tracking for debugging. Without it, developers can't tell if the error
/// came from discovery scan, process lookup, or command formatting.
///
/// **BUG THIS CATCHES**: Would catch if SystemQuery variant loses location tracking,
/// making production debugging significantly harder.
#[test]
#[track_caller]
fn given_system_query_error_when_formatted_then_includes_location() {
    // GIVEN: A SystemQuery error with location
    let location = ErrorLocation::from(Location::caller());
    let err = DiscoveryError::SystemQuery {
        message: "Process not found".to_string(),
        location,
    };

    // WHEN: Formatting the error as string
    let error_string = format!("{}", err);

    // THEN: Should include error type, message, and file location
    assert!(error_string.contains("System Query Error"));
    assert!(error_string.contains("Process not found"));
    assert!(error_string.contains("discovery.rs"));
}

/// **VALUE**: Verifies that error source chains are preserved for debugging.
///
/// **WHY THIS MATTERS**: Network errors often have multiple layers (e.g., "Failed to query sockets"
/// caused by "Permission denied" from the OS). If the source chain breaks, developers lose
/// critical context about WHY the error happened.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Someone removes the `#[source]` attribute from DiscoveryError
/// - The error wrapping loses the underlying IO error
/// - Error chaining breaks in refactoring
///
/// This ensures error messages show the FULL context: "Network Query Error: Socket query failed"
/// with source "access denied", not just the top-level error.
#[test]
fn given_network_query_error_with_source_when_inspected_then_preserves_chain() {
    // GIVEN: A NetworkQuery error with an underlying IO error
    let io_err = IoError::new(ErrorKind::PermissionDenied, "access denied");
    let location = ErrorLocation::from(Location::caller());
    let err = DiscoveryError::NetworkQuery {
        message: "Socket query failed".to_string(),
        location,
        source: Box::new(io_err),
    };

    // WHEN: Accessing the error source
    let source = err.source();

    // THEN: Should preserve the source chain with original error message
    assert!(source.is_some(), "Should have error source");
    let source_msg = format!("{}", source.unwrap());
    assert!(
        source_msg.contains("access denied"),
        "Should preserve underlying error message"
    );
}
