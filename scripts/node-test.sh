#!/usr/bin/env bash
# Run the @indiecraft/app-type-detector Node binding test suite.
#
# Prereqs: pnpm (preferred) or npm on PATH, plus a Rust toolchain so that
# `napi build` can link against the workspace's crates. Gracefully falls
# back to npm when pnpm is missing.

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BINDING_DIR="$ROOT_DIR/app/bindings/node"

if [ ! -d "$BINDING_DIR" ]; then
  echo "[node-test] missing $BINDING_DIR" >&2
  exit 2
fi

if command -v pnpm >/dev/null 2>&1; then
  PM=pnpm
elif command -v npm >/dev/null 2>&1; then
  PM=npm
else
  echo "[node-test] neither pnpm nor npm is on PATH; skipping" >&2
  exit 0
fi

echo "==> [node-test] installing dependencies with $PM"
if [ "$PM" = "pnpm" ]; then
  (cd "$BINDING_DIR" && pnpm install --frozen-lockfile 2>/dev/null || pnpm install)
else
  (cd "$BINDING_DIR" && npm install)
fi

echo "==> [node-test] building native binding (debug)"
(cd "$BINDING_DIR" && "$PM" run build:debug)

echo "==> [node-test] running vitest"
(cd "$BINDING_DIR" && "$PM" test)
