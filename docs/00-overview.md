# Overview

## What this library is

A deterministic rule engine that classifies a codebase into:

- a `DetectionReport.app_type` role (what *kind* of thing this project is);
- a `DetectionReport.tech_stack` description (which languages, frameworks,
  platforms, databases, CI, etc. it uses);
- a `DetectionReport.scorecard` explaining exactly which rules fired, which
  predicates they matched, and which fields they contributed to.

## What this library is not

- **Not an LLM.** Rules are declarative; detection is O(files) and produces the
  same report for the same input every time.
- **Not a network client.** No HTTP, no telemetry, no metrics.
- **Not a subprocess runner.** Never spawns `git`, `find`, `node`, or anything
  else.
- **Not a taxonomy dictator.** `AppType` and `TechStack` are intentionally
  open-ended and non-exhaustive; consumers translate them into their own
  taxonomies via a small lookup table on their side.

## Design shape

- **Inputs:** either a directory on disk (`detect_path`) or an in-memory file
  map (`detect_files`).
- **Engine:** pure rule evaluation against an `InputSnapshot`.
- **Synthesizer:** the only thing that decides which claims win. Applies the
  dominance-margin rule for `app_type`, unions list fields by weight, picks
  scalar fields by highest weight.
- **Renderer:** consumes only the JSON-shaped report, so the human output
  never drifts from the machine output.

## Status (v0.1.0)

- Rust core + CLI shipped and tested.
- 24+ fixtures covering Unity, Godot, Bevy, Next.js + Prisma + Stripe, Astro,
  FastAPI, SwiftUI iOS, Jetpack Compose Android, Tauri, Electron, MCP servers,
  Claude skills, Chrome extensions, Rust CLI + library, polyglot monorepo,
  empty dir, and git-only dir.
- Node and Python bindings scaffolded under `app/bindings/` as v0.2 work.
