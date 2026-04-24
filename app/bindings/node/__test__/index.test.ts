import { describe, it, expect, beforeAll } from "vitest";
import { execFileSync } from "child_process";
import { readdirSync, statSync, existsSync } from "fs";
import { join } from "path";
import type { DetectionReport } from "../index";

// eslint-disable-next-line @typescript-eslint/no-var-requires
const binding = require("../index") as typeof import("../index");

const repoRoot = join(__dirname, "..", "..", "..", "..");
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
const cargoManifest = join(__dirname, "..", "..", "..", "Cargo.toml");

function listFixtures(): string[] {
  return readdirSync(fixturesDir)
    .filter((entry) => {
      const p = join(fixturesDir, entry);
      try {
        return statSync(p).isDirectory();
      } catch {
        return false;
      }
    })
    .sort();
}

function cliDetect(path: string): DetectionReport {
  const out = execFileSync(
    "cargo",
    [
      "run",
      "--quiet",
      "--manifest-path",
      cargoManifest,
      "-p",
      "app-type-detector-cli",
      "--",
      "detect",
      path,
      "--format",
      "json",
    ],
    { encoding: "utf8", maxBuffer: 32 * 1024 * 1024, cwd: repoRoot }
  );
  return JSON.parse(out) as DetectionReport;
}

// The CLI reports a per-run elapsed_ms; this is the one field we expect to
// drift between the CLI process and the in-process Node call. Zero it out
// on both sides before deep-equaling.
function normalize(report: DetectionReport): DetectionReport {
  const copy = JSON.parse(JSON.stringify(report)) as DetectionReport;
  copy.scorecard.elapsed_ms = 0;
  return copy;
}

describe("fixture parity (Node binding vs CLI)", () => {
  const fixtures = listFixtures();

  // Warm the CLI binary once to keep each fixture check fast.
  beforeAll(() => {
    execFileSync(
      "cargo",
      [
        "build",
        "--quiet",
        "--manifest-path",
        cargoManifest,
        "-p",
        "app-type-detector-cli",
      ],
      { stdio: "inherit", cwd: repoRoot }
    );
  }, 180_000);

  for (const name of fixtures) {
    it(`round-trips fixture: ${name}`, () => {
      const fixturePath = join(fixturesDir, name);
      if (!existsSync(fixturePath)) return;

      const fromCli = normalize(cliDetect(fixturePath));
      const fromNode = normalize(binding.detectPath(fixturePath) as DetectionReport);

      expect(fromNode).toEqual(fromCli);
    });
  }
});

describe("renderHumanReadable", () => {
  it("produces a non-empty string for the unity-game fixture", () => {
    const report = binding.detectPath(join(fixturesDir, "unity-game")) as DetectionReport;
    const text = binding.renderHumanReadable(report);
    expect(typeof text).toBe("string");
    expect(text.length).toBeGreaterThan(0);
    expect(text).toContain("App Type");
    expect(text).toContain("Tech Stack");
  });

  it("throws a napi error for a malformed report", () => {
    // @ts-expect-error intentional bad input
    expect(() => binding.renderHumanReadable({ not: "a report" })).toThrow(
      /invalid DetectionReport/
    );
  });
});
