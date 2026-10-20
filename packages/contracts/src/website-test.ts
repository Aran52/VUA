/** User-triggered website header tests. Separate from the legacy regional report. */

/** Statistics of a successful regional reference measurement; ordinary website
 * rows never carry this. `elapsedMs` on the same row holds the minimum sample. */
export interface RegionalMeasurement {
  readonly medianMs: number;
  readonly p90Ms: number;
  readonly samples: number;
  readonly connections: number;
  readonly viaProxy: boolean;
}
export interface WebsiteObservation {
  readonly url: string;
  readonly status: "reachable" | "http_error" | "redirected" | "timeout" | "connection_failed" | "probe_error";
  readonly elapsedMs: number;
  readonly httpStatus: number | null;
  readonly regional?: RegionalMeasurement;
}
export interface WebsiteTestParams { readonly urls: readonly string[] }
export interface WebsiteTestResult { readonly websiteTests: readonly WebsiteObservation[] }
export const WEBSITE_TEST_LIMIT = 12;

export function isRegionalMeasurement(value: unknown): value is RegionalMeasurement {
  if (!value || typeof value !== "object" || Object.keys(value).length !== 5) return false;
  const { medianMs, p90Ms, samples, connections, viaProxy } = value as Record<string, unknown>;
  const whole = (candidate: unknown, minimum: number): candidate is number =>
    typeof candidate === "number" && Number.isSafeInteger(candidate) && candidate >= minimum;
  return whole(medianMs, 0) && whole(p90Ms, 0) && whole(samples, 1) && whole(connections, 1)
    && typeof viaProxy === "boolean";
}

/** Only credential-free HTTPS destinations; no commands, browser sessions or headers. */
export function isWebsiteTestUrl(value: unknown): value is string {
  if (typeof value !== "string" || value.length > 2048 || value.trim() !== value) return false;
  try {
    const url = new URL(value);
    return url.protocol === "https:" && !!url.hostname && !url.username && !url.password
      && !url.hash && (!url.port || url.port === "443");
  } catch { return false; }
}
export function isWebsiteTestParams(value: unknown): value is WebsiteTestParams {
  if (!value || typeof value !== "object" || Object.keys(value).length !== 1 || !("urls" in value)) return false;
  const urls = value.urls;
  return Array.isArray(urls) && urls.length > 0 && urls.length <= WEBSITE_TEST_LIMIT
    && urls.every(isWebsiteTestUrl) && new Set(urls).size === urls.length;
}
export function isWebsiteTestResult(value: unknown): value is WebsiteTestResult {
  if (!value || typeof value !== "object" || Object.keys(value).length !== 1 || !("websiteTests" in value)) return false;
  const rows = value.websiteTests;
  return Array.isArray(rows) && rows.length > 0 && rows.length <= WEBSITE_TEST_LIMIT && rows.every(row => {
    if (!row || typeof row !== "object") return false;
    const keys = Object.keys(row);
    const withRegional = keys.length === 5 && "regional" in row && isRegionalMeasurement(row.regional);
    if ((keys.length !== 4 && !withRegional) || !isWebsiteTestUrl(row.url)
      || !Number.isSafeInteger(row.elapsedMs) || row.elapsedMs < 0) return false;
    if (row.status === "reachable") return Number.isInteger(row.httpStatus) && row.httpStatus >= 200 && row.httpStatus < 300;
    if (row.status === "redirected") return Number.isInteger(row.httpStatus) && row.httpStatus >= 300 && row.httpStatus < 400;
    if (row.status === "http_error") return Number.isInteger(row.httpStatus) && row.httpStatus >= 400 && row.httpStatus < 600;
    return ["timeout", "connection_failed", "probe_error"].includes(row.status) && row.httpStatus === null;
  });
}
