# Catalog enrichment v0.1

> Document version: 0.1
> Status: Implementation baseline (2026-11-11)
> Owner: AMF acquisition

This face supplements existing BOOTH product observations. Electron Main owns the
session and HTTP reads; AMF parses supplied documents and owns local membership,
retry admission and persistence. No credentials cross the application boundary.
The frozen catalog-sync faces remain unchanged.

## Application methods

All methods use the v0.1 application envelope and closed parameter objects.

- Query `catalog.enrichmentQueue {schemaVersion: "0.1", limit}` returns
  `{schemaVersion: "0.1", productIds}`. Limit is an integer from 1 to 60.
  Only products in the player's recorded account library qualify, excluding
  removed cards. A missing category, shop name/URL, description or publish date
  qualifies. Bought membership takes precedence over gifts, then free downloads;
  product ID breaks ties. Querying does not record an attempt.
- Command `catalog.beginEnrichment {schemaVersion: "0.1", productId}` returns
  `{schemaVersion: "0.1", productId, claimed}`. It atomically rechecks the same
  eligibility and stores the Provider's current timestamp in
  `bdl_meta["enrich_attempted_at:" + productId]` before Main reads the product.
  Attempts less than seven days old, including future timestamps after clock
  rollback, cannot be claimed. The caller cannot supply the clock or membership.
- Command `catalog.observeProduct {schemaVersion: "0.1", productId, format,
  sourceUrl, body, fetchedAt}` returns `{schemaVersion: "0.1", productId}`.
  Format is `json` or `html`. Product IDs are `booth:<digits>` (1–20 digits).
  Source URLs are exactly `https://booth.pm/<locale>/items/<digits>` with `.json`
  only for JSON; locales are `ja`, `en`, `ko`, `zh-cn`, `zh-tw`. URL and document
  identity must match productId. Body is nonempty and at most 5,000,000 UTF-8
  bytes. fetchedAt is a valid UTC RFC 3339 timestamp with seconds 00–59. This command also serves
  an explicit product-detail lookup; it never creates account membership.

Invalid shape, URL or time returns `vua.catalog.invalid_params`; an unrecognized
document returns `vua.catalog.invalid_product`; an identity mismatch returns
`vua.catalog.product_mismatch`. Storage and readiness errors remain
`vua.catalog.store_failed` and `vua.catalog.unavailable`.

## Product facts

JSON is the optional, author-approved public `items/<id>.json` representation,
not a claimed stable or officially documented BOOTH API. The pure extractor
requires product identity and ignores unknown upstream fields. Missing or
malformed fields remain extraction gaps. Category/name, shop/name and URL,
description, published_at, explicit adult/sale flags, images and variations
populate existing observation fields; unrelated upstream fields are not stored.
Published dates retain their observed timezone. Prices are accepted only when
their amount and currency can both be read. Unknown variant availability stays
unknown. HTML remains the fallback when JSON is absent or unrecognized.

Description text preserves literal newlines and HTML line/paragraph breaks,
trims line edges and keeps at most one blank line. Product and shop sections are
joined by a blank line. Other HTML fields retain their existing normalization.
Publication timestamps keep their source offset and sort by the represented
instant; invalid or missing timestamps sort after known dates. Calendar-invalid
dates and leap-second spellings are not accepted as publication metadata.
Search indexes collapse whitespace from the final merged stored facts, so a
sparse observation cannot erase previously searchable text. Null observations
preserve existing scalar facts. Description links remain unconfirmed local
evidence; reading a description never fetches linked products or terms.

## Background transport

One application-lifetime scheduler starts after sync completion or 60 seconds of
startup idle. It claims one eligible product at a time, reads JSON first, and
falls back to HTML on an unrecognized JSON response. Every background HTTP read,
including the initial post-sync/foreground spacing and HTML fallback, is
separated from the previous BOOTH session HTTP read by at least five seconds. At
most 60 products are attempted per application launch, including failed reads.
Each read has a 30-second deadline and uses the existing local BOOTH session.

Network failures, HTTP 403, 429 or 5xx stop background reads for the rest of the
launch. Active downloads, sync, foreground detail/search reads, unavailable AMF
and shutdown suspend/abort reads; no pending request survives disposal. Resuming
does not reset the launch budget or failure latch. Membership is rechecked after
waiting for the next request slot, before admission, and no other pages are
enumerated. A completed batch emits `vua:catalog-sync:enriched` with
`{schemaVersion: "0.1", updated}` (1–60), without a notification. The typed
`catalogSync.subscribeEnrichment` port provides a refresh hint; Claude owns the
page subscription and authoritative list reread. Explicit detail reads retain
the legacy HTML writer when an older AMF does not support the new observation
method. Such an AMF cannot admit background enrichment.

Schemas and shared synthetic vectors live in `schemas/catalog-enrichment/v0.1`.
Real-account enrichment and multi-item order recognition require separate local
author acceptance; public-JSON compatibility alone does not prove them.

## Document changelog

- 0.1 (2026-11-10): bounded local-library enrichment and product JSON fallback.
