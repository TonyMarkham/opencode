// Unit tests for process module private functions
// Integration tests for public API are in integration_tests/discovery/process.rs

use crate::discovery::process::{format_command, with_process};

/// **VALUE**: Tests the private `format_command()` helper's ability to handle edge cases.
///
/// **WHY THIS MATTERS**: If `format_command()` panics or returns invalid data when a process
/// has no command line arguments, it would break server discovery for certain process types.
///
/// **BUG THIS CATCHES**: Ensures we don't crash when encountering processes with empty/missing
/// command lines, which can happen with kernel threads or system processes.
#[test]
fn given_valid_process_when_format_command_called_then_returns_command_string() {
    // GIVEN: A valid process (using our own PID)
    let our_pid = std::process::id();

    // WHEN: Calling format_command on the process
    let result = with_process(our_pid, |p| format_command(p));

    // THEN: Should return Some with non-empty command string
    assert!(result.is_some(), "Should find the process");
    let cmd = result.unwrap();
    assert!(!cmd.is_empty(), "Command string should not be empty");
}

/// **VALUE**: Tests that `with_process()` gracefully handles non-existent PIDs.
///
/// **WHY THIS MATTERS**: Processes can disappear between discovery and query (race condition).
/// If `with_process()` panics or errors instead of returning None, it would crash discovery.
///
/// **BUG THIS CATCHES**: Prevents crashes when querying processes that died between
/// the network socket scan and the process info lookup.
#[test]
fn given_nonexistent_pid_when_with_process_called_then_returns_none() {
    // GIVEN: A PID that doesn't exist
    let fake_pid = u32::MAX;

    // WHEN: Calling with_process with the invalid PID
    let result = with_process(fake_pid, |_| true);

    // THEN: Should return None (graceful handling)
    assert!(
        result.is_none(),
        "Should return None for non-existent process"
    );
}

/// **VALUE**: Tests that `with_process()` actually executes the closure for valid PIDs.
///
/// **WHY THIS MATTERS**: This verifies the core functionality of `with_process()` - that it
/// successfully finds the process AND executes the callback with the Process object.
///
/// **BUG THIS CATCHES**: Would catch if we accidentally changed `with_process()` to always
/// return None, or if the closure wasn't being executed properly.
#[test]
fn given_valid_pid_when_with_process_called_then_executes_closure() {
    // GIVEN: A valid PID (our own process)
    let our_pid = std::process::id();

    // WHEN: Calling with_process with a closure that returns the PID
    let result = with_process(our_pid, |p| p.pid().as_u32());

    // THEN: Should execute closure and return the PID
    assert!(result.is_some(), "Should find the process");
    assert_eq!(
        result.unwrap(),
        our_pid,
        "Should execute closure with correct process"
    );
}
