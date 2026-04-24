use assert_cmd::Command;
use predicates::str::contains;

fn fixture(name: &str) -> std::path::PathBuf {
    // The CLI crate is a sibling of the core crate; find the fixtures relative
    // to the manifest dir.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .join("app-type-detector/tests/fixtures")
        .join(name)
}

#[test]
fn detect_json_on_unity_fixture() {
    let mut cmd = Command::cargo_bin("app-type-detector").unwrap();
    cmd.args(["detect", "--format", "json"])
        .arg(fixture("unity-game"));
    cmd.assert()
        .success()
        .stdout(contains("\"primary\": \"game\""))
        .stdout(contains("\"unity\""));
}

#[test]
fn detect_text_on_nextjs_fixture() {
    let mut cmd = Command::cargo_bin("app-type-detector").unwrap();
    cmd.args(["detect", "--format", "text"])
        .arg(fixture("nextjs-postgres-saas"));
    cmd.assert()
        .success()
        .stdout(contains("Web"))
        .stdout(contains("Next.js"));
}

#[test]
fn detect_tsv_on_unity_fixture() {
    let mut cmd = Command::cargo_bin("app-type-detector").unwrap();
    cmd.args(["detect", "--format", "tsv"])
        .arg(fixture("unity-game"));
    cmd.assert()
        .success()
        .stdout(contains("game\tcsharp\tunity"));
}

#[test]
fn detect_fires_jsonl_on_unity_fixture() {
    let mut cmd = Command::cargo_bin("app-type-detector").unwrap();
    cmd.args(["detect", "--format", "fires-jsonl"])
        .arg(fixture("unity-game"));
    cmd.assert()
        .success()
        .stdout(contains("\"rule_id\":\"unity-engine\""));
}

#[test]
fn detect_no_evidence_strips_evidence() {
    let mut cmd = Command::cargo_bin("app-type-detector").unwrap();
    cmd.args(["detect", "--format", "fires-jsonl", "--no-evidence"])
        .arg(fixture("unity-game"));
    cmd.assert().success().stdout(contains("\"evidence\":[]"));
}

#[test]
fn missing_path_errors_nonzero() {
    let mut cmd = Command::cargo_bin("app-type-detector").unwrap();
    cmd.args(["detect", "--format", "json", "/tmp/nope-xyz-123"]);
    cmd.assert().failure();
}
