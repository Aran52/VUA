# Local multi-layer diagnostics and guarded client helpers v0.1

> Document version: 0.1.0
> Status: Candidate
> Owner: Environment
> Updated: 2026-11-14
> Schemas: [request](../../schemas/local-diagnostics/v0.1/request.schema.json), [snapshot](../../schemas/local-diagnostics/v0.1/snapshot.schema.json), [symptom-diagnosis](../../schemas/local-diagnostics/v0.1/symptom-diagnosis.schema.json), [osc-cache-clear-receipt](../../schemas/local-diagnostics/v0.1/osc-cache-clear-receipt.schema.json), [client-config-update-receipt](../../schemas/local-diagnostics/v0.1/client-config-update-receipt.schema.json), [vectors](../../schemas/local-diagnostics/v0.1/vectors.json)

This Candidate face observes local desktop VR runtime, OSC, cache, SteamVR configuration and
privacy-safe log failure signatures, joins those live facts into deterministic multi-layer
symptom-to-SOP diagnoses, and offers two guarded offline helpers (`diagnostics.clearOscAvatarCache`
and `diagnostics.updateClientConfig`). It belongs to the Host (`Environment`) provider and remains
available whether or not the AMF module is enabled.

## Safety and privacy invariants

1. **Zero game-process attachment or EAC contact**: process checks use read-only OS process table
   snapshots (`Toolhelp32` on Windows) without opening game memory or handles for injection.
2. **Zero UDP port seizure**: UDP `9000` (client OSC input) and `9001` (client OSC output) are
   inspected strictly through the OS UDP owner table (`GetExtendedUdpTable` `AF_INET`,
   `UDP_TABLE_OWNER_PID`). VUA never binds a listener on `9000` or `9001`.
3. **Privacy-safe projections**:
   - OSC cache inspection emits file count, total byte size and newest modification timestamp only;
     `usr_*` and `avtr_*` identifiers are never exported.
   - UDP port owners emit PID and lowercased executable basename only, never full user paths.
   - Client log inspection reads at most the trailing `128 KiB` of the newest `output_log_*.txt`
     and emits only a closed classification enum plus modification timestamp — raw log lines, world
     IDs (`wrld_*`), user IDs (`usr_*`) and local paths never cross the contract boundary.
4. **Fail-closed offline helpers**: `diagnostics.clearOscAvatarCache` and
   `diagnostics.updateClientConfig` refuse with `vua.diagnostics.client_running` whenever
   `VRChat.exe` is running or process liveness cannot be verified.

## Operations

| Method | Kind | Params | Result |
| --- | --- | --- | --- |
| `diagnostics.observeLocal` | query | `{ schemaVersion: "0.1" }` | `{ localDiagnostics }` |
| `diagnostics.diagnoseSymptom` | query | `{ schemaVersion: "0.1", symptom }` | `{ symptomDiagnosis }` |
| `diagnostics.clearOscAvatarCache` | command | `{ schemaVersion: "0.1" }` | `{ oscCacheClearReceipt }` |
| `diagnostics.updateClientConfig` | command | `{ schemaVersion: "0.1", cacheSizeGb?, cacheExpiryDelayDays? }` | `{ clientConfigReceipt }` |

Commands carry a closed `commandId` (`1..=128` ASCII `[A-Za-z0-9_.:-]+`) at the Application Contract
`0.1` envelope level; Desktop Gateway moves `commandId` from IPC params without changing identity.
Replaying an admitted command ID with identical parameters returns the recorded receipt without
repeating filesystem writes; replaying the same command ID with different parameters fails with
`vua.diagnostics.invalid_command`. The process-local admission ledger retains up to 256 commands.

## Closed vocabularies

- `symptom`:
  - `audio_silent_or_wrong_mic`
  - `osc_or_face_tracking_not_responding`
  - `low_fps_or_vram_pressure`
  - `disk_cache_full`
  - `crash_or_eac_block`
  - `avatar_hidden_or_grey_robot`
  - `cannot_upload_avatar`
- `recentClientLog.classification`:
  - `no_logs`, `clean_or_inconclusive`, `eac_launch_violation`, `disk_full_write_error`,
    `d3d11_device_lost_or_oom`, `osc_socket_conflict`, `openvr_init_failed`, `read_failed`
- `findings[].code`:
  - `osc_port_9000_unbound_while_client_running`
  - `osc_port_9000_conflict`
  - `osc_port_9001_unbound`
  - `osc_avatar_cache_present`
  - `steamvr_driver_blocked_by_safe_mode`
  - `steamvr_high_supersampling`
  - `steamvr_app_overscaled`
  - `client_cache_near_limit`
  - `drive_free_space_low`
  - `client_config_malformed`
  - `log_eac_violation_detected`
  - `log_disk_full_detected`
  - `log_d3d11_crash_detected`
  - `log_osc_conflict_detected`
  - `log_openvr_init_failed`
- `layer`:
  - `hardware_os`, `steamvr_openxr`, `companion_osc`, `client_config_cache`, `avatar_sdk`

## Guarded helper rules

- `diagnostics.clearOscAvatarCache` walks `VRChat/VRChat/OSC/usr_*/Avatars` under `local_low`,
  skips symlinks and reparse points, removes regular `.json` files only, and returns
  `{ removedFileCount, freedBytes, clearedAt }`.
- `diagnostics.updateClientConfig` requires at least one of `cacheSizeGb` (`1..=200` integer) or
  `cacheExpiryDelayDays` (`1..=365` integer). If `VRChat/VRChat/config.json` exists and is not a
  JSON object, it fails with `vua.diagnostics.config_malformed` without modifying the file.
  Otherwise it preserves all existing JSON properties, writes `config.json.vua.bak` when updating
  an existing file, syncs a sibling temporary file to disk, and atomically renames it into place.

## Document changelog

- 0.1.0 (2026-11-14): introduce Host-owned local multi-layer observation, deterministic symptom-to-SOP diagnosis and offline OSC/config helpers.
