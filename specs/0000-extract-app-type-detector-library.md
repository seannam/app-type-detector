# Feature: Extract a Reusable App-Type Detection Library (Rust core, npm + PyPI bindings)

## Feature Description

Build `app-type-detector`, a single-source-of-truth library that classifies any local directory or git repository by the kind of app it is. The library answers two layered questions:

1. **What build system / package manager / project skeleton does this directory use?** (e.g. `ios-xcodegen`, `node`, `rust-cargo`, `python-pyproject`, `android-gradle`, `godot`, `unity`, `obsidian-plugin`, `wordpress-plugin`, `xcode-only`, `spm-library`, `universal`).
2. **What kind of app is this most likely to be?** (e.g. `ios_app`, `ios_game`, `mac_app`, `web_app`, `saas`, `api`, `dev_tool`, `cli_tool`, `library`, `mcp_server`, `claude_skill`, `browser_plugin`, `desktop_app`, `menu_bar_app`, `steam_game`, `utility`, etc.).

The core detector is written in Rust for speed and one-place correctness. It ships through three distribution channels so every codebase Sean owns can call the same logic:

- **crates.io** for Rust consumers.
- **npm** (TypeScript types included) via `napi-rs` native addons for the Node ecosystem (used by `auto-build-log`, the BuildLog enricher, scout, and any future TS tooling).
- **PyPI** via `pyo3` + `maturin` for Python consumers (and any agentic tooling written in Python).

It runs in two modes: **filesystem mode** (walk a real path) for shell tools like the version skill scanner, and **virtual mode** (accept a `{path: contents}` map) for cases where the caller has already fetched files remotely (BuildLog reads files from GitHub raw content endpoints).

Today the same logic is partially reimplemented in two unrelated places:

- `~/Developer/__versioning_projects/.versioning-scan.config.json` + `~/.claude/scripts/version/scan.sh` use a bash + `jq` rule engine over `stack_rules` to classify projects for the `/version` skill.
- `~/Developer/auto-build-log/app/buildlog-worker/src/agents/enricher/{kind-inference.ts, tools/github-read.ts}` + `app/agents/detective/strategies/types.ts` define a parallel `BuildCategory` enum, fetch a fixed allowlist of files, and pass them to an LLM that has to (re)derive the category from raw file bodies.

Both implementations have the same shape (read a small set of well-known files, run rules over their presence/contents, return a label) but they cannot share fixes, defaults, or improvements. Extracting them solves that.

## User Story

As a developer (Sean) maintaining an ever-growing fleet of projects across iOS, macOS, web, CLIs, browser extensions, MCP servers, and Claude skills,
I want a single library that any of my tools (bash scripts, Node services, Python utilities, future Rust agents) can call to ask "what kind of project is this?",
So that I stop reimplementing detection logic in three different syntaxes, every consumer benefits from the same fixes, and adding support for a new project type (visionOS app, tvOS app, Tauri desktop app, Solana program, etc.) is a one-line ruleset addition that propagates to every downstream consumer the moment they bump the version.

## Problem Statement

There are at least two production surfaces (the `/version` skill's project scanner and BuildLog's enricher) that need to answer "what is this project?" and they currently:

1. **Duplicate the rules.** The version skill encodes rules in JSON consumed by bash. BuildLog encodes rules implicitly inside an LLM prompt plus a separate Detective strategy registry plus a hand-written Zod enum. These are not in sync. Adding `tauri-app` to one does not add it to the other.
2. **Encode rules in the wrong language for the job.** Bash + `jq` glob matching is slow and fragile (the `compgen -G` based loop in `scan.sh` is opaque and has no tests). LLM-driven inference is non-deterministic, costs tokens per invocation, and blocks the rest of the enrich pipeline waiting for a category that a 50-line rule would have returned for free.
3. **Cannot be reused by other tooling.** A future mobile app that displays my portfolio, a Tauri admin dashboard, a Python ASO audit script, or any Claude skill that needs to know "is the current dir an iOS project?" all have to either shell out to bash or rebuild the rules a third time.
4. **Have no test surface.** Neither the bash `detect_stack` function nor the BuildLog category-inference path has a unit test that pins behavior on a fixture project. Refactors are scary.
5. **Cannot evolve schema together.** When I add `visionos_app` or `mcp_server` to BuildLog's enum, the version skill's `stack_rules` does not learn anything. When the version skill learns to detect Obsidian plugins, BuildLog still has to hallucinate that category from the README.

## Solution Statement

Ship `app-type-detector` as a single Rust crate plus two thin language bindings:

- A Rust crate (`app-type-detector` on crates.io) implementing the rule engine, the bundled default ruleset (a superset of today's `stack_rules` and `BUILD_CATEGORY_VALUES`), and the two input modes (filesystem walk and in-memory file map).
- A Node binding (`@snam/app-type-detector` on npm) using `napi-rs`, shipping prebuilt binaries for every triple BuildLog runs on (`x86_64-linux-gnu`, `aarch64-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`) plus a WASM fallback for environments that cannot load native addons.
- A Python binding (`app-type-detector` on PyPI) using `pyo3` + `maturin`, shipping prebuilt wheels for the same triples.
- A small `app-type-detector` CLI (Rust binary) so bash callers like `scan.sh` can drop the bash `detect_stack` function and exec the binary instead.

The detector is **pure functions over a snapshot of the project**. Its inputs are:

1. A set of files at known paths (allowlist or glob-driven), with their textual contents available on demand.
2. Optional environment hints (e.g. "this came from GitHub, use the default branch listing").

Its outputs are a single typed `DetectionReport` containing:

- `stacks: [StackMatch]` — every stack rule that matched, ordered by rule priority (versioning skill semantics: first-match-wins is the default but the report includes secondaries for tooling that wants them).
- `primary_stack` — the winning stack (what the version skill would write to `.version-preset`).
- `categories: [CategoryMatch]` — candidate `BuildCategory` values inferred from stacks plus content signals.
- `primary_category` — the most likely category (what BuildLog's enricher would record as `category`).
- `signals` — the raw evidence (`package-json:stripe-dep`, `readme:mentions-pricing`, `path:has-xcodeproj`) so downstream tools can show "why" and so the LLM call in BuildLog can be downgraded from "decide the category" to "rubber-stamp the deterministic guess".
- `confidence` per match, so callers can decide when to fall back to a human or an LLM.

The default ruleset is compiled into the binary so there is zero config to ship. Callers may override or extend it by passing a `Ruleset` constructed from a JSON file (the same shape as `versioning-scan.config.json`'s `stack_rules`, plus a new `category_rules` section). This keeps `__versioning_projects` working without behavior change while letting BuildLog inject its richer category mapping.

Migration is opt-in per consumer:

- `scan.sh` swaps its bash `detect_stack` function for a single shell out to the new CLI.
- BuildLog's enricher swaps `kind-inference.ts` and the file allowlist for `import { detect } from '@snam/app-type-detector'`, then changes the LLM prompt from "pick a category" to "validate this category guess and refine the title/tagline/why/how".

The library itself never reaches into the network and never spawns child processes. All I/O is the caller's responsibility (filesystem walk inside Rust is the only exception, gated behind a `fs` feature flag).

## Relevant Files

The current `app-type-detector` working directory is a fresh scaffold (only `README.md`, `.gitignore`, and empty `app/`, `scripts/`, `specs/`, `ai_docs/`, `adws/`, `business/`, `docs/` directories). Every file below this section is a new file.

The existing implementations being extracted live in two **read-only reference** locations outside this repo. We do not edit them in this spec; we only port their behavior:

- `~/Developer/__versioning_projects/.versioning-scan.config.json` — canonical source of today's stack rules. The new default ruleset must reproduce every rule here verbatim before it adds anything new.
- `~/.claude/scripts/version/scan.sh` (lines 88–189, the `detect_stack` function) — the rule evaluation algorithm we are reimplementing in Rust. Note especially the `any_of`, `any_of_glob`, `all_of`, `all_of_glob` (with pipe-separated OR groups), and `none_of` semantics.
- `~/Developer/auto-build-log/app/buildlog-worker/src/agents/enricher/schema.ts` (`BUILD_CATEGORY_VALUES`, `BUILD_KIND_VALUES`) — canonical source of the high-level category enum.
- `~/Developer/auto-build-log/app/buildlog-worker/src/agents/enricher/kind-inference.ts` — content-signal extraction (Stripe deps, pricing keywords, store links). Behavior must port 1:1.
- `~/Developer/auto-build-log/app/buildlog-worker/src/agents/enricher/tools/github-read.ts` — the file allowlist (`README.md`, `CLAUDE.md`, `ARCHITECTURE.md`, `ROADMAP.md`, `package.json`, `Cargo.toml`, `pyproject.toml`, `build.gradle`, `manifest.json`, `.buildlog.yml`) plus the `*.xcodeproj` directory listing requirement. The library's "virtual mode" input shape mirrors this.
- `~/Developer/auto-build-log/app/agents/detective/strategies/types.ts` — `DeploymentPlatform` and `BuildCategory` enums; we adopt these names verbatim to keep BuildLog's downstream code unchanged.

### New Files

#### Rust workspace

- `app/Cargo.toml` — workspace root listing the three crates and bindings.
- `app/rust-toolchain.toml` — pin a stable Rust toolchain (1.82+).
- `app/.cargo/config.toml` — workspace lints, target settings.

##### `app/crates/app-type-detector/` (the core crate)

- `app/crates/app-type-detector/Cargo.toml` — crate manifest. Features: `fs` (default, enables filesystem walking), `serde` (default, JSON ruleset support), `default-rules` (default, embeds the bundled ruleset).
- `app/crates/app-type-detector/src/lib.rs` — public API (`detect_path`, `detect_files`, `default_ruleset`, re-exports).
- `app/crates/app-type-detector/src/types.rs` — `DetectionReport`, `StackMatch`, `CategoryMatch`, `Signal`, `BuildCategory`, `BuildKind`, `DeploymentPlatform` (the exact strings BuildLog already uses).
- `app/crates/app-type-detector/src/rules.rs` — `Ruleset`, `StackRule`, `CategoryRule`, `SignalRule`, `Matcher` (any_of / any_of_glob / all_of / all_of_glob / none_of with pipe-separated OR groups, mirroring `scan.sh` semantics).
- `app/crates/app-type-detector/src/engine.rs` — pure rule evaluation against an `InputSnapshot`.
- `app/crates/app-type-detector/src/snapshot.rs` — `InputSnapshot` trait + `MemorySnapshot` (HashMap-backed) + `FilesystemSnapshot` (gated behind `fs` feature; uses `walkdir` + `globset`, never crosses `.git/`, `node_modules/`, `dist/`, `.next/`, `target/`, `build/`).
- `app/crates/app-type-detector/src/signals.rs` — content-based signal extraction (Stripe regex, pricing keywords, app-store link, etc.) ported from `kind-inference.ts`.
- `app/crates/app-type-detector/src/category.rs` — stacks + signals → category inference, including the priority/confidence model.
- `app/crates/app-type-detector/src/default_rules.rs` — `pub fn default_ruleset() -> &'static Ruleset` returning a `OnceLock<Ruleset>`-cached, embedded default. Rules ported from `versioning-scan.config.json`.
- `app/crates/app-type-detector/src/default_rules.json` — human-edited source of truth for the bundled rules (compiled into the binary by `build.rs`). Adding `tauri-app` is a JSON edit.
- `app/crates/app-type-detector/build.rs` — validates `default_rules.json` parses cleanly and fails the build otherwise.
- `app/crates/app-type-detector/tests/fixtures/` — one minimal directory tree per supported stack (`ios-xcodegen/project.yml`, `rust-cargo/Cargo.toml`, `python-pyproject/pyproject.toml`, `node/package.json`, `obsidian-plugin/manifest.json` + `versions.json`, `godot/project.godot`, `unity/ProjectSettings/ProjectSettings.asset`, `mcp-server/package.json` with `@modelcontextprotocol/sdk`, etc.).
- `app/crates/app-type-detector/tests/detect_path.rs` — integration tests exercising every fixture against `detect_path`.
- `app/crates/app-type-detector/tests/detect_memory.rs` — same fixtures fed in via `MemorySnapshot` to prove parity between filesystem and virtual modes.
- `app/crates/app-type-detector/tests/parity_versioning_skill.rs` — for each rule in `versioning-scan.config.json`, assert the new detector produces the same `(stack, preset, mode, preset_missing)` tuple.
- `app/crates/app-type-detector/tests/parity_kind_inference.rs` — port the BuildLog `kind-inference.test.ts` cases verbatim.
- `app/crates/app-type-detector/benches/detect.rs` — Criterion benchmark, target <5 ms per project on macOS.

##### `app/crates/app-type-detector-cli/` (a thin CLI for shell consumers)

- `app/crates/app-type-detector-cli/Cargo.toml`.
- `app/crates/app-type-detector-cli/src/main.rs` — `clap`-based CLI: `app-type-detector detect [PATH] [--rules FILE] [--format json|text|tsv]`. Default output is the TSV `stack\tpreset\tmode\tpreset_missing` so `scan.sh` can adopt it without rewriting its parser.
- `app/crates/app-type-detector-cli/tests/cli.rs` — `assert_cmd`-based smoke tests over the same fixtures.

##### `app/bindings/node/` (npm package)

- `app/bindings/node/Cargo.toml` — `napi-rs` crate.
- `app/bindings/node/src/lib.rs` — N-API exports: `detectPath(path)`, `detectFiles({files, listing})`, `defaultRuleset()`. Uses `napi-derive` to generate TS types automatically.
- `app/bindings/node/package.json` — name `@snam/app-type-detector`, optional-deps for each prebuilt binary triple, `napi.triples` config.
- `app/bindings/node/index.js` — `napi-rs` shim.
- `app/bindings/node/index.d.ts` — generated, committed.
- `app/bindings/node/__test__/index.test.ts` — `vitest` round-trip test that fetches files from `tests/fixtures/` and asserts the same report the Rust integration test produces.
- `app/bindings/node/README.md` — npm-facing usage docs, install matrix, fallback notes.

##### `app/bindings/python/` (PyPI package)

- `app/bindings/python/Cargo.toml` — `pyo3` crate.
- `app/bindings/python/src/lib.rs` — `pyo3` module exporting `detect_path`, `detect_files`, `default_ruleset`, plus dataclass-shaped Python returns.
- `app/bindings/python/pyproject.toml` — `maturin` build backend, project metadata, classifiers.
- `app/bindings/python/python/app_type_detector/__init__.py` — re-exports + `__all__`, plus type stubs.
- `app/bindings/python/python/app_type_detector/py.typed` — PEP 561 marker.
- `app/bindings/python/tests/test_detect.py` — `pytest` parity test over the same fixtures.

#### Examples

- `app/examples/rust-fs-walk/` — `cargo run -- /path/to/repo`.
- `app/examples/node-fixture/` — `node index.js` showing both filesystem and in-memory invocation.
- `app/examples/python-fixture/` — `python detect.py` mirror.

#### Scripts

- `scripts/dev.sh` — install toolchains (`rustup component add clippy rustfmt`, `cargo install cargo-nextest cargo-edit`), `pnpm install` for the Node binding, `uv sync` for the Python binding.
- `scripts/start.sh` — kept as a stub that delegates to `cargo run -p app-type-detector-cli -- detect .` so the README's "scripts to start" affordance is honored.
- `scripts/test-all.sh` — `cargo nextest run --workspace`, then `pnpm --dir app/bindings/node test`, then `cd app/bindings/python && uv run pytest`.
- `scripts/lint-all.sh` — `cargo clippy --all-targets --workspace -- -D warnings && cargo fmt --all -- --check`.
- `scripts/build-rules.sh` — validate `default_rules.json` against the JSON schema, also emit a generated `RULES.md` reference doc.
- `scripts/release-crate.sh` — `cargo publish` (called by CI).
- `scripts/release-npm.sh` — `napi build --release --target <triple>` per triple, `napi prepublish`, `npm publish`.
- `scripts/release-pypi.sh` — `maturin publish` per triple.

#### CI

- `.github/workflows/ci.yml` — fmt + clippy + nextest + node test + python test on Linux/macOS, plus a separate WASM build job for the npm fallback.
- `.github/workflows/release-crate.yml` — triggered by `crate-vX.Y.Z` git tag.
- `.github/workflows/release-npm.yml` — matrix per triple (`ubuntu-latest`, `macos-13`, `macos-14`, `windows-latest`), uploads to npm. Triggered by `npm-vX.Y.Z` tag.
- `.github/workflows/release-pypi.yml` — matrix per triple via `maturin-action`. Triggered by `pypi-vX.Y.Z` tag.

#### Docs

- `docs/00-architecture.md` — diagram + invariants.
- `docs/01-rules.md` — the rule grammar (any_of / all_of / all_of_glob OR groups / none_of), worked examples.
- `docs/02-rust-usage.md`.
- `docs/03-node-usage.md`.
- `docs/04-python-usage.md`.
- `docs/05-cli-usage.md`.
- `docs/06-adding-a-new-stack.md` — JSON edit + test fixture + bump default-ruleset version, that's it.
- `docs/RULES.md` — generated reference of every bundled rule.
- `docs/07-migration-version-skill.md` — exact diff for `scan.sh`.
- `docs/08-migration-buildlog-enricher.md` — exact diff for the BuildLog enricher.
- `ai_docs/napi-rs-overview.md` — distilled `napi-rs` reference (publish flow, triples matrix).
- `ai_docs/pyo3-maturin-overview.md` — distilled `pyo3` + `maturin` reference (build wheels, ABI3 vs per-version).

#### Project metadata

- `LICENSE` (MIT, since this is meant to be reused).
- `CHANGELOG.md` — keep-a-changelog format, three sections (Rust crate, npm, PyPI).

## Implementation Plan

### Phase 1: Foundation

Stand up an empty Rust workspace with the three crates (`app-type-detector`, `app-type-detector-cli`, plus the two bindings) and wire CI so every push runs fmt + clippy + nextest. Pin types and the `BuildCategory` / stack vocabulary so neither binding ever drifts. Land the default ruleset as a JSON file plus a build.rs that validates it. Author 15+ minimal fixture directories (one per stack) and prove they round-trip through `MemorySnapshot` end-to-end before any binding code exists. The phase ends when `cargo nextest run` passes against fixtures and `default_ruleset()` returns the canonical ruleset.

### Phase 2: Core Implementation

Port the `detect_stack` evaluator from `scan.sh` to Rust line-for-line, including the pipe-separated `all_of_glob` OR-group behavior and the first-match-wins ordering. Add the second-layer category inference: take the matched stacks plus content-derived signals (Stripe in `package.json`/`Cargo.toml`/`pyproject.toml`, pricing/signup/subscribe regex in README, store links, MCP SDK dependency, Claude skill `SKILL.md` signature, browser extension `manifest.json` schema) and map them to `BuildCategory`. Each rule carries a confidence weight and a list of supporting signals so the final report explains itself. Land the FS-backed snapshot with a curated ignore list (`.git`, `node_modules`, `dist`, `.next`, `target`, `build`, `.venv`, `.gradle`). Add the parity tests against both `versioning-scan.config.json` and the BuildLog `kind-inference.test.ts` cases. Finish the CLI binary and prove it produces the exact TSV row `scan.sh` expects.

### Phase 3: Integration

Stand up the two language bindings in parallel:

- **Node binding** via `napi-rs`. Build a per-triple matrix in CI, publish optional-dep packages, ship a WASM fallback for environments where the native addon refuses to load. The exported TypeScript surface is generated by `napi-derive` so the `.d.ts` is always in sync. Add a `vitest` smoke test that pulls every fixture and compares the report shape to a snapshot.
- **Python binding** via `pyo3` + `maturin`. Build wheels for the same triples plus ABI3 for forward Python-version compatibility. Add a `pytest` parity test mirroring the Rust one. Ship `py.typed` so editors get type info.

Then wire two consumer-side migration PRs (drafted, not landed in this spec):

- The `/version` skill swaps the bash `detect_stack` function for a single `app-type-detector detect --format tsv` shell-out. Behavior must be byte-identical (the parity test gates it).
- BuildLog's enricher swaps `kind-inference.ts` and the file allowlist for `import { detectFiles } from '@snam/app-type-detector'`, downgrading the LLM call from "pick a category from 21 options" to "validate this category and write copy".

Tag and publish `crate-v0.1.0`, `npm-v0.1.0`, `pypi-v0.1.0` (the three release workflows handle the rest). Document the migration paths.

## Step by Step Tasks

IMPORTANT: Execute every step in order, top to bottom.

### 1. Initialize git and project metadata

- `git init` if not already done; verify `.gitignore` covers `target/`, `node_modules/`, `dist/`, `.venv/`, `*.node`.
- Add `LICENSE` (MIT).
- Update `README.md` with the real elevator pitch (currently a scaffold) and a one-paragraph "what this is".
- Create empty `CHANGELOG.md`.

### 2. Scaffold the Rust workspace

- Write `app/Cargo.toml` declaring a workspace with members `crates/app-type-detector`, `crates/app-type-detector-cli`, `bindings/node`, `bindings/python`, and shared `[workspace.dependencies]` for `serde`, `serde_json`, `globset`, `walkdir`, `regex`, `once_cell`, `thiserror`, `clap`, `napi`, `napi-derive`, `pyo3`.
- Write `app/rust-toolchain.toml` pinning stable + `rustfmt`, `clippy`.
- Write `app/.cargo/config.toml` with strict workspace lints (`unused`, `clippy::pedantic` opt-ins).
- `cargo new --lib app/crates/app-type-detector` then prune the auto-generated `src/lib.rs` to a minimal `pub fn version() -> &'static str`.
- Verify `cd app && cargo check --workspace` passes.

### 3. Author the public type surface (compile-only)

- In `app/crates/app-type-detector/src/types.rs` declare `BuildCategory`, `BuildKind`, `DeploymentPlatform`, `StackMatch`, `CategoryMatch`, `Signal`, `DetectionReport` with `serde::Serialize`/`Deserialize` derived behind `serde` feature.
- Mirror the exact strings from `~/Developer/auto-build-log/app/buildlog-worker/src/agents/enricher/schema.ts` so BuildLog's existing enum stays valid after migration.
- Add `lib.rs` re-exports.
- Run `cargo doc --no-deps` and visually skim the generated docs; every public item should have a one-line doc comment that explains *why* the field exists, not what it is.

### 4. Define the rule grammar

- In `app/crates/app-type-detector/src/rules.rs` define `Matcher { any_of, any_of_glob, all_of, all_of_glob, none_of }` with the exact semantics in `scan.sh` lines 90–189. Document the pipe-separated OR-group rule with a worked example in the doc comment.
- Define `StackRule { stack, preset, mode, preset_missing, matcher }`.
- Define `CategoryRule { category, kind, weight, when_stacks: Vec<String>, when_signals: Vec<String>, unless_signals: Vec<String> }`.
- Define `SignalRule { id, file: PathPattern, pattern: Regex }`.
- Define `Ruleset { stack_rules, category_rules, signal_rules }` with `Ruleset::from_json(&str) -> Result<Self>`.
- Unit-test the matcher against tiny in-memory file lists (no FS) to pin OR-group, none-of, and glob behaviors.

### 5. Land the default ruleset

- Author `app/crates/app-type-detector/src/default_rules.json` with the contents of `~/Developer/__versioning_projects/.versioning-scan.config.json` `stack_rules` array verbatim, plus a `category_rules` section that maps:
  - `ios-xcodegen` + xcodeproj listing → `ios_app` (downgrade to `ios_game` if README mentions "game", "tycoon", "idle", "clicker").
  - `xcode-only` → `ios_app` or `mac_app` based on `*.xcworkspace` / Info.plist hints.
  - `node` + `@modelcontextprotocol/sdk` dep → `mcp_server`.
  - `node` + `manifest.json` with `manifest_version` field → `browser_plugin`.
  - `node` + `next`/`astro`/`remix` dep → `web_app`.
  - `node` + `commander`/`yargs` dep + `bin` field → `cli_tool`.
  - `python-pyproject` + `[project.scripts]` → `cli_tool`.
  - `python-pyproject` + FastAPI/Django/Flask dep → `api`.
  - `rust-cargo` + `[[bin]]` and no `[lib]` → `cli_tool`; library otherwise.
  - `obsidian-plugin` → `browser_plugin` (closest fit, also expose `obsidian_plugin` as a future-proof category if the BuildLog enum is extended).
  - `godot` / `unity` → `desktop_app` for now (with TODO to add `pc_game`).
  - `wordpress-plugin` → `library`.
  - Stripe signal → bumps `kind` toward `product`.
- Author `build.rs` that includes the JSON via `include_str!` and parses it once at build time so a malformed default fails the build.
- Author `default_rules.rs` exposing `default_ruleset() -> &'static Ruleset` (cached in a `OnceLock`).

### 6. Implement the snapshot abstraction

- In `app/crates/app-type-detector/src/snapshot.rs` define `trait InputSnapshot { fn glob(&self, pattern: &str) -> Vec<String>; fn read(&self, path: &str) -> Option<Cow<str>>; fn list_root_dirs(&self) -> Vec<String>; }`.
- Implement `MemorySnapshot { files: HashMap<String, Option<String>>, root_dirs: Vec<String> }`.
- Implement `FilesystemSnapshot` (behind `fs` feature) using `walkdir` with the curated ignore list and `globset` for pattern matching. Cap traversal at depth 4 by default to keep `detect_path` fast.
- Unit-test both implementations against a known fixture tree.

### 7. Implement the rule engine

- In `app/crates/app-type-detector/src/engine.rs` write `evaluate(snapshot: &dyn InputSnapshot, rules: &Ruleset) -> DetectionReport`.
- Step A: walk `stack_rules` in order; for each, evaluate the matcher. First-match wins for `primary_stack`, but record every match in `stacks` with its rule index as a confidence prior.
- Step B: walk `signal_rules`, collect every fire into `signals: Vec<Signal>`.
- Step C: walk `category_rules`, score each candidate, sort by weighted score, set `primary_category`.
- Step D: assemble `DetectionReport`.
- Add unit tests for each step in isolation.

### 8. Port the content signals

- In `app/crates/app-type-detector/src/signals.rs` port the regexes from `kind-inference.ts` (Stripe in package.json/Cargo.toml/pyproject, pricing/signup/subscribe in README, app-store/play-store/CWS link in README) into `signal_rules` entries in `default_rules.json`. Compile regexes once in a `LazyLock`.
- Add unit tests with the exact text fixtures from `enrich-kind-inference.test.ts`.

### 9. Build the parity test against the version-skill rules

- Add `app/crates/app-type-detector/tests/parity_versioning_skill.rs`. Read `~/Developer/__versioning_projects/.versioning-scan.config.json` as a fixture (committed copy at `tests/fixtures/versioning-scan.config.json`).
- For each rule, build a `MemorySnapshot` with the minimal files the rule expects and assert the report's `(primary_stack, preset, mode, preset_missing)` tuple matches.

### 10. Build the parity test against BuildLog kind-inference

- Add `app/crates/app-type-detector/tests/parity_kind_inference.rs`. Translate every `it(...)` in `enrich-kind-inference.test.ts` into a Rust test asserting the same `signals` set.

### 11. Build the CLI

- `cargo new --bin app/crates/app-type-detector-cli`.
- Implement `clap`-based parser: `app-type-detector detect [PATH] [--rules <file>] [--format json|text|tsv]`. Default `PATH` is `.`, default `--format` is `tsv` (matches `scan.sh` expectations).
- The `tsv` format prints one line: `stack\tpreset\tmode\tpreset_missing`, identical to today's `detect_stack` output.
- `text` format adds the category, top 5 signals, and confidence.
- `json` format dumps the full `DetectionReport`.
- Add `tests/cli.rs` using `assert_cmd` over the fixture tree.

### 12. Build the Node binding

- Scaffold `app/bindings/node` with `napi build --release` and the `napi.triples` matrix.
- In `src/lib.rs` export `detectPath(path: String) -> DetectionReport`, `detectFiles(input: { files: HashMap<String, Option<String>>, rootDirs: Vec<String> }) -> DetectionReport`, `defaultRuleset() -> Ruleset`. Use `#[napi(object)]` derives to get a TS type per struct.
- Configure `package.json` with `optionalDependencies` for the prebuilt-binary subpackages.
- Add a tiny `index.js` dispatcher and the generated `index.d.ts` checked in.
- `vitest` smoke test that round-trips a fixture through both `detectPath` and `detectFiles` and asserts equality with the Rust integration-test snapshot (committed JSON).

### 13. Build the Python binding

- Scaffold `app/bindings/python` with `maturin new --bindings pyo3`.
- In `src/lib.rs` expose `detect_path(path: PathBuf) -> PyResult<PyObject>` returning a Python dataclass-shaped dict, plus `detect_files`, `default_ruleset`.
- Configure `pyproject.toml` with `[tool.maturin]` for the ABI3 wheels.
- Add `python/app_type_detector/__init__.py` with type-stub re-exports and `py.typed`.
- `pytest` parity test over the same fixtures.

### 14. Wire CI

- `.github/workflows/ci.yml`: matrix `(ubuntu-latest, macos-14)` × `(stable rust, node 20, python 3.12)`. Steps: cargo fmt --check, cargo clippy --workspace --all-targets -- -D warnings, cargo nextest run --workspace, pnpm --dir app/bindings/node test, uv --directory app/bindings/python sync && uv --directory app/bindings/python run pytest.
- `.github/workflows/release-crate.yml`: trigger on `crate-v*` tag, run `cargo publish -p app-type-detector` then `cargo publish -p app-type-detector-cli`.
- `.github/workflows/release-npm.yml`: trigger on `npm-v*` tag, matrix-build per triple via `napi build`, run `napi prepublish`, `npm publish --access public`.
- `.github/workflows/release-pypi.yml`: trigger on `pypi-v*` tag, matrix-build wheels via `PyO3/maturin-action`, then `maturin publish`.

### 15. Author docs

- Write `docs/00-architecture.md`, `docs/01-rules.md`, the three usage docs, the CLI doc, and the "adding a new stack" doc.
- Author the two migration docs (`07-migration-version-skill.md`, `08-migration-buildlog-enricher.md`) with line-for-line diffs.
- Wire `scripts/build-rules.sh` to regenerate `docs/RULES.md` from `default_rules.json` so reference docs never drift.

### 16. Cut the v0.1.0 releases

- Bump the three crates/packages to `0.1.0`.
- Tag `crate-v0.1.0`, `npm-v0.1.0`, `pypi-v0.1.0`.
- Confirm each release workflow succeeded and the published artifacts install via `cargo add app-type-detector`, `npm i @snam/app-type-detector`, `pip install app-type-detector`.

### 17. Run the full validation suite (last step)

- Execute every command in the **Validation Commands** section below.
- All commands must exit zero with no test failures or clippy warnings.

## Testing Strategy

### Unit Tests

- **Rule grammar** (`rules.rs`): each matcher variant in isolation against tiny synthetic `MemorySnapshot`s (any_of literal, any_of_glob, all_of, all_of_glob with single and multi-OR-group, none_of overrides).
- **Default ruleset loading**: parse `default_rules.json` and assert key invariants (every `category_rule.when_stacks` references a real stack, every `signal_rules.id` is unique, every category in `category_rules` is a valid `BuildCategory` enum value).
- **Snapshot implementations**: `MemorySnapshot::glob` matches the same patterns as `FilesystemSnapshot::glob` for an identical synthetic tree.
- **Engine steps**: `evaluate_stacks`, `evaluate_signals`, `evaluate_categories` each tested in isolation with a hand-built ruleset.
- **Signal regexes** (`signals.rs`): one test per regex, both positive and negative cases.

### Integration Tests

- **Filesystem fixtures** (`tests/detect_path.rs`): one fixture directory per supported stack, asserting the full `DetectionReport`.
- **Memory parity** (`tests/detect_memory.rs`): same fixtures fed via `MemorySnapshot`, asserting byte-identical `DetectionReport` to the FS path.
- **Versioning-skill parity** (`tests/parity_versioning_skill.rs`): every rule in `versioning-scan.config.json` round-trips through the new detector with the same `(stack, preset, mode, preset_missing)` tuple.
- **BuildLog kind-inference parity** (`tests/parity_kind_inference.rs`): every test case in `enrich-kind-inference.test.ts` produces the same `signals` set.
- **CLI** (`tests/cli.rs`): the CLI's `tsv` output matches the bash `detect_stack` output for every fixture.
- **Node binding** (`__test__/index.test.ts`): both `detectPath` and `detectFiles` produce a report deeply-equal to a committed JSON snapshot.
- **Python binding** (`tests/test_detect.py`): same as Node, in `pytest`.

### Edge Cases

- Empty directory (no files at all) → `primary_stack = "unknown"`, `primary_category = "utility"`, low confidence.
- Directory with only a `.git/` folder → same as empty, `primary_stack` falls through to the universal catchall.
- Polyglot project (e.g. Rust workspace + Python tooling + Node tooling) → stacks contains all three, `primary_stack` follows the rule order in `default_rules.json` (which mirrors `versioning-scan.config.json` ordering).
- iOS project with `project.yml` AND `*.xcodeproj` → `ios-xcodegen` wins over `xcode-only` (the `none_of` clause on `xcode-only` enforces this).
- Project with `package.json` AND `Cargo.toml` AND `pyproject.toml` → `python-pyproject` wins by rule order (matches today's bash behavior).
- Filesystem snapshot encountering a symlink loop → `walkdir` `.follow_links(false)` prevents infinite recursion.
- Files larger than 64 KB → truncate before regex match, mirroring `github-read.ts`. Document the cap.
- Memory snapshot with `None` for a known path → treat as "file does not exist" (matches the `null` semantics in `github-read.ts`).
- Non-UTF-8 file contents → signals that read content silently skip; rules that only check existence still fire.
- Rule JSON with an unknown matcher key → `Ruleset::from_json` returns a typed error pointing at the offending rule index.
- Caller passes a nonexistent path → `detect_path` returns an `Err`, never panics.

## Acceptance Criteria

- [ ] `cargo nextest run --workspace` passes with zero failures and >=80% coverage on `engine.rs`, `rules.rs`, `signals.rs`.
- [ ] Every `stack_rule` in `~/Developer/__versioning_projects/.versioning-scan.config.json` round-trips through the new detector with the same primary tuple. Asserted by `parity_versioning_skill.rs`.
- [ ] Every test case in `~/Developer/auto-build-log/app/buildlog-worker/tests/enrich-kind-inference.test.ts` produces the same signal set. Asserted by `parity_kind_inference.rs`.
- [ ] `app-type-detector detect --format tsv .` on each fixture produces a TSV row byte-identical to the bash `detect_stack` function's output for the same fixture.
- [ ] `npm i @snam/app-type-detector` then `import { detectPath } from '@snam/app-type-detector'` works on Linux x64, Linux arm64, macOS arm64, macOS x64, Windows x64 (the WASM fallback covers everything else).
- [ ] `pip install app-type-detector` then `from app_type_detector import detect_path` works on the same triples plus ABI3 wheels for Python 3.10–3.13.
- [ ] The published Rust crate has `cargo doc --no-deps` rendering cleanly with no missing-docs warnings on public items.
- [ ] The library never spawns a child process and never opens a network socket. Asserted by a `forbid(unsafe_code)` and a CI grep that fails if `std::process` or `reqwest`/`hyper` is referenced from the core crate.
- [ ] A new stack (e.g. `tauri-app`) can be added by editing `default_rules.json`, adding one fixture, and re-running the test suite. No Rust code changes required for a pure file-presence rule. Documented in `docs/06-adding-a-new-stack.md`.
- [ ] Detection of a typical project completes in under 5 ms on macOS arm64 (Criterion bench `detect.rs`).

## Validation Commands

Execute every command to validate the feature works correctly with zero regressions.

- `cd /Users/seannam/Developer/app-type-detector/app && cargo fmt --all -- --check` — format check
- `cd /Users/seannam/Developer/app-type-detector/app && cargo clippy --workspace --all-targets -- -D warnings` — lint with zero warnings
- `cd /Users/seannam/Developer/app-type-detector/app && cargo nextest run --workspace` — every Rust test (unit + integration + parity)
- `cd /Users/seannam/Developer/app-type-detector/app && cargo test --doc --workspace` — doctests
- `cd /Users/seannam/Developer/app-type-detector/app && cargo bench -p app-type-detector --bench detect -- --quick` — perf bench within budget
- `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli -- detect crates/app-type-detector/tests/fixtures/ios-xcodegen --format tsv` — CLI smoke against an iOS fixture
- `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli -- detect crates/app-type-detector/tests/fixtures/node-mcp --format json` — CLI smoke for an MCP server fixture
- `cd /Users/seannam/Developer/app-type-detector/app/bindings/node && pnpm install && pnpm run build && pnpm test` — Node binding builds and passes its parity test
- `cd /Users/seannam/Developer/app-type-detector/app/bindings/python && uv sync && uv run maturin develop && uv run pytest` — Python binding builds and passes its parity test
- `cd /Users/seannam/Developer/app-type-detector && bash scripts/test-all.sh` — single-shot all-language test suite
- `cd /Users/seannam/Developer/app-type-detector && grep -RIn -E "std::process|reqwest|hyper" app/crates/app-type-detector/src && echo "FAIL: forbidden dep referenced" && exit 1 || echo "OK: core crate has no IO escape hatches"` — invariant check that the core crate has no network or subprocess code paths

## Notes

- **Why Rust core, not Go or pure TS?** Rust gives one binary that ships to crates.io, npm (via `napi-rs` native addons), and PyPI (via `pyo3` + `maturin` wheels) without a runtime. TS would force a Node runtime on Python consumers. Go would force CGO across both bindings. Rust is the pragmatic least-common-denominator for "fast, embeddable, three-ecosystem release path".
- **Why `napi-rs` not WASM for the npm package?** `napi-rs` produces native addons that hit native FS speeds and do not require a sync FS shim. We keep a WASM build as a fallback for Edge/Cloudflare Workers and any environment where loading `.node` files is forbidden. The `package.json` `optionalDependencies` mechanism transparently chooses the right artifact.
- **Why `pyo3` + `maturin` not pure Python?** Same reason: keep one source of truth in Rust. `maturin` produces ABI3 wheels so Python 3.10–3.13 share a single binary per triple, which keeps the wheel matrix small.
- **Naming on npm.** The package will be scoped (`@snam/app-type-detector`) so it can publish without name clashes, even if `app-type-detector` on npm is later taken.
- **New Rust dependencies.** `serde`, `serde_json`, `globset`, `walkdir`, `regex`, `once_cell`, `thiserror` for the core; `clap` for the CLI; `napi` + `napi-derive` for Node; `pyo3` for Python; `criterion` (dev) for benches; `assert_cmd` + `predicates` (dev) for CLI tests; `cargo-nextest` (dev tool, not a crate dep).
- **New Node tooling.** `pnpm`, `vitest`, `@napi-rs/cli`, `typescript`. Installed under `app/bindings/node/`.
- **New Python tooling.** `uv` (already present on Sean's machine), `maturin`, `pytest`, `pyright` (optional, for type-stub linting). Installed under `app/bindings/python/`.
- **Schema versioning.** `default_rules.json` carries a top-level `"schema_version": 1`. The Rust loader rejects unknown versions so future breaking changes to the rule grammar are explicit.
- **Telemetry.** None. The library never phones home.
- **Future categories that the BuildLog enum lacks.** `obsidian_plugin`, `pc_game`, `tauri_app`, `solana_program`, `tvos_game`. The library may emit these as `extras` even when the BuildLog enum cannot persist them; consumers can drop them. We will land them in `default_rules.json` once BuildLog adds them to its DB enum.
- **Migration order.** Land the library at v0.1.0 → migrate the version skill (lowest risk, byte-identical TSV path) → migrate BuildLog (downgrades the LLM call to a validator). Each migration is a separate PR in its own repo, not part of this spec.
- **Out of scope.** A web UI for visualizing detection results, a daemon mode that watches for project changes, and any LLM-powered fallback for when the rule engine returns low confidence (BuildLog will keep that locally for now). All three are good v0.2 follow-ups.
