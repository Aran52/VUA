import { describe, expect, it } from "vitest";
import type { FaceTrackingFactsV01 } from "@vua/contracts";
import { ftCapability, ftFailure } from "./face-tracking-model.ts";

const facts = (matched: Record<string, number>): FaceTrackingFactsV01 => ({
  schemaVersion: "0.1", evidence: "unity_source", observedAt: "2026-10-22T06:00:00Z", state: "observed",
  runtime: "unknown", builtParameterBudget: "unknown", truncated: false, manualReplayRequired: false,
  expressionParametersObserved: false, sourceSyncedParameterBits: 0,
  meshes: [{ renderer: "Body", blendShapeCount: 60, missingMesh: false, profiles: [
    { id: "arkit52-v1", referenceCount: 52, matched: Array.from({ length: matched["arkit52-v1"] ?? 0 }, (_, i) => `shape_${i}`) },
    { id: "unified-core12-v1", referenceCount: 12, matched: Array.from({ length: matched["unified-core12-v1"] ?? 0 }, (_, i) => `shape_u${i}`) },
    { id: "sranipal-core12-v1", referenceCount: 12, matched: [] },
  ] }],
  parameters: [], components: [], patchMenus: [],
});

describe("ftCapability", () => {
  it("reads a complete reference set on any mesh as full", () => {
    expect(ftCapability(facts({ "unified-core12-v1": 12 }))).toBe("full");
    expect(ftCapability(facts({ "arkit52-v1": 52 }))).toBe("full");
  });
  it("reads partial matches as part and silence as none", () => {
    expect(ftCapability(facts({ "arkit52-v1": 51 }))).toBe("part");
    expect(ftCapability(facts({}))).toBe("none");
  });
});

describe("ftFailure", () => {
  it("names the next step of each known refusal, by bare code or by the engine's sentence", () => {
    expect(ftFailure("vua.amf_production.project_open")).toBe("projectOpen");
    expect(ftFailure("Unity has this line's project open. Close that Unity window, then try again.")).toBe("projectOpen");
    expect(ftFailure("Select an unambiguous imported menu from the latest inspection.")).toBe("menu");
    expect(ftFailure("Review the current creator patch again.")).toBe("drift");
    expect(ftFailure("Inspect a completed line before adding face tracking.")).toBe("inspect");
    expect(ftFailure("vua.amf_production.inspect_required")).toBe("inspect");
  });
  it("leaves unknown messages to the engine's own words", () => {
    expect(ftFailure("The creator session did not record its manual replay boundary.")).toBeNull();
  });
});
