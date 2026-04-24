use serde::{Deserialize, Serialize};

/// Top-level tech-stack description for a detection report.
///
/// Fields use wire-name strings so the same JSON vocabulary used by the default
/// rules round-trips directly. Strongly-typed enums (`enums::Language`, etc.) are
/// exposed as a separate convenience surface for consumers that prefer them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TechStack {
    pub languages: LanguagesFinding,

    #[serde(default)]
    pub build_systems: Vec<String>,
    #[serde(default)]
    pub package_managers: Vec<String>,
    #[serde(default)]
    pub frameworks: Vec<String>,
    #[serde(default)]
    pub runtimes: Vec<String>,
    #[serde(default)]
    pub platforms: Vec<String>,

    #[serde(default)]
    pub databases: Vec<String>,
    #[serde(default)]
    pub caches: Vec<String>,
    #[serde(default)]
    pub queues: Vec<String>,
    #[serde(default)]
    pub storage: Vec<String>,

    #[serde(default)]
    pub testing: Vec<String>,
    #[serde(default)]
    pub linting: Vec<String>,
    #[serde(default)]
    pub formatting: Vec<String>,
    #[serde(default)]
    pub ci: Vec<String>,
    #[serde(default)]
    pub containerization: Vec<String>,
    #[serde(default)]
    pub orchestration: Vec<String>,
    #[serde(default)]
    pub iac: Vec<String>,
    #[serde(default)]
    pub observability: Vec<String>,
    #[serde(default)]
    pub auth_providers: Vec<String>,
    #[serde(default)]
    pub payment_processors: Vec<String>,

    #[serde(default)]
    pub web: Option<WebStack>,
    #[serde(default)]
    pub mobile: Option<MobileStack>,
    #[serde(default)]
    pub desktop: Option<DesktopStack>,
    #[serde(default)]
    pub game: Option<GameStack>,
    #[serde(default)]
    pub extension: Option<ExtensionStack>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LanguagesFinding {
    pub primary: Option<String>,
    #[serde(default)]
    pub all: Vec<LanguageUsage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageUsage {
    pub language: String,
    pub role: String,
    pub file_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WebStack {
    #[serde(default)]
    pub backend_frameworks: Vec<String>,
    #[serde(default)]
    pub frontend_frameworks: Vec<String>,
    #[serde(default)]
    pub css_frameworks: Vec<String>,
    #[serde(default)]
    pub bundlers: Vec<String>,
    #[serde(default)]
    pub ssr_strategy: Option<String>,
    #[serde(default)]
    pub api_styles: Vec<String>,
    #[serde(default)]
    pub orms: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MobileStack {
    #[serde(default)]
    pub ui_frameworks: Vec<String>,
    #[serde(default)]
    pub min_platform_versions: Vec<PlatformVersion>,
    #[serde(default)]
    pub notable_sdks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformVersion {
    pub platform: String,
    pub version: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DesktopStack {
    #[serde(default)]
    pub shells: Vec<String>,
    #[serde(default)]
    pub installer_formats: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GameStack {
    #[serde(default)]
    pub engines: Vec<String>,
    #[serde(default)]
    pub engine_version: Option<String>,
    #[serde(default)]
    pub rendering_pipelines: Vec<String>,
    #[serde(default)]
    pub shader_languages: Vec<String>,
    #[serde(default)]
    pub physics_engines: Vec<String>,
    #[serde(default)]
    pub networking: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExtensionStack {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
}
