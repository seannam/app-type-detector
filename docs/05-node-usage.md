# Node.js usage

`@indiecraft/app-type-detector` is a thin
[`napi-rs`](https://napi.rs/) binding around the Rust core crate. It
ships as a **single npm package** with every prebuilt native binary
bundled inside: no `node-gyp`, no post-install toolchain, no network
I/O at install time, no per-triple subpackages.

## Install

```sh
npm i @indiecraft/app-type-detector
# or: pnpm add @indiecraft/app-type-detector
# or: yarn add @indiecraft/app-type-detector
```

A single tarball contains all six prebuilt `.node` binaries (~15 MB
unpacked, ~6 MB gzipped). The loader picks the right one for your
platform at runtime; on Linux it auto-detects glibc vs musl.

## Usage

```ts
import {
  detectPath,
  detectFiles,
  defaultRuleset,
  renderHumanReadable,
} from "@indiecraft/app-type-detector";

// Classify a directory on disk.
const report = detectPath("./my-project");
console.log(report.app_type.primary, report.app_type.confidence);

// Classify an in-memory file map.
const fromMemory = detectFiles({
  files: {
    "Cargo.toml": '[package]\nname = "demo"\nversion = "0.1.0"\n',
    "src/main.rs": 'fn main() { println!("hi"); }',
  },
});
console.log(fromMemory.tech_stack.languages.primary); // "rust"

// Read the bundled default ruleset.
const rules = defaultRuleset();

// Render a report as human-readable text.
console.log(renderHumanReadable(report));
```

### Worked example: Unity project

```ts
import { detectPath } from "@indiecraft/app-type-detector";

const report = detectPath("/path/to/unity-game");
if (report.app_type.primary === "game" && report.tech_stack.game) {
  console.log("engine:", report.tech_stack.game.engines[0]);
  console.log("version:", report.tech_stack.game.engine_version);
}
```

### Worked example: Next.js app

```ts
import { detectPath } from "@indiecraft/app-type-detector";

const report = detectPath("/path/to/nextjs-app");
if (report.tech_stack.web) {
  console.log(
    "frameworks:",
    report.tech_stack.web.frameworks.join(", ")
  );
  console.log("databases:", report.tech_stack.databases);
}
```

### Worked example: polyglot monorepo

```ts
import { detectPath } from "@indiecraft/app-type-detector";

const report = detectPath("/path/to/monorepo");
for (const lang of report.tech_stack.languages.all) {
  console.log(
    `${lang.language} (${lang.role ?? "unclassified"}):`,
    lang.file_count,
    "files"
  );
}
```

## The scorecard

Every report carries a machine-readable `scorecard`. Each entry in
`scorecard.fires` names the rule that fired, the evidence it matched,
and the fields it contributed to. This makes the output explainable:

```ts
for (const fire of report.scorecard.fires) {
  console.log(
    `${fire.rule_id} (weight ${fire.weight}) ->`,
    fire.contributes_to.map((c) => c.field).join(", ")
  );
}
```

## Triple matrix

| Triple              | OS       | CPU     | libc   | Notes                     |
| ------------------- | -------- | ------- | ------ | ------------------------- |
| `darwin-arm64`      | macOS    | Apple   | —      | Apple Silicon             |
| `darwin-x64`        | macOS    | x86\_64 | —      | Intel Mac                 |
| `linux-x64-gnu`     | Linux    | x86\_64 | glibc  | Standard distributions    |
| `linux-arm64-gnu`   | Linux    | aarch64 | glibc  | Raspberry Pi 5, AWS Graviton |
| `linux-x64-musl`    | Linux    | x86\_64 | musl   | Alpine, `node:alpine`     |
| `win32-x64-msvc`    | Windows  | x86\_64 | —      | MSVC runtime              |

A WASM fallback for browser / edge runtimes is tracked for a future
release.

## Troubleshooting

### "failed to load native binding for `<triple>`"

The single bundled package contains every supported `.node` binary
in its tarball, so this should not happen on a clean install. If it
does, your install was truncated or your platform's binary was
stripped (some packagers ignore unknown file extensions). Try:

```sh
rm -rf node_modules
npm i @indiecraft/app-type-detector
```

### "unsupported platform/arch combination"

The detected triple isn't in the matrix above. The loader names the
triple it detected; please file an issue on GitHub with that string.
FreeBSD and 32-bit Windows are not supported.

### Building from source

You need a Rust toolchain, `pnpm`, and the `@napi-rs/cli` dev
dependency:

```sh
cd app/bindings/node
pnpm install
pnpm run build        # release build
pnpm run build:debug  # debug build
pnpm test             # runs the parity suite
```

The loader prefers a freshly-built artifact under
`app/bindings/node/app-type-detector.<triple>.node`, so rebuilds are
picked up immediately.

## Release and maintenance notes

The npm channel rides the same `v*.*.*` git tag the Rust crate does:
`scripts/version/release.sh` is the only code path that tags and
creates GitHub releases. The `.github/workflows/release-npm.yml`
workflow listens for that tag, runs the six-triple build matrix in
parallel, copies every `.node` artifact into the package root, and
runs a single `npm publish --access public --provenance`.

### Why a single bundled package, not optional-dep subpackages

The napi-rs default ships `@scope/pkg-<triple>` as separate
`optionalDependencies`. We deliberately do not. One package means:

- One npm publish per release (vs seven).
- One trusted-publisher entry on npm.com (vs seven).
- One version source of truth (no drift between root and subpackages).
- One thing to think about when something goes wrong.

The cost is download size: every consumer pulls all six binaries
(~6 MB gzipped) instead of one. For a tool used at build / CI time,
that trade-off is worth the operational simplification.

### Publishing credentials

Publishing uses npm Trusted Publishing (OIDC) — no `NPM_TOKEN` secret
is stored anywhere. The trusted publisher is configured once on
npmjs.com under `@indiecraft/app-type-detector → Settings →
Trusted publishing`, with:

- Provider: GitHub Actions
- Organization: `seannam`
- Repository: `app-type-detector`
- Workflow filename: `release-npm.yml`
- Environment: *(blank)*

Trusted publishing requires the package to exist already, so the
**first publish** is done locally with `npm publish --access public`
(authenticated via the maintainer's passkey through `auth-type=web`).
After the package exists, configure the trusted publisher and every
subsequent release flows through CI.
