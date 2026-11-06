/** The plan's confirmation with pictures: the BOOTH product image of the Avatar and of each
 * outfit beside the sentence, so the player recognizes the materials at a glance. Images come
 * from the library; a material without a product link shows its name only. */
import { useEffect, useState } from "react";
import { Icon } from "@vua/design-system";
import { useGateway, type AmfProductionRun, type AmfProductionSource, type LibraryBrowserPort } from "../../gateway/index.ts";
import type { AmfAssemblySelection } from "@vua/contracts";
import { strings } from "../../i18n/index.ts";

const copy = strings.amfProduction;
type Product = { readonly title: string | null; readonly imageUrl: string | null };

interface LibraryPictures { readonly products: ReadonlyMap<string, Product>; readonly entries: ReadonlyMap<string, string> }
const FRESH_MS = 60_000;
// One library read per port for a minute: libraries are small and these pictures show often.
const catalogs = new WeakMap<LibraryBrowserPort, { readonly at: number; readonly read: Promise<LibraryPictures> }>();
function readCatalog(library: LibraryBrowserPort): Promise<LibraryPictures> {
  const known = catalogs.get(library);
  if (known && Date.now() - known.at < FRESH_MS) return known.read;
  const read = (async () => {
    const products = new Map<string, Product>();
    const entries = new Map<string, string>();
    let offset = 0; let total = Number.POSITIVE_INFINITY;
    while (offset < total && offset < 1000) {
      const page = await library.list({ schemaVersion: "0.1", limit: 200, offset });
      total = page.total; offset += page.limit > 0 ? page.limit : 200;
      for (const item of page.catalog.kind === "results" ? page.catalog.items : []) products.set(item.productId, { title: item.title, imageUrl: item.imageUrl });
      for (const entry of page.localEntries) {
        const facts = page.facts.get(entry.warehouseItemId);
        const image = facts?.metadata?.thumbnailRef ?? facts?.sourceMatch?.product.imageUrl ?? null;
        if (image) entries.set(entry.warehouseItemId, image);
      }
    }
    return { products, entries };
  })();
  // A failed read may succeed next time.
  read.catch(() => catalogs.delete(library));
  catalogs.set(library, { at: Date.now(), read });
  return read;
}

function useLibraryPictures(): LibraryPictures | null {
  const library = useGateway().libraryBrowser;
  const [pictures, setPictures] = useState<LibraryPictures | null>(null);
  useEffect(() => {
    if (!library) return;
    let alive = true;
    void readCatalog(library).then(found => { if (alive) setPictures(found); }, () => { /* names stand alone */ });
    return () => { alive = false; };
  }, [library]);
  return pictures;
}

/** The picture of a material: its product's image, or the one saved with a local package. */
export function useMaterialImages(): (source: Pick<AmfProductionSource, "itemId" | "productIds"> | undefined) => string | null {
  const pictures = useLibraryPictures();
  return source => {
    if (!source || !pictures) return null;
    const product = source.productIds?.map(id => pictures.products.get(id)).find(found => found?.imageUrl);
    return product?.imageUrl ?? pictures.entries.get(source.itemId) ?? null;
  };
}

export function MaterialThumb({ src }: { src: string | null }) {
  const [failed, setFailed] = useState<string | null>(null);
  return src && failed !== src ? <img className="vua-material-thumb" src={src} alt="" loading="lazy" draggable={false} onError={() => setFailed(src)} />
    : <span className="vua-material-thumb vua-material-thumb--blank" aria-hidden="true"><Icon name="folder" size={20} /></span>;
}

export function PlanImages({ run, selection, librarySources }: {
  run: AmfProductionRun; selection: AmfAssemblySelection; librarySources: readonly AmfProductionSource[];
}) {
  const products = useLibraryPictures()?.products ?? new Map<string, Product>();
  const avatar = run.prefabs.find(prefab => prefab.isAvatar && prefab.path === selection.avatarPrefab);
  const rows = [
    ...(avatar ? [{ key: "avatar", role: "avatar" as const, sourceId: avatar.sourceId, fallback: avatar.name }] : []),
    ...selection.outfits.map(outfit => ({ key: outfit.sourceId, role: "outfit" as const, sourceId: outfit.sourceId,
      fallback: outfit.toggleName || run.prefabs.find(prefab => prefab.path === outfit.prefabPath)?.name || outfit.sourceId })),
  ];
  if (rows.length === 0) return null;
  return <ul className="vua-plan-images">{rows.map(row => {
    const source = run.sources.find(item => item.sourceId === row.sourceId);
    const product = librarySources.find(item => item.sourceId === row.sourceId)?.productIds
      ?.map(id => products.get(id)).find(found => found !== undefined);
    const title = product?.title ?? source?.itemName.replace(/\.zip$/iu, "") ?? row.fallback;
    return <li key={row.key}>
      {product?.imageUrl ? <img src={product.imageUrl} alt="" loading="lazy" />
        : <span className="vua-plan-images__blank" aria-hidden="true"><Icon name={row.role === "avatar" ? "avatar" : "outfit"} size={24} /></span>}
      <span><small>{row.role === "avatar" ? copy.avatar : copy.outfit}</small><strong title={title}>{title}</strong></span>
    </li>;
  })}</ul>;
}
