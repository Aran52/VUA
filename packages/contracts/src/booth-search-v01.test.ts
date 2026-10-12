import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { isApplicationRequestV01 } from "./application-contract.js";
import { isDesktopGatewayRequestV1, DESKTOP_GATEWAY_METHOD_KINDS } from "./desktop-gateway.js";
import { isBoothSearchParamsV01, isBoothSearchQueryV01, isBoothSearchResultV01 } from "./booth-search-v01.js";
const vector = (name: string) => JSON.parse(readFileSync(new URL(`../../../schemas/booth-search/v0.1/examples/${name}.json`, import.meta.url), "utf8"));
describe("public BOOTH search contract consumers", () => {
  for (const name of ["public.request", "invalid-source.request", "invalid-empty.request"]) it(name, () => {
    const request = vector(name); const valid = !name.startsWith("invalid");
    expect(isBoothSearchParamsV01(request.params)).toBe(valid);
    expect(isApplicationRequestV01(request)).toBe(valid);
    expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: request.requestId, method: request.method, params: request.params })).toBe(valid);
    expect(DESKTOP_GATEWAY_METHOD_KINDS[request.method as "catalog.searchProducts"]).toBe("query");
  });
  it("keeps query URL, closed fields, scalar and byte bounds consistent", () => {
    const params = vector("public.request").params;
    expect(isBoothSearchParamsV01({ ...params, query: "Different" })).toBe(false);
    expect(isBoothSearchParamsV01({ ...params, cookie: "private" })).toBe(false);
    expect(isBoothSearchParamsV01({ ...params, html: "猫".repeat(1_666_667) })).toBe(false);
    expect(isBoothSearchQueryV01("\ud800")).toBe(false);
    expect(isBoothSearchQueryV01("x\u0085y")).toBe(false);
    expect(isBoothSearchQueryV01("🧵".repeat(200))).toBe(true);
    expect(isBoothSearchQueryV01("🧵".repeat(201))).toBe(false);
    const result = vector("public.response");
    expect(isBoothSearchResultV01(result)).toBe(true);
    expect(isBoothSearchResultV01({ ...result, items: [...result.items, ...result.items] })).toBe(false);
    expect(isBoothSearchResultV01({ ...result, accountOwned: true })).toBe(false);
    expect(isBoothSearchResultV01({ ...result, items: Array(25).fill(result.items[0]) })).toBe(false);
  });
});
