# Node.js usage

`@indiecraft/app-type-detector` is a thin
[`napi-rs`](https://napi.rs/) binding around the Rust core crate. It
ships per-triple prebuilt binaries as optional-dep subpackages: no
`node-gyp`, no post-install toolchain, no network I/O at install time.

## Install

```sh
npm i @indiecraft/app-type-detector
# or: pnpm add @indiecraft/app-type-detector
# or: yarn add @indiecraft/app-type-detector
```

The root package pulls exactly one native subpackage matching your
machine's triple via `optionalDependencies`. On Linux the loader
auto-detects glibc vs musl at runtime and loads the correct binary.

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

Usually means the optional dependency didn't install. Check for:

- `--no-optional` or `npm_config_optional=false` in your environment.
- A lockfile that was generated on a different triple and re-used as-is.

Fix by reinstalling with optional deps enabled:

```sh
rm -rf node_modules
npm i --include=optional @indiecraft/app-type-detector
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
`app/bindings/node/app-type-detector.<triple>.node` over the installed
optional dependency, so rebuilds are picked up immediately.

## Release and maintenance notes

The npm channel rides the same `v*.*.*` git tag the Rust crate does:
`scripts/version/release.sh` is the only code path that tags and
creates GitHub releases. The `.github/workflows/release-npm.yml`
workflow listens for that tag, runs the six-triple build matrix, and
publishes each subpackage followed by the root package.

Maintainers rotating the publishing credential: the `NPM_TOKEN`
secret is scoped to "Publish → Packages" on the `@indiecraft` npm
organization. Rotate it under the repository's Actions secrets.
