_default:
    @just --list

# Detect the app type of a directory (defaults to the current working directory).
# Usage: just detect               # scans $PWD
#        just detect ./some-repo   # scans the given path
#        just detect ~/code/my-app text
#        just detect . json
detect path='.' format='text':
    cargo run --quiet --manifest-path app/Cargo.toml -p app-type-detector-cli -- \
        detect {{ absolute_path(path) }} --format {{format}}

# Same as `detect` but with a release build (faster on large repos).
detect-release path='.' format='text':
    cargo run --release --quiet --manifest-path app/Cargo.toml -p app-type-detector-cli -- \
        detect {{ absolute_path(path) }} --format {{format}}

# Run fmt + clippy + tests.
test:
    bash scripts/test-all.sh

# Build the Node binding (debug build, loads via app/bindings/node/index.js).
node-build:
    cd app/bindings/node && (command -v pnpm >/dev/null && pnpm run build:debug || npm run build:debug)

# Run the Node binding test suite (installs deps on first use).
node-test:
    bash scripts/node-test.sh

# Pack the Node binding tarball (inspect with `tar -tzf`).
node-pack:
    cd app/bindings/node && (command -v pnpm >/dev/null && pnpm pack || npm pack)
