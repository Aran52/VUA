import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

// The development build compiles the preload with tsc, unbundled. A sandboxed preload can require
// only Electron, so any other runtime import leaves window.vua undefined for the whole renderer.
describe("sandboxed preload", () => {
  it("imports only Electron at runtime", () => {
    const source = readFileSync(new URL("./preload.ts", import.meta.url), "utf8");
    const runtime = [...source.matchAll(/^import\s+(?!type\b)[^;]*?from\s+["']([^"']+)["']/gmu)].map(match => match[1]);
    expect(runtime).toEqual(["electron"]);
    expect(source).not.toMatch(/\brequire\(/u);
  });
});
