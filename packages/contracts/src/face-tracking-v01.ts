/** Source observations only. None of these fields proves runtime tracking or built SDK cost. */
export interface FaceTrackingMenuV01 {
  readonly id: string; readonly sourceId: string; readonly scriptPath: string; readonly menuPath: string;
}
export interface FaceTrackingProfileV01 {
  readonly id: "arkit52-v1" | "unified-core12-v1" | "sranipal-core12-v1";
  readonly referenceCount: number; readonly matched: readonly string[];
}
export interface FaceTrackingFactsV01 {
  readonly schemaVersion: "0.1"; readonly evidence: "unity_source"; readonly observedAt: string;
  readonly state: "no_avatar" | "observed" | "partial"; readonly runtime: "unknown"; readonly builtParameterBudget: "unknown";
  readonly truncated: boolean; readonly manualReplayRequired: boolean;
  readonly expressionParametersObserved: boolean; readonly sourceSyncedParameterBits: number;
  readonly meshes: readonly { readonly renderer: string; readonly blendShapeCount: number; readonly missingMesh: boolean; readonly profiles: readonly FaceTrackingProfileV01[] }[];
  readonly parameters: readonly string[]; readonly components: readonly string[]; readonly patchMenus: readonly FaceTrackingMenuV01[];
}
export interface FaceTrackingPlanV01 {
  readonly schemaVersion: "0.1"; readonly runId: string; readonly revision: number; readonly planDigest: string;
  readonly projectFingerprint: string; readonly unityFingerprint: string; readonly menu: FaceTrackingMenuV01;
  readonly avatarGlobalObjectId: string; readonly execution: "interactive_editor"; readonly replay: "manual";
  readonly requiresTrustedCodeConfirmation: true; readonly warning: string;
}
const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: readonly string[]) => Object.keys(v).length === keys.length && keys.every(k => k in v);
const plain = (v: unknown, max: number): v is string => typeof v === "string" && [...v].length <= max && !/[\u0000-\u001f\u007f-\u009f]/u.test(v);
const text = (v: unknown, max: number): v is string => plain(v, max) && v.length > 0;
const count = (v: unknown, max: number): v is number => typeof v === "number" && Number.isSafeInteger(v) && v >= 0 && v <= max;
const names = (v: unknown, max: number, length: number): v is string[] => Array.isArray(v) && v.length <= max && v.every(s => text(s, length)) && new Set(v).size === v.length;
export const isFaceTrackingDigest = (v: unknown): v is string => typeof v === "string" && /^[a-f0-9]{64}$/u.test(v);
export function isFaceTrackingMenu(v: unknown): v is FaceTrackingMenuV01 {
  return record(v) && exact(v, ["id", "sourceId", "scriptPath", "menuPath"]) && isFaceTrackingDigest(v.id)
    && text(v.sourceId, 100) && text(v.scriptPath, 1024) && v.scriptPath.startsWith("Assets/") && /\.cs$/iu.test(v.scriptPath)
    && !/[\\:]/u.test(v.scriptPath) && v.scriptPath.split("/").every(s => s !== "" && s !== "." && s !== "..")
    && text(v.menuPath, 400) && v.menuPath.includes("/") && v.menuPath.split("/").every(s => s.length > 0)
    && !["File", "Edit", "Assets", "GameObject", "Component", "Window", "Help", "CONTEXT", "VRChat SDK"].includes(v.menuPath.split("/")[0] ?? "")
    && !/[%#&]/u.test(v.menuPath) && !v.menuPath.includes(" _");
}
export function isFaceTrackingFacts(v: unknown): v is FaceTrackingFactsV01 {
  return record(v) && exact(v, ["schemaVersion", "evidence", "observedAt", "state", "runtime", "builtParameterBudget",
    "truncated", "manualReplayRequired", "expressionParametersObserved", "sourceSyncedParameterBits", "meshes", "parameters", "components", "patchMenus"])
    && v.schemaVersion === "0.1" && v.evidence === "unity_source" && text(v.observedAt, 64)
    && /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{1,9})?Z$/u.test(v.observedAt) && !Number.isNaN(Date.parse(v.observedAt))
    && ["no_avatar", "observed", "partial"].includes(String(v.state)) && v.runtime === "unknown" && v.builtParameterBudget === "unknown"
    && typeof v.truncated === "boolean" && (!v.truncated || v.state !== "observed") && typeof v.manualReplayRequired === "boolean"
    && typeof v.expressionParametersObserved === "boolean" && count(v.sourceSyncedParameterBits, 65536)
    && (v.expressionParametersObserved || v.sourceSyncedParameterBits === 0)
    && names(v.parameters, 1024, 256) && names(v.components, 64, 256)
    && Array.isArray(v.meshes) && v.meshes.length <= 128 && v.meshes.every(mesh => record(mesh)
      && exact(mesh, ["renderer", "blendShapeCount", "missingMesh", "profiles"]) && plain(mesh.renderer, 1024)
      && count(mesh.blendShapeCount, 65536) && typeof mesh.missingMesh === "boolean" && (!mesh.missingMesh || mesh.blendShapeCount === 0)
      && Array.isArray(mesh.profiles) && mesh.profiles.length === 3 && new Set(mesh.profiles.map(p => record(p) ? p.id : null)).size === 3
      && mesh.profiles.every(p => record(p) && exact(p, ["id", "referenceCount", "matched"])
        && ["arkit52-v1", "unified-core12-v1", "sranipal-core12-v1"].includes(String(p.id))
        && p.referenceCount === (p.id === "arkit52-v1" ? 52 : 12) && names(p.matched, Number(p.referenceCount), 100)
        && p.matched.length <= Number(mesh.blendShapeCount)))
    && Array.isArray(v.patchMenus) && v.patchMenus.length <= 64 && v.patchMenus.every(isFaceTrackingMenu)
    && new Set(v.patchMenus.map(m => m.id)).size === v.patchMenus.length;
}
export function isFaceTrackingPlan(v: unknown): v is FaceTrackingPlanV01 {
  return record(v) && exact(v, ["schemaVersion", "runId", "revision", "planDigest", "projectFingerprint", "unityFingerprint",
    "menu", "avatarGlobalObjectId", "execution", "replay", "requiresTrustedCodeConfirmation", "warning"])
    && v.schemaVersion === "0.1" && text(v.runId, 100) && count(v.revision, Number.MAX_SAFE_INTEGER)
    && isFaceTrackingDigest(v.planDigest) && text(v.projectFingerprint, 256) && text(v.unityFingerprint, 256)
    && isFaceTrackingMenu(v.menu) && text(v.avatarGlobalObjectId, 256)
    && v.execution === "interactive_editor" && v.replay === "manual" && v.requiresTrustedCodeConfirmation === true && text(v.warning, 2048);
}
