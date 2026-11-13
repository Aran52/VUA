---
catalog_schema: "vua.tool-entry/v3"
id: "external.vrcs"
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

# VRCS

[VRCS](https://github.com/Dreaminko/VRCS) provides speech recognition, translation and VRChat
chatbox output (`0.3.7`, `VRCS-0.3.7-windows-x64.exe`, `AGPL-3.0`, license verified 2026-11-11).

- **Permissions and installation (`medium` risk):** VUA acquires the pinned official per-user NSIS
  installer, verifies exact byte length and SHA-256, refuses installation while `vrcs.exe` is
  running, and runs `/S` without shell invocation.
- **Discovery, launch and stop:** VUA inspects `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\VRCS`
  and `%LOCALAPPDATA%\VRCS` for `VRCS.exe`, launches with job breakaway through
  [installer-tool v0.1](../../protocols/installer-tool-v0.1.md), and sends normal window close only
  to an owned instance.
- **Cancellation, rollback and user data:** Cooperative cancellation before process start removes
  only VUA's staged partial file; installed files and VRCS user settings stay untouched.
- **License and mirroring (`AGPL-3.0`):** VRCS runs as a separate external process over standard OS
  boundaries; VUA does not link, modify or bundle VRCS code. Acquisition uses the official GitHub
  release; if mirrored in the future, the unmodified installer must be accompanied by the exact
  `0.3.7` upstream source archive and `AGPL-3.0` license text.
