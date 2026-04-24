#!/usr/bin/env bash
# Run every Rust test in the workspace, plus the Node binding tests when
# pnpm/npm is available and the binding has been installed.
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR/app"

echo "==> cargo fmt --check"
cargo fmt --all -- --check

echo "==> cargo clippy -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo test"
cargo test --workspace --exclude app-type-detector-node

# Node binding: only run if a package manager is on PATH AND the binding
# has been installed locally (we don't auto-install from test-all to keep
# the Rust-only path fast).
if command -v pnpm >/dev/null 2>&1 || command -v npm >/dev/null 2>&1; then
  if [ -d "$ROOT_DIR/app/bindings/node/node_modules" ]; then
    echo "==> Node binding tests"
    bash "$ROOT_DIR/scripts/node-test.sh"
  else
    echo "==> Node binding tests: skipped (run 'bash scripts/node-test.sh' once to install, then rerun)"
  fi
fi
