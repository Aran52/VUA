import { describe, expect, it } from "vitest";
import type { AmfProductionSource, LibraryBrowserPort, LibraryCardFacts, LibraryPageView, WarehouseEntry } from "../../gateway/index.ts";
import { readStartLibrary } from "./materials-start-model.ts";

const entry = (id: string): WarehouseEntry => ({ warehouseItemId: id, folderName: id, displayName: id,
  kind: "imported_material", createdAt: "2026-10-27T00:00:00Z", artifactMode: null,
  effectiveArtifactMode: "use_original_unitypackage", artifacts: [] });
const facts = (id: string, productId: string | null, basis: "mapping" | "name" | "product_id" = "mapping"): LibraryCardFacts => ({
  storage: { state: "present", storedCopies: 1, presentCopies: 1, missingCopies: 0, changedCopies: 0, unreadableCopies: 0,
    supersededGeneratedCopies: 0, currentGeneratedCopies: 0, productionQualification: "not_evaluated" }, sources: [], operation: null,
  metadata: { schemaVersion: "0.1", entryId: id, revision: 1, displayName: id, productId: basis === "mapping" ? productId : null, thumbnailRef: null },
  ...(productId === null ? {} : { sourceMatch: { product: { productId, title: productId, libraryType: null, importedArtifacts: 0,
    shopName: null, variantName: null, price: null, imageUrl: null, imageUrls: [], availability: "unknown", availabilityRaw: null, entityCount: 0, entityTypes: [] },
    basis, content: "unverified" } }),
});
const source = (itemId: string, productIds: string[] = []): AmfProductionSource => ({ sourceId: `source-${itemId}`, itemId, itemName: itemId, name: "Avatar.unitypackage", productIds });
function library(entries: WarehouseEntry[], mappings: Map<string, LibraryCardFacts>, pageSize = 200): LibraryBrowserPort {
  return {
    async list({ offset = 0 }) { return { catalog: { schemaVersion: 1, kind: "results", items: [], total: 0,
      vocabulary: { availabilities: [], entityTypes: [], relationKinds: [] } }, localEntries: entries.slice(offset, offset + pageSize),
      facts: mappings, total: entries.length, limit: pageSize, offset } satisfies LibraryPageView; },
    async productFiles() { throw new Error("not used"); },
  };
}
const target = { productId: "booth:90", title: "Avatar", imageUrl: null } as const;

describe("material entry identity", () => {
  it("starts the exact local entry without a product association", async () => {
    const result = await readStartLibrary(library([entry("a"), entry("b")], new Map()), { entryId: "b", title: "Local", imageUrl: null }, [source("a"), source("b")]);
    expect(result.avatarEntryIds).toEqual(["b"]);
  });
  it("does not expand a selected local entry to other copies of the same product", async () => {
    const mappings = new Map(["a", "b"].map(id => [id, facts(id, "booth:90")]));
    const result = await readStartLibrary(library([entry("a"), entry("b")], mappings), { entryId: "a", title: null, imageUrl: null }, [source("a"), source("b")]);
    expect(result.avatarEntryIds).toEqual(["a"]);
  });
  it("uses saved associations but excludes name/ID guesses and corrected historical links", async () => {
    const mappings = new Map([
      ["confirmed", facts("confirmed", "booth:90")], ["name", facts("name", "booth:90", "name")],
      ["id", facts("id", "booth:90", "product_id")], ["corrected", facts("corrected", "booth:99")],
    ]);
    const result = await readStartLibrary(library([...mappings.keys()].map(entry), mappings), target, [...mappings.keys()].map(id => source(id, ["booth:90"])));
    expect(result.avatarEntryIds).toEqual(["confirmed"]);
  });
  it("keeps downloaded sources accessible when their product card has no imported origins", async () => {
    expect((await readStartLibrary(library([], new Map()), target, [source("download", ["booth:90"]), source("other", ["booth:91"])])).avatarEntryIds).toEqual(["download"]);
  });
  it("reads confirmed links after the former 1000-row cutoff", async () => {
    const entries = Array.from({ length: 1001 }, (_, index) => entry(`entry-${index}`));
    const result = await readStartLibrary(library(entries, new Map([["entry-1000", facts("entry-1000", "booth:90")]])), target, []);
    expect(result.avatarEntryIds).toEqual(["entry-1000"]);
  });
  it("does not turn a failed library read into an empty source list", async () => {
    const port = library([], new Map()); port.list = async () => { throw new Error("read failed"); };
    await expect(readStartLibrary(port, target, [])).rejects.toThrow("read failed");
  });
});
