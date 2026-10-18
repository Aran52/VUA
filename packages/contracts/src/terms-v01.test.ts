import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { isApplicationRequestV01 } from "./application-contract.js";
import { isDesktopGatewayRequestV1, DESKTOP_GATEWAY_METHOD_KINDS } from "./desktop-gateway.js";
import { isProductTermsV01, isTermsUseCheckV01, isTermsAcceptedV01, isTermsRefreshReportV01, TERMS_PRESET_IDS } from "./terms-v01.js";

describe("VN3 terms v0.1", () => {
  it("consumes the native closed request vectors at both Gateway boundaries", () => {
    const root = new URL("../../../schemas/terms/v0.1/requests/", import.meta.url);
    for (const name of readdirSync(root)) {
      const value = JSON.parse(readFileSync(new URL(name, root), "utf8"));
      const command = value.kind === "command";
      expect(isApplicationRequestV01({ contractVersion: "0.1", requestId: "terms", correlationId: "terms", ...value, ...(command ? { commandId: "terms-once" } : {}) }), name).toBe(name.endsWith(".valid.json"));
      if (name.endsWith(".valid.json")) expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: "terms", method: value.method, params: value.params }), name).toBe(true);
    }
    expect(DESKTOP_GATEWAY_METHOD_KINDS["terms.refreshProduct"]).toBe("command");
    expect(DESKTOP_GATEWAY_METHOD_KINDS["terms.checkUse"]).toBe("query");
    const definitions = JSON.parse(readFileSync(new URL("../../../crates/acquisition/data/vn3-presets-v1.json", import.meta.url), "utf8"));
    expect(definitions.presets.map((p: { id: string }) => p.id)).toEqual(TERMS_PRESET_IDS);
  });
  it("consumes result vectors and rejects conformity with unknown evidence or hidden notes", () => {
    const root = new URL("../../../schemas/terms/v0.1/results/", import.meta.url);
    for (const name of readdirSync(root)) {
      const value: unknown = JSON.parse(readFileSync(new URL(name, root), "utf8"));
      expect(isProductTermsV01(value) || isTermsUseCheckV01(value) || isTermsAcceptedV01(value) || isTermsRefreshReportV01(value), name).toBe(name.endsWith(".valid.json"));
    }
  });
});
