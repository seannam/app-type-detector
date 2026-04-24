//! Compile-time validation of the bundled default ruleset.
//!
//! - The JSON parses.
//! - Every rule id is unique.
//! - Every `AppType` value we claim to ship is referenced by at least one rule.

use std::collections::HashSet;

#[derive(serde::Deserialize)]
struct ValidationRuleset {
    schema_version: u32,
    rules: Vec<ValidationRule>,
}

#[derive(serde::Deserialize)]
struct ValidationRule {
    id: String,
    payload: ValidationPayload,
}

#[derive(serde::Deserialize)]
struct ValidationPayload {
    contributions: Vec<ValidationContribution>,
}

#[derive(serde::Deserialize)]
struct ValidationContribution {
    field: String,
    value: serde_json::Value,
}

const REQUIRED_APP_TYPES: &[&str] = &[
    "web_app",
    "web_api",
    "static_site",
    "mobile_app",
    "desktop_app",
    "game",
    "cli_tool",
    "library",
    "mcp_server",
    "claude_skill",
    "browser_extension",
    "editor_extension",
];

fn main() {
    println!("cargo:rerun-if-changed=src/default_rules.json");
    let raw = include_str!("src/default_rules.json");
    let parsed: ValidationRuleset =
        serde_json::from_str(raw).expect("default_rules.json must parse");
    assert_eq!(parsed.schema_version, 1, "schema_version must be 1");

    let mut ids = HashSet::new();
    let mut app_types = HashSet::new();
    for rule in &parsed.rules {
        assert!(
            ids.insert(rule.id.clone()),
            "duplicate rule id: {}",
            rule.id
        );
        for c in &rule.payload.contributions {
            if c.field == "app_type" {
                if let Some(s) = c.value.as_str() {
                    app_types.insert(s.to_string());
                }
            }
        }
    }
    for required in REQUIRED_APP_TYPES {
        assert!(
            app_types.contains(*required),
            "no rule covers required app_type `{}`",
            required
        );
    }
}
