#![allow(missing_docs)]

//! Deterministic rule engine.
//!
//! Walks every rule in a [`Ruleset`] against an [`InputSnapshot`] and emits one
//! [`Fire`] per rule that matched. The engine is pure: it never touches the
//! filesystem except through the snapshot, never spawns child processes, and
//! never opens a network socket. The same input + ruleset always yields the
//! same output, in the same order.

use regex::Regex;

use crate::rules::{MatchExpr, Rule, Ruleset};
use crate::snapshot::InputSnapshot;
use crate::types::scorecard::{Contribution, Evidence, Fire};

/// Evaluate every rule against the snapshot. Returns one fire per matched rule
/// in definition order. Rules that did not match produce nothing.
pub fn evaluate(snapshot: &dyn InputSnapshot, ruleset: &Ruleset) -> Vec<Fire> {
    let mut fires = Vec::with_capacity(ruleset.rules.len());
    for rule in &ruleset.rules {
        if let Some(fire) = evaluate_rule(snapshot, rule) {
            fires.push(fire);
        }
    }
    fires
}

fn evaluate_rule(snapshot: &dyn InputSnapshot, rule: &Rule) -> Option<Fire> {
    let mut evidence = Vec::new();
    if !evaluate_match(snapshot, &rule.when, &mut evidence) {
        return None;
    }

    // Lift any capture-into directive into an additional contribution.
    let mut contributions = rule.payload.contributions.clone();
    if let Some(ci) = rule.payload.captures_into.as_ref() {
        if let Some(cap) = first_capture_for(&evidence, &ci.from_file) {
            contributions.push(Contribution {
                field: ci.field.clone(),
                value: serde_json::Value::String(cap),
                delta: None,
            });
        }
    }

    Some(Fire {
        rule_id: rule.id.clone(),
        weight: rule.payload.confidence_weight,
        evidence,
        contributes_to: contributions,
    })
}

fn first_capture_for(evidence: &[Evidence], file: &str) -> Option<String> {
    for ev in evidence {
        if let Evidence::Content {
            file: f, captures, ..
        } = ev
        {
            if f == file {
                return captures.first().cloned();
            }
        }
    }
    None
}

fn evaluate_match(
    snapshot: &dyn InputSnapshot,
    expr: &MatchExpr,
    evidence: &mut Vec<Evidence>,
) -> bool {
    match expr {
        MatchExpr::FileExists { path } => {
            let matched = snapshot.file_exists(path);
            evidence.push(Evidence::FileExists {
                path: path.clone(),
                matched,
            });
            matched
        }
        MatchExpr::Glob { pattern, min_count } => {
            let count = snapshot.glob_count(pattern);
            evidence.push(Evidence::Glob {
                pattern: pattern.clone(),
                matched_count: count,
            });
            count >= *min_count
        }
        MatchExpr::Content { file, regex } => {
            let Ok(re) = Regex::new(regex) else {
                evidence.push(Evidence::Content {
                    file: file.clone(),
                    regex: regex.clone(),
                    captures: vec![],
                });
                return false;
            };
            let Some(contents) = snapshot.file_contents(file) else {
                evidence.push(Evidence::Content {
                    file: file.clone(),
                    regex: regex.clone(),
                    captures: vec![],
                });
                return false;
            };
            let Some(caps) = re.captures(&contents) else {
                evidence.push(Evidence::Content {
                    file: file.clone(),
                    regex: regex.clone(),
                    captures: vec![],
                });
                return false;
            };
            let mut captures: Vec<String> = Vec::new();
            for i in 1..caps.len() {
                if let Some(m) = caps.get(i) {
                    captures.push(m.as_str().to_string());
                }
            }
            evidence.push(Evidence::Content {
                file: file.clone(),
                regex: regex.clone(),
                captures,
            });
            true
        }
        MatchExpr::All { of } => {
            for sub in of {
                if !evaluate_match(snapshot, sub, evidence) {
                    return false;
                }
            }
            !of.is_empty()
        }
        MatchExpr::Any { of } => {
            let mut any_hit = false;
            for sub in of {
                if evaluate_match(snapshot, sub, evidence) {
                    any_hit = true;
                }
            }
            any_hit
        }
        MatchExpr::Not { of } => {
            let mut temp = Vec::new();
            let inner = evaluate_match(snapshot, of, &mut temp);
            // Record inner evidence for visibility, but invert the result.
            evidence.extend(temp);
            !inner
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{Rule, RulePayload, Ruleset};
    use crate::snapshot::MemorySnapshot;

    fn rs_with(r: Rule) -> Ruleset {
        Ruleset {
            schema_version: 1,
            version: "test".into(),
            rules: vec![r],
        }
    }

    #[test]
    fn file_exists_rule_fires() {
        let snap = MemorySnapshot::new().with_file("Cargo.toml", "[package]");
        let rs = rs_with(Rule {
            id: "cargo".into(),
            description: String::new(),
            when: MatchExpr::FileExists {
                path: "Cargo.toml".into(),
            },
            payload: RulePayload {
                confidence_weight: 1.0,
                contributions: vec![],
                captures_into: None,
            },
        });
        let fires = evaluate(&snap, &rs);
        assert_eq!(fires.len(), 1);
    }

    #[test]
    fn content_rule_captures() {
        let snap = MemorySnapshot::new()
            .with_file("ProjectVersion.txt", "m_EditorVersion: 2022.3.42f1\nrest\n");
        let rs = rs_with(Rule {
            id: "unity-version".into(),
            description: String::new(),
            when: MatchExpr::Content {
                file: "ProjectVersion.txt".into(),
                regex: r"m_EditorVersion:\s*(\S+)".into(),
            },
            payload: RulePayload {
                confidence_weight: 1.0,
                contributions: vec![],
                captures_into: None,
            },
        });
        let fires = evaluate(&snap, &rs);
        assert_eq!(fires.len(), 1);
        match &fires[0].evidence[0] {
            Evidence::Content { captures, .. } => {
                assert_eq!(captures, &vec!["2022.3.42f1".to_string()]);
            }
            _ => panic!("expected content"),
        }
    }

    #[test]
    fn not_rule_inverts() {
        let snap = MemorySnapshot::new();
        let rs = rs_with(Rule {
            id: "no-cargo".into(),
            description: String::new(),
            when: MatchExpr::Not {
                of: Box::new(MatchExpr::FileExists {
                    path: "Cargo.toml".into(),
                }),
            },
            payload: RulePayload {
                confidence_weight: 1.0,
                contributions: vec![],
                captures_into: None,
            },
        });
        assert_eq!(evaluate(&snap, &rs).len(), 1);
    }
}
