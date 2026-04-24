// Loader stub for @indiecraft/app-type-detector.
//
// Resolves the correct per-triple prebuilt native subpackage at import
// time and re-exports its symbols. Pattern mirrors the @napi-rs/cli
// `create-npm-dirs` loader shipped by packages such as better-sqlite3
// and simple-git-hooks: no postinstall scripts, no binary downloads at
// install time, no child processes.

"use strict";

const { existsSync } = require("fs");
const { join } = require("path");
const { resolveTriple, isMusl } = require("./loader.js");

function loadBinding(options) {
  const opts = options || {};
  const platform = opts.platform || process.platform;
  const arch = opts.arch || process.arch;
  const muslProbe = opts.isMusl || isMusl;
  const requireFn = opts.require || require;

  const triple = resolveTriple(platform, arch, muslProbe);
  if (!triple) {
    throw new Error(
      "@indiecraft/app-type-detector: unsupported platform/arch combination " +
        `(${platform} ${arch}). Supported triples: linux-x64-gnu, ` +
        "linux-arm64-gnu, linux-x64-musl, darwin-x64, darwin-arm64, " +
        "win32-x64-msvc. File an issue at " +
        "https://github.com/snam/app-type-detector/issues."
    );
  }

  const pkg = "@indiecraft/app-type-detector-" + triple;
  // Local artifacts sit alongside per-triple package.json files at
  // npm/<triple>/app-type-detector.<triple>.node after `napi build`.
  const localPath = join(
    __dirname,
    "npm",
    triple,
    "app-type-detector." + triple + ".node"
  );
  const siblingPath = join(__dirname, "app-type-detector." + triple + ".node");

  // Order: local dev artifact (just-built) first, then the installed
  // optional dependency. Flipping this would make `pnpm build` in the
  // monorepo behave surprisingly.
  if (existsSync(siblingPath)) {
    return requireFn(siblingPath);
  }
  if (existsSync(localPath)) {
    return requireFn(localPath);
  }
  try {
    return requireFn(pkg);
  } catch (e) {
    const err = new Error(
      "@indiecraft/app-type-detector: failed to load native binding for " +
        triple +
        ". Make sure the matching optional dependency '" +
        pkg +
        "' is installed (check your npm/pnpm log for '--no-optional' or " +
        "'npm_config_optional=false'). Underlying error: " +
        (e && e.message ? e.message : String(e))
    );
    err.cause = e;
    throw err;
  }
}

module.exports = loadBinding();
module.exports.__internal = { resolveTriple, loadBinding, isMusl };
