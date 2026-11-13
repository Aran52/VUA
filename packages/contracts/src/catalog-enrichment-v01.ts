/** Private Main/AMF transport for bounded BOOTH metadata enrichment. */
export interface CatalogEnrichmentQueueV01 { readonly schemaVersion: "0.1"; readonly limit: number }
export interface CatalogEnrichmentBeginV01 { readonly schemaVersion: "0.1"; readonly productId: string }
export interface CatalogProductObservationV01 {
  readonly schemaVersion: "0.1";
  readonly productId: string;
  readonly format: "json" | "html";
  readonly sourceUrl: string;
  readonly body: string;
  readonly fetchedAt: string;
}
export interface CatalogEnrichmentQueueResultV01 { readonly schemaVersion: "0.1"; readonly productIds: readonly string[] }
export interface CatalogEnrichmentBeginResultV01 { readonly schemaVersion: "0.1"; readonly productId: string; readonly claimed: boolean }
export interface CatalogProductObservationResultV01 { readonly schemaVersion: "0.1"; readonly productId: string }
export type CatalogEnrichmentResultV01 = CatalogEnrichmentQueueResultV01 | CatalogEnrichmentBeginResultV01 | CatalogProductObservationResultV01;

const record = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: readonly string[]) => Object.keys(v).sort().join() === [...keys].sort().join();
export const isCatalogProductIdV01 = (v: unknown): v is string => typeof v === "string" && /^booth:[0-9]{1,20}$/.test(v);
export function isCatalogTimestampV01(v: unknown): v is string {
  return typeof v === "string" && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?Z$/.test(v)
    && Number.isFinite(Date.parse(v)) && new Date(v).toISOString().slice(0, 19) === v.slice(0, 19);
}
export function isCatalogEnrichmentParamsV01(method: string, v: unknown): boolean {
  if (!record(v) || v.schemaVersion !== "0.1") return false;
  if (method === "catalog.enrichmentQueue") return exact(v, ["schemaVersion", "limit"]) && Number.isInteger(v.limit) && Number(v.limit) >= 1 && Number(v.limit) <= 60;
  if (!isCatalogProductIdV01(v.productId)) return false;
  if (method === "catalog.beginEnrichment") return exact(v, ["schemaVersion", "productId"]);
  if (method !== "catalog.observeProduct" || !exact(v, ["schemaVersion", "productId", "format", "sourceUrl", "body", "fetchedAt"])) return false;
  if (v.format !== "json" && v.format !== "html" || typeof v.sourceUrl !== "string") return false;
  const match = /^https:\/\/booth\.pm\/(ja|en|ko|zh-cn|zh-tw)\/items\/([0-9]{1,20})(\.json)?$/.exec(v.sourceUrl);
  return !!match && `booth:${match[2]}` === v.productId && !!match[3] === (v.format === "json")
    && typeof v.body === "string" && v.body.length > 0 && new TextEncoder().encode(v.body).length <= 5_000_000
    && isCatalogTimestampV01(v.fetchedAt);
}
export function isCatalogEnrichmentQueueResultV01(v: unknown): v is CatalogEnrichmentQueueResultV01 {
  return record(v) && exact(v, ["schemaVersion", "productIds"]) && v.schemaVersion === "0.1"
    && Array.isArray(v.productIds) && v.productIds.length <= 60 && v.productIds.every(isCatalogProductIdV01)
    && new Set(v.productIds).size === v.productIds.length;
}
export function isCatalogEnrichmentBeginResultV01(v: unknown): v is CatalogEnrichmentBeginResultV01 {
  return record(v) && exact(v, ["schemaVersion", "productId", "claimed"]) && v.schemaVersion === "0.1"
    && isCatalogProductIdV01(v.productId) && typeof v.claimed === "boolean";
}
export function isCatalogProductObservationResultV01(v: unknown): v is CatalogProductObservationResultV01 {
  return record(v) && exact(v, ["schemaVersion", "productId"]) && v.schemaVersion === "0.1" && isCatalogProductIdV01(v.productId);
}
