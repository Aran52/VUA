import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { isFaceTrackingFacts } from "./face-tracking-v01.js";
import { isDesktopGatewayRequestV1, DESKTOP_GATEWAY_METHOD_KINDS } from "./desktop-gateway.js";
import { isApplicationRequestV01 } from "./application-contract.js";

const facts = JSON.parse(readFileSync(new URL("../../../schemas/unity-bridge/v5/examples/face-tracking.valid.result.json", import.meta.url), "utf8")).data.faceTracking;
describe("face tracking evidence and action authority", () => {
  it("keeps geometry evidence separate from runtime and final budget", () => {
    expect(isFaceTrackingFacts(facts)).toBe(true);
    for (const change of [{ runtime: "working" }, { builtParameterBudget: "passed" }, { sourceSyncedParameterBits: -1 },
      { expressionParametersObserved: false }, { truncated: true }, { patchMenus: [{ ...facts.patchMenus[0], scriptPath: "Assets/../Original.cs" }] },
      { parameters: ["Face/v2/JawOpen", "Face/v2/JawOpen"] }]) {
      expect(isFaceTrackingFacts({ ...facts, ...change })).toBe(false);
    }
    expect(isFaceTrackingFacts({ ...facts, state: "partial", truncated: true })).toBe(true);
  });
  it("requires a command and an explicit confirmation bound to a concrete menu and review", () => {
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.faceTrackingPlan"]).toBe("query");
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.faceTrackingPatch"]).toBe("command");
    const params = { schemaVersion: "0.1", runId: "n3-test", menuId: "a".repeat(64), planDigest: "b".repeat(64), confirmTrustedCode: true };
    const request = { schemaVersion: 1, requestId: "ft", method: "amf.production.faceTrackingPatch", params };
    expect(isDesktopGatewayRequestV1(request)).toBe(true);
    for (const change of [{ confirmTrustedCode: false }, { confirmTrustedCode: undefined }, { planDigest: "" },
      { menuId: "Tools/Arbitrary" }, { executable: "patch.exe" }]) {
      expect(isDesktopGatewayRequestV1({ ...request, params: { ...params, ...change } })).toBe(false);
    }
    expect(isApplicationRequestV01({ schemaVersion: "0.1", requestId: "ft", kind: "query", method: request.method, params })).toBe(false);
  });
  it("accepts creator menu roots but excludes reserved roots and Unity shortcut spellings", () => {
    const withMenu = (menuPath: string) => ({ ...facts, patchMenus: [{ ...facts.patchMenus[0], menuPath }] });
    expect(isFaceTrackingFacts(withMenu("Creator FT/Apply"))).toBe(true);
    for (const path of ["Assets/Apply", "Window/Apply", "Tools//Apply", "Tools/Apply %g", "Tools/Apply _g", "Tools/"]) {
      expect(isFaceTrackingFacts(withMenu(path))).toBe(false);
    }
  });
});
