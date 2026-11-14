# Pose source and device graph v0.1

> Document version: 0.3.0
> Status: Candidate
> Owner: Environment
> Updated: 2026-11-14
> Schemas: [request](../../schemas/pose-graph/v0.1/request.schema.json), [snapshot](../../schemas/pose-graph/v0.1/snapshot.schema.json), [vectors](../../schemas/pose-graph/v0.1/vectors.json)

This Candidate face aggregates local pose-tracking runtimes, SteamVR external drivers, Safe Mode
driver blocks, companion pose processes (`Space Calibrator`, `SlimeVR`, `PICO Connect`, `mocopi`,
`HaritoraX`, `Rebocap`, `VRCFaceTracking`) and the **Pharos** optical+IMU dual-mode tracking hub
contract into a single privacy-safe read-only snapshot (`environment.observePoseGraph`). It belongs
to the Host (`Environment`) provider and remains available whether or not the AMF module is enabled.

## Scope and honesty invariants

1. **Read-only local observation**: inspects the Windows OpenXR active runtime registry value,
   `openvrpaths.vrpath` `external_drivers` registrations plus each driver folder's
   `driver.vrdrivermanifest` `name` (`<= 64 KiB`, non-symlink), `steamvr.vrsettings` `LastKnown`
   headset hints and `driver_*.blocked_by_safe_mode` flags, local tool installations, Pharos
   `hub-status.json` (`<= 64 KiB`, non-symlink), and OS process basenames in a single `Toolhelp32`
   snapshot pass without opening foreign process memory.
2. **Privacy-safe projections**:
   - OpenXR runtime names and headset hints are trimmed display labels (`<= 80` UTF-16 code units)
     stripped of filesystem paths; raw manifest paths and user account directories never cross the
     wire.
   - `steamvrDrivers` and `blockedSteamvrDrivers` emit only recognized closed driver IDs
     (`space_calibrator`, `slimevr`, `alvr`, `virtual_desktop`, `lighthouse`, `pharos`, `mocopi`,
     `haritorax`, `rebocap`), never unknown third-party folder names.
3. **Honest hardware absence and freshness gate (Pharos hub contract)**:
   - VUA serves as the PC control-center anchor for sibling project **Pharos** (optical `2x2x2`
     stereo base stations + panoramic spatial anchor base station + active IR+IMU fusion wearables
     on `left_foot`, `right_foot`, `waist` and `chest` with automatic battery-depletion fallback to
     `passive_retroreflective` tracking and foot-heading support).
   - `pharosHub.telemetryState` is `"live"` **only** when a Pharos process (`pharos.exe` or
     `pharos-hub.exe`) is actively running right now AND `hub-status.json` carries a valid RFC 3339
     `updatedAt` timestamp within `[-2 s, +15 s]` of `capturedAt`.
   - Whenever `telemetryState` is `"missing"`, `"stale"`, or `"malformed"`, `activeModes` is `[]`,
     `calibrationState` is `"unknown"`, `spatialAnchorReady` is `false`, all four wearable slots
     report `"not_connected"` (`trackingMode: null`, `batteryPercent: null`,
     `footHeadingAvailable: false`), and `stereoBaseStationCount` / `panoramicAnchorCount` are
     `null` — VUA never fabricates connected hardware or stale counts.

## Operation

| Method | Kind | Params | Result |
| --- | --- | --- | --- |
| `environment.observePoseGraph` | query | `{ schemaVersion: "0.1" }` | `{ poseGraph }` |

## Closed vocabularies (`0.3.0`)

- `fbtDriverConflictRisk`:
  - `none` — zero or one unblocked external OpenVR full-body tracking driver registered in `steamvrDrivers`
  - `multiple_openvr_fbt_drivers` — two or more unblocked external OpenVR full-body tracking drivers among `pharos`, `slimevr`, `mocopi`, `haritorax` and `rebocap` are simultaneously registered, risking virtual tracker slot contention
- `poseSources[].sourceId` (emitted in canonical order, 10 items):
  - `headset_6dof` — HMD / OpenXR / SteamVR 6DoF head & hand pose
  - `pharos` — Pharos optical+IMU dual-mode full-body & spatial anchor hub
  - `slimevr` — SlimeVR IMU full-body tracking server / SteamVR driver
  - `pico_motion` — PICO Connect PCVR streaming & PICO Motion Tracker bridge
  - `lighthouse` — SteamVR Lighthouse optical base-station ecosystem
  - `space_calibrator` — OpenVR Space Calibrator multi-tracking-space alignment
  - `mocopi` — Sony mocopi companion / OpenVR receiver
  - `haritorax` — Shiftall HaritoraX configurator / VR bridge
  - `rebocap` — Rebocap full-body tracking companion / driver
  - `eye_face_osc` — VRCFaceTracking eye & expression OSC bridge
- `poseSources[].state`: `active | ready | installed | not_detected` (`detailCode: "blocked_by_safe_mode"` surfaces silent SteamVR Safe Mode blocks)
- `steamvrDrivers[]` / `blockedSteamvrDrivers[]`:
  - `space_calibrator`, `slimevr`, `alvr`, `virtual_desktop`, `lighthouse`, `pharos`, `mocopi`, `haritorax`, `rebocap`
- `pharosHub`:
  - `telemetryState`: `missing | live | stale | malformed`
  - `calibrationState`: `unknown | uncalibrated | calibrating | calibrated`
  - `spatialAnchorReady`: `boolean` (PH-R zero-QR multi-user co-location readiness)
  - `activeModes[]`: `optical_stereo_2x2x2`, `imu_fusion`, `passive_retroreflective`, `panoramic_spatial_anchor`
  - `wearableNodes[]`: `slot` (`left_foot | right_foot | waist | chest`), `state`, `trackingMode` (`optical_imu | passive_retroreflective | imu_only | null`), `batteryPercent` (`0..=100 | null`), `footHeadingAvailable`

## Document changelog

- 0.3.0 (2026-11-14): add `fbtDriverConflictRisk` (`none | multiple_openvr_fbt_drivers`) for simultaneous unblocked external OpenVR FBT drivers (`pharos`, `slimevr`, `mocopi`, `haritorax`, `rebocap`) and bound `libraryfolders.vdf` reads (`<= 64 KiB`, non-symlink).
- 0.2.0 (2026-11-14): add freshness-gated Pharos `hub-status.json` live telemetry (per-node
  `trackingMode` including `passive_retroreflective` battery-depletion fallback, `batteryPercent`,
  `calibrationState`, `spatialAnchorReady`), `driver.vrdrivermanifest` inspection,
  `blockedSteamvrDrivers` Safe Mode reporting, single-pass `Toolhelp32` process observation, and
  `space_calibrator` / `mocopi` / `haritorax` / `rebocap` pose sources.
- 0.1.0 (2026-11-14): introduce Host-owned unified pose source & device graph observation and the
  Pharos optical+IMU dual-mode hub contract.
