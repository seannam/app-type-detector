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
