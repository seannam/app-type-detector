//! Edge cases: empty dirs, missing paths, and content rules that silently skip
//! unreadable files.

use app_type_detector::{detect_files, detect_path, MemorySnapshot};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn empty_directory_returns_null_primary_and_warning() {
    let report = detect_path(fixture("empty-dir")).unwrap();
    assert!(report.app_type.primary.is_none());
    assert!(report
        .scorecard
        .warnings
        .iter()
        .any(|w| w.contains("no rules fired")));
}

#[test]
fn git_only_directory_is_treated_as_empty() {
    let report = detect_path(fixture("git-only-dir")).unwrap();
    assert!(report.app_type.primary.is_none());
}

#[test]
fn missing_path_returns_err() {
    assert!(detect_path("/tmp/this-should-not-exist-pls-123abc").is_err());
}

#[test]
fn memory_snapshot_with_none_contents_does_not_crash() {
    let snap = MemorySnapshot::new().with_empty("Cargo.toml");
    let report = detect_files(&snap);
    // Content rules can't match (no content), but file-exists rules can.
    assert!(!report.scorecard.fires.is_empty() || !report.scorecard.warnings.is_empty());
}

#[test]
fn broken_utf8_in_content_skips_cleanly() {
    // We can't easily put raw bytes in a tracked fixture file, but we can
    // exercise the path: a file exists but its contents are not returned.
    let snap = MemorySnapshot::new().with_empty("pyproject.toml");
    let report = detect_files(&snap);
    // FastAPI rule should NOT fire since there is no content to match.
    assert!(report
        .scorecard
        .fires
        .iter()
        .all(|f| f.rule_id != "fastapi-api"));
}
