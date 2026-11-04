import { describe, it, expect } from "vitest";
import {
  REGIONAL_REFERENCES, REGIONAL_TIER_ABSOLUTE_MS, REGIONAL_TIER_RELATIVE,
  REGIONAL_UNSTEADY_ABSOLUTE_MS, REGIONAL_UNSTEADY_RELATIVE, referenceElapsed, regionalTierPhrase, regionalTiers,
} from "./regional-reference.ts";
import type { WebsiteObservation } from "@vua/contracts";

const reference = (id: string) => REGIONAL_REFERENCES.find(region => region.id === id)!;
const observation = (id: string, elapsedMs: number, p90Ms = elapsedMs): WebsiteObservation => {
  const target = reference(id);
  return {
    url: target.url,
    status: "http_error",
    elapsedMs,
    httpStatus: 404,
    regional: { medianMs: elapsedMs, p90Ms, samples: 9, connections: 3, viaProxy: false },
  };
};
const failed = (id: string): WebsiteObservation => {
  const target = reference(id);
  return { url: target.url, status: "timeout", elapsedMs: 6000, httpStatus: null };
};

describe("regional reference timing", () => {
  it("accepts the expected anonymous 404 response without claiming API success", () => {
    const target = REGIONAL_REFERENCES[0];
    expect(referenceElapsed(target, { url: target.url, status: "http_error", httpStatus: 404, elapsedMs: 210 })).toBe(210);
  });
  it("never turns timeout, redirect, rate limit or another destination into latency", () => {
    const target = REGIONAL_REFERENCES[0];
    for (const [status, httpStatus] of [["timeout", null], ["connection_failed", null], ["probe_error", null], ["redirected", 302], ["http_error", 429], ["http_error", 503]] as const) {
      expect(referenceElapsed(target, { url: target.url, status, httpStatus, elapsedMs: 6000 })).toBeNull();
    }
    expect(referenceElapsed(target, { url: REGIONAL_REFERENCES[1].url, status: "reachable", httpStatus: 200, elapsedMs: 100 } as WebsiteObservation)).toBeNull();
    expect(referenceElapsed(target)).toBeNull();
  });
});

describe("regional tiers", () => {
  it("orders regions by value and keeps exactly-15ms or exactly-10% neighbors in one tier", () => {
    expect(REGIONAL_TIER_ABSOLUTE_MS).toBe(15);
    expect(REGIONAL_TIER_RELATIVE).toBe(0.1);
    const tiers = regionalTiers([
      { reference: reference("jp"), observation: observation("jp", 100) },
      { reference: reference("usw"), observation: observation("usw", 115) },
      { reference: reference("eu") },
    ]);
    // 10% of 100 is 10 ms, so the wider boundary is the absolute 15 ms.
    expect(tiers.map(tier => [tier.id, tier.value, tier.tier])).toEqual([["jp", 100, 0], ["usw", 115, 0], ["eu", null, null]]);
    const split = regionalTiers([
      { reference: reference("jp"), observation: observation("jp", 100) },
      { reference: reference("usw"), observation: observation("usw", 116) },
    ]);
    expect(split.map(tier => tier.tier)).toEqual([0, 1]);
    const relative = regionalTiers([
      { reference: reference("jp"), observation: observation("jp", 200) },
      { reference: reference("usw"), observation: observation("usw", 220) },
      { reference: reference("eu"), observation: observation("eu", 221) },
    ]);
    // 10% of 200 is 20 ms, wider than the absolute 15 ms.
    expect(relative.map(tier => tier.tier)).toEqual([0, 0, 1]);
  });
  it("compares each tier member with the tier's first region, not its predecessor", () => {
    const tiers = regionalTiers([
      { reference: reference("usw"), observation: observation("usw", 300) },
      { reference: reference("use"), observation: observation("use", 330) },
      { reference: reference("jp"), observation: observation("jp", 45) },
    ]);
    // 10% of 45 is 4.5 ms, so the boundary is 15 ms: 300 starts a new tier.
    expect(tiers.map(tier => [tier.id, tier.tier])).toEqual([["jp", 0], ["usw", 1], ["use", 1]]);
  });
  it("marks a region unsteady only beyond max(20 ms, half the value)", () => {
    expect(REGIONAL_UNSTEADY_ABSOLUTE_MS).toBe(20);
    expect(REGIONAL_UNSTEADY_RELATIVE).toBe(0.5);
    const unsteadySpread = (id: string, elapsedMs: number, p90Ms: number) =>
      regionalTiers([{ reference: reference(id), observation: observation(id, elapsedMs, p90Ms) }])[0]!;
    expect(unsteadySpread("jp", 30, 50).unsteady).toBe(false);
    expect(unsteadySpread("jp", 30, 51).unsteady).toBe(true);
    expect(unsteadySpread("eu", 100, 120).unsteady).toBe(false);
    expect(unsteadySpread("eu", 100, 151).unsteady).toBe(true);
  });
  it("sorts regions without a successful value last, without a tier", () => {
    const tiers = regionalTiers([
      { reference: reference("eu"), observation: observation("eu", 240) },
      { reference: reference("jp"), observation: observation("jp", 20) },
      { reference: reference("usw"), observation: failed("usw") },
      { reference: reference("use") },
    ]);
    expect(tiers.map(tier => [tier.id, tier.value, tier.tier, tier.unsteady]))
      .toEqual([["jp", 20, 0, false], ["eu", 240, 1, false], ["usw", null, null, false], ["use", null, null, false]]);
  });
});

describe("regional tier phrases", () => {
  it("reads the first region as closest, tier mates as about the same, later tiers by number", () => {
    const tiers = regionalTiers([
      { reference: reference("eu"), observation: observation("eu", 240) },
      { reference: reference("jp"), observation: observation("jp", 20) },
      { reference: reference("usw"), observation: observation("usw", 30) },
      { reference: reference("use"), observation: observation("use", 120) },
    ]);
    expect(tiers.map((_, index) => regionalTierPhrase(tiers, index))).toEqual(["closest", "about_same", 2, 3]);
  });
  it("gives unmeasured regions no phrase, including a first row when nothing measured", () => {
    const tiers = regionalTiers([
      { reference: reference("usw"), observation: failed("usw") },
      { reference: reference("jp") },
    ]);
    expect(tiers.map((_, index) => regionalTierPhrase(tiers, index))).toEqual([null, null]);
    expect(regionalTierPhrase(tiers, 4)).toBeNull();
    const partial = regionalTiers([
      { reference: reference("jp"), observation: observation("jp", 45) },
      { reference: reference("eu") },
    ]);
    expect(partial.map((_, index) => regionalTierPhrase(partial, index))).toEqual(["closest", null]);
  });
  it("keeps the phrase stable under the 15 ms boundary data of the tier tests", () => {
    const shared = regionalTiers([
      { reference: reference("jp"), observation: observation("jp", 100) },
      { reference: reference("usw"), observation: observation("usw", 115) },
      { reference: reference("eu") },
    ]);
    expect(shared.map((_, index) => regionalTierPhrase(shared, index))).toEqual(["closest", "about_same", null]);
    const split = regionalTiers([
      { reference: reference("jp"), observation: observation("jp", 100) },
      { reference: reference("usw"), observation: observation("usw", 116) },
    ]);
    expect(split.map((_, index) => regionalTierPhrase(split, index))).toEqual(["closest", 2]);
  });
});
