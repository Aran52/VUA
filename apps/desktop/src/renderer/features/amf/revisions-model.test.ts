import { describe, expect, it } from "vitest";
import type { AmfProductionRevisionSummaryItem } from "../../gateway/index.ts";
import { revisionFailure, revisionNames } from "./revisions-model.ts";

const revision = (summary: Partial<AmfProductionRevisionSummaryItem["summary"]>): AmfProductionRevisionSummaryItem => ({
  id: "rev-1", before: "assembly", takenAt: "2026-10-22T00:00:00Z", editedInUnity: false,
  summary: { avatar: null, outfits: [], sources: [], lightingMaterials: 0, sdkFixes: [], ...summary },
});

describe("revisionFailure", () => {
  it("names the next step of each known refusal, by bare code or by the engine's sentence", () => {
    expect(revisionFailure("vua.amf_production.revision_source_missing")).toBe("sourceMissing");
    expect(revisionFailure("This version's original packages are unavailable: Coat.unitypackage is no longer in the library. Supply these exact original packages in the library, then retry.")).toBe("sourceMissing");
    expect(revisionFailure("vua.amf_production.face_tracking_manual_replay")).toBe("faceTracking");
    expect(revisionFailure("This version contains an opaque creator patch and requires manual replay.")).toBe("faceTracking");
    expect(revisionFailure("The project or version record changed. Review the restore again.")).toBe("drift");
    expect(revisionFailure("The project changed before restoration. Review it again.")).toBe("drift");
    expect(revisionFailure("vua.amf_production.plan_drift")).toBe("drift");
    expect(revisionFailure("Reinspect interrupted work before reviewing a restore.")).toBe("inspect");
    expect(revisionFailure("inspect_required")).toBe("inspect");
    expect(revisionFailure("This version record is no longer retained. Review the line's current versions.")).toBe("gone");
  });
  it("leaves unknown messages to the engine's own words", () => {
    expect(revisionFailure("Unity closed unexpectedly")).toBeNull();
  });
});

describe("revisionNames", () => {
  it("names the Avatar with its outfits", () => {
    expect(revisionNames(revision({ avatar: "Meiyu", outfits: ["Tshirt", "Pins"] }))).toBe("Meiyu · Tshirt · Pins");
  });
  it("falls back to the source packages of a version taken before the first assembly", () => {
    expect(revisionNames(revision({ sources: ["A.zip", "B.zip"] }))).toBe("A.zip · B.zip");
  });
});
