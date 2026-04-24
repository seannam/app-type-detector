import { describe, it, expect } from "vitest";
import type { DetectionReport } from "../../index";

// eslint-disable-next-line @typescript-eslint/no-var-requires
const binding = require("../../index") as typeof import("../../index");

describe("detectFiles (in-memory snapshot)", () => {
  it("classifies a hand-built rust CLI as a cli_tool", () => {
    const files: Record<string, string | null> = {
      "Cargo.toml": [
        "[package]",
        'name = "demo"',
        'version = "0.1.0"',
        'edition = "2021"',
        "",
        "[[bin]]",
        'name = "demo"',
        'path = "src/main.rs"',
      ].join("\n"),
      "src/main.rs": "fn main() { println!(\"hi\"); }",
    };

    const report = binding.detectFiles({ files }) as DetectionReport;
    expect(report.schema_version).toBe(1);
    expect(report.app_type.primary).toBe("cli_tool");
  });

  it("treats null values as empty files, not missing files", () => {
    const files: Record<string, string | null> = {
      "package.json": JSON.stringify({
        name: "x",
        version: "0.0.1",
        dependencies: { next: "14.0.0" },
      }),
      "pages/index.js": null,
    };
    const report = binding.detectFiles({ files }) as DetectionReport;
    expect(report.scorecard.input_summary.files_scanned).toBeGreaterThanOrEqual(2);
  });

  it("rejects a values map with non-string/non-null entries", () => {
    const bad = { files: { "Cargo.toml": 42 } } as unknown;
    // @ts-expect-error intentional bad input
    expect(() => binding.detectFiles(bad)).toThrow(/string or null/);
  });
});
