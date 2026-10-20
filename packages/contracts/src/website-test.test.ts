import { describe, expect, it } from "vitest";
import { isWebsiteTestParams, isWebsiteTestResult } from "./website-test.js";
import { isDesktopGatewayRequestV1 } from "./desktop-gateway.js";
import { isApplicationRequestV01 } from "./application-contract.js";

describe("website test contract", () => {
  it("accepts explicit HTTPS destinations through both request boundaries", () => {
    const request = { requestId: "website", method: "environment.testWebsites", params: { urls: ["https://vrchat.com/"] } };
    expect(isDesktopGatewayRequestV1({ ...request, schemaVersion: 1 })).toBe(true);
    expect(isApplicationRequestV01({ ...request, contractVersion: "0.1", correlationId: "website", kind: "query" })).toBe(true);
  });
  it("rejects credentials, non-HTTPS, duplicates, oversized batches and extra authority", () => {
    for (const params of [{ urls: [] }, { urls: ["file:///secret"] }, { urls: ["https://u:p@example.com"] },
      { urls: ["https://github.com", "https://github.com"] }, { urls: ["https://github.com"], headers: { Cookie: "private" } },
      { urls: Array.from({ length: 13 }, (_, n) => `https://site${n}.example/`) }]) expect(isWebsiteTestParams(params)).toBe(false);
  });
  it("does not turn an HTTP denial or missing response into a successful test", () => {
    const row = { url: "https://github.com/", status: "http_error", elapsedMs: 132, httpStatus: 403 };
    expect(isWebsiteTestResult({ websiteTests: [row] })).toBe(true);
    expect(isWebsiteTestResult({ websiteTests: [{ ...row, status: "reachable" }] })).toBe(false);
    expect(isWebsiteTestResult({ websiteTests: [{ ...row, status: "timeout", httpStatus: null }] })).toBe(true);
    expect(isWebsiteTestResult({ websiteTests: [{ ...row, elapsedMs: -1 }] })).toBe(false);
  });
  it("accepts a regional row only with a complete regional measurement", () => {
    const row = { url: "https://objectstorage.ap-tokyo-1.oraclecloud.com/", status: "http_error", elapsedMs: 21, httpStatus: 404 };
    const regional = { medianMs: 28, p90Ms: 35, samples: 9, connections: 3, viaProxy: true };
    expect(isWebsiteTestResult({ websiteTests: [{ ...row, regional }] })).toBe(true);
    for (const malformed of [
      { ...row, regional: { medianMs: 28, p90Ms: 35, samples: 9, connections: 3 } },
      { ...row, regional: { medianMs: 28, p90Ms: 35, samples: 9, connections: 3, viaProxy: "yes" } },
      { ...row, regional: { medianMs: -1, p90Ms: 35, samples: 9, connections: 3, viaProxy: true } },
      { ...row, regional: { medianMs: 28, p90Ms: 35, samples: 0, connections: 3, viaProxy: true } },
      { ...row, regional: { medianMs: 28.5, p90Ms: 35, samples: 9, connections: 3, viaProxy: true } },
      { ...row, regional: null },
      { ...row, detail: regional },
    ]) expect(isWebsiteTestResult({ websiteTests: [malformed] })).toBe(false);
  });
});
