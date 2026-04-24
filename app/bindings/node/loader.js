// Pure resolver for the triple that matches a given
// platform/arch/libc combination. Split out from `index.js` so unit
// tests can exercise it without triggering native load.

"use strict";

const { existsSync, readFileSync } = require("fs");

function readGlibcAbi() {
  try {
    const { glibcVersionRuntime } = process.report.getReport().header;
    return typeof glibcVersionRuntime === "string";
  } catch (_) {
    return false;
  }
}

function isMusl() {
  if (process.platform !== "linux") {
    return false;
  }
  if (readGlibcAbi()) {
    return false;
  }
  try {
    const lddPath = "/usr/bin/ldd";
    if (existsSync(lddPath)) {
      const contents = readFileSync(lddPath, "utf8");
      if (contents.includes("musl")) {
        return true;
      }
    }
  } catch (_) {
    // fallthrough
  }
  return true;
}

function resolveTriple(platform, arch, muslProbe) {
  if (platform === "darwin") {
    if (arch === "arm64") return "darwin-arm64";
    if (arch === "x64") return "darwin-x64";
  }
  if (platform === "linux") {
    const musl = typeof muslProbe === "function" ? muslProbe() : muslProbe;
    if (arch === "x64") return musl ? "linux-x64-musl" : "linux-x64-gnu";
    if (arch === "arm64" && !musl) return "linux-arm64-gnu";
  }
  if (platform === "win32" && arch === "x64") {
    return "win32-x64-msvc";
  }
  return null;
}

module.exports = { resolveTriple, isMusl };
