/** Candidate Unity license metadata and explicit check; scenario software checks stay separate. */
export type UnityLicenseKind = "classic_entitlement" | "msix_entitlement" | "legacy_ulf";
export type UnityLicenseHint = "likely_active" | "needs_hub_client" | "not_found" | "unknown";
export type UnityLicenseCheckState = "active" | "not_activated" | "needs_hub_client" | "unknown";
export interface UnityLicenseFile { readonly kind: UnityLicenseKind; readonly present: boolean | null; readonly modifiedAt: string | null }
export interface UnityLicenseLastCheck { readonly state: UnityLicenseCheckState; readonly editorVersion: string; readonly checkedAt: string }
export interface UnityLicenseSnapshotV01 {
  readonly schemaVersion: "0.1";
  readonly files: readonly UnityLicenseFile[];
  readonly hubType: "classic" | "msix" | "none";
  readonly hubRunning: boolean | null;
  readonly licensingClientRunning: boolean | null;
  readonly hubCliPresent: boolean | null;
  readonly lastCheck: UnityLicenseLastCheck | null;
  readonly state: UnityLicenseHint;
  readonly nextAction: "none" | "start_hub" | "sign_in_and_activate" | "run_check";
}
export interface UnityLicenseCheckResultV01 extends UnityLicenseLastCheck { readonly schemaVersion: "0.1"; readonly reason: string | null }
export interface UnityLicenseAcceptedV01 { readonly schemaVersion: "0.1"; readonly taskId: string }
export type UnityLicenseBodyV01 =
  | { readonly method: "environment.unityLicense"; readonly params: {readonly schemaVersion: "0.1"} }
  | { readonly method: "environment.checkUnityLicense"; readonly params: {readonly schemaVersion: "0.1"; readonly editorPath: string} };
export type UnityLicenseRequestV1 = {readonly schemaVersion: 1; readonly requestId: string} & UnityLicenseBodyV01;

const record = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === "object" && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: readonly string[]) => Object.keys(v).length === keys.length && keys.every(k => k in v);
const nullableBool = (v: unknown) => v === null || typeof v === "boolean";
const timestamp = (v: unknown): v is string => typeof v === "string" && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?Z$/u.test(v) && Number.isFinite(Date.parse(v));
const editor = (v: unknown) => v === "2022.3.22f1" || v === "2022.3.22f1c1";
const states: readonly unknown[] = ["active", "not_activated", "needs_hub_client", "unknown"];
const checked = (v: unknown): v is UnityLicenseLastCheck => record(v) && states.includes(v.state) && editor(v.editorVersion) && timestamp(v.checkedAt);
const reasons: readonly unknown[] = ["The license check was cancelled.", "Unity could not be started.", "Unity did not finish within five minutes.",
  "Unity's check log could not be read completely.", "The licensing client could not be observed.", "Unity exited without completing the license check."];

export function isUnityLicenseParams(method: unknown, params: unknown): boolean {
  if (!record(params) || params.schemaVersion !== "0.1") return false;
  if (method === "environment.unityLicense") return exact(params, ["schemaVersion"]);
  return method === "environment.checkUnityLicense" && exact(params, ["schemaVersion", "editorPath"])
    && typeof params.editorPath === "string" && params.editorPath.length > 0 && params.editorPath.length <= 4096 && !/[\u0000-\u001f\u007f-\u009f]/u.test(params.editorPath);
}
export function isUnityLicenseSnapshotV01(v: unknown): v is UnityLicenseSnapshotV01 {
  if (!record(v) || !exact(v,["schemaVersion","files","hubType","hubRunning","licensingClientRunning","hubCliPresent","lastCheck","state","nextAction"])
    || v.schemaVersion !== "0.1" || !["classic","msix","none"].includes(String(v.hubType))
    || !nullableBool(v.hubRunning) || !nullableBool(v.licensingClientRunning) || !nullableBool(v.hubCliPresent)
    || (v.lastCheck !== null && !(checked(v.lastCheck) && record(v.lastCheck) && exact(v.lastCheck,["state","editorVersion","checkedAt"])))
    || !Array.isArray(v.files) || v.files.length !== 3) return false;
  const kinds = ["classic_entitlement","msix_entitlement","legacy_ulf"];
  if (!v.files.every((file: unknown, i: number) => record(file) && exact(file,["kind","present","modifiedAt"]) && file.kind === kinds[i]
    && nullableBool(file.present) && (file.modifiedAt === null || timestamp(file.modifiedAt)) && (file.present === true || file.modifiedAt === null))) return false;
  if (v.state === "needs_hub_client") return v.nextAction === "start_hub";
  if (v.state === "not_found") return v.nextAction === "sign_in_and_activate";
  if (v.state === "unknown") return v.nextAction === "run_check";
  if (v.state !== "likely_active") return false;
  return v.nextAction === "run_check" || (v.nextAction === "none" && checked(v.lastCheck) && v.lastCheck.state === "active")
    || (v.nextAction === "sign_in_and_activate" && checked(v.lastCheck) && v.lastCheck.state === "not_activated");
}
export function isUnityLicenseCheckResultV01(v: unknown): v is UnityLicenseCheckResultV01 {
  return record(v) && exact(v,["schemaVersion","state","editorVersion","checkedAt","reason"]) && v.schemaVersion === "0.1" && checked(v)
    && (v.state === "unknown" ? reasons.includes(v.reason) : v.reason === null);
}
export function isUnityLicenseAcceptedV01(v: unknown): v is UnityLicenseAcceptedV01 {
  return record(v) && exact(v,["schemaVersion","taskId"]) && v.schemaVersion === "0.1" && typeof v.taskId === "string"
    && /^[a-zA-Z0-9-]{1,256}$/u.test(v.taskId);
}
