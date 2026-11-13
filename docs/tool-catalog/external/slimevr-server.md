---
catalog_schema: "vua.tool-entry/v3"
id: "external.slimevr-server"
boundary: "external"
status: "experimental"
risk: "medium"
risk_rule: "vua.risk-gate/v1"
delivery: "unscheduled"
maintainer: "external-upstream"
distribution: "external-connection"
platforms: ["windows"]
capabilities: ["external.runtime.discover", "external.runtime.connect"]
---

# SlimeVR Server

[SlimeVR Server](https://github.com/SlimeVR/SlimeVR-Installer) coordinates Wi-Fi IMU full-body
trackers and installs its SteamVR driver and bundled Java runtime through the official Windows
installer (`v5.0.0`, `slimevr_web_installer.exe`, `MIT AND Apache-2.0`).

- **Permissions and elevation (`medium` risk):** the official NSIS web installer declares
  `requireAdministrator`, installs under `%ProgramFiles(x86)%\SlimeVR Server` (or `%ProgramFiles%\SlimeVR Server`),
  and registers the SlimeVR SteamVR driver. Windows UAC owns the elevation prompt during
  `install_official_tool`; VUA passes only `/S` after verifying the pinned byte length and SHA-256.
- **Discovery, launch and stop:** VUA inspects `HKLM\...\Uninstall\SlimeVR` and standard Program
  Files roots for `SlimeVR.exe`, launches the verified executable with job breakaway through
  [installer-tool v0.1](../../protocols/installer-tool-v0.1.md), and sends a normal window close
  only to an instance adopted by the current session.
- **Cancellation, rollback and user data:** download and hash verification stop cooperatively on
  cancellation; once the native installer starts, cancellation waits for process exit and retains
  installed files without forced termination or automatic OS rollback. Tracker calibration, Wi-Fi
  credentials and SteamVR driver configuration remain owned by SlimeVR.
- **Mirroring:** `MIT AND Apache-2.0` permits unmodified installer mirroring when accompanied by
  upstream license and attribution notices; official GitHub release remains primary.
