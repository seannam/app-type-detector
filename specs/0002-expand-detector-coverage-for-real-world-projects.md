# Feature: Expand detector coverage for real-world project shapes

## Feature Description

Spec 0001 closed the largest ruleset gaps (Bun/Fastify, Python bots, cargo
workspaces). This spec closes the next tier, measured directly against the
`~/Developer/__versioning_projects/versioning_scan.json` corpus of 112 real
projects via `scripts/detector-scan-test.py`.

Baseline measurement on that corpus (ruleset v0.2.0):

- 112 projects total.
- 38 projects get a populated `app_type.primary` (34%).
- 74 projects report `app_type.primary = null` (66%).
- `scripts/detector-scan-test.py` currently exits **1** because 2 SwiftPM
  library projects (`ReviewKit`, `VersionGateKit`) fail their
  `language=swift` expectation with a fully empty tech_stack.

This spec adds ~10 new rules that collectively drive the scan test to pass
and raise the populated-`app_type` rate on this corpus above 60%, without
regressing any existing integration test.

## User Story

As someone who runs `detect` across every repo on my laptop (and as any
tool building on this crate), I want the detector to give me a concrete
`app_type.primary` for the common shapes I actually ship — SwiftPM
libraries, Node CLIs and libraries, Remix / SvelteKit / Nuxt apps, Python
CLIs, Python libraries, WordPress plugins, Obsidian plugins — so that I
don't have to keep squinting at a `null` `app_type` and guessing. When the
detector adds a rule for a stack I use, I want my tooling to inherit the
answer automatically the next time I upgrade the crate.

## Problem Statement

The scan test against `~/Developer` exposes four systemic gaps in the
v0.2.0 ruleset:

1. **SwiftPM-only packages are completely invisible.** Projects that have
   `Package.swift` but no `*.xcodeproj` / `project.yml` produce an empty
   report — not even `languages.primary = "swift"`. The two projects
   tagged `spm-library` in the versioning scan return zero fires. This is
   the only hard failure in the current scan test.
2. **Node projects with no web framework return `app_type = null`.** Of
   40 Node projects, 28 fall through because they are CLIs (`package.json`
   with a `"bin"` field), libraries (`"main"` / `"exports"` without a
   framework dep), or use a web framework the ruleset does not yet know
   (Remix, SvelteKit, Nuxt, Gatsby, Vue CLI, Angular).
3. **Python projects with no web framework fall through similarly.** The
   `python-project` baseline from spec 0001 correctly populates
   `languages.primary = "python"`, but there is no rule that claims
   `app_type` for click/typer CLIs, pure libraries, or Celery daemons.
4. **WordPress and Obsidian plugins have no rule at all.** The versioning
   config already names both stacks; the detector is silent on both.

The rule grammar, engine, synthesizer, and renderer are all correct. The
fix is again isolated to `default_rules.json` + fixtures + tests, with one
small update to `scripts/detector-scan-expectations.json` so the scan
test's expectations reflect the new coverage.

## Solution Statement

Extend `app/crates/app-type-detector/src/default_rules.json` with the
following rules, ordered by ROI on the scan corpus:

1. **`spm-library`** — `Package.swift` exists AND no `*.xcodeproj`. Maps to
   `app_type=library`, `languages=swift`,
   `package_managers=swift_package_manager`. Weight `0.8`.
2. **`spm-executable`** — `Package.swift` content matches
   `\.executableTarget` or `products:\s*\[[^\]]*\.executable`. Same weight,
   `app_type=cli_tool`, `languages=swift`.
3. **`node-cli-tool`** — `package.json` content matches `"bin"\s*:` AND
   does NOT match any of the known web-framework keys (`"next"`, `"astro"`,
   `"fastify"`, `"express"`, `"hono"`, `"koa"`, `"remix"`, `"@sveltejs/kit"`,
   `"nuxt"`, `"gatsby"`, `"@angular/core"`, `"vue"`, `"electron"`,
   `"@modelcontextprotocol/sdk"`). Weight `0.8`, `app_type=cli_tool`,
   `languages=javascript`.
4. **`node-library`** — relax `typescript-library` to also fire on JS
   packages. Add a sibling rule `node-library` that requires `"main"` or
   `"exports"` in `package.json` AND a `not` over the same web-framework
   and CLI keys as above. Weight `0.5`, `app_type=library`,
   `languages=javascript`.
5. **Additional web frameworks** (each weight `1.0`, shape mirrors
   `fastify-api`): `remix-app` (web_app), `sveltekit-app` (web_app),
   `nuxt-app` (web_app), `gatsby-site` (static_site), `vue-app` (web_app),
   `angular-app` (web_app).
6. **`python-click-cli`** — `any` of content regexes matching
   `"click"` (word-boundary form) in `pyproject.toml` or `requirements.txt`.
   Weight `0.9`, `app_type=cli_tool`, `languages=python`,
   `frameworks=click`.
7. **`python-typer-cli`** — same shape with `typer`. `frameworks=typer`.
8. **`python-library`** — `pyproject.toml` has `[project]` AND does NOT
   have `[project.scripts]` AND content does NOT match any of the web,
   CLI, or bot framework keys from specs 0001 / 0002. Low weight `0.4`,
   `app_type=library`, `languages=python`.
9. **`wordpress-plugin`** — glob `**/*.php` AND content match for the
   WordPress plugin header (`Plugin Name:` in any top-level `.php` file).
   Weight `1.0`, `app_type=cms_plugin`, `languages=php`,
   `frameworks=wordpress`.
10. **`obsidian-plugin`** — `all` of `file_exists manifest.json`,
    `file_exists versions.json`, content `manifest.json` regex
    `"minAppVersion"\s*:`. Weight `1.0`, `app_type=editor_extension`,
    `extension.host=obsidian`, `extension.kind=plugin`,
    `languages=typescript`.
11. **`html-static-site`** — `file_exists index.html` AND no
    `package.json` AND no `astro`/`next`/`vite` markers. Low weight
    `0.3`, `app_type=static_site`, `languages=html`.

Weights: framework-specific rules stay at the existing 1.0 tier so they
dominate the 0.3 – 0.5 baselines. The `not` sub-expressions on
`node-cli-tool` and `node-library` avoid double-classifying a project as
both CLI and the detected web framework (the synthesizer's 1.5x dominance
check would otherwise flip some Next.js repos to `null` when CLI fires at
0.8 alongside Next at 1.0).

## Steps to Reproduce (baseline)

```sh
cd /Users/seannam/Developer/app-type-detector
cargo build --release -p app-type-detector-cli --quiet

# 1. SwiftPM library — currently empty.
cargo run -p app-type-detector-cli --quiet -- \
  detect /Users/seannam/Developer/VersionGateKit --format json \
  | jq '.app_type.primary, .tech_stack.languages.primary'
# Observed: null, null

# 2. Node CLI — currently null app_type.
cargo run -p app-type-detector-cli --quiet -- \
  detect /Users/seannam/Developer/auto-build-log --format json \
  | jq '.app_type.primary, .tech_stack.languages.primary'
# Observed: null, "typescript"

# 3. Scan test — currently exits 1.
scripts/detector-scan-test.py --jobs 8
echo "exit=$?"
# Observed: exit=1, 2 FAIL from spm-library, ~70 no-expectation.
```

## Root Cause Analysis

Ruleset v0.2.0 has no rule that fires on `Package.swift`, no rule that
fires on `"bin"` in `package.json`, no rule for Remix / SvelteKit / Nuxt /
Vue / Gatsby / Angular, no rule for click / typer, no rule for WordPress
plugin headers, and no rule for Obsidian plugins. Everything in the
fall-through bucket is content-addressable in the manifest files already
in scope. No engine, snapshot, or synthesizer change is required.

The `not`-over-web-framework trick already appears in the existing
`rust-cargo-cli` rule (`not` of `[lib]`), so the grammar supports it
directly — no new `MatchExpr` variant needed.

## Relevant Files

Use these files to close the gaps:

- `app/crates/app-type-detector/src/default_rules.json` — add all new
  rules here. Only source file that must change for the fix itself.
- `app/crates/app-type-detector/src/rules.rs` — read only; confirms grammar
  supports `not` over `any`/`content`, which the Node rules rely on.
- `app/crates/app-type-detector/src/synthesis.rs` — read only; confirms
  `tech_stack.frameworks` (used by click/typer/wordpress) is routed via
  `set_list_field`.
- `app/crates/app-type-detector/src/types/app_type.rs` — read only;
  confirms `cms_plugin`, `editor_extension`, `static_site`, `library`,
  `cli_tool`, `web_app` are all already valid enum values.
- `app/crates/app-type-detector/tests/detect_path.rs` — add one
  `#[test]` per new rule.
- `app/crates/app-type-detector/tests/fixtures/` — new per-rule fixtures.
- `scripts/detector-scan-test.py` — add an `N/A` status for projects
  with no detectable signal (zero rules fired, no language, no app_type)
  and for missing-directory rows. Update the summary line to print the
  N/A count. Exit semantics unchanged: only `FAIL` rows trigger a
  non-zero exit.
- `scripts/detector-scan-expectations.json` — update `stack_expectations`
  for `spm-library`, `obsidian-plugin`, `wordpress-plugin`; populate
  `skip_projects` with `dokploy_server` and `seannam_cloudflare_pages`;
  populate `project_overrides` with the `app-type-detector` entry.

### New Files

- `app/crates/app-type-detector/tests/fixtures/spm-library/Package.swift`
- `app/crates/app-type-detector/tests/fixtures/spm-library/Sources/MyLib/MyLib.swift`
- `app/crates/app-type-detector/tests/fixtures/spm-executable/Package.swift`
- `app/crates/app-type-detector/tests/fixtures/spm-executable/Sources/mycli/main.swift`
- `app/crates/app-type-detector/tests/fixtures/node-cli-tool/package.json`
- `app/crates/app-type-detector/tests/fixtures/node-cli-tool/bin/cli.js`
- `app/crates/app-type-detector/tests/fixtures/node-library/package.json`
- `app/crates/app-type-detector/tests/fixtures/node-library/src/index.js`
- `app/crates/app-type-detector/tests/fixtures/remix-app/package.json`
- `app/crates/app-type-detector/tests/fixtures/remix-app/app/root.tsx`
- `app/crates/app-type-detector/tests/fixtures/sveltekit-app/package.json`
- `app/crates/app-type-detector/tests/fixtures/sveltekit-app/src/routes/+page.svelte`
- `app/crates/app-type-detector/tests/fixtures/nuxt-app/package.json`
- `app/crates/app-type-detector/tests/fixtures/nuxt-app/nuxt.config.ts`
- `app/crates/app-type-detector/tests/fixtures/gatsby-site/package.json`
- `app/crates/app-type-detector/tests/fixtures/gatsby-site/gatsby-config.js`
- `app/crates/app-type-detector/tests/fixtures/vue-app/package.json`
- `app/crates/app-type-detector/tests/fixtures/vue-app/src/main.ts`
- `app/crates/app-type-detector/tests/fixtures/angular-app/package.json`
- `app/crates/app-type-detector/tests/fixtures/angular-app/angular.json`
- `app/crates/app-type-detector/tests/fixtures/python-click-cli/pyproject.toml`
- `app/crates/app-type-detector/tests/fixtures/python-click-cli/src/mycli.py`
- `app/crates/app-type-detector/tests/fixtures/python-typer-cli/pyproject.toml`
- `app/crates/app-type-detector/tests/fixtures/python-typer-cli/src/mycli.py`
- `app/crates/app-type-detector/tests/fixtures/python-library/pyproject.toml`
- `app/crates/app-type-detector/tests/fixtures/python-library/src/mylib/__init__.py`
- `app/crates/app-type-detector/tests/fixtures/wordpress-plugin/composer.json`
- `app/crates/app-type-detector/tests/fixtures/wordpress-plugin/my-plugin.php`
- `app/crates/app-type-detector/tests/fixtures/obsidian-plugin/manifest.json`
- `app/crates/app-type-detector/tests/fixtures/obsidian-plugin/versions.json`
- `app/crates/app-type-detector/tests/fixtures/obsidian-plugin/main.ts`
- `app/crates/app-type-detector/tests/fixtures/html-static-site/index.html`
- `app/crates/app-type-detector/tests/fixtures/html-static-site/styles.css`

## Step by Step Tasks

Execute every step in order, top to bottom.

### 1. Bump ruleset version

- In `default_rules.json` change `"version": "0.2.0"` to `"version": "0.3.0"`.
  Every scorecard carries the ruleset version, so bumping it makes the
  wider coverage visible in output.

### 2. Add SwiftPM rules

- `spm-library` (weight `0.8`): `all` of `file_exists Package.swift` AND
  `not` of glob `**/*.xcodeproj/project.pbxproj`. Contributions:
  `app_type=library` delta `0.8`, `tech_stack.languages=swift`,
  `tech_stack.package_managers=swift_package_manager`,
  `tech_stack.build_systems=swift_package_manager`.
- `spm-executable` (weight `0.9`): `all` of `file_exists Package.swift`
  AND content `Package.swift` regex
  `(?s)\.executableTarget|\.executable\s*\(`. Contributions:
  `app_type=cli_tool` delta `0.9`, same language/build_system.

### 3. Add Node CLI rule

- `node-cli-tool` (weight `0.8`): `all` of `file_exists package.json`,
  content `package.json` regex `"bin"\s*:`, and a single `not` over a
  nested `any` of content regexes matching the known web-framework and
  bot keys (`"next"`, `"astro"`, `"fastify"`, `"express"`, `"hono"`,
  `"koa"`, `"remix"`, `"@sveltejs/kit"`, `"nuxt"`, `"gatsby"`,
  `"@angular/core"`, `"vue"`, `"electron"`, `"@modelcontextprotocol/sdk"`).
  Contributions: `app_type=cli_tool` delta `0.8`,
  `tech_stack.languages=javascript`, `tech_stack.runtimes=node`.

### 4. Add Node library rule

- `node-library` (weight `0.5`): `all` of `file_exists package.json`,
  content regex matching `"main"\s*:` OR `"exports"\s*:`, and `not` over
  the same web/bot framework + `"bin"` key list as step 3.
  Contributions: `app_type=library` delta `0.5`,
  `tech_stack.languages=javascript`, `tech_stack.runtimes=node`.
- Keep the existing `typescript-library` rule unchanged — it will still
  fire additively on packages that declare `"types"`.

### 5. Add web-framework rules (weight `1.0`, shape mirrors `fastify-api`)

- `remix-app`: content `"@remix-run/"` OR `"remix"\s*:`. `app_type=web_app`,
  `backend_frameworks=remix`, `frontend_frameworks=react`,
  `languages=typescript`.
- `sveltekit-app`: content `"@sveltejs/kit"`. `app_type=web_app`,
  `backend_frameworks=sveltekit`, `frontend_frameworks=svelte`,
  `languages=typescript`.
- `nuxt-app`: `any` of `file_exists nuxt.config.ts`,
  `file_exists nuxt.config.js`, content `"nuxt"\s*:`. `app_type=web_app`,
  `backend_frameworks=nuxt`, `frontend_frameworks=vue`,
  `languages=typescript`.
- `gatsby-site`: `any` of `file_exists gatsby-config.js`,
  `file_exists gatsby-config.ts`. `app_type=static_site`,
  `backend_frameworks=gatsby`, `frontend_frameworks=react`,
  `languages=typescript`.
- `vue-app`: content `"vue"\s*:` AND NOT `"nuxt"\s*:`. `app_type=web_app`,
  `frontend_frameworks=vue`, `languages=typescript`.
- `angular-app`: `any` of `file_exists angular.json`, content
  `"@angular/core"`. `app_type=web_app`, `frontend_frameworks=angular`,
  `languages=typescript`.

### 6. Add Python CLI frameworks

- `python-click-cli` (weight `0.9`): `any` of content regexes matching
  `(?i)"click(?:"|[><=!~])` in `pyproject.toml` or `(?mi)^click(?:$|[^\w-])`
  in `requirements.txt`. Contributions: `app_type=cli_tool` delta `0.9`,
  `tech_stack.languages=python`, `tech_stack.runtimes=python`,
  `tech_stack.frameworks=click`.
- `python-typer-cli` (weight `0.9`): same shape for `typer`.
  `frameworks=typer`.

### 7. Add Python library baseline

- `python-library` (weight `0.4`): `all` of `file_exists pyproject.toml`,
  content `[project]` header, and a `not` over content regexes for
  `[project.scripts]`, `fastapi`, `flask`, `django`,
  `python-telegram-bot`, `discord\.py`, `slack[-_]bolt`, `click`, `typer`.
  Contributions: `app_type=library` delta `0.4`,
  `tech_stack.languages=python`, `tech_stack.runtimes=python`.
- Low delta so a framework rule at 1.0 still dominates by the
  synthesizer's 1.5x margin.

### 8. Add WordPress-plugin rule

- `wordpress-plugin` (weight `1.0`): content `**/*.php` glob regex
  matching the plugin header. Implement as `all` of: glob `*.php` (root
  level, `min_count=1`) AND content regex against the first `.php` file's
  path. Since the engine's `content` kind needs a fixed `file` path and
  globs cannot be templated, use the pattern:
  - `any` of content regexes in the common top-level PHP file names:
    `plugin.php`, `main.php`, `index.php`, plus the directory label if
    known. Simpler alternative: match the `composer.json` content for
    `"type"\s*:\s*"wordpress-plugin"` OR match any `**/*.php` glob AND
    `composer.json` content for `wordpress`.
- Final rule: `all` of `file_exists composer.json` AND content
  `composer.json` regex `"type"\s*:\s*"wordpress-plugin"` OR content
  regex `wpackagist|wordpress/wordpress`. Contributions:
  `app_type=cms_plugin` delta `1.0`, `languages=php`,
  `frameworks=wordpress`. (The plugin-header-comment fallback is out of
  scope here — the engine's `content` kind requires a fixed path, and
  adding a templated path is a separate grammar change.)

### 9. Add Obsidian-plugin rule

- `obsidian-plugin` (weight `1.0`): `all` of `file_exists manifest.json`,
  `file_exists versions.json`, and content `manifest.json` regex
  `"minAppVersion"\s*:`. Contributions: `app_type=editor_extension` delta
  `1.0`, `tech_stack.extension.host=obsidian`,
  `tech_stack.extension.kind=plugin`, `languages=typescript`,
  `runtimes=node`.

### 10. Add static-HTML fallback

- `html-static-site` (weight `0.3`): `all` of `file_exists index.html`,
  `not` of `file_exists package.json`, and `not` of
  `file_exists Cargo.toml`. Contributions: `app_type=static_site` delta
  `0.3`, `languages=html`.
- Low delta so that a real framework (Astro/Next/Gatsby) still dominates
  when present.

### 11. Create fixtures (one per new rule)

- Add directories under `tests/fixtures/` from the "New Files" list. Keep
  each to 1 – 3 files — the minimum needed to fire the target rule and no
  more.

### 12. Extend integration tests

- In `tests/detect_path.rs`, add one `#[test]` per new fixture asserting
  the expected `app_type.primary`, headline framework (when applicable),
  and `languages.primary`.
- Add a regression test that asserts the existing `polyglot-monorepo`
  fixture still produces an ambiguous primary (no accidental dominance
  from the new Node library rule).

### 13. Update scan expectations

- In `scripts/detector-scan-expectations.json`, update `stack_expectations`:
  - `spm-library` → `{ "app_type": "library", "language": "swift" }`
    (was `{ "language": "swift" }`).
  - `obsidian-plugin` →
    `{ "app_type": "editor_extension", "language": "typescript" }`
    (was `{ "language": ["typescript", "javascript"] }`).
  - `wordpress-plugin` → `{ "app_type": "cms_plugin", "language": "php" }`
    (was `{ "language": "php" }`).
- Leave `node`, `python-pyproject`, `universal`, `unknown` entries
  unchanged. The test deliberately does not assert `app_type` for `node`
  /  `python-pyproject` because those stacks cover many shapes.
- Populate `project_overrides` with known-correct values that the
  versioning scan's stack rules misclassify:
  - `"app-type-detector": { "app_type": "library", "language": "rust" }` —
    the versioning config's `rust-cargo` rule checks for `Cargo.toml` at
    root, but this repo's workspace Cargo.toml is at `app/Cargo.toml`, so
    the scan labels it `unknown`. The detector itself will FAIL this row
    against the override until the workspace-in-subdir gap is addressed
    (out of scope; the FAIL is informative and intended).
- Populate `skip_projects` with `["dokploy_server",
  "seannam_cloudflare_pages"]`. Both are infra/deployment repos whose
  contents are intentionally out of scope for app-type detection.

### 14. Extend the test harness with an N/A status

- `scripts/detector-scan-test.py` currently buckets every non-failing row
  into `PASS` / `SKIP` / `-----` (no expectation). Add a new status `N/A`
  for projects that have no detectable code signal (empty repos, docs-only
  trees, paths that have been deleted since the scan JSON was generated).
- Signal: the detector's report has `scorecard.rules_fired == 0` AND
  `app_type.primary is None` AND `tech_stack.languages.primary is None`.
  That is exactly the "no code / only docs" state — the same shape the
  `empty-dir` and `git-only-dir` fixtures produce.
- Also: if the project's path does not exist on disk (stale scan JSON),
  treat the row as `N/A` rather than `FAIL`. A missing directory is
  literally "no code files present"; it is not a detector regression.
- Summary line should include the N/A count:
  `66 PASS · 3 FAIL · 2 SKIP · 25 N/A · 16 no-expectation (of 112 total)`.
- Exit semantics are unchanged: N/A rows do not cause a non-zero exit.
  Only `FAIL` rows do.

### 15. Run the scan test as the acceptance gate

- Run `scripts/detector-scan-test.py --jobs 8`.
- Required: `FAIL` count must not grow from its post-override baseline.
  The `app-type-detector` row is expected to remain a `FAIL` (workspace-
  in-subdir is tracked as a separate follow-up). Any *new* FAIL not in
  that known-gap list is a regression.
- Required: across the 112-project corpus, the count of rows with
  `status == PASS` goes from 4 to at least 6 (spm-library FAILs turn
  into PASSes once the new rule lands). Populated-`app_type` rate on
  the corpus rises above 60%. Both can be verified from the JSON
  produced by `--json-out /tmp/scan.json`.

### 16. Re-run full workspace gate

- `cd app && cargo fmt --all --check`
- `cd app && cargo clippy --workspace --all-targets -- -D warnings`
- `cd app && cargo test --workspace`
- `./scripts/test-all.sh`
- All must pass.

## Validation Commands

Execute every command to validate the feature lands with zero regressions.

- `cd /Users/seannam/Developer/app-type-detector/app && cargo fmt --all --check`
- `cd /Users/seannam/Developer/app-type-detector/app && cargo clippy --workspace --all-targets -- -D warnings`
- `cd /Users/seannam/Developer/app-type-detector/app && cargo test --workspace`
- `cd /Users/seannam/Developer/app-type-detector && ./scripts/test-all.sh`
- `scripts/detector-scan-test.py --jobs 8` — exit code must be `0`, `FAIL`
  count `0`, PASS count `>= 6`.
- Targeted post-fix reproductions (must all print non-null values):
  - `app/target/release/app-type-detector detect /Users/seannam/Developer/VersionGateKit --format json | jq '.app_type.primary, .tech_stack.languages.primary'` → `"library"`, `"swift"`.
  - `app/target/release/app-type-detector detect /Users/seannam/Developer/auto-build-log --format json | jq '.app_type.primary, .tech_stack.languages.primary'` → `"cli_tool"` or `"library"`, `"typescript"`.
- `app/target/release/app-type-detector detect app/crates/app-type-detector/tests/fixtures/empty-dir --format text` — must still say "no rules fired".

## Notes

- **No new dependencies.** Pure ruleset JSON, fixtures, tests, and two
  lines of JSON in the scan expectations file.
- **Keep baseline weights low.** The Python-library rule at 0.4, the
  HTML static-site rule at 0.3, and the Node-library rule at 0.5 are
  deliberately below the 1.0 framework tier. Do not raise them.
- **Out of scope — filed as follow-ups:**
  - *Templated `content` paths* for per-plugin PHP file inspection
    (would let us match the canonical "Plugin Name:" header without
    relying on `composer.json`). Tracked separately.
  - *Snapshot-level test-fixture ignore list*, still the root cause of
    the `app-type-detector` repo root misclassifying as `mobile_app`
    (spec 0001, Notes). Not addressed here.
  - *Content detection via file extension* for `.go`, `.rb`, `.scala`,
    `.jl`, etc. — add only when a concrete project surfaces the gap.
  - *AI agent SDKs* (langchain, openai, anthropic) as framework
    contributions. Out of scope: they classify as `app_type=daemon` /
    `cli_tool` today via the underlying framework, and a dedicated
    enum variant would widen blast radius.
- **`cms_plugin` / `editor_extension`** are already valid `AppType`
  values (`app_type.rs:18, 17`). No enum changes required.
