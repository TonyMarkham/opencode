// Unit tests for spawn module private functions
// Integration tests for public API are in integration_tests/discovery/spawn.rs

use crate::OPENCODE_BINARY;
use crate::discovery::spawn::{build_spawn_command, get_url_regex};

/// **VALUE**: Verifies that `build_spawn_command()` constructs commands with the correct binary name.
///
/// **WHY THIS MATTERS**: If someone refactors `build_spawn_command()` and accidentally changes
/// the binary name or breaks the command construction, spawn will fail silently.
///
/// **BUG THIS CATCHES**: Would catch regressions where the command is built with the wrong
/// executable name (e.g., "node" instead of "opencode"), or if Command construction breaks.
#[test]
fn given_port_arg_when_build_spawn_command_called_then_sets_correct_binary() {
    // GIVEN: A port argument
    let port = "4096";

    // WHEN: Building the spawn command
    let cmd = build_spawn_command(port);

    // THEN: Should use the correct binary name
    let program = cmd.as_std().get_program();
    assert_eq!(program, OPENCODE_BINARY, "Should use correct binary name");
}

/// **VALUE**: Tests that the URL parsing regex correctly matches valid server output.
///
/// **WHY THIS MATTERS**: If the regex pattern breaks, spawn will never find the server URL
/// in stdout, causing all spawn attempts to timeout even when the server starts successfully.
///
/// **BUG THIS CATCHES**: Would catch if someone changes the regex pattern and breaks the
/// capture group names ("host" and "port"), or if the pattern no longer matches valid URLs.
#[test]
fn given_valid_server_url_when_regex_applied_then_matches_and_extracts_parts() {
    // GIVEN: A valid server URL in stdout output
    let re = get_url_regex();
    let test_line = "Server listening on http://127.0.0.1:4096";

    // WHEN: Applying the regex
    let caps = re.captures(test_line);

    // THEN: Should match and extract host and port
    assert!(caps.is_some(), "Regex should match valid URL");
    let caps = caps.unwrap();
    assert_eq!(caps.name("host").unwrap().as_str(), "127.0.0.1");
    assert_eq!(caps.name("port").unwrap().as_str(), "4096");
}

/// **VALUE**: Tests that the URL regex rejects malformed/invalid URLs.
///
/// **WHY THIS MATTERS**: If the regex is too permissive, it might match garbage output
/// and extract invalid ports/hosts, causing connection failures with confusing error messages.
///
/// **BUG THIS CATCHES**: Would catch if someone makes the regex too greedy and it starts
/// matching non-URL strings, leading to parse errors or failed connections.
#[test]
fn given_invalid_urls_when_regex_applied_then_does_not_match() {
    // GIVEN: Various invalid URL formats
    let re = get_url_regex();
    let invalid_cases = vec![
        "not a url at all",
        "http://",
        "http://localhost",     // no port
        "localhost:4096",       // no protocol
        "ftp://127.0.0.1:4096", // wrong protocol
    ];

    // WHEN: Applying regex to invalid URLs
    // THEN: Should not match any of them
    for invalid in invalid_cases {
        assert!(
            re.captures(invalid).is_none(),
            "Regex should not match: {invalid}"
        );
    }
}

/// **VALUE**: Tests that the URL regex correctly extracts port numbers across the valid range.
///
/// **WHY THIS MATTERS**: Different port numbers (especially edge cases like 1, 80, 65535)
/// need to parse correctly. If the regex breaks for certain port ranges, spawning will fail.
///
/// **BUG THIS CATCHES**: Would catch if the port capture group regex is too restrictive
/// (e.g., only matches 4-digit ports) and fails for valid ports like 80 or 8080.
#[test]
fn given_various_port_numbers_when_regex_applied_then_extracts_correctly() {
    // GIVEN: URLs with different port numbers
    let re = get_url_regex();
    let test_cases = vec![
        ("http://127.0.0.1:80", "80"),
        ("http://127.0.0.1:8080", "8080"),
        ("http://127.0.0.1:65535", "65535"),
        ("http://127.0.0.1:1", "1"),
    ];

    // WHEN: Applying regex to each URL
    // THEN: Should extract the correct port number
    for (line, expected_port) in test_cases {
        let caps = re.captures(line);
        assert!(caps.is_some(), "Should match: {line}");
        assert_eq!(
            caps.unwrap().name("port").unwrap().as_str(),
            expected_port,
            "Should extract correct port from: {line}"
        );
    }
}
