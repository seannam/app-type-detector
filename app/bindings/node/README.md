# `@indiecraft/app-type-detector`

Classify any codebase from Node.js. Given a directory on disk or an
in-memory file map, returns a typed `DetectionReport` describing the
project's app type (game, web app, CLI tool, library, …), tech stack
(languages, build systems, runtimes, platforms, frameworks), and a
machine-readable scorecard of every rule that fired.

Thin native binding around the Rust core crate via
[`napi-rs`](https://napi.rs/). Ships as a **single npm package** with
all six prebuilt `.node` binaries bundled inside: no `node-gyp`, no
post-install toolchain, no network I/O at install time, no per-triple
subpackages.

## Install

```sh
npm i @indiecraft/app-type-detector
```

The loader picks the right binary for your platform at runtime; on
Linux it auto-detects glibc vs musl.

## Example

```ts
import {
  detectPath,
  detectFiles,
  defaultRuleset,
  renderHumanReadable,
} from "@indiecraft/app-type-detector";

const report = detectPath("./my-project");
console.log(report.app_type.primary, report.app_type.confidence);
console.log(renderHumanReadable(report));
```

`detectFiles` classifies an in-memory file map (useful for editor
integrations, CI jobs that already have contents loaded, or
sandboxed environments):

```ts
const fromMemory = detectFiles({
  files: {
    "package.json": JSON.stringify({ dependencies: { next: "14.0.0" } }),
    "pages/index.tsx": null, // null = file exists but contents unknown
  },
});
```

`defaultRuleset()` returns the bundled ruleset as a plain JS object,
and `renderHumanReadable(report)` reproduces the CLI's `--format text`
output.

## Triple matrix

| Triple              | OS       | CPU     | libc   |
| ------------------- | -------- | ------- | ------ |
| `darwin-arm64`      | macOS    | Apple   | —      |
| `darwin-x64`        | macOS    | x86\_64 | —      |
| `linux-x64-gnu`     | Linux    | x86\_64 | glibc  |
| `linux-arm64-gnu`   | Linux    | aarch64 | glibc  |
| `linux-x64-musl`    | Linux    | x86\_64 | musl   |
| `win32-x64-msvc`    | Windows  | x86\_64 | —      |

A WASM fallback for browser / edge runtimes is planned.

Full usage guide, troubleshooting, and release notes live in
[`docs/05-node-usage.md`](https://github.com/seannam/app-type-detector/blob/main/docs/05-node-usage.md).

## License

MIT. Source at
[github.com/seannam/app-type-detector](https://github.com/seannam/app-type-detector).
