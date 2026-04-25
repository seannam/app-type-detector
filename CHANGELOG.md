# Changelog

## [0.4.0] - 2026-04-25

### New

- bundle native binaries into a single npm package


## [0.3.4] - 2026-04-24


### Fixes

- run darwin-x64 build on macos-14 (cross-compile)


## [0.3.3] - 2026-04-24


### Fixes

- tighten build.rs comment wording
- drop --frozen-lockfile; sync.sh bumps optionalDependencies


## [0.3.2] - 2026-04-24


### Fixes

- comment napi_build extern crate
- trigger release-npm via workflow_run, not tag push


## [0.3.1] - 2026-04-24


### Fixes

- wire npm publishing via OIDC Trusted Publishing
- retrigger npm release workflow for v0.3.0 publish


## [0.3.0] - 2026-04-24

### New

- publish @indiecraft/app-type-detector to npm
- expand default ruleset to cover 10 new real-world stacks

### Fixes

- expand default ruleset to cover Bun/Fastify, Python bots, Rust workspaces


## [0.2.0] - 2026-04-24

### New

- extract app-type-detector library, CLI, fixtures, and docs
- initial commit

## Unreleased

### New

- **npm (`@indiecraft/app-type-detector`) · 0.2.0** — first real Node
  release. Ships the `napi-rs` binding (`detectPath`, `detectFiles`,
  `defaultRuleset`, `renderHumanReadable`) with prebuilt native
  binaries for six triples (`linux-x64-gnu`, `linux-arm64-gnu`,
  `linux-x64-musl`, `darwin-x64`, `darwin-arm64`, `win32-x64-msvc`) as
  optional-dep subpackages. Includes committed TypeScript typings,
  byte-identical parity with the Rust crate's JSON output asserted by
  `__test__/index.test.ts`, and a release workflow
  (`.github/workflows/release-npm.yml`) that rides the existing
  monorepo `v*.*.*` tag so the crate and the npm channel stay in
  lockstep via the extended `rust-cargo` version preset.

_Adopted by /version:adopt; prior history below is preserved as-is._

# Changelog

All notable changes to `app-type-detector` are tracked here, split by release
track. Each release track is versioned independently.

## Rust crate (`app-type-detector`)

### 0.1.0 (unreleased)
- Initial release.
- Core detection engine: `detect_path`, `detect_files`, rule grammar, synthesizer.
- Default ruleset covering Unity, Godot, Bevy, Next.js, Astro, FastAPI, SwiftUI,
  Jetpack Compose, Tauri, Electron, MCP servers, Claude skills, browser and
  editor extensions, Rust CLIs and libraries, common databases, CI, containers,
  linting, testing, and payment tooling.
- CLI binary (`app-type-detector detect [PATH] [--format ...]`).
- Human-readable renderer that consumes only the JSON shape.

## npm (`@snam/app-type-detector`)

### 0.1.0 (unreleased)
- Planned: `napi-rs` native bindings with prebuilt binaries per triple and a WASM
  fallback.

## PyPI (`app-type-detector`)

### 0.1.0 (unreleased)
- Planned: `pyo3` + `maturin` ABI3 wheels for Python 3.10+.
