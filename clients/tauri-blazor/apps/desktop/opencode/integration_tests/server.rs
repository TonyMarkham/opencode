use client_core::discovery::process;
use models::ServerInfo;
use opencode::state::{AppState, StateCommand};

// ============================================================================
// Integration tests for state + client-core integration
// These test the integration between AppState and client-core operations
// ============================================================================

/// **VALUE**: Tests that real process discovery integrates with state management.
///
/// **WHY THIS MATTERS**: This is the closest we can get to an end-to-end test without
/// Tauri runtime. It calls real client-core::discover() and updates real AppState.
///
/// **BUG THIS CATCHES**: Would catch if client-core::discover() returns data that
/// can't be stored in AppState, or if there are type mismatches between the crates.
///
/// **NOTE**: This test's outcome depends on whether an opencode server is running.
/// We're testing the integration works, not whether a server exists.
#[tokio::test]
async fn given_real_discovery_when_updating_state_then_integration_works() {
    // GIVEN: Fresh AppState
    let state = AppState::new();

    // WHEN: Calling real client-core discovery
    let discovery_result = process::discover();

    // THEN: Discovery should not error (Ok(None) or Ok(Some) are both valid)
    assert!(
        discovery_result.is_ok(),
        "Discovery should not error: {:?}",
        discovery_result
    );

    // WHEN: If discovery found a server, update state
    if let Ok(Some(server_info)) = discovery_result {
        let update_result = state
            .update(StateCommand::SetServer(server_info.clone()))
            .await;

        // THEN: State update should succeed
        assert!(update_result.is_ok(), "State update should succeed");

        // Give actor time to process
        tokio::task::yield_now().await;

        // AND: State should contain the discovered server
        let retrieved = state.get_server().await;
        assert!(retrieved.is_some(), "State should have server");

        let server = retrieved.unwrap();
        assert_eq!(server.pid, server_info.pid, "State should have correct PID");
    }
    // If no server found, that's also a valid test outcome
}

/// **VALUE**: Tests that real health check integrates with state-stored server info.
///
/// **WHY THIS MATTERS**: Commands read server info from state, then call client-core
/// operations. This tests that the data flow works end-to-end.
///
/// **BUG THIS CATCHES**: Would catch if ServerInfo data stored in state can't be used
/// to call client-core functions (e.g., base_url format mismatch).
#[tokio::test]
async fn given_server_in_state_when_checking_health_then_uses_state_data() {
    // GIVEN: AppState with a mock server
    let state = AppState::new();
    let mock_server = ServerInfo {
        pid: 12345,
        port: 65534, // Port unlikely to have a server
        base_url: String::from("http://127.0.0.1:65534"),
        name: String::from("opencode"),
        command: String::from("opencode serve"),
        owned: false,
    };
    state
        .update(StateCommand::SetServer(mock_server.clone()))
        .await
        .unwrap();

    // Give actor time to process
    tokio::task::yield_now().await;

    // WHEN: Getting server from state and checking health
    let server_info = state.get_server().await;
    assert!(server_info.is_some());

    let server = server_info.unwrap();
    let health_result = process::check_health(&server.base_url).await;

    // THEN: Health check should complete without panicking
    // (will likely return false since port 65534 probably has no server,
    // but the important thing is it doesn't crash)
    assert!(
        !health_result,
        "Health check should return false for non-existent server"
    );
}

/// **VALUE**: Tests that concurrent state reads during simulated discovery don't deadlock.
///
/// **WHY THIS MATTERS**: UI might be polling state while discovery is updating it.
/// If reads block writes or vice versa, the UI will freeze.
///
/// **BUG THIS CATCHES**: Would catch if RwLock is misconfigured and readers block
/// writers indefinitely, or if the actor pattern has deadlock issues.
#[tokio::test]
async fn given_concurrent_reads_and_writes_when_executed_then_no_deadlock() {
    // GIVEN: Shared AppState
    let state = AppState::new();

    // WHEN: Spawning concurrent readers and writers
    let state1 = state.clone();
    let state2 = state.clone();
    let state3 = state.clone();
    let state4 = state.clone();

    let writer1 = tokio::spawn(async move {
        let server = ServerInfo {
            pid: 11111,
            port: 4001,
            base_url: String::from("http://127.0.0.1:4001"),
            name: String::from("opencode"),
            command: String::from("opencode serve"),
            owned: true,
        };
        state1.update(StateCommand::SetServer(server)).await
    });

    let reader1 = tokio::spawn(async move { state2.get_server().await });

    let reader2 = tokio::spawn(async move { state3.get_server().await });

    let writer2 = tokio::spawn(async move { state4.update(StateCommand::ClearServer).await });

    // THEN: All operations should complete without deadlock
    let timeout = tokio::time::Duration::from_secs(2);
    let result = tokio::time::timeout(timeout, async {
        let (w1, r1, r2, w2) = tokio::join!(writer1, reader1, reader2, writer2);
        (w1.is_ok(), r1.is_ok(), r2.is_ok(), w2.is_ok())
    })
    .await;

    assert!(
        result.is_ok(),
        "Operations should complete within 2 seconds (no deadlock)"
    );

    let (w1_ok, r1_ok, r2_ok, w2_ok) = result.unwrap();
    assert!(
        w1_ok && r1_ok && r2_ok && w2_ok,
        "All operations should succeed"
    );
}
