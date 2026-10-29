import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { consumeLicenseStep, requestLicenseStep } from "./license-step-navigation.ts";
import { requestProductionRun, takeRequestedProductionRun } from "../amf/production-recipe-navigation.ts";

const key = "vua-unity-license-step-request";
const values = new Map<string, string>();
beforeEach(() => {
  values.clear();
  vi.stubGlobal("sessionStorage", { getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value), removeItem: (key: string) => values.delete(key) });
});
afterEach(() => { vi.unstubAllGlobals(); takeRequestedProductionRun(); });
describe("license-step view navigation", () => {
  it("retains the same run for the workshop and consumes the view hint once", () => {
    requestLicenseStep({ runId: "line-123" });
    const request = consumeLicenseStep();
    expect(request).toEqual({ runId: "line-123" });
    expect(consumeLicenseStep()).toBeNull();
    requestProductionRun(request!.runId!);
    expect(takeRequestedProductionRun()).toBe("line-123");
    expect(takeRequestedProductionRun()).toBeNull();
  });
  it("can open the scenario without a production run", () => {
    requestLicenseStep(); expect(consumeLicenseStep()).toEqual({}); expect(consumeLicenseStep()).toBeNull();
  });
  it.each(["", "../line", "line\n", "a".repeat(101), 1, null])("discards invalid run ID %j", runId => {
    values.set(key, JSON.stringify({ runId })); expect(consumeLicenseStep()).toBeNull(); expect(values.has(key)).toBe(false);
  });
  it("refuses corrupted or command-bearing hints and unavailable storage", () => {
    for (const raw of ["broken", "[]", JSON.stringify({ runId: "line-1", action: "retry" })]) {
      values.set(key, raw); expect(consumeLicenseStep()).toBeNull();
    }
    vi.stubGlobal("sessionStorage", { getItem: () => { throw new Error("unavailable"); }, setItem: () => { throw new Error("unavailable"); } });
    expect(() => requestLicenseStep()).not.toThrow(); expect(consumeLicenseStep()).toBeNull();
  });
});
