// Unit tests for error module
// Tests error serialization (critical for Tauri IPC)

use crate::error::OpencodeError;
use models::ErrorLocation;
use std::panic::Location;

/// **VALUE**: Tests that errors can be serialized (required for Tauri IPC).
///
/// **WHY THIS MATTERS**: Tauri commands must return serializable errors to send them
/// to the frontend. If serialization breaks, the frontend receives opaque errors.
///
/// **BUG THIS CATCHES**: Would catch if someone removes the `#[derive(Serialize)]`
/// or if the error structure becomes non-serializable (e.g., adding a non-serializable field).
#[test]
fn given_opencode_error_when_serialized_then_succeeds() {
    // GIVEN: An OpencodeError
    let err = OpencodeError::NoServer {
        message: String::from("Test"),
        location: ErrorLocation::from(Location::caller()),
    };

    // WHEN: Serializing to JSON
    let result = serde_json::to_string(&err);

    // THEN: Should succeed
    assert!(result.is_ok(), "Error should be serializable for Tauri IPC");

    // AND: Should contain the error data
    let json = result.unwrap();
    assert!(
        json.contains("NoServer"),
        "JSON should contain variant name"
    );
    assert!(json.contains("Test"), "JSON should contain message");
}
