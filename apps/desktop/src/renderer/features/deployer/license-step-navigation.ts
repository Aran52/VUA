const KEY = "vua-unity-license-step-request";
export interface LicenseStepRequest { readonly runId?: string }
function readRequest(value: unknown): LicenseStepRequest | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (Object.keys(record).some(key => key !== "runId")) return null;
  if (record.runId === undefined) return {};
  return typeof record.runId === "string" && /^[a-zA-Z0-9-]{1,100}$/u.test(record.runId) ? { runId: record.runId } : null;
}
/** A one-use view hint; it grants no installation, task or production authority. */
export function requestLicenseStep(request: LicenseStepRequest = {}): void {
  try {
    const valid = readRequest(request);
    if (valid) sessionStorage.setItem(KEY, JSON.stringify(valid));
    else sessionStorage.removeItem(KEY);
  } catch { /* The scenario remains available through Avatar editing. */ }
}
export function consumeLicenseStep(): LicenseStepRequest | null {
  try {
    const raw = sessionStorage.getItem(KEY); sessionStorage.removeItem(KEY);
    return raw === null ? null : readRequest(JSON.parse(raw));
  } catch { return null; }
}
