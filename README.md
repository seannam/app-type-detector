# app-type-detector

A generic, reusable library for classifying any codebase. Hand it a directory
(or an in-memory file map) and it hands back a typed `DetectionReport` with:

- **`app_type`** — the *role* of the project (`game`, `web_app`, `mobile_app`,
  `desktop_app`, `cli_tool`, `library`, `mcp_server`, `claude_skill`, …).
- **`tech_stack`** — the *how*: languages, build systems, runtimes, platforms,
  databases, CI systems, and optional domain sub-records (`web`, `mobile`,
  `desktop`, `game`, `extension`).
- **`scorecard`** — a machine-readable trace of every rule that fired, the
  predicates each rule matched, and the fields it contributed to.

`app_type` and `tech_stack` are intentionally orthogonal: a Godot mobile game
is `app_type = "game"` AND `tech_stack.game.engines = ["godot"]` AND
`tech_stack.platforms = ["ios", "android"]` — never a single collapsed string.

## What's in this repo

- `app/crates/app-type-detector` — the core Rust crate (library + engine +
  default ruleset + human renderer).
- `app/crates/app-type-detector-cli` — a CLI binary wrapping the core crate.
- `app/bindings/node` and `app/bindings/python` — planned language bindings
  (directories scaffolded, implementations are v0.2 work).
- `docs/` — output format spec, rule grammar reference, vocabulary notes.

## Quick start (Rust)

```rust
use app_type_detector::detect_path;

let report = detect_path("./my-project")?;
println!("{}", report.to_json());
```

## CLI

```sh
cd app
cargo run -p app-type-detector-cli -- detect ./my-project --format json
cargo run -p app-type-detector-cli -- detect ./my-project --format text
```

Formats: `json`, `text`, `tsv`, `fires-jsonl`. Default is `text`.

## Properties

- No network I/O, no child processes, no telemetry.
- Treats unknown / polyglot / empty codebases as legitimate inputs that return a
  low-confidence answer rather than an error.
- Pure functions: the engine, the synthesizer, and the renderer never touch
  global state. The same input always produces the same output.
- Human-readable rendering lives in the library but consumes only the JSON
  shape, so it ships identically across bindings.

## Project layout

```
├── app/crates/app-type-detector/      # core library
├── app/crates/app-type-detector-cli/  # CLI binary
├── app/bindings/{node,python}/        # planned language bindings (v0.2)
├── specs/                             # feature specs
├── docs/                              # vocabulary, rule grammar, output format
└── CHANGELOG.md, LICENSE (MIT)
```

## License

MIT. See [LICENSE](LICENSE).
