import { isBoothSearchQueryV01, isBoothSearchResultV01, type BoothSearchResultV01 } from "@vua/contracts";

/** Transport remains in Electron's BOOTH partition; parsing belongs to acquisition. */
export async function searchPublicBoothProducts(query: unknown, ports: {
  fetch(url: string): Promise<{ status: number; body: string; finalUrl: string }>;
  parse(params: { schemaVersion: "0.1"; query: string; sourceUrl: string; html: string }): Promise<{ ok: boolean; value?: unknown }>;
}): Promise<BoothSearchResultV01> {
  if (!isBoothSearchQueryV01(query)) throw new Error("invalid_search_query");
  const url = `https://booth.pm/ja/search/${encodeURIComponent(query)}`;
  const page = await ports.fetch(url);
  if (page.status !== 200 || page.body.length > 5_000_000 || page.finalUrl !== url) throw new Error("search_unavailable");
  const result = await ports.parse({ schemaVersion: "0.1", query, sourceUrl: url, html: page.body });
  if (!result.ok || !isBoothSearchResultV01(result.value) || result.value.query !== query) throw new Error("search_unavailable");
  return result.value;
}
