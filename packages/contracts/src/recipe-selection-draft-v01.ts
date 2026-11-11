export interface RecipeSelectionV01 {
  readonly identity: string; readonly displayName: string; readonly source: "cloud" | "local";
  readonly variantName?: string | null; readonly shopName?: string | null; readonly warehouseItemId?: string | null;
}
export interface RecipeDraftListParamsV01 { readonly schemaVersion: "0.1" }
export interface RecipeDraftGetParamsV01 { readonly schemaVersion: "0.1"; readonly draftId: string }
export interface RecipeDraftSaveParamsV01 extends RecipeDraftGetParamsV01 {
  readonly title: string; readonly baseRevision: number; readonly selections: readonly RecipeSelectionV01[];
}
export interface RecipeDraftAddParamsV01 extends RecipeDraftGetParamsV01 {
  readonly baseRevision: number; readonly selections: readonly RecipeSelectionV01[];
}
export interface RecipeSelectionDraftV01 {
  readonly schemaVersion: "0.1"; readonly kind: "selection_draft"; readonly draftId: string;
  readonly title: string; readonly createdAt: string; readonly selections: readonly RecipeSelectionV01[];
}
export interface RecipeDraftReadV01 {
  readonly schemaVersion: "0.1"; readonly document: RecipeSelectionDraftV01; readonly revision: number; readonly updatedAt: string;
  readonly addedCount?: number; readonly existingCount?: number;
}
export interface RecipeDraftListV01 {
  readonly schemaVersion: "0.1"; readonly entries: readonly { readonly draftId: string; readonly title: string;
    readonly revision: number; readonly updatedAt: string; readonly selectionCount: number }[];
}
const record = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: readonly string[], optional: readonly string[] = []): boolean => keys.every((key) => key in v) && Object.keys(v).every((key) => keys.includes(key) || optional.includes(key));
const integer = (v: unknown): v is number => typeof v === "number" && Number.isSafeInteger(v) && v >= 0;
const bounded = (v: unknown, max: number): v is string => typeof v === "string" && v.trim().length > 0 && Array.from(v).length <= max;
const stamp = (v: unknown): boolean => typeof v === "string" && Number.isFinite(Date.parse(v));
export const isRecipeSelectionDraftId = (v: unknown): v is string => typeof v === "string" && /^recipe-draft-[A-Za-z0-9_-]{1,100}$/.test(v);
export function isRecipeSelectionV01(v: unknown): v is RecipeSelectionV01 {
  if (!record(v) || !exact(v, ["identity", "displayName", "source"], ["variantName", "shopName", "warehouseItemId"]) || !bounded(v.displayName, 512)
    || typeof v.identity !== "string" || ![v.variantName, v.shopName].every((v) => v === undefined || v === null || typeof v === "string" && Array.from(v).length <= 512)
    || !(v.warehouseItemId === undefined || v.warehouseItemId === null || typeof v.warehouseItemId === "string" && /^[A-Za-z0-9_.:-]{1,128}$/.test(v.warehouseItemId))) return false;
  return v.source === "cloud" ? /^booth:[0-9]+$/.test(v.identity) && (v.warehouseItemId === undefined || v.warehouseItemId === null)
    : v.source === "local" && /^sha256:[a-f0-9]{64}$/.test(v.identity);
}
export function isRecipeDraftParamsV01(method: string, v: unknown): boolean {
  if (!record(v) || v.schemaVersion !== "0.1") return false;
  if (method === "recipeDraft.list" || method === "recipeDraft.legacyCensus" || method === "recipeDraft.migrateLegacy") return exact(v, ["schemaVersion"]);
  if (!isRecipeSelectionDraftId(v.draftId)) return false;
  if (method === "recipeDraft.get" || method === "recipeDraft.selectionStatus" || method === "recipeDraft.materialsForLine") return exact(v, ["schemaVersion", "draftId"]);
  if (!integer(v.baseRevision) || v.baseRevision >= Number.MAX_SAFE_INTEGER || !Array.isArray(v.selections) || v.selections.length > 512 || !v.selections.every(isRecipeSelectionV01)) return false;
  if (method === "recipeDraft.addSelection") return exact(v, ["schemaVersion", "draftId", "baseRevision", "selections"]);
  return method === "recipeDraft.save" && exact(v, ["schemaVersion", "draftId", "title", "baseRevision", "selections"]) && bounded(v.title, 120);
}
export function isRecipeDraftReadV01(v: unknown): v is RecipeDraftReadV01 {
  if (!record(v) || !exact(v, ["schemaVersion", "document", "revision", "updatedAt"], ["addedCount", "existingCount"]) || v.schemaVersion !== "0.1"
    || !integer(v.revision) || v.revision === 0 || !stamp(v.updatedAt) || !record(v.document)) return false;
  const d = v.document;
  return exact(d, ["schemaVersion", "kind", "draftId", "title", "createdAt", "selections"]) && d.schemaVersion === "0.1" && d.kind === "selection_draft"
    && isRecipeSelectionDraftId(d.draftId) && bounded(d.title, 120) && stamp(d.createdAt) && Array.isArray(d.selections) && d.selections.length <= 512 && d.selections.every(isRecipeSelectionV01)
    && (v.addedCount === undefined && v.existingCount === undefined || integer(v.addedCount) && integer(v.existingCount) && v.addedCount + v.existingCount <= 512 && v.addedCount <= d.selections.length);
}
export function isRecipeDraftListV01(v: unknown): v is RecipeDraftListV01 {
  return record(v) && exact(v, ["schemaVersion", "entries"]) && v.schemaVersion === "0.1" && Array.isArray(v.entries)
    && v.entries.every((entry) => record(entry) && exact(entry, ["draftId", "title", "revision", "updatedAt", "selectionCount"])
      && isRecipeSelectionDraftId(entry.draftId) && bounded(entry.title, 120) && integer(entry.revision) && entry.revision > 0 && stamp(entry.updatedAt) && integer(entry.selectionCount) && entry.selectionCount <= 512);
}
export interface RecipeDraftSelectionStatusV01 {
  readonly schemaVersion: "0.1"; readonly draftId: string; readonly revision: number;
  readonly items: readonly { readonly index: number; readonly storedCopies: number; readonly presentCopies: number; readonly state: "present" | "missing" | "not_stored" }[];
}
export function isRecipeDraftSelectionStatusV01(v: unknown): v is RecipeDraftSelectionStatusV01 {
  return record(v) && exact(v, ["schemaVersion", "draftId", "revision", "items"]) && v.schemaVersion === "0.1" && isRecipeSelectionDraftId(v.draftId)
    && integer(v.revision) && v.revision > 0 && Array.isArray(v.items) && v.items.length <= 512 && v.items.every((item, index) => record(item)
      && exact(item, ["index", "storedCopies", "presentCopies", "state"]) && item.index === index && integer(item.storedCopies) && integer(item.presentCopies)
      && item.presentCopies <= item.storedCopies && item.state === (item.presentCopies > 0 ? "present" : item.storedCopies === 0 ? "not_stored" : "missing"));
}
export type RecipeDraftMissingReasonV01 = "not_stored" | "missing" | "content_changed" | "no_package";
export interface RecipeDraftLineSourceV01 {
  readonly sourceId: string; readonly itemId: string; readonly itemName: string; readonly name: string;
  readonly productIds: readonly string[]; readonly selectionIndices: readonly number[];
}
export interface RecipeDraftMaterialsForLineV01 {
  readonly schemaVersion: "0.1"; readonly draftId: string; readonly revision: number;
  readonly sources: readonly RecipeDraftLineSourceV01[];
  readonly missing: readonly { readonly index: number; readonly reason: RecipeDraftMissingReasonV01 }[];
}
const MISSING_REASONS: readonly string[] = ["not_stored", "missing", "content_changed", "no_package"];
const UNRESOLVED_REASONS: readonly string[] = [
  "entity_only", "unsupported_provider", "invalid_product_id", "entry_not_found",
  "missing_local_copy", "content_changed", "unparseable_document", "unsupported_version",
];
const strictlyIncreasingIndices = (v: unknown, maxLen: number): v is readonly number[] =>
  Array.isArray(v) && v.length <= maxLen && v.every((idx, i) => integer(idx) && idx < 512 && (i === 0 || idx > (v[i - 1] as number)));
export function isRecipeDraftMaterialsForLineV01(v: unknown): v is RecipeDraftMaterialsForLineV01 {
  if (!record(v) || !exact(v, ["schemaVersion", "draftId", "revision", "sources", "missing"]) || v.schemaVersion !== "0.1"
    || !isRecipeSelectionDraftId(v.draftId) || !integer(v.revision) || v.revision === 0
    || !Array.isArray(v.sources) || v.sources.length > 4096 || !Array.isArray(v.missing) || v.missing.length > 512) return false;
  const missingSet = new Set<number>();
  for (let i = 0; i < v.missing.length; i += 1) {
    const item = v.missing[i];
    if (!record(item) || !exact(item, ["index", "reason"]) || !integer(item.index) || item.index >= 512
      || typeof item.reason !== "string" || !MISSING_REASONS.includes(item.reason)
      || (i > 0 && item.index <= (v.missing[i - 1] as { index: number }).index)) return false;
    missingSet.add(item.index);
  }
  return v.sources.every((s) => record(s) && exact(s, ["sourceId", "itemId", "itemName", "name", "productIds", "selectionIndices"])
    && typeof s.sourceId === "string" && /^[A-Za-z0-9_.:-]{1,160}$/.test(s.sourceId)
    && typeof s.itemId === "string" && /^[A-Za-z0-9_.:-]{1,128}$/.test(s.itemId)
    && bounded(s.itemName, 512) && bounded(s.name, 2048)
    && Array.isArray(s.productIds) && s.productIds.length <= 128 && s.productIds.every((p) => typeof p === "string" && /^booth:[0-9]+$/.test(p))
    && strictlyIncreasingIndices(s.selectionIndices, 512) && s.selectionIndices.length > 0
    && s.selectionIndices.every((idx) => !missingSet.has(idx)));
}
export type RecipeLegacyUnresolvedReasonV01 =
  | "entity_only" | "unsupported_provider" | "invalid_product_id" | "entry_not_found"
  | "missing_local_copy" | "content_changed" | "unparseable_document" | "unsupported_version";
export interface RecipeLegacyUnresolvedAssetV01 { readonly assetId: string; readonly reason: RecipeLegacyUnresolvedReasonV01 }
export interface RecipeDraftLegacyCensusDocumentV01 {
  readonly recipeId: string; readonly revision: number | null; readonly formatVersion: string | null;
  readonly parseable: boolean; readonly targetDraftId: string; readonly existingDraftRevision: number | null;
  readonly verifiableSelections: number; readonly unresolvedAssets: readonly RecipeLegacyUnresolvedAssetV01[];
  readonly hasGraphSettings: boolean;
}
export interface RecipeDraftLegacyCensusV01 {
  readonly schemaVersion: "0.1"; readonly totalDocuments: number; readonly parseableDocuments: number;
  readonly migratedDrafts: number; readonly documents: readonly RecipeDraftLegacyCensusDocumentV01[];
}
export type RecipeDraftLegacyMigrationStatusV01 = "created" | "updated" | "unchanged" | "unresolved_only" | "failed";
export interface RecipeDraftLegacyMigrationItemV01 {
  readonly recipeId: string; readonly targetDraftId: string; readonly status: RecipeDraftLegacyMigrationStatusV01;
  readonly draftRevision: number | null; readonly migratedSelections: number;
  readonly unresolvedAssets: readonly RecipeLegacyUnresolvedAssetV01[]; readonly hasGraphSettings: boolean;
}
export interface RecipeDraftLegacyMigrationV01 {
  readonly schemaVersion: "0.1"; readonly totalDocuments: number; readonly migratedCount: number;
  readonly unchangedCount: number; readonly unresolvedOnlyCount: number; readonly failedCount: number;
  readonly items: readonly RecipeDraftLegacyMigrationItemV01[];
}
const isUnresolvedAsset = (v: unknown): v is RecipeLegacyUnresolvedAssetV01 =>
  record(v) && exact(v, ["assetId", "reason"]) && bounded(v.assetId, 64)
  && typeof v.reason === "string" && UNRESOLVED_REASONS.includes(v.reason);
const isLegacyRecipeId = (v: unknown): v is string => typeof v === "string" && /^[A-Za-z0-9_-]{1,128}$/.test(v);
export function isRecipeDraftLegacyCensusV01(v: unknown): v is RecipeDraftLegacyCensusV01 {
  if (!record(v) || !exact(v, ["schemaVersion", "totalDocuments", "parseableDocuments", "migratedDrafts", "documents"])
    || v.schemaVersion !== "0.1" || !integer(v.totalDocuments) || !integer(v.parseableDocuments) || !integer(v.migratedDrafts)
    || !Array.isArray(v.documents) || v.documents.length !== v.totalDocuments) return false;
  let parseable = 0; let migrated = 0;
  for (const doc of v.documents) {
    if (!record(doc) || !exact(doc, ["recipeId", "revision", "formatVersion", "parseable", "targetDraftId", "existingDraftRevision", "verifiableSelections", "unresolvedAssets", "hasGraphSettings"])
      || !isLegacyRecipeId(doc.recipeId) || !(doc.revision === null || integer(doc.revision) && doc.revision > 0)
      || !(doc.formatVersion === null || bounded(doc.formatVersion, 32)) || typeof doc.parseable !== "boolean"
      || !isRecipeSelectionDraftId(doc.targetDraftId) || !(doc.existingDraftRevision === null || integer(doc.existingDraftRevision) && doc.existingDraftRevision > 0)
      || !integer(doc.verifiableSelections) || doc.verifiableSelections > 512
      || !Array.isArray(doc.unresolvedAssets) || doc.unresolvedAssets.length > 512 || !doc.unresolvedAssets.every(isUnresolvedAsset)
      || typeof doc.hasGraphSettings !== "boolean") return false;
    if (doc.parseable) parseable += 1;
    if (doc.existingDraftRevision !== null) migrated += 1;
  }
  return parseable === v.parseableDocuments && migrated === v.migratedDrafts;
}
export function isRecipeDraftLegacyMigrationV01(v: unknown): v is RecipeDraftLegacyMigrationV01 {
  if (!record(v) || !exact(v, ["schemaVersion", "totalDocuments", "migratedCount", "unchangedCount", "unresolvedOnlyCount", "failedCount", "items"])
    || v.schemaVersion !== "0.1" || !integer(v.totalDocuments) || !integer(v.migratedCount) || !integer(v.unchangedCount)
    || !integer(v.unresolvedOnlyCount) || !integer(v.failedCount) || !Array.isArray(v.items) || v.items.length !== v.totalDocuments
    || v.migratedCount + v.unchangedCount + v.unresolvedOnlyCount + v.failedCount !== v.totalDocuments) return false;
  let migrated = 0; let unchanged = 0; let unresolvedOnly = 0; let failed = 0;
  for (const item of v.items) {
    if (!record(item) || !exact(item, ["recipeId", "targetDraftId", "status", "draftRevision", "migratedSelections", "unresolvedAssets", "hasGraphSettings"])
      || !isLegacyRecipeId(item.recipeId) || !isRecipeSelectionDraftId(item.targetDraftId)
      || !integer(item.migratedSelections) || item.migratedSelections > 512
      || !Array.isArray(item.unresolvedAssets) || item.unresolvedAssets.length > 512 || !item.unresolvedAssets.every(isUnresolvedAsset)
      || typeof item.hasGraphSettings !== "boolean") return false;
    if (item.status === "created" || item.status === "updated") {
      if (!integer(item.draftRevision) || item.draftRevision === 0 || item.migratedSelections === 0) return false;
      migrated += 1;
    } else if (item.status === "unchanged") {
      if (!integer(item.draftRevision) || item.draftRevision === 0) return false;
      unchanged += 1;
    } else if (item.status === "unresolved_only") {
      if (item.draftRevision !== null || item.migratedSelections !== 0 || item.unresolvedAssets.length === 0) return false;
      unresolvedOnly += 1;
    } else if (item.status === "failed") {
      if (item.draftRevision !== null || item.migratedSelections !== 0) return false;
      failed += 1;
    } else {
      return false;
    }
  }
  return migrated === v.migratedCount && unchanged === v.unchangedCount && unresolvedOnly === v.unresolvedOnlyCount && failed === v.failedCount;
}
