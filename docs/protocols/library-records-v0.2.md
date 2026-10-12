# Library card removal v0.2

> Document version: 0.2
> Status: Implementation baseline (2026-10-12)
> Owner: AMF

`library.removeCards` is a command with closed params `{schemaVersion:"0.2", targets}` and an
Application command ID. Each target is exactly `{kind:"entry"|"product", id}`; there are 1–200
unique targets. Entry IDs are actual warehouse identities (including downloaded entries);
product IDs are observed `booth:` numeric identities. The response echoes the exact ordered targets.

BDL validates all identities in one transaction before recording removal. Missing/invalid targets
roll back the complete selection. Identical command replay returns its receipt across restart;
changed arguments conflict. Persistent format 0.9/migration 010 retains the prior tables, adds
card tombstones and separate command receipts, and keeps the older local-record method unchanged.

Removal hides complete entry cards or product cards from `library.list` before filtering, counts
and pagination. Missing files do not prevent removal. It does not unlink files, delete catalog,
copy/source/warehouse records, retarget Recipe/draft references, revoke account membership or
cancel tasks. BOOTH observations/sync do not clear tombstones. The library confirmation names the
selected cards and count and explains file/reference preservation. Physical deletion is the
separate [file-maintenance command](library-maintenance-v0.1.md).

An entry tombstone removes that entry's contribution from unified listing and reconciliation,
including its copies previously shown under an account product. The account product can still
exist independently. A product tombstone hides its product presentation; unrelated unverified
local entries remain independent. Frozen detail/reference faces retain the underlying identities.
Restoration UI remains future work; a new explicit local import creates a new entry under existing
intake rules. Do not infer restoration from a metadata refresh or background download completion.

Schemas/examples are [library-records v0.2](../../schemas/library-records/v0.2). TypeScript consumes
the same target vector as the Rust provider wire test. Evidence is synthetic command/list/restart
and isolated UI verification; real-account/material and human N5 review remain open.

## Document changelog

- 0.2 (2026-10-12): add persistent card removal for local and account product presentations,
  independently of file deletion and the existing v0.1 local-record method.
