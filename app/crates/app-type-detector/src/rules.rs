#![allow(missing_docs)]

//! Declarative rule grammar for detection.
//!
//! A [`Rule`] couples a [`MatchExpr`] (the "when") with a list of
//! [`Contribution`]s (the "what this rule proves about the project").
//!
//! Rulesets are authored as JSON and compiled into the binary, but callers can
//! also construct them programmatically or extend the defaults via
//! [`Ruleset::extend`].

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::types::scorecard::Contribution;

/// Match expression: the "when" side of a rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MatchExpr {
    /// A specific file exists (exact path from the project root).
    FileExists { path: String },
    /// At least one file matches the glob pattern.
    Glob {
        pattern: String,
        #[serde(default = "default_min_count")]
        min_count: u64,
    },
    /// A file's contents match a regular expression. A match fires at least
    /// once regardless of how many lines matched.
    Content { file: String, regex: String },
    /// Logical AND across sub-expressions.
    All { of: Vec<MatchExpr> },
    /// Logical OR across sub-expressions.
    Any { of: Vec<MatchExpr> },
    /// Negation of a sub-expression. Fires if the sub-expression does NOT match.
    Not { of: Box<MatchExpr> },
}

fn default_min_count() -> u64 {
    1
}

/// The "what it proves" side of a rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RulePayload {
    #[serde(default = "default_weight")]
    pub confidence_weight: f32,
    pub contributions: Vec<Contribution>,
    /// Optional name of a content-kind expression whose first capture group
    /// should be lifted into a contribution at runtime. The contribution's
    /// value is used as a template where `${capture}` is replaced with the
    /// first capture group.
    #[serde(default)]
    pub captures_into: Option<CaptureInto>,
}

fn default_weight() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaptureInto {
    /// The path of the content-kind sub-expression to capture from. Format is
    /// the `file` string of the sub-expression; the synthesizer looks for the
    /// first `Content` evidence in the rule with a matching file.
    pub from_file: String,
    /// The field path to write into.
    pub field: String,
}

/// A detection rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rule {
    pub id: String,
    #[serde(default)]
    pub description: String,
    pub when: MatchExpr,
    pub payload: RulePayload,
}

/// A collection of rules plus metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ruleset {
    pub schema_version: u32,
    pub version: String,
    pub rules: Vec<Rule>,
}

impl Ruleset {
    pub fn from_json(json: &str) -> Result<Self, RuleError> {
        let rs: Ruleset = serde_json::from_str(json).map_err(RuleError::Parse)?;
        rs.validate()?;
        Ok(rs)
    }

    /// Extend this ruleset with rules from another. Duplicate rule ids error.
    pub fn extend(&mut self, other: Ruleset) -> Result<(), RuleError> {
        for rule in &self.rules {
            if other.rules.iter().any(|r| r.id == rule.id) {
                return Err(RuleError::DuplicateRuleId(rule.id.clone()));
            }
        }
        self.rules.extend(other.rules);
        Ok(())
    }

    fn validate(&self) -> Result<(), RuleError> {
        if self.schema_version != 1 {
            return Err(RuleError::UnsupportedSchemaVersion(self.schema_version));
        }
        let mut seen = std::collections::HashSet::new();
        for rule in &self.rules {
            if !seen.insert(rule.id.clone()) {
                return Err(RuleError::DuplicateRuleId(rule.id.clone()));
            }
            validate_match_expr(&rule.when)?;
        }
        Ok(())
    }
}

fn validate_match_expr(expr: &MatchExpr) -> Result<(), RuleError> {
    match expr {
        MatchExpr::Content { regex, .. } => {
            regex::Regex::new(regex)
                .map_err(|e| RuleError::BadRegex(regex.clone(), e.to_string()))?;
            Ok(())
        }
        MatchExpr::Glob { pattern, .. } => {
            globset::Glob::new(pattern)
                .map_err(|e| RuleError::BadGlob(pattern.clone(), e.to_string()))?;
            Ok(())
        }
        MatchExpr::FileExists { .. } => Ok(()),
        MatchExpr::All { of } | MatchExpr::Any { of } => {
            for sub in of {
                validate_match_expr(sub)?;
            }
            Ok(())
        }
        MatchExpr::Not { of } => validate_match_expr(of),
    }
}

#[derive(Debug, Error)]
pub enum RuleError {
    #[error("failed to parse ruleset JSON: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unsupported ruleset schema_version: {0}")]
    UnsupportedSchemaVersion(u32),
    #[error("duplicate rule id: {0}")]
    DuplicateRuleId(String),
    #[error("invalid regex {0}: {1}")]
    BadRegex(String, String),
    #[error("invalid glob {0}: {1}")]
    BadGlob(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_exists_parses() {
        let e: MatchExpr =
            serde_json::from_str(r#"{"kind":"file_exists","path":"Cargo.toml"}"#).unwrap();
        assert_eq!(
            e,
            MatchExpr::FileExists {
                path: "Cargo.toml".into()
            }
        );
    }

    #[test]
    fn any_of_parses() {
        let e: MatchExpr = serde_json::from_str(
            r#"{"kind":"any","of":[{"kind":"file_exists","path":"a"},{"kind":"file_exists","path":"b"}]}"#,
        )
        .unwrap();
        match e {
            MatchExpr::Any { of } => assert_eq!(of.len(), 2),
            _ => panic!("expected any"),
        }
    }

    #[test]
    fn bad_regex_rejected() {
        let json = r#"{
          "schema_version": 1,
          "version": "0.0.0",
          "rules": [{
            "id": "x",
            "when": {"kind":"content","file":"f","regex":"(unclosed"},
            "payload": {"confidence_weight": 1.0, "contributions": []}
          }]
        }"#;
        assert!(matches!(
            Ruleset::from_json(json),
            Err(RuleError::BadRegex(_, _))
        ));
    }

    #[test]
    fn duplicate_ids_rejected() {
        let json = r#"{
          "schema_version": 1,
          "version": "0.0.0",
          "rules": [
            {"id":"x","when":{"kind":"file_exists","path":"a"},"payload":{"contributions":[]}},
            {"id":"x","when":{"kind":"file_exists","path":"b"},"payload":{"contributions":[]}}
          ]
        }"#;
        assert!(matches!(
            Ruleset::from_json(json),
            Err(RuleError::DuplicateRuleId(_))
        ));
    }

    #[test]
    fn extend_rejects_duplicates() {
        let a = Ruleset {
            schema_version: 1,
            version: "0.0.0".into(),
            rules: vec![Rule {
                id: "shared".into(),
                description: String::new(),
                when: MatchExpr::FileExists { path: "a".into() },
                payload: RulePayload {
                    confidence_weight: 1.0,
                    contributions: vec![],
                    captures_into: None,
                },
            }],
        };
        let b = a.clone();
        let mut a = a;
        assert!(a.extend(b).is_err());
    }
}
