---
catalog_schema: "vua.tool-entry/v3"
id: "external.vrct"
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

# VRCT

[VRCT](https://github.com/misyaguziya/VRCT) provides VRChat chatbox translation and speech
transcription over OSC (`v3.6.1`, `VRCT_3.6.1_x64-setup.exe`).

- **Permissions and installation (`medium` risk):** the official per-user NSIS installer installs
  under `%LOCALAPPDATA%\VRCT` (`HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\VRCT`)
  with `/S` after VUA verifies the pinned byte size and SHA-256. Refuses installation while
  `vrct.exe` is running.
- **Discovery, launch and stop:** VUA observes `VRCT.exe`, launches it directly with job breakaway
  through [installer-tool v0.1](../../protocols/installer-tool-v0.1.md), and requests normal window
  close only for an owned instance.
- **Cancellation, rollback and user data:** cancelling before installer launch removes only VUA's
  staged partial download; cancelling during NSIS execution waits for installer completion without
  deleting `%LOCALAPPDATA%\VRCT` or user translation/API settings.
- **License and mirroring restriction:** VRCT's code is `MIT`, **except** its bundled chat-bubble
  3D model which is licensed exclusively for VRCT and prohibits redistribution without written
  author permission. VUA acquires `VRCT_3.6.1_x64-setup.exe` **strictly from the official GitHub
  release** and never mirrors or redistributes the installer.
