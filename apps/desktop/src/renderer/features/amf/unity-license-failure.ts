const CODE = "vua.amf_production.unity_license_unavailable";
export function isUnityLicenseFailure(error: string | null | undefined): boolean {
  return error?.startsWith(CODE) ?? false;
}
/** Keep the structured license code when a command refuses before creating a task. */
export function productionErrorMessage(error: unknown, fallback: string): string {
  if (!(error instanceof Error)) return fallback;
  return "code" in error && typeof error.code === "string" && isUnityLicenseFailure(error.code)
    ? `${error.code} — detail: ${error.message}` : error.message;
}
