import {
  isTermsProductId,
  TERMS_PRESET_IDS,
  TERMS_SOURCE_STATES,
  TERM_CONDITIONS,
  type TermsBlockingItem,
  type TermsDecision,
  type TermsRequiredCondition,
  type TermsSourceState,
  type TermsSpecialNote,
  type TermsUseDecision,
} from "./terms-v01.js";

/** Candidate VN3 terms v0.2: cross-platform / PH-R format-conversion presets
 * (`I` + `C` + `J`, plus `E` for shared spaces) and deterministic `platformRestrictionHint`.
 * Leaves terms v0.1 completely untouched. */
export const PHR_TERMS_PRESET_IDS = [
  "phr_personal_avatar",
  "phr_object_head",
  "phr_shared_space",
] as const;
export type PhrTermsPresetId = typeof PHR_TERMS_PRESET_IDS[number];

export const TERMS_PRESET_IDS_V02 = [
  ...TERMS_PRESET_IDS,
  ...PHR_TERMS_PRESET_IDS,
] as const;
export type TermsPresetIdV02 = typeof TERMS_PRESET_IDS_V02[number];

export interface TermsUseCheckParamsV02 {
  readonly schemaVersion: "0.2";
  readonly presetId: TermsPresetIdV02;
  readonly productIds: readonly string[];
}

export interface TermsUseDecisionV02 extends TermsUseDecision {
  readonly platformRestrictionHint: boolean;
}

export interface TermsProductUseDecisionV02 extends TermsUseDecisionV02 {
  readonly productId: string;
  readonly sourceState: TermsSourceState;
}

export interface TermsUseCheckV02 {
  readonly schemaVersion: "0.2";
  readonly presetId: TermsPresetIdV02;
  readonly products: readonly TermsProductUseDecisionV02[];
  readonly matchingProductIds: readonly string[];
  readonly combination: TermsUseDecisionV02;
}

export interface TermsCheckUseGatewayRequestV2 {
  readonly schemaVersion: 1;
  readonly requestId: string;
  readonly method: "terms.checkUse";
  readonly params: TermsUseCheckParamsV02;
}

const record = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === "object" && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: readonly string[]) =>
  Object.keys(v).length === keys.length && keys.every(k => k in v);
const string = (v: unknown): v is string => typeof v === "string" && v.length <= 2 * 1024 * 1024;
const nullable = (v: unknown): v is string | null => v === null || string(v);
const code = (v: unknown): v is string => typeof v === "string" && /^[a-z_]{1,100}$/u.test(v);
const language = (v: unknown) => v === null || v === "ja" || v === "en";
const letter = (v: unknown): v is string => typeof v === "string" && /^[A-X]$/u.test(v);
const array = (v: unknown, guard: (v: unknown) => boolean, max = 20000): v is unknown[] =>
  Array.isArray(v) && v.length <= max && v.every(guard);
const condition = (v: unknown) => v === null || (TERM_CONDITIONS as readonly unknown[]).includes(v);
const sourceState = (v: unknown) => (TERMS_SOURCE_STATES as readonly unknown[]).includes(v);

const note = (v: unknown): v is TermsSpecialNote =>
  record(v) && exact(v, ["productId", "sourceId", "language", "text"])
  && isTermsProductId(v.productId) && string(v.sourceId) && language(v.language) && string(v.text) && v.text.length > 0;
const blocker = (v: unknown): v is TermsBlockingItem =>
  record(v) && exact(v, ["productId", "item", "termKey", "reason"])
  && isTermsProductId(v.productId) && (v.item === null || letter(v.item)) && nullable(v.termKey) && code(v.reason);
const required = (v: unknown): v is TermsRequiredCondition =>
  record(v) && exact(v, ["productId", "item", "condition"])
  && isTermsProductId(v.productId) && letter(v.item) && v.condition !== null && condition(v.condition);

export function isPhrTermsPresetId(v: unknown): v is PhrTermsPresetId {
  return (PHR_TERMS_PRESET_IDS as readonly unknown[]).includes(v);
}

export function isTermsUseCheckParamsV02(v: unknown): v is TermsUseCheckParamsV02 {
  return record(v)
    && exact(v, ["schemaVersion", "presetId", "productIds"])
    && v.schemaVersion === "0.2"
    && (TERMS_PRESET_IDS_V02 as readonly unknown[]).includes(v.presetId)
    && array(v.productIds, isTermsProductId, 500)
    && v.productIds.length > 0
    && new Set(v.productIds).size === v.productIds.length;
}

const decisionKeysV02 = [
  "decision",
  "blockingItems",
  "conditions",
  "advisories",
  "hasSpecialNote",
  "specialNotes",
  "platformRestrictionHint",
] as const;

const ranks: Readonly<Record<TermsDecision, number>> = {
  conforms: 0,
  contact_licensor: 1,
  unknown: 2,
  nonconforming: 3,
};
const rank = (v: unknown) => (ranks as Readonly<Record<string, number>>)[String(v)] ?? -1;

function useDecisionV02(v: Record<string, unknown>): boolean {
  return rank(v.decision) >= 0
    && array(v.blockingItems, blocker)
    && array(v.conditions, required)
    && array(v.advisories, blocker)
    && array(v.specialNotes, note)
    && typeof v.hasSpecialNote === "boolean"
    && v.hasSpecialNote === (v.specialNotes.length > 0)
    && typeof v.platformRestrictionHint === "boolean"
    && (v.decision !== "conforms" || v.blockingItems.length === 0);
}

export function isTermsUseCheckV02(v: unknown): v is TermsUseCheckV02 {
  if (
    !record(v)
    || !exact(v, ["schemaVersion", "presetId", "products", "matchingProductIds", "combination"])
    || v.schemaVersion !== "0.2"
    || !(TERMS_PRESET_IDS_V02 as readonly unknown[]).includes(v.presetId)
    || !Array.isArray(v.products)
    || v.products.length < 1
    || v.products.length > 500
    || !array(v.matchingProductIds, isTermsProductId, 500)
    || !record(v.combination)
    || !exact(v.combination, decisionKeysV02)
    || !useDecisionV02(v.combination)
  ) {
    return false;
  }
  if (!v.products.every((p: unknown) =>
    record(p)
    && exact(p, ["productId", "sourceState", ...decisionKeysV02])
    && isTermsProductId(p.productId)
    && sourceState(p.sourceState)
    && useDecisionV02(p)
    && (p.decision !== "conforms" || p.sourceState === "parsed")
  )) {
    return false;
  }
  const products = v.products as TermsProductUseDecisionV02[];
  return new Set(products.map(p => p.productId)).size === products.length
    && JSON.stringify(v.matchingProductIds) === JSON.stringify(products.filter(p => p.decision === "conforms").map(p => p.productId))
    && rank(v.combination.decision) === Math.max(...products.map(p => rank(p.decision)))
    && v.combination.hasSpecialNote === products.some(p => p.hasSpecialNote)
    && v.combination.platformRestrictionHint === products.some(p => p.platformRestrictionHint)
    && JSON.stringify(v.combination.specialNotes) === JSON.stringify(products.flatMap(p => p.specialNotes))
    && JSON.stringify(v.combination.conditions) === JSON.stringify(products.flatMap(p => p.conditions))
    && JSON.stringify(v.combination.advisories) === JSON.stringify(products.flatMap(p => p.advisories))
    && JSON.stringify(v.combination.blockingItems) === JSON.stringify(products.flatMap(p => p.blockingItems));
}
