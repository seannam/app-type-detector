//! Parity: the same fixtures fed through `MemorySnapshot` must produce
//! byte-identical reports to `detect_path`, modulo volatile fields
//! (elapsed_ms, file sizes, file counts — these depend on FS traversal).

use std::fs;
use std::path::Path;

use app_type_detector::{detect_files, detect_path, MemorySnapshot};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn load_memory(root: &Path) -> MemorySnapshot {
    let mut snap = MemorySnapshot::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            !matches!(name.as_str(), ".git" | "node_modules" | "target")
        })
    {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = match entry.path().strip_prefix(root) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        let contents = fs::read_to_string(entry.path()).unwrap_or_default();
        snap = snap.with_file(rel, contents);
    }
    snap
}

fn assert_parity(fixture_name: &str) {
    let path = fixture(fixture_name);
    let fs_report = detect_path(&path).unwrap();
    let mem_snap = load_memory(&path);
    let mem_report = detect_files(&mem_snap);
    assert_eq!(
        fs_report.app_type, mem_report.app_type,
        "app_type mismatch for {fixture_name}"
    );
    assert_eq!(
        fs_report.tech_stack, mem_report.tech_stack,
        "tech_stack mismatch for {fixture_name}"
    );
    assert_eq!(
        fs_report.scorecard.rules_fired, mem_report.scorecard.rules_fired,
        "fires_count mismatch for {fixture_name}"
    );
}

#[test]
fn parity_unity() {
    assert_parity("unity-game");
}

#[test]
fn parity_godot() {
    assert_parity("godot-game");
}

#[test]
fn parity_bevy() {
    assert_parity("bevy-game");
}

#[test]
fn parity_nextjs() {
    assert_parity("nextjs-postgres-saas");
}

#[test]
fn parity_astro() {
    assert_parity("astro-static-site");
}

#[test]
fn parity_fastapi() {
    assert_parity("fastapi-api");
}

#[test]
fn parity_swiftui_ios() {
    assert_parity("swiftui-ios-app");
}

#[test]
fn parity_android() {
    assert_parity("kotlin-android-app");
}

#[test]
fn parity_tauri() {
    assert_parity("tauri-desktop-app");
}

#[test]
fn parity_electron() {
    assert_parity("electron-desktop-app");
}

#[test]
fn parity_cli_rust() {
    assert_parity("cli-rust");
}

#[test]
fn parity_library_rust() {
    assert_parity("library-rust");
}

#[test]
fn parity_mcp() {
    assert_parity("mcp-server-typescript");
}

#[test]
fn parity_claude_skill() {
    assert_parity("claude-skill");
}

#[test]
fn parity_chrome_extension() {
    assert_parity("chrome-extension");
}

#[test]
fn parity_polyglot() {
    assert_parity("polyglot-monorepo");
}
