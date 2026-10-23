/** Candidate AMF material-first production. Frozen legacy production faces are separate. */
import { isTexturePlatform, isTextureTargetRank, isTexturePath } from "./texture-memory-v01.js";
import type { TexturePlatform, TextureTargetRank } from "./texture-memory-v01.js";
import { isFaceTrackingDigest } from "./face-tracking-v01.js";
export type AmfSourceRole = "auto" | "avatar" | "outfit" | "dependency";
export interface AmfSourceChoice { readonly sourceId: string; readonly role: AmfSourceRole }
export interface AmfOutfitSelection {
  readonly sourceId: string; readonly prefabPath: string;
  readonly mode: "merge_armature" | "attach_to_bone"; readonly bone: string;
  readonly toggleName: string; readonly active: boolean;
}
export interface AmfAssemblySelection { readonly avatarPrefab: string; readonly outfits: readonly AmfOutfitSelection[] }
export type AmfRecipeApplyChoice = "recipe_wins" | "avatar_wins" | "new_avatar";
/** Which state of the line a revision recorded; `restore` is the protected current state. */
export type AmfProductionRevisionKind = "materials" | "assembly" | "rebuild" | "sdk_fix" | "face_tracking" | "editor_save" | "lighting" | "texture_memory" | "restore";
export type AmfProductionCapacity = 1 | 2 | 3;
/** The lilToon brightness limits of one material, as serialized in its file. */
export interface AmfLightingLimits { readonly min: string; readonly max: string }
/** One outfit material whose lilToon brightness limits differ from the Avatar's reference. */
export interface AmfLightingEntry { readonly material: string; readonly sourceId: string; readonly current: AmfLightingLimits; readonly proposed: AmfLightingLimits }
/** A material the lighting plan leaves unchanged, with the reason. */
export interface AmfLightingSkip { readonly material: string; readonly reason: string }
/** Why a lighting plan has no reference: the Avatar has no lilToon materials, or no unique pair wins. */
export type AmfLightingReferenceReason = "no_avatar_liltoon" | "tie";
export interface AmfProductionWorkV01 { readonly taskId: string; readonly runId: string; readonly operation: string }
export interface AmfProductionCapacityResultV01 { readonly schemaVersion: "0.1"; readonly capacity: AmfProductionCapacity }
export interface AmfProductionLaneV01 {
  readonly capacity: AmfProductionCapacity;
  readonly busy: boolean;
  readonly running: readonly AmfProductionWorkV01[];
  readonly queue: readonly AmfProductionWorkV01[];
}
export type AmfProductionBodyV01 =
  | { readonly method: "amf.production.planTextureMemory"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly platform: TexturePlatform; readonly targetRank: TextureTargetRank } }
  | { readonly method: "amf.production.applyTextureMemory"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly platform: TexturePlatform; readonly targetRank: TextureTargetRank; readonly planDigest: string; readonly textures: readonly string[] } }
  | { readonly method: "amf.production.faceTrackingPlan"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly menuId: string } }
  | { readonly method: "amf.production.faceTrackingPatch"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly menuId: string; readonly planDigest: string; readonly confirmTrustedCode: true } }
  | { readonly method: "amf.production.setCapacity"; readonly params: { readonly schemaVersion: "0.1"; readonly capacity: AmfProductionCapacity } }
  | { readonly method: "amf.production.recipeApplyPlan"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly recipeId: string; readonly avatarGlobalObjectId?: string } }
  | { readonly method: "amf.production.applyRecipe"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly recipeId: string; readonly planDigest: string; readonly choice: AmfRecipeApplyChoice; readonly avatarGlobalObjectId?: string } }
  | { readonly method: "amf.production.previewRecipeShare" | "amf.production.importRecipe"; readonly params: { readonly schemaVersion: "0.1"; readonly text: string } }
  | { readonly method: "amf.production.exportRecipe"; readonly params: { readonly schemaVersion: "0.1"; readonly recipeId: string; readonly planDigest: string; readonly sources: Readonly<Record<string,string>> } }
  | { readonly method: "amf.production.list"; readonly params: { readonly schemaVersion: "0.1" } }
  | { readonly method: "amf.production.listRecipes"; readonly params: { readonly schemaVersion: "0.1" } }
  | { readonly method: "amf.production.getRecipe" | "amf.production.recipeSharePlan"; readonly params: { readonly schemaVersion: "0.1"; readonly recipeId: string } }
  | { readonly method: "amf.production.saveRecipe"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly title: string } }
  | { readonly method: "amf.production.previewImage"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly path: string } }
  | { readonly method: "amf.production.releaseState" | "amf.production.bakeTurntable"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string } }
  | { readonly method: "amf.production.turntableFrame"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly index: number } }
  | { readonly method: "amf.production.upload"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly avatarName: string; readonly rightsConfirmed: true } }
  /** Runs the VRChat SDK's own Auto Fix for the chosen items of its Avatar check, named by the SDK's exact words. */
  | { readonly method: "amf.production.closeEditor"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly confirmed: true } }
  | { readonly method: "amf.production.sdkFix"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly fixes: readonly string[] } }
  /** Changes the line's own materials; the line keeps its identity and plans again. */
  | { readonly method: "amf.production.changeSources"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly sources: readonly AmfSourceChoice[] } }
  /** Deletes a line with its work project and release record. */
  | { readonly method: "amf.production.delete"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string } }
  | { readonly method: "amf.production.prepareRecipe"; readonly params: { readonly schemaVersion: "0.1"; readonly recipeId: string; readonly editorPath: string } }
  | { readonly method: "amf.production.get" | "amf.production.reinspect" | "amf.production.open" | "amf.production.rollbackPlan"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string } }
  /** Reviews the lilToon brightness alignment of an assembled line. */
  | { readonly method: "amf.production.planLighting"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string } }
  /** Aligns the chosen reviewed materials' lilToon brightness limits with the Avatar's, behind a restore point. */
  | { readonly method: "amf.production.applyLighting"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly planDigest: string; readonly materials: readonly string[] } }
  | { readonly method: "amf.production.rollback"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly planDigest: string; readonly target?: "restorePoint" | "prePatch" } }
  | { readonly method: "amf.production.revisionPlan"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly revisionId: string } }
  | { readonly method: "amf.production.restoreRevision"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly revisionId: string; readonly planDigest: string } }
  | { readonly method: "amf.production.listFits"; readonly params: { readonly schemaVersion: "0.1" } }
  /** Saves the line's pending manual fit adjustments of one source under a name; synchronous, no task. */
  | { readonly method: "amf.production.saveFit"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly sourceId: string; readonly name: string; readonly changesDigest: string } }
  | { readonly method: "amf.production.renameFit"; readonly params: { readonly schemaVersion: "0.1"; readonly fitId: string; readonly name: string } }
  | { readonly method: "amf.production.deleteFit"; readonly params: { readonly schemaVersion: "0.1"; readonly fitId: string } }
  | { readonly method: "amf.production.prepare"; readonly params: { readonly schemaVersion: "0.1"; readonly sources: readonly AmfSourceChoice[]; readonly editorPath: string } }
  | { readonly method: "amf.production.plan"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly selection: AmfAssemblySelection } }
  | { readonly method: "amf.production.assemble"; readonly params: { readonly schemaVersion: "0.1"; readonly runId: string; readonly planDigest: string; readonly selection: AmfAssemblySelection } };
export interface AmfEditorCloseResult {
  readonly schemaVersion: "0.1"; readonly runId: string; readonly closeId: string;
  readonly outcome: "saved" | "failed"; readonly scenesSaved: boolean; readonly assetsSaved: boolean;
  readonly editorClosed: boolean; readonly reason: string; readonly detail: string;
}
export type AmfProductionRequestV1 = { readonly schemaVersion: 1; readonly requestId: string } & AmfProductionBodyV01;

/** Run phases. `inspecting` covers reinspection ("continue"); `stopped` keeps the work project and its materials. */
export const AMF_RUN_PHASES = ["preparing", "inspecting", "ready", "assembling", "restoring", "succeeded", "failed", "cancelled", "stopped", "inspect_required"] as const;
export type AmfRunPhaseV01 = typeof AMF_RUN_PHASES[number];
/** Phases during which a task works on the run. */
export const AMF_ACTIVE_PHASES: readonly AmfRunPhaseV01[] = ["preparing", "inspecting", "assembling", "restoring"];
/** Recorded steps, in the order a production meets them. */
export const AMF_STEP_KEYS = ["project", "materials", "inspect", "reinspect", "snapshot", "assembly", "protect_current", "restore", "turntable", "upload", "sdk_fix", "face_tracking", "editor_close", "lighting", "texture_memory"] as const;
export type AmfStepKey = typeof AMF_STEP_KEYS[number];
export const AMF_STEP_STATES = ["running", "succeeded", "failed", "stopped", "interrupted"] as const;
export type AmfStepState = typeof AMF_STEP_STATES[number];
/** Stages of a Unity batch run, read from the Editor log, plus `fingerprint`: VUA's own project-file check after the Editor exits. The set is closed. */
export const AMF_UNITY_STAGES = ["starting", "license", "packages", "importing", "compiling", "bridge", "previews", "finishing", "fingerprint"] as const;
export type AmfUnityStage = typeof AMF_UNITY_STAGES[number];
/** Unity's live progress: its stage, imported work-project assets and its last log activity. No log text or paths. */
export interface AmfUnityProgress { readonly stage: AmfUnityStage; readonly current?: number; readonly total?: number; readonly lastActivityAt: string }
export interface AmfStepV01 { readonly key: AmfStepKey; readonly name: string; readonly state: AmfStepState; readonly detail: string; readonly startedAt: string | null; readonly endedAt: string | null }
/** Payload of a task progress event of an `amf.production` task. `step` is `open` while the Editor opens for the SDK. */
export interface AmfProductionProgressV01 {
  readonly operation: "amf.production"; readonly runId: string; readonly step: AmfStepKey | "open"; readonly name: string;
  readonly startedAt: string; readonly unity?: AmfUnityProgress;
}
/** The running step in `amf.production.get`: elapsed and usual seconds on this computer, and Unity's progress inside Unity. */
export interface AmfRunProgressV01 {
  readonly step: AmfStepKey; readonly name: string; readonly startedAt: string; readonly elapsedSeconds: number;
  readonly usualSeconds: number | null; readonly unity?: AmfUnityProgress;
}
/** Median seconds of each step kind that finished on this computer. */
export type AmfUsualDurations = Readonly<Partial<Record<AmfStepKey, number>>>;

const record = (value: unknown): value is Record<string, unknown> => !!value && typeof value === "object" && !Array.isArray(value);
const text = (value: unknown, limit = 4096): value is string => typeof value === "string" && value.length > 0 && value.length <= limit && !value.includes("\0");
const exact = (value: Record<string, unknown>, keys: readonly string[]) => Object.keys(value).length === keys.length && keys.every(key => key in value);
const roles: readonly string[] = ["auto", "avatar", "outfit", "dependency"];

const count = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const runId = (value: unknown): value is string => text(value, 100) && /^[A-Za-z0-9-]+$/u.test(value);
/** A saved fit's identity: `fit-` plus 1 to 72 word characters. */
const fitId = (value: unknown): value is string => typeof value === "string" && /^fit-[A-Za-z0-9_-]{1,72}$/u.test(value);
/** A fit's player-given name: 1 to 80 characters on one line, with a non-whitespace character. */
const fitName = (value: unknown): value is string => typeof value === "string" && value.trim().length > 0 && [...value].length <= 80 && !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
/** A UTC timestamp as VUA writes it, `YYYY-MM-DDTHH:MM:SS.fffZ`. */
export function isAmfTimestamp(value: unknown): value is string {
  return typeof value === "string" && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,9})?Z$/u.test(value) && !Number.isNaN(Date.parse(value));
}
export function isAmfUnityProgress(value: unknown): value is AmfUnityProgress {
  return record(value) && Object.keys(value).every(key => ["stage", "current", "total", "lastActivityAt"].includes(key))
    && (AMF_UNITY_STAGES as readonly unknown[]).includes(value.stage) && isAmfTimestamp(value.lastActivityAt)
    && (value.current === undefined || count(value.current)) && (value.total === undefined || count(value.total));
}
export function isAmfStep(value: unknown): value is AmfStepV01 {
  return record(value) && exact(value, ["key", "name", "state", "detail", "startedAt", "endedAt"])
    && (AMF_STEP_KEYS as readonly unknown[]).includes(value.key) && typeof value.name === "string" && typeof value.detail === "string"
    && (AMF_STEP_STATES as readonly unknown[]).includes(value.state)
    && (value.startedAt === null || isAmfTimestamp(value.startedAt)) && (value.endedAt === null || isAmfTimestamp(value.endedAt));
}
export function isAmfUsualDurations(value: unknown): value is AmfUsualDurations {
  return record(value) && Object.entries(value).every(([key, seconds]) => (AMF_STEP_KEYS as readonly string[]).includes(key) && count(seconds));
}
export function isAmfRunProgress(value: unknown): value is AmfRunProgressV01 {
  return record(value) && Object.keys(value).every(key => ["step", "name", "startedAt", "elapsedSeconds", "usualSeconds", "unity"].includes(key))
    && (AMF_STEP_KEYS as readonly unknown[]).includes(value.step) && text(value.name, 1024) && isAmfTimestamp(value.startedAt)
    && count(value.elapsedSeconds) && (value.usualSeconds === null || count(value.usualSeconds))
    && (value.unity === undefined || isAmfUnityProgress(value.unity));
}
/** Validates optional saving facts retained by a close task, separately from project release. */
export function isAmfEditorCloseResult(value: unknown): value is AmfEditorCloseResult {
  if (!record(value) || !exact(value, ["schemaVersion", "runId", "closeId", "outcome", "scenesSaved", "assetsSaved", "editorClosed", "reason", "detail"])
    || value.schemaVersion !== "0.1" || !runId(value.runId) || !runId(value.closeId)
    || !["saved", "failed"].includes(String(value.outcome)) || typeof value.scenesSaved !== "boolean"
    || typeof value.assetsSaved !== "boolean" || typeof value.editorClosed !== "boolean"
    || typeof value.reason !== "string" || !["", "scene_save_failed", "asset_save_failed", "untitled_scene", "editor_busy", "expired", "cancelled", "save_failed", "timeout"].includes(value.reason)
    || typeof value.detail !== "string" || value.detail.length > 1000 || value.detail.includes("\0")) return false;
  return value.outcome === "saved" ? value.scenesSaved && value.assetsSaved && ["", "timeout"].includes(value.reason) : value.reason !== "";
}
/** Reads the payload of an `amf.production` task progress event; anything else is `null`. */
export function readAmfProductionProgress(payload: unknown): AmfProductionProgressV01 | null {
  if (!record(payload) || payload.operation !== "amf.production"
    || !Object.keys(payload).every(key => ["operation", "runId", "step", "name", "startedAt", "unity"].includes(key))) return null;
  const valid = runId(payload.runId) && ([...AMF_STEP_KEYS, "open"] as readonly unknown[]).includes(payload.step) && text(payload.name, 1024)
    && isAmfTimestamp(payload.startedAt) && (payload.unity === undefined || isAmfUnityProgress(payload.unity));
  return valid ? payload as unknown as AmfProductionProgressV01 : null;
}

/** A Bridge preview inside the work project: `.vua/previews/<command>/<name>.png`. */
export function isAmfPreviewPath(value: unknown): value is string {
  return typeof value === "string" && value.length <= 512 && /^\.vua\/previews\/[A-Za-z0-9_-][A-Za-z0-9_.-]*\/[A-Za-z0-9_-][A-Za-z0-9_.-]*\.png$/u.test(value)
    && !value.split("/").includes("..");
}
/** A material asset inside the work project: `Assets/**` ending in `.mat`. */
export function isAmfMaterialPath(value: unknown): value is string {
  return typeof value === "string" && value.length <= 4096 && value.startsWith("Assets/") && /\.[mM][aA][tT]$/u.test(value)
    && !/[\\:\u0000]/u.test(value) && !value.split("/").some(part => part === "" || part === "." || part === "..");
}
/** SDK items one fix request may name. */
export const AMF_SDK_FIX_LIMIT = 16;
/** Frames in a baked AMF turntable, camera yaw 0° to 354° in 6° steps. */
export const AMF_TURNTABLE_FRAMES = 60;
/** A name VRChat shows for a new Avatar: trimmed, 1 to 64 characters, no control characters. */
export function isAmfAvatarName(value: unknown): value is string {
  return typeof value === "string" && value.trim() === value && [...value].length >= 1 && [...value].length <= 64 && !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
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
    case "amf.production.setCapacity": return exact(value, ["schemaVersion", "capacity"]) && isAmfProductionCapacity(value.capacity);
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
    case "amf.production.releaseState": case "amf.production.bakeTurntable":
      return exact(value, ["schemaVersion", "runId"]) && text(value.runId,100) && /^[a-zA-Z0-9-]+$/u.test(value.runId);
    case "amf.production.turntableFrame": return exact(value, ["schemaVersion", "runId", "index"]) && text(value.runId,100) && /^[a-zA-Z0-9-]+$/u.test(value.runId)
      && Number.isInteger(value.index) && Number(value.index) >= 0 && Number(value.index) < AMF_TURNTABLE_FRAMES;
    // The player confirms the content rights statement in VUA for each upload.
    case "amf.production.upload": return exact(value, ["schemaVersion", "runId", "avatarName", "rightsConfirmed"]) && text(value.runId,100) && /^[a-zA-Z0-9-]+$/u.test(value.runId)
      && isAmfAvatarName(value.avatarName) && value.rightsConfirmed === true;
    case "amf.production.faceTrackingPlan": return exact(value, ["schemaVersion", "runId", "menuId"]) && runId(value.runId) && isFaceTrackingDigest(value.menuId);
    case "amf.production.faceTrackingPatch": return exact(value, ["schemaVersion", "runId", "menuId", "planDigest", "confirmTrustedCode"])
      && runId(value.runId) && isFaceTrackingDigest(value.menuId) && isFaceTrackingDigest(value.planDigest) && value.confirmTrustedCode === true;
    case "amf.production.closeEditor": return exact(value, ["schemaVersion", "runId", "confirmed"]) && runId(value.runId) && value.confirmed === true;
    case "amf.production.sdkFix": return exact(value, ["schemaVersion", "runId", "fixes"]) && text(value.runId,100) && /^[a-zA-Z0-9-]+$/u.test(value.runId)
      && Array.isArray(value.fixes) && value.fixes.length >= 1 && value.fixes.length <= AMF_SDK_FIX_LIMIT
      && value.fixes.every(fix => text(fix, 600)) && new Set(value.fixes).size === value.fixes.length;
    case "amf.production.changeSources":
      return exact(value, ["schemaVersion", "runId", "sources"]) && text(value.runId,100) && /^[a-zA-Z0-9-]+$/u.test(value.runId) && Array.isArray(value.sources) && value.sources.length > 0 && value.sources.length <= 64
        && value.sources.every(source => record(source) && exact(source, ["sourceId", "role"]) && text(source.sourceId, 256) && typeof source.role === "string" && roles.includes(source.role))
        && new Set(value.sources.map((source: AmfSourceChoice) => source.sourceId)).size === value.sources.length;
    case "amf.production.delete": return exact(value, ["schemaVersion", "runId"]) && text(value.runId,100) && /^[a-zA-Z0-9-]+$/u.test(value.runId);
    case "amf.production.prepareRecipe": return exact(value, ["schemaVersion", "recipeId", "editorPath"]) && text(value.recipeId,100) && /^[a-zA-Z0-9-]+$/u.test(value.recipeId) && text(value.editorPath);
    case "amf.production.get": case "amf.production.reinspect": case "amf.production.open": case "amf.production.rollbackPlan": case "amf.production.planLighting":
      return exact(value, ["schemaVersion", "runId"]) && text(value.runId, 100) && /^[a-zA-Z0-9-]+$/u.test(value.runId);
    case "amf.production.planTextureMemory":
      return exact(value, ["schemaVersion", "runId", "platform", "targetRank"]) && runId(value.runId) && isTexturePlatform(value.platform) && isTextureTargetRank(value.targetRank);
    case "amf.production.applyTextureMemory":
      return exact(value, ["schemaVersion", "runId", "platform", "targetRank", "planDigest", "textures"]) && runId(value.runId)
        && isTexturePlatform(value.platform) && isTextureTargetRank(value.targetRank) && isFaceTrackingDigest(value.planDigest)
        && Array.isArray(value.textures) && value.textures.length > 0 && value.textures.length <= 2048
        && value.textures.every(isTexturePath) && new Set(value.textures).size === value.textures.length;
    case "amf.production.applyLighting":
      return exact(value, ["schemaVersion", "runId", "planDigest", "materials"]) && text(value.runId, 100) && /^[a-zA-Z0-9-]+$/u.test(value.runId)
        && typeof value.planDigest === "string" && /^[a-f0-9]{64}$/u.test(value.planDigest)
        && Array.isArray(value.materials) && value.materials.length >= 1 && value.materials.length <= 512
        && value.materials.every(isAmfMaterialPath) && new Set(value.materials).size === value.materials.length;
    case "amf.production.rollback":
      return exact(value, ["schemaVersion", "runId", "planDigest", ...("target" in value ? ["target"] : [])])
        && text(value.runId, 100) && /^[a-zA-Z0-9-]+$/u.test(value.runId)
        && typeof value.planDigest === "string" && /^[a-f0-9]{64}$/u.test(value.planDigest)
        && (!("target" in value) || ["restorePoint", "prePatch"].includes(String(value.target)));
    case "amf.production.revisionPlan":
      return exact(value, ["schemaVersion", "runId", "revisionId"]) && text(value.runId, 100) && /^[a-zA-Z0-9-]+$/u.test(value.runId)
        && text(value.revisionId, 128) && /^[A-Za-z0-9_-]+$/u.test(value.revisionId);
    case "amf.production.restoreRevision":
      return exact(value, ["schemaVersion", "runId", "revisionId", "planDigest"]) && text(value.runId, 100) && /^[a-zA-Z0-9-]+$/u.test(value.runId)
        && text(value.revisionId, 128) && /^[A-Za-z0-9_-]+$/u.test(value.revisionId)
        && typeof value.planDigest === "string" && /^[a-f0-9]{64}$/u.test(value.planDigest);
    case "amf.production.listFits": return exact(value, ["schemaVersion"]);
    case "amf.production.saveFit":
      return exact(value, ["schemaVersion", "runId", "sourceId", "name", "changesDigest"]) && text(value.runId, 100) && /^[a-zA-Z0-9-]+$/u.test(value.runId)
        && text(value.sourceId, 256) && fitName(value.name)
        && typeof value.changesDigest === "string" && /^[a-f0-9]{64}$/u.test(value.changesDigest);
    case "amf.production.renameFit":
      return exact(value, ["schemaVersion", "fitId", "name"]) && fitId(value.fitId) && fitName(value.name);
    case "amf.production.deleteFit":
      return exact(value, ["schemaVersion", "fitId"]) && fitId(value.fitId);
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

export function isAmfProductionCapacity(value: unknown): value is AmfProductionCapacity {
  return value === 1 || value === 2 || value === 3;
}
