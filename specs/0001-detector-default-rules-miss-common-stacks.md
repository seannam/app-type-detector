# Bug: Default ruleset misses common stacks (Bun/Fastify/TS, Python bots, Rust workspaces)

## Bug Description

`detect` returns an empty report for several very common project shapes. The
renderer prints:

```
App Type
  unable to determine a single app type (no rule dominated)

Tech Stack

Scorecard (0/36 rules fired ...)
  ! no rules fired; input may be empty or out of vocabulary
```

Confirmed reproductions (all on ruleset v0.1.0 shipped with v0.2.0 of the
binary):

1. **Bun + Fastify + TypeScript web API** (`package.json` with `"fastify"`
   dependency, `tsconfig.json`, `src/server.ts`). **Zero rules fire.** No
   `tech_stack.languages`, no `tech_stack.web.backend_frameworks`, no
   `app_type.primary`.
2. **Python Telegram bot** (`pyproject.toml` and `requirements.txt` with
   `python-telegram-bot`, `src/bot.py`). **Zero rules fire.** The detector
   cannot see that this is a Python project.
3. **The `app-type-detector` repo root itself** (Rust workspace with two
   member crates, plus committed `tests/fixtures/*` directories). The
   workspace `Cargo.toml` has no `[lib]` or `[[bin]]` at the workspace root,
   so neither `rust-cargo-library` nor `rust-cargo-cli` fires. Only the
   `swiftui-ios-app` and `bevy-engine` rules fire (by matching files that
   live *inside* `tests/fixtures/*`), so the report misclassifies this as a
   `mobile_app`. The user reports this dir as "unable to determine" in other
   runs; both failure modes share the same root cause (ruleset gaps) even
   though the fixture-leakage produces a misleading positive.

Expected behavior: each of these stacks should produce a populated
`tech_stack` (languages, runtimes, frameworks) and a confident
`app_type.primary`. The user reports that only Swift iOS apps and Rust
libraries (`[lib]`) detect correctly today.

## Problem Statement

The default ruleset bundled in
`app/crates/app-type-detector/src/default_rules.json` has coverage gaps for
the most common web backend frameworks, generic Node/Python projects, Bun
runtimes, common Python bot frameworks, and Rust cargo workspaces. In
addition, it has no baseline "this file extension implies this language"
rules, so projects whose framework isn't on the hard-coded list produce a
completely empty `tech_stack` instead of at least reporting the language.

Because the engine, synthesizer, and renderer are all well-behaved and
already return empty state cleanly, **the fix is isolated to
`default_rules.json` plus regression fixtures.** No changes to the rule
grammar, engine, or synthesizer are required or desired.

## Solution Statement

Extend `default_rules.json` with:

1. **Framework-specific rules** for the stacks the user flagged:
   - Node.js backends: Fastify, Express, Hono, Koa (all map to
     `app_type=web_api`, `tech_stack.web.backend_frameworks`).
   - Python backends: Flask (`web_api`), Django (`web_app`).
   - Bun runtime detection (`bun.lockb` or `bun.lock`).
   - Common Python bot frameworks (`python-telegram-bot`, `discord.py`,
     `slack-bolt`) map to `app_type=daemon` (which is the closest existing
     enum value for a long-running bot process).
2. **Generic project baselines** that fire when manifests exist but no
   specific framework matched:
   - `node-project`: `package.json` exists (low weight, contributes
     `languages=javascript`/`typescript` depending on tsconfig, plus
     `runtimes=node`, `package_managers=npm`).
   - `python-project`: `pyproject.toml` or `requirements.txt` exists (low
     weight, contributes `languages=python`, `runtimes=python`).
   - `cargo-workspace`: `Cargo.toml` with `[workspace]` section (low
     weight, contributes `languages=rust`, `build_systems=cargo`,
     `package_managers=cargo`, plus `app_type=library` at low delta so
     workspace roots fall back to `library` when no member crate signals
     dominate).
3. **Baseline language-extension rules** so at least the language shows up
   even when no framework rule matches:
   - `tsconfig.json` or `**/*.ts` glob (min_count tuned to avoid single
     stray files): contributes `languages=typescript`.
   - `**/*.js` glob with min_count >= 3: contributes `languages=javascript`.
   - `**/*.py` glob with min_count >= 3: contributes `languages=python`.
   - `**/*.go` glob with min_count >= 3: contributes `languages=go`
     (`go-module` already exists but only fires when `go.mod` is present).

The new rules use low `confidence_weight` values (0.2 – 0.5) so they never
outweigh a proper framework-specific rule. Framework-specific rules keep the
weight pattern already used in the file (0.8 – 1.5).

## Steps to Reproduce

```sh
# 1. Bun/Fastify/TS web app
mkdir -p /tmp/bfs && cd /tmp/bfs
cat > package.json <<'EOF'
{"name":"my-api","dependencies":{"fastify":"^4.28.1"},"devDependencies":{"typescript":"^5.5.0","bun-types":"^1.1.0"}}
EOF
cat > tsconfig.json <<'EOF'
{"compilerOptions":{"target":"ES2022"}}
EOF
mkdir -p src && echo "import Fastify from 'fastify';" > src/server.ts

cd /Users/seannam/Developer/app-type-detector/app
cargo run -p app-type-detector-cli --quiet -- detect /tmp/bfs --format text
# Observed: "App Type: unable to determine a single app type (no rule dominated)"
#           "Scorecard (0/36 rules fired...)  ! no rules fired"

# 2. Python telegram bot
mkdir -p /tmp/pytg && cd /tmp/pytg
cat > pyproject.toml <<'EOF'
[project]
name = "mybot"
dependencies = ["python-telegram-bot>=21.0"]
EOF
mkdir -p src && echo "from telegram.ext import Application" > src/bot.py

cd /Users/seannam/Developer/app-type-detector/app
cargo run -p app-type-detector-cli --quiet -- detect /tmp/pytg --format text
# Observed: zero fires, empty tech_stack.

# 3. Repo root itself
cargo run -p app-type-detector-cli --quiet -- detect /Users/seannam/Developer/app-type-detector --format text
# Observed: misidentifies as mobile_app (swiftui-ios-app fixture leaks).
# Expected: some signal (at minimum languages=rust, build_systems=cargo).
```

## Root Cause Analysis

`app/crates/app-type-detector/src/default_rules.json` contains 36 rules. The
only Node.js backend rule is `nextjs-app`. The only Python backend rule is
`fastapi-api`. The Python CLI rule requires `[project.scripts]` to be
present. No rule matches Fastify, Express, Hono, Koa, Flask, Django, Bun,
or any bot framework. No rule establishes `tech_stack.languages` from file
extensions alone.

As a result, when a project's framework isn't in that curated list, the
engine produces zero fires. The synthesizer then executes the `[] => (None,
0.0, vec![])` branch in `synthesis.rs:91-93` and emits the "no rules fired"
warning. This is working as designed at the engine/synthesizer layer — the
defect is purely a content gap in `default_rules.json`.

The repo-root misclassification (#3) has a second, smaller contributing
factor: `FilesystemSnapshot` walks into `tests/fixtures/*` and the globs
`**/*.xcodeproj/project.pbxproj` + `**/*.swift` match fixture files. That
is a separate, larger scope issue (snapshot-level test-fixture ignore list)
that this plan explicitly does **not** fix — it is tracked as a follow-up
in "Notes". Adding a cargo-workspace rule gives this repo a legitimate
positive signal so the fixture leak no longer dominates the ranking.

No production logs needed: this is a library/CLI with no Dokploy service;
the bug is reproducible locally.

## Relevant Files

Use these files to fix the bug:

- `app/crates/app-type-detector/src/default_rules.json` — the authoritative
  bundled ruleset. All new rules are added here. This is the only source
  file that must change for the fix itself.
- `app/crates/app-type-detector/src/rules.rs` — declares the schema. Read
  only; no changes needed. Used to confirm new rules conform
  (`schema_version=1`, `kind: file_exists | glob | content | any | all |
  not`).
- `app/crates/app-type-detector/src/synthesis.rs` — confirms the field
  paths for new contributions
  (`tech_stack.web.backend_frameworks`, `tech_stack.runtimes`,
  `tech_stack.languages`, etc.) are routed through
  `set_list_field`/`set_scalar_field`. Read only.
- `app/crates/app-type-detector/src/types/app_type.rs` — lists legal
  `app_type` string values. Read only; `daemon` is already present and
  will be used for bots.
- `app/crates/app-type-detector/tests/detect_path.rs` — integration tests
  that assert against fixtures. New test cases added here.
- `app/crates/app-type-detector/tests/fixtures/` — new fixtures added for
  each new stack (Bun/Fastify, Python telegram bot, Rust workspace,
  Express, Flask, Django).
- `app/crates/app-type-detector/src/render.rs` — confirm it handles the
  new values (only reads from the typed report; no changes needed).

### New Files

- `app/crates/app-type-detector/tests/fixtures/bun-fastify-api/package.json`
- `app/crates/app-type-detector/tests/fixtures/bun-fastify-api/tsconfig.json`
- `app/crates/app-type-detector/tests/fixtures/bun-fastify-api/bun.lockb`
- `app/crates/app-type-detector/tests/fixtures/bun-fastify-api/src/server.ts`
- `app/crates/app-type-detector/tests/fixtures/express-api/package.json`
- `app/crates/app-type-detector/tests/fixtures/express-api/src/index.js`
- `app/crates/app-type-detector/tests/fixtures/flask-api/pyproject.toml`
- `app/crates/app-type-detector/tests/fixtures/flask-api/app.py`
- `app/crates/app-type-detector/tests/fixtures/django-app/requirements.txt`
- `app/crates/app-type-detector/tests/fixtures/django-app/manage.py`
- `app/crates/app-type-detector/tests/fixtures/python-telegram-bot/pyproject.toml`
- `app/crates/app-type-detector/tests/fixtures/python-telegram-bot/src/bot.py`
- `app/crates/app-type-detector/tests/fixtures/cargo-workspace/Cargo.toml`
- `app/crates/app-type-detector/tests/fixtures/cargo-workspace/crates/a/Cargo.toml`
- `app/crates/app-type-detector/tests/fixtures/cargo-workspace/crates/a/src/lib.rs`

## Step by Step Tasks

IMPORTANT: Execute every step in order, top to bottom.

### 1. Bump ruleset version

- In `app/crates/app-type-detector/src/default_rules.json`, change
  `"version": "0.1.0"` to `"version": "0.2.0"`. Every scorecard already
  carries the ruleset version, so bumping it makes the wider coverage
  visible in output and lets tests target it.

### 2. Add generic Node.js project baseline

- Add rule `node-project` (weight `0.3`) that fires on
  `file_exists package.json`. Contributions:
  `tech_stack.languages=javascript`, `tech_stack.runtimes=node`,
  `tech_stack.package_managers=npm`, `tech_stack.build_systems=npm`,
  `tech_stack.platforms=web`. No `app_type` contribution — framework
  rules decide that.

### 3. Add TypeScript language baseline

- Add rule `typescript-sources` (weight `0.3`) that fires on `any`:
  `file_exists tsconfig.json` OR glob `**/*.ts` with `min_count=3`.
  Contributions: `tech_stack.languages=typescript`.
- Add rule `javascript-sources` (weight `0.2`) that fires on glob
  `**/*.js` with `min_count=3`. Contributions:
  `tech_stack.languages=javascript`.

### 4. Add Fastify, Express, Hono, Koa rules

- Add `fastify-api` (weight `1.0`, mirrors `fastapi-api`): content
  `package.json` regex `"fastify"\s*:`. Contributions:
  `app_type=web_api` (delta 1.0), `tech_stack.web.backend_frameworks=fastify`,
  `tech_stack.languages=typescript`, `tech_stack.runtimes=node`,
  `tech_stack.platforms=web`, `tech_stack.build_systems=npm`,
  `tech_stack.package_managers=npm`.
- Add `express-api` (weight `1.0`): content `package.json` regex
  `"express"\s*:`. Same shape, `backend_frameworks=express`,
  `languages=javascript` (express is predominantly JS; typescript
  still gets contributed by `typescript-sources` when relevant).
- Add `hono-api` (weight `1.0`): content `package.json` regex
  `"hono"\s*:`. Same shape, `backend_frameworks=hono`,
  `languages=typescript`.
- Add `koa-api` (weight `1.0`): content `package.json` regex
  `"koa"\s*:`. Same shape, `backend_frameworks=koa`,
  `languages=javascript`.

### 5. Add Bun runtime detection

- Add `bun-runtime` (weight `0.4`) that fires on `any`:
  `file_exists bun.lockb` OR `file_exists bun.lock`. Contributions:
  `tech_stack.runtimes=bun`, `tech_stack.package_managers=bun`.

### 6. Add generic Python project baseline

- Add `python-project` (weight `0.3`) that fires on `any`:
  `file_exists pyproject.toml` OR `file_exists requirements.txt` OR
  `file_exists setup.py`. Contributions: `tech_stack.languages=python`,
  `tech_stack.runtimes=python`.

### 7. Add Flask and Django rules

- Add `flask-api` (weight `1.0`): any of content `pyproject.toml` regex
  `(?i)\bflask\b` OR content `requirements.txt` regex `(?i)^flask\b`
  (must be a distinct line/package, not `flask-login` etc — use
  word-boundary-aware regex). Contributions: `app_type=web_api`,
  `tech_stack.web.backend_frameworks=flask`,
  `tech_stack.languages=python`, `tech_stack.runtimes=python`,
  `tech_stack.platforms=web`.
- Add `django-app` (weight `1.0`): any of content regexes matching
  `django` in `pyproject.toml` or `requirements.txt`, OR
  `file_exists manage.py`. Contributions: `app_type=web_app`,
  `tech_stack.web.backend_frameworks=django`,
  `tech_stack.languages=python`, `tech_stack.runtimes=python`,
  `tech_stack.platforms=web`.

### 8. Add bot-framework rules (→ daemon)

- Add `python-telegram-bot` (weight `0.9`): any of content regexes
  matching `python-telegram-bot` in `pyproject.toml` or
  `requirements.txt`. Contributions: `app_type=daemon` (delta 0.9),
  `tech_stack.languages=python`, `tech_stack.runtimes=python`,
  and a new list field contribution `tech_stack.frameworks=python_telegram_bot`.
- Add `discord-py-bot` (weight `0.9`): similar, matches `discord\.py`
  or `\"discord.py\"`, contributions identical but framework
  `discord_py`.
- Add `slack-bolt-bot` (weight `0.8`): matches `slack[-_]bolt`.

### 9. Add Rust cargo-workspace rule

- Add `cargo-workspace` (weight `0.4`): all of
  `file_exists Cargo.toml` AND content `Cargo.toml` regex `\[workspace\]`.
  Contributions: `app_type=library` (delta 0.4 — low so a member
  crate's `[lib]` or `[[bin]]` still wins if scanned), plus
  `tech_stack.languages=rust`, `tech_stack.build_systems=cargo`,
  `tech_stack.package_managers=cargo`.

### 10. Create fixtures for each new rule

- Add directories under
  `app/crates/app-type-detector/tests/fixtures/` listed in "New Files"
  above. Each fixture contains the minimum files needed to fire the
  target rule (plus any baseline rule). Keep fixtures small (1 – 3
  files each).

### 11. Extend integration tests

- In `app/crates/app-type-detector/tests/detect_path.rs`, add one
  `#[test]` per new fixture, asserting:
  - `report.app_type.primary.as_deref() == Some("<expected>")`
  - The headline framework is present in the corresponding
    `tech_stack.*.backend_frameworks` (or `tech_stack.frameworks` for
    bots).
  - `report.tech_stack.languages.primary` is the expected language.
  - For the bun-fastify fixture: `report.tech_stack.runtimes`
    contains `"bun"` AND `"node"` (both fire; bun rule is additive).

### 12. Add regression test for empty-input behavior

- Keep the existing `empty_dir_fixture` and `git_only_dir_fixture`
  tests passing unchanged. This validates the new baseline rules
  don't fire on truly empty directories.

### 13. Add unit test covering the `no-framework-but-language-present`
  case

- Add a fixture `python-unknown-framework` that contains only
  `pyproject.toml` with a made-up dependency and a single `.py`
  file. Assert that `languages.primary == Some("python")` and that
  `app_type.primary` is `None` (because no `app_type` contribution
  fires). This documents the intended behavior of baseline rules:
  they populate `tech_stack` but never claim `app_type`.

### 14. Re-reproduce the three originally-failing cases

- Run the three reproductions from "Steps to Reproduce" again and
  confirm:
  - `/tmp/bfs`: `app_type.primary == Some("web_api")`,
    `tech_stack.web.backend_frameworks` contains `"fastify"`,
    `tech_stack.runtimes` contains `"bun"` and `"node"`,
    `tech_stack.languages.primary == Some("typescript")`.
  - `/tmp/pytg`: `app_type.primary == Some("daemon")`,
    `tech_stack.frameworks` contains `"python_telegram_bot"`,
    `tech_stack.languages.primary == Some("python")`.
  - `/Users/seannam/Developer/app-type-detector` (or `app/`
    subdir): `app_type.primary == Some("library")` (from the
    cargo-workspace rule), `tech_stack.languages.primary ==
    Some("rust")`. Note that the fixture-leak issue still causes
    the swiftui-ios-app rule to fire; acceptable because the new
    cargo-workspace signal outweighs it with alternatives visible.
    (Full fix is out of scope — see Notes.)

### 15. Run full validation

- Run every command in "Validation Commands" below. Every command
  must pass without errors.

## Validation Commands

Execute every command to validate the bug is fixed with zero regressions.

- `cd /Users/seannam/Developer/app-type-detector/app && cargo fmt --all --check` — formatting.
- `cd /Users/seannam/Developer/app-type-detector/app && cargo clippy --workspace --all-targets -- -D warnings` — linting.
- `cd /Users/seannam/Developer/app-type-detector/app && cargo test --workspace` — unit + integration tests (includes all new fixture-based tests added in steps 11 – 13).
- `cd /Users/seannam/Developer/app-type-detector && ./scripts/test-all.sh` (or `just test` if `just` is installed) — the combined gate used by CI.
- Bug reproduction, post-fix:
  - `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli --quiet -- detect /tmp/bfs --format json | jq '.app_type.primary, .tech_stack.web.backend_frameworks, .tech_stack.runtimes, .tech_stack.languages.primary'` — must print `"web_api"`, an array containing `"fastify"`, an array containing `"bun"` and `"node"`, and `"typescript"`.
  - `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli --quiet -- detect /tmp/pytg --format json | jq '.app_type.primary, .tech_stack.languages.primary'` — must print `"daemon"` and `"python"`.
  - `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli --quiet -- detect /Users/seannam/Developer/app-type-detector/app --format json | jq '.app_type.primary, .tech_stack.languages.primary, .tech_stack.build_systems'` — must print `"library"`, `"rust"`, and an array containing `"cargo"`.
  - `cd /Users/seannam/Developer/app-type-detector/app && cargo run -p app-type-detector-cli --quiet -- detect /Users/seannam/Developer/app-type-detector/app/crates/app-type-detector/tests/fixtures/empty-dir --format text` — must still say "no rules fired" (baseline rules must not fire on empty input).

## Notes

- **No new dependencies.** The fix is pure JSON (ruleset) + Rust test
  code. The existing `regex` + `globset` + `serde_json` crates already
  cover every new rule.
- **Keep baseline rules low-weight.** The synthesizer uses a `1.5x`
  dominance margin over the runner-up. Baseline rules intentionally use
  weights <= 0.5 so a framework-specific rule at weight 1.0 still
  dominates (1.0/0.3 ≈ 3.3x). Do not raise baseline weights without
  revisiting the dominance math.
- **Out of scope — filed as follow-up:** `FilesystemSnapshot` in
  `snapshot.rs:16-31` ignores build/dependency dirs but not committed
  test-fixture trees. On `app-type-detector`'s own repo root this
  causes `tests/fixtures/swiftui-ios-app/*` to match the
  `swiftui-ios-app` rule, producing a misleading `mobile_app` result.
  The cargo-workspace rule added here gives the repo a correct
  competing signal (at weight 0.4 vs swiftui at 1.0 the fixture still
  wins the tie, so the caller may need to run on `./app/crates/...`
  to avoid the leak). A proper fix requires adding a snapshot-level
  ignore pattern for common test-fixture paths; track that as a new
  spec. Do not extend this plan to cover it.
- **`app_type=daemon` for bots** is the closest fit in the existing
  `AppType` enum (`app_type.rs:15`). If we later add a dedicated `bot`
  variant, the telegram/discord/slack rules can be re-pointed with a
  single JSON edit. Do not add a new enum variant as part of this fix
  — it would widen the blast radius into bindings, docs, and the
  TypeScript/Python consumer code.
- **Regex caution on `flask` / `django`**: a naked `flask` substring
  in a lockfile or comment can over-fire. Use word-boundary or
  line-anchor regexes (e.g. `(?m)^flask\b` for `requirements.txt`,
  `"flask"\s*=\s*` for `pyproject.toml`) to keep false positives low.
  Confirm against the existing `fastapi-api` rule pattern for
  consistency.
