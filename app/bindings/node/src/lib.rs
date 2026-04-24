#![deny(clippy::all)]

//! N-API bindings for `app-type-detector`.
//!
//! Four entrypoints mirror the Rust crate's public surface:
//! `detectPath`, `detectFiles`, `defaultRuleset`, `renderHumanReadable`.
//! Reports are passed as plain JS objects via napi-rs's `serde-json`
//! feature, so the wire shape is byte-identical to the crate's JSON.

use app_type_detector::{
    default_rules::default_ruleset as core_default_ruleset, detect_path as core_detect_path,
    detect_with, render_human_readable as core_render_human_readable, DetectionReport,
    MemorySnapshot, SynthesisConfig,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

fn report_to_value(report: &DetectionReport) -> Result<serde_json::Value> {
    serde_json::to_value(report).map_err(|e| Error::from_reason(e.to_string()))
}

#[napi(js_name = "detectPath")]
pub fn detect_path(path: String) -> Result<serde_json::Value> {
    let report = core_detect_path(&path)
        .map_err(|e| Error::from_reason(format!("detectPath({}) failed: {}", path, e)))?;
    report_to_value(&report)
}

/// `detectFiles({ files: Record<string, string | null> })`.
///
/// The argument must be a plain object with a `files` map whose values are
/// either strings (file contents) or `null` (the file exists but has no
/// tracked contents; matches `MemorySnapshot::with_empty`).
#[napi(js_name = "detectFiles")]
pub fn detect_files(input: serde_json::Value) -> Result<serde_json::Value> {
    let files = input
        .get("files")
        .ok_or_else(|| Error::from_reason("detectFiles input is missing `files`"))?
        .as_object()
        .ok_or_else(|| Error::from_reason("detectFiles `files` must be an object"))?
        .clone();

    let mut snapshot = MemorySnapshot::new();
    for (path, contents) in files.into_iter() {
        snapshot = match contents {
            serde_json::Value::String(text) => snapshot.with_file(path, text),
            serde_json::Value::Null => snapshot.with_empty(path),
            _ => {
                return Err(Error::from_reason(format!(
                    "detectFiles `files[{}]` must be a string or null",
                    path
                )));
            }
        };
    }
    let ruleset = core_default_ruleset();
    let report = detect_with(&snapshot, ruleset, &SynthesisConfig::default());
    report_to_value(&report)
}

#[napi(js_name = "defaultRuleset")]
pub fn default_ruleset() -> Result<serde_json::Value> {
    let ruleset = core_default_ruleset();
    serde_json::to_value(ruleset).map_err(|e| Error::from_reason(e.to_string()))
}

#[napi(js_name = "renderHumanReadable")]
pub fn render_human_readable(report: serde_json::Value) -> Result<String> {
    let parsed: DetectionReport = serde_json::from_value(report)
        .map_err(|e| Error::from_reason(format!("invalid DetectionReport: {}", e)))?;
    Ok(core_render_human_readable(&parsed))
}
