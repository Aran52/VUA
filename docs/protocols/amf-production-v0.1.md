# AMF material-first production v0.1

> Document version: 0.33.0
> Status: Candidate
> Updated: 2026-10-23
> Owner: AMF

This face implements the first N3 production path. Users select local library inputs before a
project is prepared. AMF creates an independent managed work project; no external project is
modified. It uses real SDK/MA packages and Unity-observed prefab candidates. The host owns module
lifecycle and tasks, not material roles or assembly semantics. It does not replace frozen
`production.*`, Recipe, plan/job, or Bridge v1-v4 faces.

## Methods

All params have `schemaVersion: "0.1"`. Unknown keys are rejected.

| Method | Kind | Additional params | Result |
| --- | --- | --- | --- |
| `amf.production.list` | Query | none | Local UnityPackage source identities, detected admitted Editors, retained run summaries, `capacity`, `running`, `busy` and the waiting `queue` |
| `amf.production.setCapacity` | Command | `capacity: 1 \| 2 \| 3` | Saved `capacity` with `schemaVersion: "0.1"` |
| `amf.production.closeEditor` | Command | `runId`, `confirmed: true` | Durable task: protect the on-disk project, save open scenes/assets, and wait for Unity to release it |
| `amf.production.get` | Query | `runId` | Run facts, actual candidates, timed steps, checks, recovery state, version record summaries, usual step durations and the running step's progress |
| `amf.production.faceTrackingPlan` | Query | `runId`, `menuId` | Exact imported menu, active owned Avatar, project/revision digest, interactive execution and manual replay/trust warning |
| `amf.production.faceTrackingPatch` | Command | `runId`, `menuId`, `planDigest`, `confirmTrustedCode: true` | Durable task: snapshot, creator session, independent reinspection, automatic restoration on failure |
| `amf.production.prepare` | Command | `sources: [{sourceId, role}]`, `editorPath` | Durable `taskId`, `runId` acceptance |
| `amf.production.plan` | Query | `runId`, `selection` | Exact digest-bound selection, effective input roles, `rebuild`, pending Recipe `adjustments` and the estimated synced `parameterBudget` |
| `amf.production.assemble` | Command | `runId`, `planDigest`, `selection` | Durable task acceptance |
| `amf.production.reinspect` | Command | `runId` | Durable task acceptance; continues a stopped run; never automatically resumes an interrupted assembly |
| `amf.production.changeSources` | Command | `runId`, `sources: [{sourceId, role}]` | Durable task changing the line's own materials behind a restore point; the line plans again |
| `amf.production.delete` | Command | `runId` | Durable task deleting the line's work project, release record and run record |
| `amf.production.rollbackPlan` | Query | `runId` | Digest-bound restore point, the optional `prePatch` target of a marked line, current drift notice and current-state preservation promise |
| `amf.production.rollback` | Command | `runId`, `planDigest`, optional `target: "restorePoint" \| "prePatch"` | Durable task acceptance; protect current state, restore the reviewed target, then inspect in Unity |
| `amf.production.revisionPlan` | Query | `runId`, `revisionId` | Digest-bound replay review: the steps a restore to that version takes and skips, and the foreign-edit warning |
| `amf.production.restoreRevision` | Command | `runId`, `revisionId`, `planDigest` | Durable task acceptance; records the current state as a version, then replays the reviewed one |
| `amf.production.open` | Command | `runId` | Durable task acceptance; open project, select result and request official SDK panel |
| `amf.production.listRecipes` | Query | none | Retained local Recipe names and revisions |
| `amf.production.getRecipe` | Query | `recipeId` | Validated saved Recipe document |
| `amf.production.saveRecipe` | Command | `runId`, `title` | Save a new Recipe from current verified scene facts |
| `amf.production.prepareRecipe` | Command | `recipeId`, `editorPath` | Durable preparation task using resolved original inputs and pinned dependencies |
| `amf.production.recipeSharePlan` | Query | `recipeId` | Digest-bound referenced-material source choices and unresolved candidates |
| `amf.production.exportRecipe` | Command | `recipeId`, `planDigest`, `sources` | Closed share declaration, paste text and JSON; remember explicit source choices |
| `amf.production.previewRecipeShare` | Query | `text` | Validated received declaration, without saving or execution |
| `amf.production.importRecipe` | Command | `text` | Idempotently saved local received Recipe |
| `amf.production.recipeApplyPlan` | Query | `runId`, `recipeId`, optional `avatarGlobalObjectId` | Observed differences, retained outfits, selectable owned Avatars, target identity and digest |
| `amf.production.applyRecipe` | Command | `runId`, `recipeId`, `planDigest`, `choice`, optional `avatarGlobalObjectId` | Durable overlay or same-project new-Avatar task |
| `amf.production.previewImage` | Query | `runId`, `path` | One preview PNG that the run reports, as `mediaType: "image/png"` and base64 `data` |
| `amf.production.releaseState` | Query | `runId` | The run's turntable (`frames`, `size`, `bakedAt`, `current`), its last 10 uploads newest first and `editorOpen` |
| `amf.production.turntableFrame` | Query | `runId`, `index` (0–59) | One turntable frame as `mediaType: "image/png"` and base64 `data` |
| `amf.production.bakeTurntable` | Command | `runId` | Durable task baking the 60-frame turntable of the finished Avatar |
| `amf.production.upload` | Command | `runId`, `avatarName`, `rightsConfirmed: true` | Durable task building and uploading the finished Avatar with the VRChat SDK |
| `amf.production.planTextureMemory` | Query | `runId`, `platform: pc / android`, `targetRank: excellent / good / medium / poor` | Digest-bound current inventory, suggested importer changes, predicted bytes/rank and `reachesTarget` |
| `amf.production.applyTextureMemory` | Command | `runId`, `platform`, `targetRank`, `planDigest`, `textures` | Durable confirmed subset task; snapshot, version record, importer changes, independent reinspection and automatic restore on failure |
| `amf.production.planLighting` | Query | `runId` | Digest-bound lilToon brightness alignment: the Avatar's reference limits, the outfit entries that differ and the shared materials left alone |
| `amf.production.applyLighting` | Command | `runId`, `planDigest`, `materials` | Durable task acceptance; aligns the chosen reviewed materials behind a restore point and rechecks the project in Unity |
| `amf.production.listFits` | Query | none | Retained saved fits: name, times, object count and the garment/Avatar content identities each belongs to |
| `amf.production.saveFit` | Command | `runId`, `sourceId`, `name`, `changesDigest` | The stored fit; binds the reviewed `fitChanges` entry, a stale review is refused as `plan_drift` |
| `amf.production.renameFit` | Command | `fitId`, `name` | The renamed fit |
| `amf.production.deleteFit` | Command | `fitId` | The deleted `fitId` |

Roles: `auto`, `avatar`, `outfit`, `dependency`. The player path starts with `auto` sources.
Unity discovers actual prefabs. `recommendAssembly` in the contracts package starts the plan with a
stated recommendation over those candidates: for the Avatar, the creator's base for outfits (改変),
then the prefab named after its package, then the package's top-level prefab; for each outfit, the
only candidate or the one that names the Avatar. Equally plausible candidates or a static
attachment target require a choice. Assembly requires an observed Avatar and at
least one outfit. Unselected automatic inputs are explicitly shown as `import_only`, not silently
claimed as assembled dependencies. Source IDs resolve only through the module's present local
library; no renderer-provided source or project paths are accepted. Editor paths are reverified
against executable identity: only full `2022.3.22f1` or `2022.3.22f1c1` is admitted.

`selection` contains `avatarPrefab` (asset path) and `outfits` with `sourceId`, `prefabPath`,
`mode` (`merge_armature` or `attach_to_bone`), `bone`, `toggleName`, and `active`. Every chosen
prefab must occur in the run's observed candidates for that source. Bone attachment uses an
observed humanoid bone name, not an arbitrary hierarchy guess. Empty `toggleName` preserves the
material's own menu without adding a new toggle.

Direct UnityPackages and safe UnityPackage members of local ZIP files are selectable without
changing the library originals. Import preflight rejects differing content at a shared path,
inconsistent GUID metadata, file/directory conflicts and Windows case collisions before copying.
Malformed unrelated ZIP files are not offered as package sources. Each listed source names the
BOOTH products its library entry maps to (`productIds`, from download provenance and local
matches; empty when none is known), so a line's materials can be checked against [VN3 terms](terms-v0.1.md).

The plan digest binds the observation revision, candidates, chosen inputs, Editor, project
fingerprints and selection. Reinspection invalidates every earlier confirmation.
Reinspection also reads saved scene ownership and actual object, binding and switch state.
When those relationships are verifiable, editing starts from `facts.observedSelection`, including
supported changes made in Unity. The retained requested `selection` is not proof of current scene
state. Changed or missing relationships are shown before another assembly is reviewed.
Confirmation never survives drift. Preparation may download declared SDK/MA packages into an
isolated package environment and import the chosen files only into the managed work project.
Unknown extra dependencies are reported, not guessed or automatically repaired.

## Applying to a retained Avatar

`recipeApplyPlan` compares a saved declaration with a verified owned Avatar in a managed run.
Omitting the target selects the active Avatar; an explicit identity must resolve to exactly one
observed owned object. Display names never identify a target. Changing the selection recomputes
the comparison before execution, and the execution request retains that identity.
The exact Editor, package versions, prepared content hashes and prefab identities must match;
missing inputs and object ambiguity are distinct from value conflicts. No files or run state change
during comparison. The digest binds the Recipe revision, observations, current project and all
comparison facts. `applyRecipe` accepts `recipe_wins`, `avatar_wins` or `new_avatar`; cancellation
closes the review without an execution request.

An overlay retains outfits absent from the declaration. Conflicting supported values either use
the Recipe or retain the current observation. An empty switch name does not delete an owned
switch. Different prefab identities are not resolved by value priority. A new Avatar is instantiated
in the same managed Unity scene with independent ownership keys; previous Avatars remain intact.
All choices use the protected assembly transaction and verify the actual target and effective
settings before reporting success. Repeated overlays reuse the same object identities. Unchanged
and unmentioned outfits are not reconfigured. Continued ordinary editing also binds the active
object and changes only selected settings; it does not require outfits used only by other Avatars.
Omission of an existing outfit or clearing an existing switch is not an implicit removal.

This Candidate currently compares owned managed Avatars and already prepared inputs. Adding
new input packages to a retained project, selecting unowned scene targets, explicit removal,
and external-project intake remain separate work. Old development work projects need the updated
Candidate Bridge and reinspection before this new capability can be used.

### Another base on an assembled line

`plan` reports `rebuild: true` when the selection names another Avatar prefab than the one Unity
observes on an assembled line. Assembling it rebuilds the line: after the restore point, AMF
removes its work scene (`Assets/VUA`), Unity prepares a fresh one, and the new base and every
chosen outfit are assembled as in a first assembly, so every `outfit` input needs its target.
Imported materials and packages stay. If a step fails, the restore point brings the earlier scene
back and the run keeps its earlier observations; after success, a reviewed restore returns to the
earlier base. With the same base, assembly binds the active object and writes only new or changed
outfits, as before.

## Project fingerprints

The run record's project fingerprint covers `Assets`, `Packages` and `ProjectSettings`
of the managed work project; symbolic links are refused. AMF computes it with the
cached v2 algorithm: each file's SHA-256 is remembered by path, size and modification
time to the nanosecond, so only files that changed are read again. The total combines
each file's sorted path, length and content hash and carries the prefix
`vua-project-tree-v2:`. Values written before the cache carry the prefix `sha256:`
(the uncached v1 algorithm, which other flows still use unchanged).

The cache lives in `.vua/fingerprint-cache.json` inside the work project, outside the
fingerprinted scopes. It names its `version`, is written atomically (`.pending`, then
rename) and bounded in size; a missing, damaged, future-version or oversized cache
counts as no cache. The cache decides only whether a file's bytes are read again: a file
is re-read when its size or modification time differs from its cached entry, and every
write moves the modification time on NTFS, where work projects live. An added, deleted or
renamed file always moves the value.

A run record that still carries a v1 value is compared by computing v1 once for that
comparison; an unchanged project is never reported as changed because the algorithm
moved on. Once the record matches, saving a Recipe, assembly and the next check rewrite it
with the v2 value; until then the turntable and upload checks repeat the v1 comparison.

## Facts and recovery

Run phases: `preparing`, `inspecting`, `ready`, `assembling`, `restoring`, `succeeded`, `failed`,
`cancelled`, `stopped`, `inspect_required`. Steps have stable keys, readable input names, actual
outcomes and their times. Persisted nonterminal work becomes `inspect_required` after restart;
reinspection or a new preparation is an explicit action. [Stop](#progress-stop-and-continue) ends
a running Unity batch run at once; other work stops at the next step boundary.

Before assembly, the managed project is snapshotted and the exact confirmation/input facts are
rechecked. Failures retain sources and records, report restoration separately, and do not delete
the player's materials. A successful receipt must report actual Avatar/MA/menu relationships.
The retained result records the input hashes, exact Editor and installed VPM versions, snapshot
identity, actual instance targets and per-step state. SDK descriptor recognition is an observation;
official SDK build/upload checks remain `not_run` until actually performed. No upload is automated.

A retained `snapshotId` also enables user-requested restoration. The review digest binds the
run, observation revision, Editor, managed project, the **chosen restore target** and the
**current** project fingerprint. A line carrying the creator-patch marker keeps a second full
copy from before the patch (see [Face tracking](#face-tracking-source-evidence-and-creator-sessions));
its review lists that copy as `prePatch` with its own digest, and `rollback` restores it when
`target` names `prePatch`. If external edits exist, the review reports `currentChanged`; no
current edit is discarded silently. Execution refuses a stale review. It snapshots the current
project and persists that recovery identity before restoring the reviewed state. Actual Unity
inspection must succeed before the run becomes `ready`; partial restore, cancellation or failed
inspection attempts recovery to the protected current state and reports its outcome separately.
The protected state remains the next restore point, making restoration itself reversible.
Interrupted restoration requires explicit reinspection. Original library inputs are never
restoration targets.

### Version records

Every protected change — a material change, an assembly, a rebuild on another base, an SDK fix,
a lighting alignment, texture tuning and every restore — first records the line's state as a **version record**
and keeps it on the run (`revisions`, newest first, at most 50; the oldest are discarded beyond
that). A record carries its identity, the change it precedes (`materials`, `assembly`, `rebuild`,
`sdk_fix`, `face_tracking`, `lighting`, `texture_memory` or `restore`), when it was taken, and the state a replay needs: the source
choices and each source's content identity (original package filename and SHA-256), the assembly
selection, the brightness limits written into each in-use lilToon material, the SDK fixes applied
so far and the observed VPM dependency versions. A display `summary` names the Avatar, outfits and
sources and counts the adjusted materials. `editedInUnity` is true when the project's files no
longer matched the run record at record time. `amf.production.get` reports each record without
its replay state. Run records written before version records read with one synthesized `assembly`
version built from their retained restore point; what the restore point never recorded stays empty.

A change that fails and whose automatic restore brings the project back discards the version it
recorded: the state never changed. The full-copy snapshot under `.vua/snapshots/` remains the
hard guarantee for failed or interrupted changes, and exactly one is retained per line — with one
exception: while the line carries the creator-patch marker, the copy taken before the patch is
retained next to the current restore point, so later protected edits cannot make the pre-patch
state unreachable. Once the run record naming the retained copies is persisted, every
unreferenced snapshot directory is deleted. Pruning validates each entry as a plain directory
inside the project's own snapshot folder, never follows links, and only records diagnostics
(`snapshot-prune.json` in AMF's production data root) when an entry cannot be removed — the
change itself never fails over it.

`revisionPlan` reviews a restore to a recorded version: the steps the replay takes (`materials`,
`assembly` — with `rebuild` — `lighting`, `sdk_fix`) and which it skips, the record's summary, and
a `warning` when `editedInUnity` reports edits the restore would replace. Every recorded source is
verified against the library by content hash; a missing or changed original is refused as
`vua.amf_production.revision_source_missing` naming the package. The digest binds the run, its
revision, the current project fingerprint and the version's state hash; a discarded version, a
changed line or a changed project is refused as `plan_drift` and asks for a fresh review.

`restoreRevision` records the current state as a `restore` version first, so the restore is itself
reversible, then protects the current project behind a full-copy snapshot and replays the recorded
state with the existing steps: changed materials through the material-change path, the recorded
selection assembled, the recorded lighting values written back and verified in Unity, and the
recorded SDK fixes that the line has not applied yet (applied SDK fixes cannot be unrun; a restore
can only add the recorded ones). The assembly step reports `rebuild` when the work scene starts
fresh: the version has another base, lacks an outfit or switch the line wears now, or recorded no
assembly at all — then the line plans again, as after a material change. A material change starts
the scene fresh by itself. A version record holds the line's state before its change, so the
version taken before a line's first assembly has no selection. A final Unity inspection confirms the
restored project. Any failed step names itself and restores the full copy taken before the replay.
A run whose change VUA interrupted mid-step must be reinspected or restored through `rollback`
first; replay review answers `inspect_required` there.

### Saved fits

After every successful assembly the Bridge records a **fit baseline** in the work project
(`.vua/amf-fit-baseline.json`): every object of each assembled garment instance with its local
position, rotation and scale, addressed by a path relative to the garment instance root
(`/` separated child names, `name#n` for the n-th same-name sibling, empty for the root itself).
The prefab's own offsets are part of the baseline. Inspections and every save of the work scene
in the Editor compare the current local Transforms against the baseline; an object counts as
moved past 0.0001 units of position or scale, or 0.01° of rotation. Moved objects are reported as
`fitChanges` per garment with their new local Transforms; paths whose object the player deleted
are listed separately as `missing` and are not errors. `amf.production.get` carries the current
`fitChanges` — the newer of the last inspection's facts and the Editor's scene-save report, each
entry with its `changesDigest` — and the `availableFits` each selected outfit can use.

`saveFit` stores the garment's overall state — the baseline with the reviewed moves folded in and
the missing paths dropped — as a named **fit** under `fits/` in AMF's production data root,
independent of any line. The `changesDigest` binds the exact reported changes; a stale review is
refused as `plan_drift`. A fit's identity is the garment source's content SHA-256 and prefab path
together with the Avatar source's content SHA-256 and prefab path, so another line with the same
Avatar and the same garment content recognises it, while changed content or another Avatar never
matches. Saving again for the same identity updates the fit in place, keeping its `fitId`. Names
are a single line of 1–80 characters; the default is the garment's item name plus `For` plus the
Avatar's name, without whitespace (`HairClipForMeiyun`). At most 500 fits are retained, each with
at most 2048 objects. Saving updates the baseline to the saved state and clears the garment's
reported changes. `renameFit` and `deleteFit` manage retained fits. None of the four methods
starts Unity or enters the line's Unity queue.

Every assembly plan — a fresh plan, a Recipe application and a version-record replay — lists the
fits that apply per outfit (`fits: [{sourceId, fitId, name}]`), and the plan digest binds their
content. Assembly sends each fit's objects to the Bridge, which writes them into the new instance
by path; paths the prefab no longer has are skipped and reported, never a failure. The baseline
is recorded after application, so an applied fit is not itself reported as a move. A version
record carries the fits its state was assembled with, and a restore replays that recorded content
even after the fit itself changed or was deleted. Recipes carry fits in `adjustments.fits`; a
Recipe's fits take precedence over stored fits for the garments they name.

## Progress, stop and continue

Step keys are `project`, `materials`, `inspect`, `reinspect`, `snapshot`, `assembly`,
`protect_current`, `restore`, `turntable`, `upload`, `sdk_fix`, `face_tracking`, `lighting` and `texture_memory`. Each step records `startedAt` and `endedAt` as UTC RFC 3339
times; records written before step times carry `null`. Step states are `running`, `succeeded`,
`failed` and `stopped`. `amf.production.get` shows the unfinished step of work whose task no
longer runs as `interrupted`, next to the run phase `inspect_required`.

`amf.production.get` also reports `usualDurations`: for each step key, the median in whole
seconds of the succeeded steps of the 30 most recent runs on this computer. `progress` is `null`
unless a task works on the run (`preparing`, `inspecting`, `assembling`, `restoring`); then it
names the running `step` with its `name`, `startedAt`, `elapsedSeconds`, `usualSeconds` (`null`
before a step of that kind has finished here) and, during a Unity batch run, `unity`.

Task progress events of an `amf.production` task carry `operation: "amf.production"`, `runId`,
`step`, `name`, `startedAt` and, during a Unity batch run, `unity`. During `materials`, `name` is
the material being imported. `step: "open"` marks the Editor opening for the SDK handoff; the
Editor shows its own progress window there.

`unity` comes from the batch run's own log `.vua/logs/<commandId>.log`, read every half second.
`stage` is one of `starting`, `license`, `packages`, `importing`, `compiling`, `bridge`,
`previews`, `finishing` and `fingerprint`, from Unity's own messages, the [Bridge v5](unity-bridge-v5.md) stage
lines, the preview folder and the result file. The last one is VUA's own: after the
Editor exits, `fingerprint` names the [project-file check](#project-fingerprints)
while it reads the work project. Stages move forward; importing and compiling
alternate. While importing, `current` counts the work-project assets Unity has started to import,
and during the first preparation `total` is the number of material files and their folders.
`lastActivityAt` is when the log last grew, or when Unity started until it writes. A page says
Unity has been quiet since that time after two minutes without growth. An event goes out when the
stage changes, when the count moves (at most once a second) and when the activity time moves (at
most every 15 seconds). Events and `get` carry stages, counts and times; log text and paths stay
in the log. The [progress](../../schemas/amf-production-flow/v0.1/progress.schema.json) and
[status](../../schemas/amf-production-flow/v0.1/status.schema.json) schemas carry the vectors.

Stop is task cancellation through `task.requestCancellation`. During a Unity batch run, VUA ends
Unity's whole process tree through its Windows job object within half a second. The work
project, its imported materials and the log stay. A stopped assembly or restoration recovers the
protected project state, as a failure does. The run becomes `stopped` once its materials are in
the work project, otherwise `cancelled`, which a new preparation follows. Continue is
`amf.production.reinspect`: the run is `inspecting` while Unity reinspects the retained project,
preparing its work scene when the stopped run had none, and `ready` afterwards. Confirmations
from before the stop are invalid. Unity's batch mode has no pause, so Stop and Continue serve as
pause, as the author approved on 2026-10-17. Before and after each batch run, VUA removes a
`Temp/UnityLockfile` that no Editor holds, so the next run and the SDK handoff find the project
closed.

### Queue

Unity work has a capacity of 1, 2 or 3, defaulting to 1. The commands that start it — `prepare`, `prepareRecipe`,
`assemble`, `reinspect`, `sdkFix`, `faceTrackingPatch`, `changeSources`, `applyRecipe`, `rollback`, `restoreRevision`, `open`, `delete`,
`bakeTurntable`, `upload`, `closeEditor` and `applyLighting` — are accepted while another job runs: their task stays `queued` and starts by itself,
in request order, when admitted by these rules:

- At most `capacity` jobs run at once. Capacity 1 runs one job at a time in submission order.
- At most one job runs for a given `runId`.
- `upload`, `open` and `delete` run alone, including across different lines.
- Only the first waiting job can start. When its line is busy, or it needs exclusive access,
  later jobs wait behind it even when capacity is available. Removing a waiting job advances
  the next one. Jobs are never overtaken by later submissions.

Checks of the request itself (keys, plan digest
shape, Avatar name, rights confirmation, SDK items) answer at once, and so does `project_open`
for work that changes the project (reinspection, assembly, a material change, an SDK fix, a
restore, a version restore, a Recipe application, lighting, texture tuning and deletion) while Unity holds it; other checks against
the work project and the run (Editor identity, `project_open` again, `project_drift`,
`result_not_ready`) run when the job starts, and a job that cannot start then ends `failed` with
that reason. A `runId` without a run answers `run_not_found`.
`amf.production.list` reports the configured `capacity`, `running` in admission order and
`queue` in submission order. Each entry has `taskId`, `runId` and `operation`. `busy` is true
exactly when `running` is nonempty. Cancelling a queued task through
`task.requestCancellation` ends it `cancelled` without running it. `plan`, `rollbackPlan`,
`revisionPlan` and
`recipeApplyPlan` answer `busy` while work for that run runs or waits; saving, importing and
exporting Recipes answer `busy` while a job runs. Work still waiting when VUA closes does not start
at the next launch; its task asks for inspection like other unfinished work, and the run keeps the
state it had.

`amf.production.setCapacity` requires an idle lane: no running or waiting work. Otherwise it
returns `vua.amf_production.busy`. It saves `{ "capacity": N }` to `settings.json` in AMF's
production data root (`amf-production`) before changing admission. The write uses a flushed
temporary file and rename; a failed save leaves the active capacity unchanged. Startup reads
the setting; missing, unreadable, malformed or out-of-range values use 1 without rewriting the
file. The chosen capacity applies to all Editors and lines in that AMF provider process.

AMF serializes writes to its private VPM environment with one process-wide mutex, including
backend initialization, feed registration and refresh, SDK metadata, package resolution,
downloads and installation, and the reads of cached feeds that locate bundled package copies.
A separate process-wide mutex protects ZIP extraction into the shared package cache. Unity Hub's licensing-client check, startup and readiness wait share
another mutex, released before Unity executes. Project files, Bridge requests, fingerprints,
snapshots and release records belong to individual managed projects or runs and use per-run
exclusion. SQLite task and BDL writes retain their existing transactional locks. Interactive
Editor and SDK upload work use the exclusive lane rule.

### Changing a line's materials

`amf.production.changeSources` gives a prepared line (`ready`, `succeeded`, `failed` or `stopped`) a
different set of materials; the same set, a source no longer in the library, or a project that
changed since its last check is refused before anything changes. The task protects the work
project (`snapshot`), brings it to the new materials (`materials`) and checks it in Unity
(`inspect`): files that only the previous materials used leave the project with the folders they
leave empty, files the new materials share stay, a changed version replaces its file, new files
arrive, and AMF's work scene `Assets/VUA` starts fresh. The line keeps its `runId` and work
project, returns to `ready` and keeps the choices that still name a present material; a Recipe
origin no longer applies. The restore point keeps the previous materials with the files, so
`rollback` brings both back, and a restore across the change can itself be undone. A failure
restores the protected project and the previous materials.

### Aligning lilToon brightness limits

`amf.production.planLighting` reviews an assembled line's lilToon materials: the Avatar's own
brightness limits against each outfit material's `_LightMinLimit` and `_LightMaxLimit`, read from
the work project's `.mat` files. It needs the Bridge's `materialUse` facts ([Bridge v5](unity-bridge-v5.md));
a run whose last check predates them answers `inspect_required`, and a project changed since its
last check answers `project_drift`.

The reference pair is the Avatar's: among the Avatar's lilToon materials, the pair of the
renderer whose name (the last hierarchy-path segment, case-insensitive) is `Body`. Without one,
the most common pair across the Avatar's lilToon materials wins, one vote per material. An
ambiguous Body pair, a tied vote and an Avatar without lilToon materials all report
`reference: null` with a `referenceReason` (`tie` or `no_avatar_liltoon`) and no entries. Each
outfit lilToon material (deduplicated by path) whose limits differ from the reference is one
entry `{material, sourceId, current, proposed}`; values are the exact serialized text. A material
shared by the Avatar and an outfit is never changed and is listed in `skipped` with its reason;
non-lilToon materials never appear.

`amf.production.applyLighting` takes the review's `planDigest` and a subset of its entry paths
(`materials`), so each material can be confirmed on its own. A changed project or a different
plan is refused before anything happens (`project_drift` / `plan_drift`). The task protects the
work project (`snapshot`), writes the two value lines of each chosen material — nothing else in
the file changes (`lighting`) — then rechecks the project in Unity (`inspect`) so the run's
facts and fingerprint reflect the change, and re-reads each written file against the review.
Any failure restores the protected project; the restore point is the workshop's ordinary restore
target, so the change can be undone later. The run keeps its phase.

Changing the line's materials (`changeSources`) re-imports the original packages, and this
adjustment leaves with them; plan again afterwards.

### Deleting a line

`amf.production.delete` removes a line: its work project with the restore points, previews and
turntable inside it, its release record, its assembly records and its run record. Saved Recipes
and Avatars uploaded to VRChat stay. A line with work running or waiting answers `busy`; a work
project Unity holds answers `project_open`.

## Previews

Every inspection and assembly reports what the Bridge rendered. `facts.previews` lists the active
owned Avatar from the `front`, `side` and `back`, and after inspection each candidate in `prefabs`
carries a `preview`. Each value is a work-project-relative path of the form
`.vua/previews/<commandId>/<name>.png`. Each path names the folder of the command that reported
it, and its resolved location stays inside the work project; a receipt with any other path is
invalid. An image that is missing, not a regular file or larger than 640 KiB is left out of the
facts with the warning `amf.preview.failed`. Before the next Unity run, preview folders that the
run no longer reports are removed.

`amf.production.previewImage` returns one of the paths the run currently reports, after the same
checks and a PNG signature check. The Avatar page shows the three views with the result and a
picture per candidate while choosing prefabs; the [AI tools](ai-tools-v0.1.md) return them as
image content. A preview is a visual aid for the player and the agent; the observed facts above
remain the record of what was assembled.

## Release

A finished run (`succeeded`) leaves the factory through the release room. Its record lives beside
the run in `releases/<runId>.json`; the run's phase and result stay as assembly left them. Both
release commands share the production [queue](#queue), add a `turntable` or `upload` step to the
run while they work, and end that step as `succeeded` or `failed` with the reason in its detail.

`amf.production.bakeTurntable` runs [Bridge v5](unity-bridge-v5.md#turntable) `amf_turntable`
for a finished run whose work project Unity does not hold and whose fingerprint matches the run's
last check (`project_open` and `project_drift` otherwise). The turntable is recorded with the
Unity scene fingerprint it shows; `releaseState` reports it as `current` while the run's scene
fingerprint is the same. `amf.production.turntableFrame` serves only the recorded command's
frames, after the containment, size and PNG checks of previews.

`amf.production.upload` requires `rightsConfirmed: true`: the player confirmed the content rights
statement in VUA for this upload. `avatarName` (trimmed, 1–64 characters) names a new VRChat
Avatar; an Avatar already on VRChat keeps its name. With Unity closed, the project must match the
run's last check; with Unity open, the SDK uploads the scene as it is there. The task hands the
upload to the [open Editor](unity-bridge-v5.md#managed-upload-in-the-open-editor), records each
upload's `stage`, upload `progress` and SDK `status` while it runs, and ends with `state`
`succeeded` and the VRChat `blueprintId` (`avtr_<uuid>`), or `failed`/`cancelled` with a
`reason`, the SDK's `detail` and its validation `errors`. Each upload record also names
`sdkSignIn` — `vua_session` when the SDK panel started signed in from the player's experimental
[VRChat session in VUA](vrchat-session-v0.1.md), `panel` when the sign-in happened inside the
panel; older records carry nothing. Task cancellation stops the upload. An
upload still marked running when a new one starts ended with VUA and reads `interrupted`. The
VRChat sign-in and the SDK's own ownership agreement stay inside the SDK; VUA reports only
`sdkSignedIn` from its facts. An AI Agent requests an upload as a pending request that the player
approves on the release page with the same rights confirmation (see [AI tools](ai-tools-v0.1.md)).
The release page shows the SDK's own checks of the finished Avatar from the run facts
(`sdkReview`, read by [Bridge v5](unity-bridge-v5.md) during inspection and assembly, and again in
the Editor when a managed upload ends, so an Auto Fix the player made in the SDK panel shows): the
performance rank for the build target, the items the SDK refuses an upload with, its reminders,
and the categories rated below Good. The upload confirmation names how many items the SDK
refuses an upload with; the SDK's own check during the upload decides.

`amf.production.sdkFix` (`runId`, `fixes`) runs the SDK's own Auto Fix for items its last check
offers one for, named by the SDK's exact words (1 to 16). A ready or finished run takes a restore
point (step `snapshot`), then runs the fix in Unity (step `sdk_fix`) and keeps its phase; the
facts read the SDK check again and list `sdkFixesApplied`. Items the last check does not offer a
fix for are refused with `vua.amf_production.sdk_fix_unavailable` before any restore point; a
failure restores the project. The release page offers it as "Fix with the SDK" after the player
reads what happens.

## Save and close the open Editor

`amf.production.closeEditor` is an explicit, confirmed lifecycle command for one managed line.
It occupies the exclusive Unity lane like `open` and `upload`, so it cannot close an Editor while
another AMF job uses it. A closed project returns `{schemaVersion: "0.1", runId,
outcome: "already_closed", editorClosed: true}` without saving or creating a restore point.

For an open project the engine captures its current **on-disk** `Assets`, `Packages` and
`ProjectSettings` in a verified full-copy restore point before dispatch. It compares fingerprints
around the copy and rejects concurrent disk changes. Unsaved Editor memory is not part of a disk
snapshot. The line first persists the restore point and an `editor_save` version record (marked
`editedInUnity: true` because replay cannot reproduce arbitrary manual edits), then starts step
`editor_close` and writes the [Bridge save/close request](unity-bridge-v5.md#save-and-close-handoff).
It does not run Avatar edits, SDK fixes, uploads or the previously refused production operation.

The task succeeds only with a matching successful save receipt **and** the existing project-lock
probe observing that Unity released the project. It waits at most two minutes, never terminates
Unity, removes an unconsumed request on every return, and expires late requests. Cancellation
removes an unconsumed request and marks a picked-up request cancelled; the Editor checks again
before exit. A synchronous save/exit already past that check may still finish. Missing or old
Bridge support times out and asks the player to reopen the project to load the current Bridge.
An absent receipt remains unknown even when the project lock has disappeared.

The optional run field `editorClose` retains `{schemaVersion: "0.1", runId, closeId,
outcome: "saved" | "failed", scenesSaved, assetsSaved, editorClosed, reason, detail}`. The Editor
writes saving facts before exit; only AMF's lock observation sets `editorClosed`. `reason` is
empty for a completed save/close, or `scene_save_failed`, `asset_save_failed`, `untitled_scene`,
`editor_busy`, `expired`, `cancelled`, `save_failed` or `timeout`; `detail` is at most 1000 characters. A saved
receipt with `reason: "timeout"` and `editorClosed: false` means saving succeeded but exit did not.
Failures return `editor_save_failed`, or `editor_close_timeout` for a timeout. Receipt shape and
identity failures return `receipt_invalid`; other handoff I/O failures return `editor_close_failed`.

A line with unfinished snapshot recovery refuses another save/close so that its earlier recovery
point cannot be replaced; the player saves/closes Unity manually and resolves that recovery first.

A bound `editor_busy`, `untitled_scene` or early `expired` receipt with both saved flags false
proves no save was attempted: discard that change's version and allow a retry after the player
resolves the blocker. Other failures attempt the existing automatic snapshot restore. Its project-lock guard remains in
force: when Unity still holds the project, the restore point and version remain and `recovery`
reports `restore_failed: vua.amf_production.project_open`; VUA never overwrites a live Editor's
files. After the player resolves saving and closes Unity, the existing reviewed full-copy
rollback recovers that point. Successful automatic restoration discards the failed change's
version. Every successful close invalidates the confirmed plan; changed project files put the
line in `inspect_required` before the player checks and assembles again.

All `project_open` errors retain their code and add the text that saving and closing Unity is
available and optional `params.suggestedAction: "amf.production.closeEditor"`. Renderer and MCP
consumers can offer the action without automatically invoking it. Renderer boundaries validate
both new fields when present and accept old runs/errors without them. The renderer port exposes
the confirmed task and preserves the typed error hint; request-card buttons and automatic retry
are separate frontend work. MCP's `vua_request_amf_close_editor` uses the existing local player
approval queue.

## Unity producing face

[Bridge v5](unity-bridge-v5.md) is a separate Candidate entry point for these operations. Frozen
v1-v4 serialization, version gates and vectors remain unchanged. Scene IDs stay local; observed
scene facts feed the separate Candidate [Recipe v0.4](amf-recipe-v0.4.md). Local save/reopen, reviewed sharing/import and
new-project preparation and bounded managed-Avatar application do not establish the full N4 gate or player visual acceptance.

Desktop launches the GUI Editor outside the strict AMF worker job after AMF admits the handoff.
Unity returns a receipt once it holds the project. The open succeeds when the Avatar is selected;
`sdkPanelRequested` records whether the SDK panel's menu ran, and the panel opens from VRChat SDK >
Show Control Panel when it stays closed on screen. AMF waits up to 10 minutes for the receipt, as
for a managed upload's pickup, because a project's first start imports every asset first. When
Unity cannot start, Desktop answers with `reason: "editor_unavailable"`. This is a
handoff, not a successful build/upload. Disabling AMF does not terminate that player's GUI Editor.

Before each Unity run and Editor handoff, AMF starts Unity Hub when its licensing client is not
running and waits up to 90 seconds for the client. A packaged (MSIX) Hub starts through its
application identity, so the client reads the license from Hub's package folder.

Preparation resolves missing shaders through the [known packages](../development/known-packages.md)
table, on the first preparation and on a line's material change alike. When a selected material
needs an `auto` package whose shader none of the selected sources provides, and the project does
not carry the package and no Recipe pins it, preparation installs from the package's official
feed: the newest stable release of the material's single version line, pinned as an exact
`PackageRequestV1.version`, or the newest resolving release for packages without version lines.
Materials spanning several lines and projects already carrying the package get no install.
A legacy copy bundled inside a selected package — located through the feed cache's
`legacyFolders`/`legacyFiles`, or the table's fallback before the first feed refresh — leaves
for the official package only when one recorded release's complete `.shader` set covers every
GUID in the copy, the shaders the selected materials reference and the package's shaders other
selected sources provide; the install pins that exact release, so a copy never meets an
unverified release. Multi-line Poiyomi copies and the utility shaders shipped with them leave
too; lilToon keeps its historical always-replace rule. Any other copy imports unchanged and
nothing is installed for it. Installation failures keep failing the step.
After Unity reports `facts.missingShaderMaterials`, each recognized `guide` package, and each
recognized `auto` package whose needed version lines the project does not carry, adds one
`amf.package.guide` warning naming the package, the affected material count, the needed version
lines and the official page. `bundled` packages are detected without any installation.

## Evidence limits

Synthetic tests check contracts, confirmation, recovery and relationships. They do not establish
N3's complex real-material gate or player visual acceptance. N3/N4 acceptance remains owned by
[the N sequence](../development-outline.md#n3-complex-real-material-avatar-production).

## Face tracking source evidence and creator sessions

`result.facts.faceTracking` is optional; older runs without it still pass the renderer boundary.
When present it follows the [Bridge v5 facts](unity-bridge-v5.md#face-tracking).
The port validates all new facts. Shape coverage, integration components and recognized parameter
names are source observations; runtime tracking and final built cost remain unknown.

`faceTrackingPlan` is read-only. It requires a successfully assembled line whose current files
match the last observation. `menuId` must resolve to a unique menu observed in a script owned by
one of that run's prepared sources. Its SHA-256 `planDigest` binds the run revision, both project
fingerprints, menu/source/script, active Avatar and the exact execution/trust/replay warning.
The result includes `execution: "interactive_editor"`, `replay: "manual"` and
`requiresTrustedCodeConfirmation: true`.

`faceTrackingPatch` requires that review and an explicit player confirmation for this action.
There is no executable, arbitrary menu path or library-original path parameter. No MCP mutation
tool is added. All permission modes stop for this opaque-code confirmation. The native host
serializes work through the existing production lane, rejects an open project and checks the
digest again when the task runs.

The engine records the pre-change `face_tracking` version and full snapshot, then launches a
dedicated supervised interactive Editor. Only the `amf_ft_patch`
operation omits batch/quit flags. The session has no fixed completion budget: a creator patch
with manual steps waits however long the player needs, and Stop still ends Unity's process tree
and restores the snapshot. An 8-hour safety cap is the only time limit, so a runaway Editor
process cannot outlive its supervision; reaching it restores the snapshot like any failure.
The player completes creator dialogs and prefab steps on
the existing Avatar, then chooses Finish and check or Cancel and restore. Dispatch is not a
completion receipt. After a successful session exits, a separate batch inspection must retain
the same owned Avatar and outfit selection and observe the manual-replay marker. The phase
returns to `succeeded`; actual FT movement still requires a VRChat check. Missing completion,
cancellation, the safety cap, target drift or inspection failure restores the snapshot after
process exit. Successful automatic restoration also restores the prior run facts and removes the
failed change's version record. Failed restoration remains explicit.
If child-process cleanup cannot be verified, `vua.amf_production.process_cleanup_unverified`
retains the snapshot and defers automatic restoration. The player must close the creator helper
before confirming a manual restore; the engine does not replace files under a possible writer.

An opaque creator patch is not a reproducible Recipe adjustment. Its marker is kept inside
the snapshotted `ProjectSettings/VUA.FaceTracking.json` file, outside the AssetDatabase import
pipeline. Version state adds optional/default-false
`faceTrackingManual`. Saving a new Recipe, assembling/rebuilding, replacing sources or applying
a Recipe to a marked line is refused as `vua.amf_production.face_tracking_manual_replay`.
Version replay also refuses a marked current or target state.
While the marker exists, the line keeps the full copy taken before its first patch
(`ftPrePatchSnapshot` on the run view) next to its current restore point: `sdkFix`,
`applyLighting` and every other protected change take their own restore point, and pruning
never replaces the pre-patch copy. `rollbackPlan` offers that copy as `prePatch` with its own
digest, and `rollback` with `target: "prePatch"` returns the line to it; while the copy equals
the current restore point — directly after a patch — the default restore already reaches it and
no second target is offered. A successful return to the pre-patch state removes the marker with
the files and ends the retention: the patched state the return left behind becomes the restore
point, so the return is reversible, and a later patch records a new pre-patch copy.
The Recipe format and external-tool lifecycle are unchanged.

The recovery boundary covers VUA's work-project writes. Imported scripts execute with ordinary
Editor privileges; this is not an OS sandbox and cannot undo external writes. See the
[design and SOP drafts](../development/face-tracking.md) for adapter limits and real-device review.

## Texture memory and importer tuning

`facts.textureMemory` is optional on retained runs. A new Bridge reports either an empty list
(no active Avatar) or PC and Android reports in that order. Each report contains `platform`,
`textures`, `totalBytes`, `complete` and the texture-category `rank`. Each texture has a distinct
`id`, project-relative `path` (empty for generated textures), imported `width`/`height`, `format`,
`mipmap`, `faces`, `maxSize`, `estimatedBytes`, `known`, `editable` and a `reason` for exclusions.
The dependency graph includes inactive renderers and animation-referenced materials, and counts
one texture object once even when several materials share it. This is a scene/importer estimate
before build processors; the SDK's built Avatar statistics remain authoritative.

Known estimates sum the decoded GPU storage of every mip, rounding each compressed mip to whole
blocks and multiplying cube faces. The CPU-readable copy, allocator overhead, file/download size
and transient Editor allocations are excluded. Crunch is estimated as its decoded DXT/ETC format.
Unknown formats, non-POT importer scaling and unsupported platform formats have `known: false`
and zero known bytes. Any unknown entry makes `complete: false` and `rank: unknown`; the displayed
sum is then a lower bound, never a successful rank claim.

Thresholds consulted on **2026-10-23**: VRChat's [official performance ranks](https://creators.vrchat.com/avatars/avatar-performance-ranking-system/)
publish PC Excellent/Good/Medium/Poor limits of 40/75/110/150 MB and mobile limits of 10/18/25/40 MB.
This estimate uses inclusive limits in MiB (1,048,576 bytes), displays MiB explicitly, and assigns
`very_poor` above Poor. It describes only texture memory, independently of the overall SDK rank.
Unity's [2022.3 texture format documentation](https://docs.unity3d.com/2022.3/Documentation/Manual/class-TextureImporterOverride.html)
was consulted on the same date for platform compression and Crunch's unchanged decoded footprint.

Planning requires a succeeded line, new measured facts and an unchanged project fingerprint.
The digest binds the run identity/revision, project fingerprint, platform, target and the full
plan. Suggestions first use BC7 on PC or ASTC 4x4 on Android when this saves memory without
changing resolution. Already smaller compressed formats are retained. If needed, the greatest
remaining saving is selected one halving at a time, down to a conservative maximum-dimension
floor of 512. Mipmaps, alpha, sRGB, read/write and other importer properties are preserved.
Only ordinary sRGB Texture2D color assets with POT sources and writable local files qualify;
normal/data/HDR textures, special shapes, package/generated/sub-assets and textures shared with
another Avatar in the saved scene remain unchanged. An unreachable target is explicit.

The player confirms every mutation: ask mode submits one reviewed path at a time; smart mode
reviews a selectable list; full mode confirms the target's entire suggested set with one action.
Full mode's target does not grant unattended mutation authority. Every command names a nonempty,
distinct subset of the digest-bound plan; other paths and stale digests fail before the snapshot.
The queued task validates the project again on admission, saves a restore point and a
`texture_memory` version record before writes, changes only platform importer settings in the
managed work project, then independently inspects Unity and verifies the selected settings.
A complete suggested set that was predicted to reach the target must reach it in the recheck.
Cancellation, failed import, missing facts or verification failure restore both project files
and the prior observations. A failed restoration stays explicit with its recovery point.
An interrupted texture step also offers the retained full-copy restore point.

Importer tuning is not captured in Recipes in this slice. Version records retain stable texture
facts; replay across different recorded texture settings fails with `texture_manual_replay`,
rather than claiming to reconstruct them. The retained full-copy restore point can undo the last
protected change. Full historical importer/Recipe replay remains follow-up work.

lilAvatarUtils was evaluated on **2026-10-23**. Its [MIT-licensed texture panel](https://lilxyzw.github.io/lilAvatarUtils/ja_JP/docs/TexturesGUI.html)
provides inventory, VRAM sorting and batch importer edits. It does not provide VUA's reviewed
digests, permission receipts, durable tasks, restore points or failure recovery. This slice uses
Unity's public importer API directly and neither copies nor distributes lilAvatarUtils code.

## Document changelog

- 0.33.0 (2026-10-23): add platform texture-memory observations and digest-bound, confirmed importer tuning with restore and consumer validation.

## Synced parameter budget estimate

`amf.production.plan` carries `parameterBudget`, the plan's estimate of the synced
expression-parameter bits the assembled Avatar would use against VRChat's 256-bit limit. It
merges, by merged name and in MA's build order, the selected Avatar's own Expression Parameters
asset, the MA Parameters its prefab already carries (the outfits it wears), and each selected
outfit's MA Parameters, and adds one synced Bool for each switch VUA adds to a labelled outfit
(an MA Menu Item whose parameter MA creates at build time under a unique name). `state` is
`fits`, `near_limit` (above 90 % of the limit), `over_limit` (the SDK refuses the upload; the
plan says so) or `unknown`. `unknown` covers runs whose last inspection predates the evidence,
an Avatar whose Expression Parameters asset could not be read, and any truncated observation;
it carries no number. `sources` attributes every counted bit to one row — the Avatar's own
asset, its embedded components, or one selected outfit with its switch — as the bits that
source adds to the running total, so a merged name stays with the first source that declares
it and the rows sum to `estimateBits`. `unresolvedPrefixConfigs` counts MA prefix configs whose
parameters exist only after build-time resolution; they are not in the estimate, which is
therefore a floor where they appear. The evidence and its bounds are the Bridge's
[candidate `parameterBudget`](unity-bridge-v5.md#synced-parameter-budget-evidence). The number
is a source estimate (`evidence: "unity_source_estimate"`), never the SDK's built cost, which
the SDK's own check reports after assembly.

## Document changelog

- 0.32.0 (2026-10-23): add confirmed, protected save/close tasks, bound Editor saving receipts plus project-release observation, optional close facts and project-open action hints.

- 0.31.0 (2026-10-23): `amf.production.plan` estimates the assembled Avatar's synced parameter bits against the 256-bit limit from per-candidate Bridge evidence, counting one bit for each switch VUA adds, with per-source attribution that sums to the estimate and an explicit `unknown` instead of a number when the evidence is missing or truncated.

- 0.30.0 (2026-10-23): the creator-patch session waits for the player's Finish or Cancel instead of the 20-minute Bridge budget (Stop unchanged; an 8-hour cap only ends a runaway process), and a marked line keeps its pre-patch full copy next to the current restore point, offered by `rollbackPlan` and `rollback` as the digest-bound `prePatch` target.

- 0.29.0 (2026-10-23): bundled Poiyomi copies spanning several version lines — and the utility shaders shipped with them — leave for the official package when one recorded release's complete shader set covers them, pinned to that exact release; the feed cache reads vrc-get's nested repository documents.

- 0.28.0 (2026-10-21): add optional FT source facts, a digest-bound confirmed creator-menu session with snapshot/version/recheck recovery, and explicit opaque-patch replay restrictions.

- 0.27.0 (2026-10-20): saved fits keep the player's per-object local-Transform adjustments of a garment on an Avatar and re-apply them whenever the same garment content meets the same Avatar content; adds `amf.production.listFits`, `saveFit`, `renameFit` and `deleteFit`, run-view `fitChanges` and `availableFits`, and plan `fits`.
- 0.26.0 (2026-10-20): a managed upload replaces the run's `sdkReview` with the SDK check the Editor reads once the upload ends.
- 0.25.0 (2026-10-19): protected changes record replayable version records (`revisions`, at most 50 per line), `amf.production.revisionPlan` and `amf.production.restoreRevision` restore a recorded version by replaying its materials, assembly, lighting values and SDK fixes, and each line retains exactly one full-copy snapshot for failure and interruption recovery.

- 0.24.0 (2026-10-19): each source in `amf.production.list` carries `productIds`, the BOOTH products its library entry maps to.
- 0.23.0 (2026-10-19): `amf.production.open` succeeds once Unity selects the Avatar and keeps `sdkPanelRequested` as a fact; the receipt wait is 10 minutes, and an Editor that cannot start answers `editor_unavailable`.
- 0.22.0 (2026-10-19): a missing run answers `run_not_found`; work that changes a project Unity holds answers `project_open` when it is requested; deleting a line also removes its assembly records.
- 0.21.0 (2026-10-19): `amf.production.planLighting` and `amf.production.applyLighting` align outfit lilToon brightness limits with the Avatar's reference behind a restore point, from the Bridge's material-use facts.
- 0.20.0 (2026-10-19): another Avatar prefab on an assembled line rebuilds the line on that base behind a restore point; `plan` reports it as `rebuild`.
- 0.19.0 (2026-10-19): add persisted capacity 1–3, per-line exclusion, exclusive Editor and deletion work, strict queue admission, running task identities and shared-resource write locks.
- 0.18.0 (2026-10-19): preparation resolves missing shaders through the known-package table with `auto`, `guide` and `bundled` delivery, per-line pinned installs, legacy-copy replacement only when the exact release covers the copy, and the `amf.package.guide` diagnostic naming needed version lines.
- 0.17.0 (2026-10-18): `amf.production.changeSources` changes a line's own materials behind a restore point that keeps them, and `amf.production.delete` deletes a line with its work project and records.
- 0.16.0 (2026-10-18): Unity work requested while a job runs waits as a `queued` task and starts by itself in request order; `list` reports the `queue`, and cancelling a queued task removes it without running it.
- 0.15.0 (2026-10-18): `recommendAssembly` recommends the Avatar and outfits with stated rules and leaves equally plausible candidates to the player.
- 0.14.0 (2026-10-18): `amf.production.sdkFix` and the `sdk_fix` step run the SDK's own Auto Fix behind a restore point.
- 0.13.0 (2026-10-18): cached v2 project fingerprints with a per-file hash cache in `.vua/fingerprint-cache.json`, one-time v1 comparison for older records, and the `fingerprint` progress stage for the after-Unity project-file check.
- 0.12.0 (2026-10-18): run facts carry `sdkReview`, the SDK's own Avatar checks and performance ratings, shown on the release page.
- 0.11.0 (2026-10-18): the upload record names `sdkSignIn` (`vua_session` / `panel`), the sign-in source of the SDK panel, and links the experimental [VRChat sign-in](vrchat-session-v0.1.md).
- 0.10.0 (2026-10-17): release the finished Avatar: a recorded 60-frame turntable and managed upload through the VRChat SDK after the player's rights confirmation.
- 0.9.0 (2026-10-17): add step times, usual step durations, Unity stages in task progress and `get`, Stop through task cancellation, and the `stopped` and `inspecting` phases.
- 0.8.0 (2026-10-16): report result and candidate previews and read them through `amf.production.previewImage`.
- 0.7.0 (2026-10-16): start Unity Hub before Unity work and install lilToon for lilToon materials whose shader is absent.
- 0.6.0 (2026-10-15): add observed Recipe differences, reviewed priority choices, same-project new Avatars and preserved unmentioned outfits.

- 0.5.0 (2026-10-15): connect source-reviewed export and idempotent received Recipes through the closed production Gateway.

- 0.4.0 (2026-10-15): connect verified scene settings to local Recipe saving, reopening and pinned new-project preparation.
- 0.3.0 (2026-10-14): expose actual saved-scene observations and prefer verified current settings when editing again.
- 0.2.0 (2026-10-14): extend the Candidate with reviewed, reversible user restoration and continued editing after assembly.
- 0.1.0 (2026-10-13): add the Candidate material-first N3 face, isolated managed-project path, exact confirmation and explicit evidence/recovery limits.
