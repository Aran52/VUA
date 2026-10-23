import { describe, expect, it } from "vitest";
import { isAmfParameterBudgetPlan, type AmfParameterBudgetPlanV01 } from "./parameter-budget-v01.js";

const plan: AmfParameterBudgetPlanV01 = {
  schemaVersion: "0.1", evidence: "unity_source_estimate", limitBits: 256,
  state: "fits", estimateBits: 11, unresolvedPrefixConfigs: 1,
  sources: [
    { kind: "expression_parameters", origin: "avatar_base", name: "Avatar", bits: 9 },
    { kind: "ma_parameters", origin: "avatar_base", name: "Avatar", bits: 1 },
    { kind: "ma_parameters", origin: "selected_outfit", name: "Coat", bits: 1 },
  ],
};
describe("parameter budget estimate", () => {
  it("accepts a source-attributed estimate", () => {
    expect(isAmfParameterBudgetPlan(plan)).toBe(true);
    expect(isAmfParameterBudgetPlan({ ...plan, state: "over_limit", estimateBits: 264,
      sources: [{ kind: "expression_parameters", origin: "avatar_base", name: "Avatar", bits: 264 }] })).toBe(true);
  });
  it("keeps the limit, the attribution and unknown honest", () => {
    for (const change of [{ limitBits: 128 }, { estimateBits: -1 }, { state: "plenty" }, { evidence: "sdk_build" },
      { sources: [{ kind: "ma_parameters", origin: "selected_outfit", name: "Coat", bits: 5 }] },
      { sources: [...plan.sources, { kind: "ma_parameters", origin: "nowhere", name: "X", bits: 1 }] },
      { estimateBits: 12 }, { unresolvedPrefixConfigs: -1 }]) {
      expect(isAmfParameterBudgetPlan({ ...plan, ...change })).toBe(false);
    }
    expect(isAmfParameterBudgetPlan({ ...plan, estimateBits: 12, state: "near_limit" })).toBe(false);
  });
  it("reports unknown without a number or rows", () => {
    const unknown = { ...plan, state: "unknown", estimateBits: 0, unresolvedPrefixConfigs: 0, sources: [] };
    expect(isAmfParameterBudgetPlan(unknown)).toBe(true);
    expect(isAmfParameterBudgetPlan({ ...unknown, estimateBits: 11 })).toBe(false);
    expect(isAmfParameterBudgetPlan({ ...unknown, sources: plan.sources })).toBe(false);
  });
});
