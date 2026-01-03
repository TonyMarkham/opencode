use crate::ErrorLocation;
use std::panic::Location;

/// **VALUE**: Verifies that `ErrorLocation::from()` correctly captures file, line, and column.
///
/// **WHY THIS MATTERS**: ErrorLocation is the foundation of the entire error tracking system.
/// If it fails to capture accurate location data, ALL error messages throughout the codebase
/// lose their debugging value.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - `Location::caller()` stops being propagated correctly
/// - File path extraction breaks
/// - Line/column capture fails
///
/// This is the most critical test in the common crate - everything else depends on this working.
#[test]
#[track_caller]
fn given_location_caller_when_error_location_created_then_captures_file_line_column() {
    // GIVEN: Current caller location
    // WHEN: Creating ErrorLocation from caller
    let location = ErrorLocation::from(Location::caller());

    // THEN: Should capture file, line, and column
    assert!(
        location.file.contains("error_location.rs"),
        "Should capture file path"
    );
    assert_eq!(location.line, 16, "Should capture correct line number");
    assert!(location.column > 0, "Should capture column number");
}

/// **VALUE**: Verifies that ErrorLocation Display formatting produces the expected format.
///
/// **WHY THIS MATTERS**: Error messages are shown to users and developers. If the format breaks,
/// error messages become unreadable or lose critical location information.
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Display implementation changes format (e.g., removes brackets)
/// - File path, line, or column are missing from output
/// - Format is inconsistent (wrong number of colons)
///
/// This ensures error messages consistently show "[file:line:column]" format.
#[test]
#[track_caller]
fn given_error_location_when_formatted_then_produces_bracketed_format() {
    // GIVEN: An ErrorLocation
    let location = ErrorLocation::from(Location::caller());

    // WHEN: Formatting as string
    let formatted = format!("{}", location);

    // THEN: Should produce "[file:line:column]" format
    assert!(formatted.starts_with('['), "Should start with '['");
    assert!(formatted.ends_with(']'), "Should end with ']'");
    assert!(
        formatted.contains("error_location.rs"),
        "Should include filename"
    );
    assert!(
        formatted.contains(&location.line.to_string()),
        "Should include line number"
    );
    assert!(
        formatted.contains(&location.column.to_string()),
        "Should include column number"
    );
    assert_eq!(
        formatted.matches(':').count(),
        2,
        "Should have exactly 2 colons"
    );
}

/// **VALUE**: Verifies that `#[track_caller]` propagation works correctly.
///
/// **WHY THIS MATTERS**: The entire error location system depends on `#[track_caller]`
/// propagating through function calls. If this breaks, all errors will report the wrong
/// location (e.g., always pointing to the error constructor instead of the actual error site).
///
/// **BUG THIS CATCHES**: Would catch if:
/// - Rust's `#[track_caller]` behavior changes
/// - Someone removes `#[track_caller]` from `ErrorLocation::from()`
/// - Location propagation breaks in refactoring
///
/// This test proves that different call sites get different line numbers, which is essential
/// for accurate error tracking across the codebase.
#[test]
fn given_multiple_call_sites_when_capturing_location_then_each_has_unique_line() {
    // GIVEN: A helper function that captures location
    #[track_caller]
    fn capture_location() -> ErrorLocation {
        ErrorLocation::from(Location::caller())
    }

    // WHEN: Capturing location from different call sites
    let loc1 = capture_location();
    let loc2 = capture_location();

    // THEN: Should have same file but different line numbers
    assert_eq!(loc1.file, loc2.file, "Should have same file");
    assert_ne!(loc1.line, loc2.line, "Should have different line numbers");
    assert_eq!(loc1.line + 1, loc2.line, "Lines should be sequential");
}
