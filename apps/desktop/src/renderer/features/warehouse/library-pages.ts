/** Materials reads its rows in batches as the wall scrolls and keeps them all, in the order the library
 * sent them, so selection, keys and details cover everything loaded so far. */
import type { LibraryPageView } from "../../gateway/index.ts";

export const LIBRARY_BATCH = 100;
/** The most rows one read may ask for. */
export const LIBRARY_READ_MAX = 200;

export const rowsLoaded = (page: LibraryPageView): number => page.order?.length ?? (page.catalog.kind === "results" ? page.catalog.items.length : 0) + page.localEntries.length;
export const moreRowsExist = (page: LibraryPageView | null): boolean => page !== null && rowsLoaded(page) < page.total;

function unique<T>(values: readonly T[], key: (value: T) => string): T[] {
  const seen = new Set<string>();
  return values.filter(value => { const id = key(value); if (seen.has(id)) return false; seen.add(id); return true; });
}

/** Later rows follow earlier ones. A row that moved between reads stays where it was first seen. */
export function mergeLibraryPages(first: LibraryPageView, next: LibraryPageView): LibraryPageView {
  const items = first.catalog.kind === "results" && next.catalog.kind === "results"
    ? unique([...first.catalog.items, ...next.catalog.items], item => item.productId) : null;
  return {
    ...next,
    catalog: items !== null && next.catalog.kind === "results" ? { ...next.catalog, items } : next.catalog,
    localEntries: unique([...first.localEntries, ...next.localEntries], entry => entry.warehouseItemId),
    facts: new Map([...first.facts, ...next.facts]),
    order: unique([...(first.order ?? []), ...(next.order ?? [])], key => key),
    offset: first.offset,
  };
}

/** A refresh of the rows at the top keeps the rows further down in place while the total is unchanged. */
export function refreshLibraryHead(current: LibraryPageView, head: LibraryPageView): LibraryPageView {
  if (head.total !== current.total) return head;
  const fresh = new Set(head.order ?? []);
  const rest: LibraryPageView = {
    ...current,
    catalog: current.catalog.kind === "results" ? { ...current.catalog, items: current.catalog.items.filter(item => !fresh.has(productRow(item.productId))) } : current.catalog,
    localEntries: current.localEntries.filter(entry => !fresh.has(entryRow(entry.warehouseItemId))),
    order: (current.order ?? []).filter(key => !fresh.has(key)),
  };
  return mergeLibraryPages(head, rest);
}

/** Reads `from` to `to` in the sizes the library accepts, one read at a time. */
export async function readLibraryRange(read: (offset: number, limit: number) => Promise<LibraryPageView>, from: number, to: number, start: LibraryPageView | null): Promise<LibraryPageView> {
  let page = start;
  let offset = from;
  do {
    const limit = Math.min(LIBRARY_READ_MAX, Math.max(1, to - offset));
    const next = await read(offset, limit);
    page = page === null ? next : mergeLibraryPages(page, next);
    offset += limit;
    if (offset >= next.total) break;
  } while (offset < to);
  return page;
}

export const productRow = (productId: string): string => `product:${productId}`;
export const entryRow = (entryId: string): string => `entry:${entryId}`;

export type WallRow<P, C> = { readonly kind: "product"; readonly item: P } | { readonly kind: "local"; readonly card: C };

/** Products and local imports in the library's own order. Rows the order does not name follow it. */
export function wallRows<P extends { readonly productId: string }, C extends { readonly entry: { readonly warehouseItemId: string } }>(order: readonly string[] | undefined, items: readonly P[], cards: readonly C[]): readonly WallRow<P, C>[] {
  const products = new Map(items.map(item => [productRow(item.productId), item]));
  const entries = new Map<string, C[]>();
  for (const card of cards) { const key = entryRow(card.entry.warehouseItemId); entries.set(key, [...(entries.get(key) ?? []), card]); }
  const rows: WallRow<P, C>[] = [];
  for (const key of order ?? []) {
    const item = products.get(key);
    if (item !== undefined) { rows.push({ kind: "product", item }); products.delete(key); continue; }
    for (const card of entries.get(key) ?? []) rows.push({ kind: "local", card });
    entries.delete(key);
  }
  for (const item of products.values()) rows.push({ kind: "product", item });
  for (const list of entries.values()) for (const card of list) rows.push({ kind: "local", card });
  return rows;
}
