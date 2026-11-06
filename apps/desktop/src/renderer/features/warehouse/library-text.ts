/** Small texts both the wall and the details page show. */
import type { LibrarySortFact } from "./library-view-model.ts";
import type { CatalogProductSummary, LibraryCardFacts } from "../../gateway/index.ts";
import { format, formatDateTime, strings } from "../../i18n/index.ts";
import { sizeText } from "./acquire-model.ts";
import { priceKind } from "./warehouse-model.ts";

const copy = strings.warehouse;

export function sizeLabel(bytes: number): string {
  const size = sizeText(bytes);
  return format(copy.acquire[size.unitKey], { amount: size.amount });
}

/** Amounts stay the recorded text and never pass through a JS number. */
export function priceLabel(product: { price: CatalogProductSummary["price"]; priceHigh?: string | null | undefined }): string | null {
  switch (priceKind(product.price)) {
    case "free": return copy.card.free;
    case "none": return null;
    case "priced": return format(
      product.priceHigh !== null && product.priceHigh !== undefined && product.priceHigh !== product.price?.amount ? copy.card.priceRange : copy.card.price,
      { currency: product.price?.currency ?? "", amount: product.price?.amount ?? "", high: product.priceHigh ?? "" },
    );
  }
}

/** The value a non-default order goes by, shown on the card so the order explains itself. */
export function sortFactLabel(fact: LibrarySortFact, facts: LibraryCardFacts | undefined, price: CatalogProductSummary["price"] | undefined): string | null {
  switch (fact) {
    case "size": return (facts?.sizeBytes ?? 0) > 0 ? sizeLabel(facts!.sizeBytes!) : null;
    case "price": return price === undefined ? null : priceLabel({ price });
    case "published": return facts?.publishedAt ? formatDateTime(facts.publishedAt) : null;
    case "added": return facts?.addedAt ? formatDateTime(facts.addedAt) : null;
    case null: return null;
  }
}
