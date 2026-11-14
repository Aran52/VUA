import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { isApplicationRequestV01 } from "./application-contract.js";
import { isDesktopGatewayRequestV1 } from "./desktop-gateway.js";
import {
  PHR_TERMS_PRESET_IDS,
  TERMS_PRESET_IDS_V02,
  isPhrTermsPresetId,
  isTermsUseCheckParamsV02,
  isTermsUseCheckV02,
} from "./terms-v02.js";

describe("VN3 terms v0.2 (cross-platform & PH-R presets)", () => {
  const vectors = JSON.parse(
    readFileSync(new URL("../../../schemas/terms/v0.2/vectors.json", import.meta.url), "utf8"),
  ) as {
    validRequests: unknown[];
    invalidRequests: unknown[];
    validResults: unknown[];
    invalidResults: unknown[];
  };

  it("aligns preset IDs with vn3-presets-v2.json and validates v0.2 checkUse requests without widening v0.1 methods", () => {
    const definitions = JSON.parse(
      readFileSync(new URL("../../../crates/acquisition/data/vn3-presets-v2.json", import.meta.url), "utf8"),
    ) as { presets: { id: string }[] };
    expect(definitions.presets.map(p => p.id)).toEqual(TERMS_PRESET_IDS_V02);
    for (const id of PHR_TERMS_PRESET_IDS) {
      expect(isPhrTermsPresetId(id)).toBe(true);
    }
    expect(isPhrTermsPresetId("vrchat_personal")).toBe(false);

    for (const params of vectors.validRequests) {
      expect(isTermsUseCheckParamsV02(params)).toBe(true);
      expect(isApplicationRequestV01({
        contractVersion: "0.1",
        requestId: "terms-v2",
        correlationId: "terms-v2",
        kind: "query",
        method: "terms.checkUse",
        params,
      })).toBe(true);
      expect(isDesktopGatewayRequestV1({
        schemaVersion: 1,
        requestId: "terms-v2",
        method: "terms.checkUse",
        params,
      })).toBe(true);
    }

    for (const params of vectors.invalidRequests) {
      expect(isTermsUseCheckParamsV02(params)).toBe(false);
    }

    // Ensure v0.1-only methods still reject schemaVersion "0.2"
    expect(isApplicationRequestV01({
      contractVersion: "0.1",
      requestId: "terms-v2",
      correlationId: "terms-v2",
      kind: "query",
      method: "terms.getProduct",
      params: { schemaVersion: "0.2", productId: "booth:1" },
    })).toBe(false);
  });

  it("validates v0.2 checkUse results including combination platformRestrictionHint aggregation", () => {
    for (const result of vectors.validResults) {
      expect(isTermsUseCheckV02(result)).toBe(true);
    }
    for (const result of vectors.invalidResults) {
      expect(isTermsUseCheckV02(result)).toBe(false);
    }
  });
});
