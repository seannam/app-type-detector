use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Scorecard {
    pub rules_evaluated: u32,
    pub rules_fired: u32,
    pub elapsed_ms: f64,
    pub input_summary: InputSummary,
    #[serde(default)]
    pub ignored_paths: Vec<String>,
    #[serde(default)]
    pub fires: Vec<Fire>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl Scorecard {
    /// Serialize each fire on its own line (JSONL).
    pub fn fires_jsonl(&self) -> String {
        let mut out = String::new();
        for fire in &self.fires {
            match serde_json::to_string(fire) {
                Ok(line) => {
                    out.push_str(&line);
                    out.push('\n');
                }
                Err(_) => continue,
            }
        }
        out
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InputSummary {
    pub files_scanned: u64,
    pub bytes_scanned: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fire {
    pub rule_id: String,
    pub weight: f32,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub contributes_to: Vec<Contribution>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Evidence {
    FileExists {
        path: String,
        matched: bool,
    },
    Glob {
        pattern: String,
        matched_count: u64,
    },
    Content {
        file: String,
        regex: String,
        #[serde(default)]
        captures: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contribution {
    pub field: String,
    pub value: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub delta: Option<f32>,
}
