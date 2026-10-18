import { describe, expect, it } from "vitest";
import type { AmfSourceChoice } from "@vua/contracts";
import { avatarFitNames, buildStartChoices, groupOutfitsByFit, normalizeFitText, outfitFitsAvatar } from "./materials-fit-model.ts";

/** The engine's acceptance rule for preparation choices (orchestrator `validate_choices`): at most
 * one Avatar; without automatic materials, exactly one Avatar and at least one outfit. */
function validateStartChoices(choices: readonly AmfSourceChoice[]): boolean {
  const avatars = choices.filter(choice => choice.role === "avatar").length;
  const auto = choices.some(choice => choice.role === "auto");
  const outfits = choices.some(choice => choice.role === "outfit");
  return choices.length > 0 && choices.length <= 64 && avatars <= 1 && (auto || (avatars === 1 && outfits))
    && new Set(choices.map(choice => choice.sourceId)).size === choices.length;
}

describe("fit comparison form", () => {
  it("folds case, full-width characters and spaces", () => {
    expect(normalizeFitText("Ｍｅｉｙｕｎ ＶＲ")).toBe("meiyunvr");
    expect(normalizeFitText("Mei yun")).toBe(normalizeFitText("ｍｅｉｙｕｎ"));
  });
});

describe("avatar fit names", () => {
  it("names the Avatar by its packages without version and extension", () => {
    expect(avatarFitNames({ productName: "Meiyun 素体", packageNames: ["Meiyun_v2.1.0.zip", "Meiyun_v2.1.0/Meiyun.unitypackage"] })).toEqual(["meiyun"]);
    expect(avatarFitNames({ productName: null, packageNames: ["Manuka_ver1.02.zip"] })).toEqual(["manuka"]);
  });
  it("adds the name the product title quotes", () => {
    expect(avatarFitNames({ productName: "オリジナル3Dモデル「マヌカ」", packageNames: ["Manuka_ver1.02.zip"] })).toEqual(["manuka", "マヌカ"]);
  });
  it("ignores empty input and names too short to mean anything", () => {
    expect(avatarFitNames({ productName: null, packageNames: [] })).toEqual([]);
    expect(avatarFitNames({ productName: "「M」", packageNames: ["M.zip"] })).toEqual([]);
  });
});

describe("outfit fit rule", () => {
  const meiyun = { productName: "Meiyun 素体", packageNames: ["Meiyun_v2.1.0.zip"] };
  it("marks outfits whose names mention the Avatar, as in the author's library", () => {
    for (const materialName of ["TshirtShorts_Meiyun_v1.00.zip", "Meiyun_Costume4_v1.0.0.zip", "landmine vampire(meiyun).unitypackage", "ChainedCrystal_Meiyun_1.0.unitypackage.zip"]) {
      expect(outfitFitsAvatar(meiyun, { productName: null, materialName }), materialName).toBe(true);
    }
  });
  it("matches through the outfit's observable text such as the target-Avatar line", () => {
    expect(outfitFitsAvatar(meiyun, { productName: "School set", materialName: "School_v1.zip", text: "対応アバター：Meiyun" })).toBe(true);
  });
  it("matches a quoted Japanese name and full-width spellings", () => {
    const manuka = { productName: "オリジナル3Dモデル「マヌカ」", packageNames: ["Manuka_ver1.02.zip"] };
    expect(outfitFitsAvatar(manuka, { productName: "【マヌカ対応】ニットワンピース", materialName: "Knit_v1.zip" })).toBe(true);
    expect(outfitFitsAvatar(manuka, { productName: null, materialName: "ＭＡＮＵＫＡ ｄｒｅｓｓ" })).toBe(true);
  });
  it("leaves unrelated outfits and material-only packages with the others", () => {
    expect(outfitFitsAvatar(meiyun, { productName: "Dress for Kikyu", materialName: "Kikyu_Dress.zip", text: "対応アバター：ききゅう" })).toBe(false);
    expect(outfitFitsAvatar(meiyun, { productName: null, materialName: "ChainedCrystal_Material_1.0.unitypackage.zip" })).toBe(false);
  });
  it("assumes nothing when the outfit has no text", () => {
    expect(outfitFitsAvatar(meiyun, { productName: null, materialName: null, text: null })).toBe(false);
  });
});

describe("outfit grouping", () => {
  it("puts fitted outfits first and keeps the rest selectable in order", () => {
    const meiyun = { productName: "Meiyun", packageNames: ["Meiyun_v2.1.0.zip"] };
    const dress = { itemId: "b", productName: "Dress", materialName: "Kikyu_Dress.zip" };
    const shirt = { itemId: "c", productName: "T-shirt", materialName: "TshirtShorts_Meiyun_v1.00.zip" };
    const costume = { itemId: "d", productName: null, materialName: "Meiyun_Costume4.unitypackage" };
    expect(groupOutfitsByFit(meiyun, [dress, shirt, costume])).toEqual({ fitted: [shirt, costume], others: [dress] });
  });
});

describe("start request assembly", () => {
  it("names a single Avatar package and sends outfits as automatic materials", () => {
    const choices = buildStartChoices(["meiyun"], ["tshirt", "liltoon"]);
    expect(choices).toEqual([{ sourceId: "meiyun", role: "avatar" }, { sourceId: "tshirt", role: "auto" }, { sourceId: "liltoon", role: "auto" }]);
    expect(validateStartChoices(choices)).toBe(true);
  });
  it("sends an Avatar with several packages as automatic materials", () => {
    const choices = buildStartChoices(["body", "physbones"], ["tshirt"]);
    expect(choices.every(choice => choice.role === "auto")).toBe(true);
    expect(validateStartChoices(choices)).toBe(true);
  });
  it("starts an Avatar-only line that outfits can join later", () => {
    const choices = buildStartChoices(["meiyun"], []);
    expect(choices).toEqual([{ sourceId: "meiyun", role: "auto" }]);
    expect(validateStartChoices(choices)).toBe(true);
  });
  it("keeps each material once and never lists the Avatar as an outfit", () => {
    expect(buildStartChoices(["s1", "s1"], ["s2", "s1"])).toEqual([{ sourceId: "s1", role: "avatar" }, { sourceId: "s2", role: "auto" }]);
  });
});
