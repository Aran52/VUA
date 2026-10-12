# Library intake v0.2

> Document version: 0.2
> Status: Implementation baseline (2026-10-12)
> Owner: AMF

`library.importPaths` accepts an explicit native file/folder drop as a command with closed params
`{ schemaVersion: "0.2", sourcePaths: string[], autoGenerate?: boolean }`. Paths are absolute,
nonempty, NUL-free, unique (Windows case/separator aliases rejected), with at most 200 inputs.
The acceptance is `{ schemaVersion: "0.2", operation: "library.importPaths", taskId, correlationId }`.
Command IDs use a separate idempotency namespace; replay does not create duplicate entries and
different selections cannot reuse the same accepted command. A failed/unknown submission retry
retains the drop's command ID; a new explicit drop has a new ID. Folder-only
[v0.1](library-intake-v0.1.md) and frozen `warehouse.import` retain their existing faces.

The host's [native drop face](desktop-library-v0.1.md) obtains paths from Electron-backed `File`
objects. Renderer sends the resolved paths through its typed Gateway. An ordinary file creates
one managed entry containing only that file; its parent directory and siblings are never imported.
A folder creates one entry preserving relative paths. Host input normalization removes duplicate
paths and descendants already covered by a selected folder. AMF remains the copying authority.
Selected links/reparse points, special files and inputs inside or containing the managed warehouse
fail before entry creation. Child link skipping, size limits, original preservation, hashing,
managed ZIP expansion and archive lineage follow the v0.1 intake rules without granting production
qualification or BOOTH ownership.

The normal task channel owns progress, cancellation and terminal state. Copying, hashing and ZIP
expansion check cancellation; completed copies remain queryable, while an interrupted task follows
the task store's explicit inspection/recovery rules. It never resumes implicitly. Internal import
reports retain the existing `foldersRequested`/`foldersImported` counters; for this operation each
counts selected paths, including individual files. The host library refreshes from durable facts
after its registered import task settles.

Dropping does not enable an absent AMF. The host explains the prerequisite and retains the native
selection until the user explicitly enables/imports or cancels. Canceling the prompt neither changes
the module choice nor submits an import. Disabling AMF keeps the native path resolver and import
command unavailable. Browser previews cannot simulate native copying; remote websites have no
preload or privileged intake face.

Schema vectors are consumed by both TypeScript Gateway guards and the actual Rust wire tests.
The [controlled drop smoke](../../apps/desktop/scripts/smoke-library-drop.mjs) uses an isolated
profile, real Main/preload/providers and synthetic native files. This scope does not close N5's
real-material or human-review acceptance; raw local profiles and screenshots are not tracked.

## Document changelog

- 0.2 (2026-10-12): add mixed native file/folder intake with explicit AMF enablement, scoped copy semantics and the existing observable/recoverable task path.
