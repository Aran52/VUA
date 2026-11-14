import { isTermsProductId } from "./terms-v01.js";

/** Candidate glb-inspection v0.1: AMF-owned pure-Rust streaming inspection of
 * warehouse .glb / .vrm artifacts, VRMC_vrm 1.0 humanoid readiness and PH-R
 * Stage 1 MR Object-Head >= 5 cm clearance. */
export const GLB_INSPECTION_SCHEMA = "vua.glb-inspection/v0.1" as const;

export const GLB_FORMATS_V01 = [
  "vrm_1_0",
  "vrm_0_x",
  "glb_2_0",
  "invalid_glb",
] as const;
export type GlbFormatV01 = typeof GLB_FORMATS_V01[number];

export const OBJECT_HEAD_CLEARANCE_STATUSES_V01 = [
  "passes",
  "insufficient_margin",
  "likely_unscaled",
  "unknown",
] as const;
export type ObjectHeadClearanceStatusV01 = typeof OBJECT_HEAD_CLEARANCE_STATUSES_V01[number];

export const PHR_READINESS_VALUES_V01 = [
  "ready_vrm1_humanoid",
  "ready_object_head",
  "vrm0_needs_upgrade",
  "clearance_insufficient",
  "scale_suspect",
  "missing_head_bone",
  "invalid_glb",
] as const;
export type PhrReadinessV01 = typeof PHR_READINESS_VALUES_V01[number];

export const REFERENCE_HEAD_HMD_METERS_V01: readonly [number, number, number] = [0.22, 0.26, 0.28];

export type InspectGlbParamsV01 =
  | { readonly schemaVersion: "0.1"; readonly productId: string }
  | { readonly schemaVersion: "0.1"; readonly entryId: string };

export interface GlbVrmSummaryV01 {
  readonly specVersion: string | null;
  readonly hasHumanoidHeadBone: boolean;
  readonly boneCount: number;
  readonly modelName: string | null;
  readonly authors: readonly string[];
  readonly avatarPermission: string | null;
  readonly commercialUsage: string | null;
  readonly modification: string | null;
  readonly licenseUrl: string | null;
}

export interface GlbBoundsMetersV01 {
  readonly size: readonly [number, number, number];
  readonly min: readonly [number, number, number];
  readonly max: readonly [number, number, number];
}

export interface ObjectHeadClearanceV01 {
  readonly status: ObjectHeadClearanceStatusV01;
  readonly referenceHeadHmdMeters: readonly [number, number, number];
  readonly marginsMeters: readonly [number, number, number] | null;
}

export interface GlbInspectedFileV01 {
  readonly copyId: string;
  readonly fileName: string;
  readonly sizeBytes: number;
  readonly format: GlbFormatV01;
  readonly extensionsUsed: readonly string[];
  readonly vrm: GlbVrmSummaryV01 | null;
  readonly boundsMeters: GlbBoundsMetersV01 | null;
  readonly objectHeadClearance: ObjectHeadClearanceV01;
  readonly phrReadiness: PhrReadinessV01;
}

export interface GlbInspectionResultV01 {
  readonly schemaVersion: typeof GLB_INSPECTION_SCHEMA;
  readonly items: readonly GlbInspectedFileV01[];
  readonly truncated: boolean;
}

export interface InspectGlbGatewayRequestV1 {
  readonly schemaVersion: 1;
  readonly requestId: string;
  readonly method: "library.inspectGlb";
  readonly params: InspectGlbParamsV01;
}

function record(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function exactKeys(v: unknown, expected: readonly string[]): v is Record<string, unknown> {
  return record(v) && Object.keys(v).length === expected.length && expected.every(k => Object.hasOwn(v, k));
}

function nullableString(v: unknown, max: number): v is string | null {
  return v === null || (typeof v === "string" && v.length <= max);
}

function vec3(v: unknown, nonNegative = false): v is readonly [number, number, number] {
  return Array.isArray(v)
    && v.length === 3
    && v.every(n => typeof n === "number" && Number.isFinite(n) && (!nonNegative || n >= 0));
}

export function isInspectGlbParamsV01(v: unknown): v is InspectGlbParamsV01 {
  if (!record(v) || v.schemaVersion !== "0.1") return false;
  const keys = Object.keys(v);
  if (keys.length === 2 && Object.hasOwn(v, "schemaVersion") && Object.hasOwn(v, "productId")) {
    return isTermsProductId(v.productId);
  }
  if (keys.length === 2 && Object.hasOwn(v, "schemaVersion") && Object.hasOwn(v, "entryId")) {
    return typeof v.entryId === "string"
      && v.entryId.length >= 1
      && v.entryId.length <= 128
      && !/[\\/]/u.test(v.entryId);
  }
  return false;
}

function isVrmSummary(v: unknown): v is GlbVrmSummaryV01 {
  if (!exactKeys(v, [
    "specVersion",
    "hasHumanoidHeadBone",
    "boneCount",
    "modelName",
    "authors",
    "avatarPermission",
    "commercialUsage",
    "modification",
    "licenseUrl",
  ])) return false;
  return nullableString(v.specVersion, 32)
    && typeof v.hasHumanoidHeadBone === "boolean"
    && typeof v.boneCount === "number"
    && Number.isSafeInteger(v.boneCount)
    && v.boneCount >= 0
    && v.boneCount <= 512
    && nullableString(v.modelName, 256)
    && Array.isArray(v.authors)
    && v.authors.length <= 32
    && v.authors.every(a => typeof a === "string" && a.length <= 256)
    && nullableString(v.avatarPermission, 64)
    && nullableString(v.commercialUsage, 64)
    && nullableString(v.modification, 64)
    && nullableString(v.licenseUrl, 2048);
}

function isSafeRelativeFileName(v: unknown): v is string {
  if (typeof v !== "string" || v.length < 1 || v.length > 512) return false;
  // Never expose Windows drive prefixes, UNC paths, or parent-directory traversal.
  if (/^[A-Za-z]:/u.test(v) || v.startsWith("\\\\") || v.startsWith("/") || v.includes("..\\") || v.includes("../")) {
    return false;
  }
  return !v.includes("\\");
}

function isGlbInspectedFile(v: unknown): v is GlbInspectedFileV01 {
  if (!exactKeys(v, [
    "copyId",
    "fileName",
    "sizeBytes",
    "format",
    "extensionsUsed",
    "vrm",
    "boundsMeters",
    "objectHeadClearance",
    "phrReadiness",
  ])) return false;
  if (typeof v.copyId !== "string" || v.copyId.length < 1 || v.copyId.length > 128) return false;
  if (!isSafeRelativeFileName(v.fileName)) return false;
  if (typeof v.sizeBytes !== "number" || !Number.isSafeInteger(v.sizeBytes) || v.sizeBytes < 0) return false;
  if (!GLB_FORMATS_V01.includes(v.format as GlbFormatV01)) return false;
  if (!Array.isArray(v.extensionsUsed) || v.extensionsUsed.length > 64) return false;
  if (!v.extensionsUsed.every(e => typeof e === "string" && e.length >= 1 && e.length <= 128)) return false;
  if (v.vrm !== null && !isVrmSummary(v.vrm)) return false;
  if (v.boundsMeters !== null) {
    if (!exactKeys(v.boundsMeters, ["size", "min", "max"])) return false;
    if (!vec3(v.boundsMeters.size, true) || !vec3(v.boundsMeters.min) || !vec3(v.boundsMeters.max)) return false;
  }
  if (!exactKeys(v.objectHeadClearance, ["status", "referenceHeadHmdMeters", "marginsMeters"])) return false;
  const c = v.objectHeadClearance;
  if (!OBJECT_HEAD_CLEARANCE_STATUSES_V01.includes(c.status as ObjectHeadClearanceStatusV01)) return false;
  if (!vec3(c.referenceHeadHmdMeters, true)) return false;
  if (c.marginsMeters !== null && !vec3(c.marginsMeters)) return false;
  return PHR_READINESS_VALUES_V01.includes(v.phrReadiness as PhrReadinessV01);
}

export function isGlbInspectionResultV01(v: unknown): v is GlbInspectionResultV01 {
  if (!exactKeys(v, ["schemaVersion", "items", "truncated"])) return false;
  if (v.schemaVersion !== GLB_INSPECTION_SCHEMA || typeof v.truncated !== "boolean") return false;
  if (!Array.isArray(v.items) || v.items.length > 32) return false;
  return v.items.every(isGlbInspectedFile);
}
