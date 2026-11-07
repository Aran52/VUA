import { describe, expect, it } from "vitest";
import type { LibraryPageView } from "../../gateway/index.ts";
import { entryRow, mergeLibraryPages, moreRowsExist, productRow, readLibraryRange, refreshLibraryHead, rowsLoaded, wallRows } from "./library-pages.ts";

function page(ids: readonly string[], total: number, offset = 0): LibraryPageView {
  const products = ids.filter(id => !id.startsWith("e"));
  const entries = ids.filter(id => id.startsWith("e"));
  return {
    catalog: { schemaVersion: 1, kind: "results", items: products.map(productId => ({ productId })), total, vocabulary: { availabilities: [], entityTypes: [], relationKinds: [] } },
    localEntries: entries.map(warehouseItemId => ({ warehouseItemId })),
    facts: new Map(ids.map(id => [id, { storage: {}, sources: [], operation: null }])),
    order: ids.map(id => id.startsWith("e") ? entryRow(id) : productRow(id)),
    total, offset, limit: ids.length,
  } as unknown as LibraryPageView;
}

describe("library batches", () => {
  it("adds later rows after earlier ones and drops a row seen twice", () => {
    const merged = mergeLibraryPages(page(["a", "e1"], 4), page(["e1", "b", "c"], 4, 2));
    expect(merged.order).toEqual([productRow("a"), entryRow("e1"), productRow("b"), productRow("c")]);
    expect(rowsLoaded(merged)).toBe(4);
    expect(moreRowsExist(merged)).toBe(false);
    expect(merged.offset).toBe(0);
  });

  it("knows when rows remain", () => {
    expect(moreRowsExist(page(["a"], 2))).toBe(true);
    expect(moreRowsExist(null)).toBe(false);
  });

  it("reads a long range in sizes the library accepts", async () => {
    const asked: [number, number][] = [];
    const result = await readLibraryRange(async (offset, limit) => { asked.push([offset, limit]); return page(Array.from({ length: limit }, (_, i) => `p${offset + i}`), 1000, offset); }, 0, 450, null);
    expect(asked).toEqual([[0, 200], [200, 200], [400, 50]]);
    expect(rowsLoaded(result)).toBe(450);
  });

  it("stops at the end of the library", async () => {
    const asked: number[] = [];
    await readLibraryRange(async (offset, limit) => { asked.push(offset); return page(Array.from({ length: Math.min(limit, 150 - offset) }, (_, i) => `p${offset + i}`), 150, offset); }, 0, 500, null);
    expect(asked).toEqual([0]);
  });

  it("refreshes the top in place while the total holds", () => {
    const current = page(["a", "b", "c", "d"], 4);
    const head = page(["a", "b"], 4);
    expect(refreshLibraryHead(current, head).order).toEqual(["a", "b", "c", "d"].map(productRow));
  });

  it("starts over from the refreshed rows when the total changed", () => {
    expect(refreshLibraryHead(page(["a", "b", "c"], 3), page(["z", "a"], 4)).order).toEqual(["z", "a"].map(productRow));
  });
});

describe("wall rows", () => {
  const items = [{ productId: "a" }, { productId: "b" }];
  const cards = [{ key: "e1:0", entry: { warehouseItemId: "e1" } }, { key: "e1:1", entry: { warehouseItemId: "e1" } }, { key: "e2:0", entry: { warehouseItemId: "e2" } }];

  it("keeps products and local imports in the library's order", () => {
    const rows = wallRows([productRow("b"), entryRow("e2"), productRow("a"), entryRow("e1")], items, cards);
    expect(rows.map(row => row.kind === "product" ? row.item.productId : row.card.key)).toEqual(["b", "e2:0", "a", "e1:0", "e1:1"]);
  });

  it("shows rows the order does not name after it", () => {
    const rows = wallRows(undefined, items, cards);
    expect(rows.map(row => row.kind === "product" ? row.item.productId : row.card.key)).toEqual(["a", "b", "e1:0", "e1:1", "e2:0"]);
  });
});
