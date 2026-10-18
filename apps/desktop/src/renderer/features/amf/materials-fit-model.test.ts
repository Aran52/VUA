import { describe, expect, it } from "vitest";
import { avatarFitNames, buildStartChoices, groupOutfitsByFit, normalizeFitText, outfitFitsAvatar } from "./materials-fit-model.ts";

describe("fit comparison form", () => {
  it("folds case, full-width characters and spaces", () => {
    expect(normalizeFitText("Ｍｅｉｙｕｎ ＶＲ")).toBe("meiyunvr");
    expect(normalizeFitText("Mei yun")).toBe(normalizeFitText("ｍｅｉｙｕｎ"));
  });
});

describe("avatar fit names", () => {
  it("collects the product and material names", () => {
    expect(avatarFitNames({ productName: "Meiyun 素体", materialName: "Meiyun_v2.1.0.zip" }))
      .toEqual(["meiyun素体", "meiyun_v2.1.0.zip", "meiyun", "meiyu"]);
  });
  it("adds the recorded spellings only when the Avatar's own name carries that name", () => {
    expect(avatarFitNames({ productName: "Another Avatar", materialName: null })).toEqual(["anotheravatar"]);
  });
  it("ignores empty input and names too short to mean anything", () => {
    expect(avatarFitNames({ productName: null, materialName: "" })).toEqual([]);
    expect(avatarFitNames({ productName: "M", materialName: null })).toEqual([]);
  });
});

describe("outfit fit rule", () => {
  const meiyun = { productName: "Meiyun 素体", materialName: "Meiyun_v2.1.0.zip" };
  it("marks an outfit whose material name mentions the Avatar", () => {
    expect(outfitFitsAvatar(meiyun, { productName: "T-shirt & shorts", materialName: "TshirtShorts_Meiyun_v1.00.zip", text: null })).toBe(true);
  });
  it("matches through the outfit's observable text such as the target-Avatar line", () => {
    expect(outfitFitsAvatar(meiyun, { productName: "School set", materialName: "School_v1.zip", text: "対応アバター：Meiyun" })).toBe(true);
  });
  it("matches a recorded spelling with different width, case and spacing", () => {
    expect(outfitFitsAvatar(meiyun, { productName: null, materialName: "ｍｅｉｙｕ 改変", text: null })).toBe(true);
  });
  it("leaves an unrelated outfit with the others", () => {
    expect(outfitFitsAvatar(meiyun, { productName: "Dress for Kikyu", materialName: "Kikyu_Dress.zip", text: "対応アバター：ききゅう" })).toBe(false);
  });
  it("assumes nothing when the outfit has no text", () => {
    expect(outfitFitsAvatar(meiyun, { productName: null, materialName: null, text: null })).toBe(false);
  });
});

describe("outfit grouping", () => {
  it("puts fitted outfits first and keeps the rest selectable in order", () => {
    const meiyun = { productName: "Meiyun", materialName: null };
    const dress = { itemId: "b", productName: "Dress", materialName: "Kikyu_Dress.zip" };
    const shirt = { itemId: "c", productName: "T-shirt", materialName: "TshirtShorts_Meiyun_v1.00.zip" };
    const costume = { itemId: "d", productName: null, materialName: "Meiyun_Costume4.unitypackage" };
    expect(groupOutfitsByFit(meiyun, [dress, shirt, costume])).toEqual({ fitted: [shirt, costume], others: [dress] });
  });
});

describe("start request assembly", () => {
  it("sends the Avatar materials with the avatar role and the outfits with the outfit role", () => {
    expect(buildStartChoices(["s1", "s2"], ["s3", "s4"])).toEqual([
      { sourceId: "s1", role: "avatar" }, { sourceId: "s2", role: "avatar" },
      { sourceId: "s3", role: "outfit" }, { sourceId: "s4", role: "outfit" },
    ]);
  });
  it("keeps each material once and never dresses the Avatar in its own outfit role", () => {
    expect(buildStartChoices(["s1", "s1"], ["s2", "s1"])).toEqual([{ sourceId: "s1", role: "avatar" }, { sourceId: "s2", role: "outfit" }]);
  });
});
