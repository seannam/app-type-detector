#![allow(missing_docs)]

pub mod app_type;
pub mod enums;
pub mod report;
pub mod scorecard;
pub mod tech_stack;

pub use app_type::AppType;
pub use enums::{
    AuthProvider, BuildSystem, Cache, CiSystem, Containerizer, Database, Formatter, IacTool,
    Language, Linter, ObservabilityTool, Orchestrator, PackageManager, PaymentProcessor, Platform,
    Queue, Runtime, Storage, TestFramework,
};
pub use report::{Alternative, AppTypeFinding, DetectionReport, SCHEMA_VERSION};
pub use scorecard::{Contribution, Evidence, Fire, InputSummary, Scorecard};
pub use tech_stack::{
    DesktopStack, ExtensionStack, GameStack, LanguageUsage, LanguagesFinding, MobileStack,
    PlatformVersion, TechStack, WebStack,
};
