import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { isApplicationRequestV01 } from "./application-contract.js";
import { isDesktopGatewayRequestV1 } from "./desktop-gateway.js";
import { isCatalogEnrichmentParamsV01, isCatalogEnrichmentQueueResultV01, isCatalogEnrichmentBeginResultV01, isCatalogProductObservationResultV01 } from "./catalog-enrichment-v01.js";
const cases = JSON.parse(readFileSync(new URL("../../../schemas/catalog-enrichment/v0.1/cases.json", import.meta.url), "utf8"));
describe("catalog enrichment application boundaries", () => {
  for (const vector of cases.requests) it(vector.name, () => {
    expect(isCatalogEnrichmentParamsV01(vector.request.method, vector.request.params)).toBe(vector.valid);
    expect(isApplicationRequestV01(vector.request)).toBe(vector.valid);
    // Transport/attempt bookkeeping is Main-owned, not an arbitrary renderer command.
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"synthetic",method:vector.request.method,params:vector.request.params})).toBe(false);
  });
  for (const vector of cases.responses) it(`response: ${vector.name}`, () => {
    expect(isCatalogEnrichmentQueueResultV01(vector.value) || isCatalogEnrichmentBeginResultV01(vector.value) || isCatalogProductObservationResultV01(vector.value)).toBe(vector.valid);
  });
  it("enforces UTF-8 bytes, envelope kind and command identity", () => {
    const request = cases.requests.find((v: {name:string}) => v.name === "product-json").request;
    expect(isApplicationRequestV01({...request,params:{...request.params,body:"猫".repeat(1_666_667)}})).toBe(false);
    expect(isApplicationRequestV01({...request,kind:"query"})).toBe(false);
    expect(isApplicationRequestV01({...request,commandId:""})).toBe(false);
  });
});
