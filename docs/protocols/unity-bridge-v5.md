# Unity Bridge v5: AMF production

> Document version: 0.22.0
> Status: Candidate
> Updated: 2026-10-30

This separate producing face admits the exact development pair `2022.3.22f1` and
`2022.3.22f1c1`, retaining the complete running version. It does not change v1-v4 behavior.
The fixed entry point is `Vua.Editor.Bridge.AmfProductionEntryPoint.Run`.

Operations are `amf_prepare`, `amf_inspect`, `amf_assemble`, `amf_turntable`, `amf_sdk_fix`, `amf_ft_patch`, `amf_texture_import` and `amf_outfit_remove`. The command envelope has `schemaVersion: 5`,
`commandId`, `projectId`, `operation`, `dryRun`, optional `expectedProjectFingerprint` and
`payload`. `payload.planRef` names an `*.amf-plan.json` request under the managed project's
`.vua/bridge/`; `payload.planHash` is its lowercase 64-character SHA-256 (without a prefix).
Other legacy envelope payload fields have no v5 semantics. This request is separate from frozen
Recipe/production-plan files. `dryRun` is true only for inspection.
The file's closed format is `schemaVersion: "0.1"`, `runId`, `sources` (sourceId/role/assetPaths)
and `selection`, with optional `applyTarget` for Recipe application. An outfit may carry `fit`,
saved per-object local Transforms the Bridge writes into the new instance after binding. Mutating
assembly requires an exact current fingerprint and a saved
managed scene. It validates all prefab/target/menu inputs before changes.

Preparation creates a new empty managed work scene and refuses an existing scene. Inspection
reads the selected source paths and the saved managed scene, reports actual prefab descriptor,
bone and MA facts, and changes no scene. Saved ownership resolves exact scene objects; a missing
object is never replaced by a same-name match. It checks current prefab identity, actual binding
targets, owned switches with their saved state and exclusive group, activation, missing scripts and
renderer references. `observedSelection`
is available only when all retained relationships can be represented and verified; it reflects
current scene settings rather than the last requested selection. Missing or changed relationships
remain explicit diagnostics. Assembly uses stable run/instance-owned objects, public MA
components, scene-local targets and an independent managed scene. An unowned collision is a
failure, not permission to edit an existing object. Return facts include actual target names,
source-to-instance mapping, binding and menu checks, descriptor recognition, diagnostics,
complete Editor version and project fingerprint. SDK build/upload and visual review are separate.
`usedSourceIds` records inputs whose asset paths occur in the selected Avatar's actual dependency graph,
so Recipe capture need not include unrelated automatic inputs that were only imported.

Recipe application and continued editing bind `applyTarget.avatarGlobalObjectId` to the exact
selected owned Avatar, making that ownership scope active without matching display names.
`newAvatarKey` is either null for an overlay or a digest-derived new scope for another Avatar in
the same scene. `writeSourceIds` admits only selected outfits whose supported settings change,
or all selected outfits for a new Avatar. Unmentioned or unchanged objects are observed without
reconfiguring their components. Existing ownership keys and scene objects are retained; a reused
new scope is rejected. `avatars` observes each retained owned Avatar's relationships and settings separately,
allowing preservation to be verified rather than inferred from an ownership file. Dependency
collection uses [Unity's object dependency API](https://docs.unity3d.com/2022.3/Documentation/ScriptReference/EditorUtility.CollectDependencies.html)
for the active Avatar so another scene Avatar does not expand a saved Recipe.

Every operation sets the work project's company and product name to `VUA` and `VUA Avatar Work`.
Unity keeps the VRChat SDK sign-in per company and product, so one sign-in serves every work
project. `sdkSignedIn` is `yes` when the SDK holds a saved sign-in for that product, `no` when it
holds none and `unknown` when the SDK cannot be asked. `missingShaderMaterials` lists the selected
sources' materials and the active Avatar's renderer materials that Unity draws with its error
shader because their shader is absent, with the warning `amf.shaders.missing`.

After inspection and assembly, `materialUse` lists the active Avatar's renderer materials as
`{owner, renderer, material}`: every `SkinnedMeshRenderer` and `MeshRenderer` under the Avatar
root with each `.mat` asset its `sharedMaterials` use. `renderer` is the renderer's hierarchy path
relative to the Avatar root (empty for a renderer on the root itself). A renderer under an owned
outfit instance belongs to that outfit's `sourceId`; the rest belong to `owner` `"avatar"`.
Materials outside `Assets/` — built-in and embedded ones — are skipped. The list holds at most
512 entries; a longer list is truncated with the warning `amf.material_use.truncated`. Results
written before this fact existed carry no `materialUse`.

After inspection and assembly, `sdkReview` carries the VRChat SDK's own checks of the active owned
Avatar. The Bridge runs the SDK control panel's Avatar check (the avatar builder's
`OnGUIAvatarCheck` in SDK 3.10.5) against a hidden panel instance, without opening the panel, and
reads the panel's error, warning and information lists into `issues` as `{level, text, canFix}`:
`level` is `error`, `warning` or `info`, `text` is the SDK's own sentence and `canFix` names an
item the SDK offers Auto Fix for. `stats` lists each performance category the SDK rates as
`{category, rating, text}`, computed with the SDK's performance calculation for the project's
build target (`platform` `pc` or `android`); `rating` runs from `Excellent` to `VeryPoor` and
`overall` is the SDK's overall rating. `state` is `checked`; `partial` when the SDK stopped
part-way, keeping the items found before; `unavailable` when the SDK offers no check, with
`detail`; and `not_run` for preparation and the turntable bake. The check only reads: the scene,
assets and settings stay as they are. At most 64 items are kept, each text up to 600 characters.

After inspection and assembly, the Bridge renders the active owned Avatar at 512×512 from the
front, side and back (camera yaw 0°, 90° and 180°) and lists them in `previews` as
`{view, path}`. During inspection it also renders each prefab candidate from the front at
256×256, up to 128 candidates, into the candidate's `preview`; a candidate without an image has an
empty `preview`. Each render copies the object into its own preview scene, lit by the white
three-light rig of the `build_preview` turntable, on an opaque neutral grey background, so the
work scene and its lighting stay unchanged. Images are opaque PNG files under
`.vua/previews/<commandId>/` in the work project, named `front.png`, `side.png`, `back.png` and
`candidate-NNN.png`; paths are relative to the project root with `/` separators. Rendering runs in
`-batchmode` with a graphics device. A failed render adds the warning `amf.preview.failed` and the
operation continues with its facts.

A batch run writes its Unity log to `.vua/logs/<commandId>.log` in the work project. The entry
point writes `[VUA] stage=bridge` when it starts, `[VUA] stage=previews` before rendering and
`[VUA] stage=finishing` before it writes the result, each as one line without a stack trace. AMF
reads these lines with Unity's own messages for its [live progress](amf-production-v0.1.md#progress-stop-and-continue).
A stopped run ends with Unity's process tree; the next run opens the project as Unity left it.

## Turntable

`amf_turntable` bakes the release turntable of the finished Avatar. It takes the same request file
as inspection (`dryRun` false, no `expectedProjectFingerprint`), opens the saved work scene,
observes it like inspection and renders the active owned Avatar 60 times at 512×512, camera yaw
0° to 354° in 6° steps, with the rig and background of the three views above. The frames are
`frame-00.png` … `frame-59.png` under `.vua/turntables/<commandId>/`, and `data.turntable` reports
`{folder, frames: 60, size: 512}`; other operations report an empty folder with zero frames and
size. The operation renders no previews and leaves the work scene unsaved and unchanged. A render
failure fails the operation.

The caller bakes only for a finished run whose project Unity does not hold and whose fingerprint
matches the run's last check. It records the turntable when the observed scene fingerprint equals
the run's, every frame is a regular PNG of at most 640 KiB inside the work project, and the folder
is a plain command name under `.vua/turntables/`. The turntable stays bound to that scene; once
the scene changes, the release room shows the three views and offers to bake again. Earlier
turntable folders are removed after each bake.

## SDK fixes

`amf_sdk_fix` runs the VRChat SDK's own Auto Fix for items of its Avatar check. It writes, so it
is not a dry run and carries `expectedProjectFingerprint`, like assembly. The request names the
items in `sdkFixes` by the SDK's exact words (1 to 16 items, each up to 600 characters, as
`sdkReview.issues[].text` reported them). The Bridge runs the SDK check again on the active owned
Avatar, invokes the Auto Fix of each named item the SDK still offers one for, saves the work
scene and assets, and then observes the scene and reads `sdkReview` again. `sdkFixesApplied`
lists the items it fixed; `amf.sdk.fix_unavailable` names how many chosen items no longer offer a
fix, and `amf.sdk.fix_failed` carries each fix the SDK could not complete.

## Save and close handoff

The interface-mode `InstallHandoffObserver` accepts `.vua/bridge/amf-close.json` before the SDK
open request, using the same exact Editor pair. This is an Editor lifecycle handoff, not another
batch command or permission to change the Avatar. The file is written atomically after the
player confirms in VUA and the engine persists its disk restore point and version record:

`{schemaVersion: "0.1", runId, closeId, confirmed: true, expiresAt}`.

`runId` must match `.vua/amf-run.txt`; IDs are plain ASCII alphanumeric/hyphen strings of at most
100 characters. The request is at most 8 KiB, consumed once before saving and valid for two
minutes. The observer refuses expired/invalid times and expiry more than three minutes away.
It waits through compiling and importing. Play mode, an open prefab stage or an active/pending
SDK upload/open handoff answers `editor_busy`, without exit.

Before saving any scene, the observer rejects a loaded dirty scene with no saved path as
`untitled_scene`, so the player chooses its path in Unity. It saves each loaded dirty scene with
[EditorSceneManager.SaveScene](https://docs.unity3d.com/2022.3/Documentation/ScriptReference/SceneManagement.EditorSceneManager.SaveScene.html),
checks its return and dirty state, then calls
[AssetDatabase.SaveAssets](https://docs.unity3d.com/2022.3/Documentation/ScriptReference/AssetDatabase.SaveAssets.html).
Exceptions, save error/exception/assert logs, remaining persistent dirty serialized assets/importers on disk in the project (`Assets/` or embedded `Packages/`), and scenes dirtied
again by save hooks prevent exit. Unity's internal MonoScript reflection objects and registry-package
cache resources are excluded from the dirty-object check; script import settings in the project
still count through their MonoImporter. It never treats dismissing a save dialog as saving.

It atomically writes `.vua/bridge/amf-close.result.json` before attempting exit:

`{schemaVersion: "0.1", runId, closeId, outcome: "saved" | "failed", scenesSaved, assetsSaved,
editorClosed: false, reason, detail}`.

`scenesSaved` and `assetsSaved` record the steps verified so far; `saved` requires both and an
empty reason. Failure reasons are `scene_save_failed`, `asset_save_failed`, `untitled_scene`,
`editor_busy`, `expired`, `cancelled` or `save_failed`; `detail` is at most 1000 characters. A receipt-write
failure keeps Unity open. Only a successful saving receipt, an unexpired request and no matching
`.vua/bridge/amf-close.cancel` (plain close ID) permit `EditorApplication.Exit(0)` in that Editor
update. Cancellation after that check cannot interrupt synchronous exit. The native caller bounds
receipts to regular files of at most 16 KiB, validates their closed shape and nonce/run binding,
ignores stale identities, and separately waits for the project lock to release. The Editor
never claims its own process has already ended. Timeouts neither kill Unity nor claim saving.

## Managed upload in the open Editor

The official VRChat SDK builds and uploads an Avatar through its public builder API,
`VRC.SDK3A.Editor.IVRCSdkAvatarBuilderApi.BuildAndUpload(target, avatar, thumbnailPath,
cancellationToken)`. SDK 3.10.5 hands out the builder (`VRCSdkControlPanel.TryGetBuilder`) only
while its control panel window is open, requires a signed-in account allowed to publish Avatars
(`APIUser.CurrentUser.canPublishAvatars`) and shows VRChat's copyright ownership agreement inside
that panel once per session and Avatar. Upload therefore runs in the Editor's interface, not in
batch mode. The Bridge reaches these SDK types by reflection, so the package compiles in every
project it is installed into.

1. The player confirms the content rights statement in VUA for this upload and, for a new
   Avatar, its name. VUA then writes `.vua/bridge/amf-upload.json`:
   `{schemaVersion: "0.1", runId, uploadId, avatarGlobalObjectId, avatarName, thumbnail,
   rightsConfirmed: true}`. `thumbnail` is the front preview under `.vua/previews/`.
2. When no Editor holds the project, VUA starts Unity Hub and opens the project, as for the SDK
   handoff. The Editor's VUA observer (interface mode only) reads the request when the run marker
   matches, opens the work scene, selects the owned Avatar and opens the SDK control panel.
3. The observer waits for the builder and the VRChat sign-in, which stay entirely inside the SDK
   panel. It adds a `PipelineManager` when the Avatar has none. An Avatar without a blueprint ID
   is a new VRChat Avatar: it gets the player's name, an empty description, private release
   status and the front preview as thumbnail. An Avatar with a blueprint ID keeps its details on
   VRChat, read through `VRCApi.GetAvatar`, and its thumbnail.
4. It calls `BuildAndUpload` and writes `.vua/bridge/amf-upload.progress.json` lines
   `{runId, uploadId, stage, progress, status}` from the builder's build and upload events. Stages
   are `sdk_panel`, `sign_in`, `checking`, `building`, `uploading` and `finishing`; VUA adds
   `starting` before the Editor answers. `progress` is the upload share from 0 to 1, or -1.
5. It ends with one `.vua/bridge/amf-upload.result.json`: `{runId, uploadId, outcome, blueprintId,
   reason, detail, errors, firstUpload, sceneSaved, sdkReview}`. `outcome` is `succeeded`, `failed` or
   `cancelled`. `reason` names why an upload did not succeed: `sign_in_required`,
   `no_publish_permission`, `validation` (with the SDK's `errors`), `ownership`,
   `agreement_declined`, `build`, `build_blocked`, `upload`, `thumbnail_missing`, `avatar_missing`,
   `scene_not_saved`, `interrupted`, `editor_unavailable`, `timeout`, `cancelled` or `unknown`;
   `detail` carries the SDK's own message. After a first upload the SDK writes the new blueprint ID
   into the scene, and the observer saves the work scene so later uploads update the same Avatar.
   `sdkReview` is the SDK's Avatar check read again once the upload ends, in the shape inspection
   reports it, so an Auto Fix the player made in the SDK panel during the upload reaches VUA; it
   stays `not_run` when the work scene or the Avatar is not open.

Stop writes `.vua/bridge/amf-upload.cancel` with the upload ID; the observer cancels the SDK's
token and answers `cancelled`. A domain reload ends a running upload: the observer remembers the
started upload in `.vua/bridge/amf-upload.started` and answers `interrupted` instead of building it
twice. VUA accepts only answers bound to its run and upload ID, of at most 256 KiB, and a success
only with a VRChat Avatar ID of the form `avtr_<uuid>`. It waits up to 10 minutes for the Editor to
pick up the request and up to 90 minutes in all.

Requests and results are atomically written under the controlled Bridge directory. File hash,
run binding, exact Editor pair, allowed operations and path containment are checked by Unity.
The caller additionally validates schema version, operation, command identity and receipt status.
The v5 source files are installed only into a real-SDK/MA production project, and are excluded
from legacy conversion scaffolds that intentionally use minimal compile stubs.

## Saved fits

After a successful `amf_assemble` the Bridge records a fit baseline in
`.vua/amf-fit-baseline.json`: for each owned garment (`sourceId`, ownership `ownerKey`) every
object of the instance with its local position, rotation (quaternion) and scale, addressed by a
path relative to the instance root — `/` separated child names, `name#n` (1-based) for the n-th
same-name sibling, the empty path for the instance root itself. The baseline covers the prefab's
own offsets. A partial (Recipe `applyTarget`) assembly updates the baseline only for the garments
it wrote and keeps the rest. The baseline lives in the managed project, not in Unity assets.

Observation — inspection, and the observation after assembly and SDK fixes — compares each
baseline object against the scene: position or scale differences past 0.0001 units, or a rotation
past 0.01°, are reported in `data.fitChanges` as `{sourceId, moved, missing}` per garment.
`moved` carries each changed path with its new local Transform; `missing` names baseline paths
whose object is gone, which is not an error. Garments without changes are omitted. The same
comparison runs when the player saves the work scene in the Editor (interface mode only): the
observer atomically writes `.vua/bridge/amf-fit-changes.json` —
`{schemaVersion: "0.1", runId, writtenAt, fitChanges}` — only for the run the marker names and
only for the work scene; a failure there logs a warning and never interrupts the save. A fresh
assembly deletes a pending report.

An outfit's `fit` in the request is a list of saved local Transforms in the same shape. The
Bridge applies them to the new instance right after binding, by the same path rule; paths the
instance does not have are skipped and reported in `data.fitSkipped` as `{sourceId, paths}`,
never a failure. The baseline is recorded after application, so an applied fit is not itself
reported as a move. Results written before these facts existed carry no `fitChanges` or
`fitSkipped`.

## Face tracking

Optional `data.faceTracking` has `schemaVersion: "0.1"`, `evidence: "unity_source"`, a UTC
`observedAt`, `state: no_avatar | observed | partial`, `truncated`, `manualReplayRequired`,
`runtime: "unknown"` and `builtParameterBudget: "unknown"`. Absence is compatible with older
results. `meshes` contains at most 128 separate SkinnedMeshRenderer observations: relative
renderer path, shape count, missing-mesh flag and exact matched names for `arkit52-v1` (52),
`unified-core12-v1` (12) and `sranipal-core12-v1` (12). The latter two are deliberately named
subsets, not complete UE/SRanipal standards. Coverage is never combined across renderers.

`parameters` lists up to 1024 distinct FT-like names from source Animator controllers,
MA Merge Animator controllers and descriptor expression parameters. Names using any prefix before `/v2/`, known control names,
`FT/`/`VRCFT/` prefixes or the detection vocabularies are hints. `components` lists up to 64
MA/VRCFury/Pawlygon integration component type names, which do not prove FT wiring.
`expressionParametersObserved` says whether the descriptor's expression-parameter list could
be read. Only when true does `sourceSyncedParameterBits` describe its synced cost (Bool 1,
Int/Float 8, unsynced 0); otherwise the field is zero and must not be displayed as a measured
budget. NDMF/VRCFury build-time changes are outside this source observation.

`patchMenus` lists at most 64 unique imported creator menus under `Tools/` or a custom top-level
root, with `id`, `sourceId`,
`scriptPath` and `menuPath`. IDs hash the exact source/script/menu identity. Discovery reads
MenuItem metadata without executing methods or validators. It excludes duplicate registrations,
context/built-in paths, shortcuts, ambiguous source ownership, unsupported signatures and
types which MonoScript cannot resolve. This is a bounded list of possible creator entry points,
not a claim that every menu performs FT. Missing results require creator-specific follow-up;
there is no arbitrary-string fallback.

`amf_ft_patch` requires `dryRun: false`, the current `expectedProjectFingerprint`, and plan
`faceTrackingPatch: {menuId, avatarGlobalObjectId, confirmTrustedCode: true}`. Batch mode is
rejected before dispatch. In the dedicated GUI process, the Bridge verifies the owned Avatar,
its assembled selection and the freshly rediscovered menu. It persists dispatch state in
Unity SessionState before executing the menu once. Domain reload only reopens VUA's completion
window; process restart cannot resume it. The parent engine does not apply the Bridge's usual
20-minute command budget to this process: supervision waits for the completion window's exit
(Finish or Cancel) however long the player's manual steps take, while the engine's Stop ends
Unity's process tree within half a second and an 8-hour safety cap ends a runaway process.

Finish saves the single managed scene and checks the original Avatar/outfit relationships,
writes `ProjectSettings/VUA.FaceTracking.json` with local source/menu/command identities and manual replay
status, then emits the bound successful receipt and exits. Cancel exits without success.
The parent engine owns the snapshot and independent batch recheck; no Bridge menu success
alone declares FT functional. The operation's listed changed paths are the managed scene and
marker, not an exhaustive claim about opaque creator writes.

## Explicit outfit removal

`amf_outfit_remove` is mutating and requires the current `expectedProjectFingerprint`.
The request adds `outfitRemoval`, using the [AMF removal inventory](amf-production-v0.1.md#removing-one-owned-outfit).
Inspection and successful mutations report optional `data.outfitRemoval` inventories. A successful
removal additionally reports exactly one `data.outfitRemovalApplied` entry with `sourceId`,
`avatarGlobalObjectId` and the complete `deletedObjectIds`; other operations return an empty
array. Old receipts may omit both fields. New consumers validate present fields.

The existing `Assets/VUA/Ownership.json` records, scoped by the active Avatar key, are the only
authority for resolving the garment and menu roots. Every child/component is inventoried by
GlobalObjectId; display paths never locate deletions. Another ownership record within either
root, duplicate/unresolved identities or an over-limit inventory prevents application. The
Bridge observes the saved scene, recomputes and compares the full inventory before the first
deletion, rejects face-tracking markers, destroys only the two recorded roots and verifies the
listed IDs no longer resolve. It saves the scene and ownership, prunes only the target's fit
baseline/report entries and reinspects the active Avatar through the usual observer. The AMF
engine then performs a separate Bridge inspection and verifies retained outfits/Avatars.

Scene additions use Unity's [added-object override API](https://docs.unity3d.com/2022.3/Documentation/ScriptReference/PrefabUtility.IsAddedGameObjectOverride.html)
and prefab correspondence, conservatively warning about added components of unknown provenance.
Identity resolution uses [GlobalObjectId](https://docs.unity3d.com/2022.3/Documentation/ScriptReference/GlobalObjectId.GlobalObjectIdentifierToObjectSlow.html),
never object names. API references checked 2026-10-23.

## Recipe inspection evidence

Each inspected prefab additionally reports optional `prefabGuid`, the exact lowercase 32-digit
result of `AssetDatabase.AssetPathToGUID(path)`, and optional `objectPaths`, up to 2048 distinct
fit-compatible paths of its current Transform hierarchy, including the empty root path. Each path
is at most 512 characters. The result's optional `data.materialPaths` lists up to 4096 distinct
loadable `.mat` asset paths from the requested sources, in ordinal order. Older receipts may omit
these fields. Present fields are validated at the native and renderer boundaries.

AMF uses these observations to map [Recipe v0.5](amf-recipe-v0.5.md) prefabs by GUID, exact path or
unique identical filename, and to list missing prefab/fit/material targets before assembly.
Inventories stay bounded; unreported paths count as missing for review. Reporting a match does
not confirm or execute a Recipe difference. No asset content or local hierarchy inventory is
copied into a shared Recipe beyond its existing declared fit transforms.

## Texture memory observations and importer changes

`data.textureMemory` is an optional additive observation field. When an active owned Avatar is
present it contains PC and Android reports; without one it is empty. Older receipts omit it.
The closed report/texture fields and estimate/threshold rules are owned by
[AMF texture tuning](amf-production-v0.1.md#texture-memory-and-importer-tuning) and represented
in `schemas/unity-bridge/v5/result.schema.json`. Native and renderer consumers validate present
fields; absence remains readable and requires fresh inspection before tuning.

`amf_texture_import` is a mutating operation with `dryRun: false` and an exact current
`expectedProjectFingerprint`. Its separate request adds a nonempty `textureChanges` array of
`{path, platform, maxSize, format}`. The managed run marker, active saved Avatar ownership and
payload digest are checked before changes. Each path must be an editable texture dependency of
the active Avatar. The entire batch is checked before its first write, including duplicate
paths, allowed platform/format, no size increase, a power-of-two Max Size (no reduction below 512, with existing smaller sizes retained), local
asset and `.meta` ownership, and absence of reparse points on their paths. Files in the original
library, package assets and other Avatar dependencies cannot be changed.

For PC the platform override name is `Standalone`; for Android it is `Android`. The Bridge uses
`TextureImporter.SetPlatformTextureSettings` and `SaveAndReimport`. It changes the selected
platform's Max Size and format, disabling incompatible Crunch for BC7/ASTC, and preserves the
remaining properties and the other platform override. `changedPaths` lists the written `.meta`
files. It re-observes textures, SDK checks and project fingerprints; the engine performs a second
independent inspection and owns restoration if any part fails. No image source is rewritten.

## Synced parameter budget evidence

Each inspection candidate may carry `parameterBudget`, source evidence for VRChat's 256-bit
synced expression-parameter limit. Absence is compatible with older results. It holds:

- `expressionParametersObserved`: whether the candidate's own Expression Parameters are fully
  known. An outfit has none, so it reports `true` with an empty list. An Avatar reports `true`
  only when its descriptor's expression-parameter asset could be read whole; a missing asset is
  `false` because the build substitutes the SDK default asset, whose cost this observation
  cannot read. When `false` the list is empty and must not be displayed as a measured zero.
- `expressionParameters`: the asset's synced entries as `{name, bits, unique: false}`, counting
  Bool as 1 and Int/Float as 8 bits. Entries with `networkSynced` false cost no synced bits and
  are left out.
- `maComponents`: one entry per `Modular Avatar Parameters` component under the prefab, with the
  component's path inside the prefab (empty names the prefab root) and its synced configs as
  `{name, bits, unique}`. `NotSynced` and local-only configs are left out because the merged
  parameter keeps `networkSynced` off. `internalParameter` entries are `unique: true`: MA
  renames them at build time, so they never merge with a same-named declaration. `remapTo`
  owns the merged name. On an Avatar candidate these components are the outfits its prefab
  already wears; on an outfit candidate they are what assembling it would add.
- `unresolvedPrefixConfigs`: configs using a parameter-name prefix. They bind parameters that
  exist only after build-time resolution, carry no bits here, and make any total that includes
  them an understatement.
- `truncated`: a bound was hit (more than 256 asset entries, 64 components or 256 component
  entries, or an unreadable name or path), so the lists are incomplete evidence rather than a
  budget.

Modular Avatar merges all of this into the Avatar's asset only at build time, deduplicating by
merged name with the Avatar's own asset first. The consumer's plan estimate reproduces that
merge order and adds the switches VUA itself creates (a Bool per independent switch, an Int per
exclusive group); it cannot
reproduce what else only exists after the build: other automatically created menu-item
parameters, parameters other tools (VRCFury, reactive components) generate, or an
unsynced asset entry that an MA config upgrades to synced. The authoritative synced cost stays
with the SDK's own check on the built result.

## Candidate footprint evidence

Each inspection candidate may carry `footprint`, the pre-assembly cost facts for that imported
prefab. Absence is compatible with older results. It holds:

- `triangles`, `materialSlots`, `meshes`, `skinnedMeshes`: counted from the candidate's
  renderers — index counts per submesh divided by three, and every renderer's `sharedMaterials`
  length. A renderer without a mesh contributes nothing.
- `textureBytes`, `textureComplete`: decoded PC texture memory over the prefab's dependencies,
  using the same importer rules and decoded-size arithmetic as the texture-memory report. False
  when any texture's size cannot be decided or the inventory exceeds 2048 textures; the bytes
  then cover only the decidable textures.
- `meshBytes`, `meshComplete`: vertex stride from each mesh's own vertex-attribute layout times
  its vertex count, plus its index buffer at the mesh's index format. A mesh whose layout cannot
  be read assumes 32 bytes per vertex and marks itself incomplete.
- `physBones`, `physBonesKnown`: the count of `VRCPhysBone` components found by type name;
  `physBonesKnown` is false when the SDK type does not resolve, so a zero is never presented as
  a measured absence.

The consumer's plan prediction reuses these numbers, the texture-memory rules and the parameter
estimate; it does not re-derive them. The built result stays authoritative, and the expected
tolerance is stated in
[amf-production v0.1](amf-production-v0.1.md#pre-assembly-cost-prediction).

## Switch options

A plan's outfit may carry `saved` (default `true`) and `group` (default empty). Unity's
JsonUtility keeps the field initializers when VUA leaves them out. Assembly writes `saved` to the
owned MA Menu Item's saved setting. An independent switch keeps an empty parameter, so MA creates
its own; the switches of a group share the parameter `VUA/Group/<group>`, with values 1..n in the
selection's order, so the menu works as radio buttons. Inspection reads both back into
`observedSelection`: `saved` from the item, `group` from the parameter's suffix (empty for an
independent switch). A grouped item with a value below 1 or a parameter outside that prefix does
not verify.

## Document changelog

- 0.22.0 (2026-10-30): each inspection candidate may carry `footprint` — triangles, material slots, renderer counts, decoded texture bytes, mesh buffer bytes and PhysBone counts observed before assembly, each group stating whether it is complete.

- 0.21.0 (2026-10-25): report optional prefab GUIDs and bounded prefab-object/material path inventories for reviewed Recipe v0.5 reproduction, retaining switch options in plans and observations.

- 0.20.0 (2026-10-24): plan outfits carry optional `saved` and `group`; grouped switches share `VUA/Group/<group>` with values 1..n, and inspection reports both in `observedSelection`.

- 0.19.0 (2026-10-23): add ownership-ID-bound outfit-removal inventories and a protected mutating operation; retain other garments, Avatars and fit baselines.

- 0.18.0 (2026-10-23): add platform texture-memory observations and digest-bound, confirmed importer tuning with restore and consumer validation.

- 0.17.0 (2026-10-23): add the confirmed, expiring save/close observer handoff, explicit scene/asset saving facts and cooperative cancellation before exit.

- 0.16.0 (2026-10-23): each inspection candidate may carry `parameterBudget`, the source-side synced expression-parameter evidence (asset entries, MA Parameters per component, unresolved prefix configs) behind the assembly-plan estimate against the 256-bit limit.

- 0.15.0 (2026-10-22): `amf_ft_patch` supervision waits for the completion window's exit instead of the Bridge's 20-minute command budget; Stop still ends the process tree and an 8-hour safety cap ends a runaway process.

- 0.14.0 (2026-10-21): add optional per-mesh FT source evidence and confirmed interactive `amf_ft_patch`; retain unknown runtime/built-budget facts and explicit manual replay.

- 0.13.0 (2026-10-20): record a per-garment local-Transform fit baseline after assembly, report moved and missing objects as `fitChanges` (also from scene saves in the open Editor), and apply request `fit` transforms with per-path skips reported as `fitSkipped`.
- 0.12.0 (2026-10-20): the upload receipt carries `sdkReview`, the SDK's Avatar check read again after the upload.
- 0.11.0 (2026-10-19): report the active Avatar's renderer material use (`materialUse`) after inspection and assembly, bounded and attributed to the Avatar or the owning outfit.
- 0.10.1 (2026-10-19): the result schema covers every field Unity writes, verified against real preparation, inspection, assembly and turntable results; SDK fix results are covered by a synthetic vector.
- 0.10.0 (2026-10-18): `amf_sdk_fix` runs the SDK's own Auto Fix for chosen items of its Avatar check.
- 0.9.0 (2026-10-18): read the VRChat SDK's own Avatar checks and performance ratings into `sdkReview` after inspection and assembly.
- 0.8.0 (2026-10-17): bake the 60-frame release turntable with `amf_turntable`, and upload through the SDK builder in the open Editor after the player's rights confirmation in VUA.
- 0.7.0 (2026-10-17): write bridge, previews and finishing stage lines to the run log.
