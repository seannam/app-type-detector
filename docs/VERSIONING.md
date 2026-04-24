# Versioning

Preset: `rust-cargo` (mode: `auto`). Source of truth: git tag `vX.Y.Z`.
Current: `0.1.0`. Primary manifest: `app/Cargo.toml`. Changelog: `CHANGELOG.md`.

## Bumping (mode: auto)

This repo auto-releases on push to `main`. Conventional commits drive the bump:

- `feat:` -> minor bump
- `fix:`, `perf:` -> patch bump
- `feat!:` / `BREAKING CHANGE:` -> major bump
- `chore:`, `docs:`, `test:`, etc. -> no release

The workflow lives at `.github/workflows/auto-release-on-push.yml`. It is
intentionally small: no builds, no tests, runs in ~10-20 seconds.

To force a local bump (emergency): `scripts/version/bump.sh --force`.

## Files kept in sync on every bump (app_root=app)

- `app/Cargo.toml` (toml: package.version)
- `VERSION` (plain)

## UI integration

Rust has the `env!("CARGO_PKG_VERSION")` macro which reads Cargo.toml at compile time. This is the idiomatic choice for binaries, daemons, and menu bar apps. For CLIs, clap exposes it automatically via #[command(version)].

### axum-endpoint

```
async fn version_handler() -> &'static str { env!("CARGO_PKG_VERSION") }
```

### clap

```
#[derive(Parser)]
#[command(version, about)]
struct Cli { /* ... */ }
```

### const

```
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
```

### daemon-log

```
tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting daemon");
```


