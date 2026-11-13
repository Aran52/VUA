# Recipe selection drafts v0.1

> Document version: 0.2.0
> Status: Implementation baseline
> Updated: 2026-11-11
> Maintainer: AMF

The user accepted collecting library selections in drafts while the intended Recipe format is
still being designed (2026-10-08). This face provides a durable place for references. It does not
decide Avatar roles, Unity versions, attachment locations, dependencies or production operations.
The existing frozen [Recipe v0.3](../../schemas/recipe/v0.3) remains a distinct production contract;
a selection draft cannot enter its planning, execution or reproduction paths.

Schemas and synthetic vectors: [recipe-selection-draft/v0.1](../../schemas/recipe-selection-draft/v0.1).

## Methods

| Method | Kind | Meaning |
| --- | --- | --- |
| `recipeDraft.list` | query | List saved draft titles, IDs, revisions, timestamps and reference counts |
| `recipeDraft.get` | query | Read one draft and its current revision |
| `recipeDraft.selectionStatus` | query | Read current file-presence facts for the saved revision; no draft mutation |
| `recipeDraft.materialsForLine` | query | Resolve verified on-disk package sources for opening a production line, plus per-selection missing reasons; never infers Avatar or outfit roles |
| `recipeDraft.legacyCensus` | query | Read-only census of legacy Recipe v0.3 documents, parseability, verifiable references, unresolved assets and graph-setting flags |
| `recipeDraft.save` | command | Create at revision zero or replace the whole draft with an exact base revision |
| `recipeDraft.addSelection` | command | Append references with an exact base revision and return actual new/existing counts |
| `recipeDraft.migrateLegacy` | command | Idempotently copy verifiable material references from legacy Recipe v0.3 documents into `recipe-draft-legacy-*` drafts while preserving original documents |

All requests require `schemaVersion: "0.1"`; unknown fields are refused. Draft IDs use
`recipe-draft-` plus 1–100 ASCII letters, digits, hyphens or underscores. A title is trimmed,
nonblank and at most 120 Unicode characters. A draft holds 0–512 references; an empty draft is
still a saved collection. Base revisions are nonnegative safe integers below the maximum so
that the next stored revision remains safe. A conflict preserves the stored document and
returns `vua.recipe_draft.revision_conflict`; clients reload before proposing another save.

## Provisional collection shape

A document has `schemaVersion`, `kind: "selection_draft"`, `draftId`, `title`, store-maintained
`createdAt` and `selections`. A reference has `identity`, `displayName`, `source` and optional
nullable `variantName`, `shopName` and `warehouseItemId`. Display and optional text are bounded
to 512 Unicode characters. The response canonicalizes omitted optional reference fields to null.

Cloud selections reference a numeric `booth:` identity and have no warehouse ID. Local selections
reference a lowercase SHA-256 identity and may retain their existing warehouse entry ID. No
absolute material paths, paid asset bodies, credentials or production claims are collected.
Names are display facts, not inferred Avatar roles or declarations of source authenticity.

Deduplication uses `(source, identity, variantName, warehouseItemId)`. Different variants and
separate local entries remain separate references even if product/content identity matches.
Duplicate submissions count once. Append returns `addedCount` and `existingCount`, whose sum
equals the unique submitted count. The client verifies those counts before showing success.
Each accepted save increments the revision; an append containing only existing references is
an accepted save and reports zero newly added references.

## Persistence and UI

The acquisition/AMF service owns a separate `recipe-selection-drafts` directory beside the
managed warehouse. It uses the existing whole-document store with flushed writes, atomic file
replacement and optimistic revisions, without mixing draft files into the production Recipe
directory. List errors and corrupt documents are surfaced as failures, never an empty library.
There is no silent promotion or automatic continuation into production.

The library selection dialog creates a draft or appends to one. The Recipe page lists these
collections separately and allows viewing, renaming and removing references. A verified save
invalidates the list even if the composing dialog is closed. Reload explicitly replaces unsaved
editor state with the saved draft. Removing a reference changes the collection, not the file.

The separate selectionStatus read returns draftId, revision and sequential per-selection indices,
storedCopies, presentCopies and state (`present`, `missing`, `not_stored`). Metadata presence does
not establish production qualification. Consumers refuse mismatched revisions/counts, keep read
failure distinct from absence, and do not assign saved indices to unsaved edited references.
Local-file removal preserves the document under the [maintenance ruling](../decisions/library-file-maintenance.md).
Content-pinned local references do not follow a changed original's new fingerprint automatically.

Draft promotion, formal roles/settings, relink/version-switch semantics and the final production
Recipe design remain undecided. No draft-delete operation is defined in this baseline.

## Verified line prefill and legacy Recipe v0.3 migration

`recipeDraft.materialsForLine` prepares package-source options for the open-line dialog without
inferring Avatar, outfit or dependency roles. For each saved selection, AMF checks active
managed-warehouse containment, regular-file shape and actual on-disk SHA-256 digests (including
safe `.unitypackage` members inside original `.zip` archives under `drop_expanded_duplicates`)
rather than trusting database digest rows alone. Verified sources expose `sourceId` (matching
`amf.production.list` `sources` / `amf.production.prepare`), `itemId`, `itemName`, `name`, `productIds`
and strictly increasing `selectionIndices`. Unresolved selections appear in `missing` with
sequential `index` and `reason` (`not_stored`, `missing`, `content_changed`, `no_package`). The
player confirms roles explicitly when opening a line.

`recipeDraft.legacyCensus` and `recipeDraft.migrateLegacy` support audited, non-destructive
migration from legacy Recipe v0.3 documents in `<AMF data>/production/recipes`. Census reads
without mutation and reports document counts, parseability, `formatVersion`, deterministic
`targetDraftId` (`recipe-draft-legacy-*`), `existingDraftRevision`, `verifiableSelections`,
`unresolvedAssets` and `hasGraphSettings`. Migration writes only verifiable `booth` product
references and on-disk hash-verified `warehouseItemId` copies into `targetDraftId`, leaves
already-complete drafts untouched without bumping their revision, keeps original Recipe v0.3
documents intact, and surfaces `entity_only`, unsupported provider or unverified local references
in `unresolvedAssets` instead of guessing or translating graph settings into production settings.

## Evidence

Synthetic consumers exercise request/response schemas, strict guards, creation and restart,
independent production/draft stores, deduplication, append counts, revision conflicts, corrupt
document failure, verified line-source resolution (including expanded ZIP deduplication and
tampered-file rejection) and non-destructive legacy migration idempotence. TypeScript clients
refuse mismatched draft receipts and unavailable lists.
These checks do not establish human UI or real-material production acceptance.

## Document changelog

- 0.2.0 (2026-11-11): add verified `recipeDraft.materialsForLine` line-source prefill without role inference, plus read-only `recipeDraft.legacyCensus` and non-destructive idempotent `recipeDraft.migrateLegacy`.
- 0.1 (2026-10-08): implement durable material-reference collections while preserving separate production Recipe authority.
