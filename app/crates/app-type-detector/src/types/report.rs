use serde::{Deserialize, Serialize};

use super::scorecard::Scorecard;
use super::tech_stack::TechStack;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectionReport {
    pub schema_version: u32,
    pub ruleset_version: String,
    pub app_type: AppTypeFinding,
    pub tech_stack: TechStack,
    pub scorecard: Scorecard,
}

impl DetectionReport {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn to_json_compact(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn to_tsv(&self) -> String {
        let primary_app_type = self.app_type.primary.clone().unwrap_or_default();
        let primary_language = self
            .tech_stack
            .languages
            .primary
            .clone()
            .unwrap_or_default();
        let primary_build_system = self
            .tech_stack
            .build_systems
            .first()
            .cloned()
            .unwrap_or_default();
        let confidence = self.app_type.confidence;
        format!(
            "{}\t{}\t{}\t{:.2}",
            primary_app_type, primary_language, primary_build_system, confidence
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppTypeFinding {
    pub primary: Option<String>,
    pub confidence: f32,
    #[serde(default)]
    pub alternatives: Vec<Alternative>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alternative {
    pub value: String,
    pub confidence: f32,
}
