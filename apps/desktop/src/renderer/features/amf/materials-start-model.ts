import { confirmedLibraryProductId, type AmfProductionSource, type LibraryBrowserPort } from "../../gateway/index.ts";

export type MaterialsStartAvatar = {
  readonly title: string | null;
  readonly imageUrl: string | null;
} & ({ readonly entryId: string; readonly productId?: never } | { readonly productId: string; readonly entryId?: never });

/** Local entry selection is exact, even when several copies share a product.
 * Product entry points use confirmed library links and admitted downloaded sources. */
export async function readStartLibrary(library: LibraryBrowserPort, avatar: MaterialsStartAvatar, sources: readonly AmfProductionSource[]) {
  const avatarEntryIds = new Set<string>(avatar.entryId === undefined ? [] : [avatar.entryId]);
  const avatarMaterialNames = new Set<string>();
  const titles = new Map<string, string | null>();
  const localIds = new Set<string>();
  let offset = 0;
  let total = Number.POSITIVE_INFINITY;
  while (offset < total) {
    const page = await library.list({ schemaVersion: "0.1", limit: 200, offset });
    total = page.total;
    if (page.limit <= 0) throw new Error("library_page_invalid");
    offset += page.limit;
    for (const product of page.catalog.kind === "results" ? page.catalog.items : []) {
      for (const entry of page.facts.get(product.productId)?.localEntries ?? []) {
        if (!titles.has(entry.entryId)) titles.set(entry.entryId, product.title);
        if (avatar.productId === product.productId) {
          avatarEntryIds.add(entry.entryId); avatarMaterialNames.add(entry.displayName);
        }
      }
    }
    for (const entry of page.localEntries) {
      const id = entry.warehouseItemId;
      localIds.add(id);
      const facts = page.facts.get(id);
      const confirmed = confirmedLibraryProductId(facts);
      if (id === avatar.entryId || (avatar.productId !== undefined && confirmed === avatar.productId)) {
        avatarEntryIds.add(id); avatarMaterialNames.add(entry.displayName);
      }
      titles.set(id, facts?.sourceMatch?.basis === "mapping" ? facts.sourceMatch.product.title : null);
    }
  }
  if (avatar.productId !== undefined) {
    // Downloaded packages may live under product cards, without localEntries.
    // Local rows have already resolved their current, possibly corrected source above.
    for (const source of sources) if (!localIds.has(source.itemId) && source.productIds?.includes(avatar.productId)) {
      avatarEntryIds.add(source.itemId); avatarMaterialNames.add(source.itemName);
    }
  }
  return { avatarEntryIds: [...avatarEntryIds], avatarMaterialNames: [...avatarMaterialNames], titles };
}
