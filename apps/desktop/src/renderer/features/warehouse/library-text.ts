/** Small texts both the wall and the details page show. */
import type { LibrarySortFact } from "./library-view-model.ts";
import type { CatalogProductSummary, LibraryCardFacts } from "../../gateway/index.ts";
import { currentLocale, format, formatDateTime, strings } from "../../i18n/index.ts";
import { sizeText } from "./acquire-model.ts";
import { priceKind } from "./warehouse-model.ts";

const copy = strings.warehouse;

export function sizeLabel(bytes: number): string {
  const size = sizeText(bytes);
  return format(copy.acquire[size.unitKey], { amount: size.amount });
}

/** Amounts stay the recorded text and never pass through a JS number; the language picks the symbol and its place. */
function moneyText(currency: string, amount: string): string {
  const grouped = amount.replace(/^\d+/u, digits => digits.replace(/\B(?=(\d{3})+$)/gu, ","));
  try {
    return new Intl.NumberFormat(currentLocale, { style: "currency", currency, maximumFractionDigits: 0 }).formatToParts(0)
      .map(part => part.type === "integer" ? grouped : part.value).join("");
  } catch { return `${currency} ${grouped}`; }
}

export function priceLabel(product: { price: CatalogProductSummary["price"]; priceHigh?: string | null | undefined }): string | null {
  switch (priceKind(product.price)) {
    case "free": return copy.card.free;
    case "none": return null;
    case "priced": {
      const currency = product.price?.currency ?? "", amount = product.price?.amount ?? "", high = product.priceHigh;
      return high === null || high === undefined || high === amount ? moneyText(currency, amount)
        : format(copy.card.priceRange, { low: moneyText(currency, amount), high: moneyText(currency, high) });
    }
  }
}

/** Where a recorded video opens, and the site's name for its button. An embed address opens as the site's own page. */
export function videoLink(url: string): { readonly href: string; readonly site: string } {
  try {
    const parsed = new URL(url);
    const host = parsed.hostname.replace(/^www\./u, "");
    if (/(^|\.)(youtube\.com|youtube-nocookie\.com|youtu\.be)$/u.test(host)) {
      const id = /^\/embed\/([\w-]+)/u.exec(parsed.pathname)?.[1];
      return { href: id === undefined ? url : `https://www.youtube.com/watch?v=${id}`, site: "YouTube" };
    }
    return { href: url, site: host };
  } catch { return { href: url, site: url }; }
}

/** BOOTH's recorded description is one flat line: its bold marks go, and its rule lines become paragraph breaks. */
export function readableDescription(text: string): string {
  return text.replace(/\*\*/gu, "").replace(/\s*[=＝─━_~-]{4,}\s*/gu, "\n\n").trim();
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
