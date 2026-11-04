import { describe, expect, it } from "vitest";
import type { AmfRecipeDocument } from "@vua/contracts";
import type { InstalledPackageRowV01, InstalledPackageRowV02 } from "../../gateway/index.ts";
import { guideNeeds, packageConclusion, packagePageDetail, packagePinLines, recipePinsByPackage } from "./package-mode-model.ts";

const recipe = (recipeId: string, title: string, packages: Record<string, string>): AmfRecipeDocument => ({
  formatVersion: "0.4", recipeId, title, editorVersion: "2022.3.22f1",
  materials: [{ id: "m-" + "0".repeat(64), packageName: "Avatar_v1.0.unitypackage", sha256: "0".repeat(64) }],
  avatar: { materialId: "m-" + "0".repeat(64), prefabPath: "Assets/Avatar/Avatar.prefab" },
  outfits: [{ materialId: "m-" + "0".repeat(64), prefabPath: "Assets/Avatar/Avatar.prefab", mode: "merge_armature", bone: "", toggleName: "Avatar", active: true }],
  packages,
});

describe("package page detail by permission mode", () => {
  it("maps full auto to conclusions only, limited auto to next step, manual review to the table", () => {
    expect(packagePageDetail("full")).toBe("conclusion");
    expect(packagePageDetail("smart")).toBe("next_step");
    expect(packagePageDetail("ask")).toBe("table");
  });
});

describe("recipe pins", () => {
  const pins = recipePinsByPackage([
    recipe("b-recipe", "Stage Meiyun", { "jp.lilxyzw.liltoon": "1.8.5", "nadena.dev.modular-avatar": "1.11.0" }),
    recipe("a-recipe", "Summer Meiyun", { "jp.lilxyzw.liltoon": "1.8.5" }),
    recipe("c-recipe", "Old hoodie", { "jp.lilxyzw.liltoon": "1.7.3" }),
  ]);
  it("collects every Recipe pinning a package, in title order", () => {
    expect(pins.get("jp.lilxyzw.liltoon")?.map(pin => [pin.title, pin.version]))
      .toEqual([["Old hoodie", "1.7.3"], ["Stage Meiyun", "1.8.5"], ["Summer Meiyun", "1.8.5"]]);
    expect(pins.get("nadena.dev.modular-avatar")?.length).toBe(1);
    expect(pins.has("com.vrchat.avatars")).toBe(false);
  });
  it("keeps only installed packages and groups the titles per pinned version", () => {
    const lines = packagePinLines(pins, new Set(["jp.lilxyzw.liltoon"]));
    expect(lines).toEqual([
      { packageId: "jp.lilxyzw.liltoon", version: "1.7.3", recipes: ["Old hoodie"] },
      { packageId: "jp.lilxyzw.liltoon", version: "1.8.5", recipes: ["Stage Meiyun", "Summer Meiyun"] },
    ]);
    expect(packagePinLines(pins, new Set(["com.vrchat.avatars"]))).toEqual([]);
  });
});

describe("guide needs", () => {
  it("picks amf.package.guide diagnostics and reads the official page from the fixed message shape", () => {
    const needs = guideNeeds([
      { code: "amf.package.guide", message: "2 materials need Poiyomi Pro; get it from https://www.poiyomi.com/." },
      { code: "amf.package.guide", message: "1 material needs UnlitWF 2.9 and 2.14; get it from https://whiteflare.github.io/vpm-repos/vpm.json." },
      { code: "amf.shaders.missing", message: "2 materials miss their shaders." },
    ]);
    expect(needs).toEqual([
      { message: "2 materials need Poiyomi Pro; get it from https://www.poiyomi.com/.", url: "https://www.poiyomi.com/" },
      { message: "1 material needs UnlitWF 2.9 and 2.14; get it from https://whiteflare.github.io/vpm-repos/vpm.json.", url: "https://whiteflare.github.io/vpm-repos/vpm.json" },
    ]);
  });
  it("keeps an off-shape message without a link, and tolerates no diagnostics", () => {
    expect(guideNeeds([{ code: "amf.package.guide", message: "legacy wording" }])).toEqual([{ message: "legacy wording", url: null }]);
    expect(guideNeeds(undefined)).toEqual([]);
  });
});

describe("the conclusion numbers", () => {
  const v01 = (packageId: string): InstalledPackageRowV01 => ({ packageId, version: "1.0.0", dependencies: [] });
  const v02 = (packageId: string, updateAvailable: boolean | null): InstalledPackageRowV02 =>
    ({ packageId, version: "1.0.0", dependencies: [], latestVersion: "1.2.0", updateAvailable });
  it("counts updates only when every row carries the backend's judgment", () => {
    expect(packageConclusion([v02("a", true), v02("b", false), v02("c", true)])).toEqual({ installed: 3, updates: 2 });
    expect(packageConclusion([v02("a", false), v02("b", false)])).toEqual({ installed: 2, updates: 0 });
    expect(packageConclusion([v02("a", null), v02("b", true)])).toEqual({ installed: 2, updates: null });
    expect(packageConclusion([v01("a"), v01("b")])).toEqual({ installed: 2, updates: null });
    expect(packageConclusion([])).toEqual({ installed: 0, updates: 0 });
  });
});
