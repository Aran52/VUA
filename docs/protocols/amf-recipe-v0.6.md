# AMF production Recipe v0.6

> Document version: 0.1.0
> Status: Candidate
> Updated: 2026-11-11
> Owner: AMF

Recipe v0.6 extends [Recipe v0.5](amf-recipe-v0.5.md) solely for lines where optional
[Avatar Optimizer](../tool-catalog/external/avatar-optimizer.md) (`com.anatawa12.avatar-optimizer`)
is explicitly enabled on VUA's owned Avatar. Lines with Avatar Optimizer off continue to capture and
share `formatVersion: "0.5"` (`vuar0.5.`) so older VUA installations can still import and prepare
them without upgrading. Existing [v0.4](amf-recipe-v0.4.md) and [v0.5](amf-recipe-v0.5.md) documents
and paste codes remain readable, importable and preparable.

## Conditional version emission

- `amf.production.saveRecipe` emits `formatVersion: "0.6"` if and only if the line's active owned
  Avatar has Avatar Optimizer enabled (`run.optimizer.enabled == true`). Otherwise it emits `"0.5"`.
- A `0.6` Recipe MUST carry:
  - `packages["com.anatawa12.avatar-optimizer"]` pinned to the exact package version observed in the
    managed work project (alongside `com.vrchat.avatars` and `nadena.dev.modular-avatar`), and
  - `adjustments.optimizer: true`.
- A document claiming `formatVersion: "0.6"` without `adjustments.optimizer: true` or without
  `com.anatawa12.avatar-optimizer` in `packages` is rejected by both the closed
  [Recipe schema](../../schemas/amf-recipe/v0.6/recipe.schema.json) and the Rust/TypeScript
  validators. Conversely, `"0.4"` and `"0.5"` documents cannot carry `adjustments.optimizer: true`.

## Preparation, assembly and sharing

- `prepareRecipe` installs every package listed in `recipe.packages`, resolving
  `com.anatawa12.avatar-optimizer` from the pinned `https://vpm.anatawa12.com/vpm.json` feed under
  [known packages](../development/known-packages.md) and checking compatibility with the project's
  VRChat Avatars SDK and NDMF versions before installing into the managed work project.
- `amf.production.plan` includes `adjustments.optimizer: true` in its digest-bound pending
  adjustments, and `assemble` applies `Anatawa12.AvatarOptimizer.TraceAndOptimize` on the owned
  Avatar root after assembly, reporting the local NDMF `AvatarProcessor.ManualProcessAvatar`
  before/after measurement (`localMeasurementOnly: true`, `officialSdkChecks: "not_run"`).
- Sharing uses `formatVersion: "0.6"` in the [share schema](../../schemas/amf-recipe/v0.6/share.schema.json)
  and `vuar0.6.` base64url paste codes with the same 256 KiB decoded limit and BOOTH product
  provenance rules as [v0.5](amf-recipe-v0.5.md).

## Document changelog

- 0.1.0 (2026-11-11): introduce conditional `0.6` Recipes and `vuar0.6.` share codes for lines with
  optional `com.anatawa12.avatar-optimizer` (`TraceAndOptimize`) enabled, preserving `0.5` when off.
