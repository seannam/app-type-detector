# Feature: `app-type-detector` — a generic, reusable library for classifying any codebase

## Feature Description

`app-type-detector` is a self-contained library that takes a codebase and returns a structured description of what it is. The contract is intentionally narrow:

- **Input:** a directory path on disk, or an in-memory snapshot of a directory's files (paths + textual contents + the top-level entry listing).
- **Output:** a single `DetectionReport` object (never a string) with three top-level pieces:
  - `app_type` — what role this codebase plays (e.g. `game`, `web_app`, `mobile_app`, `cli_tool`, `library`, `mcp_server`).
  - `tech_stack` — a richly-detailed object describing *how* it is built: primary and secondary languages, build systems, package managers, frameworks, platforms, databases, runtimes, plus domain-specific sub-records when relevant (a `game` carries `tech_stack.game.engines`, `rendering_pipelines`, `shader_languages`; a `web_app` carries `tech_stack.web.backend_frameworks`, `databases`, `api_styles`; a `mobile_app` carries `tech_stack.mobile.ui_frameworks` and `min_platform_versions`; a `desktop_app` carries `tech_stack.desktop.shells` and `installer_formats`).
  - `scorecard` — a machine-readable trace of every rule that fired, the predicates each rule matched, the weight it carried, and the fields it contributed to. The scorecard is the primary explainability surface and is what a human-readable report is rendered from.

The library ships through three channels so any tool, in any language, can consume it without re-implementing detection:

- **crates.io** — the canonical Rust crate.
- **npm** (`@snam/app-type-detector`) — `napi-rs` native addons with prebuilt binaries for the major triples and a WASM fallback for restricted runtimes.
- **PyPI** (`app-type-detector`) — `pyo3` + `maturin` ABI3 wheels so Python 3.10+ shares one binary per triple.

A small `app-type-detector` CLI binary is also published, exposing the engine to shell consumers.

The library performs zero network I/O, never spawns child processes, and treats unknown / polyglot / empty codebases as legitimate inputs that produce a low-confidence answer rather than an error.

## User Story

As a tool builder writing scripts, services, or apps that need to know "what is this codebase?",
I want a single dependency that takes a path or a file map and hands back a typed object describing the app type, the full tech stack, and an explainable scorecard,
So that I never write yet another shell pipeline of `find`, `jq`, and globs (or yet another LLM prompt) to re-derive the same answer, and so that my tool inherits new detections (visionOS apps, new game engines, future MCP variants, new web frameworks) the moment the library learns them, without me changing a line of my own code.

## Problem Statement

Across the developer-tools ecosystem, "what kind of project is this?" is solved over and over in incompatible ways:

1. **Duplicated, drifting rule sets.** Most tools that need this answer encode it inline: a bash script with `find` + `jq`, an LLM prompt that asks "based on these files, what is this?", a hand-coded if/else over `package.json` keys. Each implementation has its own gaps and its own bugs.
2. **The wrong tool for the job.** Bash + glob loops are slow and brittle. LLMs are non-deterministic, cost tokens, and add latency to a question that a 50-line rule answers for free with full explainability.
3. **No shared vocabulary.** Every tool invents its own labels — and worse, conflates concepts that should be separated. "iOS game", "Unity game", "macOS app", and "Android app" all collapse into a single string somewhere, even though they really mean two orthogonal things: an *app type* (game, app) and a *tech stack* (Unity, iOS, macOS, Android). When labels conflate dimensions, downstream consumers cannot easily ask "show me all games regardless of engine" or "show me everything built in Unity regardless of platform".
4. **No shared evidence.** Even tools that arrive at the same label cannot tell each other *why*. A library that returns a structured scorecard (rules fired, predicates matched, weights, contributions) lets downstream tools build trust, surface justifications, and decide when to fall back to a human or an LLM.
5. **Adding a new project type is N edits across N codebases.** Every time a new ecosystem appears, every interested tool has to update its own detection independently. There is no place to land "the new rule" once.

## Solution Statement

Build `app-type-detector` as a generic detection engine with a clean, two-axis output model and a structured explainability layer:

- **Two orthogonal axes.** `AppType` is the *role* (game, web_app, mobile_app, desktop_app, cli_tool, library, …). `TechStack` is the *how* (languages, frameworks, engines, platforms, databases, runtimes, …). A Godot mobile game returns `app_type = "game"` AND `tech_stack.game.engines = ["godot"]` AND `tech_stack.platforms = ["ios", "android"]`. A Unity Steam game returns `app_type = "game"` AND `tech_stack.game.engines = ["unity"]` AND `tech_stack.platforms = ["windows", "macos"]`. The library never collapses these dimensions into a single string.
- **Rich, domain-aware tech stack.** `TechStack` carries the breadth a downstream tool actually needs. For a web app: backend framework, frontend framework, ORM, database engine, cache, queue, CSS framework, bundler, API style. For a game: engine, engine version, rendering pipeline, shader languages, physics engine, networking. For a mobile app: UI framework, min platform versions, installed SDKs of note. For every codebase: cross-cutting fields like testing frameworks, linters, formatters, CI systems, containerization, IaC, auth providers, payment processors. Optional sub-records (`tech_stack.web`, `tech_stack.mobile`, `tech_stack.game`, `tech_stack.desktop`) appear only when relevant.
- **Scorecard-first explainability.** Every rule that fires emits a `Fire` record into `scorecard.fires` listing the predicates it matched (with captures), the weight it contributed, and the exact `(field, value, delta)` contributions to the report. The `app_type` and `tech_stack` fields are derived from the scorecard, not the other way around. A consumer can disable the human renderer entirely and consume the scorecard JSON directly.
- **Multiple output formats from one report.** The canonical wire format is JSON (`DetectionReport.to_json()`). A human-readable Markdown / plain-text rendering (`DetectionReport.to_human_readable()`) is built on top of the same scorecard. A one-line TSV summary is available for shell pipelines. A JSONL stream of fires is available for `jq` consumers.
- **One canonical rule format.** Detection rules live in a single JSON document compiled into the binary as the default ruleset. The format is documented, versioned, and overridable: callers may pass their own ruleset or extend the defaults.
- **Two input shapes, one engine.** `detect_path(path)` walks a real directory (gated behind an `fs` feature flag); `detect_files(snapshot)` accepts an in-memory file map for callers that already fetched files from a remote source.
- **Pure functions, explicit evidence.** No hidden state. No telemetry. No network. No subprocesses.
- **Three publishing channels from one source.** Same Rust core, three idiomatic bindings.
- **Open vocabulary mapping for consumers.** The library does not know about any specific downstream tool. Consumers translate `app_type` and `tech_stack` into their own taxonomies on their side via a thin lookup table.

## Output Format

This section is the contract. Every binding (Rust, Node, Python, CLI) produces this same shape, byte-for-byte, modulo idiomatic naming (`snake_case` in JSON and Python; `camelCase` is also accepted in the Node binding via a config flag).

### Schema overview

A `DetectionReport` is the following object. Every field is documented; optional sub-records appear only when the corresponding rule fires.

```jsonc
{
  "schema_version": 1,                           // bumped only for breaking changes
  "ruleset_version": "1.4.0",                    // version of the bundled or supplied ruleset

  "app_type": {
    "primary": "game",                           // AppType enum value, or null when no rule dominates
    "confidence": 0.97,                          // 0..1
    "alternatives": [                            // ranked secondaries
      { "value": "desktop_app", "confidence": 0.42 }
    ]
  },

  "tech_stack": {
    "languages": {
      "primary": "csharp",                       // Language enum, or null
      "all": [
        { "language": "csharp", "role": "gameplay", "file_count": 142 },
        { "language": "hlsl",   "role": "shaders",  "file_count": 8 }
      ]
    },
    "build_systems": ["unity"],                  // [BuildSystem]
    "package_managers": ["nuget"],               // [PackageManager]
    "frameworks": [],                            // [Framework] (cross-cutting; domain frameworks live in sub-records)
    "runtimes": [],                              // [Runtime]: node, deno, bun, jvm, dotnet, python, ruby, …
    "platforms": ["windows", "macos", "ios"],    // [Platform]: ios, android, macos, windows, linux, web, visionos, tvos, watchos, steamdeck

    "databases": [],                             // [Database]: postgres, mysql, sqlite, mongodb, redis, …
    "caches": [],                                // [Cache]: redis, memcached, …
    "queues": [],                                // [Queue]: rabbitmq, kafka, sqs, sidekiq, …
    "storage": [],                               // [Storage]: s3, r2, gcs, azure_blob, …

    "testing": [],                               // [TestFramework]: vitest, jest, pytest, xctest, cargo_test, …
    "linting": [],                               // [Linter]: eslint, ruff, clippy, swiftlint, …
    "formatting": [],                            // [Formatter]: prettier, ruff_format, rustfmt, swift_format, …
    "ci": ["github_actions"],                    // [CiSystem]: github_actions, gitlab_ci, circleci, jenkins, …
    "containerization": [],                      // [Containerizer]: docker, podman
    "orchestration": [],                         // [Orchestrator]: kubernetes, nomad, ecs, …
    "iac": [],                                   // [IacTool]: terraform, pulumi, ansible, …
    "observability": [],                         // [ObservabilityTool]: sentry, datadog, prometheus, …
    "auth_providers": [],                        // [AuthProvider]: clerk, auth0, supabase_auth, firebase_auth, …
    "payment_processors": [],                    // [PaymentProcessor]: stripe, paddle, …

    // Optional domain sub-records — present only when the relevant app_type or strong signals appear.
    "web": null,
    "mobile": null,
    "desktop": null,
    "game": {
      "engines": ["unity"],                      // [GameEngine]: unity, godot, unreal, bevy, defold, gamemaker, custom
      "engine_version": "2022.3.42f1",           // best-effort string
      "rendering_pipelines": ["urp"],            // [RenderingPipeline]: urp, hdrp, builtin, vulkan, metal, opengl, directx, custom
      "shader_languages": ["hlsl", "shaderlab"], // [ShaderLanguage]: hlsl, glsl, wgsl, metal, shaderlab, godot_shader
      "physics_engines": ["physx"],              // [PhysicsEngine]: physx, box2d, jolt, rapier, godot_physics, custom
      "networking": []                           // [GameNetworking]: photon, mirror, netcode_for_gameobjects, steam_networking, …
    },
    "extension": null                            // populated for browser_extension, editor_extension, cms_plugin, mcp_server, claude_skill
  },

  "scorecard": {
    "rules_evaluated": 142,
    "rules_fired": 8,
    "elapsed_ms": 3.4,
    "input_summary": { "files_scanned": 412, "bytes_scanned": 1832004 },
    "ignored_paths": [".git", "Library", "Temp", "obj", "bin"],
    "fires": [
      {
        "rule_id": "unity-engine",
        "weight": 1.0,
        "evidence": [
          { "kind": "file_exists", "path": "ProjectSettings/ProjectSettings.asset", "matched": true },
          { "kind": "glob",        "pattern": "Assets/**/*.cs",                     "matched_count": 142 },
          { "kind": "content",     "file": "ProjectSettings/ProjectVersion.txt",
            "regex": "m_EditorVersion:\\s*(\\S+)",
            "captures": ["2022.3.42f1"] }
        ],
        "contributes_to": [
          { "field": "app_type",                       "value": "game",        "delta": 1.0 },
          { "field": "tech_stack.game.engines",        "value": "unity" },
          { "field": "tech_stack.game.engine_version", "value": "2022.3.42f1" },
          { "field": "tech_stack.languages",           "value": "csharp" },
          { "field": "tech_stack.build_systems",       "value": "unity" }
        ]
      }
      // … one entry per fired rule …
    ],
    "warnings": []                                // e.g. "rule 'foo' had a regex that compiled but never matched against an oversized file"
  }
}
```

### Output formats

The same `DetectionReport` is serialized in multiple forms; consumers pick whatever fits.

| Format         | Producer                                  | Intended consumer                              |
|----------------|-------------------------------------------|------------------------------------------------|
| `json`         | `report.to_json()` (pretty or compact)    | services, downstream tools, archives           |
| `text`         | `report.to_human_readable()`              | terminals, PR comments, CI logs                |
| `tsv`          | `report.to_tsv()`                         | shell pipelines: `app_type\tprimary_language\tprimary_build_system\tconfidence` |
| `fires-jsonl`  | `report.scorecard.fires_jsonl()`          | `jq -c` pipelines, audit logs                  |

The `text` format is rendered *from* the scorecard on the consumer's side (the renderer is part of the library but it consumes only the JSON). That keeps the human report and the machine report from drifting apart.

### Worked example 1: a Unity game with URP shaders and GitHub Actions

Input directory tree (abridged):

```
my-game/
├── Assets/
│   ├── Scripts/Player.cs              (132 more .cs files)
│   └── Shaders/Water.shader           (7 more .shader files in HLSL/ShaderLab)
├── Packages/manifest.json             (references com.unity.render-pipelines.universal)
├── ProjectSettings/
│   ├── ProjectSettings.asset
│   └── ProjectVersion.txt             (m_EditorVersion: 2022.3.42f1)
├── .github/workflows/build.yml
└── README.md
```

JSON output (`app-type-detector detect ./my-game --format json`):

```json
{
  "schema_version": 1,
  "ruleset_version": "1.4.0",
  "app_type": {
    "primary": "game",
    "confidence": 0.97,
    "alternatives": []
  },
  "tech_stack": {
    "languages": {
      "primary": "csharp",
      "all": [
        { "language": "csharp",    "role": "gameplay", "file_count": 142 },
        { "language": "hlsl",      "role": "shaders",  "file_count": 8 },
        { "language": "shaderlab", "role": "shaders",  "file_count": 8 }
      ]
    },
    "build_systems": ["unity"],
    "package_managers": ["nuget", "unity_package_manager"],
    "frameworks": [],
    "runtimes": ["mono"],
    "platforms": ["windows", "macos", "ios", "android"],
    "databases": [],
    "caches": [],
    "queues": [],
    "storage": [],
    "testing": [],
    "linting": [],
    "formatting": [],
    "ci": ["github_actions"],
    "containerization": [],
    "orchestration": [],
    "iac": [],
    "observability": [],
    "auth_providers": [],
    "payment_processors": [],
    "web": null,
    "mobile": null,
    "desktop": null,
    "game": {
      "engines": ["unity"],
      "engine_version": "2022.3.42f1",
      "rendering_pipelines": ["urp"],
      "shader_languages": ["hlsl", "shaderlab"],
      "physics_engines": ["physx"],
      "networking": []
    },
    "extension": null
  },
  "scorecard": {
    "rules_evaluated": 142,
    "rules_fired": 6,
    "elapsed_ms": 3.4,
    "input_summary": { "files_scanned": 412, "bytes_scanned": 1832004 },
    "ignored_paths": [".git", "Library", "Temp", "obj", "bin"],
    "fires": [
      {
        "rule_id": "unity-engine",
        "weight": 1.0,
        "evidence": [
          { "kind": "file_exists", "path": "ProjectSettings/ProjectSettings.asset", "matched": true },
          { "kind": "glob", "pattern": "Assets/**/*.cs", "matched_count": 142 },
          { "kind": "content", "file": "ProjectSettings/ProjectVersion.txt",
            "regex": "m_EditorVersion:\\s*(\\S+)", "captures": ["2022.3.42f1"] }
        ],
        "contributes_to": [
          { "field": "app_type", "value": "game", "delta": 1.0 },
          { "field": "tech_stack.game.engines", "value": "unity" },
          { "field": "tech_stack.game.engine_version", "value": "2022.3.42f1" },
          { "field": "tech_stack.languages", "value": "csharp" },
          { "field": "tech_stack.build_systems", "value": "unity" },
          { "field": "tech_stack.runtimes", "value": "mono" }
        ]
      },
      {
        "rule_id": "unity-urp-pipeline",
        "weight": 0.6,
        "evidence": [
          { "kind": "content", "file": "Packages/manifest.json",
            "regex": "com\\.unity\\.render-pipelines\\.universal", "captures": [] }
        ],
        "contributes_to": [
          { "field": "tech_stack.game.rendering_pipelines", "value": "urp" }
        ]
      },
      {
        "rule_id": "shaderlab-and-hlsl",
        "weight": 0.4,
        "evidence": [
          { "kind": "glob", "pattern": "Assets/**/*.shader", "matched_count": 8 }
        ],
        "contributes_to": [
          { "field": "tech_stack.game.shader_languages", "value": "shaderlab" },
          { "field": "tech_stack.game.shader_languages", "value": "hlsl" },
          { "field": "tech_stack.languages", "value": "hlsl" },
          { "field": "tech_stack.languages", "value": "shaderlab" }
        ]
      },
      {
        "rule_id": "github-actions",
        "weight": 0.2,
        "evidence": [
          { "kind": "glob", "pattern": ".github/workflows/*.yml", "matched_count": 1 }
        ],
        "contributes_to": [
          { "field": "tech_stack.ci", "value": "github_actions" }
        ]
      }
    ],
    "warnings": []
  }
}
```

Pretty-text rendering (`app-type-detector detect ./my-game --format text`):

```
my-game

App Type
  game (97%)

Tech Stack
  Languages       C# (primary), HLSL, ShaderLab
  Build System    Unity 2022.3.42f1
  Package Mgr     NuGet, Unity Package Manager
  Runtime         Mono
  Platforms       Windows, macOS, iOS, Android
  CI              GitHub Actions

Game
  Engine          Unity 2022.3.42f1
  Rendering       URP (Universal Render Pipeline)
  Shaders         HLSL, ShaderLab
  Physics         PhysX

Scorecard (6/142 rules fired in 3.4 ms · ruleset v1.4.0)
  ✓ unity-engine            w=1.00   →  app_type=game, engine=unity@2022.3.42f1, language=csharp
  ✓ unity-urp-pipeline      w=0.60   →  rendering_pipeline=urp
  ✓ shaderlab-and-hlsl      w=0.40   →  shader_languages=hlsl, shaderlab
  ✓ github-actions          w=0.20   →  ci=github_actions
```

TSV rendering: `game\tcsharp\tunity\t0.97`

### Worked example 2: a Next.js + PostgreSQL + Stripe web app

Input directory tree (abridged):

```
my-saas/
├── package.json              (deps: next, react, prisma, @prisma/client, stripe, tailwindcss, …)
├── prisma/schema.prisma      (provider = "postgresql")
├── next.config.mjs
├── tailwind.config.ts
├── app/                      (Next.js App Router)
├── Dockerfile
├── docker-compose.yml        (postgres, redis services)
├── .github/workflows/ci.yml
├── vitest.config.ts
└── .eslintrc.json
```

JSON (relevant excerpts):

```json
{
  "schema_version": 1,
  "app_type": { "primary": "web_app", "confidence": 0.94, "alternatives": [
    { "value": "web_api", "confidence": 0.31 }
  ] },
  "tech_stack": {
    "languages": {
      "primary": "typescript",
      "all": [
        { "language": "typescript", "role": "application", "file_count": 87 },
        { "language": "css",        "role": "styles",      "file_count": 4 }
      ]
    },
    "build_systems": ["npm"],
    "package_managers": ["npm"],
    "frameworks": [],
    "runtimes": ["node"],
    "platforms": ["web"],
    "databases": ["postgres", "redis"],
    "caches": ["redis"],
    "queues": [],
    "storage": [],
    "testing": ["vitest"],
    "linting": ["eslint"],
    "formatting": [],
    "ci": ["github_actions"],
    "containerization": ["docker"],
    "orchestration": [],
    "iac": [],
    "observability": [],
    "auth_providers": [],
    "payment_processors": ["stripe"],
    "web": {
      "backend_frameworks": ["nextjs"],
      "frontend_frameworks": ["react"],
      "css_frameworks": ["tailwindcss"],
      "bundlers": ["turbopack"],
      "ssr_strategy": "hybrid",
      "api_styles": ["rest"],
      "orms": ["prisma"]
    },
    "mobile": null,
    "desktop": null,
    "game": null,
    "extension": null
  },
  "scorecard": {
    "rules_evaluated": 142,
    "rules_fired": 11,
    "elapsed_ms": 2.1,
    "fires": [
      {
        "rule_id": "nextjs-app",
        "weight": 1.0,
        "evidence": [
          { "kind": "content", "file": "package.json", "regex": "\"next\"\\s*:", "captures": [] },
          { "kind": "file_exists", "path": "next.config.mjs", "matched": true }
        ],
        "contributes_to": [
          { "field": "app_type", "value": "web_app", "delta": 1.0 },
          { "field": "tech_stack.web.backend_frameworks", "value": "nextjs" },
          { "field": "tech_stack.web.frontend_frameworks", "value": "react" },
          { "field": "tech_stack.languages", "value": "typescript" },
          { "field": "tech_stack.runtimes", "value": "node" }
        ]
      },
      {
        "rule_id": "prisma-postgres",
        "weight": 0.7,
        "evidence": [
          { "kind": "content", "file": "prisma/schema.prisma",
            "regex": "provider\\s*=\\s*\"postgresql\"", "captures": [] }
        ],
        "contributes_to": [
          { "field": "tech_stack.web.orms", "value": "prisma" },
          { "field": "tech_stack.databases", "value": "postgres" }
        ]
      },
      {
        "rule_id": "tailwindcss",
        "weight": 0.3,
        "evidence": [
          { "kind": "file_exists", "path": "tailwind.config.ts", "matched": true }
        ],
        "contributes_to": [
          { "field": "tech_stack.web.css_frameworks", "value": "tailwindcss" }
        ]
      },
      {
        "rule_id": "stripe-payments",
        "weight": 0.4,
        "evidence": [
          { "kind": "content", "file": "package.json", "regex": "\"stripe\"\\s*:", "captures": [] }
        ],
        "contributes_to": [
          { "field": "tech_stack.payment_processors", "value": "stripe" }
        ]
      },
      {
        "rule_id": "docker-compose-redis",
        "weight": 0.3,
        "evidence": [
          { "kind": "content", "file": "docker-compose.yml", "regex": "image:\\s*redis", "captures": [] }
        ],
        "contributes_to": [
          { "field": "tech_stack.caches", "value": "redis" },
          { "field": "tech_stack.databases", "value": "redis" },
          { "field": "tech_stack.containerization", "value": "docker" }
        ]
      }
    ],
    "warnings": []
  }
}
```

Pretty-text rendering:

```
my-saas

App Type
  web_app (94%)
  · web_api (31%)

Tech Stack
  Languages       TypeScript (primary), CSS
  Build System    npm
  Runtime         Node
  Platforms       Web
  Databases       PostgreSQL, Redis
  Caches          Redis
  Containers      Docker
  Testing         Vitest
  Linting         ESLint
  CI              GitHub Actions
  Payments        Stripe

Web
  Backend         Next.js
  Frontend        React
  Styling         Tailwind CSS
  Bundler         Turbopack
  SSR             hybrid
  API style       REST
  ORM             Prisma

Scorecard (11/142 rules fired in 2.1 ms · ruleset v1.4.0)
  ✓ nextjs-app              w=1.00   →  app_type=web_app, framework=nextjs, language=typescript, runtime=node
  ✓ prisma-postgres         w=0.70   →  orm=prisma, database=postgres
  ✓ stripe-payments         w=0.40   →  payment_processor=stripe
  ✓ tailwindcss             w=0.30   →  css_framework=tailwindcss
  ✓ docker-compose-redis    w=0.30   →  cache=redis, database=redis, container=docker
  ✓ vitest-config           w=0.20   →  testing=vitest
  ✓ eslintrc                w=0.20   →  linting=eslint
  ✓ github-actions          w=0.20   →  ci=github_actions
```

### Worked example 3: a polyglot / uncertain repo

Input: a directory with `README.md`, `Cargo.toml`, `pyproject.toml`, `package.json`, but no obvious entry point — the kind of monorepo that defeats most heuristics.

JSON (excerpts):

```json
{
  "schema_version": 1,
  "app_type": {
    "primary": null,
    "confidence": 0.0,
    "alternatives": [
      { "value": "library", "confidence": 0.45 },
      { "value": "cli_tool", "confidence": 0.30 }
    ]
  },
  "tech_stack": {
    "languages": {
      "primary": null,
      "all": [
        { "language": "rust",       "role": "library",  "file_count": 12 },
        { "language": "python",     "role": "tooling",  "file_count": 5 },
        { "language": "typescript", "role": "tooling",  "file_count": 3 }
      ]
    },
    "build_systems": ["cargo", "uv", "npm"],
    "package_managers": ["cargo", "uv", "npm"],
    "platforms": [],
    "web": null, "mobile": null, "desktop": null, "game": null, "extension": null
  },
  "scorecard": {
    "rules_fired": 5,
    "warnings": [
      "no rule dominated for app_type (top two within 1.5x weight margin); leaving primary as null"
    ]
  }
}
```

Pretty-text rendering:

```
(unnamed)

App Type
  unable to determine a single app type (no rule dominated)
  Candidates:
    · library (45%)
    · cli_tool (30%)

Tech Stack
  Languages       Rust (12 files), Python (5 files), TypeScript (3 files)
  Build Systems   Cargo, uv, npm
  Package Mgrs    cargo, uv, npm
```

Returning `null` for `app_type.primary` is the correct, non-lying behavior: consumers can fall back to a human or an LLM with full context. The library never guesses past its confidence threshold.

## Relevant Files

The current `app-type-detector` working directory is a fresh scaffold (only `README.md`, `.gitignore`, and empty subdirectories). Every file below this section is new.

There are two **motivating reference implementations** of this same problem living in unrelated repos. They are *not* requirements, dependencies, or test fixtures for this library. They informed which initial `AppType` values and which initial `TechStack` facets to ship at v0.1.0. Nothing more:

- `~/Developer/__versioning_projects/.versioning-scan.config.json` — a JSON ruleset for build-system detection. Useful as a sanity check that the default ruleset's `TechStack.build_systems` and `TechStack.package_managers` cover XcodeGen, raw Xcode, SPM, Gradle, npm, Cargo, uv, Go, Godot, Unity, WordPress.
- `~/Developer/auto-build-log/app/buildlog-worker/src/agents/enricher/{kind-inference.ts, tools/github-read.ts}` and `app/agents/detective/strategies/types.ts` — useful as a sanity check that the default ruleset's `AppType` covers the realistic v0.1.0 set (web_app, web_api, mobile_app, desktop_app, game, cli_tool, library, daemon, mcp_server, claude_skill, browser_extension, editor_extension, cms_plugin, static_site, unknown).

Neither file is read by the library; neither tool's vocabulary is adopted verbatim; neither tool's migration is scoped here.

### New Files

#### Rust workspace

- `app/Cargo.toml` — workspace root.
- `app/rust-toolchain.toml` — pin a stable Rust toolchain (1.82+).
- `app/.cargo/config.toml` — workspace-wide lints (`unsafe_code = "forbid"`, opt-in `clippy::pedantic`).

##### `app/crates/app-type-detector/` — the core crate

- `app/crates/app-type-detector/Cargo.toml` — default features `fs`, `serde`, `default-rules`, `human-renderer`.
- `app/crates/app-type-detector/src/lib.rs` — public API and re-exports.
- `app/crates/app-type-detector/src/types/mod.rs` — top-level type re-exports.
- `app/crates/app-type-detector/src/types/app_type.rs` — `AppType` enum: `WebApp`, `WebApi`, `StaticSite`, `MobileApp`, `DesktopApp`, `Game`, `CliTool`, `Library`, `Daemon`, `BrowserExtension`, `EditorExtension`, `CmsPlugin`, `McpServer`, `ClaudeSkill`, `Unknown`. `#[non_exhaustive]`. Note: no platform name (iOS, macOS) and no engine name (Unity, Godot) appears in this enum; those live in `TechStack`.
- `app/crates/app-type-detector/src/types/tech_stack.rs` — `TechStack` struct with the rich field set described in **Output Format**. Domain sub-records (`WebStack`, `MobileStack`, `DesktopStack`, `GameStack`, `ExtensionStack`) are separate structs with their own enums (`GameEngine`, `RenderingPipeline`, `ShaderLanguage`, `PhysicsEngine`, `GameNetworking`, `BackendFramework`, `FrontendFramework`, `Bundler`, `CssFramework`, `Orm`, `ApiStyle`, `SsrStrategy`, `UiFramework`, `MobilePlatformVersion`, `DesktopShell`, `InstallerFormat`, `ExtensionHost`, …). All enums are `#[non_exhaustive]`.
- `app/crates/app-type-detector/src/types/enums.rs` — flat-namespace enums: `Language`, `BuildSystem`, `PackageManager`, `Runtime`, `Platform`, `Database`, `Cache`, `Queue`, `Storage`, `TestFramework`, `Linter`, `Formatter`, `CiSystem`, `Containerizer`, `Orchestrator`, `IacTool`, `ObservabilityTool`, `AuthProvider`, `PaymentProcessor`. Each variant carries a `serde` rename to match the JSON spec exactly.
- `app/crates/app-type-detector/src/types/report.rs` — `DetectionReport`, `AppTypeFinding`, `LanguagesFinding`, `LanguageUsage`, `Confidence`, `Alternative`.
- `app/crates/app-type-detector/src/types/scorecard.rs` — `Scorecard`, `Fire`, `Evidence`, `Contribution`, `InputSummary`. Includes `Scorecard::fires_jsonl()` for streaming.
- `app/crates/app-type-detector/src/rules.rs` — declarative rule grammar. A rule is a `MatchExpr` plus a `RulePayload` that lists the `Contribution`s the rule wants to make. The synthesizer is the only thing that decides whether a contribution wins.
- `app/crates/app-type-detector/src/engine.rs` — pure rule evaluation against an `InputSnapshot`, producing a `Vec<Fire>`.
- `app/crates/app-type-detector/src/snapshot.rs` — `InputSnapshot` trait + `MemorySnapshot` + `FilesystemSnapshot` (gated behind `fs`; `walkdir` + `globset`; curated ignore list `.git`, `node_modules`, `dist`, `.next`, `target`, `build`, `Library`, `Temp`, `obj`, `bin`, `.venv`, `.gradle`, `Pods`, `DerivedData`).
- `app/crates/app-type-detector/src/synthesis.rs` — combines `Vec<Fire>` into a `DetectionReport` with `app_type`, `tech_stack`, `scorecard`. Conflict resolution: per-field accumulation by weight, primary-selection requires the leader to exceed the runner-up by ≥1.5× weight (configurable), otherwise primary is `None` and a warning is emitted.
- `app/crates/app-type-detector/src/render.rs` — `report.to_human_readable()` Markdown / plain-text renderer (gated behind `human-renderer`). Consumes only the JSON shape, never internal types, so the renderer is identical across bindings.
- `app/crates/app-type-detector/src/default_rules.rs` — `pub fn default_ruleset() -> &'static Ruleset` cached in a `OnceLock`.
- `app/crates/app-type-detector/src/default_rules.json` — human-edited source of truth for the bundled rules. Adding a new ecosystem is a JSON edit + test fixture.
- `app/crates/app-type-detector/build.rs` — validates `default_rules.json` parses and that every `AppType` in the initial set is referenced by at least one rule.
- `app/crates/app-type-detector/tests/fixtures/` — minimal directory trees per scenario: `unity-game`, `godot-game`, `bevy-game`, `nextjs-postgres-saas`, `astro-static-site`, `fastapi-api`, `swiftui-ios-app`, `kotlin-android-app`, `tauri-desktop-app`, `electron-desktop-app`, `menubar-mac-app`, `cli-rust`, `cli-python`, `library-rust`, `library-typescript`, `mcp-server-typescript`, `mcp-server-python`, `claude-skill`, `chrome-extension`, `vscode-extension`, `obsidian-plugin`, `wordpress-plugin`, `polyglot-monorepo`, `empty-dir`, `git-only-dir`.
- `app/crates/app-type-detector/tests/detect_path.rs` — `detect_path` against every fixture; assert the full `DetectionReport`.
- `app/crates/app-type-detector/tests/detect_memory.rs` — same fixtures via `MemorySnapshot`; assert byte-identical reports.
- `app/crates/app-type-detector/tests/scorecard.rs` — every fixture's `scorecard.fires` is non-empty (or for `empty-dir`, properly empty with a warning), every `contributes_to` references a real field path, every regex has either a `captures` array or an explicit empty one.
- `app/crates/app-type-detector/tests/render.rs` — golden-file comparison: `report.to_human_readable()` against committed snapshot strings per fixture.
- `app/crates/app-type-detector/tests/polyglot.rs` — multi-language and multi-framework projects produce explainable, deterministic output and emit the correct `warnings`.
- `app/crates/app-type-detector/tests/edge_cases.rs` — empty dirs, broken UTF-8, oversized files (>64 KB), symlink loops, missing paths.
- `app/crates/app-type-detector/benches/detect.rs` — Criterion bench, target <5 ms.

##### `app/crates/app-type-detector-cli/` — the CLI

- `app/crates/app-type-detector-cli/Cargo.toml`.
- `app/crates/app-type-detector-cli/src/main.rs` — `clap`-based CLI: `app-type-detector detect [PATH] [--rules FILE] [--format json|text|tsv|fires-jsonl] [--no-evidence] [--margin FLOAT]`. Default `PATH` is `.`, default `--format` is `text`.
- `app/crates/app-type-detector-cli/tests/cli.rs` — `assert_cmd` smoke tests verifying every output format.

##### `app/bindings/node/` — npm package

- `app/bindings/node/Cargo.toml` — `napi-rs` crate.
- `app/bindings/node/src/lib.rs` — N-API exports: `detectPath(path)`, `detectFiles({files, rootDirs})`, `defaultRuleset()`, `renderHumanReadable(report)`. `napi-derive` produces TS types.
- `app/bindings/node/package.json` — `name: "@snam/app-type-detector"`.
- `app/bindings/node/index.js` — `napi-rs` dispatcher.
- `app/bindings/node/index.d.ts` — generated, committed.
- `app/bindings/node/__test__/index.test.ts` — `vitest` round-trip test, golden snapshot per fixture.
- `app/bindings/node/README.md` — npm-facing docs with the same worked examples.

##### `app/bindings/python/` — PyPI package

- `app/bindings/python/Cargo.toml` — `pyo3` crate.
- `app/bindings/python/src/lib.rs` — `pyo3` module exposing `detect_path`, `detect_files`, `default_ruleset`, `render_human_readable`.
- `app/bindings/python/pyproject.toml` — `maturin` build backend.
- `app/bindings/python/python/app_type_detector/__init__.py` — re-exports + dataclass typing stubs.
- `app/bindings/python/python/app_type_detector/py.typed` — PEP 561 marker.
- `app/bindings/python/tests/test_detect.py` — `pytest` parity test.

#### Examples

- `app/examples/rust-fs-walk/` — `cargo run -- /path/to/repo`.
- `app/examples/node-fixture/` — `node index.js` showing both surfaces.
- `app/examples/python-fixture/` — Python mirror.
- `app/examples/consumer-mapping/` — a 30-line lookup table illustrating the recommended pattern for translating this library's `AppType` and `TechStack` into a downstream tool's own taxonomy. **No tool-specific code lives in the library.**

#### Scripts

- `scripts/dev.sh`, `scripts/start.sh`, `scripts/test-all.sh`, `scripts/lint-all.sh`, `scripts/build-rules.sh`, `scripts/release-crate.sh`, `scripts/release-npm.sh`, `scripts/release-pypi.sh`.

#### CI

- `.github/workflows/ci.yml`, `.github/workflows/release-crate.yml`, `.github/workflows/release-npm.yml`, `.github/workflows/release-pypi.yml`.

#### Docs

- `docs/00-overview.md` — what the library is and is not.
- `docs/01-vocabulary.md` — every `AppType` value, every `Language`, every `Framework`, every domain sub-record. One-paragraph definition + canonical example each.
- `docs/02-output-format.md` — the schema, with the three worked examples from this spec.
- `docs/03-rules.md` — the rule grammar.
- `docs/04-rust-usage.md`, `docs/05-node-usage.md`, `docs/06-python-usage.md`, `docs/07-cli-usage.md`.
- `docs/08-extending.md` — adding a new rule.
- `docs/09-consumer-mapping.md` — the recommended pattern for downstream tools.
- `docs/RULES.md` — generated reference of every bundled rule.
- `ai_docs/napi-rs-overview.md`, `ai_docs/pyo3-maturin-overview.md`.

#### Project metadata

- `LICENSE` (MIT), `CHANGELOG.md` with three release tracks.

## Implementation Plan

### Phase 1: Foundation

Stand up the Rust workspace and pin the public type surface. Define `AppType`, `TechStack` (with all domain sub-records), `Scorecard`, `Fire`, `Evidence`, `Contribution`, `DetectionReport`. Author the JSON wire-format documentation in `docs/02-output-format.md` with the three worked examples. Land the default ruleset as a JSON file plus a `build.rs` that validates it. Author 24+ minimal fixtures (one per scenario in **Output Format**) and prove they round-trip through `MemorySnapshot` end-to-end before any binding code exists. Wire CI to run fmt + clippy + nextest + golden-file diff on every push. Phase ends when `cargo nextest run` passes against the fixture suite.

### Phase 2: Core Implementation

Implement the rule engine, the FS-backed snapshot (with the curated ignore list and depth cap), and the synthesizer that combines `Vec<Fire>` into a `DetectionReport`. The synthesizer is the only place that decides the primary `AppType` (using the configurable margin), aggregates per-field contributions into the `TechStack`, and emits warnings. Add the polyglot test suite. Add the edge-cases suite. Implement the human-readable renderer that consumes the JSON shape (not internal types) so it can later be reused unchanged by the bindings. Ship the CLI binary with `json`, `text`, `tsv`, and `fires-jsonl` formats. Add the Criterion benchmark and confirm <5 ms on a typical project.

### Phase 3: Bindings and Distribution

Build the two language bindings:

- **Node** via `napi-rs`: per-triple matrix in CI, prebuilt binaries published as optional-dep subpackages, WASM fallback. `napi-derive` generates the TS types. Vitest golden-snapshot test against the same fixtures. Surface `renderHumanReadable(report)` so the same renderer ships in JS.
- **Python** via `pyo3` + `maturin`: ABI3 wheels for the same triples + Python 3.10–3.13. `py.typed` ships. `pytest` golden-snapshot test against the same fixtures. Surface `render_human_readable(report)` so the same renderer ships in Python.

Tag and publish `crate-v0.1.0`, `npm-v0.1.0`, `pypi-v0.1.0`. Confirm `cargo add app-type-detector`, `npm i @snam/app-type-detector`, `pip install app-type-detector` all install and run a hello-world detection on a fresh machine.

## Step by Step Tasks

IMPORTANT: Execute every step in order, top to bottom.

### 1. Project metadata

- Add `LICENSE` (MIT).
- Update `README.md` with the elevator pitch, install snippets per ecosystem, and a 10-line usage example per ecosystem that prints the JSON for a sample fixture.
- Create empty `CHANGELOG.md` with three sections (Rust crate, npm, PyPI).

### 2. Scaffold the Rust workspace

- Write `app/Cargo.toml`, `app/rust-toolchain.toml`, `app/.cargo/config.toml`.
- `cargo new --lib app/crates/app-type-detector` and prune.
- Verify `cd app && cargo check --workspace` passes.

### 3. Pin the public type surface

- Implement every type listed in **Relevant Files → core crate** (`AppType`, `TechStack`, `WebStack`, `MobileStack`, `DesktopStack`, `GameStack`, `ExtensionStack`, all flat enums, `DetectionReport`, `Scorecard`, `Fire`, `Evidence`, `Contribution`).
- Use `serde` rename attributes so the JSON shape exactly matches **Output Format → Schema overview**.
- Mark every enum `#[non_exhaustive]` so future variants are minor-version-safe.
- Add `lib.rs` re-exports.
- Add a single integration test `types_serialize.rs` that constructs the example reports from **Output Format → Worked example 1, 2, 3** and asserts they round-trip through `serde_json` byte-for-byte.

### 4. Define the rule grammar

- In `src/rules.rs` define `MatchExpr` (`FileExists`, `Glob`, `Content { file, regex }`, `All`, `Any`, `Not`).
- Define `RulePayload` as `Vec<Contribution>` with optional `confidence_weight: f32` (default 1.0).
- Define `Rule { id, when: MatchExpr, payload: RulePayload, evidence_label: String }`.
- Define `Ruleset { schema_version: u32, rules: Vec<Rule> }` with `Ruleset::from_json` and `Ruleset::extend` (for callers that want to add to the defaults).
- Document the grammar in doc comments with worked examples.
- Unit-test each `MatchExpr` variant.

### 5. Author the default ruleset

- Write `src/default_rules.json` with one or more rules per supported `AppType` and per common `TechStack` facet. Examples (non-exhaustive):
  - `unity-engine` → contributes `app_type=game`, `tech_stack.game.engines=unity`, `tech_stack.languages=csharp`, `tech_stack.build_systems=unity`, `tech_stack.runtimes=mono`. Captures `tech_stack.game.engine_version` from `ProjectVersion.txt`.
  - `unity-urp-pipeline`, `unity-hdrp-pipeline`, `unity-builtin-pipeline` → contribute the rendering pipeline based on `Packages/manifest.json` content.
  - `godot-engine` → contributes `app_type=game`, `tech_stack.game.engines=godot`, `tech_stack.languages=gdscript`. Captures engine version from `project.godot`.
  - `bevy-engine` → contributes `app_type=game`, `tech_stack.game.engines=bevy`, `tech_stack.languages=rust`.
  - `nextjs-app` → contributes `app_type=web_app`, `tech_stack.web.backend_frameworks=nextjs`, `tech_stack.web.frontend_frameworks=react`.
  - `astro-static-site` → contributes `app_type=static_site`, `tech_stack.web.backend_frameworks=astro`.
  - `fastapi-api` → contributes `app_type=web_api`, `tech_stack.web.backend_frameworks=fastapi`, `tech_stack.languages=python`.
  - `swiftui-ios-app` → contributes `app_type=mobile_app`, `tech_stack.mobile.ui_frameworks=swiftui`, `tech_stack.platforms=ios`, `tech_stack.languages=swift`.
  - `jetpack-compose-android` → contributes `app_type=mobile_app`, `tech_stack.mobile.ui_frameworks=jetpack_compose`, `tech_stack.platforms=android`, `tech_stack.languages=kotlin`.
  - `tauri-desktop` → contributes `app_type=desktop_app`, `tech_stack.desktop.shells=tauri`, `tech_stack.languages=rust`.
  - `electron-desktop` → contributes `app_type=desktop_app`, `tech_stack.desktop.shells=electron`, `tech_stack.languages=typescript`.
  - `mcp-server-typescript`, `mcp-server-python` → contribute `app_type=mcp_server`, `tech_stack.extension.host=mcp_client`.
  - `claude-skill` → contributes `app_type=claude_skill` when `SKILL.md` exists at root.
  - `chrome-extension`, `firefox-extension`, `vscode-extension`, `obsidian-plugin`, `wordpress-plugin` → corresponding extension classifications.
  - `prisma-postgres`, `prisma-mysql`, `prisma-sqlite`, `drizzle-postgres`, `sqlalchemy-postgres`, etc. → contribute `tech_stack.web.orms` and `tech_stack.databases`.
  - `tailwindcss`, `unocss`, `bootstrap`, `chakra-ui` → CSS frameworks.
  - `vitest`, `jest`, `pytest`, `xctest`, `cargo-test` → testing frameworks.
  - `eslint`, `ruff`, `clippy`, `swiftlint` → linters.
  - `github-actions`, `gitlab-ci`, `circleci` → CI systems.
  - `docker`, `podman`, `dockerfile`, `docker-compose-redis`, `docker-compose-postgres` → containerization + databases.
  - `stripe-payments`, `paddle-payments` → payment processors.
  - `clerk-auth`, `auth0`, `supabase-auth` → auth providers.
- Add `build.rs` that `include_str!`s the JSON, parses it, and asserts every `AppType` and every domain sub-record is referenced by at least one rule.
- Add `default_rules.rs` exposing `default_ruleset()`.

### 6. Implement the snapshot abstraction

- `MemorySnapshot { files: HashMap<String, Option<String>>, root: Vec<DirEntry> }`.
- `FilesystemSnapshot` (behind `fs`) using `walkdir` with curated ignore list and depth cap of 4. `.follow_links(false)`.
- Unit-test both against an identical synthetic tree.

### 7. Implement the rule engine

- `evaluate(snapshot: &dyn InputSnapshot, rules: &Ruleset) -> Vec<Fire>`.
- Each `Fire` includes `rule_id`, `weight`, the `Vec<Evidence>` produced (with regex captures expanded), and the `Vec<Contribution>` from the rule's payload.
- Pure function. Deterministic.

### 8. Implement the synthesizer

- `synthesize(fires: Vec<Fire>, config: SynthesisConfig) -> DetectionReport`.
- Per-field aggregation:
  - `app_type`: tally weights per `AppType`, pick the top one if it exceeds the runner-up by ≥`config.dominance_margin` (default 1.5×); otherwise primary is `None`.
  - `tech_stack` list fields: union of every contribution's value, deduped, ordered by total contributing weight.
  - `tech_stack` scalar fields (e.g. `engine_version`, `ssr_strategy`): take the value with the highest contributing weight; ties broken by rule id alphabetical.
- Always populate `scorecard` with every fire (including the ones whose contributions did not win), `rules_evaluated`, `rules_fired`, `elapsed_ms`, `input_summary`, `ignored_paths`, `warnings`.

### 9. Implement the human-readable renderer

- `report.to_human_readable() -> String` that produces the format shown in **Output Format → Worked example 1/2/3**.
- The renderer reads only from the JSON-shaped `DetectionReport` (no private fields), so the bindings can re-export it unchanged.
- Golden-file tests in `tests/render.rs` against committed `.txt` snapshots per fixture.

### 10. Build the CLI

- `clap`-based parser. Default `PATH` is `.`, default `--format` is `text`.
- `--format json` prints the canonical JSON.
- `--format text` prints `to_human_readable()`.
- `--format tsv` prints `primary_app_type\tprimary_language\tprimary_build_system\tconfidence`.
- `--format fires-jsonl` prints one fire per line.
- `--no-evidence` strips the `scorecard.fires[].evidence` arrays for compact output.
- `--margin <FLOAT>` overrides the synthesizer's dominance margin.
- `assert_cmd` tests for every format.

### 11. Build the Node binding

- `napi-rs` exports `detectPath(path)`, `detectFiles({files, rootDirs})`, `defaultRuleset()`, `renderHumanReadable(report)`.
- `package.json` `optionalDependencies` for prebuilt-binary subpackages.
- Vitest golden-snapshot test against the same fixtures, asserting the JSON deep-equals the committed Rust snapshots.

### 12. Build the Python binding

- `pyo3` + `maturin`. ABI3 wheels.
- Expose `detect_path(path)`, `detect_files(snapshot)`, `default_ruleset()`, `render_human_readable(report)`.
- Return Python dataclass-shaped objects (or plain dicts plus a `dataclasses` wrapper, whichever round-trips JSON cleanly).
- `pytest` golden-snapshot test against the same fixtures.

### 13. Wire CI

- `.github/workflows/ci.yml`: matrix `(ubuntu-latest, macos-14)` × `(stable rust, node 20, python 3.12)`. Steps: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `pnpm --dir app/bindings/node test`, `uv --directory app/bindings/python sync && uv run pytest`.
- Release workflows for crates.io, npm, PyPI.

### 14. Author docs

- Write the ten numbered docs.
- `docs/02-output-format.md` is the canonical schema reference; the worked examples from this spec ship there verbatim.
- Wire `scripts/build-rules.sh` to regenerate `docs/RULES.md` from `default_rules.json`.
- Write `app/examples/consumer-mapping/` to demonstrate the lookup-table pattern on the consumer's side.

### 15. Cut v0.1.0 releases

- Bump all three packages to `0.1.0`.
- Tag `crate-v0.1.0`, `npm-v0.1.0`, `pypi-v0.1.0`.
- Confirm artifacts install on a fresh machine.

### 16. Run full validation

- Execute every command in **Validation Commands**.

## Testing Strategy

### Unit Tests

- **Rule grammar** (`rules.rs`): each `MatchExpr` variant in isolation, including nested `All`/`Any`/`Not` and content regex compilation errors.
- **Default ruleset loading**: parse `default_rules.json`; assert every regex compiles, every `AppType` and every domain sub-record is referenced by at least one rule, every rule id is unique, every contribution's `field` path is a real `DetectionReport` field path (validated via a small schema map).
- **Snapshot implementations**: `MemorySnapshot::glob` and `FilesystemSnapshot::glob` produce identical results on the same synthetic tree.
- **Engine**: `evaluate` produces deterministic fires.
- **Synthesizer**: ties, near-ties, single-rule wins, multi-rule wins, conflicting rules, and the `dominance_margin` knob.
- **Renderer**: golden snapshots per fixture (so format drift is caught).

### Integration Tests

- **Filesystem fixtures** (`tests/detect_path.rs`): one per scenario, asserting the full `DetectionReport`.
- **Memory parity** (`tests/detect_memory.rs`): byte-identical to the FS path.
- **Scorecard invariants** (`tests/scorecard.rs`): every fire's contributions reference real fields; every regex with captures emits them; warnings are populated when expected.
- **Renderer goldens** (`tests/render.rs`): `to_human_readable()` matches a committed `.txt` snapshot per fixture.
- **Polyglot** (`tests/polyglot.rs`): primary `app_type` is correctly `None` when no rule dominates; warnings explain why.
- **Edge cases** (`tests/edge_cases.rs`): empty dirs, broken UTF-8, oversized files, symlink loops, missing paths.
- **CLI** (`tests/cli.rs`): each output format produces the documented shape.
- **Node binding** (`__test__/index.test.ts`): JSON deep-equals committed Rust snapshots.
- **Python binding** (`tests/test_detect.py`): same.

### Edge Cases

- Empty directory → `app_type.primary = null`, low confidence, `tech_stack.languages.primary = null`, `scorecard.fires = []`, `warnings` includes `"no rules fired"`.
- Directory with only `.git/` → same as empty.
- Directory containing only ignored paths → same as empty.
- Polyglot codebase (Rust + Python + Node together) → all three appear in `tech_stack.languages.all`; `primary` is whichever wins by file count and contributing weight; `app_type.primary` is `null` if no rule dominates.
- Project with both Unity and a sibling web admin → `app_type.primary = game` (Unity rule dominates), but `tech_stack.web` is also populated and surfaced in the report.
- Engine version detection failure (e.g. `ProjectVersion.txt` malformed) → `tech_stack.game.engine_version` is `null`; warning emitted; `app_type` still resolves correctly.
- File larger than 64 KB → truncated before content regex; documented cap.
- Memory snapshot with `None` for a known path → treated as "file does not exist".
- Non-UTF-8 file contents → content rules silently skip; existence rules still fire.
- Rule JSON with an unknown matcher key or unknown field path in a contribution → typed error pointing at the offending rule index.
- Caller passes a nonexistent path → `Err`, not panic.
- Symlink loop → `walkdir` `.follow_links(false)` prevents recursion.

## Acceptance Criteria

- [ ] `cargo nextest run --workspace` passes with zero failures and ≥80% coverage on `engine.rs`, `rules.rs`, `synthesis.rs`, `render.rs`.
- [ ] The library returns a `DetectionReport` *object* (never a single string) with the exact JSON shape documented in **Output Format → Schema overview**, validated by serde round-trip tests for the three worked examples.
- [ ] `app_type` and `tech_stack` are orthogonal: `AppType` carries no platform or engine names; engine and platform information lives only in `tech_stack`. Asserted by a CI grep that fails if any `AppType` variant name contains `unity`, `godot`, `ios`, `android`, `mac`, `linux`, `windows`, `web`, `electron`, `tauri`.
- [ ] `tech_stack` carries domain-specific richness: a `game` fixture's report populates `tech_stack.game.engines`, `rendering_pipelines`, `shader_languages`; a `web_app` fixture's report populates `tech_stack.web.backend_frameworks`, `frontend_frameworks`, `databases`, `payment_processors`; a `mobile_app` fixture populates `tech_stack.mobile.ui_frameworks` and `platforms`. Asserted by per-fixture report tests.
- [ ] Every detection produces a `scorecard` with at least one of `fires` non-empty or `warnings` non-empty. Asserted by `tests/scorecard.rs`.
- [ ] `report.to_human_readable()` matches a committed golden file per fixture. Asserted by `tests/render.rs`.
- [ ] All four output formats (`json`, `text`, `tsv`, `fires-jsonl`) are produced from the same in-memory `DetectionReport`. The `text` renderer consumes only the JSON-shaped report (no private types). Asserted by a renderer test that round-trips through `serde_json` first.
- [ ] `npm i @snam/app-type-detector` then `import { detectPath } from '@snam/app-type-detector'` works on Linux x64, Linux arm64, macOS arm64, macOS x64, Windows x64; WASM fallback covers everything else.
- [ ] `pip install app-type-detector` then `from app_type_detector import detect_path` works on the same triples for Python 3.10–3.13.
- [ ] `cargo doc --no-deps` renders cleanly with no missing-docs warnings on public items.
- [ ] The library has zero IO escape hatches in the core crate: no `std::process`, `reqwest`, `hyper`, `tokio::net`. Asserted by a CI grep.
- [ ] `unsafe_code` is forbidden in the core crate.
- [ ] Adding a new ecosystem is a JSON edit + test fixture + (if a new enum variant is needed) a single non-exhaustive enum addition. Documented in `docs/08-extending.md`.
- [ ] Detection of a typical project completes in under 5 ms on macOS arm64 (Criterion bench).
- [ ] The library's vocabulary does not reference any specific consumer or downstream tool, anywhere in code or docs.

## Validation Commands

Execute every command to validate the feature works correctly with zero regressions.

- `cd /Users/seannam/Developer/app-type-detector/app && cargo fmt --all -- --check` — format check
- `cd /Users/seannam/Developer/app-type-detector/app && cargo clippy --workspace --all-targets -- -D warnings` — lint with zero warnings
- `cd /Users/seannam/Developer/app-type-detector/app && cargo nextest run --workspace` — every Rust test (unit + integration + scorecard + render + polyglot + edge cases)
- `cd /Users/seannam/Developer/app-type-detector/app && cargo test --doc --workspace` — doctests
- `cd /Users/seannam/Developer/app-type-detector/app && cargo bench -p app-type-detector --bench detect -- --quick` — perf bench within budget
- `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli -- detect crates/app-type-detector/tests/fixtures/unity-game --format json | jq '.app_type.primary, .tech_stack.game.engines'` — Unity fixture surfaces `"game"` and `["unity"]`
- `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli -- detect crates/app-type-detector/tests/fixtures/godot-game --format json | jq '.app_type.primary, .tech_stack.game.engines'` — Godot fixture surfaces `"game"` and `["godot"]`
- `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli -- detect crates/app-type-detector/tests/fixtures/nextjs-postgres-saas --format json | jq '.tech_stack.web, .tech_stack.databases'` — Next.js fixture surfaces full web sub-record
- `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli -- detect crates/app-type-detector/tests/fixtures/unity-game --format text` — human render snapshot
- `cd /Users/seannam/Developer/app-type-detector/app/bindings/node && pnpm install && pnpm run build && pnpm test` — Node binding builds and passes its parity test
- `cd /Users/seannam/Developer/app-type-detector/app/bindings/python && uv sync && uv run maturin develop && uv run pytest` — Python binding builds and passes its parity test
- `cd /Users/seannam/Developer/app-type-detector && bash scripts/test-all.sh` — single-shot all-language test suite
- `cd /Users/seannam/Developer/app-type-detector && grep -RIn -E "std::process|reqwest|hyper|tokio::net" app/crates/app-type-detector/src && echo "FAIL: forbidden dep referenced" && exit 1 || echo "OK: core crate has no IO escape hatches"`
- `cd /Users/seannam/Developer/app-type-detector && grep -RIn -iE "buildlog|version[-_ ]skill|versioning_projects" app/ docs/ && echo "FAIL: consumer-specific naming leaked" && exit 1 || echo "OK: library is consumer-neutral"`
- `cd /Users/seannam/Developer/app-type-detector && grep -RInE "AppType::(Unity|Godot|Ios|Android|Mac|Linux|Windows|Web|Electron|Tauri)" app/crates/ && echo "FAIL: AppType conflates platform/engine with role" && exit 1 || echo "OK: app_type and tech_stack are orthogonal"`

## Notes

- **Why Rust core, not Go or pure TS?** Rust gives one binary that ships to crates.io, npm (`napi-rs` native addons), and PyPI (`pyo3` + `maturin` wheels) without a runtime. TS would force Node on Python consumers. Go would force CGO across both bindings.
- **Why `napi-rs`, not WASM, for the npm package?** Native addons hit native FS speeds and avoid a sync-FS shim. We keep WASM as a fallback for Edge / Cloudflare Workers via `optionalDependencies`.
- **Why `pyo3` + `maturin`?** ABI3 wheels mean Python 3.10–3.13 share one binary per triple, keeping the wheel matrix small.
- **Why orthogonal `app_type` and `tech_stack`?** A "game" is a different question from "Unity" or "Godot". An "iOS app" is the intersection of `app_type=mobile_app` and `tech_stack.platforms=[ios]`. Conflating them into one string makes downstream queries like "show me all games regardless of engine" or "show me everything built in Unity regardless of platform" needlessly hard. Keeping them separate also lets the same enum live for years while engines and platforms churn.
- **Why a scorecard?** Every detection should be inspectable. A consumer that disagrees with a label should be able to look at the scorecard and see exactly which rule fired and what it matched. This is the antidote to the "LLM said so" anti-pattern.
- **Why a separate human renderer?** Splitting machine output (JSON) from human output (text) means the JSON contract is stable and machine-validatable, while the human format can iterate on look-and-feel without breaking any consumer. The renderer reads only JSON, so it is identical across all three bindings.
- **New Rust dependencies.** `serde`, `serde_json`, `globset`, `walkdir`, `regex`, `once_cell`, `thiserror` for the core; `clap` for the CLI; `napi` + `napi-derive` for Node; `pyo3` for Python; `criterion` (dev) for benches; `assert_cmd` + `predicates` (dev) for CLI tests.
- **New Node tooling.** `pnpm`, `vitest`, `@napi-rs/cli`, `typescript`.
- **New Python tooling.** `uv`, `maturin`, `pytest`.
- **Schema versioning.** `default_rules.json` and the JSON wire format both carry `"schema_version": 1`. Loaders reject unknown versions so breaking changes are explicit.
- **Telemetry.** None.
- **Consumer mapping is the consumer's job.** Any downstream tool that wants to translate this library's `AppType` and `TechStack` into its own taxonomy keeps that mapping table on its side. `docs/09-consumer-mapping.md` and `app/examples/consumer-mapping/` document the recommended pattern.
- **Two motivating reference implementations exist** in unrelated repos. They informed which initial `AppType` values and `TechStack` facets to ship at v0.1.0 — nothing more. Their migration is out of scope.
- **Out of scope.** Any consumer-specific category mapping. A web UI for visualizing detection results. A daemon mode that watches for project changes. An LLM fallback for low-confidence cases (consumers may build this on top). Network-driven detection from a remote URL (consumers fetch and pass to `detect_files`). All are good v0.2 ideas to consider after v0.1 adoption.
