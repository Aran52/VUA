# GLB, VRM 1.0 and Object-Head clearance inspection v0.1

> Document version: 0.2.0
> Status: Candidate
> Owner: AMF
> Updated: 2026-11-14
> Schemas: [request](../../schemas/glb-inspection/v0.1/request.schema.json), [result](../../schemas/glb-inspection/v0.1/result.schema.json), [vectors](../../schemas/glb-inspection/v0.1/vectors.json)

This Candidate face inspects `.glb` and `.vrm` binary glTF 2.0 artifacts already stored inside
VUA's local warehouse (`library.inspectGlb`) — both standalone files and `.glb`/`.vrm` members
inside unexpanded `.zip` archives — without launching Unity, writing temporary files to disk, or
reading binary geometry/texture buffers into memory. It validates humanoid **VRM 1.0 (`VRMC_vrm`)**
and **Pharos** 15-bone + toe readiness, detects legacy **VRM 0.x (`VRM`)** archives that benefit
from upgrade, extracts geometry/material/texture/SpringBone summaries and embedded license metadata,
and evaluates **PH-R Stage 1 MR Object-Head clearance** (`>= 5 cm` per-side margin around a
reference human head + HMD envelope plus mount-center offset).

## Safety and streaming bounds

1. **Warehouse-contained artifacts only**: callers pass either `productId` (`booth:<digits>`) or
   `entryId` (`warehouseItemId`), never a filesystem path. Only regular non-symlink files verified
   inside the canonical warehouse root by `PresenceCheck` are opened.
2. **Header + JSON chunk streaming only (standalone and `.zip` members)**:
   - Reads the 12-byte GLB header (`magic == 0x46546C67`, `version == 2`) and the 8-byte Chunk 0
     header (`chunkType == 0x4E4F534A` JSON).
   - For unexpanded `.zip` copies (those without an `expanded/<copyId>/<member>` copy already
     registered), scans at most `4,096` central-directory entries, skips directories, symlinks and
     encrypted members, and streams at most `20 + 16 MiB` of each `.glb`/`.vrm` entry (`sourceKind:
     "zip_member"`, `containerFileName: "<archive>.zip"`).
   - Refuses JSON chunks larger than `16 MiB` as `invalid_glb` and never reads Chunk 1 (`BIN`
     geometry/texture payload), keeping inspection fast and memory-bounded even on `100+ MiB`
     avatars.
   - Inspects at most `64` `.glb`/`.vrm` items per query and sets `truncated: true` when more exist
     or when a `.zip` exceeds `4,096` entries.
3. **Path redaction**: `fileName` and `containerFileName` expose only warehouse-relative names,
   never host filesystem paths.

## Operation

| Method | Kind | Params | Result |
| --- | --- | --- | --- |
| `library.inspectGlb` | query | `{ schemaVersion: "0.1", productId }` XOR `{ schemaVersion: "0.1", entryId }` | `GlbInspectionResultV01` |

## Extracted glTF / VRM metrics (`0.2.0`)

- **Geometry (`contentSummary`)**: `meshCount`, `primitiveCount`, deterministic `triangleEstimate`
  from index or `POSITION` accessor counts, `skinCount`, `nodeCount`, and `animationCount`.
- **Materials & textures (`materialSummary`, `textureSummary`)**: total materials, `KHR_materials_unlit`
  and `VRMC_materials_mtoon` / VRM 0.x MToon counts, plus embedded vs external-URI image counts so
  broken non-self-contained `.glb` exports are surfaced before import.
- **Secondary motion (`secondaryMotion`)**: `VRMC_springBone` / VRM 0.x `secondaryAnimation` joint
  counts, `VRMC_node_constraint` node counts, and `VRMC_vrm.expressions` / VRM 0.x BlendShape group
  counts.
- **Pharos & humanoid VRM metrics (`vrm`)**: `hasHumanoidHeadBone`, `hasFullBodyBones` (15 core
  humanoid bones), `hasToeBones` (`leftToes` + `rightToes` for Pharos optical/IMU foot orientation),
  and `headBoneHeightMeters` (scene-space `Y` of the `head` bone node in `[0, 10]` m).

## Object-Head clearance rule (PH-R Stage 1)

PH-R Stage 1 mixed-reality head replacement mounts a standard glTF 2.0 `.glb` prop over the wearer's
physical head and HMD so occlusion hides the physical headset cleanly.

- **Reference head + HMD envelope (`referenceHeadHmdMeters`)**: `[0.22, 0.26, 0.28]` meters
  (`[width X, height Y, depth Z]`).
- **Scene bounds (`boundsMeters`)**: computed by traversing the active glTF scene's node hierarchy
  (`matrix` column-major `4x4` or `translation` / `rotation` / `scale` TRS, bounded to `128` depth
  and `16,384` node visits) and transforming each mesh primitive `POSITION` accessor's `[min, max]`
  3D bounding-box corners into root scene meters.
- **Per-side margin (`marginsMeters`)** and **mount center (`mountOffsetCenterMeters`)**:
  `[(sizeX - 0.22) / 2, (sizeY - 0.26) / 2, (sizeZ - 0.28) / 2]` and `[(min + max) / 2]`.
- **Classification (`objectHeadClearance.status`)**:
  - `passes`: `max(sizeX, sizeY, sizeZ) <= 2.0` m and all three per-side margins are `>= 0.05` m
    (`5 cm`, i.e. total outer dimensions `>= [0.32, 0.36, 0.38]` m).
  - `likely_unscaled`: `max(sizeX, sizeY, sizeZ) > 2.0` m (for example centimeter-scale FBX/DCC
    exports that were not scaled `0.01x` to glTF meters).
  - `insufficient_margin`: valid bounds `<= 2.0` m where at least one axis has `< 0.05` m per-side
    clearance.
  - `unknown`: no valid 3D `POSITION` accessor `min`/`max` bounds exist in the scene.

## PH-R readiness summary (`phrReadiness`)

- `ready_vrm1_humanoid`: `format == "vrm_1_0"` and `VRMC_vrm.humanoid.humanBones.head` points to a
  valid node index.
- `missing_head_bone`: `format == "vrm_1_0"` without a valid `head` bone binding.
- `vrm0_needs_upgrade`: `format == "vrm_0_x"` (`extensions.VRM` legacy 0.x archive; can be upgraded
  to VRM 1.0 or converted via `VRM Converter for VRChat`).
- `ready_object_head`: `format == "glb_2_0"` and `objectHeadClearance.status == "passes"`.
- `scale_suspect`: `format == "glb_2_0"` and `objectHeadClearance.status == "likely_unscaled"`.
- `clearance_insufficient`: `format == "glb_2_0"` and `objectHeadClearance.status` is
  `"insufficient_margin"` or `"unknown"`.
- `invalid_glb`: file header or JSON chunk is not valid glTF 2.0 binary.

## Document changelog

- 0.2.0 (2026-11-14): add zero-disk streaming `.glb`/`.vrm` inspection inside unexpanded warehouse
  `.zip` archives (`sourceKind`, `containerFileName`, `maxItems: 64`), geometry/material/texture/
  secondary-motion summaries, Pharos 15-bone + toe & head-height checks, and Object-Head mount
  center offset (`mountOffsetCenterMeters`).
- 0.1.0 (2026-11-14): introduce pure-Rust streaming `.glb` / `VRMC_vrm` 1.0 / Object-Head `5 cm`
  clearance inspection over warehouse copies.
