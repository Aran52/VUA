/** The assembly plan's estimate of the synced expression-parameter bits the selected Avatar and
 *  outfits would use, against VRChat's 256-bit limit. Modular Avatar merges parameters only at
 *  build time, so this is source evidence, never the built cost the SDK reports. */
export interface AmfParameterBudgetSourceV01 {
  readonly kind: "expression_parameters" | "ma_parameters";
  readonly origin: "avatar_base" | "selected_outfit";
  readonly name: string;
  readonly bits: number;
}
export interface AmfParameterBudgetPlanV01 {
  readonly schemaVersion: "0.1";
  readonly evidence: "unity_source_estimate";
  readonly limitBits: 256;
  readonly state: "fits" | "near_limit" | "over_limit" | "unknown";
  readonly estimateBits: number;
  readonly unresolvedPrefixConfigs: number;
  readonly sources: readonly AmfParameterBudgetSourceV01[];
}
const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: readonly string[]) => Object.keys(v).length === keys.length && keys.every(k => k in v);
const text = (v: unknown, max: number): v is string => typeof v === "string" && v.length > 0 && [...v].length <= max && !/[\u0000-\u001f\u007f-\u009f]/u.test(v);
const count = (v: unknown, max: number): v is number => typeof v === "number" && Number.isSafeInteger(v) && v >= 0 && v <= max;
const source = (v: unknown): v is AmfParameterBudgetSourceV01 => record(v) && exact(v, ["kind", "origin", "name", "bits"])
  && ["expression_parameters", "ma_parameters"].includes(String(v.kind))
  && ["avatar_base", "selected_outfit"].includes(String(v.origin))
  && text(v.name, 4096) && count(v.bits, 65536);
export function isAmfParameterBudgetPlan(v: unknown): v is AmfParameterBudgetPlanV01 {
  if (!(record(v) && exact(v, ["schemaVersion", "evidence", "limitBits", "state", "estimateBits", "unresolvedPrefixConfigs", "sources"])
    && v.schemaVersion === "0.1" && v.evidence === "unity_source_estimate" && v.limitBits === 256
    && ["fits", "near_limit", "over_limit", "unknown"].includes(String(v.state))
    && count(v.estimateBits, 65536) && count(v.unresolvedPrefixConfigs, 4096)
    && Array.isArray(v.sources) && v.sources.length <= 66 && v.sources.every(source))) return false;
  if (v.state === "unknown") return v.estimateBits === 0 && v.unresolvedPrefixConfigs === 0 && v.sources.length === 0;
  // Every counted bit belongs to exactly one source row, so the rows explain the estimate.
  return v.sources.reduce((total: number, row) => total + row.bits, 0) === v.estimateBits;
}
