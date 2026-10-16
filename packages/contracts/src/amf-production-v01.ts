/** Candidate AMF material-first production. Frozen legacy production faces are separate. */
export type AmfSourceRole = "auto" | "avatar" | "outfit" | "dependency";
export interface AmfSourceChoice { readonly sourceId: string; readonly role: AmfSourceRole }
export interface AmfOutfitSelection {
  readonly sourceId: string; readonly prefabPath: string;
  readonly mode: "merge_armature" | "attach_to_bone"; readonly bone: string;
  readonly toggleName: string; readonly active: boolean;
}
export interface AmfAssemblySelection { readonly avatarPrefab: string; readonly outfits: readonly AmfOutfitSelection[] }
export type AmfRecipeApplyChoice = "recipe_wins" | "avatar_wins" | "new_avatar";
export type AmfProductionBodyV01 =
  | { readonly method: "amf.production.recipeApplyPlan"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly recipeId: string; readonly avatarGlobalObjectId?: string } }
  | { readonly method: "amf.production.applyRecipe"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly recipeId: string; readonly planDigest: string; readonly choice: AmfRecipeApplyChoice; readonly avatarGlobalObjectId?: string } }
  | { readonly method: "amf.production.previewRecipeShare" | "amf.production.importRecipe"; readonly params: { readonly schemaVersion: "0.1"; readonly text: string } }
  | { readonly method: "amf.production.exportRecipe"; readonly params: { readonly schemaVersion: "0.1"; readonly recipeId: string; readonly planDigest: string; readonly sources: Readonly<Record<string,string>> } }
  | { readonly method: "amf.production.list"; readonly params: { readonly schemaVersion: "0.1" } }
  | { readonly method: "amf.production.listRecipes"; readonly params: { readonly schemaVersion: "0.1" } }
  | { readonly method: "amf.production.getRecipe" | "amf.production.recipeSharePlan"; readonly params: { readonly schemaVersion: "0.1"; readonly recipeId: string } }
  | { readonly method: "amf.production.saveRecipe"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly title: string } }
  | { readonly method: "amf.production.previewImage"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly path: string } }
  | { readonly method: "amf.production.prepareRecipe"; readonly params: { readonly schemaVersion: "0.1"; readonly recipeId: string; readonly editorPath: string } }
  | { readonly method: "amf.production.get" | "amf.production.reinspect" | "amf.production.open" | "amf.production.rollbackPlan"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string } }
  | { readonly method: "amf.production.rollback"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly planDigest: string } }
  | { readonly method: "amf.production.prepare"; readonly params: { readonly schemaVersion: "0.1"; readonly sources: readonly AmfSourceChoice[]; readonly editorPath: string } }
  | { readonly method: "amf.production.plan"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly selection: AmfAssemblySelection } }
  | { readonly method: "amf.production.assemble"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly planDigest: string; readonly selection: AmfAssemblySelection } };
export type AmfProductionRequestV1 = { readonly schemaVersion: 1; readonly requestId: string } & AmfProductionBodyV01;

const record = (value: unknown): value is Record<string, unknown> => !!value && typeof value === "object" && !Array.isArray(value);
const text = (value: unknown, limit = 4096): value is string => typeof value === "string" && value.length > 0 && value.length <= limit && !value.includes("\0");
const exact = (value: Record<string, unknown>, keys: readonly string[]) => Object.keys(value).length === keys.length && keys.every(key => key in value);
const roles: readonly string[] = ["auto", "avatar", "outfit", "dependency"];

/** A Bridge preview inside the work project: `.vua/previews/<command>/<name>.png`. */
export function isAmfPreviewPath(value: unknown): value is string {
  return typeof value === "string" && value.length <= 512 && /^\.vua\/previews\/[A-Za-z0-9_-][A-Za-z0-9_.-]*\/[A-Za-z0-9_-][A-Za-z0-9_.-]*\.png$/u.test(value)
    && !value.split("/").includes("..");
}
export function isAmfAssemblySelection(value: unknown): value is AmfAssemblySelection {
  if (!record(value) || !exact(value, ["avatarPrefab", "outfits"]) || !text(value.avatarPrefab) || !Array.isArray(value.outfits) || value.outfits.length < 1 || value.outfits.length > 64) return false;
  return value.outfits.every(outfit => record(outfit) && exact(outfit, ["sourceId", "prefabPath", "mode", "bone", "toggleName", "active"])
    && text(outfit.sourceId, 256) && text(outfit.prefabPath) && (outfit.mode === "merge_armature" || outfit.mode === "attach_to_bone")
    && typeof outfit.bone === "string" && outfit.bone.length <= 80 && typeof outfit.toggleName === "string" && outfit.toggleName.length <= 80
    && !/[\u0000-\u001f]/u.test(outfit.toggleName) && typeof outfit.active === "boolean");
}
export function isAmfProductionParams(method: unknown, value: unknown): boolean {
  if (!record(value) || value.schemaVersion !== "0.1") return false;
  switch (method) {
    case "amf.production.recipeApplyPlan": case "amf.production.applyRecipe":
      return exact(value, [...(method === "amf.production.recipeApplyPlan" ? ["schemaVersion","runId","recipeId"] : ["schemaVersion","runId","recipeId","planDigest","choice"]), ...("avatarGlobalObjectId" in value ? ["avatarGlobalObjectId"] : [])])
        && (!("avatarGlobalObjectId" in value) || text(value.avatarGlobalObjectId,256))
        && text(value.runId,100) && /^[a-zA-Z0-9-]+$/u.test(value.runId) && text(value.recipeId,100) && /^[a-zA-Z0-9-]+$/u.test(value.recipeId)
        && (method === "amf.production.recipeApplyPlan" || (typeof value.planDigest === "string" && /^[a-f0-9]{64}$/u.test(value.planDigest) && ["recipe_wins","avatar_wins","new_avatar"].includes(String(value.choice))));
    case "amf.production.previewRecipeShare": case "amf.production.importRecipe": return exact(value,["schemaVersion","text"]) && text(value.text,349557);
    case "amf.production.exportRecipe": return exact(value,["schemaVersion","recipeId","planDigest","sources"]) && text(value.recipeId,100) && /^[a-zA-Z0-9-]+$/u.test(value.recipeId)
      && typeof value.planDigest === "string" && /^[a-f0-9]{64}$/u.test(value.planDigest) && record(value.sources) && Object.keys(value.sources).length <= 64
      && Object.entries(value.sources).every(([id,booth]) => /^m-[a-f0-9]{64}$/u.test(id) && typeof booth === "string" && /^[1-9][0-9]{0,19}$/u.test(booth));
    case "amf.production.list": case "amf.production.listRecipes": return exact(value, ["schemaVersion"]);
    case "amf.production.getRecipe": case "amf.production.recipeSharePlan": return exact(value, ["schemaVersion", "recipeId"]) && text(value.recipeId,100) && /^[a-zA-Z0-9-]+$/u.test(value.recipeId);
    case "amf.production.saveRecipe": return exact(value, ["schemaVersion", "runId", "title"]) && text(value.runId,100) && /^[a-zA-Z0-9-]+$/u.test(value.runId)
      && typeof value.title === "string" && value.title.trim().length > 0 && [...value.title].length <= 120 && !/[\u0000-\u001f\u007f-\u009f]/u.test(value.title);
    case "amf.production.previewImage": return exact(value, ["schemaVersion", "runId", "path"]) && text(value.runId,100) && /^[a-zA-Z0-9-]+$/u.test(value.runId)
      && text(value.path,512) && isAmfPreviewPath(value.path);
    case "amf.production.prepareRecipe": return exact(value, ["schemaVersion", "recipeId", "editorPath"]) && text(value.recipeId,100) && /^[a-zA-Z0-9-]+$/u.test(value.recipeId) && text(value.editorPath);
    case "amf.production.get": case "amf.production.reinspect": case "amf.production.open": case "amf.production.rollbackPlan":
      return exact(value, ["schemaVersion", "runId"]) && text(value.runId, 100) && /^[a-zA-Z0-9-]+$/u.test(value.runId);
    case "amf.production.rollback":
      return exact(value, ["schemaVersion", "runId", "planDigest"]) && text(value.runId, 100) && /^[a-zA-Z0-9-]+$/u.test(value.runId)
        && typeof value.planDigest === "string" && /^[a-f0-9]{64}$/u.test(value.planDigest);
    case "amf.production.prepare":
      return exact(value, ["schemaVersion", "sources", "editorPath"]) && text(value.editorPath) && Array.isArray(value.sources) && value.sources.length > 0 && value.sources.length <= 64
        && value.sources.every(source => record(source) && exact(source, ["sourceId", "role"]) && text(source.sourceId, 256) && typeof source.role === "string" && roles.includes(source.role))
        && new Set(value.sources.map((source: AmfSourceChoice) => source.sourceId)).size === value.sources.length;
    case "amf.production.plan": case "amf.production.assemble":
      return exact(value, method === "amf.production.plan" ? ["schemaVersion", "runId", "selection"] : ["schemaVersion", "runId", "selection", "planDigest"])
        && text(value.runId, 100) && /^[a-zA-Z0-9-]+$/u.test(value.runId) && isAmfAssemblySelection(value.selection)
        && (method === "amf.production.plan" || (typeof value.planDigest === "string" && /^[a-f0-9]{64}$/u.test(value.planDigest)));
    default: return false;
  }
}
