//! # app-type-detector
//!
//! A generic, reusable library for classifying any codebase.
//!
//! Given either a directory on disk ([`detect_path`]) or an in-memory file map
//! ([`detect_files`]), the crate returns a [`DetectionReport`] describing:
//!
//! - `app_type` — the *role* of the codebase (game, web_app, mobile_app, …).
//! - `tech_stack` — the *how*: languages, build systems, runtimes, platforms,
//!   databases, and optional domain sub-records (`web`, `mobile`, `desktop`,
//!   `game`, `extension`).
//! - `scorecard` — a machine-readable trace of every rule that fired, the
//!   predicates each rule matched, and the fields it contributed to.
//!
//! The library performs zero network I/O, never spawns child processes, and
//! treats unknown / polyglot / empty codebases as legitimate inputs that
//! produce a low-confidence answer rather than an error.

#![warn(missing_docs)]
#![deny(clippy::all)]
#![doc(html_root_url = "https://docs.rs/app-type-detector/0.1.0")]

pub mod engine;
pub mod rules;
pub mod snapshot;
pub mod synthesis;
pub mod types;

#[cfg(feature = "default-rules")]
pub mod default_rules;

#[cfg(feature = "human-renderer")]
pub mod render;

pub use rules::{MatchExpr, Rule, RuleError, RulePayload, Ruleset};
pub use snapshot::{FileEntry, InputSnapshot, MemorySnapshot};
pub use synthesis::{detect_with, synthesize, SynthesisConfig};
pub use types::{
    Alternative, AppType, AppTypeFinding, Contribution, DesktopStack, DetectionReport, Evidence,
    ExtensionStack, Fire, GameStack, InputSummary, LanguageUsage, LanguagesFinding, MobileStack,
    PlatformVersion, Scorecard, TechStack, WebStack, SCHEMA_VERSION,
};

#[cfg(feature = "fs")]
pub use snapshot::FilesystemSnapshot;

/// Detect the app type of a directory on disk using the default ruleset.
#[cfg(all(feature = "fs", feature = "default-rules"))]
pub fn detect_path(path: impl AsRef<std::path::Path>) -> std::io::Result<DetectionReport> {
    let snap = FilesystemSnapshot::new(path)?;
    let ruleset = default_rules::default_ruleset();
    Ok(detect_with(&snap, ruleset, &SynthesisConfig::default()))
}

/// Detect the app type of an in-memory file map using the default ruleset.
#[cfg(feature = "default-rules")]
pub fn detect_files(snapshot: &MemorySnapshot) -> DetectionReport {
    let ruleset = default_rules::default_ruleset();
    detect_with(snapshot, ruleset, &SynthesisConfig::default())
}

/// Render a report as human-readable text. Consumes only the JSON shape, so
/// bindings can call it unchanged.
#[cfg(feature = "human-renderer")]
pub fn render_human_readable(report: &DetectionReport) -> String {
    render::render(report)
}
