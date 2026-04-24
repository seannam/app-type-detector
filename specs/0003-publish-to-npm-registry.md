# Feature: Publish `@indiecraft/app-type-detector` to the npm registry

## Feature Description

Ship the first real npm release of `@indiecraft/app-type-detector` — a thin
[napi-rs](https://napi.rs/) binding that wraps the existing Rust core crate and
gives Node.js consumers the same `detectPath`, `detectFiles`, `defaultRuleset`,
and `renderHumanReadable` surface that Rust and the CLI already expose. The
package ships per-triple prebuilt native binaries as optional-dep subpackages
(no `node-gyp`, no post-install toolchain), a loader stub that picks the right
binary at import time, committed TypeScript typings, and a golden-snapshot test
suite that round-trips the same fixtures used by the Rust crate to guarantee
byte-identical `DetectionReport` JSON across bindings.

The work covers three concerns:

1. **Binding crate.** A new `app-type-detector-node` crate under
   `app/bindings/node` that depends on the core crate and exposes the four
   N-API entrypoints.
2. **npm package.** `app/bindings/node/package.json` (`@indiecraft/app-type-detector`),
   the loader stub (`index.js`), committed types (`index.d.ts`), a README with
   worked examples mirroring spec `0000`, and a `vitest` parity suite.
3. **Release pipeline.** The existing version harness under
   `scripts/version/` is the single source of truth for bumps, tags,
   CHANGELOG generation, and GitHub releases. We extend it — not replace
   it — so npm releases ride the same conventional-commit → tag flow as
   the crate:
   - The active `rust-cargo` preset at `scripts/version/presets/rust-cargo.json`
     gains additional `sync_targets` for every `package.json` in
     `app/bindings/node/` (root + six triple subpackages). When
     `auto-release-on-push.yml` runs `scripts/version/sync.sh rust-cargo <X.Y.Z>`,
     all eight version numbers move in lockstep with `app/Cargo.toml`.
   - A new `.github/workflows/release-npm.yml` triggers on the same
     `v*.*.*` tag the existing `scripts/version/release.sh cut` emits
     (`on: push: tags: ['v[0-9]+.[0-9]+.[0-9]+']`), reads the canonical
     version via `scripts/version/current.sh --raw`, and runs a six-triple
     build matrix (`linux-x64-gnu`, `linux-arm64-gnu`, `darwin-x64`,
     `darwin-arm64`, `win32-x64-msvc`, `linux-x64-musl`). It publishes
     one subpackage per triple
     (`@indiecraft/app-type-detector-linux-x64-gnu`, …) and finally the
     root `@indiecraft/app-type-detector` package that references them as
     `optionalDependencies`. `npm i @indiecraft/app-type-detector` pulls
     exactly one native subpackage matching the installer's
     `os`/`cpu`/`libc`.
   - No separate `npm-v*` tag: the monorepo tag drives every channel,
     and `scripts/version/release.sh` remains the only code path that
     talks to `git tag`, `git push --tags`, or `gh release`.

Zero runtime dependencies on the consumer side. No network I/O, no child
processes, no telemetry — inherited from the core crate.

## User Story

As a Node.js developer building a tool that needs to classify a codebase,
I want to `npm i @indiecraft/app-type-detector` and call `detectPath("./my-project")`,
So that I get the same typed `DetectionReport` the Rust crate produces, without
running a Rust toolchain, managing a child process, or paying CGO / WASM
overhead on every call.

## Problem Statement

Spec `0000-extract-app-type-detector-library.md` declared three publishing
channels (crates.io, npm, PyPI) as acceptance criteria for v0.1.0, but the
implementation landed only the Rust core and the CLI — `app/bindings/node/`
is a placeholder directory with a single `README.md`. Node consumers currently
have no path to this library other than spawning `app-type-detector` as a
subprocess, which:

- Violates the "zero subprocess" contract for downstream tools embedding the
  detector in latency-sensitive code paths.
- Forces consumers to ship their own binary per platform and handle
  `ENOENT`/`EACCES`/PATH bootstrapping.
- Loses the typed `DetectionReport` object: consumers must re-parse JSON and
  re-derive TypeScript types from `docs/02-output-format.md` by hand.
- Leaves the spec-`0000` acceptance criterion `npm i @indiecraft/app-type-detector`
  unmet.

The ecosystem expects first-class Node support for a "what is this codebase?"
library, and the existing design (napi-rs + prebuilt binaries + WASM fallback)
is documented but unbuilt.

## Solution Statement

Build the `napi-rs` binding exactly as spec `0000` anticipated, close the
spec-0000 acceptance gap, and release `@indiecraft/app-type-detector@0.2.0` to
npm. Concretely:

- A new `app-type-detector-node` Cargo crate (cdylib) uses `napi` +
  `napi-derive` to expose four entrypoints. The crate joins the workspace so
  `cargo build --workspace` keeps passing but is excluded from
  `cargo test --workspace` (cdylib-only build, no Rust tests live here; parity
  is asserted from the Node side).
- `napi build --platform --release` emits one `.node` artifact per triple
  plus a generated `index.d.ts` / `index.js`. We commit the generated
  `index.d.ts` (stable, reviewable TS surface) and write a hand-rolled
  `index.js` loader that resolves the correct per-triple subpackage at import
  time, matching the pattern used by `@napi-rs/*` packages (e.g. `better-sqlite3`, `simple-git-hooks`).
- The package exposes the same JSON shape byte-for-byte as the Rust crate.
  Vitest tests load the committed Rust golden snapshots (copied or re-used
  from `app/crates/app-type-detector/tests/fixtures/**`) and assert
  `deepEqual`, guaranteeing parity.
- A new `.github/workflows/release-npm.yml` fans out a build matrix on
  the existing `v*.*.*` tag (the same tag `auto-release-on-push.yml` +
  `scripts/version/release.sh cut` already emit), uploads one prebuilt
  `.node` per platform-subpackage to npm, then publishes the root
  package. Secrets: `NPM_TOKEN` (scoped to `@indiecraft`). The root
  auto-release workflow stays untouched — it continues to own version
  bumping, manifest syncing, tagging, and the GitHub release, and the
  new workflow purely consumes its tag output.
- Versioning: npm package starts at `0.2.0` to align with the current
  crate workspace version (`app/Cargo.toml` → `version = "0.2.0"`).
  Future bumps are emitted by `scripts/version/sync.sh` via the extended
  `rust-cargo` preset — no separate bump script, no divergence risk.
  `scripts/version/current.sh --raw` is the canonical version read
  inside the npm workflow.
- Docs: update the root `README.md` Node snippet to show real install and
  usage; refresh `app/bindings/node/README.md` with the worked examples
  from spec `0000`; add `docs/05-node-usage.md` (new — anticipated by spec
  `0000` but not yet written).

## Relevant Files

Use these files to implement the feature:

- `README.md` — root pitch; update the "Quick start" section to include a
  Node snippet alongside the Rust one, matching the pattern anticipated in
  `specs/0000`.
- `app/Cargo.toml` — workspace root; add `bindings/node` as a workspace
  member and pin `napi` / `napi-derive` / `napi-build` in
  `[workspace.dependencies]`.
- `app/crates/app-type-detector/src/lib.rs` — source of truth for the public
  API the binding must re-export (`detect_path`, `detect_files`,
  `default_ruleset`, `render_human_readable`, `DetectionReport`).
- `app/crates/app-type-detector/tests/fixtures/**` — golden fixtures that
  the Node parity test must round-trip byte-identically. Current layout
  ships Unity, Godot, Next.js, polyglot, empty-dir, etc.
- `app/crates/app-type-detector-cli/src/main.rs` — reference for how the
  library is called end-to-end; the Node binding mimics the same call path.
- `scripts/test-all.sh` — current test runner; extend it to also run the
  Node binding tests when `pnpm` is present.
- `scripts/version/presets/rust-cargo.json` — extend `sync_targets` so
  every `package.json` under `app/bindings/node/` (root + six
  per-triple subpackages) syncs alongside `app/Cargo.toml` when the
  auto-release workflow runs `sync.sh`. This is the hinge change that
  makes the npm package ride the crate's version for free.
- `scripts/version/current.sh` — read-only; the npm workflow calls it
  with `--raw` to resolve the version being published.
- `scripts/version/release.sh` — read-only; remains the sole code path
  that creates `v*.*.*` tags and GitHub releases. The npm workflow
  never writes tags or releases itself.
- `scripts/version/changelog.sh` — read-only; `auto-release-on-push.yml`
  already uses it to update `CHANGELOG.md`. The npm release inherits
  whatever section was just appended.
- `justfile` — add `just node-build` and `just node-test` recipes so the
  Node flow is discoverable.
- `.github/workflows/auto-release-on-push.yml` — existing release flow;
  leave it unchanged. It keeps owning "decide bump, sync manifests,
  update CHANGELOG, commit, tag, release." The new npm workflow
  listens for its output tag.
- `CHANGELOG.md` — append an `npm (@indiecraft/app-type-detector) · 0.2.0` entry
  under the Unreleased block.

### New Files

#### Binding crate

- `app/bindings/node/Cargo.toml` — `napi-rs` crate (crate-type `["cdylib"]`),
  depends on `app-type-detector` path-dependency, `napi = { version = "2",
  features = ["napi6", "serde-json"] }`, `napi-derive = "2"`, `serde_json`.
- `app/bindings/node/src/lib.rs` — N-API entrypoints:
  - `#[napi] pub fn detect_path(path: String) -> Result<serde_json::Value>`
  - `#[napi] pub fn detect_files(snapshot: DetectFilesInput) -> Result<serde_json::Value>`
  - `#[napi] pub fn default_ruleset() -> serde_json::Value`
  - `#[napi] pub fn render_human_readable(report: serde_json::Value) -> String`
- `app/bindings/node/build.rs` — calls `napi_build::setup()`.
- `app/bindings/node/.npmignore` — strips Rust sources from the published
  tarball (only `index.js`, `index.d.ts`, `package.json`, `README.md`, and
  the chosen `.node` artifact ship).

#### Distribution (npm-facing)

- `app/bindings/node/package.json` — root package metadata:
  - `"name": "@indiecraft/app-type-detector"`
  - `"version": "0.2.0"`
  - `"main": "index.js"`, `"types": "index.d.ts"`
  - `"license": "MIT"`
  - `"os"`: `["darwin", "linux", "win32"]`
  - `"cpu"`: `["x64", "arm64"]`
  - `"optionalDependencies"`: `{ "@indiecraft/app-type-detector-linux-x64-gnu": "0.2.0", …one entry per triple }`
  - `"scripts"`: `{ "build": "napi build --platform --release", "build:debug": "napi build --platform", "test": "vitest run", "prepublishOnly": "napi prepublish -t npm --skip-gh-release" }`
  - `"devDependencies"`: `@napi-rs/cli`, `vitest`, `typescript`, `@types/node`.
- `app/bindings/node/index.js` — hand-rolled loader:
  1. Detect `process.platform`, `process.arch`, and (on Linux) glibc vs musl
     via a small `isMusl()` probe copied from the `@napi-rs/cli` template.
  2. `require('@indiecraft/app-type-detector-' + triple)` and re-export its
     exports.
  3. Throw a helpful error if no matching subpackage is installed
     (listing the detected triple and pointing to the GitHub issues URL).
- `app/bindings/node/index.d.ts` — committed, hand-reviewed TS surface:
  ```ts
  export interface DetectionReport { /* mirrors docs/02-output-format.md */ }
  export function detectPath(path: string): DetectionReport;
  export function detectFiles(input: {
    files: Record<string, string | null>;
    rootDirs?: string[];
  }): DetectionReport;
  export function defaultRuleset(): unknown;
  export function renderHumanReadable(report: DetectionReport): string;
  ```
- `app/bindings/node/npm/linux-x64-gnu/package.json`,
  `app/bindings/node/npm/linux-arm64-gnu/package.json`,
  `app/bindings/node/npm/linux-x64-musl/package.json`,
  `app/bindings/node/npm/darwin-x64/package.json`,
  `app/bindings/node/npm/darwin-arm64/package.json`,
  `app/bindings/node/npm/win32-x64-msvc/package.json` — one skeleton each,
  carrying correct `os`/`cpu`/`libc` fields and pointing at a placeholder
  `.node` file that CI writes at release time.
- `app/bindings/node/npm/<triple>/README.md` — one-liner "internal artifact
  of `@indiecraft/app-type-detector`; do not install directly."

#### Tests

- `app/bindings/node/__test__/index.test.ts` — Vitest parity suite:
  - Load every fixture directory under
    `app/crates/app-type-detector/tests/fixtures/<name>/`.
  - For each, call `detectPath(fixturePath)` and the CLI
    (`cargo run -p app-type-detector-cli -- detect <path> --format json`),
    parse both, and `expect(nodeReport).toEqual(cliReport)`.
  - Also assert `renderHumanReadable` parity by comparing against
    `app/crates/app-type-detector/tests/render/*.txt` snapshots.
- `app/bindings/node/__test__/loader.test.ts` — unit-tests `index.js`'s
  triple-resolution logic on mocked `process.platform` / `process.arch`
  / `isMusl()` inputs.
- `app/bindings/node/__test__/fixtures/memory-snapshot.test.ts` — covers
  `detectFiles` with an in-memory map, asserting the same report shape as
  an equivalent on-disk fixture.
- `app/bindings/node/vitest.config.ts` — Vitest configuration (Node
  environment, root `app/bindings/node`).
- `app/bindings/node/tsconfig.json` — strict TS, targets Node 20.

#### Release pipeline

- `.github/workflows/release-npm.yml` — matrix job that rides the
  existing monorepo tag:
  - `on: push: tags: ['v[0-9]+.[0-9]+.[0-9]+']` — the same tag
    `scripts/version/release.sh cut` emits from
    `auto-release-on-push.yml`. No bespoke `npm-v*` trigger.
  - `resolve-version` job: runs `scripts/version/current.sh --raw` to
    read the canonical version, exports it as an output so every
    downstream job uses the same value (never parses `$GITHUB_REF_NAME`
    itself — the version scripts are the only source of truth).
  - `build-<triple>` × 6 jobs: each runs
    `napi build --platform --release --target <rust-target>` and
    uploads the `*.node` artifact.
  - `publish` job: `needs: [resolve-version, build-<6>]`; downloads
    every artifact, writes each into its
    `npm/<triple>/app-type-detector.<triple>.node`, sanity-checks each
    `package.json` version equals the resolved version (guards against
    preset-drift), then runs `napi prepublish -t npm --skip-gh-release`
    and `npm publish --access public` for each subpackage followed by
    the root package.
  - The workflow never creates tags, never runs `gh release create`,
    and never edits `CHANGELOG.md`. All of that already happened in
    `auto-release-on-push.yml` before this workflow fired.
  - Secrets: `NPM_TOKEN` scoped to publish on `@indiecraft`.
- `scripts/node-test.sh` — wraps `pnpm -C app/bindings/node install && pnpm run build:debug && pnpm test`
  so `scripts/test-all.sh` can call it behind a feature gate. Pure
  local helper; no release duties.

#### Docs

- `docs/05-node-usage.md` — new: install, import, the three worked examples
  from spec `0000` transposed to TypeScript, a note on how to consume the
  scorecard, the triple matrix, and the troubleshooting section for
  "no matching prebuilt binary" errors.
- Root `README.md` gets a Node section next to the existing Rust snippet.

## Implementation Plan

### Phase 1: Foundation

Stand up the `napi-rs` binding crate, wire it into the workspace, and prove
it can produce a loadable `.node` file locally. Land the hand-rolled
`index.js` loader, the committed `index.d.ts`, and a minimal "smoke" vitest
test that calls `detectPath` on the `cli-rust` fixture and asserts the
returned object has `app_type.primary === "cli_tool"`. Phase ends when
`pnpm -C app/bindings/node run build:debug && pnpm -C app/bindings/node test`
passes on the developer's laptop.

### Phase 2: Parity and ergonomics

Bring the parity suite online: round-trip every fixture through both the
Node binding and the CLI, deep-equal the JSON, and snapshot the human
renderer. Add `detectFiles` coverage, loader unit tests, and negative-path
tests (missing path, invalid ruleset). Extend `scripts/test-all.sh` to run
the Node suite when `pnpm` is available. Update `justfile` with
`node-build` and `node-test` recipes.

### Phase 3: Release pipeline and publish

Extend the `rust-cargo` preset at
`scripts/version/presets/rust-cargo.json` with seven new `sync_targets`
(one per `package.json` in `app/bindings/node/`) so the next
auto-release-on-push run sets every npm manifest to the same version as
`app/Cargo.toml` without any bespoke bump script. Author
`release-npm.yml` with the six-triple matrix, per-triple subpackage
scaffolds, and the `napi prepublish` orchestration, triggered on the
existing `v*.*.*` tag. Wire `NPM_TOKEN`. Dry-run on a fork
(`npm publish --dry-run`) to verify tarball contents. Merge a
release-worthy commit to `main`; `auto-release-on-push.yml` emits
`v0.3.0` (or whatever semver the preset picks), which in turn triggers
`release-npm.yml`. Confirm
`npm i @indiecraft/app-type-detector` on a fresh machine correctly
installs exactly one prebuilt subpackage and runs a hello-world
detection. Close the spec-`0000` acceptance criterion:
"`npm i @indiecraft/app-type-detector` then
`import { detectPath } from '@indiecraft/app-type-detector'`
works on Linux x64, Linux arm64, macOS arm64, macOS x64, Windows x64".

## Step by Step Tasks

IMPORTANT: Execute every step in order, top to bottom.

### 1. Pin napi-rs dependencies in the workspace

- Add `napi`, `napi-derive`, and `napi-build` under
  `[workspace.dependencies]` in `app/Cargo.toml`.
- Add `app/bindings/node` to the workspace `members` list.
- Run `cd app && cargo check --workspace` to confirm the empty member is
  accepted.

### 2. Scaffold the binding crate

- Write `app/bindings/node/Cargo.toml` with `crate-type = ["cdylib"]`,
  path-dependency on `app-type-detector`, and the three napi deps.
- Write `app/bindings/node/build.rs` that calls `napi_build::setup()`.
- Write a minimal `src/lib.rs` that re-exports `default_ruleset()` via
  `#[napi]`. Verify `cd app/bindings/node && cargo build` produces a
  `.dylib` / `.so` / `.dll` under `target/debug/`.

### 3. Implement the four N-API entrypoints

- Implement `detect_path`, `detect_files`, `default_ruleset`,
  `render_human_readable` in `src/lib.rs`.
- Each returns `serde_json::Value` (napi's `serde-json` feature handles the
  JS conversion). The `detect_files` input is a Rust struct with `#[napi(object)]`
  containing `files: HashMap<String, Option<String>>` and optional
  `root_dirs: Vec<String>`.
- Errors surface as napi `Error::from_reason(...)`.
- Generate TS via `napi build --platform --release --dts index.d.ts`. Hand-
  edit the generated `index.d.ts` to tighten the return types into the
  committed `DetectionReport` interface, and commit the final file.

### 4. Author the loader and per-triple skeletons

- Write `app/bindings/node/index.js` using the `@napi-rs/cli` loader
  pattern (runtime detection of `process.platform`, `process.arch`,
  `isMusl()`, then `require(@indiecraft/app-type-detector-<triple>)`).
- Write six `app/bindings/node/npm/<triple>/package.json` skeletons with
  `"name"`, `"version": "0.2.0"`, `"os"`, `"cpu"`, `"libc"` (where
  applicable), `"main": "app-type-detector.<triple>.node"`, and a README
  explaining that the subpackage is internal.
- Write the root `package.json` with all six subpackages in
  `optionalDependencies`.

### 5. Build locally and write the smoke test

- `pnpm -C app/bindings/node install`
- `pnpm -C app/bindings/node run build:debug`
- Write `__test__/smoke.test.ts` that requires `./index.js` and calls
  `detectPath(path.join(__dirname, "..", "..", "..", "crates/app-type-detector/tests/fixtures/cli-rust"))`.
- Assert `report.app_type.primary === "cli_tool"`.
- `pnpm -C app/bindings/node test` must pass.

### 6. Author the full parity suite

- Enumerate every fixture under
  `app/crates/app-type-detector/tests/fixtures/*` at test boot.
- For each fixture, shell out to the CLI for ground-truth JSON and call
  `detectPath` from the binding. `expect(nodeReport).toEqual(cliReport)`.
- Add a `renderHumanReadable` golden-snapshot assertion per fixture against
  `app/crates/app-type-detector/tests/render/<fixture>.txt` (which
  already exists for the Rust golden tests).
- Add `__test__/fixtures/memory-snapshot.test.ts` covering `detectFiles`
  with a hand-built in-memory map.

### 7. Loader unit tests

- Add `__test__/loader.test.ts` that imports `index.js` via a factory that
  accepts injected `process.platform` / `process.arch` / `isMusl()` values.
- Cases: all six supported triples resolve correctly; unsupported triple
  (e.g. `freebsd-x64`) throws a descriptive error.

### 8. Wire local tooling

- Update `scripts/test-all.sh`: after the Rust section, if `command -v pnpm`
  is present and `app/bindings/node/node_modules` exists (or install was
  forced), run `bash scripts/node-test.sh`.
- Add `scripts/node-test.sh` implementing
  `pnpm -C app/bindings/node install --frozen-lockfile && pnpm -C app/bindings/node run build:debug && pnpm -C app/bindings/node test`.
- Update `justfile` with `node-build`, `node-test`, and `node-pack`
  (`pnpm -C app/bindings/node pack`) recipes.

### 9. Extend the version preset

- Edit `scripts/version/presets/rust-cargo.json` and append seven
  entries to `sync_targets`, one per `package.json`:
  - `{app_root}/bindings/node/package.json` (JSON selector `.version`, `primary: false`)
  - `{app_root}/bindings/node/npm/linux-x64-gnu/package.json`
  - `{app_root}/bindings/node/npm/linux-arm64-gnu/package.json`
  - `{app_root}/bindings/node/npm/linux-x64-musl/package.json`
  - `{app_root}/bindings/node/npm/darwin-x64/package.json`
  - `{app_root}/bindings/node/npm/darwin-arm64/package.json`
  - `{app_root}/bindings/node/npm/win32-x64-msvc/package.json`
- Locally test by running
  `scripts/version/sync.sh rust-cargo $(scripts/version/current.sh --raw)`
  and confirming every `package.json` matches `app/Cargo.toml`'s version.
- Commit the preset edit alongside the seeded `package.json` files so
  the first post-merge `auto-release-on-push.yml` run sees them in sync.

### 10. Release workflow

- Create `.github/workflows/release-npm.yml`:
  - Trigger: `on: push: tags: ['v[0-9]+.[0-9]+.[0-9]+']` — matches the
    tag `scripts/version/release.sh cut` produces. No `npm-v*` trigger.
  - `resolve-version` job: checks out the repo, runs
    `VERSION=$(./scripts/version/current.sh --raw)`, exports as a job
    output. Every downstream job consumes this, never parses the ref
    name directly.
  - `build-<triple>` matrix of six jobs: each runs
    `pnpm -C app/bindings/node install --frozen-lockfile` then
    `pnpm -C app/bindings/node exec napi build --platform --release --target <rust-target>`,
    uploading `*.node` as an artifact.
  - `publish` job: `needs: [resolve-version, build-*]`; downloads every
    artifact, asserts `jq -r .version` of each `package.json` equals
    the resolved version (fails loudly on drift), runs
    `pnpm -C app/bindings/node exec napi prepublish -t npm --skip-gh-release`,
    and publishes each per-triple subpackage with
    `npm publish --access public` (setting `NODE_AUTH_TOKEN=${{ secrets.NPM_TOKEN }}`).
    Publishes the root package last so atomicity is preserved (root
    references the subpackages via `optionalDependencies`).
  - The workflow contains zero `git tag`, `git push --tags`, or
    `gh release` calls. Those stay exclusive to `scripts/version/release.sh`.
- Add the `NPM_TOKEN` secret guidance in `docs/05-node-usage.md` so
  future maintainers can rotate it.

### 11. Docs

- Write `docs/05-node-usage.md` with:
  - Install snippet.
  - Three worked examples (Unity, Next.js, polyglot) — `import { detectPath }`
    plus `console.log(JSON.stringify(report, null, 2))`.
  - Triple matrix.
  - Troubleshooting (no matching prebuilt binary; how to rebuild locally
    with `pnpm run build`).
- Replace `app/bindings/node/README.md` with npm-facing content (install,
  import, worked example, triple matrix, link to `docs/05-node-usage.md`).
- Update the root `README.md` Quick start to include Node alongside Rust.
- Append an `npm · 0.2.0` entry to `CHANGELOG.md`.

### 12. Dry-run publish

- Run `scripts/version/sync.sh rust-cargo $(scripts/version/current.sh --raw)`
  locally; verify every `package.json` under `app/bindings/node/` now
  matches `app/Cargo.toml`'s version. Commit any preset-driven sync
  diff.
- `pnpm -C app/bindings/node run build && pnpm -C app/bindings/node pack`
  — inspect the tarball contents (must contain only `index.js`,
  `index.d.ts`, `package.json`, `README.md`).
- `pnpm -C app/bindings/node publish --dry-run --access public` —
  confirms npm accepts the package shape and the `@indiecraft` scope.
- GitHub-dry: on a fork, create a prerelease tag via the real harness
  (`./scripts/version/release.sh seed --version 0.2.0-rc.1 --prerelease`),
  push, confirm `release-npm.yml` builds all six triples and would
  publish, then delete the tag + release. Do not run the dry-run
  against the main repo.

### 13. Real publish

- Merge a release-worthy commit to `main` (for example, the
  preset-sync commit from step 9). `auto-release-on-push.yml` runs,
  `scripts/version/release.sh cut` emits `v0.2.0` (or the next
  conventional-commit-driven bump), and `release-npm.yml` fires on
  that tag.
- Wait for `release-npm.yml` to succeed (build + publish jobs green).
- On a fresh machine: `mkdir /tmp/probe && cd /tmp/probe && pnpm init -y && pnpm i @indiecraft/app-type-detector`
  then run `node -e "console.log(require('@indiecraft/app-type-detector').detectPath('.').app_type)"`.
- Confirm exactly one subpackage was installed (matching the probe's
  triple).

### 14. Run full validation

- Execute every command in **Validation Commands**.

## Testing Strategy

### Unit Tests

- **Loader resolution** (`__test__/loader.test.ts`): injected platform /
  arch / libc combinations resolve to the correct subpackage name; invalid
  combinations throw a descriptive error.
- **N-API edge cases** (`__test__/smoke.test.ts`): calling `detectPath` on
  a nonexistent path returns a well-formed JS `Error` (not a panic).
- **`detectFiles` input validation**: missing `files` key throws; values
  that are neither string nor `null` throw.

### Integration Tests

- **Fixture parity** (`__test__/index.test.ts`): for every directory under
  `app/crates/app-type-detector/tests/fixtures/`, the Node binding's JSON
  output deep-equals the CLI's JSON output. This is the strongest guarantee
  of parity because it exercises the exact same core-crate entrypoints.
- **Render parity** (`__test__/render.test.ts`):
  `renderHumanReadable(report)` matches the Rust golden under
  `tests/render/<fixture>.txt`.
- **Memory snapshot parity**
  (`__test__/fixtures/memory-snapshot.test.ts`): a hand-built in-memory map
  representing the `cli-rust` fixture produces the same report as
  `detectPath` on the real directory.
- **Release pipeline dry-run** (`v0.2.0-rc.1` prerelease tag seeded via
  `scripts/version/release.sh seed --version 0.2.0-rc.1 --prerelease`
  on a fork): verifies `release-npm.yml` builds all six triples without
  attempting to publish to the real npm registry.

### Edge Cases

- Consumer installs on `freebsd-x64` → `index.js` throws a readable error
  mentioning the detected triple.
- Consumer installs with `--no-optional` → `index.js` throws the same error
  (no native addon available).
- Consumer installs on an Alpine image → musl libc probe returns `true`,
  `@indiecraft/app-type-detector-linux-x64-musl` is loaded.
- Fixture with non-UTF-8 file contents → the binding doesn't panic; the
  content rules silently skip matching (inherited from the core crate).
- `detectFiles({ files: { "package.json": null } })` treats `null` as
  "file does not exist" and still produces a valid report.
- Calling `renderHumanReadable` on a malformed JS object (missing
  `scorecard`) throws a descriptive napi error rather than crashing the
  process.
- `npm publish --dry-run` of the root package lists exactly
  `index.js`, `index.d.ts`, `package.json`, `README.md` (no Rust sources,
  no `target/`).

## Acceptance Criteria

- [ ] `npm i @indiecraft/app-type-detector` on a fresh machine installs exactly
      one prebuilt subpackage matching the machine's triple and produces a
      working `detectPath(".")` call that returns a typed `DetectionReport`.
- [ ] `import { detectPath, detectFiles, defaultRuleset, renderHumanReadable } from "@indiecraft/app-type-detector"`
      type-checks under `"strict": true` TypeScript with no `any`.
- [ ] The Node binding's JSON output deep-equals the CLI's JSON output for
      every fixture in `app/crates/app-type-detector/tests/fixtures/`.
      Asserted by `__test__/index.test.ts`.
- [ ] `renderHumanReadable` output matches the committed Rust golden
      snapshot per fixture. Asserted by `__test__/render.test.ts`.
- [ ] Prebuilt binaries ship for these six triples: `linux-x64-gnu`,
      `linux-arm64-gnu`, `linux-x64-musl`, `darwin-x64`, `darwin-arm64`,
      `win32-x64-msvc`. A seventh WASM fallback subpackage is an explicit
      non-goal for this spec and is tracked in **Notes → Out of scope**.
- [ ] The release workflow publishes atomically: if any per-triple
      subpackage publish fails, the root package is not published.
      Implemented via job ordering (`publish` job `needs: [resolve-version, build-*]`
      and publishes subpackages before the root package within the same job).
- [ ] `release-npm.yml` contains zero `git tag`, `git push --tags`, or
      `gh release` commands. `scripts/version/release.sh` remains the
      only code path that talks to the git remote for tags / releases.
- [ ] Version synchronization is driven by the `rust-cargo` preset's
      extended `sync_targets`, not by a bespoke bump script. Asserted
      by a CI check that runs
      `scripts/version/sync.sh rust-cargo $(scripts/version/current.sh --raw) --check`
      and fails if any `package.json` under `app/bindings/node/` drifts
      from `app/Cargo.toml`.
- [ ] `app/bindings/node/package.json` is not published with Rust sources:
      `npm publish --dry-run` reports only `index.js`, `index.d.ts`,
      `package.json`, `README.md`, and the `.node` artifact.
- [ ] `scripts/test-all.sh` runs the Node test suite when `pnpm` is
      available and passes.
- [ ] `CHANGELOG.md` has an `npm · 0.2.0` entry describing the initial
      release; `docs/05-node-usage.md` exists and mirrors the three worked
      examples from `specs/0000-extract-app-type-detector-library.md`.
- [ ] The library's vocabulary is unchanged: the binding emits the same
      `AppType` and `TechStack` enum values as the Rust crate (i.e. the
      binding is a pass-through, not a re-mapper). Asserted by the parity
      suite.
- [ ] `app/bindings/node/README.md` is no longer a placeholder; it is
      npm-facing documentation that renders correctly on npmjs.com.

## Validation Commands

Execute every command to validate the feature works correctly with zero
regressions.

- `cd /Users/seannam/Developer/app-type-detector/app && cargo fmt --all -- --check` — workspace fmt
- `cd /Users/seannam/Developer/app-type-detector/app && cargo clippy --workspace --all-targets -- -D warnings` — workspace lint (covers the new binding crate)
- `cd /Users/seannam/Developer/app-type-detector/app && cargo build -p app-type-detector-node --release` — native addon builds in release mode
- `pnpm -C /Users/seannam/Developer/app-type-detector/app/bindings/node install --frozen-lockfile` — reproducible install
- `pnpm -C /Users/seannam/Developer/app-type-detector/app/bindings/node run build` — napi build succeeds on the local triple
- `pnpm -C /Users/seannam/Developer/app-type-detector/app/bindings/node test` — unit + parity + render + memory-snapshot suites pass
- `pnpm -C /Users/seannam/Developer/app-type-detector/app/bindings/node pack --pack-destination /tmp && tar -tzf /tmp/indiecraft-app-type-detector-0.2.0.tgz | sort` — published tarball contains only expected files
- `pnpm -C /Users/seannam/Developer/app-type-detector/app/bindings/node exec tsc --noEmit --project tsconfig.json` — TS surface compiles under strict mode
- `cd /Users/seannam/Developer/app-type-detector && bash scripts/test-all.sh` — aggregated Rust + Node suite
- `cd /Users/seannam/Developer/app-type-detector && bash scripts/node-test.sh` — Node-only shortcut
- `cd /Users/seannam/Developer/app-type-detector && scripts/version/sync.sh rust-cargo $(scripts/version/current.sh --raw)` — re-sync every manifest in the extended rust-cargo preset and confirm no drift between `app/Cargo.toml` and the seven `package.json` files under `app/bindings/node/`
- `cd /Users/seannam/Developer/app-type-detector && jq -r .version app/bindings/node/package.json app/bindings/node/npm/*/package.json | sort -u` — exactly one line of output, equal to `$(scripts/version/current.sh --raw)`; multiple lines means drift
- `cd /Users/seannam/Developer/app-type-detector && act -j build --matrix triple:darwin-arm64 -W .github/workflows/release-npm.yml` — optional local dry-run of the release workflow via `act`; skip if `act` is unavailable
- `cd /tmp && rm -rf npm-probe && mkdir npm-probe && cd npm-probe && pnpm init -y && pnpm i @indiecraft/app-type-detector@0.2.0 && node -e "const d = require('@indiecraft/app-type-detector'); console.log(d.detectPath(process.cwd()).app_type.primary)"` — post-publish end-to-end smoke test on a fresh directory
- `grep -RIn "app-type-detector" /tmp/npm-probe/node_modules/@indiecraft/ | head -20` — confirms exactly one native subpackage landed

## Notes

- **New Rust dependencies.** `napi = "2"`, `napi-derive = "2"`, `napi-build = "2"`
  added to `[workspace.dependencies]` in `app/Cargo.toml` and wired into
  `app/bindings/node/Cargo.toml`. `napi` is pulled with the `serde-json`
  and `napi6` features. No `uv add` because this is not a Python package;
  the Python binding lands in a separate follow-up spec.
- **New Node tooling.** `pnpm` (existing preference in the test-all
  script), `@napi-rs/cli` (dev), `vitest` (dev), `typescript` (dev),
  `@types/node` (dev). All live in `app/bindings/node/package.json`.
- **Why napi-rs, not WASM as the default?** Native addons hit native-FS
  speeds and avoid a sync-FS shim for `detectPath`. WASM is deferred to a
  follow-up spec (fallback subpackage `@indiecraft/app-type-detector-wasm32-wasi`)
  so Edge / Cloudflare Workers can consume the library. Deferring WASM keeps
  the initial matrix small and unblocks the spec-`0000` acceptance criterion.
- **Why ride the existing `v*.*.*` tag instead of a bespoke `npm-v*`
  trigger?** Because `scripts/version/release.sh` is already the sole
  authority on tags and GitHub releases, and the `rust-cargo` preset is
  the single source of truth for what version a given commit is. Adding
  a parallel `npm-v*` tag scheme would create two versioning worlds
  that could drift, duplicate the bump-decision logic, and violate the
  "one script owns the git remote" invariant that `release.sh` enforces.
  The npm workflow is a pure consumer: it listens for the monorepo tag
  the harness already emits. The six-triple build matrix takes 10–15
  minutes but runs in parallel and runs only on tag pushes (not on
  every `main` commit), which is a strict subset of release-worthy
  events — no gating needed beyond the tag filter. Per-channel
  CHANGELOG sections in `CHANGELOG.md` still make sense for human
  readability, but they no longer imply per-channel tags.
- **Version alignment at launch.** We launch npm at `0.2.0` (matching the
  current crate) rather than `0.1.0` because the crate has already shipped
  a `0.2.0` on crates.io workspace-wide, and consumers expect the
  single-source-of-truth version numbers across channels for the same
  feature set. Post-launch, each channel bumps independently.
- **Out of scope.** The Python (PyPI) binding, the WASM fallback
  subpackage, a CDN-hosted WASM build for browser consumers, a daemon
  mode, and the LLM fallback for low-confidence reports. All anticipated
  in spec `0000` and cleanly addressable in follow-up specs after `0.2.0`
  adoption.
- **Security.** No `postinstall` script. No network I/O at runtime. No
  binary downloads at install time (optional-deps replace that pattern
  entirely). The `NPM_TOKEN` secret scope is "Publish → Packages" limited
  to the `@indiecraft` scope.
- **Harness-first by design.** The crate and the npm package now share
  one version pipeline. Any future channel (PyPI, WASM fallback,
  crates.io re-publish) is a preset `sync_targets` extension plus a
  new tag-triggered publish workflow. The pattern is: extend the
  preset, listen to the existing tag, never add a new tag scheme.
