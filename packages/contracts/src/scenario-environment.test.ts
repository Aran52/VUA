import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import {
  isPlayScenarioSopV02,
  isPlaySetupCatalogV02,
  isScenarioEnvironmentCheck,
  isScenarioSop,
} from "./scenario-environment.js";
import { isDesktopGatewayRequestV1 } from "./desktop-gateway.js";
import { isApplicationRequestV01 } from "./application-contract.js";
const root = new URL("../../../schemas/scenario-environment/v0.1/", import.meta.url);
const rootV02 = new URL("../../../schemas/scenario-environment/v0.2/", import.meta.url);
const read = (name: string) => JSON.parse(readFileSync(new URL(name, root), "utf8"));
const readV02 = (name: string) => JSON.parse(readFileSync(new URL(name, rootV02), "utf8"));
describe("independent scenario Candidate boundaries", () => {
  it("consumes all native v0.1 and v0.2 request vectors through both gateways", () => {
    for (const [dir, load] of [[root, read], [rootV02, readV02]] as const) {
      for (const name of readdirSync(new URL("examples/", dir))) {
        const value = load(`examples/${name}`);
        const expected = name.includes(".valid.");
        expect(isApplicationRequestV01(value), name).toBe(expected);
        if (value.kind === "query") {
          expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: value.requestId, method: value.method, params: value.params }), name).toBe(expected);
        }
      }
    }
  });
  it("accepts honest software facts and rejects license/installation claims", () => {
    for (const name of readdirSync(new URL("results/", root))) expect(isScenarioEnvironmentCheck(read(`results/${name}`)), name).toBe(name.includes(".valid."));
    const value = read("results/check-ready.valid.json");
    expect(isScenarioEnvironmentCheck({ ...value, observations: [] })).toBe(false);
    expect(isScenarioEnvironmentCheck({ ...value, observations: [{ ...value.observations[0], presence: "missing" }] })).toBe(false);
    expect(isScenarioEnvironmentCheck({ ...value, nextAction: "plan_installation" })).toBe(false);
  });
  it("does not equate a static SOP with observed readiness", () => {
    const value = {
      schemaVersion: "vua.scenario-environment/v0.1",
      scenario: "desktop_play",
      connection: null,
      requirements: ["steam", "vrchat"],
      performed: "sop_only",
      nextAction: "check_environment",
      licenseRequiredFor: "none",
      steps: ["check", "install", "launch"].map(id => ({ id, title: "step", explanation: "explanation", expectedResult: "observed result", automatic: true })),
    };
    expect(isScenarioSop(value)).toBe(true);
    expect(isScenarioSop({ ...value, softwareReady: true })).toBe(false);
    expect(isScenarioSop({ ...value, licenseRequiredFor: "editor_execution" })).toBe(false);
    expect(isScenarioSop({ ...value, steps: [value.steps[0], value.steps[0], value.steps[1]] })).toBe(false);
  });
  it("validates v0.2 play setup catalog and composed SOP results strictly", () => {
    for (const name of readdirSync(new URL("results/", rootV02))) {
      const value = readV02(`results/${name}`);
      const expected = name.includes(".valid.");
      const actual = isPlaySetupCatalogV02(value) || isPlayScenarioSopV02(value);
      expect(actual, name).toBe(expected);
    }
    const sop = readV02("results/sop-vrchat-pc.valid.json");
    expect(isPlayScenarioSopV02({ ...sop, steps: [sop.steps[0], sop.steps[0]] })).toBe(false);
    expect(isPlayScenarioSopV02({ ...sop, engine: { kind: "tool", toolId: "vrchat" } })).toBe(false);
  });
});
