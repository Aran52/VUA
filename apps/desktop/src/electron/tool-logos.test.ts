import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createToolLogos, isIcoFile, isToolLogoId, toolLogoUrl } from "./tool-logos.ts";

const ico = new Uint8Array([0, 0, 1, 0, 1, 0, 32, 32, 0, 0, 1, 0, 32, 0, 4, 0, 0, 0, 22, 0, 0, 0, 9, 9, 9, 9]);
const roots: string[] = [];
const root = async () => { const dir = await fs.mkdtemp(path.join(os.tmpdir(), "vua-tool-logos-")); roots.push(dir); return dir; };
afterEach(async () => { for (const dir of roots.splice(0)) await fs.rm(dir, { recursive: true, force: true }); });

describe("tool logos come from Steam and are cached in the profile", () => {
  it("accepts only the fixed tools and real icon files", () => {
    expect(isToolLogoId("oyasumi")).toBe(true);
    expect(isToolLogoId("vrcs")).toBe(false);
    expect(isToolLogoId("toString")).toBe(false);
    expect(isIcoFile(ico)).toBe(true);
    expect(isIcoFile(new TextEncoder().encode("<html>not found</html>"))).toBe(false);
    expect(toolLogoUrl("cdn.akamai.steamstatic.com", "liv")).toBe("https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/755540/1f014d873d7c47ee077c9be46ebbe3d7c30cfe7e.ico");
  });

  it("falls back to the second CDN front, caches the icon and reads the cache next time", async () => {
    const cacheRoot = await root();
    const fetch = vi.fn(async (url: string) => url.includes("cloudflare") ? new Response("down", { status: 503 }) : new Response(ico));
    const first = await createToolLogos({ cacheRoot, fetch })("oyasumi");
    expect(first).toBe(`data:image/x-icon;base64,${Buffer.from(ico).toString("base64")}`);
    expect(fetch).toHaveBeenCalledTimes(2);
    const offline = vi.fn(async () => { throw new Error("offline"); });
    expect(await createToolLogos({ cacheRoot, fetch: offline })("oyasumi")).toBe(first);
    expect(offline).not.toHaveBeenCalled();
  });

  it("returns null when no front serves an icon, and shares one request between callers", async () => {
    const cacheRoot = await root();
    const fetch = vi.fn(async () => new Response("<html></html>"));
    const logos = createToolLogos({ cacheRoot, fetch });
    const [a, b] = await Promise.all([logos("vrcft"), logos("vrcft")]);
    expect(a).toBeNull();
    expect(b).toBeNull();
    expect(fetch).toHaveBeenCalledTimes(2);
  });
});
