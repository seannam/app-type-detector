//! Scorecard invariants: every fire's contributions must name a real field
//! path, regex captures round-trip, and warnings fire for empty inputs.

use app_type_detector::{detect_path, Contribution, Evidence};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn is_known_field(field: &str) -> bool {
    matches!(
        field,
        "app_type"
            | "tech_stack.languages"
            | "tech_stack.languages.primary"
            | "tech_stack.languages.detail"
            | "tech_stack.build_systems"
            | "tech_stack.package_managers"
            | "tech_stack.frameworks"
            | "tech_stack.runtimes"
            | "tech_stack.platforms"
            | "tech_stack.databases"
            | "tech_stack.caches"
            | "tech_stack.queues"
            | "tech_stack.storage"
            | "tech_stack.testing"
            | "tech_stack.linting"
            | "tech_stack.formatting"
            | "tech_stack.ci"
            | "tech_stack.containerization"
            | "tech_stack.orchestration"
            | "tech_stack.iac"
            | "tech_stack.observability"
            | "tech_stack.auth_providers"
            | "tech_stack.payment_processors"
            | "tech_stack.web.backend_frameworks"
            | "tech_stack.web.frontend_frameworks"
            | "tech_stack.web.css_frameworks"
            | "tech_stack.web.bundlers"
            | "tech_stack.web.ssr_strategy"
            | "tech_stack.web.api_styles"
            | "tech_stack.web.orms"
            | "tech_stack.mobile.ui_frameworks"
            | "tech_stack.mobile.notable_sdks"
            | "tech_stack.desktop.shells"
            | "tech_stack.desktop.installer_formats"
            | "tech_stack.game.engines"
            | "tech_stack.game.engine_version"
            | "tech_stack.game.rendering_pipelines"
            | "tech_stack.game.shader_languages"
            | "tech_stack.game.physics_engines"
            | "tech_stack.game.networking"
            | "tech_stack.extension.host"
            | "tech_stack.extension.kind"
    )
}

fn assert_invariants(fixture_name: &str, allow_empty: bool) {
    let report = detect_path(fixture(fixture_name)).unwrap();
    if !allow_empty {
        assert!(
            !report.scorecard.fires.is_empty(),
            "expected at least one fire for {fixture_name}"
        );
    }
    for fire in &report.scorecard.fires {
        assert!(
            fire.weight > 0.0,
            "rule {} had non-positive weight",
            fire.rule_id
        );
        for c in &fire.contributes_to {
            let Contribution { field, .. } = c;
            assert!(
                is_known_field(field),
                "rule {} contributes to unknown field `{}`",
                fire.rule_id,
                field
            );
        }
        for e in &fire.evidence {
            if let Evidence::Content { regex, .. } = e {
                assert!(!regex.is_empty(), "empty regex in rule {}", fire.rule_id);
            }
        }
    }
    if report.scorecard.fires.is_empty() {
        assert!(
            !report.scorecard.warnings.is_empty(),
            "empty fires must carry a warning for {fixture_name}"
        );
    }
}

#[test]
fn scorecard_unity() {
    assert_invariants("unity-game", false);
}

#[test]
fn scorecard_nextjs() {
    assert_invariants("nextjs-postgres-saas", false);
}

#[test]
fn scorecard_empty() {
    assert_invariants("empty-dir", true);
}

#[test]
fn scorecard_claude_skill() {
    assert_invariants("claude-skill", false);
}

#[test]
fn captured_engine_version_reaches_game_stack() {
    let report = detect_path(fixture("unity-game")).unwrap();
    let version = report
        .tech_stack
        .game
        .and_then(|g| g.engine_version)
        .expect("engine version captured");
    assert_eq!(version, "2022.3.42f1");
}
