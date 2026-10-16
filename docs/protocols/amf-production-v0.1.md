# AMF material-first production v0.1

> Document version: 0.7.0
> Status: Candidate
> Updated: 2026-10-16
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
| `amf.production.list` | Query | none | Local UnityPackage source identities, detected admitted Editors, and retained run summaries |
| `amf.production.get` | Query | `runId` | Run facts, actual candidates, steps, checks and recovery state |
| `amf.production.prepare` | Command | `sources: [{sourceId, role}]`, `editorPath` | Durable `taskId`, `runId` acceptance |
| `amf.production.plan` | Query | `runId`, `selection` | Exact digest-bound selection and effective input roles |
| `amf.production.assemble` | Command | `runId`, `planDigest`, `selection` | Durable task acceptance |
| `amf.production.reinspect` | Command | `runId` | Durable task acceptance; never automatically resumes an interrupted assembly |
| `amf.production.rollbackPlan` | Query | `runId` | Digest-bound restore point, current drift notice and current-state preservation promise |
| `amf.production.rollback` | Command | `runId`, `planDigest` | Durable task acceptance; protect current state, restore, then inspect in Unity |
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

Roles: `auto`, `avatar`, `outfit`, `dependency`. The player path starts with `auto` sources.
Unity discovers actual prefabs; a unique Avatar/outfit candidate is preselected, while ambiguity
or a static attachment target requires a choice. Assembly requires an observed Avatar and at
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
Malformed unrelated ZIP files are not offered as package sources.

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

## Facts and recovery

Run phases: `preparing`, `ready`, `assembling`, `restoring`, `succeeded`, `failed`, `cancelled`,
`inspect_required`. Steps have stable keys, readable input names and actual outcomes.
Persisted nonterminal work becomes `inspect_required` after restart; reinspection or a new
preparation is an explicit action. Cancellation is cooperative at safe step boundaries.

Before assembly, the managed project is snapshotted and the exact confirmation/input facts are
rechecked. Failures retain sources and records, report restoration separately, and do not delete
the player's materials. A successful receipt must report actual Avatar/MA/menu relationships.
The retained result records the input hashes, exact Editor and installed VPM versions, snapshot
identity, actual instance targets and per-step state. SDK descriptor recognition is an observation;
official SDK build/upload checks remain `not_run` until actually performed. No upload is automated.

A retained `snapshotId` also enables user-requested restoration. The review digest binds the
run, observation revision, Editor, managed project, restore point and the **current** project
fingerprint. If external edits exist, the review reports `currentChanged`; no current edit is
discarded silently. Execution refuses a stale review. It snapshots the current project and
persists that recovery identity before restoring the reviewed state. Actual Unity inspection
must succeed before the run becomes `ready`; partial restore, cancellation or failed inspection
attempts recovery to the protected current state and reports its outcome separately. The
protected state remains the next restore point, making restoration itself reversible. Interrupted
restoration requires explicit reinspection. Original library inputs are never restoration targets.

## Unity producing face

[Bridge v5](unity-bridge-v5.md) is a separate Candidate entry point for these operations. Frozen
v1-v4 serialization, version gates and vectors remain unchanged. Scene IDs stay local; observed
scene facts feed the separate Candidate [Recipe v0.4](amf-recipe-v0.4.md). Local save/reopen, reviewed sharing/import and
new-project preparation and bounded managed-Avatar application do not establish the full N4 gate or player visual acceptance.

Desktop launches the GUI Editor outside the strict AMF worker job after AMF admits the handoff.
Unity returns a receipt for actual Avatar selection and official SDK panel invocation. This is a
handoff, not a successful build/upload. Disabling AMF does not terminate that player's GUI Editor.

Before each Unity run and Editor handoff, AMF starts Unity Hub when its licensing client is not
running and waits up to 90 seconds for the client. A packaged (MSIX) Hub starts through its
application identity, so the client reads the license from Hub's package folder.

Preparation installs lilToon from its official feed when a selected package bundles lilToon or
carries a lilToon material whose shader none of the selected packages provides.

## Evidence limits

Synthetic tests check contracts, confirmation, recovery and relationships. They do not establish
N3's complex real-material gate or player visual acceptance. N3/N4 acceptance remains owned by
[the N sequence](../development-outline.md#n3-complex-real-material-avatar-production).

## Document changelog

- 0.7.0 (2026-10-16): start Unity Hub before Unity work and install lilToon for lilToon materials whose shader is absent.
- 0.6.0 (2026-10-15): add observed Recipe differences, reviewed priority choices, same-project new Avatars and preserved unmentioned outfits.

- 0.5.0 (2026-10-15): connect source-reviewed export and idempotent received Recipes through the closed production Gateway.

- 0.4.0 (2026-10-15): connect verified scene settings to local Recipe saving, reopening and pinned new-project preparation.
- 0.3.0 (2026-10-14): expose actual saved-scene observations and prefer verified current settings when editing again.
- 0.2.0 (2026-10-14): extend the Candidate with reviewed, reversible user restoration and continued editing after assembly.
- 0.1.0 (2026-10-13): add the Candidate material-first N3 face, isolated managed-project path, exact confirmation and explicit evidence/recovery limits.
