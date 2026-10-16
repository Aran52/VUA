# Unity Bridge v5: AMF production

> Document version: 0.5.0
> Status: Candidate
> Updated: 2026-10-16

This separate producing face admits the exact development pair `2022.3.22f1` and
`2022.3.22f1c1`, retaining the complete running version. It does not change v1-v4 behavior.
The fixed entry point is `Vua.Editor.Bridge.AmfProductionEntryPoint.Run`.

Operations are `amf_prepare`, `amf_inspect` and `amf_assemble`. The command envelope has `schemaVersion: 5`,
`commandId`, `projectId`, `operation`, `dryRun`, optional `expectedProjectFingerprint` and
`payload`. `payload.planRef` names an `*.amf-plan.json` request under the managed project's
`.vua/bridge/`; `payload.planHash` is its lowercase 64-character SHA-256 (without a prefix).
Other legacy envelope payload fields have no v5 semantics. This request is separate from frozen
Recipe/production-plan files. `dryRun` is true only for inspection.
The file's closed format is `schemaVersion: "0.1"`, `runId`, `sources` (sourceId/role/assetPaths)
and `selection`, with optional `applyTarget` for Recipe application. Mutating assembly requires an exact current fingerprint and a saved
managed scene. It validates all prefab/target/menu inputs before changes.

Preparation creates a new empty managed work scene and refuses an existing scene. Inspection
reads the selected source paths and the saved managed scene, reports actual prefab descriptor,
bone and MA facts, and changes no scene. Saved ownership resolves exact scene objects; a missing
object is never replaced by a same-name match. It checks current prefab identity, actual binding
targets, owned switches, activation, missing scripts and renderer references. `observedSelection`
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

Requests and results are atomically written under the controlled Bridge directory. File hash,
run binding, exact Editor pair, allowed operations and path containment are checked by Unity.
The caller additionally validates schema version, operation, command identity and receipt status.
The v5 source files are installed only into a real-SDK/MA production project, and are excluded
from legacy conversion scaffolds that intentionally use minimal compile stubs.

## Document changelog

- 0.5.0 (2026-10-16): share one company and product name across work projects, report the SDK sign-in state and list materials whose shader is absent.
- 0.4.0 (2026-10-15): bind Recipe application to observed Avatar identities, support independent same-scene Avatar scopes, and observe retained Avatars separately.

- 0.3.0 (2026-10-15): report source dependencies actually used by the saved scene for Recipe capture.
- 0.2.0 (2026-10-14): observe retained scene objects and current supported settings during reinspection and restoration.
- 0.1.0 (2026-10-13): define a separate Candidate N3 entry point, exact f1/c1 identity, hash-bound requests and observed result relationships.
