import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type { DesktopGatewaySuccessValueV1 } from "@vua/contracts";
import type { GatewayClient } from "./gateway-client.ts";
import { createLiveLibraryBrowserPort } from "./library-browser-port.ts";
const vector = (): Record<string, unknown> => JSON.parse(readFileSync(new URL("../../../../../schemas/library-view/v0.1/examples/local-source.response.json", import.meta.url), "utf8")) as Record<string, unknown>;
const client = (value: unknown): GatewayClient => ({ invoke: async () => ({ ok: true, value: value as DesktopGatewaySuccessValueV1 }), subscribe: () => () => {} });
describe("library source reconciliation projection", () => {
  it("retains observed variant counts and the exact reverse link filter without widening frozen catalog metadata", async () => {
    const value = vector(); (value.items as Record<string, unknown>[])[0]!.variantCount = 3;
    const requests: unknown[] = [];
    const port = createLiveLibraryBrowserPort({ ...client(value), invoke: async request => { requests.push(request); return { ok: true, value: value as unknown as DesktopGatewaySuccessValueV1 }; } });
    const page = await port.list({ schemaVersion: "0.1", descriptionLinksTo: "booth:902" });
    expect(page.facts.get("booth:90")?.variantCount).toBe(3);
    expect(requests[0]).toMatchObject({ method: "library.list", params: { descriptionLinksTo: "booth:902" } });
  });
  it("preserves separate identities, candidate metadata and per-card deletion scopes", async () => {
    const page = await createLiveLibraryBrowserPort(client(vector())).list({ schemaVersion: "0.1" });
    expect(page.catalog.kind).toBe("results");
    expect(page.localEntries[0]?.warehouseItemId).toBe("wh-imported");
    expect(page.facts.get("booth:90")?.copyIds).toEqual([]);
    expect(page.facts.get("wh-imported")).toMatchObject({ copyIds: ["cpy-imported"], sources: ["bought"], sourceMatch: { product: { productId: "booth:90", title: "Synthetic item" }, basis: "name", content: "unverified" } });
    expect(page.total).toBe(2);
  });
  it("rejects contradictory content claims and expanded file scopes", async () => {
    const value = vector(); const rows = value.items as Record<string, unknown>[];
    rows[1]!.copyIds = ["cpy-imported", "cpy-another-card"];
    await expect(createLiveLibraryBrowserPort(client(value)).list({ schemaVersion: "0.1" })).rejects.toThrow("library_read_failed");
  });
});

it("retains purpose annotations and refuses missing, foreign or unknown usable claims",async()=>{
  const value=vector();value.schemaVersion="0.2";const rows=value.items as Record<string,unknown>[];
  rows[0]!.use={presetId:"vtuber_individual",status:"conditional"};rows[1]!.use={presetId:"vtuber_individual",status:"no_terms"};
  const port=createLiveLibraryBrowserPort(client(value));
  expect((await port.list({schemaVersion:"0.2",use:"vtuber_individual",useMode:"all"})).facts.get("wh-imported")?.use?.status).toBe("no_terms");
  await expect(port.list({schemaVersion:"0.2",use:"vtuber_individual"})).rejects.toThrow("library_use_invalid");
  rows[1]!.use={presetId:"vrchat_personal",status:"allowed"};
  await expect(port.list({schemaVersion:"0.2",use:"vtuber_individual",useMode:"all"})).rejects.toThrow("library_use_invalid");
  delete rows[1]!.use;
  await expect(port.list({schemaVersion:"0.2",use:"vtuber_individual",useMode:"all"})).rejects.toThrow("library_use_invalid");
  expect((await port.list({schemaVersion:"0.2"})).total).toBe(2);
});
