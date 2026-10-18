/** Candidate VN3 terms. Queries consume local evidence; refreshes are explicit commands. */
export const TERMS_PRESET_IDS = ["vrchat_personal", "vtuber_individual", "vtuber_corporate", "game_development", "outfit_for_sale", "merchandise_publication", "adult_content"] as const;
export type TermsPresetId = typeof TERMS_PRESET_IDS[number];
export type TermValue = "permitted" | "prohibited" | "contact_licensor" | "unknown";
export const TERM_CONDITIONS = ["content_rating_required", "between_users_only", "commercial_and_non_profit", "credit_preferred", "modified_distinguishable", "non_profit_only", "private_use_only", "private_use_exception", "limited_publication_only"] as const;
export type TermCondition = typeof TERM_CONDITIONS[number];
export interface TermWarning { readonly code: string; readonly item: string | null }
export interface TermItem { readonly item: string; readonly termKey: string; readonly value: TermValue; readonly condition: TermCondition | null; readonly rawText: string; readonly summaryRawText: string | null }
export interface TermsDocumentV01 {
  readonly sha256: string; readonly parserVersion: "vn3-parser/1"; readonly status: "parsed" | "unrecognized" | "unsupported";
  readonly generatorVersion: string | null; readonly vn3Version: string | null; readonly language: "ja" | "en" | null;
  readonly contentTitle: string | null; readonly licensor: string | null; readonly creditNotation: string | null;
  readonly hashtags: readonly string[]; readonly agreementVersion: string | null; readonly specialNote: string | null;
  readonly warnings: readonly TermWarning[]; readonly items: readonly TermItem[];
}
export interface TermsSourceV01 {
  readonly sourceId: string; readonly sourceKind: "drive" | "docs" | "vn3" | "link"; readonly sourceUrl: string; readonly googleFileId: string | null;
  readonly status: "pending" | "link_only" | "parsed" | "unrecognized" | "unsupported" | "not_public" | "failed";
  readonly fetchedAt: string | null; readonly problem: string | null; readonly document: TermsDocumentV01 | null;
}
export interface TermsSpecialNote { readonly productId: string; readonly sourceId: string; readonly language: "ja" | "en" | null; readonly text: string }
export const TERMS_SOURCE_STATES = ["not_observed", "no_vn3", "link_only", "pending", "parsed", "partial", "unrecognized", "unsupported", "not_public", "failed"] as const;
export type TermsSourceState = typeof TERMS_SOURCE_STATES[number];
export interface ProductTermsV01 {
  readonly schemaVersion: "0.1"; readonly productId: string; readonly state: TermsSourceState; readonly observedAt: string | null;
  readonly captureProblem: "source_limit" | null;
  readonly sources: readonly TermsSourceV01[]; readonly effectiveLanguage: "ja" | "en" | null; readonly effectiveItems: readonly TermItem[];
  readonly warnings: readonly TermWarning[]; readonly hasSpecialNote: boolean; readonly specialNotes: readonly TermsSpecialNote[];
}
export interface TermsBlockingItem { readonly productId: string; readonly item: string | null; readonly termKey: string | null; readonly reason: string }
export interface TermsRequiredCondition { readonly productId: string; readonly item: string; readonly condition: TermCondition }
export type TermsDecision = "conforms" | "contact_licensor" | "nonconforming" | "unknown";
export interface TermsUseDecision {
  readonly decision: TermsDecision; readonly blockingItems: readonly TermsBlockingItem[]; readonly conditions: readonly TermsRequiredCondition[];
  readonly advisories: readonly TermsBlockingItem[]; readonly hasSpecialNote: boolean; readonly specialNotes: readonly TermsSpecialNote[];
}
export interface TermsProductUseDecision extends TermsUseDecision { readonly productId: string; readonly sourceState: TermsSourceState }
export interface TermsUseCheckV01 {
  readonly schemaVersion: "0.1"; readonly presetId: TermsPresetId; readonly products: readonly TermsProductUseDecision[];
  readonly matchingProductIds: readonly string[]; readonly combination: TermsUseDecision;
}
export interface TermsAcceptedV01 { readonly schemaVersion: "0.1"; readonly taskId: string }
export interface TermsRefreshReportV01 { readonly schemaVersion: "0.1"; readonly processedProducts: number; readonly parsedDocuments: number; readonly problemDocuments: number; readonly cachedDocuments: number }
export type TermsBodyV01 =
  | { readonly method: "terms.refreshProduct" | "terms.getProduct"; readonly params: { readonly schemaVersion: "0.1"; readonly productId: string } }
  | { readonly method: "terms.refreshLibrary"; readonly params: { readonly schemaVersion: "0.1" } }
  | { readonly method: "terms.checkUse"; readonly params: { readonly schemaVersion: "0.1"; readonly presetId: TermsPresetId; readonly productIds: readonly string[] } };
export type TermsRequestV1 = { readonly schemaVersion: 1; readonly requestId: string } & TermsBodyV01;

const record = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === "object" && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: readonly string[]) => Object.keys(v).length === keys.length && keys.every(k => k in v);
const string = (v: unknown): v is string => typeof v === "string" && v.length <= 2 * 1024 * 1024;
const nullable = (v: unknown): v is string | null => v === null || string(v);
const code = (v: unknown): v is string => typeof v === "string" && /^[a-z_]{1,100}$/u.test(v);
const language = (v: unknown) => v === null || v === "ja" || v === "en";
const letter = (v: unknown): v is string => typeof v === "string" && /^[A-X]$/u.test(v);
const array = (v: unknown, guard: (v: unknown) => boolean, max = 20000): v is unknown[] => Array.isArray(v) && v.length <= max && v.every(guard);
const timestamp = (v: unknown) => v === null || typeof v === "string" && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/u.test(v) && Number.isFinite(Date.parse(v));
export const isTermsProductId = (v: unknown): v is string => typeof v === "string" && /^booth:[1-9][0-9]{0,19}$/u.test(v);
const condition = (v: unknown) => v === null || (TERM_CONDITIONS as readonly unknown[]).includes(v);
const sourceState = (v: unknown) => (TERMS_SOURCE_STATES as readonly unknown[]).includes(v);
const warning = (v: unknown) => record(v) && exact(v, ["code", "item"]) && code(v.code) && (v.item === null || letter(v.item));
const item = (v: unknown): v is TermItem => record(v) && exact(v, ["item", "termKey", "value", "condition", "rawText", "summaryRawText"])
  && letter(v.item) && typeof v.termKey === "string" && new RegExp(`^vn3\\.${v.item}\\.[a-z_]+$`, "u").test(v.termKey)
  && ["permitted", "prohibited", "contact_licensor", "unknown"].includes(String(v.value)) && condition(v.condition) && (v.value !== "unknown" || v.condition === null) && string(v.rawText) && nullable(v.summaryRawText);
const note = (v: unknown): v is TermsSpecialNote => record(v) && exact(v, ["productId", "sourceId", "language", "text"])
  && isTermsProductId(v.productId) && string(v.sourceId) && language(v.language) && string(v.text) && v.text.length > 0;
const blocker = (v: unknown): v is TermsBlockingItem => record(v) && exact(v, ["productId", "item", "termKey", "reason"])
  && isTermsProductId(v.productId) && (v.item === null || letter(v.item)) && nullable(v.termKey) && code(v.reason);
const required = (v: unknown): v is TermsRequiredCondition => record(v) && exact(v, ["productId", "item", "condition"])
  && isTermsProductId(v.productId) && letter(v.item) && v.condition !== null && condition(v.condition);

export function isTermsParams(method: unknown, v: unknown): boolean {
  if (!record(v) || v.schemaVersion !== "0.1") return false;
  if (method === "terms.refreshLibrary") return exact(v, ["schemaVersion"]);
  if (method === "terms.refreshProduct" || method === "terms.getProduct") return exact(v, ["schemaVersion", "productId"]) && isTermsProductId(v.productId);
  return method === "terms.checkUse" && exact(v, ["schemaVersion", "presetId", "productIds"]) && (TERMS_PRESET_IDS as readonly unknown[]).includes(v.presetId)
    && array(v.productIds, isTermsProductId, 500) && v.productIds.length > 0 && new Set(v.productIds).size === v.productIds.length;
}
export function isTermsDocumentV01(v: unknown): v is TermsDocumentV01 {
  return record(v) && exact(v, ["sha256", "parserVersion", "status", "generatorVersion", "vn3Version", "language", "contentTitle", "licensor", "creditNotation", "hashtags", "agreementVersion", "specialNote", "warnings", "items"])
    && typeof v.sha256 === "string" && /^[a-f0-9]{64}$/u.test(v.sha256) && v.parserVersion === "vn3-parser/1" && ["parsed", "unrecognized", "unsupported"].includes(String(v.status))
    && nullable(v.generatorVersion) && nullable(v.vn3Version) && language(v.language) && nullable(v.contentTitle) && nullable(v.licensor) && nullable(v.creditNotation)
    && array(v.hashtags, string) && nullable(v.agreementVersion) && nullable(v.specialNote) && array(v.warnings, warning) && array(v.items, item, 24)
    && (v.status === "parsed" ? ["1.10", "1.11"].includes(String(v.generatorVersion)) && v.vn3Version === "1.10" && v.language !== null && v.items.map(i => (i as TermItem).item).join("") === "ABCDEFGHIJKLMNOPQRSTUVWX" : v.items.length === 0);
}
function source(v: unknown): v is TermsSourceV01 {
  return record(v) && exact(v, ["sourceId", "sourceKind", "sourceUrl", "googleFileId", "status", "fetchedAt", "problem", "document"])
    && string(v.sourceId) && ["drive", "docs", "vn3", "link"].includes(String(v.sourceKind)) && typeof v.sourceUrl === "string" && /^https?:\/\//u.test(v.sourceUrl)
    && nullable(v.googleFileId) && ["pending", "link_only", "parsed", "unrecognized", "unsupported", "not_public", "failed"].includes(String(v.status))
    && timestamp(v.fetchedAt) && (v.problem === null || code(v.problem)) && (v.document === null || isTermsDocumentV01(v.document) && v.document.status === v.status)
    && (v.status !== "parsed" || v.document !== null) && (["drive", "docs"].includes(String(v.sourceKind)) ? typeof v.googleFileId === "string" : v.googleFileId === null && v.status === "link_only");
}
export function isProductTermsV01(v: unknown): v is ProductTermsV01 {
  return record(v) && exact(v, ["schemaVersion", "productId", "state", "observedAt", "captureProblem", "sources", "effectiveLanguage", "effectiveItems", "warnings", "hasSpecialNote", "specialNotes"])
    && v.schemaVersion === "0.1" && isTermsProductId(v.productId) && sourceState(v.state) && timestamp(v.observedAt) && (v.captureProblem === null || v.captureProblem === "source_limit" && v.state === "unsupported") && array(v.sources, source, 256)
    && language(v.effectiveLanguage) && array(v.effectiveItems, item, 24) && array(v.warnings, warning) && array(v.specialNotes, note)
    && typeof v.hasSpecialNote === "boolean" && v.hasSpecialNote === (v.specialNotes.length > 0);
}
const decisionKeys = ["decision", "blockingItems", "conditions", "advisories", "hasSpecialNote", "specialNotes"] as const;
const ranks: Readonly<Record<string, number>> = { conforms: 0, contact_licensor: 1, unknown: 2, nonconforming: 3 };
const rank = (v: unknown) => ranks[String(v)] ?? -1;
function useDecision(v: Record<string, unknown>): boolean {
  return rank(v.decision) >= 0 && array(v.blockingItems, blocker) && array(v.conditions, required) && array(v.advisories, blocker) && array(v.specialNotes, note)
    && typeof v.hasSpecialNote === "boolean" && v.hasSpecialNote === (v.specialNotes.length > 0) && (v.decision !== "conforms" || v.blockingItems.length === 0);
}
export function isTermsUseCheckV01(v: unknown): v is TermsUseCheckV01 {
  if (!record(v) || !exact(v, ["schemaVersion", "presetId", "products", "matchingProductIds", "combination"]) || v.schemaVersion !== "0.1"
    || !(TERMS_PRESET_IDS as readonly unknown[]).includes(v.presetId) || !Array.isArray(v.products) || v.products.length < 1 || v.products.length > 500
    || !array(v.matchingProductIds, isTermsProductId, 500) || !record(v.combination) || !exact(v.combination, decisionKeys) || !useDecision(v.combination)) return false;
  if (!v.products.every((p: unknown) => record(p) && exact(p, ["productId", "sourceState", ...decisionKeys]) && isTermsProductId(p.productId) && sourceState(p.sourceState)
    && useDecision(p) && (p.decision !== "conforms" || p.sourceState === "parsed"))) return false;
  const products = v.products as TermsProductUseDecision[];
  return new Set(products.map(p => p.productId)).size === products.length
    && JSON.stringify(v.matchingProductIds) === JSON.stringify(products.filter(p => p.decision === "conforms").map(p => p.productId))
    && rank(v.combination.decision) === Math.max(...products.map(p => rank(p.decision)))
    && v.combination.hasSpecialNote === products.some(p => p.hasSpecialNote)
    && JSON.stringify(v.combination.specialNotes) === JSON.stringify(products.flatMap(p => p.specialNotes))
    && JSON.stringify(v.combination.conditions) === JSON.stringify(products.flatMap(p => p.conditions))
    && JSON.stringify(v.combination.advisories) === JSON.stringify(products.flatMap(p => p.advisories))
    && JSON.stringify(v.combination.blockingItems) === JSON.stringify(products.flatMap(p => p.blockingItems));
}
export function isTermsAcceptedV01(v: unknown): v is TermsAcceptedV01 {
  return record(v) && exact(v, ["schemaVersion", "taskId"]) && v.schemaVersion === "0.1" && typeof v.taskId === "string" && /^[A-Za-z0-9-]{1,256}$/u.test(v.taskId);
}
export function isTermsRefreshReportV01(v: unknown): v is TermsRefreshReportV01 {
  return record(v) && exact(v, ["schemaVersion", "processedProducts", "parsedDocuments", "problemDocuments", "cachedDocuments"]) && v.schemaVersion === "0.1"
    && [v.processedProducts, v.parsedDocuments, v.problemDocuments, v.cachedDocuments].every(n => typeof n === "number" && Number.isSafeInteger(n) && n >= 0);
}
