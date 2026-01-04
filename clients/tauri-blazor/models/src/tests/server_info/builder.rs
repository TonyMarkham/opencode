use crate::{ModelError, ServerInfoBuilder};

/// **VALUE**: Verifies that builder validation rejects zero PIDs.
///
/// **WHY THIS MATTERS**: PID 0 is an invalid process ID on all platforms.
/// Allowing it would break process tracking and health checks throughout the system.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Validation logic is accidentally removed or bypassed
/// - PID zero check is deleted during refactoring
/// - Builder allows invalid ServerInfo instances to be created
///
/// This prevents corrupted ServerInfo data from entering the system.
#[test]
fn given_zero_pid_when_building_server_info_then_returns_validation_error() {
    // GIVEN: Builder with PID set to zero
    let builder = ServerInfoBuilder::default()
        .with_pid(0)
        .with_port(3000)
        .with_base_url("http://localhost:3000")
        .with_name("opencode")
        .with_command("opencode serve")
        .with_owned(true);

    // WHEN: Attempting to build
    let result = builder.build();

    // THEN: Should return validation error
    assert!(result.is_err());
    match result.unwrap_err() {
        ModelError::Validation { message, .. } => {
            assert_eq!(message, "PID must be non-zero");
        }
    }
}

/// **VALUE**: Verifies that builder validation rejects missing PID.
///
/// **WHY THIS MATTERS**: Every ServerInfo must have a PID for process tracking.
/// Missing PIDs would cause null pointer-like failures in health checks and cleanup.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Required field validation is removed
/// - Builder allows incomplete construction
/// - Optional PID accidentally allowed
///
/// This ensures all required fields are provided.
#[test]
fn given_missing_pid_when_building_then_returns_validation_error() {
    // GIVEN: Builder without PID
    let builder = ServerInfoBuilder::default()
        .with_port(3000)
        .with_base_url("http://localhost:3000")
        .with_name("opencode")
        .with_command("opencode serve")
        .with_owned(true);

    // WHEN: Attempting to build
    let result = builder.build();

    // THEN: Should return validation error
    assert!(result.is_err());
    match result.unwrap_err() {
        ModelError::Validation { message, .. } => {
            assert_eq!(message, "PID is required");
        }
    }
}

/// **VALUE**: Verifies that builder validation rejects invalid URL schemes.
///
/// **WHY THIS MATTERS**: Only http:// and https:// URLs work with our HTTP client.
/// Invalid schemes (ftp://, file://, etc.) would cause runtime failures in health checks.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - URL validation regex is broken
/// - Scheme checking logic is removed
/// - Invalid URLs are allowed through
///
/// This prevents HTTP client failures from invalid base URLs.
#[test]
fn given_invalid_url_scheme_when_building_then_returns_validation_error() {
    // GIVEN: Builder with non-http/https URL
    let builder = ServerInfoBuilder::default()
        .with_pid(12345)
        .with_port(3000)
        .with_base_url("ftp://invalid.com")
        .with_name("opencode")
        .with_command("opencode serve")
        .with_owned(true);

    // WHEN: Attempting to build
    let result = builder.build();

    // THEN: Should return validation error with URL in message
    assert!(result.is_err());
    match result.unwrap_err() {
        ModelError::Validation { message, .. } => {
            assert!(message.starts_with("Invalid base URL format:"));
            assert!(message.contains("ftp://"));
        }
    }
}

/// **VALUE**: Verifies that builder validation rejects empty base URLs.
///
/// **WHY THIS MATTERS**: Empty URLs would cause immediate failures in HTTP requests.
/// This catches configuration errors early before runtime failures occur.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Empty string validation is removed
/// - Whitespace-only URLs are allowed
/// - Required field becomes optional
///
/// This prevents empty string bugs in URL construction.
#[test]
fn given_empty_base_url_when_building_then_returns_validation_error() {
    // GIVEN: Builder with empty base URL
    let builder = ServerInfoBuilder::default()
        .with_pid(12345)
        .with_port(3000)
        .with_base_url("")
        .with_name("opencode")
        .with_command("opencode serve")
        .with_owned(true);

    // WHEN: Attempting to build
    let result = builder.build();

    // THEN: Should return validation error
    assert!(result.is_err());
    match result.unwrap_err() {
        ModelError::Validation { message, .. } => {
            assert_eq!(message, "Base URL cannot be empty");
        }
    }
}

/// **VALUE**: Verifies that builder validation rejects missing base URL.
///
/// **WHY THIS MATTERS**: Base URL is required for all server communication.
/// Missing URLs would cause null reference-like failures in HTTP client.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Required field validation is bypassed
/// - Optional URL accidentally allowed
/// - Builder allows incomplete instances
///
/// This ensures base URL is always present.
#[test]
fn given_missing_base_url_when_building_then_returns_validation_error() {
    // GIVEN: Builder without base URL
    let builder = ServerInfoBuilder::default()
        .with_pid(12345)
        .with_port(3000)
        .with_name("opencode")
        .with_command("opencode serve")
        .with_owned(true);

    // WHEN: Attempting to build
    let result = builder.build();

    // THEN: Should return validation error
    assert!(result.is_err());
    match result.unwrap_err() {
        ModelError::Validation { message, .. } => {
            assert_eq!(message, "Base URL is required");
        }
    }
}

/// **VALUE**: Verifies that builder validation rejects empty server names.
///
/// **WHY THIS MATTERS**: Server name is used in logging and UI display.
/// Empty names would produce confusing logs and broken UI elements.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Empty string validation is removed
/// - Name becomes optional
/// - Whitespace-only names allowed
///
/// This ensures meaningful server names in logs and UI.
#[test]
fn given_empty_name_when_building_then_returns_validation_error() {
    // GIVEN: Builder with empty name
    let builder = ServerInfoBuilder::default()
        .with_pid(12345)
        .with_port(3000)
        .with_base_url("http://localhost:3000")
        .with_name("")
        .with_command("opencode serve")
        .with_owned(true);

    // WHEN: Attempting to build
    let result = builder.build();

    // THEN: Should return validation error
    assert!(result.is_err());
    match result.unwrap_err() {
        ModelError::Validation { message, .. } => {
            assert_eq!(message, "Server name cannot be empty");
        }
    }
}

/// **VALUE**: Verifies that builder validation rejects empty commands.
///
/// **WHY THIS MATTERS**: Command is used for debugging and process identification.
/// Empty commands would make troubleshooting impossible and confuse process tracking.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Command validation is removed
/// - Empty strings are allowed
/// - Required field becomes optional
///
/// This ensures command information is always available for debugging.
#[test]
fn given_empty_command_when_building_then_returns_validation_error() {
    // GIVEN: Builder with empty command
    let builder = ServerInfoBuilder::default()
        .with_pid(12345)
        .with_port(3000)
        .with_base_url("http://localhost:3000")
        .with_name("opencode")
        .with_command("")
        .with_owned(true);

    // WHEN: Attempting to build
    let result = builder.build();

    // THEN: Should return validation error
    assert!(result.is_err());
    match result.unwrap_err() {
        ModelError::Validation { message, .. } => {
            assert_eq!(message, "Command cannot be empty");
        }
    }
}

/// **VALUE**: Verifies that builder validation rejects missing owned flag.
///
/// **WHY THIS MATTERS**: The 'owned' flag determines cleanup behavior.
/// Missing this flag would cause either process leaks or accidental kills of external servers.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Owned flag becomes optional
/// - Default value is incorrectly assumed
/// - Required field validation is bypassed
///
/// This prevents critical process management bugs.
#[test]
fn given_missing_owned_flag_when_building_then_returns_validation_error() {
    // GIVEN: Builder without owned flag
    let builder = ServerInfoBuilder::default()
        .with_pid(12345)
        .with_port(3000)
        .with_base_url("http://localhost:3000")
        .with_name("opencode")
        .with_command("opencode serve");

    // WHEN: Attempting to build
    let result = builder.build();

    // THEN: Should return validation error
    assert!(result.is_err());
    match result.unwrap_err() {
        ModelError::Validation { message, .. } => {
            assert_eq!(message, "Owned is required");
        }
    }
}

/// **VALUE**: Verifies that builder successfully creates ServerInfo with all valid fields.
///
/// **WHY THIS MATTERS**: This is the happy path - ensures valid data can still be created.
/// Regression here would break all server spawning and discovery.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Builder logic is broken by refactoring
/// - Valid data is incorrectly rejected
/// - Field assignments are broken
///
/// This ensures the builder works correctly for valid inputs.
#[test]
fn given_all_valid_fields_when_building_then_returns_server_info() {
    // GIVEN: Builder with all valid fields
    let builder = ServerInfoBuilder::default()
        .with_pid(12345)
        .with_port(3000)
        .with_base_url("https://localhost:3000")
        .with_name("opencode")
        .with_command("opencode serve")
        .with_owned(true);

    // WHEN: Building
    let result = builder.build();

    // THEN: Should succeed and populate all fields correctly
    assert!(result.is_ok());
    let server_info = result.unwrap();
    assert_eq!(server_info.pid, 12345);
    assert_eq!(server_info.port, 3000);
    assert_eq!(server_info.base_url, "https://localhost:3000");
    assert_eq!(server_info.name, "opencode");
    assert_eq!(server_info.command, "opencode serve");
    assert_eq!(server_info.owned, true);
}

/// **VALUE**: Verifies u16 port converts to u32 correctly for protobuf compatibility.
///
/// **WHY THIS MATTERS**: Protobuf uses u32 for ports, but Rust uses u16 for network ports.
/// This conversion must happen correctly to prevent port number corruption.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Port conversion is broken
/// - Type mismatch causes truncation
/// - Maximum port value (65535) fails
///
/// This ensures port conversion works correctly across the boundary.
#[test]
fn given_u16_port_when_building_then_converts_to_u32_correctly() {
    // GIVEN: Builder with maximum u16 port
    let builder = ServerInfoBuilder::default()
        .with_pid(12345)
        .with_port(65535_u16)
        .with_base_url("http://localhost:65535")
        .with_name("opencode")
        .with_command("opencode serve")
        .with_owned(false);

    // WHEN: Building
    let result = builder.build();

    // THEN: Should convert u16 to u32 without loss
    assert!(result.is_ok());
    let server_info = result.unwrap();
    assert_eq!(server_info.port, 65535_u32);
}

/// **VALUE**: Verifies http:// scheme is accepted for base URLs.
///
/// **WHY THIS MATTERS**: Local development servers use http://, not https://.
/// Rejecting http:// would break local development workflow.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - URL validation only allows https://
/// - http:// is accidentally removed from valid schemes
/// - Regex breaks for http:// URLs
///
/// This ensures both http and https schemes work.
#[test]
fn given_http_scheme_when_building_then_accepts_url() {
    // GIVEN: Builder with http:// URL (not https)
    let builder = ServerInfoBuilder::default()
        .with_pid(12345)
        .with_port(3000)
        .with_base_url("http://localhost:3000")
        .with_name("opencode")
        .with_command("opencode serve")
        .with_owned(true);

    // WHEN: Building
    let result = builder.build();

    // THEN: Should accept http:// scheme
    assert!(result.is_ok());
    assert_eq!(result.unwrap().base_url, "http://localhost:3000");
}

/// **VALUE**: Verifies https:// scheme is accepted for base URLs.
///
/// **WHY THIS MATTERS**: Production servers use https:// for security.
/// Rejecting https:// would break production deployments.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - URL validation only allows http://
/// - https:// is accidentally removed from valid schemes
/// - TLS URL handling breaks
///
/// This ensures both http and https schemes work.
#[test]
fn given_https_scheme_when_building_then_accepts_url() {
    // GIVEN: Builder with https:// URL
    let builder = ServerInfoBuilder::default()
        .with_pid(12345)
        .with_port(3000)
        .with_base_url("https://localhost:3000")
        .with_name("opencode")
        .with_command("opencode serve")
        .with_owned(true);

    // WHEN: Building
    let result = builder.build();

    // THEN: Should accept https:// scheme
    assert!(result.is_ok());
    assert_eq!(result.unwrap().base_url, "https://localhost:3000");
}

/// **VALUE**: Verifies owned=false is accepted (external servers).
///
/// **WHY THIS MATTERS**: The owned flag determines if we should kill the server on exit.
/// Both true (spawned) and false (discovered) must work correctly.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Only owned=true is allowed
/// - Boolean validation breaks
/// - External server tracking fails
///
/// This ensures both spawned and discovered servers work.
#[test]
fn given_owned_false_when_building_then_accepts_flag() {
    // GIVEN: Builder with owned=false (discovered server, not spawned)
    let builder = ServerInfoBuilder::default()
        .with_pid(12345)
        .with_port(3000)
        .with_base_url("http://localhost:3000")
        .with_name("opencode")
        .with_command("opencode serve")
        .with_owned(false);

    // WHEN: Building
    let result = builder.build();

    // THEN: Should accept owned=false
    assert!(result.is_ok());
    assert_eq!(result.unwrap().owned, false);
}
