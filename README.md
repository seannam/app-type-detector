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
- `app/bindings/node` — [`@indiecraft/app-type-detector`](https://www.npmjs.com/package/@indiecraft/app-type-detector) N-API binding
  (see [`docs/05-node-usage.md`](docs/05-node-usage.md)).
- `app/bindings/python` — planned Python binding (PyPI release tracked
  for a future spec).
- `docs/` — output format spec, rule grammar reference, vocabulary notes.

## Quick start (Rust)

```rust
use app_type_detector::detect_path;

let report = detect_path("./my-project")?;
println!("{}", report.to_json());
```

## Quick start (Node.js)

[![npm](https://img.shields.io/npm/v/@indiecraft/app-type-detector.svg)](https://www.npmjs.com/package/@indiecraft/app-type-detector)

```sh
npm i @indiecraft/app-type-detector
# or: pnpm add @indiecraft/app-type-detector
# or: yarn add @indiecraft/app-type-detector
```

```ts
import { detectPath, renderHumanReadable } from "@indiecraft/app-type-detector";

const report = detectPath("./my-project");
console.log(report.app_type.primary, report.app_type.confidence);
console.log(renderHumanReadable(report));
```

Ships as a **single npm package** with all six prebuilt native
binaries bundled inside (darwin-arm64, darwin-x64, linux-x64-gnu,
linux-arm64-gnu, linux-x64-musl, win32-x64-msvc) via `napi-rs`. No
`node-gyp`, no post-install toolchain, no network I/O at install
time, no per-triple optional dependencies. The loader picks the
right binary at runtime; on Linux it auto-detects glibc vs musl.

Requires Node `>= 18`. Full guide:
[`docs/05-node-usage.md`](docs/05-node-usage.md).

## CLI

```sh
cd app
cargo run -p app-type-detector-cli -- detect ./my-project --format json
cargo run -p app-type-detector-cli -- detect ./my-project --format text
```

Formats: `json`, `text`, `tsv`, `fires-jsonl`. Default is `text`.

## Just commands

If you have [`just`](https://github.com/casey/just) installed, the `justfile` at
the repo root wraps the most common flows:

```sh
just                              # list all recipes
just detect                       # scan the current working directory (text)
just detect ./my-project          # scan a specific directory
just detect ./my-project json     # pick a format: text (default), json, tsv, fires-jsonl
just detect-release ./my-project  # same as detect, but builds the CLI in release mode
just test                         # fmt + clippy + cargo test (scripts/test-all.sh)
```

Paths are resolved to absolute paths, so `just detect` works the same whether
you run it from the repo root or any subdirectory.

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
├── app/bindings/node/                 # @indiecraft/app-type-detector (napi-rs)
├── app/bindings/python/               # planned PyPI binding
├── specs/                             # feature specs
├── docs/                              # vocabulary, rule grammar, output format
└── CHANGELOG.md, LICENSE (MIT)
```

## License

MIT. See [LICENSE](LICENSE).
