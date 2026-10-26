import type { WarehouseEntryDetail } from "./acquire-port.ts";
import { confirmedLibraryProductId, type LibraryCardFacts } from "./library-browser-port.ts";
import type {
  CatalogBrowserPort,
  CatalogProductDetail,
} from "./catalog-browser-port.ts";

/**
 * Compose existing library, warehouse and catalog reads without changing wire contracts.
 * Current library associations take precedence over historical artifact mappings;
 * unconfirmed source hints never establish an association.
 *
 * No confirmed link means no-association. A link without displayable catalog media
 * means no-images, including catalog misses and failed individual reads. Albums
 * contain only returned images, and linking a source does not verify local content.
 */

/** 单个关联来源的可显示相册(仅含至少一张真实返回图的来源) */
export interface EntryPreviewAlbum {
  readonly productId: string;
  /** 详情标题;无题观测回落 productId(与云端详情抽屉同一纪律) */
  readonly title: string;
  /** 详情媒体数组;缺媒体回落主图单张(与云端详情抽屉同一回落线) */
  readonly imageUrls: readonly string[];
}

export type EntryPreviewView =
  | { kind: "no-association" }
  | { kind: "no-images" }
  | { kind: "loaded"; albums: readonly EntryPreviewAlbum[] };

/** 条目事实 → 去重后的关联来源身份(首现顺序;纯函数,不发起查询) */
export function entryPreviewProductIds(
  entry: WarehouseEntryDetail,
  facts?: Pick<LibraryCardFacts, "metadata" | "sourceMatch">,
): readonly string[] {
  const currentSource = confirmedLibraryProductId(facts);
  if (currentSource !== null) return [currentSource];
  const seen = new Set<string>();
  const ids: string[] = [];
  for (const artifact of entry.artifacts) {
    for (const productId of artifact.mappedProductIds) {
      if (!seen.has(productId)) {
        seen.add(productId);
        ids.push(productId);
      }
    }
  }
  return ids;
}

/** 媒体回落线(与云端详情抽屉一致):媒体数组 → 主图单张 → 空 */
function albumImages(product: CatalogProductDetail): readonly string[] {
  return product.media.imageUrls.length > 0
    ? product.media.imageUrls
    : product.imageUrl !== null
      ? [product.imageUrl]
      : [];
}

/**
 * 组合读:条目关联身份逐一定向查询目录详情,聚合可显示相册。
 * 永不 reject:单品查询失败按该品无图吸收(诚实空态归并见文件头)。
 */
export async function readEntryPreview(
  catalog: CatalogBrowserPort,
  entry: WarehouseEntryDetail,
  facts?: Pick<LibraryCardFacts, "metadata" | "sourceMatch">,
): Promise<EntryPreviewView> {
  const productIds = entryPreviewProductIds(entry, facts);
  if (productIds.length === 0) return { kind: "no-association" };
  const details = await Promise.all(
    productIds.map(async (productId) => ({
      productId,
      view: await catalog.detail(productId).catch(() => null),
    })),
  );
  const albums: EntryPreviewAlbum[] = [];
  for (const { productId, view } of details) {
    if (view === null || view.kind !== "detail") continue;
    const imageUrls = albumImages(view.product);
    if (imageUrls.length === 0) continue;
    albums.push({ productId, title: view.product.title ?? productId, imageUrls });
  }
  return albums.length > 0 ? { kind: "loaded", albums } : { kind: "no-images" };
}
