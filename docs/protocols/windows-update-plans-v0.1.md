# Windows update plans v0.1

> Version: 0.1 · Candidate · Owner: Integration · Reviewed: 2026-10-26

This additive Desktop v6 face handles a compatible VUA/AMF pair when independently updating
either component would strand the other. It preserves [distribution v0.2](windows-distribution-v0.2.md)
and its independent component actions. No new signing key, remote command or renderer-selected
URL is introduced. `packages/contracts/src/update-plans.ts` owns the typed face; the snapshot
schema is `schemas/windows-update-plans/v0.1/snapshot.schema.json`.

## Proposal and user choice

`updatePlans.propose()` obtains both fixed authenticated Beta indexes. It considers strictly
newer Host and AMF releases with matching UI/React/Application/Bridge compatibility and an AMF
reader that accepts the installed BDL writer. Select the highest host version, then highest AMF
version, by SemVer precedence. Independently verify and bind both referenced inventories.

The proposal has an opaque digest derived from the exact installed/target manifest pair, both
installed versions, two ordered targets (host then amf), byte estimates and restart activation.
It does not download payloads or activate either component. `stage(id)` requires that reviewed
identity. `apply(id)` requires both staged payloads, the same installed selection and explicit
user choice. A host-only action never delegates to this face or changes AMF implicitly.

States are `idle`, `checking`, `proposed`, `downloading`, `staged`, `waiting-for-idle`,
`activating`, `interrupted` and `failed`. Cancellation affects this plan's helper only. Download
progress identifies the current component; a cancelled proposal does not display its late
success. Failed feeds and missing compatible pairs never become a latest-version claim.
Snapshot/propose/stage/apply/restore return the current snapshot; subscriptions are disposable.

## Restart, health and recovery

Payloads remain immutable and content addressed. Staging never changes `active.json`, loaded
code or a shared runtime's bootstrap. Runtime bytes are reused when verified, and partial target
files and verified archive caches count toward reuse. Missing/corrupt shared runtime files may
be downloaded while running, but extraction/repair and bootstrap switching wait for exit.

Native code acquires operation/running locks, rechecks the base pair, verifies complete target
files and records a flushed activation journal before switching bootstrap/active selection.
A mounted main renderer's successful Gateway round trip and real ready provider(s) acknowledge
the exact installed builds. The launching helper allows 45 seconds for that acknowledgement.
Exit/failure/timeout before acknowledgement restores the retained compatible pair and launches
it. Ordinary launch recovers an interrupted bootstrap/selection transaction before validation.
There is no automatic application of an interrupted staged update.

Independent restore changes only its named component, and is offered only when the retained
payload/data reader is compatible with the other current component. `updatePlans.restore()` is
a separate confirmed choice to restore both versions after a pair update. Reader-incompatible
rollback fails closed with a repair outcome; this contract does not authorize opening migrated
data with an old reader or erasing a user's data.

The root launcher must itself support recovery v0.2. Before component/plan commands, native code
binds that executable's hash/length to a retained signed host inventory and probes its closed
capability response. A legacy or unbound root requires the transition installer. Updating a
payload-local helper alone does not silently qualify an old root launcher. The existing installed
layout and v0.1 API remain usable while transition and actual signed delivery are verified.

## Evidence boundary

Native/controller checks cover dependency cycles, explicit identities, staged selection,
cancellation, corruption, bootstrap interruption and compatible restore. Real signed remote
delivery, actual process health/restart, migration/backup cases, clean Windows and human review
remain U6 acceptance gates. A passing synthetic plan test does not pass those gates.
