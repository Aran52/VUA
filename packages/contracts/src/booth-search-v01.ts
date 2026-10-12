/** Public BOOTH search is provenance discovery, never an account/entitlement query. */
export interface BoothSearchParamsV01 { readonly schemaVersion: "0.1"; readonly query: string; readonly sourceUrl: string; readonly html: string }
export interface BoothSearchItemV01 {
  readonly productId: string;
  readonly title: string;
  readonly shopName: string | null;
  readonly sourceCategory: string | null;
}
export interface BoothSearchResultV01 {
  readonly schemaVersion: "0.1";
  readonly query: string;
  readonly items: readonly BoothSearchItemV01[];
  readonly truncated: boolean;
}
export const isBoothSearchQueryV01 = (v: unknown): v is string => typeof v === "string"
  && v.trim() === v && Array.from(v).length > 0 && Array.from(v).length <= 200 && !/[\x00-\x1f\x7f-\x9f\ud800-\udfff]/u.test(v);
export function isBoothSearchParamsV01(v: unknown): v is BoothSearchParamsV01 {
  if (typeof v !== "object" || v === null || Array.isArray(v)) return false;
  const p = v as Record<string, unknown>;
  return Object.keys(p).sort().join() === "html,query,schemaVersion,sourceUrl" && p.schemaVersion === "0.1"
    && isBoothSearchQueryV01(p.query) && p.sourceUrl === `https://booth.pm/ja/search/${encodeURIComponent(p.query)}`
    && typeof p.html === "string" && new TextEncoder().encode(p.html).length <= 5_000_000;
}
export function isBoothSearchResultV01(v: unknown): v is BoothSearchResultV01 {
  if (typeof v !== "object" || v === null || Array.isArray(v)) return false;
  const r = v as Record<string, unknown>;
  const nullableText = (x: unknown) => x === null || typeof x === "string" && Array.from(x).length > 0 && Array.from(x).length <= 500;
  return Object.keys(r).sort().join() === "items,query,schemaVersion,truncated" && r.schemaVersion === "0.1"
    && isBoothSearchQueryV01(r.query) && typeof r.truncated === "boolean" && Array.isArray(r.items) && r.items.length <= 24
    && r.items.every((item: unknown) => {
      if (typeof item !== "object" || item === null || Array.isArray(item)) return false;
      const i = item as Record<string, unknown>;
      return Object.keys(i).sort().join() === "productId,shopName,sourceCategory,title"
        && typeof i.productId === "string" && /^booth:[0-9]+$/.test(i.productId)
        && typeof i.title === "string" && Array.from(i.title).length > 0 && Array.from(i.title).length <= 1000
        && nullableText(i.shopName) && nullableText(i.sourceCategory);
    }) && new Set(r.items.map((i: { productId: string }) => i.productId)).size === r.items.length;
}
