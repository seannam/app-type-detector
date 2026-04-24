//! The JSON wire format must be stable. We construct report-shaped values from
//! the canonical examples in the spec and assert they round-trip through serde
//! without losing any fields.

use app_type_detector::{
    Alternative, AppTypeFinding, Contribution, DetectionReport, Evidence, Fire, GameStack,
    InputSummary, LanguageUsage, LanguagesFinding, Scorecard, TechStack, WebStack,
};
use serde_json::json;

#[test]
fn unity_example_roundtrips() {
    let report = DetectionReport {
        schema_version: 1,
        ruleset_version: "0.1.0".to_string(),
        app_type: AppTypeFinding {
            primary: Some("game".to_string()),
            confidence: 0.97,
            alternatives: vec![],
        },
        tech_stack: TechStack {
            languages: LanguagesFinding {
                primary: Some("csharp".to_string()),
                all: vec![
                    LanguageUsage {
                        language: "csharp".to_string(),
                        role: "gameplay".to_string(),
                        file_count: 142,
                    },
                    LanguageUsage {
                        language: "hlsl".to_string(),
                        role: "shaders".to_string(),
                        file_count: 8,
                    },
                ],
            },
            build_systems: vec!["unity".to_string()],
            package_managers: vec!["nuget".to_string()],
            platforms: vec!["windows".to_string(), "macos".to_string()],
            ci: vec!["github_actions".to_string()],
            game: Some(GameStack {
                engines: vec!["unity".to_string()],
                engine_version: Some("2022.3.42f1".to_string()),
                rendering_pipelines: vec!["urp".to_string()],
                shader_languages: vec!["hlsl".to_string(), "shaderlab".to_string()],
                physics_engines: vec!["physx".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        },
        scorecard: Scorecard {
            rules_evaluated: 142,
            rules_fired: 1,
            elapsed_ms: 3.4,
            input_summary: InputSummary {
                files_scanned: 412,
                bytes_scanned: 1832004,
            },
            ignored_paths: vec![".git".into(), "Library".into()],
            fires: vec![Fire {
                rule_id: "unity-engine".to_string(),
                weight: 1.0,
                evidence: vec![
                    Evidence::FileExists {
                        path: "ProjectSettings/ProjectSettings.asset".to_string(),
                        matched: true,
                    },
                    Evidence::Content {
                        file: "ProjectSettings/ProjectVersion.txt".to_string(),
                        regex: r"m_EditorVersion:\s*(\S+)".to_string(),
                        captures: vec!["2022.3.42f1".to_string()],
                    },
                ],
                contributes_to: vec![Contribution {
                    field: "app_type".to_string(),
                    value: json!("game"),
                    delta: Some(1.0),
                }],
            }],
            warnings: vec![],
        },
    };

    let json = serde_json::to_string(&report).unwrap();
    let parsed: DetectionReport = serde_json::from_str(&json).unwrap();
    assert_eq!(report, parsed);
}

#[test]
fn nextjs_example_roundtrips() {
    let report = DetectionReport {
        schema_version: 1,
        ruleset_version: "0.1.0".to_string(),
        app_type: AppTypeFinding {
            primary: Some("web_app".to_string()),
            confidence: 0.94,
            alternatives: vec![Alternative {
                value: "web_api".to_string(),
                confidence: 0.31,
            }],
        },
        tech_stack: TechStack {
            web: Some(WebStack {
                backend_frameworks: vec!["nextjs".to_string()],
                frontend_frameworks: vec!["react".to_string()],
                css_frameworks: vec!["tailwindcss".to_string()],
                orms: vec!["prisma".to_string()],
                api_styles: vec!["rest".to_string()],
                ssr_strategy: Some("hybrid".to_string()),
                ..Default::default()
            }),
            databases: vec!["postgres".to_string(), "redis".to_string()],
            ..Default::default()
        },
        scorecard: Scorecard::default(),
    };

    let json = serde_json::to_string_pretty(&report).unwrap();
    let parsed: DetectionReport = serde_json::from_str(&json).unwrap();
    assert_eq!(report, parsed);
}

#[test]
fn ambiguous_example_roundtrips() {
    let report = DetectionReport {
        schema_version: 1,
        ruleset_version: "0.1.0".to_string(),
        app_type: AppTypeFinding {
            primary: None,
            confidence: 0.0,
            alternatives: vec![
                Alternative {
                    value: "library".to_string(),
                    confidence: 0.45,
                },
                Alternative {
                    value: "cli_tool".to_string(),
                    confidence: 0.30,
                },
            ],
        },
        tech_stack: TechStack::default(),
        scorecard: Scorecard {
            warnings: vec!["no rule dominated for app_type".to_string()],
            ..Default::default()
        },
    };

    let json = serde_json::to_string(&report).unwrap();
    let parsed: DetectionReport = serde_json::from_str(&json).unwrap();
    assert_eq!(report, parsed);
}

#[test]
fn evidence_variants_roundtrip() {
    let ev = vec![
        Evidence::FileExists {
            path: "a.toml".into(),
            matched: true,
        },
        Evidence::Glob {
            pattern: "**/*.rs".into(),
            matched_count: 17,
        },
        Evidence::Content {
            file: "b.toml".into(),
            regex: "foo".into(),
            captures: vec!["cap".into()],
        },
    ];
    let json = serde_json::to_string(&ev).unwrap();
    let parsed: Vec<Evidence> = serde_json::from_str(&json).unwrap();
    assert_eq!(ev, parsed);
}
