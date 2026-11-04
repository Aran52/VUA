/** Complete material checklist and the differences a player reviews before assembly. */
export type AmfMaterialChoices = Readonly<Record<string, string>>;
export interface AmfMaterialPackage { readonly sourceId: string; readonly packageName: string; readonly sha256: string }
export interface AmfRecipeMaterialResolution {
  readonly materialId: string; readonly packageName: string; readonly boothProductId: string | null;
  readonly status: "ready" | "material_choice_required" | "material_not_downloaded" | "material_not_owned" | "material_missing";
  readonly resolved: AmfMaterialPackage | null; readonly candidates: readonly AmfMaterialPackage[]; readonly downloadNames: readonly string[];
}
export interface AmfRecipeDifference {
  readonly kind: "material_file_changed" | "prefab_mapped" | "prefab_missing" | "fit_objects_missing" | "lighting_material_missing";
  readonly materialId: string; readonly before: string; readonly after: string | null; readonly rule: "guid" | "path" | "name" | null;
  readonly paths: readonly string[]; readonly count: number;
}
export interface AmfRecipeMaterialPlan {
  readonly recipeId: string; readonly materials: readonly AmfRecipeMaterialResolution[];
  readonly materialChoices: AmfMaterialChoices; readonly ready: boolean; readonly planDigest: string;
}
const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const text = (v: unknown, max: number): v is string => typeof v === "string" && v.length <= max && !v.includes("\0");
const id = (v: unknown): v is string => typeof v === "string" && /^[a-zA-Z0-9_-]{1,100}$/u.test(v);
const booth = (v: unknown): boolean => typeof v === "string" && /^[1-9][0-9]{0,19}$/u.test(v);
const packageName = (v: unknown): boolean => text(v,240) && /\.unitypackage$/iu.test(v) && !/[\\/:\u0000-\u001f\u007f-\u009f]/u.test(v);
export const isAmfMaterialChoices = (v: unknown): v is AmfMaterialChoices => record(v) && Object.keys(v).length <= 64
  && Object.entries(v).every(([key,value]) => id(key) && text(value,256) && value.length > 0);
const materialPackage = (v: unknown): v is AmfMaterialPackage => record(v) && Object.keys(v).length === 3
  && text(v.sourceId,256) && v.sourceId.length > 0 && packageName(v.packageName) && typeof v.sha256 === "string" && /^[a-f0-9]{64}$/u.test(v.sha256);
export const isAmfRecipeMaterials = (v: unknown): v is readonly AmfRecipeMaterialResolution[] => Array.isArray(v) && v.length <= 64
  && v.every(m => record(m) && Object.keys(m).length === 7 && id(m.materialId) && packageName(m.packageName)
    && (m.boothProductId === null || booth(m.boothProductId))
    && ["ready","material_choice_required","material_not_downloaded","material_not_owned","material_missing"].includes(String(m.status))
    && (m.status === "ready" ? materialPackage(m.resolved) : m.resolved === null)
    && Array.isArray(m.candidates) && m.candidates.length <= 4096 && m.candidates.every(materialPackage)
    && Array.isArray(m.downloadNames) && m.downloadNames.length <= 4096 && m.downloadNames.every(n => text(n,4096)))
  && new Set(v.map(m => m.materialId)).size === v.length;
export const isAmfRecipeDifferences = (v: unknown): v is readonly AmfRecipeDifference[] => Array.isArray(v) && v.length <= 768
  && v.every(d => record(d) && Object.keys(d).length === 7
    && ["material_file_changed","prefab_mapped","prefab_missing","fit_objects_missing","lighting_material_missing"].includes(String(d.kind))
    && (d.materialId === "" || id(d.materialId)) && text(d.before,2048) && (d.after === null || text(d.after,2048))
    && (d.rule === null || ["guid","path","name"].includes(String(d.rule)))
    && Number.isSafeInteger(d.count) && Number(d.count) >= 1 && Number(d.count) <= 2048
    && Array.isArray(d.paths) && d.paths.length <= 2048 && d.paths.every(p => text(p,2048))
    && (d.kind !== "fit_objects_missing" || d.count === d.paths.length));
/** Optional fields allow retained runs and older engines to remain readable. */
export const hasValidAmfRecipeReview = (v: Record<string,unknown>): boolean =>
  (v.materials === undefined || isAmfRecipeMaterials(v.materials))
  && (v.materialChoices === undefined || isAmfMaterialChoices(v.materialChoices))
  && (v.recipeDifferences === undefined || isAmfRecipeDifferences(v.recipeDifferences))
  && (v.canAssemble === undefined || typeof v.canAssemble === "boolean")
  && (v.canApply === undefined || typeof v.canApply === "boolean");
export const isAmfRecipeMaterialPlan = (v: unknown): v is AmfRecipeMaterialPlan => record(v) && id(v.recipeId)
  && isAmfRecipeMaterials(v.materials) && isAmfMaterialChoices(v.materialChoices)
  && v.ready === v.materials.every(m => m.status === "ready") && typeof v.planDigest === "string" && /^[a-f0-9]{64}$/u.test(v.planDigest);
