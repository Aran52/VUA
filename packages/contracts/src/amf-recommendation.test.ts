import { describe, expect, it } from "vitest";
import { packageBaseName, recommendAssembly, type AmfCandidateFacts } from "./amf-recommendation.js";

const prefab = (sourceId: string, path: string, isAvatar: boolean, extra: Partial<AmfCandidateFacts> = {}): AmfCandidateFacts => ({
  sourceId, path, name: path.split("/").pop()!.replace(/\.prefab$/u, ""), isAvatar,
  hasSkinnedMesh: !isAvatar, hasMergeArmature: false, hasMissingScripts: false, ...extra,
});
// The author's real Meiyun materials (P-02).
const meiyun = ["Meiyu_改変 2P", "Meiyun 2P", "Meiyun_ChangeColor", "Meiyun_Costume1", "Meiyun_Costume2", "Meiyun_Costume3", "Meiyu_改変", "Meiyun"]
  .map(name => prefab("avatar", `Assets/Nigerunaya/Meiyun/Prefab/${name}.prefab`, true));
const sources = [
  { sourceId: "avatar", role: "auto" as const, itemName: "Meiyun_v2.1.0.zip" },
  { sourceId: "outfit", role: "auto" as const, itemName: "TshirtShorts_Meiyun_v1.00.zip" },
];

describe("recommending the Avatar and outfits", () => {
  it("picks the creator's base for outfits among Meiyun's eight prefabs, and the T-shirt & shorts for it", () => {
    const outfit = prefab("outfit", "Assets/Amenbo/Tshirt&Shorts/Prefab/TshirtShorts_Meiyun.prefab", false, { hasMergeArmature: true });
    const recommendation = recommendAssembly([...meiyun, outfit], sources);
    expect(recommendation.avatar).toEqual({ path: "Assets/Nigerunaya/Meiyun/Prefab/Meiyu_改変.prefab", reason: "creator_base", tied: [] });
    expect(recommendation.outfits).toEqual([{ sourceId: "outfit", path: outfit.path, reason: "only", tied: [] }]);
    expect(recommendation.selection.outfits[0]).toMatchObject({ prefabPath: outfit.path, mode: "merge_armature", toggleName: "TshirtShorts", active: true });
  });
  it("falls back to the prefab named after its package, then to the package's top-level prefab", () => {
    const plain = meiyun.filter(candidate => !candidate.name.includes("改変"));
    expect(recommendAssembly(plain, sources).avatar).toMatchObject({ path: "Assets/Nigerunaya/Meiyun/Prefab/Meiyun.prefab", reason: "package_name" });
    const nested = [prefab("avatar", "Assets/Karin/Karin_Main.prefab", true), prefab("avatar", "Assets/Karin/Variants/Karin_Summer.prefab", true)];
    expect(recommendAssembly(nested, [{ sourceId: "avatar", role: "auto", itemName: "Karin_v1.4.zip" }]).avatar).toMatchObject({ path: "Assets/Karin/Karin_Main.prefab", reason: "top_level" });
  });
  it("chooses the outfit prefab that names the Avatar in a package made for several Avatars", () => {
    const outfits = ["SailorDress_Karin", "SailorDress_Meiyun", "SailorDress_Rusk"].map(name => prefab("outfit", `Assets/Sailor/${name}.prefab`, false));
    const recommendation = recommendAssembly([...meiyun, ...outfits], [sources[0]!, { sourceId: "outfit", role: "auto", itemName: "SailorDress_v2.zip" }]);
    expect(recommendation.outfits[0]).toMatchObject({ path: "Assets/Sailor/SailorDress_Meiyun.prefab", reason: "names_avatar" });
    expect(recommendation.selection.outfits[0]?.toggleName).toBe("SailorDress");
  });
  it("asks when two candidates are equally plausible", () => {
    const twins = [prefab("avatar", "Assets/A/Alpha.prefab", true), prefab("avatar", "Assets/B/Beta.prefab", true)];
    const recommendation = recommendAssembly(twins, [{ sourceId: "avatar", role: "auto", itemName: "Pack.zip" }]);
    expect(recommendation.avatar).toEqual({ path: null, reason: null, tied: ["Assets/A/Alpha.prefab", "Assets/B/Beta.prefab"] });
    expect(recommendation.selection.avatarPrefab).toBe("");
    const outfits = ["Hoodie_Black", "Hoodie_White"].map(name => prefab("outfit", `Assets/Hoodie/${name}.prefab`, false));
    const outfit = recommendAssembly([...meiyun, ...outfits], [sources[0]!, { sourceId: "outfit", role: "outfit", itemName: "Hoodie.zip" }]).outfits[0];
    expect(outfit).toMatchObject({ path: null, reason: null, tied: ["Assets/Hoodie/Hoodie_Black.prefab", "Assets/Hoodie/Hoodie_White.prefab"] });
  });
  it("reads a package's own name without its version", () => {
    expect(packageBaseName("Meiyun_v2.1.0.zip")).toBe("meiyun");
    expect(packageBaseName("TshirtShorts_Meiyun_v1.00.zip")).toBe("tshirtshortsmeiyun");
    expect(packageBaseName("Karin v1.4.unitypackage")).toBe("karin");
  });
});
