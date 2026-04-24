#![allow(missing_docs)]

//! Combine `Vec<Fire>` into a [`DetectionReport`].
//!
//! The engine emits what each rule *claimed*. The synthesizer is the only place
//! that decides which claims win:
//!
//! - For list fields, every contributed value is unioned and deduped, ordered
//!   by total contributing weight.
//! - For scalar fields (`engine_version`, `ssr_strategy`, `languages.primary`),
//!   the value with the highest weight wins; ties break alphabetically by
//!   rule id.
//! - For `app_type.primary`, the top candidate must exceed the runner-up by at
//!   least `dominance_margin` (default 1.5x); otherwise `primary` is `None`
//!   and a warning is emitted.

use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use crate::rules::Ruleset;
use crate::snapshot::InputSnapshot;
use crate::types::report::{Alternative, AppTypeFinding, DetectionReport, SCHEMA_VERSION};
use crate::types::scorecard::{Fire, InputSummary, Scorecard};
use crate::types::tech_stack::{
    DesktopStack, ExtensionStack, GameStack, LanguageUsage, MobileStack, TechStack, WebStack,
};

/// Tuning knobs for the synthesizer.
#[derive(Debug, Clone)]
pub struct SynthesisConfig {
    pub dominance_margin: f32,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        Self {
            dominance_margin: 1.5,
        }
    }
}

pub fn synthesize(
    fires: Vec<Fire>,
    rules_evaluated: u32,
    elapsed_ms: f64,
    input_summary: InputSummary,
    ignored_paths: Vec<String>,
    ruleset_version: &str,
    config: &SynthesisConfig,
) -> DetectionReport {
    let mut warnings: Vec<String> = Vec::new();

    // Tally app_type candidates by total weight.
    let mut app_type_weights: HashMap<String, f32> = HashMap::new();
    // Tally contributions to every other field path.
    let mut list_field_votes: HashMap<String, BTreeMap<String, f32>> = HashMap::new();
    let mut scalar_field_votes: HashMap<String, Vec<(String, f32, String)>> = HashMap::new();

    for fire in &fires {
        for contribution in &fire.contributes_to {
            if contribution.field == "app_type" {
                if let Some(s) = contribution.value.as_str() {
                    let w = contribution.delta.unwrap_or(fire.weight);
                    *app_type_weights.entry(s.to_string()).or_insert(0.0) += w;
                }
            } else if is_scalar_field(&contribution.field) {
                let v = contribution_value_as_string(&contribution.value);
                let entry = scalar_field_votes
                    .entry(contribution.field.clone())
                    .or_default();
                entry.push((v, fire.weight, fire.rule_id.clone()));
            } else {
                let v = contribution_value_as_string(&contribution.value);
                let entry = list_field_votes
                    .entry(contribution.field.clone())
                    .or_default();
                *entry.entry(v).or_insert(0.0) += fire.weight;
            }
        }
    }

    // Compute app_type primary and alternatives.
    let mut ranked_app_types: Vec<(String, f32)> = app_type_weights.into_iter().collect();
    ranked_app_types.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    let (primary_app, confidence, alternatives) = match ranked_app_types.as_slice() {
        [] => (None, 0.0, vec![]),
        [only] => (Some(only.0.clone()), normalize_confidence(only.1), vec![]),
        [top, runner, rest @ ..] => {
            let dominates = if runner.1 <= f32::EPSILON {
                true
            } else {
                (top.1 / runner.1) >= config.dominance_margin
            };
            let mut alts: Vec<Alternative> = std::iter::once(runner.clone())
                .chain(rest.iter().cloned())
                .map(|(v, w)| Alternative {
                    value: v,
                    confidence: normalize_confidence(w),
                })
                .collect();
            if !dominates {
                warnings.push(format!(
                    "no rule dominated for app_type (top two within {:.2}x weight margin); leaving primary as null",
                    config.dominance_margin
                ));
                // Push the top too, since primary is null.
                alts.insert(
                    0,
                    Alternative {
                        value: top.0.clone(),
                        confidence: normalize_confidence(top.1),
                    },
                );
                (None, 0.0, alts)
            } else {
                (Some(top.0.clone()), normalize_confidence(top.1), alts)
            }
        }
    };

    // Build tech_stack from votes.
    let mut tech_stack = TechStack::default();
    apply_list_votes(&mut tech_stack, &list_field_votes);
    apply_scalar_votes(&mut tech_stack, &scalar_field_votes);

    // Derive languages.all from file-count contributions if any came through
    // a special `_file_count` meta field. Otherwise leave empty.
    finalize_languages(&mut tech_stack, &list_field_votes);

    // Warnings for empty state.
    if fires.is_empty() {
        warnings.push("no rules fired; input may be empty or out of vocabulary".to_string());
    }

    let scorecard = Scorecard {
        rules_evaluated,
        rules_fired: fires.len() as u32,
        elapsed_ms,
        input_summary,
        ignored_paths,
        fires,
        warnings,
    };

    let app_type = AppTypeFinding {
        primary: primary_app,
        confidence,
        alternatives,
    };

    DetectionReport {
        schema_version: SCHEMA_VERSION,
        ruleset_version: ruleset_version.to_string(),
        app_type,
        tech_stack,
        scorecard,
    }
}

fn normalize_confidence(raw: f32) -> f32 {
    // Squash total weight into [0, 1] using 1 - e^(-w). This way a single
    // strong rule reads as ~63% but two strong rules push confidence towards 1.
    let conf = 1.0 - (-raw).exp();
    (conf * 100.0).round() / 100.0
}

fn contribution_value_as_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => value.to_string(),
    }
}

fn is_scalar_field(field: &str) -> bool {
    matches!(
        field,
        "tech_stack.languages.primary"
            | "tech_stack.game.engine_version"
            | "tech_stack.web.ssr_strategy"
            | "tech_stack.extension.host"
            | "tech_stack.extension.kind"
    )
}

fn apply_list_votes(tech_stack: &mut TechStack, votes: &HashMap<String, BTreeMap<String, f32>>) {
    for (field, value_weights) in votes {
        let mut ordered: Vec<(String, f32)> =
            value_weights.iter().map(|(k, v)| (k.clone(), *v)).collect();
        ordered.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        let values: Vec<String> = ordered.into_iter().map(|(v, _)| v).collect();
        set_list_field(tech_stack, field, values);
    }
}

fn apply_scalar_votes(
    tech_stack: &mut TechStack,
    votes: &HashMap<String, Vec<(String, f32, String)>>,
) {
    for (field, candidates) in votes {
        // Reduce to per-value weight, then pick the max.
        let mut grouped: BTreeMap<String, (f32, String)> = BTreeMap::new();
        for (v, w, rid) in candidates {
            let e = grouped.entry(v.clone()).or_insert((0.0, rid.clone()));
            e.0 += *w;
            if rid < &e.1 {
                e.1 = rid.clone();
            }
        }
        let mut ranked: Vec<(String, f32, String)> = grouped
            .into_iter()
            .map(|(v, (w, rid))| (v, w, rid))
            .collect();
        ranked.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.cmp(&b.2))
        });
        if let Some((winner, _, _)) = ranked.into_iter().next() {
            set_scalar_field(tech_stack, field, winner);
        }
    }
}

fn finalize_languages(
    tech_stack: &mut TechStack,
    list_votes: &HashMap<String, BTreeMap<String, f32>>,
) {
    // Build `languages.all` from synthetic field `tech_stack.languages.detail`
    // contributions, which encode `lang|role|count` strings.
    if let Some(details) = list_votes.get("tech_stack.languages.detail") {
        let mut all = Vec::new();
        for key in details.keys() {
            let parts: Vec<&str> = key.splitn(3, '|').collect();
            if parts.len() == 3 {
                let count: u64 = parts[2].parse().unwrap_or(0);
                all.push(LanguageUsage {
                    language: parts[0].to_string(),
                    role: parts[1].to_string(),
                    file_count: count,
                });
            }
        }
        all.sort_by(|a, b| {
            b.file_count
                .cmp(&a.file_count)
                .then(a.language.cmp(&b.language))
        });
        tech_stack.languages.all = all;
    }

    // Promote first tech_stack.languages entry into languages.primary if not
    // set explicitly.
    if tech_stack.languages.primary.is_none() && !tech_stack.languages.all.is_empty() {
        tech_stack.languages.primary = Some(tech_stack.languages.all[0].language.clone());
    } else if tech_stack.languages.primary.is_none() {
        // Fall back to the first languages list value if it exists as a list
        // contribution.
        if let Some(langs) = list_votes.get("tech_stack.languages") {
            let mut ranked: Vec<(String, f32)> =
                langs.iter().map(|(k, v)| (k.clone(), *v)).collect();
            ranked.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });
            if let Some((top, _)) = ranked.first() {
                tech_stack.languages.primary = Some(top.clone());
            }
        }
    }

    // Populate `languages.all` from `tech_stack.languages` list if `all` is
    // empty and no details were provided.
    if tech_stack.languages.all.is_empty() {
        if let Some(langs) = list_votes.get("tech_stack.languages") {
            let mut ranked: Vec<(String, f32)> =
                langs.iter().map(|(k, v)| (k.clone(), *v)).collect();
            ranked.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });
            tech_stack.languages.all = ranked
                .into_iter()
                .map(|(lang, _)| LanguageUsage {
                    language: lang,
                    role: "application".to_string(),
                    file_count: 0,
                })
                .collect();
        }
    }
}

fn set_list_field(tech_stack: &mut TechStack, field: &str, values: Vec<String>) {
    match field {
        "tech_stack.languages" => {
            // handled in finalize_languages via `all` derivation, but we still
            // want to preserve primary pick later.
        }
        "tech_stack.build_systems" => tech_stack.build_systems = values,
        "tech_stack.package_managers" => tech_stack.package_managers = values,
        "tech_stack.frameworks" => tech_stack.frameworks = values,
        "tech_stack.runtimes" => tech_stack.runtimes = values,
        "tech_stack.platforms" => tech_stack.platforms = values,
        "tech_stack.databases" => tech_stack.databases = values,
        "tech_stack.caches" => tech_stack.caches = values,
        "tech_stack.queues" => tech_stack.queues = values,
        "tech_stack.storage" => tech_stack.storage = values,
        "tech_stack.testing" => tech_stack.testing = values,
        "tech_stack.linting" => tech_stack.linting = values,
        "tech_stack.formatting" => tech_stack.formatting = values,
        "tech_stack.ci" => tech_stack.ci = values,
        "tech_stack.containerization" => tech_stack.containerization = values,
        "tech_stack.orchestration" => tech_stack.orchestration = values,
        "tech_stack.iac" => tech_stack.iac = values,
        "tech_stack.observability" => tech_stack.observability = values,
        "tech_stack.auth_providers" => tech_stack.auth_providers = values,
        "tech_stack.payment_processors" => tech_stack.payment_processors = values,
        "tech_stack.web.backend_frameworks" => {
            tech_stack
                .web
                .get_or_insert_with(WebStack::default)
                .backend_frameworks = values;
        }
        "tech_stack.web.frontend_frameworks" => {
            tech_stack
                .web
                .get_or_insert_with(WebStack::default)
                .frontend_frameworks = values;
        }
        "tech_stack.web.css_frameworks" => {
            tech_stack
                .web
                .get_or_insert_with(WebStack::default)
                .css_frameworks = values;
        }
        "tech_stack.web.bundlers" => {
            tech_stack
                .web
                .get_or_insert_with(WebStack::default)
                .bundlers = values;
        }
        "tech_stack.web.api_styles" => {
            tech_stack
                .web
                .get_or_insert_with(WebStack::default)
                .api_styles = values;
        }
        "tech_stack.web.orms" => {
            tech_stack.web.get_or_insert_with(WebStack::default).orms = values;
        }
        "tech_stack.mobile.ui_frameworks" => {
            tech_stack
                .mobile
                .get_or_insert_with(MobileStack::default)
                .ui_frameworks = values;
        }
        "tech_stack.mobile.notable_sdks" => {
            tech_stack
                .mobile
                .get_or_insert_with(MobileStack::default)
                .notable_sdks = values;
        }
        "tech_stack.desktop.shells" => {
            tech_stack
                .desktop
                .get_or_insert_with(DesktopStack::default)
                .shells = values;
        }
        "tech_stack.desktop.installer_formats" => {
            tech_stack
                .desktop
                .get_or_insert_with(DesktopStack::default)
                .installer_formats = values;
        }
        "tech_stack.game.engines" => {
            tech_stack
                .game
                .get_or_insert_with(GameStack::default)
                .engines = values;
        }
        "tech_stack.game.rendering_pipelines" => {
            tech_stack
                .game
                .get_or_insert_with(GameStack::default)
                .rendering_pipelines = values;
        }
        "tech_stack.game.shader_languages" => {
            tech_stack
                .game
                .get_or_insert_with(GameStack::default)
                .shader_languages = values;
        }
        "tech_stack.game.physics_engines" => {
            tech_stack
                .game
                .get_or_insert_with(GameStack::default)
                .physics_engines = values;
        }
        "tech_stack.game.networking" => {
            tech_stack
                .game
                .get_or_insert_with(GameStack::default)
                .networking = values;
        }
        "tech_stack.languages.detail" => {
            // Handled in finalize_languages.
        }
        _ => {
            // Unknown field path: silently drop. The default ruleset validator
            // will have caught this at load time.
        }
    }
}

fn set_scalar_field(tech_stack: &mut TechStack, field: &str, value: String) {
    match field {
        "tech_stack.languages.primary" => {
            tech_stack.languages.primary = Some(value);
        }
        "tech_stack.game.engine_version" => {
            tech_stack
                .game
                .get_or_insert_with(GameStack::default)
                .engine_version = Some(value);
        }
        "tech_stack.web.ssr_strategy" => {
            tech_stack
                .web
                .get_or_insert_with(WebStack::default)
                .ssr_strategy = Some(value);
        }
        "tech_stack.extension.host" => {
            tech_stack
                .extension
                .get_or_insert_with(ExtensionStack::default)
                .host = Some(value);
        }
        "tech_stack.extension.kind" => {
            tech_stack
                .extension
                .get_or_insert_with(ExtensionStack::default)
                .kind = Some(value);
        }
        _ => {}
    }
}

/// End-to-end helper: run the full pipeline against a snapshot.
pub fn detect_with(
    snapshot: &dyn InputSnapshot,
    ruleset: &Ruleset,
    config: &SynthesisConfig,
) -> DetectionReport {
    let started = std::time::Instant::now();
    let fires = crate::engine::evaluate(snapshot, ruleset);
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;

    let files = snapshot.all_files();
    let input_summary = InputSummary {
        files_scanned: files.len() as u64,
        bytes_scanned: files.iter().map(|f| f.bytes).sum(),
    };

    synthesize(
        fires,
        ruleset.rules.len() as u32,
        elapsed,
        input_summary,
        snapshot.ignored_paths(),
        &ruleset.version,
        config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::scorecard::Contribution as Contrib;
    use serde_json::json;

    fn fire(id: &str, w: f32, contribs: Vec<Contrib>) -> Fire {
        Fire {
            rule_id: id.into(),
            weight: w,
            evidence: vec![],
            contributes_to: contribs,
        }
    }

    fn contrib(field: &str, val: &str) -> Contrib {
        Contrib {
            field: field.into(),
            value: json!(val),
            delta: None,
        }
    }

    #[test]
    fn single_dominant_rule_picks_primary() {
        let fires = vec![fire(
            "r1",
            1.0,
            vec![
                contrib("app_type", "game"),
                contrib("tech_stack.game.engines", "unity"),
            ],
        )];
        let report = synthesize(
            fires,
            1,
            0.0,
            InputSummary::default(),
            vec![],
            "test",
            &SynthesisConfig::default(),
        );
        assert_eq!(report.app_type.primary.as_deref(), Some("game"));
        assert_eq!(report.tech_stack.game.unwrap().engines, vec!["unity"]);
    }

    #[test]
    fn tie_within_margin_produces_null_primary() {
        let fires = vec![
            fire("r1", 1.0, vec![contrib("app_type", "web_app")]),
            fire("r2", 1.0, vec![contrib("app_type", "cli_tool")]),
        ];
        let report = synthesize(
            fires,
            2,
            0.0,
            InputSummary::default(),
            vec![],
            "test",
            &SynthesisConfig::default(),
        );
        assert!(report.app_type.primary.is_none());
        assert_eq!(report.app_type.alternatives.len(), 2);
        assert!(!report.scorecard.warnings.is_empty());
    }

    #[test]
    fn list_field_votes_ranked_by_weight() {
        let fires = vec![
            fire(
                "r1",
                1.0,
                vec![contrib("tech_stack.build_systems", "cargo")],
            ),
            fire("r2", 2.0, vec![contrib("tech_stack.build_systems", "npm")]),
        ];
        let report = synthesize(
            fires,
            2,
            0.0,
            InputSummary::default(),
            vec![],
            "test",
            &SynthesisConfig::default(),
        );
        assert_eq!(report.tech_stack.build_systems, vec!["npm", "cargo"]);
    }

    #[test]
    fn empty_fires_emit_warning() {
        let report = synthesize(
            vec![],
            10,
            0.0,
            InputSummary::default(),
            vec![],
            "test",
            &SynthesisConfig::default(),
        );
        assert!(report.app_type.primary.is_none());
        assert!(!report.scorecard.warnings.is_empty());
    }
}
