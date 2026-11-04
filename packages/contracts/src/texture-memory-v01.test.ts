import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { isTextureMemoryFacts, textureRank } from "./texture-memory-v01.js";
import { isDesktopGatewayRequestV1, DESKTOP_GATEWAY_METHOD_KINDS } from "./desktop-gateway.js";
const facts = () => JSON.parse(readFileSync(new URL("../../../schemas/unity-bridge/v5/examples/texture-memory.valid.result.json", import.meta.url), "utf8")).data.textureMemory;
describe("texture memory contracts", () => {
  it("preserves both platform inventories and refuses malformed or misleading ranks", () => {
    expect(isTextureMemoryFacts(facts())).toBe(true);
    expect(isTextureMemoryFacts([])).toBe(true);
    for (const bad of [null, {}, [facts()[0]], [{ ...facts()[0], totalBytes: 1 }, facts()[1]],
      [{ ...facts()[0], rank: "excellent" }, facts()[1]], [{ ...facts()[0], textures: [{...facts()[0].textures[0], width: -1}] }, facts()[1]]]) {
      expect(isTextureMemoryFacts(bad)).toBe(false);
    }
    expect(textureRank(40 * 1024 * 1024, "pc")).toBe("excellent");
    expect(textureRank(40 * 1024 * 1024 + 1, "pc")).toBe("good");
    expect(textureRank(18 * 1024 * 1024, "android")).toBe("good");
  });
  it("requires a digest and explicit reviewed paths for mutation", () => {
    const params = { schemaVersion: "0.1", runId: "n3-test", platform: "pc", targetRank: "good", planDigest: "a".repeat(64), textures: ["Assets/Color.png"] };
    const request = (p: unknown) => ({ schemaVersion: 1, requestId: "textures", method: "amf.production.applyTextureMemory", params: p });
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.planTextureMemory"]).toBe("query");
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.applyTextureMemory"]).toBe("command");
    expect(isDesktopGatewayRequestV1(request(params))).toBe(true);
    for (const bad of [{...params, planDigest: "stale"}, {...params, textures: []}, {...params, textures: ["Assets/../library.png"]},
      {...params, textures: ["Assets/Color.png", "Assets/Color.png"]}, {...params, targetRank: "unknown"}, {...params, platform: "ios"}, {...params, extra: true}]) {
      expect(isDesktopGatewayRequestV1(request(bad))).toBe(false);
    }
  });
});
