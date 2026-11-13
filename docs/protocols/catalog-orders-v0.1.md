# Catalog orders v0.1 — when bought products were ordered

> Document version: 0.1
> Status: Implementation baseline — freeze after the accompanying schema and consumer checks pass
> Updated: 2026-11-07
> Maintainer: AMF
> Scope: Reading order times from BOOTH's purchase history during an account-library sync

## Ownership

BOOTH's library pages carry no dates. Its purchase history at `https://accounts.booth.pm/orders` lists each
order's product picture and time, so a sync reads those pages to learn when bought products were ordered.
Gifts and free downloads are not orders and have no recorded time.

Electron reads the pages in its isolated session, right after a sync's library pages and before it finishes.
Acquisition keeps two facts per product: its number and the order's time. Names, prices, order numbers and the
raw page are not stored. Times are a convenience: if a history page cannot be read, the reader stops, the
library sync keeps its own result, and the products keep what was last recorded.

## Method

Schemas and synthetic vectors: [schemas/catalog-orders/v0.1](../../schemas/catalog-orders/v0.1).

| Method | Kind | Behavior |
| --- | --- | --- |
| `catalog.ingestOrdersPage` | command | Parse one history page and record the time of each product the account library already lists |

The request has `schemaVersion`, `sourceUrl` and `html`. `sourceUrl` is the history page, with an optional
numeric `page` query. The result reports `parsedCount` (products found on the page), `recordedCount` (those in
the library) and `nextPageUrl`. The reader accepts continuations only on the same page and path, reads at most 50
pages, and paces them like library pages. A document that is not a history page is
`vua.catalog.not_an_orders_page`; an empty history is a page with no products.

## Page grammar

An order is a link to `/orders/<number>`. Its picture's address contains `/i/<product number>/`, and its text
contains the time as `YYYY/MM/DD HH:MM:SS` in Japan time, which BOOTH prints on every language's page. Pagination
is `a[rel="next"]`, as on library pages. A link with no picture or no time gives no record. When a product appears
in several orders, the latest order's time stands.

## Storage

Times live in `bdl_meta` as `library_ordered_at:<product id>`, beside the format version, so the BDL format does
not change and builds that do not read them keep opening the store.

## Evidence

Parser tests are in `crates/acquisition/src/order_page.rs`, wire tests in
`crates/provider-host/tests/catalog_sync_wire_v01.rs` (the `orders_v01` module), guards in
`packages/contracts/src/catalog-orders.test.ts` and the reader in `apps/desktop/src/electron/catalog-sync.test.ts`.
The grammar was read from a real signed-in page on 2026-11-07; all vectors are synthetic.
