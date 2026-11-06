import type { AmfRecipeDocument, AmfRecipeMaterialResolution } from "@vua/contracts";
import { describe, expect, it } from "vitest";
import { newestFirst, recipeMaterialRows, recipeProductIds, recipeReadiness } from "./recipe-cards-model.ts";

const resolution = (materialId: string, status: AmfRecipeMaterialResolution["status"], boothProductId: string | null = null): AmfRecipeMaterialResolution =>
  ({ materialId, packageName: `${materialId}.unitypackage`, boothProductId, status, resolved: null, candidates: [], downloadNames: [] });
const recipe = {
  formatVersion: "0.5", recipeId: "r", title: "R", editorVersion: "2022.3.22f1",
  materials: [
    { id: "shader", packageName: "shader.unitypackage", sha256: "a".repeat(64) },
    { id: "coat", packageName: "coat.unitypackage", sha256: "b".repeat(64), boothProductId: "20" },
    { id: "body", packageName: "body.unitypackage", sha256: "c".repeat(64), boothProductId: "10" },
  ],
  avatar: { materialId: "body", prefabPath: "Assets/Body.prefab" },
  outfits: [{ materialId: "coat", prefabPath: "Assets/Coat.prefab", mode: "merge_armature", bone: "", toggleName: "Coat", active: true }],
  packages: { "com.vrchat.avatars": "3.7.4", "nadena.dev.modular-avatar": "1.10.3" },
} as AmfRecipeDocument;

describe("a Recipe's readiness", () => {
  it("names what stops a new line first", () => {
    expect(recipeReadiness({ ready: true, materials: [resolution("a", "ready")] })).toEqual({ kind: "ready" });
    expect(recipeReadiness({ ready: false, materials: [resolution("a", "material_choice_required"), resolution("b", "ready")] })).toEqual({ kind: "choose", count: 1 });
    expect(recipeReadiness({ ready: false, materials: [resolution("a", "material_choice_required"), resolution("b", "material_not_downloaded"), resolution("c", "material_not_owned"), resolution("d", "material_missing")] })).toEqual({ kind: "missing", count: 3 });
  });
});

describe("a Recipe's material rows", () => {
  it("lists the Avatar first, then outfits, then the rest", () => {
    expect(recipeMaterialRows(recipe).map(row => [row.materialId, row.role])).toEqual([["body", "avatar"], ["coat", "outfit"], ["shader", "other"]]);
  });

  it("takes the BOOTH product from the Recipe, else from what the plan resolved", () => {
    const rows = recipeMaterialRows(recipe, { materials: [resolution("shader", "material_not_owned", "30")] });
    expect(rows.map(row => row.productId)).toEqual(["booth:10", "booth:20", "booth:30"]);
    expect(rows.find(row => row.materialId === "shader")!.status).toBe("material_not_owned");
    expect(rows.find(row => row.materialId === "body")!.status).toBeNull();
  });

  it("collects each product once across Recipes", () => {
    const rows = recipeMaterialRows(recipe);
    expect(recipeProductIds([rows, rows])).toEqual(["booth:10", "booth:20"]);
  });
});

it("orders Recipes by their last save", () => {
  expect(newestFirst([{ updatedAt: "2026-01-01" }, { updatedAt: "2026-03-01" }]).map(item => item.updatedAt)).toEqual(["2026-03-01", "2026-01-01"]);
});
