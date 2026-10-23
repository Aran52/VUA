/** Candidate AMF removal inventory. Paths label the review; GlobalObjectIds bind deletion. */
export interface OutfitRemovalRoot { readonly kind: "outfit" | "menu"; readonly globalObjectId: string }
export interface OutfitRemovalObject { readonly globalObjectId: string; readonly path: string; readonly typeName: string; readonly manual: boolean }
export interface OutfitRemoval {
  readonly sourceId: string; readonly avatarGlobalObjectId: string;
  readonly roots: readonly OutfitRemovalRoot[]; readonly objects: readonly OutfitRemovalObject[];
  readonly blockedReason: "" | "ownership_invalid" | "target_unverified" | "contains_owned_object" | "inventory_limit" | "identity_invalid";
}
export interface OutfitRemovalApplied { readonly sourceId: string; readonly avatarGlobalObjectId: string; readonly deletedObjectIds: readonly string[] }
export interface OutfitRemovalPlan { readonly schemaVersion: "0.1"; readonly runId: string; readonly planDigest: string; readonly removal: OutfitRemoval }
const record = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === "object" && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: readonly string[]) => Object.keys(v).length === keys.length && keys.every(k => k in v);
const text = (v: unknown, max: number, empty = false): v is string => typeof v === "string" && (empty || v.length > 0) && [...v].length <= max && !v.includes("\0");
export function isOutfitRemoval(v: unknown): v is OutfitRemoval {
  if (!record(v) || !exact(v, ["sourceId", "avatarGlobalObjectId", "roots", "objects", "blockedReason"])
    || !text(v.sourceId, 256) || !text(v.avatarGlobalObjectId, 256)
    || typeof v.blockedReason !== "string" || !["", "ownership_invalid", "target_unverified", "contains_owned_object", "inventory_limit", "identity_invalid"].includes(v.blockedReason)
    || !Array.isArray(v.roots) || v.roots.length > 2
    || !v.roots.every(r => record(r) && exact(r, ["kind", "globalObjectId"]) && typeof r.kind === "string" && ["outfit", "menu"].includes(r.kind) && text(r.globalObjectId, 256))
    || !Array.isArray(v.objects) || v.objects.length > 8192
    || !v.objects.every(o => record(o) && exact(o, ["globalObjectId", "path", "typeName", "manual"])
      && text(o.globalObjectId, 256) && text(o.path, 4096, true) && text(o.typeName, 256) && typeof o.manual === "boolean")) return false;
  const roots = v.roots as OutfitRemovalRoot[], objects = v.objects as OutfitRemovalObject[];
  if (new Set(roots.map(r => r.globalObjectId)).size !== roots.length || new Set(objects.map(o => o.globalObjectId)).size !== objects.length) return false;
  return v.blockedReason !== "" || (roots.filter(r => r.kind === "outfit").length === 1 && roots.filter(r => r.kind === "menu").length <= 1
    && !objects.some(o => o.globalObjectId === v.avatarGlobalObjectId)
    && roots.every(r => objects.some(o => o.globalObjectId === r.globalObjectId && o.typeName === "UnityEngine.GameObject")));
}
export function isOutfitRemovalFacts(v: unknown): v is readonly OutfitRemoval[] {
  return Array.isArray(v) && v.length <= 64 && v.every(isOutfitRemoval) && new Set(v.map(e => e.sourceId)).size === v.length;
}
export function isOutfitRemovalApplied(v: unknown): v is OutfitRemovalApplied {
  return record(v) && exact(v, ["sourceId", "avatarGlobalObjectId", "deletedObjectIds"])
    && text(v.sourceId, 256) && text(v.avatarGlobalObjectId, 256) && Array.isArray(v.deletedObjectIds)
    && v.deletedObjectIds.length > 0 && v.deletedObjectIds.length <= 8192 && v.deletedObjectIds.every(id => text(id, 256))
    && new Set(v.deletedObjectIds).size === v.deletedObjectIds.length;
}
export function isOutfitRemovalPlan(v: unknown): v is OutfitRemovalPlan {
  return record(v) && exact(v, ["schemaVersion", "runId", "planDigest", "removal"]) && v.schemaVersion === "0.1"
    && text(v.runId, 100) && /^[a-zA-Z0-9-]+$/u.test(v.runId) && typeof v.planDigest === "string" && /^[a-f0-9]{64}$/u.test(v.planDigest)
    && isOutfitRemoval(v.removal) && v.removal.blockedReason === "";
}
