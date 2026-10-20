/** Candidate portable declarations. Local saving does not establish sharing provenance. */
export interface AmfRecipeMaterial { readonly id: string; readonly packageName: string; readonly sha256: string }
export interface AmfRecipeOutfit { readonly materialId: string; readonly prefabPath: string; readonly mode: "merge_armature" | "attach_to_bone"; readonly bone: string; readonly toggleName: string; readonly active: boolean }
export interface AmfRecipeLighting { readonly material: string; readonly min: string; readonly max: string }
/** One object's saved local-Transform adjustment of a garment on an Avatar. */
export interface AmfRecipeFitObject {
  readonly path: string;
  readonly localPosition: { readonly x: number; readonly y: number; readonly z: number };
  readonly localRotation: { readonly x: number; readonly y: number; readonly z: number; readonly w: number };
  readonly localScale: { readonly x: number; readonly y: number; readonly z: number };
}
/** The saved fit of one material's garment: every adjusted object with its local Transform. */
export interface AmfRecipeFit { readonly materialId: string; readonly objects: readonly AmfRecipeFitObject[] }
export interface AmfRecipeAdjustments { readonly lighting: readonly AmfRecipeLighting[]; readonly sdkFixes: readonly string[]; readonly fits?: readonly AmfRecipeFit[] }
export interface AmfRecipeDocument {
  readonly formatVersion: "0.4"; readonly recipeId: string; readonly title: string; readonly editorVersion: "2022.3.22f1" | "2022.3.22f1c1";
  readonly materials: readonly AmfRecipeMaterial[]; readonly avatar: { readonly materialId: string; readonly prefabPath: string };
  readonly outfits: readonly AmfRecipeOutfit[]; readonly packages: Readonly<Record<string, string>>;
  readonly adjustments?: AmfRecipeAdjustments;
}
const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: readonly string[]) => Object.keys(v).length === keys.length && keys.every(k => k in v);
const label = (v: unknown, max: number): v is string => typeof v === "string" && v.trim().length > 0 && [...v].length <= max && !/[\u0000-\u001f\u007f-\u009f]/u.test(v);
const assetPath = (v: unknown): v is string => typeof v === "string" && new TextEncoder().encode(v).length <= 2048 && v.startsWith("Assets/") && v.endsWith(".prefab") && !/[\\:\0]/u.test(v) && v.split("/").every(p => p !== "" && p !== "." && p !== "..");
const materialPath = (v: unknown): v is string => typeof v === "string" && new TextEncoder().encode(v).length <= 2048 && v.startsWith("Assets/") && v.endsWith(".mat") && !/[\\:\0]/u.test(v) && v.split("/").every(p => p !== "" && p !== "." && p !== "..");
const lightingValue = (v: unknown): v is string => typeof v === "string" && new TextEncoder().encode(v).length <= 64 && /^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$/u.test(v);
const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const fitVector = (v: unknown): boolean => record(v) && exact(v,["x","y","z"]) && finite(v.x) && finite(v.y) && finite(v.z);
const fitObject = (v: unknown): v is AmfRecipeFitObject => record(v) && exact(v,["path","localPosition","localRotation","localScale"])
  && typeof v.path === "string" && v.path.length <= 512 && !/[\u0000-\u001f\u007f-\u009f]/u.test(v.path)
  && fitVector(v.localPosition) && fitVector(v.localScale)
  && record(v.localRotation) && exact(v.localRotation,["x","y","z","w"]) && finite(v.localRotation.x) && finite(v.localRotation.y) && finite(v.localRotation.z) && finite(v.localRotation.w);
const fits = (v: unknown): v is readonly AmfRecipeFit[] => Array.isArray(v) && v.length <= 64
  && v.every(fit => record(fit) && exact(fit,["materialId","objects"]) && typeof fit.materialId === "string" && /^m-[a-f0-9]{64}$/u.test(fit.materialId)
    && Array.isArray(fit.objects) && fit.objects.length <= 2048 && fit.objects.every(fitObject));
const adjustments = (v: unknown): v is AmfRecipeAdjustments => {
  if (!record(v) || !exact(v,"fits" in v ? ["lighting","sdkFixes","fits"] : ["lighting","sdkFixes"]) || !Array.isArray(v.lighting) || v.lighting.length > 512
    || !Array.isArray(v.sdkFixes) || v.sdkFixes.length > 16) return false;
  for (const entry of v.lighting) {
    if (!record(entry) || !exact(entry,["material","min","max"]) || !materialPath(entry.material) || !lightingValue(entry.min) || !lightingValue(entry.max)) return false;
  }
  if ("fits" in v && !fits(v.fits)) return false;
  return v.sdkFixes.every(fix => typeof fix === "string" && fix.length > 0 && [...fix].length <= 600 && !fix.includes("\0"));
};
export function isAmfRecipeDocument(v: unknown): v is AmfRecipeDocument {
  if (!record(v) || !exact(v,"adjustments" in v ? ["formatVersion","recipeId","title","editorVersion","materials","avatar","outfits","packages","adjustments"] : ["formatVersion","recipeId","title","editorVersion","materials","avatar","outfits","packages"])
    || v.formatVersion !== "0.4" || typeof v.recipeId !== "string" || !/^[a-zA-Z0-9-]{1,100}$/u.test(v.recipeId) || !label(v.title,120)
    || !["2022.3.22f1","2022.3.22f1c1"].includes(String(v.editorVersion)) || !Array.isArray(v.materials) || v.materials.length < 1 || v.materials.length > 64
    || !Array.isArray(v.outfits) || v.outfits.length < 1 || v.outfits.length > 64 || !record(v.packages)) return false;
  const ids = new Set<string>();
  for (const m of v.materials) {
    if (!record(m) || !exact(m,["id","packageName","sha256"]) || typeof m.sha256 !== "string" || !/^[a-f0-9]{64}$/u.test(m.sha256)
      || m.id !== `m-${m.sha256}` || ids.has(m.id) || !label(m.packageName,240) || !m.packageName.toLowerCase().endsWith(".unitypackage") || /[\\/:]/u.test(m.packageName)) return false;
    ids.add(m.id);
  }
  if (!record(v.avatar) || !exact(v.avatar,["materialId","prefabPath"]) || typeof v.avatar.materialId !== "string" || !ids.has(v.avatar.materialId) || !assetPath(v.avatar.prefabPath)) return false;
  const outfits = new Set<string>();
  for (const o of v.outfits) {
    if (!record(o) || !exact(o,["materialId","prefabPath","mode","bone","toggleName","active"]) || typeof o.materialId !== "string" || !ids.has(o.materialId) || outfits.has(o.materialId)
      || !assetPath(o.prefabPath) || (o.mode !== "merge_armature" && o.mode !== "attach_to_bone") || typeof o.bone !== "string" || new TextEncoder().encode(o.bone).length > 80
      || /[\u0000-\u001f\u007f-\u009f]/u.test(o.bone) || (o.mode === "attach_to_bone" && !o.bone) || typeof o.toggleName !== "string" || [...o.toggleName].length > 80
      || /[\u0000-\u001f\u007f-\u009f]/u.test(o.toggleName) || typeof o.active !== "boolean") return false;
    outfits.add(o.materialId);
  }
  if ("adjustments" in v && !adjustments(v.adjustments)) return false;
  return Object.keys(v.packages).length <= 64 && typeof v.packages["com.vrchat.avatars"] === "string" && typeof v.packages["nadena.dev.modular-avatar"] === "string"
    && Object.entries(v.packages).every(([id, version]) => id.length <= 128 && id.includes(".") && /^[a-zA-Z0-9._-]+$/u.test(id) && typeof version === "string" && /^[a-zA-Z0-9.+-]{1,80}$/u.test(version));
}

export interface AmfSharedRecipe { readonly formatVersion: "0.4"; readonly recipe: AmfRecipeDocument; readonly sources: Readonly<Record<string,string>> }
export function isAmfSharedRecipe(value: unknown): value is AmfSharedRecipe {
  if (!record(value) || !exact(value,["formatVersion","recipe","sources"]) || value.formatVersion !== "0.4" || !isAmfRecipeDocument(value.recipe) || !record(value.sources)) return false;
  const sources = value.sources;
  return Object.keys(sources).length === value.recipe.materials.length && value.recipe.materials.every(material => { const id = sources[material.id]; return typeof id === "string" && /^[1-9][0-9]{0,19}$/u.test(id); });
}

/** A bounded, closed decoder for paste text and JSON files; it never loads assets. */
export function decodeAmfRecipeShare(text: string): AmfSharedRecipe | null {
  if (typeof text !== "string" || text.length > 349557) return null;
  try {
    let json = text.trim();
    if (json.startsWith("vuar0.4.")) {
      const payload = json.slice(8);
      if (!/^[a-zA-Z0-9_-]+$/u.test(payload)) return null;
      const binary = atob(payload.replace(/-/gu,"+").replace(/_/gu,"/"));
      if (binary.length > 262144) return null;
      json = new TextDecoder("utf-8",{fatal:true}).decode(Uint8Array.from(binary,char=>char.charCodeAt(0)));
    }
    if (new TextEncoder().encode(json).length > 262144) return null;
    const result: unknown = JSON.parse(json);
    return isAmfSharedRecipe(result) ? result : null;
  } catch { return null; }
}
export function sameAmfRecipeShare(a: AmfSharedRecipe, b: AmfSharedRecipe): boolean {
  const sorted = (value: unknown): unknown => Array.isArray(value) ? value.map(sorted)
    : record(value) ? Object.fromEntries(Object.keys(value).sort().map(key=>[key,sorted(value[key])])) : value;
  return JSON.stringify(sorted(a)) === JSON.stringify(sorted(b));
}
