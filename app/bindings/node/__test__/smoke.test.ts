import { describe, it, expect } from "vitest";
import { join } from "path";
import type { DetectionReport } from "../index";

// The real bindings are loaded at import time. Vitest runs in node so this
// triggers the same code path a consumer would.
// eslint-disable-next-line @typescript-eslint/no-var-requires
const binding = require("../index") as typeof import("../index");

const fixturesDir = join(
  __dirname,
  "..",
  "..",
  "..",
  "crates",
  "app-type-detector",
  "tests",
  "fixtures"
);

describe("smoke", () => {
  it("loads the native binding and exposes the four entrypoints", () => {
    expect(typeof binding.detectPath).toBe("function");
    expect(typeof binding.detectFiles).toBe("function");
    expect(typeof binding.defaultRuleset).toBe("function");
    expect(typeof binding.renderHumanReadable).toBe("function");
  });

  it("classifies the cli-rust fixture as a cli_tool", () => {
    const report = binding.detectPath(join(fixturesDir, "cli-rust")) as DetectionReport;
    expect(report.app_type.primary).toBe("cli_tool");
    expect(report.schema_version).toBe(1);
    expect(Array.isArray(report.scorecard.fires)).toBe(true);
  });

  it("returns a plain object for defaultRuleset()", () => {
    const ruleset = binding.defaultRuleset() as { schema_version: number; rules: unknown[] };
    expect(ruleset.schema_version).toBe(1);
    expect(Array.isArray(ruleset.rules)).toBe(true);
    expect(ruleset.rules.length).toBeGreaterThan(10);
  });

  it("throws a descriptive error when detectPath is called on a missing path", () => {
    const missing = join(fixturesDir, "__does_not_exist__");
    expect(() => binding.detectPath(missing)).toThrow(/detectPath|does not exist/);
  });

  it("rejects detectFiles input that is not an object", () => {
    // @ts-expect-error intentional bad input
    expect(() => binding.detectFiles({ files: "oops" })).toThrow(/files/);
  });
});
