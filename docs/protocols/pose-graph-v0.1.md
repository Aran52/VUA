# Pose source and device graph v0.1

> Document version: 0.1.0
> Status: Candidate
> Owner: Environment
> Updated: 2026-11-14
> Schemas: [request](../../schemas/pose-graph/v0.1/request.schema.json), [snapshot](../../schemas/pose-graph/v0.1/snapshot.schema.json), [vectors](../../schemas/pose-graph/v0.1/vectors.json)

This Candidate face aggregates local pose-tracking runtimes, SteamVR external drivers, companion
pose processes and the **Pharos** optical+IMU dual-mode tracking hub contract into a single
privacy-safe read-only snapshot (`environment.observePoseGraph`). It belongs to the Host
(`Environment`) provider and remains available whether or not the AMF module is enabled.

## Scope and honesty invariants

1. **Read-only local observation**: inspects the Windows OpenXR active runtime registry value,
   `openvrpaths.vrpath` `external_drivers` registrations, `steamvr.vrsettings` `LastKnown` headset
   hints, local tool installations (`SlimeVR`, `PICO Connect`, `VRCFaceTracking`) and OS process
   names (`Toolhelp32` basename snapshot without opening foreign process memory).
2. **Privacy-safe projections**:
   - OpenXR runtime names and headset hints are trimmed display labels (`<= 80` characters) stripped
     of filesystem paths; raw manifest paths and user account directories never cross the wire.
   - `steamvrDrivers` emits only recognized closed driver IDs (`space_calibrator`, `slimevr`,
     `alvr`, `virtual_desktop`, `lighthouse`, `pharos`), never unknown third-party folder names.
3. **Honest hardware absence (Pharos hub contract)**:
   - VUA serves as the PC control-center anchor for sibling project **Pharos** (optical `2x2x2`
     stereo base stations + panoramic spatial anchor base station + IMU fusion wearables on
     `left_foot`, `right_foot`, `waist` and `chest` with foot-heading support).
   - When no Pharos driver or hub process is present, `pharosHub.state` is `"not_detected"`,
     `activeModes` is `[]`, all four wearable slots report `"not_connected"` with
     `footHeadingAvailable: false`, and `stereoBaseStationCount` / `panoramicAnchorCount` are
     `null`. Even when a Pharos driver or companion process is detected without live hardware
     telemetry, node states and station counts remain `"not_connected"` / `null` — VUA never
     fabricates connected hardware or synthetic counts.

## Operation

| Method | Kind | Params | Result |
| --- | --- | --- | --- |
| `environment.observePoseGraph` | query | `{ schemaVersion: "0.1" }` | `{ poseGraph }` |

## Closed vocabularies

- `poseSources[].sourceId` (emitted in canonical order, 6 items):
  - `headset_6dof` — HMD / OpenXR / SteamVR 6DoF head & hand pose
  - `pharos` — Pharos optical+IMU dual-mode full-body & spatial anchor hub
  - `slimevr` — SlimeVR IMU full-body tracking server / SteamVR driver
  - `pico_motion` — PICO Connect PCVR streaming & PICO Motion Tracker bridge
  - `lighthouse` — SteamVR Lighthouse optical base-station / Space Calibrator ecosystem
  - `eye_face_osc` — VRCFaceTracking eye & expression OSC bridge
- `poseSources[].state`:
  - `active` — companion or runtime process currently running
  - `ready` — registered in OpenXR / `openvrpaths` external drivers and ready on launch
  - `installed` — local application detected on disk, not currently running
  - `not_detected` — no local runtime, driver or installation observed
- `steamvrDrivers[]`:
  - `space_calibrator`, `slimevr`, `alvr`, `virtual_desktop`, `lighthouse`, `pharos`
- `pharosHub.activeModes[]`:
  - `optical_stereo_2x2x2`, `imu_fusion`, `passive_retroreflective`, `panoramic_spatial_anchor`
- `pharosHub.wearableNodes[].slot`:
  - `left_foot`, `right_foot`, `waist`, `chest`

## Document changelog

- 0.1.0 (2026-11-14): introduce Host-owned unified pose source & device graph observation and the
  Pharos optical+IMU dual-mode hub contract.
