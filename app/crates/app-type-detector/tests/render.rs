//! Render tests: validate `to_human_readable()` produces output with the
//! shape documented in the spec. We avoid byte-exact golden files here since
//! the renderer is still evolving; we check for key structural lines.

use app_type_detector::{detect_path, render_human_readable};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn unity_render_contains_expected_sections() {
    let report = detect_path(fixture("unity-game")).unwrap();
    let text = render_human_readable(&report);
    assert!(text.contains("App Type"), "missing App Type header");
    assert!(text.contains("Tech Stack"), "missing Tech Stack header");
    assert!(text.contains("Game"), "missing Game sub-record");
    assert!(text.contains("Unity"), "missing pretty-printed engine");
    assert!(text.contains("Scorecard"), "missing scorecard footer");
    assert!(text.contains("unity-engine"), "missing fire rule id");
}

#[test]
fn nextjs_render_contains_web_sub_record() {
    let report = detect_path(fixture("nextjs-postgres-saas")).unwrap();
    let text = render_human_readable(&report);
    assert!(text.contains("Web"), "missing Web sub-record");
    assert!(text.contains("Next.js"), "missing Next.js label");
    assert!(text.contains("Tailwind CSS"), "missing Tailwind CSS label");
    assert!(text.contains("Prisma"), "missing Prisma label");
}

#[test]
fn empty_render_reports_ambiguity() {
    let report = detect_path(fixture("empty-dir")).unwrap();
    let text = render_human_readable(&report);
    assert!(
        text.contains("unable to determine") || text.contains("no rules fired"),
        "empty render should surface ambiguity"
    );
}
