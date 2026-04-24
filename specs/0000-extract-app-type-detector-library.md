# Feature: `app-type-detector` — a generic, reusable library for classifying any codebase

## Feature Description

`app-type-detector` is a self-contained, opinion-free library that takes a codebase as input and returns a structured description of what it is. The contract is intentionally narrow:

- **Input:** a path to a directory on disk, or an in-memory snapshot of a directory's files (paths + textual contents + a top-level entry listing).
- **Output:** a `DetectionReport` containing a primary and ranked-secondary `AppType` (e.g. `ios_app`, `web_app`, `cli_tool`, `mcp_server`, `obsidian_plugin`, `desktop_game`), a `TechStack` description (languages, build systems, frameworks, package managers detected), an `Evidence` list (the specific files and patterns that supported each conclusion), and a confidence score per claim.

It does not read configuration from any consumer's repo, does not borrow vocabulary from any consumer's database schema, and does not encode any consumer-specific rules. Its `AppType` and `TechStack` enums are defined for their own sake — for accurately describing what a codebase *is*, not for matching what any one downstream tool *wants*.

The library ships through three channels so any tool, in any language, can consume it without re-implementing detection:

- **crates.io** — the canonical Rust crate.
- **npm** (`@snam/app-type-detector`) — `napi-rs` native addons with prebuilt binaries for the major triples and a WASM fallback for restricted runtimes.
- **PyPI** (`app-type-detector`) — `pyo3` + `maturin` ABI3 wheels so Python 3.10+ shares one binary per triple.

A small `app-type-detector` CLI binary (also published) lets shell scripts and any other process consume the same engine via `app-type-detector detect [PATH] --format json|tsv|text`.

The library performs zero network I/O, never spawns child processes, and treats unknown / polyglot / empty codebases as legitimate inputs that produce a low-confidence answer rather than an error.

## User Story

As a tool builder writing scripts, services, or apps that need to know "what is this codebase?",
I want a single dependency that takes a path or a file map and hands back a typed answer with evidence,
So that I never write yet another shell pipeline of `find`, `jq`, and globs (or yet another LLM prompt) to re-derive the same answer, and so that my tool inherits new detections (visionOS apps, Tauri apps, Solana programs, future MCP servers) the moment the library learns them, without me changing a line of my own code.

## Problem Statement

Across the developer-tools ecosystem, "what kind of project is this?" is solved over and over in incompatible ways:

1. **Duplicated, drifting rule sets.** Most tools that need this answer encode it inline: a bash script with `find` + `jq`, an LLM prompt that asks "based on these files, what is this?", a hand-coded if/else over `package.json` keys. Each implementation has its own gaps and its own bugs, and a fix in one cannot help the others.
2. **The wrong tool for the job.** Bash + glob loops are slow and brittle. LLMs are non-deterministic, cost tokens, and add latency to a question that a 50-line rule answers for free with full explainability. A single small native binary is the right shape for this work.
3. **No shared vocabulary.** Every tool invents its own labels for the same concept ("ios_app" vs "iOS" vs "swift-app" vs "apple-mobile"). Without a shared, well-defined enum, two tools cannot trivially exchange "what this codebase is".
4. **No shared evidence.** Even tools that arrive at the same label cannot tell each other *why*. A library that returns evidence (the files + patterns + matches that supported each conclusion) lets downstream tools build trust, surface justifications, and decide when to fall back to a human or an LLM.
5. **Adding a new project type is N edits across N codebases.** Every time a new ecosystem appears (visionOS, Tauri, Solana, the next MCP variant), every interested tool has to update its own detection independently. There is no place to land "the new rule" once.

## Solution Statement

Build `app-type-detector` as a generic detection engine that owns the problem end to end:

- **One canonical vocabulary.** The library defines its own `AppType` and `TechStack` enums based on what is true about codebases, not based on any consumer's database. Categories are added when a real ecosystem appears, removed when one dies, and versioned via semver.
- **One canonical rule format.** Detection rules live in a single JSON document compiled into the binary as the default ruleset. The format is documented, versioned, and overridable: callers may pass their own ruleset or extend the defaults if they have a private domain (internal monorepo conventions, custom wrapper templates).
- **Two input shapes, one engine.**
  - `detect_path(path)` walks a real directory (gated behind an `fs` feature flag).
  - `detect_files(snapshot)` accepts an in-memory map of paths to contents plus the top-level directory listing. This is the shape any caller already has when files were fetched remotely (GitHub raw, a tarball, a sandbox API).
  Both produce the same `DetectionReport`.
- **Pure functions, explicit evidence.** Every claim in the output references the rule(s) and file(s) that fired. No hidden state. No telemetry. No network. No subprocesses.
- **Three publishing channels from one source.** The Rust crate is the source. `napi-rs` and `pyo3` + `maturin` produce the npm and PyPI distributions from the same engine. Adding a fourth ecosystem (Ruby gem, Go module via cgo, .NET via UniFFI) is mechanical; it does not require rewriting detection logic.
- **Open vocabulary mapping for consumers.** The library does not know about any specific downstream tool. Consumers translate the library's `AppType` into their own taxonomy with a thin lookup table on their side. This is the explicit contract: detection upstream, classification downstream.

The library answers "what is this codebase?" the same way for everyone. How any individual consumer reacts to that answer is none of the library's business.

## Relevant Files

The current `app-type-detector` working directory is a fresh scaffold (only `README.md`, `.gitignore`, and empty `app/`, `scripts/`, `specs/`, `ai_docs/`, `adws/`, `business/`, `docs/` subdirectories). Every file below this section is new.

There are two **motivating reference implementations** of this same problem living in unrelated repos. They are *not* requirements, dependencies, or test fixtures for this library. They exist only to inform the default ruleset's coverage and to confirm, by inspection, that the library's vocabulary is rich enough that those tools could adopt it later if they choose:

- `~/Developer/__versioning_projects/.versioning-scan.config.json` — a JSON ruleset for build-system detection, useful as a sanity check that the default ruleset covers iOS (XcodeGen + raw Xcode + SPM library), Android (Gradle), Node, Python (`pyproject.toml`), Rust (Cargo), Go, Obsidian plugins, Godot, Unity, and WordPress plugins.
- `~/Developer/auto-build-log/app/buildlog-worker/src/agents/enricher/{kind-inference.ts, tools/github-read.ts}` and `app/agents/detective/strategies/types.ts` — useful as a sanity check that the default ruleset's `AppType` covers iOS apps and games, macOS apps, menu-bar apps, web apps, APIs, dev tools, CLI tools, libraries, MCP servers, browser plugins, Claude skills, desktop apps, and Steam games.

Neither file is read by the library, neither file's vocabulary is adopted verbatim, and neither tool's migration is scoped here. The library defines its own labels; if those tools later want to adopt this library, they will write their own one-way mapping table on their side.

### New Files

#### Rust workspace

- `app/Cargo.toml` — workspace root listing all crates and bindings.
- `app/rust-toolchain.toml` — pin a stable Rust toolchain (1.82+).
- `app/.cargo/config.toml` — workspace-wide lints.

##### `app/crates/app-type-detector/` — the core crate

- `app/crates/app-type-detector/Cargo.toml` — crate manifest. Default features: `fs`, `serde`, `default-rules`. The `default-rules` feature embeds the bundled ruleset; turning it off yields a slimmer binary that only knows what the caller passes in.
- `app/crates/app-type-detector/src/lib.rs` — public API and re-exports.
- `app/crates/app-type-detector/src/types.rs` — `DetectionReport`, `AppType`, `TechStack`, `Language`, `BuildSystem`, `Framework`, `PackageManager`, `Evidence`, `Confidence`. All enums are non-exhaustive (`#[non_exhaustive]`) so the library can add variants in a minor release without breaking downstream `match` arms.
- `app/crates/app-type-detector/src/rules.rs` — the rule grammar. A rule is a declarative `MatchExpr` (any/all/none combinators over file-presence, glob-presence, and content-regex predicates) plus a payload (one or more `AppType` and `TechStack` annotations and a confidence weight).
- `app/crates/app-type-detector/src/engine.rs` — pure rule evaluation against an `InputSnapshot`.
- `app/crates/app-type-detector/src/snapshot.rs` — `InputSnapshot` trait + `MemorySnapshot` (HashMap-backed) + `FilesystemSnapshot` (gated behind `fs`; uses `walkdir` + `globset`, applies a curated ignore list of `.git`, `node_modules`, `dist`, `.next`, `target`, `build`, `.venv`, `.gradle`, `Pods`).
- `app/crates/app-type-detector/src/synthesis.rs` — combines per-rule fires into a single `DetectionReport`. Resolves polyglot codebases (multiple languages, multiple frameworks) by ranking on confidence and assigning a primary `AppType` only when one rule clearly dominates; otherwise leaves `primary_app_type = None` and surfaces every candidate.
- `app/crates/app-type-detector/src/default_rules.rs` — `pub fn default_ruleset() -> &'static Ruleset`, cached in a `OnceLock`.
- `app/crates/app-type-detector/src/default_rules.json` — human-edited source of truth for the bundled rules. Adding a new ecosystem is a JSON edit + test fixture.
- `app/crates/app-type-detector/build.rs` — validates `default_rules.json` parses and is internally consistent at build time.
- `app/crates/app-type-detector/tests/fixtures/` — one minimal directory tree per supported `AppType`, used as the canonical regression suite.
- `app/crates/app-type-detector/tests/detect_path.rs` — exercises every fixture through `detect_path`.
- `app/crates/app-type-detector/tests/detect_memory.rs` — exercises every fixture through `detect_files` from a `MemorySnapshot` and asserts byte-identical output to the FS path.
- `app/crates/app-type-detector/tests/polyglot.rs` — projects with multiple languages / multiple build systems / multiple frameworks, asserting the synthesizer's behavior is deterministic and explainable.
- `app/crates/app-type-detector/tests/edge_cases.rs` — empty dirs, dirs with only `.git/`, dirs containing only ignored paths, broken UTF-8, oversized files, symlink loops.
- `app/crates/app-type-detector/benches/detect.rs` — Criterion benchmark, target <5 ms per project.

##### `app/crates/app-type-detector-cli/` — the CLI

- `app/crates/app-type-detector-cli/Cargo.toml`.
- `app/crates/app-type-detector-cli/src/main.rs` — `clap`-based CLI: `app-type-detector detect [PATH] [--rules FILE] [--format json|tsv|text] [--include-evidence]`. Default output is human-readable text; `--format json` is the documented machine surface. The TSV format is a small, fixed shape (`primary_app_type\tprimary_language\tprimary_build_system\tconfidence`) for shell consumers.
- `app/crates/app-type-detector-cli/tests/cli.rs` — `assert_cmd` smoke tests.

##### `app/bindings/node/` — npm package

- `app/bindings/node/Cargo.toml` — `napi-rs` crate.
- `app/bindings/node/src/lib.rs` — N-API exports: `detectPath(path)`, `detectFiles({files, rootDirs})`, `defaultRuleset()`. `napi-derive` generates the TypeScript types automatically.
- `app/bindings/node/package.json` — `name: "@snam/app-type-detector"`, optional-deps for each prebuilt binary subpackage, `napi.triples` config.
- `app/bindings/node/index.js` — `napi-rs` dispatcher.
- `app/bindings/node/index.d.ts` — generated, committed.
- `app/bindings/node/__test__/index.test.ts` — `vitest` round-trip test against the same fixtures.
- `app/bindings/node/README.md` — npm-facing docs.

##### `app/bindings/python/` — PyPI package

- `app/bindings/python/Cargo.toml` — `pyo3` crate.
- `app/bindings/python/src/lib.rs` — `pyo3` module: `detect_path`, `detect_files`, `default_ruleset`. Returns dataclass-shaped Python objects.
- `app/bindings/python/pyproject.toml` — `maturin` build backend.
- `app/bindings/python/python/app_type_detector/__init__.py` — re-exports + type stubs.
- `app/bindings/python/python/app_type_detector/py.typed` — PEP 561 marker.
- `app/bindings/python/tests/test_detect.py` — `pytest` parity test.

#### Examples

- `app/examples/rust-fs-walk/` — `cargo run -- /path/to/repo`.
- `app/examples/node-fixture/` — `node index.js` showing `detectPath` and `detectFiles`.
- `app/examples/python-fixture/` — `python detect.py` mirror.
- `app/examples/consumer-mapping/` — illustrates the recommended pattern for a downstream tool that wants to map this library's `AppType` to its own internal taxonomy. A 30-line lookup table on the consumer side. **No tool-specific code lives in the library.**

#### Scripts

- `scripts/dev.sh` — install toolchains.
- `scripts/start.sh` — `cargo run -p app-type-detector-cli -- detect .`.
- `scripts/test-all.sh` — runs Rust nextest + Node vitest + Python pytest.
- `scripts/lint-all.sh` — `cargo clippy --all-targets --workspace -- -D warnings && cargo fmt --all -- --check`.
- `scripts/build-rules.sh` — validate `default_rules.json` and regenerate `docs/RULES.md`.
- `scripts/release-crate.sh`, `scripts/release-npm.sh`, `scripts/release-pypi.sh` — release helpers (also called by CI).

#### CI

- `.github/workflows/ci.yml` — fmt + clippy + nextest + node test + python test on Linux + macOS.
- `.github/workflows/release-crate.yml` — `crate-vX.Y.Z` tag → publish to crates.io.
- `.github/workflows/release-npm.yml` — `npm-vX.Y.Z` tag → matrix build per triple → publish to npm.
- `.github/workflows/release-pypi.yml` — `pypi-vX.Y.Z` tag → matrix wheel build via `PyO3/maturin-action` → publish to PyPI.

#### Docs

- `docs/00-overview.md` — what the library is and is not.
- `docs/01-vocabulary.md` — every `AppType`, `TechStack`, `Language`, `BuildSystem`, `Framework`, `PackageManager` value, with one-paragraph definition and at least one canonical example.
- `docs/02-rules.md` — the rule grammar.
- `docs/03-rust-usage.md`.
- `docs/04-node-usage.md`.
- `docs/05-python-usage.md`.
- `docs/06-cli-usage.md`.
- `docs/07-extending.md` — how to author a new rule, contribute it upstream, or pass a custom ruleset.
- `docs/08-consumer-mapping.md` — the recommended pattern for downstream tools to translate `AppType` into their own taxonomy. Ships an example mapping table to make the contract concrete.
- `docs/RULES.md` — generated reference of every bundled rule.
- `ai_docs/napi-rs-overview.md` — distilled `napi-rs` reference.
- `ai_docs/pyo3-maturin-overview.md` — distilled `pyo3` + `maturin` reference.

#### Project metadata

- `LICENSE` (MIT).
- `CHANGELOG.md` — keep-a-changelog format with three release tracks (Rust crate, npm, PyPI).

## Implementation Plan

### Phase 1: Foundation

Stand up the Rust workspace, pin the public type surface, and define the rule grammar from first principles. Define `AppType`, `TechStack`, and the supporting enums based on what accurately describes a codebase, not based on any consumer's needs. Author 25+ minimal fixtures (one per `AppType` we ship at v0.1.0) and prove they round-trip through `MemorySnapshot` end-to-end before any binding code exists. Land the default ruleset as a JSON file plus a `build.rs` that validates it. Wire CI to run fmt + clippy + nextest on every push. Phase ends when `cargo nextest run` passes against the fixture suite and `default_ruleset()` is the canonical source.

### Phase 2: Core Implementation

Implement the rule engine, the FS-backed snapshot (with the curated ignore list and depth cap), and the synthesizer that combines per-rule fires into a single `DetectionReport` with explicit `Evidence`. Add the polyglot test suite (multi-language repos, multi-build-system repos, projects with both web and mobile components) to pin synthesizer behavior. Add the edge-cases suite (empty dirs, broken UTF-8, oversized files, symlink loops). Ship the CLI binary with `json`, `tsv`, and `text` output formats. Land the Criterion benchmark and confirm <5 ms on a typical project.

### Phase 3: Bindings and Distribution

Build the two language bindings in parallel:

- **Node** via `napi-rs`: per-triple matrix in CI, prebuilt binaries published as optional-dep subpackages, WASM fallback. TypeScript types generated by `napi-derive`. Vitest smoke test against the same fixtures.
- **Python** via `pyo3` + `maturin`: ABI3 wheels for the same triples plus Python 3.10–3.13. `py.typed` ships. `pytest` parity test against the same fixtures.

Tag and publish `crate-v0.1.0`, `npm-v0.1.0`, `pypi-v0.1.0`. Confirm `cargo add app-type-detector`, `npm i @snam/app-type-detector`, and `pip install app-type-detector` all install and run a hello-world detection successfully on a fresh machine.

## Step by Step Tasks

IMPORTANT: Execute every step in order, top to bottom.

### 1. Project metadata

- Add `LICENSE` (MIT).
- Update `README.md` with the real elevator pitch, the install snippets for all three ecosystems, and a 10-line usage example per ecosystem.
- Create empty `CHANGELOG.md` with three sections (Rust crate, npm, PyPI).

### 2. Scaffold the Rust workspace

- Write `app/Cargo.toml` declaring the workspace and shared `[workspace.dependencies]` for `serde`, `serde_json`, `globset`, `walkdir`, `regex`, `once_cell`, `thiserror`, `clap`, `napi`, `napi-derive`, `pyo3`.
- Write `app/rust-toolchain.toml` pinning stable + `rustfmt`, `clippy`.
- Write `app/.cargo/config.toml` with workspace-wide lints (`unsafe_code = "forbid"`, `unused`, opt-in `clippy::pedantic` lints).
- `cargo new --lib app/crates/app-type-detector` and prune the auto-generated `lib.rs`.
- Verify `cd app && cargo check --workspace` passes.

### 3. Define the public type surface

- In `src/types.rs`, declare `AppType`, `Language`, `BuildSystem`, `Framework`, `PackageManager`, `TechStack`, `Confidence`, `Evidence`, `DetectionClaim`, `DetectionReport`. All enums `#[non_exhaustive]` and `serde::{Serialize, Deserialize}` behind the `serde` feature.
- Initial `AppType` set (v0.1.0): `web_app`, `static_site`, `web_api`, `cli_tool`, `library`, `daemon`, `desktop_app`, `desktop_game`, `menu_bar_app`, `ios_app`, `ios_game`, `mac_app`, `visionos_app`, `tvos_app`, `android_app`, `android_game`, `mcp_server`, `claude_skill`, `browser_extension`, `vscode_extension`, `obsidian_plugin`, `wordpress_plugin`, `godot_game`, `unity_game`, `unknown`.
- Initial `Language` set: `rust`, `typescript`, `javascript`, `python`, `swift`, `objc`, `kotlin`, `java`, `go`, `ruby`, `php`, `csharp`, `gdscript`, `c`, `cpp`, `shell`.
- Initial `BuildSystem` set: `cargo`, `npm`, `pnpm`, `yarn`, `uv`, `pip`, `poetry`, `gradle`, `maven`, `xcodebuild`, `xcodegen`, `swift_pm`, `go_modules`, `bundler`, `composer`, `dotnet`, `make`, `just`, `bazel`, `godot`, `unity`.
- Initial `Framework` set: `nextjs`, `astro`, `remix`, `nuxt`, `sveltekit`, `vite`, `react`, `vue`, `svelte`, `solid`, `swiftui`, `uikit`, `appkit`, `jetpack_compose`, `flutter`, `tauri`, `electron`, `fastapi`, `django`, `flask`, `rails`, `laravel`, `express`, `hono`, `actix`, `axum`, `rocket`.
- Initial `PackageManager` set is the union of `npm`, `pnpm`, `yarn`, `pip`, `uv`, `poetry`, `cargo`, `swift_pm`, `cocoapods`, `gradle`, `bundler`, `composer`, `nuget`, `go_modules`.
- Add `lib.rs` re-exports and run `cargo doc --no-deps`. Every public item must have a one-line doc that explains *why* it exists.

### 4. Define the rule grammar

- In `src/rules.rs` define:
  - `MatchExpr::FileExists(String)`, `MatchExpr::Glob(String)`, `MatchExpr::Content { file: String, regex: String }`, `MatchExpr::All(Vec<MatchExpr>)`, `MatchExpr::Any(Vec<MatchExpr>)`, `MatchExpr::Not(Box<MatchExpr>)`.
  - `Rule { id: String, when: MatchExpr, app_types: Vec<AppType>, languages: Vec<Language>, build_systems: Vec<BuildSystem>, frameworks: Vec<Framework>, package_managers: Vec<PackageManager>, weight: f32, evidence_label: String }`.
  - `Ruleset { schema_version: u32, rules: Vec<Rule> }` with `Ruleset::from_json(&str) -> Result<Self>`.
- Document `MatchExpr` semantics with worked examples in doc comments. Stay declarative — no implicit ordering, no first-match-wins. Conflicts are resolved by the synthesizer using rule weights.
- Unit-test each `MatchExpr` variant in isolation.

### 5. Author the default ruleset

- Write `src/default_rules.json` with one or more rules per supported `AppType`. The ruleset is authored from first principles, not copied from any external config. Examples:
  - `ios-xcodegen` → `app_types: [ios_app]`, `build_systems: [xcodegen, xcodebuild]`, when `project.yml` exists AND any `*.xcodeproj` directory at the root.
  - `mcp-server-node` → `app_types: [mcp_server]`, `languages: [typescript|javascript]`, when `package.json` contains `@modelcontextprotocol/sdk`.
  - `claude-skill` → `app_types: [claude_skill]`, when `SKILL.md` exists at root.
  - `tauri-app` → `app_types: [desktop_app]`, `frameworks: [tauri]`, when `src-tauri/tauri.conf.json` exists.
  - `obsidian-plugin` → when `manifest.json` AND `versions.json` exist at root.
  - `vscode-extension` → when `package.json` contains an `engines.vscode` key.
  - `godot-game` → when `project.godot` exists.
  - `unity-game` → when `ProjectSettings/ProjectSettings.asset` exists.
  - …and so on for every initial `AppType`.
- Add `build.rs` that `include_str!`s the JSON, parses it, and asserts every `AppType` in the initial set is referenced by at least one rule. Fail the build otherwise.
- Add `default_rules.rs` exposing `default_ruleset() -> &'static Ruleset` cached in a `OnceLock`.

### 6. Implement the snapshot abstraction

- In `src/snapshot.rs` define `trait InputSnapshot { fn glob(&self, pattern: &str) -> Vec<String>; fn read(&self, path: &str) -> Option<Cow<str>>; fn list_root(&self) -> Vec<DirEntry>; }`.
- Implement `MemorySnapshot { files: HashMap<String, Option<String>>, root: Vec<DirEntry> }`.
- Implement `FilesystemSnapshot` (behind `fs` feature) using `walkdir` with the curated ignore list and a default depth cap of 4. `.follow_links(false)` to avoid symlink loops.
- Unit-test both implementations against an identical synthetic tree and assert their `glob` outputs match.

### 7. Implement the rule engine

- In `src/engine.rs`, write `evaluate(snapshot: &dyn InputSnapshot, rules: &Ruleset) -> Vec<RuleHit>`.
- A `RuleHit` carries the rule id, the `Evidence` that supported it, and the rule's payload.
- Pure function. No I/O beyond what the snapshot offers. No allocation in the hot loop where avoidable.

### 8. Implement the synthesizer

- In `src/synthesis.rs`, write `synthesize(hits: Vec<RuleHit>) -> DetectionReport`.
- Aggregate hits by `AppType`, `Language`, `BuildSystem`, `Framework`, `PackageManager`. Sum weights. Sort.
- Set `primary_app_type` only when the top-weighted candidate exceeds the runner-up by a configurable margin (default 1.5×). Otherwise leave `primary_app_type = None` and surface every candidate so the consumer can decide.
- Always populate `evidence`, even for low-confidence reports.

### 9. Build the polyglot test suite

- `tests/polyglot.rs`: fixtures with (a) a Rust workspace + Python tooling + Node tooling, (b) a Next.js web app + a sibling Swift iOS client in the same repo, (c) a monorepo with `package.json` AND `Cargo.toml` AND `pyproject.toml`. Assert the synthesizer behavior is deterministic and explainable.

### 10. Build the edge-cases suite

- `tests/edge_cases.rs`: empty dir → `unknown` with low confidence; dir with only `.git/` → same; oversized file → truncate before regex; broken UTF-8 → silently skip content rules but allow file-existence rules; symlink loop → no crash; missing path → `Err`, not panic.

### 11. Build the CLI

- `cargo new --bin app/crates/app-type-detector-cli`.
- Implement `clap`-based parser. Default `PATH` is `.`, default `--format` is `text`.
- `text` format prints a one-screen summary with primary classifications and top 3 evidence items.
- `tsv` format prints `primary_app_type\tprimary_language\tprimary_build_system\tconfidence` for shell consumers.
- `json` format prints the full `DetectionReport`.
- Add `tests/cli.rs` using `assert_cmd` over the fixture tree.

### 12. Build the Node binding

- Scaffold `app/bindings/node` with `napi-rs`.
- Export `detectPath(path)`, `detectFiles({files, rootDirs})`, `defaultRuleset()`. `#[napi(object)]` derives produce TS types.
- Configure `package.json` with `optionalDependencies` for prebuilt-binary subpackages.
- Vitest smoke test that round-trips a fixture through both surfaces and asserts deep equality with a committed JSON snapshot.

### 13. Build the Python binding

- Scaffold `app/bindings/python` with `maturin new --bindings pyo3`.
- Expose `detect_path(path: PathBuf)`, `detect_files(snapshot: dict)`, `default_ruleset()`. Return Python dataclass-shaped objects.
- Configure ABI3 in `pyproject.toml`.
- Add `python/app_type_detector/__init__.py` with type-stub re-exports and `py.typed`.
- `pytest` parity test over the same fixtures.

### 14. Wire CI

- `.github/workflows/ci.yml`: matrix `(ubuntu-latest, macos-14)` × `(stable rust, node 20, python 3.12)`. Steps: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `pnpm --dir app/bindings/node test`, `uv --directory app/bindings/python sync && uv run pytest`.
- `.github/workflows/release-crate.yml` — trigger on `crate-v*` tag.
- `.github/workflows/release-npm.yml` — trigger on `npm-v*` tag, matrix-build per triple, publish to npm.
- `.github/workflows/release-pypi.yml` — trigger on `pypi-v*` tag, matrix-build wheels via `PyO3/maturin-action`, publish to PyPI.

### 15. Author docs

- Write the eight numbered docs in `docs/`.
- Wire `scripts/build-rules.sh` to regenerate `docs/RULES.md` from `default_rules.json` so reference docs never drift.
- Write the example consumer mapping in `app/examples/consumer-mapping/` that demonstrates how a downstream tool would translate this library's `AppType` into its own taxonomy. Make it clear the lookup table lives on the consumer side, never in the library.

### 16. Cut v0.1.0 releases

- Bump all three crates/packages to `0.1.0`.
- Tag `crate-v0.1.0`, `npm-v0.1.0`, `pypi-v0.1.0`.
- Confirm each release workflow succeeded and the artifacts install cleanly on a fresh machine.

### 17. Run full validation

- Execute every command in **Validation Commands** below.
- All commands must exit zero with no test failures and no clippy warnings.

## Testing Strategy

### Unit Tests

- **Rule grammar** (`rules.rs`): each `MatchExpr` variant in isolation against synthetic snapshots, including `Not`, nested `All`/`Any`, content regex compilation errors.
- **Default ruleset loading**: parse `default_rules.json`; assert every rule's regex compiles, every `AppType` in the initial set is referenced by at least one rule, every rule id is unique.
- **Snapshot implementations**: `MemorySnapshot::glob` and `FilesystemSnapshot::glob` produce identical results on the same synthetic tree.
- **Engine**: `evaluate` produces deterministic output for a given snapshot + ruleset.
- **Synthesizer**: ties, near-ties, single-rule wins, multi-rule wins, and conflicting rules each tested with hand-built hit lists.

### Integration Tests

- **Filesystem fixtures** (`tests/detect_path.rs`): one fixture per supported `AppType`, asserting the full `DetectionReport`.
- **Memory parity** (`tests/detect_memory.rs`): same fixtures fed via `MemorySnapshot`, asserting byte-identical reports.
- **Polyglot** (`tests/polyglot.rs`): multi-language and multi-framework projects produce explainable, deterministic output.
- **Edge cases** (`tests/edge_cases.rs`): empty dirs, broken UTF-8, oversized files, symlink loops, missing paths.
- **CLI** (`tests/cli.rs`): each output format produces the documented shape for every fixture.
- **Node binding** (`__test__/index.test.ts`): both `detectPath` and `detectFiles` produce reports deeply equal to a committed JSON snapshot.
- **Python binding** (`tests/test_detect.py`): same as Node, in `pytest`.

### Edge Cases

- Empty directory → `primary_app_type = unknown`, low confidence, evidence is empty list.
- Directory with only a `.git/` folder → same as empty.
- Directory containing only ignored paths (`node_modules`, `target`) → same as empty.
- Polyglot codebase (Rust + Python + Node together) → all three appear in `tech_stack.languages`; `primary_app_type` left `None` if no rule clearly dominates.
- Project with `project.yml` AND `*.xcodeproj` AND a sibling `Package.swift` → multiple iOS-related rules fire; synthesizer picks the highest-weight one and surfaces the rest as secondaries.
- File larger than 64 KB → truncated before content regex; documented cap.
- Memory snapshot with `None` for a known path → treated as "file does not exist".
- Non-UTF-8 file contents → content rules silently skip; existence rules still fire.
- Rule JSON with an unknown matcher key → typed error pointing at the offending rule index.
- Caller passes a nonexistent path → `Err`, not panic.
- Symlink loop in the filesystem → `walkdir` `.follow_links(false)` prevents recursion.

## Acceptance Criteria

- [ ] `cargo nextest run --workspace` passes with zero failures and >=80% coverage on `engine.rs`, `rules.rs`, `synthesis.rs`.
- [ ] Every `AppType` value in the initial set is covered by at least one rule and at least one fixture.
- [ ] `app-type-detector detect --format json .` on each fixture produces a `DetectionReport` whose `primary_app_type` matches the fixture's directory name.
- [ ] `npm i @snam/app-type-detector` then `import { detectPath } from '@snam/app-type-detector'` works on Linux x64, Linux arm64, macOS arm64, macOS x64, Windows x64. The WASM fallback covers everything else.
- [ ] `pip install app-type-detector` then `from app_type_detector import detect_path` works on the same triples for Python 3.10–3.13.
- [ ] `cargo doc --no-deps` renders cleanly with no missing-docs warnings on public items.
- [ ] The library has zero IO escape hatches: no `std::process`, no `reqwest`, no `hyper`, no `tokio::net` in the core crate. Asserted by a CI grep.
- [ ] `unsafe_code` is forbidden in the core crate.
- [ ] Adding a new `AppType` is a JSON edit + test fixture + a non-exhaustive enum variant addition. No engine code changes for a pure presence/glob/regex rule. Documented in `docs/07-extending.md`.
- [ ] Detection of a typical project completes in under 5 ms on macOS arm64 (Criterion bench).
- [ ] The library's vocabulary (`AppType`, `TechStack`, etc.) does not reference any specific consumer or downstream tool, anywhere in code or docs.

## Validation Commands

Execute every command to validate the feature works correctly with zero regressions.

- `cd /Users/seannam/Developer/app-type-detector/app && cargo fmt --all -- --check` — format check
- `cd /Users/seannam/Developer/app-type-detector/app && cargo clippy --workspace --all-targets -- -D warnings` — lint with zero warnings
- `cd /Users/seannam/Developer/app-type-detector/app && cargo nextest run --workspace` — every Rust test (unit + integration + polyglot + edge cases)
- `cd /Users/seannam/Developer/app-type-detector/app && cargo test --doc --workspace` — doctests
- `cd /Users/seannam/Developer/app-type-detector/app && cargo bench -p app-type-detector --bench detect -- --quick` — perf bench within budget
- `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli -- detect crates/app-type-detector/tests/fixtures/ios-xcodegen --format json` — CLI smoke against an iOS fixture
- `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli -- detect crates/app-type-detector/tests/fixtures/mcp-server-node --format json` — CLI smoke for an MCP server fixture
- `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli -- detect crates/app-type-detector/tests/fixtures/godot-game --format json` — CLI smoke for a Godot fixture
- `cd /Users/seannam/Developer/app-type-detector/app/bindings/node && pnpm install && pnpm run build && pnpm test` — Node binding builds and passes its parity test
- `cd /Users/seannam/Developer/app-type-detector/app/bindings/python && uv sync && uv run maturin develop && uv run pytest` — Python binding builds and passes its parity test
- `cd /Users/seannam/Developer/app-type-detector && bash scripts/test-all.sh` — single-shot all-language test suite
- `cd /Users/seannam/Developer/app-type-detector && grep -RIn -E "std::process|reqwest|hyper|tokio::net" app/crates/app-type-detector/src && echo "FAIL: forbidden dep referenced" && exit 1 || echo "OK: core crate has no IO escape hatches"` — invariant check
- `cd /Users/seannam/Developer/app-type-detector && grep -RIn -iE "buildlog|version[-_ ]skill|versioning_projects" app/ docs/ && echo "FAIL: consumer-specific naming leaked into library" && exit 1 || echo "OK: library is consumer-neutral"` — vocabulary check

## Notes

- **Why Rust core, not Go or pure TS?** Rust gives one binary that ships to crates.io, npm (`napi-rs` native addons), and PyPI (`pyo3` + `maturin` wheels) without a runtime. TS would force Node on Python consumers. Go would force CGO across both bindings. Rust is the pragmatic least-common-denominator for "fast, embeddable, three-ecosystem release path".
- **Why `napi-rs`, not WASM, for the npm package?** `napi-rs` produces native addons that hit native FS speeds and avoid a sync-FS shim. We keep a WASM build as a fallback for Edge / Cloudflare Workers and any environment where loading `.node` files is forbidden. `package.json` `optionalDependencies` transparently chooses the right artifact.
- **Why `pyo3` + `maturin`, not pure Python?** Same reason: keep one source of truth in Rust. ABI3 wheels mean Python 3.10–3.13 share one binary per triple, keeping the wheel matrix small.
- **Naming on npm.** Scoped (`@snam/app-type-detector`) so it can publish without name clashes.
- **New Rust dependencies.** `serde`, `serde_json`, `globset`, `walkdir`, `regex`, `once_cell`, `thiserror` for the core; `clap` for the CLI; `napi` + `napi-derive` for Node; `pyo3` for Python; `criterion` (dev) for benches; `assert_cmd` + `predicates` (dev) for CLI tests; `cargo-nextest` (dev tool, not a crate dep).
- **New Node tooling.** `pnpm`, `vitest`, `@napi-rs/cli`, `typescript`. Lives under `app/bindings/node/`.
- **New Python tooling.** `uv` (already present on Sean's machine), `maturin`, `pytest`. Lives under `app/bindings/python/`.
- **Schema versioning.** `default_rules.json` carries `"schema_version": 1`. The Rust loader rejects unknown versions so future breaking changes to the rule grammar are explicit and semver-safe.
- **Telemetry.** None. The library never phones home.
- **Consumer mapping is the consumer's job.** Any downstream tool that wants to translate this library's `AppType` into its own internal taxonomy keeps that mapping table on its side, not in the library. `docs/08-consumer-mapping.md` and `app/examples/consumer-mapping/` document the recommended pattern. The library never references any specific consumer in code or docs (enforced by a CI grep).
- **Two motivating reference implementations exist** in unrelated repos (`~/Developer/__versioning_projects` and `~/Developer/auto-build-log`). They informed which initial `AppType` values to ship at v0.1.0 — nothing more. Their migration to this library is out of scope.
- **Out of scope.** Any consumer-specific category mapping. A web UI for visualizing detection results. A daemon mode that watches for project changes. An LLM fallback for low-confidence cases (consumers may build this on top). Network-driven detection from a remote URL (consumers fetch and pass to `detect_files`). All are good v0.2 ideas to consider after the first set of consumers adopts the v0.1 contract.
