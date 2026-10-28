# Unified library view v0.2

> Document version: 0.2.0
> Status: Candidate
> Updated: 2026-10-29
> Owner: AMF

This new `library.list` face adds an optional purpose filter to the existing local library
projection. [Library view v0.1](library-view-v0.1.md) remains readable and keeps its closed
request and response shapes. Product files and all mutation methods retain their prior versions.

## Request

The query uses `schemaVersion: "0.2"` with the same source, storage, text, availability,
category, reverse-description-link, offset and limit fields as v0.1. The only additions are:

| Field | Values | Omitted |
| --- | --- | --- |
| `use` | One of the seven [terms preset IDs](terms-v0.1.md#use-presets-and-decisions) | No terms check or annotation |
| `useMode` | `available` or `all`; requires `use` | `available` when a use is selected |

Null, unknown IDs, unknown modes and unknown fields are rejected. The `available` mode retains
allowed and conditional rows only. The `all` mode retains every otherwise matching row and
annotates it. The renderer sends v0.1 while no purpose is selected, preserving existing behavior.

## Row annotation and filtering

A row may add the closed object `use: {presetId,status}`. The status is one of:

| Status | Stored-evidence result |
| --- | --- |
| `allowed` | `conforms` with no returned conditions |
| `conditional` | `conforms` with at least one returned condition |
| `contact` | `contact_licensor` |
| `forbidden` | `nonconforming` |
| `unknown` | Unknown or incomplete evidence, including `not_observed` |
| `no_terms` | No confirmed BOOTH identity, or an explicit `no_vn3` snapshot |

The existing terms checker is the sole decision owner; this projection never reinterprets
VN3 wording. It reads local evidence and starts no network requests or refresh tasks. A product
row uses its recorded product ID. A local row uses explicit entry metadata or a confirmed
`mapping` source association. A filename/name suggestion does not establish permission.

Acquisition combines account products and local entries, applies the ordinary filters, checks
all remaining confirmed products once, annotates rows, and applies `useMode` **before facets,
total and pagination**. Unknown and no-terms rows never enter available results. The default
preserves source/category intersections, confirmed content grouping and physical presence facts.

Results keep the prior presentation shape with `schemaVersion: "0.2"`. Optional annotations may
be absent when no purpose was requested. The renderer validates new fields while retaining
v0.1 reads. A response to an explicit purpose request must carry that same preset on every row;
available-mode results must contain only `allowed` or `conditional`. Missing, stale, malformed
or foreign-purpose annotations fail the read instead of silently claiming usability.

The [request/response schemas and positive/negative vectors](../../schemas/library-view/v0.2)
are consumed by the TypeScript guards and the real native library wire test. Synthetic tests
cover six states, a local row without identity, pagination after filtering, retained v0.1 shape,
and the strictest-combination behavior of the shared terms checker.

## Desktop consumers

The library filter offers a purpose and either available-only or all-with-decisions. Cards and
list rows show the returned status; their existing details drawer remains the terms evidence
view. Changing a purpose clears the prior result while the next read is pending. Reverse-link
navigation saves, clears and restores the purpose alongside the other filters.

Recipe import defaults to VRChat personal use, checks every referenced BOOTH product together,
shows per-product blocking items/conditions/advisories and the strictest combination, and leaves
Save available regardless of the decision. Missing evidence offers an explicit Read terms action:
it captures that product's public page through the existing native catalog API, then invokes
`terms.refreshProduct` and follows the resulting task. Local metadata and terms are read again
afterward. Opening a Recipe starts only local queries, never an automatic refresh.

Sharing and importing display recorded BOOTH identity, title and shop (or explicit missing
metadata), binding modes, switch count, fit-object adjustment count, brightness-setting count
and SDK-fix count. They state that no files are included and recipients must own the materials.
The existing Recipe v0.5 exporter already obtains `shopName` from BDL and retains received
attribution when local metadata is absent; this slice reuses that producer.

The import additions preserve image/link inputs from [Recipe image sharing](recipe-image-share-v0.1.md).
Five-language copy, pure renderer models, native wire vectors and synthetic filter/import views
are included. Real-material acceptance remains an author check: filter for individual VTubing
and inspect Shinano as usable; import a Recipe referencing it and select game development to
see the contact-required R item. Synthetic evidence does not certify that specific product.

## Changelog

- 0.2.0 (2026-10-29): add versioned purpose filtering before pagination, optional status rows,
  and informational Recipe terms/content consumers while retaining the v0.1 face.

The recorded synthetic Chinese views show the [purpose filter](../development/evidence/library-use-filter.png)
and [Recipe import check](../development/evidence/recipe-use-import.png), including missing local product metadata.
They are UI evidence only, not permissions for any real product.
