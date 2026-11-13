import { hasAmfSwitchOptions } from "./amf-production-v01.js";
import type { AmfRecipeDocument, AmfRecipeFit, AmfRecipeFitObject, AmfSharedRecipe } from "./amf-recipe-v05.js";

/** Candidate portable Recipe v0.6, emitted only when optional Avatar Optimizer is enabled. */
export type AmfRecipeDocumentV06 = AmfRecipeDocument & {
  readonly formatVersion: "0.6";
  readonly adjustments: NonNullable<AmfRecipeDocument["adjustments"]> & { readonly optimizer: true };
};
export type AmfSharedRecipeV06 = AmfSharedRecipe & {
  readonly formatVersion: "0.6";
  readonly recipe: AmfRecipeDocumentV06;
};

const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: readonly string[]) => Object.keys(v).length === keys.length && keys.every(k => k in v);
const label = (v: unknown, max: number): v is string => typeof v === "string" && v.trim().length > 0 && [...v].length <= max && !/[\u0000-\u001f\u007f-\u009f]/u.test(v);
const assetPath = (v: unknown): v is string => typeof v === "string" && new TextEncoder().encode(v).length <= 2048 && v.startsWith("Assets/") && v.endsWith(".prefab") && !/[\\:\0]/u.test(v) && v.split("/").every(p => p !== "" && p !== "." && p !== "..");
const materialPath = (v: unknown): v is string => typeof v === "string" && new TextEncoder().encode(v).length <= 2048 && v.startsWith("Assets/") && v.endsWith(".mat") && !/[\\:\0]/u.test(v) && v.split("/").every(p => p !== "" && p !== "." && p !== "..");
const lightingValue = (v: unknown): v is string => typeof v === "string" && new TextEncoder().encode(v).length <= 64 && /^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$/u.test(v);
const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const fitVector = (v: unknown): boolean => record(v) && exact(v, ["x", "y", "z"]) && finite(v.x) && finite(v.y) && finite(v.z);
const fitObject = (v: unknown): v is AmfRecipeFitObject => record(v) && exact(v, ["path", "localPosition", "localRotation", "localScale"])
  && typeof v.path === "string" && v.path.length <= 512 && !/[\u0000-\u001f\u007f-\u009f]/u.test(v.path)
  && fitVector(v.localPosition) && fitVector(v.localScale)
  && record(v.localRotation) && exact(v.localRotation, ["x", "y", "z", "w"]) && finite(v.localRotation.x) && finite(v.localRotation.y) && finite(v.localRotation.z) && finite(v.localRotation.w);
const validId = (v: unknown): v is string => typeof v === "string" && /^[a-zA-Z0-9_-]{1,100}$/u.test(v);
const guid = (v: unknown): boolean => v === undefined || (typeof v === "string" && /^[a-f0-9]{32}$/u.test(v));
const fits = (v: unknown): v is readonly AmfRecipeFit[] => Array.isArray(v) && v.length <= 64
  && v.every(fit => record(fit) && exact(fit, ["materialId", "objects"]) && validId(fit.materialId)
    && Array.isArray(fit.objects) && fit.objects.length <= 2048 && fit.objects.every(fitObject));

export function isAmfRecipeDocumentV06(v: unknown): v is AmfRecipeDocumentV06 {
  if (!record(v) || !exact(v, ["formatVersion", "recipeId", "title", "editorVersion", "materials", "avatar", "outfits", "packages", "adjustments"])
    || v.formatVersion !== "0.6" || typeof v.recipeId !== "string" || !/^[a-zA-Z0-9-]{1,100}$/u.test(v.recipeId) || !label(v.title, 120)
    || !["2022.3.22f1", "2022.3.22f1c1"].includes(String(v.editorVersion)) || !Array.isArray(v.materials) || v.materials.length < 1 || v.materials.length > 64
    || !Array.isArray(v.outfits) || v.outfits.length < 1 || v.outfits.length > 64 || !record(v.packages) || !record(v.adjustments)) return false;
  const ids = new Set<string>();
  for (const m of v.materials) {
    if (!record(m) || !exact(m, ["id", "packageName", "sha256", ...("boothProductId" in m ? ["boothProductId"] : [])])
      || (m.boothProductId !== undefined && (typeof m.boothProductId !== "string" || !/^[1-9][0-9]{0,19}$/u.test(m.boothProductId)))
      || typeof m.sha256 !== "string" || !/^[a-f0-9]{64}$/u.test(m.sha256)
      || !validId(m.id) || ids.has(m.id) || !label(m.packageName, 240) || !m.packageName.toLowerCase().endsWith(".unitypackage") || /[\\/:]/u.test(m.packageName)) return false;
    ids.add(m.id);
  }
  if (!record(v.avatar) || !exact(v.avatar, ["materialId", "prefabPath", ...("prefabGuid" in v.avatar ? ["prefabGuid"] : [])])
    || !guid(v.avatar.prefabGuid) || typeof v.avatar.materialId !== "string" || !ids.has(v.avatar.materialId) || !assetPath(v.avatar.prefabPath)) return false;
  const outfits = new Set<string>();
  for (const o of v.outfits) {
    if (!record(o) || !hasAmfSwitchOptions(o, ["materialId", "prefabPath", "mode", "bone", "toggleName", "active", ...("prefabGuid" in o ? ["prefabGuid"] : [])])
      || !guid(o.prefabGuid) || typeof o.materialId !== "string" || !ids.has(o.materialId) || outfits.has(o.materialId)
      || !assetPath(o.prefabPath) || (o.mode !== "merge_armature" && o.mode !== "attach_to_bone") || typeof o.bone !== "string" || new TextEncoder().encode(o.bone).length > 80
      || /[\u0000-\u001f\u007f-\u009f]/u.test(o.bone) || (o.mode === "attach_to_bone" && !o.bone) || typeof o.toggleName !== "string" || [...o.toggleName].length > 80
      || /[\u0000-\u001f\u007f-\u009f]/u.test(o.toggleName) || typeof o.active !== "boolean") return false;
    outfits.add(o.materialId);
  }
  const adj = v.adjustments;
  const adjKeys = "fits" in adj ? ["lighting", "sdkFixes", "fits", "optimizer"] : ["lighting", "sdkFixes", "optimizer"];
  if (!exact(adj, adjKeys) || adj.optimizer !== true || !Array.isArray(adj.lighting) || adj.lighting.length > 512 || !Array.isArray(adj.sdkFixes) || adj.sdkFixes.length > 16) return false;
  for (const entry of adj.lighting) {
    if (!record(entry) || !exact(entry, ["materialId", "material", "min", "max"]) || !validId(entry.materialId) || !ids.has(entry.materialId)
      || !materialPath(entry.material) || !lightingValue(entry.min) || !lightingValue(entry.max)) return false;
  }
  if ("fits" in adj && (!fits(adj.fits) || adj.fits.some(f => !ids.has(f.materialId)))) return false;
  if (!adj.sdkFixes.every(fix => typeof fix === "string" && fix.length > 0 && [...fix].length <= 600 && !fix.includes("\0"))) return false;
  return Object.keys(v.packages).length >= 3 && Object.keys(v.packages).length <= 64
    && typeof v.packages["com.vrchat.avatars"] === "string"
    && typeof v.packages["nadena.dev.modular-avatar"] === "string"
    && typeof v.packages["com.anatawa12.avatar-optimizer"] === "string"
    && Object.entries(v.packages).every(([id, version]) => id.length <= 128 && id.includes(".") && /^[a-zA-Z0-9._-]+$/u.test(id) && typeof version === "string" && /^[a-zA-Z0-9.+-]{1,80}$/u.test(version));
}

export function isAmfSharedRecipeV06(value: unknown): value is AmfSharedRecipeV06 {
  if (!record(value) || !exact(value, ["formatVersion", "recipe", "sources"]) || value.formatVersion !== "0.6" || !isAmfRecipeDocumentV06(value.recipe) || !record(value.sources)) return false;
  const sources = value.sources;
  return Object.keys(sources).length === value.recipe.materials.length && value.recipe.materials.every(material => {
    const id = sources[material.id];
    return record(id) && exact(id, ["boothProductId", ...("shopName" in id ? ["shopName"] : [])])
      && typeof id.boothProductId === "string" && /^[1-9][0-9]{0,19}$/u.test(id.boothProductId)
      && (material.boothProductId === undefined || material.boothProductId === id.boothProductId)
      && (id.shopName === undefined || label(id.shopName, 240));
  });
}
