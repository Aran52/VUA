---
catalog_schema: "vua.tool-entry/v3"
id: "external.vrcx"
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

# VRCX

[VRCX](https://github.com/vrcx-team/VRCX) is an independent companion application for VRChat
friends, instance history and local notifications (`v2026.09.16`, `VRCX_2026.09.16_Setup.exe`, `MIT`).

- **Account and data isolation:** VRCX owns its own VRChat sign-in, session cookies and local
  SQLite database under `%APPDATA%\VRCX`. Under the [product boundary](../../product-boundary.md),
  VUA never reads, imports, shares or modifies VRCX credentials, sessions or databases.
- **Permissions and installation (`medium` risk):** VUA downloads the pinned official NSIS setup,
  verifies byte length and SHA-256, refuses overwrite while `vrcx.exe` is running, and runs `/S`.
- **Discovery, launch and stop:** VUA inspects VRCX's uninstall registry entry and standard
  `%LOCALAPPDATA%\Programs\VRCX` / `%ProgramFiles%\VRCX` locations for `VRCX.exe`, launches with
  job breakaway through [installer-tool v0.1](../../protocols/installer-tool-v0.1.md), and closes
  only an instance started by VUA in the current session.
- **Cancellation, rollback and mirroring:** Cooperative download cancellation deletes only VUA's
  partial file; VRCX's installed files and `%APPDATA%\VRCX` data are never removed by VUA. `MIT`
  permits unmodified installer mirroring with the upstream license notice; official GitHub release
  remains primary.
