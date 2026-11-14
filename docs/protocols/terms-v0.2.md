# VN3 terms v0.2 (cross-platform & PH-R presets)

> Document version: 0.1.0
> Status: Candidate
> Owner: AMF
> Updated: 2026-11-14
> Base specification: [VN3 terms v0.1](terms-v0.1.md)
> Schemas: [check-use-request](../../schemas/terms/v0.2/check-use-request.schema.json), [check-use-result](../../schemas/terms/v0.2/check-use-result.schema.json), [vectors](../../schemas/terms/v0.2/vectors.json)

This Candidate increment extends `terms.checkUse` to `schemaVersion: "0.2"` with three cross-platform
presets tailored for `.glb` / `VRM 1.0` workflows and sibling platform **PH-R**, plus a
deterministic `platformRestrictionHint` flag for reviewing platform-exclusive special notes (`X`).

`terms v0.1` (`terms.refreshProduct`, `terms.refreshLibrary`, `terms.getProduct`, and
`terms.checkUse` with `schemaVersion: "0.1"`) remains byte-for-byte unchanged.

## Added presets (`vn3-presets-v2.json`)

In addition to the seven `v0.1` presets (`vrchat_personal`, `vtuber_individual`, `vtuber_corporate`,
`game_development`, `outfit_for_sale`, `merchandise_publication`, `adult_content`), `terms.checkUse`
with `schemaVersion: "0.2"` admits:

| Preset | Required items | Commercial requirement | Intended workflow |
| --- | --- | --- | --- |
| `phr_personal_avatar` | `C`, `I`, `J` | none | Uploading a personal avatar converted to `.vrm` / `.glb` (`I` file format conversion + `J` modification + `C` social/online platform upload) |
| `phr_object_head` | `C`, `I`, `J` | none | Converting a 3D accessory/prop into a `.glb` Object Head (`I` format conversion + `J` scale/pivot adjustment + `C` platform upload) |
| `phr_shared_space` | `C`, `E`, `I`, `J` | none | Placing converted `.glb` / `.vrm` assets inside a shared addressable space where other visitors view or interact with them (`E` provision/use with third parties) |

## Platform restriction review hint (`platformRestrictionHint`)

Some BOOTH VN3 licenses mark `C` (upload to social/online platforms) and `I` (format conversion) as
`permitted`, while restricting upload to a single named platform inside `C`'s
`limited_publication_only` condition or `X` (Special Notes) — for example `"VRChatのみ"`,
`"VRChat限定"`, `"VRChat専用"`, `"VRChat以外へのアップロード禁止"`, or `"VRChat only"`.

In `terms.checkUse` `v0.2`:

1. Every per-product decision and the `combination` decision include
   `platformRestrictionHint: boolean`.
2. `platformRestrictionHint` is `true` on a product when:
   - effective item `C` carries `condition == "limited_publication_only"`, or
   - any captured `specialNotes[].text` or effective item `C` `rawText` matches deterministic
     platform-exclusivity phrases (`vrchatのみ`, `vrchat限定`, `vrchat専用`, `vrchat以外`,
     `vrchat only`, `only in vrchat`, `only for vrchat`, `exclusive to vrchat`,
     `他プラットフォーム`, `他のプラットフォーム`, `other platform`).
3. `combination.platformRestrictionHint` equals `products.some(p => p.platformRestrictionHint)`.
4. **Honesty rule**: `platformRestrictionHint` never overrides `decision`. Free-text legal clauses
   in `X` require human reading; VUA surfaces the original `specialNotes` text alongside the warning
   badge rather than guessing legal conclusions.

## Document changelog

- 0.1.0 (2026-11-14): add `terms.checkUse` `v0.2` with `phr_personal_avatar`, `phr_object_head`,
  `phr_shared_space` (`C`/`E`/`I`/`J`) and deterministic `platformRestrictionHint`.
