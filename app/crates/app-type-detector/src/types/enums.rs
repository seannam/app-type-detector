//! Flat-namespace enums used in the cross-cutting parts of `TechStack`.
//!
//! Every enum is `#[non_exhaustive]`. Serialization uses `snake_case` with explicit
//! renames for unusual wire names (`csharp` rather than `c_sharp`, etc.).
//!
//! Internally we do not carry the enum values as strongly-typed variants through
//! the evaluation pipeline: rule contributions reference string values that
//! correspond to the JSON wire names. That keeps the default rules JSON as the
//! vocabulary authority and avoids sync drift.

use serde::{Deserialize, Serialize};

macro_rules! str_enum {
    (
        $(#[$outer:meta])*
        pub enum $name:ident {
            $(
                $(#[$inner:meta])*
                $variant:ident => $wire:literal ,
            )+
        }
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[non_exhaustive]
        pub enum $name {
            $(
                $(#[$inner])*
                #[serde(rename = $wire)]
                $variant,
            )+
        }

        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $( Self::$variant => $wire, )+
                }
            }
        }
    };
}

str_enum! {
    pub enum Language {
        Rust => "rust",
        Go => "go",
        Python => "python",
        TypeScript => "typescript",
        JavaScript => "javascript",
        CSharp => "csharp",
        Cpp => "cpp",
        C => "c",
        Java => "java",
        Kotlin => "kotlin",
        Swift => "swift",
        ObjectiveC => "objective_c",
        Ruby => "ruby",
        Php => "php",
        Html => "html",
        Css => "css",
        Hlsl => "hlsl",
        Glsl => "glsl",
        Wgsl => "wgsl",
        Metal => "metal",
        ShaderLab => "shaderlab",
        GdScript => "gdscript",
        Lua => "lua",
        Dart => "dart",
        Shell => "shell",
        Yaml => "yaml",
        Toml => "toml",
        Json => "json",
        Markdown => "markdown",
        GodotShader => "godot_shader",
    }
}

str_enum! {
    pub enum BuildSystem {
        Cargo => "cargo",
        Npm => "npm",
        Pnpm => "pnpm",
        Yarn => "yarn",
        Bun => "bun",
        Unity => "unity",
        Godot => "godot",
        Unreal => "unreal",
        GameMaker => "gamemaker",
        Xcode => "xcode",
        XcodeGen => "xcodegen",
        Gradle => "gradle",
        Maven => "maven",
        Go => "go",
        Uv => "uv",
        Pip => "pip",
        Poetry => "poetry",
        Hatch => "hatch",
        Make => "make",
        Cmake => "cmake",
        Wordpress => "wordpress",
        Swift => "swift",
        Nuget => "nuget",
    }
}

str_enum! {
    pub enum PackageManager {
        Cargo => "cargo",
        Npm => "npm",
        Pnpm => "pnpm",
        Yarn => "yarn",
        Bun => "bun",
        Uv => "uv",
        Pip => "pip",
        Poetry => "poetry",
        Nuget => "nuget",
        UnityPackageManager => "unity_package_manager",
        GoModules => "go_modules",
        Gradle => "gradle",
        Maven => "maven",
        Swift => "swift_package_manager",
        Cocoapods => "cocoapods",
        Homebrew => "homebrew",
        Composer => "composer",
        Bundler => "bundler",
    }
}

str_enum! {
    pub enum Runtime {
        Node => "node",
        Deno => "deno",
        Bun => "bun",
        Jvm => "jvm",
        Dotnet => "dotnet",
        Mono => "mono",
        Python => "python",
        Ruby => "ruby",
        Go => "go",
        Php => "php",
    }
}

str_enum! {
    pub enum Platform {
        Ios => "ios",
        Android => "android",
        Macos => "macos",
        Windows => "windows",
        Linux => "linux",
        Web => "web",
        VisionOs => "visionos",
        TvOs => "tvos",
        WatchOs => "watchos",
        SteamDeck => "steamdeck",
    }
}

str_enum! {
    pub enum Database {
        Postgres => "postgres",
        Mysql => "mysql",
        Sqlite => "sqlite",
        MongoDb => "mongodb",
        Redis => "redis",
        DynamoDb => "dynamodb",
        Cassandra => "cassandra",
        Mariadb => "mariadb",
        Neo4j => "neo4j",
    }
}

str_enum! {
    pub enum Cache {
        Redis => "redis",
        Memcached => "memcached",
    }
}

str_enum! {
    pub enum Queue {
        Rabbitmq => "rabbitmq",
        Kafka => "kafka",
        Sqs => "sqs",
        Sidekiq => "sidekiq",
        Celery => "celery",
    }
}

str_enum! {
    pub enum Storage {
        S3 => "s3",
        R2 => "r2",
        Gcs => "gcs",
        AzureBlob => "azure_blob",
    }
}

str_enum! {
    pub enum TestFramework {
        Vitest => "vitest",
        Jest => "jest",
        Pytest => "pytest",
        XcTest => "xctest",
        CargoTest => "cargo_test",
        GoTest => "go_test",
        JUnit => "junit",
        Mocha => "mocha",
    }
}

str_enum! {
    pub enum Linter {
        EsLint => "eslint",
        Ruff => "ruff",
        Clippy => "clippy",
        SwiftLint => "swiftlint",
        GolangciLint => "golangci_lint",
        Rubocop => "rubocop",
    }
}

str_enum! {
    pub enum Formatter {
        Prettier => "prettier",
        RuffFormat => "ruff_format",
        Rustfmt => "rustfmt",
        SwiftFormat => "swift_format",
        Black => "black",
        Gofmt => "gofmt",
    }
}

str_enum! {
    pub enum CiSystem {
        GithubActions => "github_actions",
        GitlabCi => "gitlab_ci",
        CircleCi => "circleci",
        Jenkins => "jenkins",
        Travis => "travis",
        AzurePipelines => "azure_pipelines",
    }
}

str_enum! {
    pub enum Containerizer {
        Docker => "docker",
        Podman => "podman",
    }
}

str_enum! {
    pub enum Orchestrator {
        Kubernetes => "kubernetes",
        Nomad => "nomad",
        Ecs => "ecs",
        Swarm => "swarm",
    }
}

str_enum! {
    pub enum IacTool {
        Terraform => "terraform",
        Pulumi => "pulumi",
        Ansible => "ansible",
        Cloudformation => "cloudformation",
    }
}

str_enum! {
    pub enum ObservabilityTool {
        Sentry => "sentry",
        Datadog => "datadog",
        Prometheus => "prometheus",
        Grafana => "grafana",
        OpenTelemetry => "open_telemetry",
    }
}

str_enum! {
    pub enum AuthProvider {
        Clerk => "clerk",
        Auth0 => "auth0",
        SupabaseAuth => "supabase_auth",
        FirebaseAuth => "firebase_auth",
        NextAuth => "next_auth",
    }
}

str_enum! {
    pub enum PaymentProcessor {
        Stripe => "stripe",
        Paddle => "paddle",
        Lemonsqueezy => "lemonsqueezy",
    }
}
