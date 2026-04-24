import { describe, it, expect } from "vitest";

// eslint-disable-next-line @typescript-eslint/no-var-requires
const { resolveTriple } = require("../loader.js") as {
  resolveTriple: (
    platform: string,
    arch: string,
    isMusl: boolean | (() => boolean)
  ) => string | null;
};

describe("resolveTriple", () => {
  const cases: Array<{ platform: string; arch: string; musl: boolean; triple: string | null }> = [
    { platform: "darwin", arch: "arm64", musl: false, triple: "darwin-arm64" },
    { platform: "darwin", arch: "x64", musl: false, triple: "darwin-x64" },
    { platform: "linux", arch: "x64", musl: false, triple: "linux-x64-gnu" },
    { platform: "linux", arch: "x64", musl: true, triple: "linux-x64-musl" },
    { platform: "linux", arch: "arm64", musl: false, triple: "linux-arm64-gnu" },
    { platform: "linux", arch: "arm64", musl: true, triple: null },
    { platform: "win32", arch: "x64", musl: false, triple: "win32-x64-msvc" },
    { platform: "win32", arch: "arm64", musl: false, triple: null },
    { platform: "freebsd", arch: "x64", musl: false, triple: null },
    { platform: "linux", arch: "s390x", musl: false, triple: null },
  ];

  for (const c of cases) {
    it(`${c.platform}/${c.arch} musl=${c.musl} -> ${String(c.triple)}`, () => {
      expect(resolveTriple(c.platform, c.arch, c.musl)).toBe(c.triple);
    });
  }

  it("invokes the musl probe lazily when it is a function", () => {
    let calls = 0;
    const probe = () => {
      calls += 1;
      return true;
    };
    expect(resolveTriple("linux", "x64", probe)).toBe("linux-x64-musl");
    expect(calls).toBe(1);

    // darwin never needs the probe, so the function must not be called
    resolveTriple("darwin", "arm64", probe);
    expect(calls).toBe(1);
  });
});
