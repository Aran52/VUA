import { describe, expect, it } from "vitest";
import { isUnityLicenseFailure, productionErrorMessage } from "./unity-license-failure.ts";
describe("production license failures", () => {
  const code = "vua.amf_production.unity_license_unavailable";
  it("recognizes the provider code, including its recorded detail", () => {
    expect(isUnityLicenseFailure(code)).toBe(true);
    expect(isUnityLicenseFailure(`${code} — detail: Selected materials are kept.`)).toBe(true);
  });
  it("never mistakes a material name or ordinary detail for a license failure", () => {
    expect(isUnityLicenseFailure("vua.amf_production.prefab_missing — detail: license.prefab")).toBe(false);
    expect(isUnityLicenseFailure("No license.prefab found")).toBe(false);
    expect(isUnityLicenseFailure(null)).toBe(false);
  });
  it("preserves a structured refusal's code before a durable task exists", () => {
    const error = Object.assign(new Error("Selected materials are kept."), { code });
    expect(isUnityLicenseFailure(productionErrorMessage(error, "failed"))).toBe(true);
    expect(productionErrorMessage(new Error("license.prefab missing"), "failed")).toBe("license.prefab missing");
    expect(productionErrorMessage(null, "failed")).toBe("failed");
  });
});
