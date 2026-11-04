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

// One library read per port: libraries are small and the plan station opens often.
const catalogs = new WeakMap<LibraryBrowserPort, Promise<ReadonlyMap<string, Product>>>();
function readCatalog(library: LibraryBrowserPort): Promise<ReadonlyMap<string, Product>> {
  const known = catalogs.get(library);
  if (known) return known;
  const read = (async () => {
    const products = new Map<string, Product>();
    let offset = 0; let total = Number.POSITIVE_INFINITY;
    while (offset < total && offset < 1000) {
      const page = await library.list({ schemaVersion: "0.1", limit: 200, offset });
      total = page.total; offset += page.limit > 0 ? page.limit : 200;
      for (const item of page.catalog.kind === "results" ? page.catalog.items : []) products.set(item.productId, { title: item.title, imageUrl: item.imageUrl });
    }
    return products;
  })();
  // A failed read may succeed next time.
  read.catch(() => catalogs.delete(library));
  catalogs.set(library, read);
  return read;
}

export function PlanImages({ run, selection, librarySources }: {
  run: AmfProductionRun; selection: AmfAssemblySelection; librarySources: readonly AmfProductionSource[];
}) {
  const library = useGateway().libraryBrowser;
  const [products, setProducts] = useState<ReadonlyMap<string, Product>>(() => new Map());
  useEffect(() => {
    if (!library) return;
    let alive = true;
    void readCatalog(library).then(found => { if (alive) setProducts(found); }, () => { /* names stand alone */ });
    return () => { alive = false; };
  }, [library]);
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
