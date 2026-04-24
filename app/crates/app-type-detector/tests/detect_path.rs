//! Integration tests: run `detect_path` against every committed fixture and
//! assert the headline findings. These tests are the canonical expectations
//! for the detector.

use app_type_detector::detect_path;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn unity_game_fixture() {
    let report = detect_path(fixture("unity-game")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("game"));
    let game = report.tech_stack.game.expect("game stack populated");
    assert_eq!(game.engines, vec!["unity"]);
    assert_eq!(game.engine_version.as_deref(), Some("2022.3.42f1"));
    assert!(game.rendering_pipelines.contains(&"urp".to_string()));
    assert!(game.shader_languages.contains(&"hlsl".to_string()));
    assert!(game.shader_languages.contains(&"shaderlab".to_string()));
    assert!(report.tech_stack.ci.contains(&"github_actions".to_string()));
    assert!(report
        .tech_stack
        .build_systems
        .contains(&"unity".to_string()));
    assert_eq!(
        report.tech_stack.languages.primary.as_deref(),
        Some("csharp")
    );
}

#[test]
fn godot_game_fixture() {
    let report = detect_path(fixture("godot-game")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("game"));
    let game = report.tech_stack.game.expect("game stack populated");
    assert_eq!(game.engines, vec!["godot"]);
    assert_eq!(
        report.tech_stack.languages.primary.as_deref(),
        Some("gdscript")
    );
}

#[test]
fn bevy_game_fixture() {
    let report = detect_path(fixture("bevy-game")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("game"));
    let game = report.tech_stack.game.expect("game stack populated");
    assert_eq!(game.engines, vec!["bevy"]);
    assert_eq!(report.tech_stack.languages.primary.as_deref(), Some("rust"));
}

#[test]
fn nextjs_postgres_fixture() {
    let report = detect_path(fixture("nextjs-postgres-saas")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("web_app"));
    let web = report.tech_stack.web.expect("web stack populated");
    assert!(web.backend_frameworks.contains(&"nextjs".to_string()));
    assert!(web.frontend_frameworks.contains(&"react".to_string()));
    assert!(web.css_frameworks.contains(&"tailwindcss".to_string()));
    assert!(web.orms.contains(&"prisma".to_string()));
    assert!(report
        .tech_stack
        .databases
        .contains(&"postgres".to_string()));
    assert!(report.tech_stack.databases.contains(&"redis".to_string()));
    assert!(report
        .tech_stack
        .payment_processors
        .contains(&"stripe".to_string()));
    assert!(report.tech_stack.testing.contains(&"vitest".to_string()));
    assert!(report.tech_stack.linting.contains(&"eslint".to_string()));
    assert!(report.tech_stack.ci.contains(&"github_actions".to_string()));
    assert!(report
        .tech_stack
        .containerization
        .contains(&"docker".to_string()));
}

#[test]
fn astro_static_site_fixture() {
    let report = detect_path(fixture("astro-static-site")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("static_site"));
    let web = report.tech_stack.web.expect("web stack populated");
    assert!(web.backend_frameworks.contains(&"astro".to_string()));
}

#[test]
fn fastapi_api_fixture() {
    let report = detect_path(fixture("fastapi-api")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("web_api"));
    let web = report.tech_stack.web.expect("web stack populated");
    assert!(web.backend_frameworks.contains(&"fastapi".to_string()));
    assert!(report.tech_stack.testing.contains(&"pytest".to_string()));
    assert!(report.tech_stack.linting.contains(&"ruff".to_string()));
}

#[test]
fn swiftui_ios_fixture() {
    let report = detect_path(fixture("swiftui-ios-app")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("mobile_app"));
    let mobile = report.tech_stack.mobile.expect("mobile stack populated");
    assert!(mobile.ui_frameworks.contains(&"swiftui".to_string()));
    assert!(report.tech_stack.platforms.contains(&"ios".to_string()));
}

#[test]
fn kotlin_android_fixture() {
    let report = detect_path(fixture("kotlin-android-app")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("mobile_app"));
    let mobile = report.tech_stack.mobile.expect("mobile stack populated");
    assert!(mobile
        .ui_frameworks
        .contains(&"jetpack_compose".to_string()));
    assert!(report.tech_stack.platforms.contains(&"android".to_string()));
}

#[test]
fn tauri_desktop_fixture() {
    let report = detect_path(fixture("tauri-desktop-app")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("desktop_app"));
    let desktop = report.tech_stack.desktop.expect("desktop stack populated");
    assert!(desktop.shells.contains(&"tauri".to_string()));
}

#[test]
fn electron_desktop_fixture() {
    let report = detect_path(fixture("electron-desktop-app")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("desktop_app"));
    let desktop = report.tech_stack.desktop.expect("desktop stack populated");
    assert!(desktop.shells.contains(&"electron".to_string()));
}

#[test]
fn cli_rust_fixture() {
    let report = detect_path(fixture("cli-rust")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("cli_tool"));
    assert_eq!(report.tech_stack.languages.primary.as_deref(), Some("rust"));
}

#[test]
fn library_rust_fixture() {
    let report = detect_path(fixture("library-rust")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("library"));
    assert_eq!(report.tech_stack.languages.primary.as_deref(), Some("rust"));
}

#[test]
fn mcp_server_typescript_fixture() {
    let report = detect_path(fixture("mcp-server-typescript")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("mcp_server"));
    let ext = report
        .tech_stack
        .extension
        .expect("extension stack populated");
    assert_eq!(ext.host.as_deref(), Some("mcp_client"));
}

#[test]
fn claude_skill_fixture() {
    let report = detect_path(fixture("claude-skill")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("claude_skill"));
    let ext = report
        .tech_stack
        .extension
        .expect("extension stack populated");
    assert_eq!(ext.host.as_deref(), Some("claude_code"));
    assert_eq!(ext.kind.as_deref(), Some("skill"));
}

#[test]
fn chrome_extension_fixture() {
    let report = detect_path(fixture("chrome-extension")).unwrap();
    assert_eq!(
        report.app_type.primary.as_deref(),
        Some("browser_extension")
    );
    let ext = report
        .tech_stack
        .extension
        .expect("extension stack populated");
    assert_eq!(ext.host.as_deref(), Some("chrome"));
}

#[test]
fn bun_fastify_api_fixture() {
    let report = detect_path(fixture("bun-fastify-api")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("web_api"));
    let web = report.tech_stack.web.expect("web stack populated");
    assert!(web.backend_frameworks.contains(&"fastify".to_string()));
    assert_eq!(
        report.tech_stack.languages.primary.as_deref(),
        Some("typescript")
    );
    assert!(report.tech_stack.runtimes.contains(&"bun".to_string()));
    assert!(report.tech_stack.runtimes.contains(&"node".to_string()));
}

#[test]
fn express_api_fixture() {
    let report = detect_path(fixture("express-api")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("web_api"));
    let web = report.tech_stack.web.expect("web stack populated");
    assert!(web.backend_frameworks.contains(&"express".to_string()));
    assert_eq!(
        report.tech_stack.languages.primary.as_deref(),
        Some("javascript")
    );
}

#[test]
fn flask_api_fixture() {
    let report = detect_path(fixture("flask-api")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("web_api"));
    let web = report.tech_stack.web.expect("web stack populated");
    assert!(web.backend_frameworks.contains(&"flask".to_string()));
    assert_eq!(
        report.tech_stack.languages.primary.as_deref(),
        Some("python")
    );
}

#[test]
fn django_app_fixture() {
    let report = detect_path(fixture("django-app")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("web_app"));
    let web = report.tech_stack.web.expect("web stack populated");
    assert!(web.backend_frameworks.contains(&"django".to_string()));
    assert_eq!(
        report.tech_stack.languages.primary.as_deref(),
        Some("python")
    );
}

#[test]
fn python_telegram_bot_fixture() {
    let report = detect_path(fixture("python-telegram-bot")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("daemon"));
    assert!(report
        .tech_stack
        .frameworks
        .contains(&"python_telegram_bot".to_string()));
    assert_eq!(
        report.tech_stack.languages.primary.as_deref(),
        Some("python")
    );
}

#[test]
fn cargo_workspace_fixture() {
    let report = detect_path(fixture("cargo-workspace")).unwrap();
    assert_eq!(report.app_type.primary.as_deref(), Some("library"));
    assert_eq!(report.tech_stack.languages.primary.as_deref(), Some("rust"));
    assert!(report
        .tech_stack
        .build_systems
        .contains(&"cargo".to_string()));
}

#[test]
fn python_unknown_framework_fixture() {
    // Baseline rules should still populate the language even when no framework
    // rule matches, but must not claim an app_type.
    let report = detect_path(fixture("python-unknown-framework")).unwrap();
    assert!(report.app_type.primary.is_none());
    assert_eq!(
        report.tech_stack.languages.primary.as_deref(),
        Some("python")
    );
}

#[test]
fn empty_dir_fixture() {
    let report = detect_path(fixture("empty-dir")).unwrap();
    assert!(report.app_type.primary.is_none());
    assert!(!report.scorecard.warnings.is_empty());
}

#[test]
fn git_only_dir_fixture() {
    let report = detect_path(fixture("git-only-dir")).unwrap();
    assert!(report.app_type.primary.is_none());
}

#[test]
fn missing_path_errors() {
    assert!(detect_path(fixture("this-does-not-exist-xyz")).is_err());
}

#[test]
fn report_round_trips_through_json() {
    let report = detect_path(fixture("unity-game")).unwrap();
    let json = report.to_json();
    let roundtripped: app_type_detector::DetectionReport = serde_json::from_str(&json).unwrap();
    assert_eq!(report, roundtripped);
}
