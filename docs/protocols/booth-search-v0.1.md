# Public BOOTH search v0.1

> Document version: 0.1
> Status: Implementation baseline (2026-10-12)
> Owner: AMF acquisition

`catalog.searchProducts` is a read-only query in the existing v0.1 application envelope.
Closed params are `{ schemaVersion: "0.1", query, sourceUrl, html }`. Query is trimmed,
nonempty, at most 200 Unicode scalars, without control characters. The exact source URL is
`https://booth.pm/ja/search/${encodeURIComponent(query)}`; HTML is bounded to 5 MB of UTF-8.
The schema covers field/shape bounds; consumers additionally enforce exact URL, trim, UTF-8
byte limit and unique product IDs. Cookies, paths and files are absent.

Main's `catalogSync.searchProducts(query)` native face accepts only keywords from a local,
ready-AMF sender. It fetches through the BOOTH partition with a 30-second deadline, one active
search and 1.5-second minimum spacing. Redirected login/foreign content fails rather than
becoming empty results. Acquisition extracts public cards independently of account-library rows.
Reply is `{ schemaVersion: "0.1", query, items, truncated }`; each of at most 24 items has
`{ productId, title, shopName, sourceCategory }`, with nullable shop/category. A recognized empty
search shell returns empty items; unrecognized HTML returns unavailable. Pagination, excess or
unreadable cards mark `truncated`. No crawling, catalog mutation or entitlement occurs.

Local lookup derives only a leaf-name keyword; the candidate dialog permits editing it. A
single exact normalized title, at least four scalars, on a complete page can auto-associate.
Other results require explicit selection. Association retrieves the official product through
the page reader with a 30-second deadline and same-product redirect check, then uses the idempotent, revision-checked
[entry-metadata command](library-entry-metadata-v0.1.md). It preserves manual name/thumbnail,
entry identity and bytes. Search/association never creates account membership or verified
content equality. Transport/receipt failures remain retryable without choosing a fuzzy first hit.

Schemas/vectors live in `schemas/booth-search/v0.1`; TypeScript guards and
`library_download_wire_v01` consume them, with the latter proving an unchanged account library.
Controlled fixtures do not replace real-account/material or human-language acceptance.

## Document changelog

- 0.1 (2026-10-12): define bounded public candidates and provenance-only association.
