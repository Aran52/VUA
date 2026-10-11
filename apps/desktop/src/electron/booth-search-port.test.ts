import { expect, it, vi } from "vitest";
import { searchPublicBoothProducts } from "./booth-search-port.js";
it("requests public search without an account library filter and keeps HTML out of the renderer result", async () => {
  const value = { schemaVersion: "0.1", query: "Synthetic coat", items: [{ productId: "booth:90", title: "Synthetic coat", shopName: null, sourceCategory: "3D衣装" }], truncated: false };
  const fetch = vi.fn(async (url: string) => ({ status: 200, body: "<ul class='market-items'></ul>", finalUrl: url }));
  const parse = vi.fn(async () => ({ ok: true, value }));
  expect(await searchPublicBoothProducts(value.query, { fetch, parse })).toEqual(value);
  expect(fetch).toHaveBeenCalledWith("https://booth.pm/ja/search/Synthetic%20coat");
  expect(parse.mock.calls).toHaveLength(1);
});
it("refuses redirects to sign-in, HTTP failures and an unrelated parser receipt", async () => {
  const parse = vi.fn(async () => ({ ok: true, value: {} }));
  await expect(searchPublicBoothProducts("Synthetic", { fetch: async () => ({ status: 200, body: "", finalUrl: "https://accounts.booth.pm/library" }), parse })).rejects.toThrow();
  expect(parse).not.toHaveBeenCalled();
  await expect(searchPublicBoothProducts("Synthetic", { fetch: async url => ({ status: 200, body: "", finalUrl: url }), parse })).rejects.toThrow();
});
