/** BOOTH's purchase history pages: when bought products were ordered (catalog-orders v0.1). */
export interface CatalogOrdersPageV01 {
  readonly schemaVersion: "0.1";
  readonly sourceUrl: string;
  readonly html: string;
}

export interface CatalogOrdersPageResultV01 {
  readonly schemaVersion: "0.1";
  readonly sourceUrl: string;
  readonly parsedCount: number;
  readonly recordedCount: number;
  readonly nextPageUrl: string | null;
}

const record = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const exact = (value: Record<string, unknown>, keys: readonly string[]) => Object.keys(value).length === keys.length && keys.every(key => Object.hasOwn(value, key));
const count = (value: unknown): value is number => Number.isSafeInteger(value) && (value as number) >= 0;

export const isCatalogOrdersUrl = (value: unknown): value is string =>
  typeof value === "string" && /^https:\/\/accounts\.booth\.pm\/orders(\?page=[1-9][0-9]*)?$/.test(value);

export function isCatalogOrdersPageV01(value: unknown): value is CatalogOrdersPageV01 {
  return record(value) && exact(value, ["schemaVersion", "sourceUrl", "html"]) && value.schemaVersion === "0.1"
    && isCatalogOrdersUrl(value.sourceUrl) && typeof value.html === "string" && value.html.length > 0;
}

export function isCatalogOrdersPageResultV01(value: unknown): value is CatalogOrdersPageResultV01 {
  return record(value) && exact(value, ["schemaVersion", "sourceUrl", "parsedCount", "recordedCount", "nextPageUrl"]) && value.schemaVersion === "0.1"
    && typeof value.sourceUrl === "string" && value.sourceUrl.length > 0 && count(value.parsedCount) && count(value.recordedCount)
    && value.recordedCount <= value.parsedCount && (value.nextPageUrl === null || typeof value.nextPageUrl === "string" && value.nextPageUrl.length > 0);
}
