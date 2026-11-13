import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { isApplicationRequestV01 } from "./application-contract.js";
import { isCatalogOrdersPageResultV01, isCatalogOrdersPageV01 } from "./catalog-orders.js";

const vector = (name: string) => JSON.parse(readFileSync(resolve(__dirname, "../../../schemas/catalog-orders/v0.1/examples", name), "utf8"));
const base = { contractVersion: "0.1", requestId: "synthetic-request", correlationId: "synthetic-correlation", kind: "command", commandId: "synthetic-command" };

describe("catalog orders v0.1", () => {
  it("accepts a history page as an application command", () => {
    const request = vector("page.request.json");
    expect(isCatalogOrdersPageV01(request.params)).toBe(true);
    expect(isApplicationRequestV01({ ...base, ...request })).toBe(true);
  });
  it("accepts only the history's own address", () => {
    const request = vector("invalid-page.request.json");
    expect(isCatalogOrdersPageV01(request.params)).toBe(false);
    expect(isApplicationRequestV01({ ...base, ...request })).toBe(false);
    const page = vector("page.request.json").params;
    expect(isCatalogOrdersPageV01({ ...page, sourceUrl: "https://accounts.booth.pm/orders?page=2" })).toBe(true);
    expect(isCatalogOrdersPageV01({ ...page, sourceUrl: "https://accounts.booth.pm/orders/1" })).toBe(false);
    expect(isCatalogOrdersPageV01({ ...page, cookies: "synthetic" })).toBe(false);
  });
  it("requires complete, integer-valued results", () => {
    const result = vector("page.response.json");
    expect(isCatalogOrdersPageResultV01(result)).toBe(true);
    expect(isCatalogOrdersPageResultV01({ ...result, parsedCount: 0.5 })).toBe(false);
    expect(isCatalogOrdersPageResultV01({ ...result, recordedCount: 3 })).toBe(false);
  });
});
