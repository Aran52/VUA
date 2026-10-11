import type { LibraryFileInventory } from "../../gateway/index.ts";
import type { RemovalDialogTarget } from "./RemoveFilesDialog.tsx";

export interface BulkProduct { readonly productId: string; readonly title: string }
export type BulkDownloadMode = "download" | "redownload";

/** Download fills missing files; re-download is an explicit replacement request. */
export function downloadableFiles(inventory: LibraryFileInventory, mode: BulkDownloadMode) {
  return inventory.items.filter(file => mode === "redownload" || !file.copies.some(copy => copy.presence === "present"));
}

/** Retain the displayed copy scope; never expand an omitted scope on the server. */
export function removalScopes(items: readonly RemovalDialogTarget[]): RemovalDialogTarget[] {
  const seen = new Set<string>();
  return items.flatMap(item => {
    const copies = (item.copyIds ?? []).filter(id => {
      if (seen.has(id)) return false;
      seen.add(id); return true;
    });
    const result: RemovalDialogTarget[] = [];
    for (let offset = 0; offset < copies.length; offset += 200) {
      result.push({ ...item, copyIds: copies.slice(offset, offset + 200) });
    }
    return result;
  });
}

export function removalProductCount(items: readonly RemovalDialogTarget[]): number {
  return new Set(items.filter(item => (item.copyIds?.length ?? 0) > 0).map(item => `${item.target.kind}:${item.target.id}`)).size;
}
