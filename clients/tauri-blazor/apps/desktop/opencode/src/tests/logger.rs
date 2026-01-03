// Unit tests for logger module initialization logic
// Tests focus on thread-safety and error handling

use crate::logger::initialize;
use std::path::PathBuf;

/// **VALUE**: Verifies that calling initialize() multiple times doesn't panic or fail.
///
/// **WHY THIS MATTERS**: In complex applications, logger initialization might be called
/// from multiple code paths (setup hooks, tests, etc.). If it panics or errors on the
/// second call, it would crash the application during startup.
///
/// **BUG THIS CATCHES**: Would catch if the Once or AtomicBool guards are removed,
/// causing fern to panic when trying to set a global logger twice.
#[test]
fn given_logger_initialized_when_called_again_then_returns_ok() {
    // GIVEN: A valid temporary directory
    let temp_dir = std::env::temp_dir().join("opencode-test-logger-1");
    std::fs::create_dir_all(&temp_dir).unwrap();

    // WHEN: Calling initialize twice
    let result1 = initialize(&temp_dir);
    let result2 = initialize(&temp_dir);

    // THEN: Both should return Ok (second one logs warning but doesn't error)
    assert!(result1.is_ok(), "First initialization should succeed");
    assert!(
        result2.is_ok(),
        "Second initialization should succeed (idempotent)"
    );

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

/// **VALUE**: Verifies that logger handles non-existent directories gracefully.
///
/// **WHY THIS MATTERS**: If the app data directory doesn't exist or can't be created
/// (permissions, disk full, etc.), the logger should return a clear error instead of
/// panicking. This prevents startup crashes from filesystem issues.
///
/// **BUG THIS CATCHES**: Would catch if `fern::log_file()` unwraps instead of returning
/// a Result, causing panics when the log file can't be created.
#[test]
fn given_invalid_log_dir_when_initialize_called_then_returns_error() {
    // GIVEN: A path that will fail (root directory, permission denied)
    // Using a path that's guaranteed to be unwritable on Unix-like systems
    let invalid_dir = PathBuf::from("/dev/null/invalid-path");

    // WHEN: Calling initialize with invalid directory
    let result = initialize(&invalid_dir);

    // THEN: Should return error (not panic)
    assert!(
        result.is_err(),
        "Should return error for invalid log directory"
    );

    let err = result.unwrap_err();
    let err_string = format!("{:?}", err);
    assert!(
        err_string.contains("Opencode"),
        "Error should be OpencodeError::Opencode variant"
    );
}
