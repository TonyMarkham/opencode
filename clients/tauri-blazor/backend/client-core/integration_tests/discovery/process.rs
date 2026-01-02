use client_core::discovery::process::{check_health, discover, stop_pid};
use client_core::discovery::set_override_port;

// ============================================================================
// Public API tests for process discovery and management
// These test the PUBLIC interface from an external consumer's perspective
// ============================================================================

// ----------------------------------------------------------------------------
// stop_pid() - Process termination tests
// ----------------------------------------------------------------------------

/// **VALUE**: Verifies that `stop_pid()` gracefully handles attempts to kill non-existent processes.
///
/// **WHY THIS MATTERS**: In production, processes can die between discovery and termination attempts
/// (race condition). If `stop_pid()` panics or errors instead of returning false, it would crash
/// the application when trying to clean up an already-dead server.
///
/// **BUG THIS CATCHES**: Would catch if `stop_pid()` throws an exception or panics when the
/// PID doesn't exist, instead of gracefully returning false.
#[test]
fn given_nonexistent_pid_when_stop_pid_called_then_returns_false() {
    // GIVEN: A PID that doesn't exist
    let fake_pid = u32::MAX;

    // WHEN: Attempting to stop the process
    let result = stop_pid(fake_pid);

    // THEN: Should return false (graceful handling)
    assert!(!result, "Should return false for non-existent process");
}

/// **VALUE**: Prevents catastrophic system crashes by refusing to kill PID 1 (init/systemd).
///
/// **WHY THIS MATTERS**: Killing PID 1 crashes the entire operating system. This is a critical
/// safety boundary that must NEVER be crossed, even if someone passes PID 1 by mistake.
///
/// **BUG THIS CATCHES**: Would immediately catch if someone removes the PID 1 safety check,
/// preventing production disasters where the entire OS crashes from a bad kill attempt.
#[test]
fn given_pid_1_when_stop_pid_called_then_refuses_and_returns_false() {
    // GIVEN: PID 1 (init/systemd)
    let pid_1 = 1;

    // WHEN: Attempting to stop PID 1
    let result = stop_pid(pid_1);

    // THEN: Should refuse and return false (safety boundary)
    assert!(!result, "Should never kill PID 1 (init process)");
}

// ----------------------------------------------------------------------------
// check_health() - Server health check tests
// ----------------------------------------------------------------------------

/// **VALUE**: Verifies that `check_health()` handles connection failures gracefully.
///
/// **WHY THIS MATTERS**: In production, health checks will fail often (server not started yet,
/// network issues, wrong port). If `check_health()` panics or hangs instead of returning false,
/// it would break server discovery and spawning workflows.
///
/// **BUG THIS CATCHES**: Would catch if someone changes the HTTP client code to unwrap() instead
/// of gracefully handling connection errors, which would panic on unreachable servers.
#[tokio::test]
async fn given_unreachable_port_when_check_health_called_then_returns_false() {
    // GIVEN: A port that definitely has no server listening
    let unreachable_url = "http://127.0.0.1:65534";

    // WHEN: Checking health
    let result = check_health(unreachable_url).await;

    // THEN: Should return false (graceful handling of connection failure)
    assert!(!result, "Should return false for unreachable server");
}

/// **VALUE**: Tests that `check_health()` handles invalid URL formats without panicking.
///
/// **WHY THIS MATTERS**: If malformed URLs cause panics instead of returning false, it would
/// crash the application when receiving corrupted server output or bad configuration.
///
/// **BUG THIS CATCHES**: Would catch if someone changes the URL parsing to unwrap() instead
/// of gracefully handling parse errors, causing crashes on invalid input.
#[tokio::test]
async fn given_malformed_url_when_check_health_called_then_returns_false() {
    // GIVEN: An invalid URL format
    let malformed_url = "not-a-valid-url";

    // WHEN: Checking health
    let result = check_health(malformed_url).await;

    // THEN: Should return false (graceful handling of parse error)
    assert!(!result, "Should return false for malformed URL");
}

/// **VALUE**: Tests defensive programming - empty strings should not cause crashes.
///
/// **WHY THIS MATTERS**: Edge case that could happen with bad configuration or corrupted state.
/// If empty strings panic, it would crash the application in unexpected ways.
///
/// **BUG THIS CATCHES**: Would catch if URL validation is missing and empty strings cause
/// unwrap() panics or infinite loops in the HTTP client.
#[tokio::test]
async fn given_empty_url_when_check_health_called_then_returns_false() {
    // GIVEN: An empty string
    let empty_url = "";

    // WHEN: Checking health
    let result = check_health(empty_url).await;

    // THEN: Should return false (defensive programming)
    assert!(!result, "Should return false for empty URL");
}

// ----------------------------------------------------------------------------
// discover() - Server discovery tests
// ----------------------------------------------------------------------------

/// **VALUE**: Tests that discovery with port override doesn't error when the port is empty.
///
/// **WHY THIS MATTERS**: Port override is used for testing and development. If it errors when
/// the port is unused, it would make testing impossible and confuse developers.
///
/// **BUG THIS CATCHES**: Would catch if `discover_on_port()` panics or returns Err() when
/// no process is listening on the target port, instead of gracefully returning Ok(None).
#[test]
fn given_port_override_with_no_server_when_discover_called_then_returns_ok() {
    // GIVEN: Port override set to a port with no server
    set_override_port(65530);

    // WHEN: Discovering servers
    let result = discover();

    // THEN: Should return Ok (may be None or Some, but shouldn't error)
    assert!(result.is_ok(), "Should not error when no server found");
    // May return None if port is empty, or Some if another process is using it
    // The important part is it doesn't panic or error
}

/// **VALUE**: Verifies that discovery handles the "no servers running" case gracefully.
///
/// **WHY THIS MATTERS**: This is the common case on first launch - no servers exist yet.
/// If discovery errors or panics instead of returning Ok(None), the application can't
/// start cleanly and offer to spawn a server.
///
/// **BUG THIS CATCHES**: Would catch if process scanning throws errors when the process
/// list is empty or when no matching processes are found, breaking first-launch experience.
#[test]
fn given_no_servers_running_when_discover_called_then_returns_ok() {
    // GIVEN: No opencode servers running (assumed in test environment)

    // WHEN: Discovering servers
    let result = discover();

    // THEN: Should return Ok (not an error)
    assert!(
        result.is_ok(),
        "Discovery should not error when no servers found"
    );
}
