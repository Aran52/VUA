import { describe, expect, it } from "vitest";
import { defaultFitName, fitFailure, validFitName } from "./fits-model.ts";

describe("defaultFitName", () => {
  it("joins the garment and Avatar names without whitespace, like the engine", () => {
    expect(defaultFitName("Hair Clip", "Meiyun")).toBe("HairClipForMeiyun");
    expect(defaultFitName("TshirtShorts_Meiyun_v1.00.zip", "Meiyun_v2.1.0.zip")).toBe("TshirtShorts_Meiyun_v1.00.zipForMeiyun_v2.1.0.zip");
  });
  it("caps at 80 characters and the joining `For` always remains", () => {
    expect(defaultFitName("x".repeat(100), "y")).toHaveLength(80);
    expect(defaultFitName(" ", " ")).toBe("For");
  });
});

describe("validFitName", () => {
  it("matches the engine's rule: a 1-80 character line that names something", () => {
    expect(validFitName("HairClipForMeiyun")).toBe(true);
    expect(validFitName("")).toBe(false);
    expect(validFitName("   ")).toBe(false);
    expect(validFitName("x".repeat(81))).toBe(false);
    expect(validFitName("a\nb")).toBe(false);
  });
});

describe("fitFailure", () => {
  it("names the next step of each known refusal, by bare code or by the engine's sentence", () => {
    expect(fitFailure("vua.amf_production.fit_invalid")).toBe("invalid");
    expect(fitFailure("A fit name is a single line of 1-80 characters.")).toBe("invalid");
    expect(fitFailure("The reported adjustments changed. Review them again.")).toBe("drift");
    expect(fitFailure("This garment reports no adjustments to save. Check the line again.")).toBe("missing");
    expect(fitFailure("This garment's identity is not fully observed. Reinspect the line.")).toBe("missing");
    expect(fitFailure("The assembly baseline is missing. Assemble the line again, then save the fit.")).toBe("baseline");
    expect(fitFailure("At most 500 fits are kept. Delete one before saving another.")).toBe("limit");
    expect(fitFailure("There is no saved fit with this ID.")).toBe("gone");
  });
  it("leaves unknown messages to the engine's own words", () => {
    expect(fitFailure("A fit carries at most 2048 objects.")).toBeNull();
  });
});
