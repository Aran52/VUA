# AMF production Recipe v0.5

> Document version: 0.2.0
> Status: Candidate
> Updated: 2026-10-25
> Owner: AMF

Recipe v0.5 identifies material by BOOTH product where a source association is known. A creator
updating a package does not invalidate the declaration. The closed [Recipe schema](../../schemas/amf-recipe/v0.5/recipe.schema.json)
and [share schema](../../schemas/amf-recipe/v0.5/share.schema.json) have positive and negative
vectors consumed by Rust and TypeScript. Existing [v0.4](amf-recipe-v0.4.md) documents and paste
codes remain readable, importable and preparable with their original hash-only resolution.
`saveRecipe` always captures v0.5; importing a v0.4 declaration preserves that version and identity.
Both generations use the existing revisioned local document store, without moving old files.

## Portable declarations

Each material has an opaque `id` (1–100 ASCII letters, digits, `_` or `-`), `packageName` (the
original filename), lowercase `sha256` (64 hex digits of the author's package) and optional
`boothProductId` (1–20 digits, no leading zero). IDs are stable within the retained declaration,
independent of package bytes. New captures allocate `material-1`, `material-2`, etc. A material
without a product ID still requires exact original bytes. Hashes remain evidence and the first
resolution rule for every material.

The Avatar and each outfit reference a material ID, `prefabPath` and optional `prefabGuid`
(32 lowercase hex digits). Current Bridge inspections supply the GUID. Absence remains valid
for an older inspection; saving does not invent a GUID. Fits reference declared opaque material
IDs. Each lighting entry additionally names its owning `materialId`; its `.mat` path and decimal
value strings keep the v0.4 grammar. Capture refuses an ambiguous lighting package owner.
Editor admission, exact VPM pins, binding modes, switches, fit transforms and size limits retain
the v0.4 rules. Asset bodies, local source IDs, project paths and scene object identities are
never portable fields.

Outfits retain the optional switch fields `saved` (default `true`) and `group` (default empty)
from v0.4. Saving, sharing, material substitution and replay preserve both fields; an absent
field keeps its default. Group names follow the existing 0–32 ASCII letter/digit/underscore/
hyphen grammar and the existing exclusive-switch validation.

Saving uses the player's retained source choices or an unambiguous existing library association.
Multiple or absent BOOTH associations remain absent until explicitly supplemented. Received
source statements belong to the received declaration; they never grant ownership or become
local confirmed source preferences.

## Local material checklist

`amf.production.recipeMaterialPlan(recipeId, materialChoices?)` reads local library and account
facts only. It returns `recipeId`, `materials`, `materialChoices`, `ready` and `planDigest`.
Each material row contains `materialId`, original `packageName`, nullable `boothProductId`,
`status`, nullable `resolved`, all product `candidates`, and recorded `downloadNames`.
A package is `{sourceId, packageName, sha256}`; these source IDs remain local.

Resolution runs independently for every material and returns the complete checklist:

1. Exact local SHA-256 wins, even when a different package was requested. Duplicate exact copies
   are ordered by source ID. A conflicting explicit choice is invalid.
2. For a BOOTH product with local associated packages, a unique exact filename wins; otherwise
   the sole package wins. Multiple remaining packages accept an explicit choice naming one of
   those packages, or report `material_choice_required` with every candidate. Filename comparison
   is case-sensitive and exact; no normalization, version stripping or fuzzy matching occurs.
3. Account-library membership with no local package reports `material_not_downloaded`, the product
   ID and recorded downloadable names. Public catalog presence alone is not ownership.
4. Without local packages or account-library membership, report `material_not_owned` with the
   product ID. A local material without an ID reports `material_missing`.

`prepareRecipe`, `recipeApplyPlan` and `applyRecipe` accept optional `materialChoices`, mapping
material IDs to local source IDs. Absent means an empty map. Unknown material IDs and foreign
source IDs are rejected. The preparation records the chosen packages' actual hashes and checks
them again before import. Unresolved preparation refuses project creation with
`recipe_materials_unresolved`; `params.materials` is the JSON checklist and `suggestedAction`
names `amf.production.recipeMaterialPlan`. It never starts a partial project or downloads files.
Application to an existing run requires these packages already imported and inspected there;
otherwise `canApply` is false with the complete checklist. Package installation into an existing
Avatar remains the existing material-change workflow.

## Unity mapping and reviewed differences

Preparation imports the selected packages and inspects Unity before retaining a selection.
Within each resolved source, prefab matching uses a unique identical GUID, then a unique exact
path, then a unique identical filename, in that order. An ambiguous or absent final match is
missing. A missing prefab blocks assembly, rather than dropping an outfit.

The plan's `recipeDifferences` is an ordered list of bounded entries:
`{kind, materialId, before, after, rule, paths, count}`. `after` and `rule` are nullable.

| Kind | Meaning |
| --- | --- |
| `material_file_changed` | Original filename → selected filename; also emitted for different bytes under the same filename |
| `prefab_mapped` | Original path → actual path; `rule` is `guid`, `path` or `name`; emitted for each substituted package's prefab, even if its path stayed the same |
| `prefab_missing` | The declared prefab could not be identified; assembly is blocked |
| `fit_objects_missing` | Saved fit paths absent from the matched prefab; `paths` and `count` name every skipped transform |
| `lighting_material_missing` | The exact `.mat` path is absent from the resolved package's observed materials; it is skipped |

Fit paths use the existing sibling-index grammar and never match by name heuristics. Material
paths likewise require exact observation. On substituted packages, absent old-Bridge object or
material evidence causes affected adjustments to be listed as skipped. An unchanged package
retains the older receipt behavior. Bridge path inventories are bounded; paths outside the
reported inventory are treated as missing and included in the review.

Assembly plans and application plans return the checklist, `materialChoices` and
`recipeDifferences`. Missing prefabs produce `canAssemble: false` or `canApply: false`.
The digest binds actual packages, explicit choices, the complete differences, mapped selection
and replayable adjustments. A nonempty difference list additionally requires
`recipeDifferencesConfirmed: true` on `assemble` or `applyRecipe`. Absent confirmation refuses
the mutation before a restore point or Avatar write. Confirmation applies only to the exact
digest, so changes require another review. The existing player approval, restore point, version
record and automatic recovery continue to own every Avatar mutation.

## Sharing and Agent reads

The share envelope uses `formatVersion: "0.5"`, the Recipe, and a source map from every material
ID to `{boothProductId, shopName?}`. `shopName` is a bounded BOOTH shop display name for attribution,
not an entitlement check. Product IDs in a material and its statement must agree when both exist.
Export records explicit BOOTH supplementation in the portable copy. JSON and `vuar0.5.` base64url
text encode the same closed declaration with the existing 256 KiB decoded limit. The old
`vuar0.4.` decoder remains available. Importing starts no Unity work.

MCP exposes `vua_recipe_materials` before preparation and `vua_recipe_apply_plan` for an observed
run. `vua_amf_plan` returns the same checklist and differences for a prepared Recipe. Agents
must explain missing purchases/downloads, obtain package choices and show all differences before
requesting assembly. These reads do not buy, download or automatically approve anything.
Receiver pages and download actions are separate work; this Candidate supplies their data and
explicit confirmation parameters.

## Document changelog

- 0.2.0 (2026-10-25): retain optional outfit `saved` and `group` options across capture, shared declarations and product-based reproduction.

- 0.1.0 (2026-10-24): introduce product-based material resolution, opaque references, prefab GUID
  capture, shop attribution, complete checklists and digest-bound explicit difference review.
