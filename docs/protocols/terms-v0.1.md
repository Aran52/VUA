# VN3 terms v0.1

> Document version: 0.1.0
> Status: Candidate
> Updated: 2026-10-19
> Owner: AMF

This face records public license links from captured BOOTH descriptions, refreshes linked
VN3 documents on request, and reads the resulting local evidence through AMF.

## Methods

All params contain `schemaVersion: "0.1"`. Unknown keys are rejected. Commands use the
application envelope's stable `commandId` and return a durable `taskId`.

| Method | Kind | Additional params | Result |
| --- | --- | --- | --- |
| `terms.refreshProduct` | Command | `productId` | Accepted task |
| `terms.refreshLibrary` | Command | none | Accepted task |
| `terms.getProduct` | Query | `productId` | Captured sources, document metadata, A–X items, states and warnings |
| `terms.checkUse` | Query | `presetId`, `productIds` | Per-product decisions, matching product IDs and the strictest combination decision |

Product IDs use `booth:<digits>`. A check takes 1–500 distinct IDs. Library refresh uses
recorded account-library or locally mapped products, excluding removed cards. Product refresh also serves a
recorded product that is outside the account library. Queries use stored evidence.

## Sources and acquisition

Pure link recognition reads anchors in the captured author-description container and URL text
in its visible description. Drive files and Docs documents become downloadable candidates;
VN3 site links, spreadsheets, folders and other HTTP(S) links remain `link_only` sources.
Missing description markup preserves the previous snapshot. Existing BOOTH dependency-link
recording retains its behavior.
Snapshots retain up to 256 sources. An excess records `captureProblem: "source_limit"` and
the `unsupported` state; a later complete capture replaces that evidence.

Refresh tasks share a dedicated lane. A product refresh uses its product ID as the lane key;
a library refresh runs exclusively in that lane. Network requests share a process-wide mutex,
with at least three seconds between a response ending and the next request. Cancellation
interrupts pacing and network waits. Restart recovery leaves an interrupted task for inspection.

The fresh HTTP client carries no cookies or credentials. Its endpoints are
`https://drive.usercontent.google.com/download?id=<id>&export=download` and
`https://docs.google.com/document/d/<id>/export?format=pdf`. Each response has a 30-second
deadline and a 20 MiB limit. A redirect to Google's account login is `not_public`; other
redirects are `unsupported`. The captured source URL remains available. PDF magic is checked
independently of MIME type.
Login/permission pages are `not_public`; other non-PDF bodies are `unsupported`; transport,
size and deadline failures are `failed`. Local PDF caching uses source kind, file ID and SHA-256.

## Parsing and evidence

`vn3-parser/1` uses a versioned closed vocabulary. Recognized version pairs and wording live
beside the parser. Text extraction uses the pure Rust `pdf-extract` library. Encrypted,
image-only or malformed PDFs are `unsupported`. An unrecognized VN3/generator version gives
`unrecognized`, retaining source/hash evidence with no item values.
The admitted generator/VN3 pairs are 1.10/1.10 and 1.11/1.10. The vocabulary covers Japanese
and English generated documents. Text extraction is bounded to 128 pages and 2 MiB of text.

The detailed section supplies A–X. Missing items, unknown headings/wording and disagreements
with the summary produce `unknown` items with original fragments and warnings. Each item
has a stable `vn3.<letter>.<name>` key, a closed value (`permitted`, `prohibited`,
`contact_licensor`, `unknown`), an optional closed condition and `rawText`.
Conditions are `content_rating_required`, `between_users_only`, `commercial_and_non_profit`,
`credit_preferred`, `modified_distinguishable`, `non_profit_only`, `private_use_only`,
`private_use_exception` and `limited_publication_only`. `summaryRawText` preserves the
corresponding summary evidence. An unalignable summary group makes its affected items unknown.

Document metadata contains generator/VN3 versions, language, content title, licensor,
credit notation, hashtags, agreement version and the original special note. Explicit
no-note markers are treated as empty notes. The source PDF stays in AMF's local data directory.
Lettered paragraphs inside X remain part of its original free text.

Within one product, Japanese evidence takes priority. Disagreement between Japanese and
English evidence, or between multiple documents of the same language, makes the affected
item `unknown` and records a warning. Failed or unrecognized candidate documents remain
visible and prevent a complete-evidence conformity claim.

## Use presets and decisions

Preset definitions are a versioned data file consumed by the query implementation.

| Preset | Required items | Commercial requirement |
| --- | --- | --- |
| `vrchat_personal` | C, J | none |
| `vtuber_individual` | A, C, O | explicit commercial permission for A |
| `vtuber_corporate` | B, O | commercial use |
| `game_development` | R; N as an advisory | embedded software with extraction-resistant content |
| `outfit_for_sale` | S, T | commercial use |
| `merchandise_publication` | P, Q | commercial use |
| `adult_content` | F | preserve all conditions, including content rating |

The strictness order is `nonconforming`, `unknown`, `contact_licensor`, `conforms`.
Per-product and combination results identify every relevant blocking item and required
condition. Unknown evidence never enters `matchingProductIds`. `no_vn3`, `not_observed`,
`not_public`, `unsupported`, `failed` and `unrecognized` remain separate source states.

`game_development` means R's software-embedding case: content is integrated so it cannot be
easily extracted. N remains a returned advisory about distributing modified asset data itself.
The corpus contains N-prohibited documents with R-contact and R-permitted choices, preserving
that distinction. Embedded use is decided by R; delivery of modified asset files is decided by N.

Special notes are an additive `hasSpecialNote` flag and original text on any decision,
including a conditional `conforms` result. Thus `adult_content` can report conformity
requiring `content_rating_required` while also presenting X for review. A special note
never removes a prohibited, unknown or contact-required item. Combination results retain
the notes and conditions of every selected product.

For example, call `terms.refreshProduct` with `{ "schemaVersion": "0.1", "productId":
"booth:6106863" }`, inspect its task through the existing task queries, then call
`terms.checkUse` with `{ "schemaVersion": "0.1", "presetId": "adult_content", "productIds":
["booth:6106863"] }`. `terms.getProduct` exposes the original evidence behind that decision.

## Storage

BDL format 0.11/migration 012 adds captured terms-source snapshots, product/source links,
content-addressed parsed documents and individual item rows. `term_observations` retains
product-level provenance using `extracted_by: "vn3-parser/1"`. Each stored column supports
source tracing, document display, cache reuse or a use-preset decision. Existing query
responses keep their contract versions and fields.
The request/result schemas and executable examples are in
[`schemas/terms/v0.1`](../../schemas/terms/v0.1). Candidate refresh tasks use the AMF task
store and place local PDFs under AMF's `terms/<source-kind>/<file-id>/<sha256>.pdf` directory.

## Document changelog

- 0.1.0 (2026-10-19): add captured license links, explicit local refresh tasks, versioned
  VN3 parsing and stored-evidence use/combination queries.
