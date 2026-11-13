# Official-installer tool launch and observation v0.1

> Document version: 0.1.0
> Status: Candidate
> Owner: Environment
> Updated: 2026-11-11
> Schemas: [request](../../schemas/installer-tool/v0.1/request.schema.json), [result](../../schemas/installer-tool/v0.1/result.schema.json), [vectors](../../schemas/installer-tool/v0.1/vectors.json)

This N2 face observes, launches and closes four fixed official-installer applications (`slimevr`,
`vrct`, `vrcx` and `vrcs`) without Steam. Acquisition, integrity verification and silent NSIS
installation run through [environment deployment v0.1](environment-deployment-v0.1.md)
(`install_official_tool`) so downloads, byte progress, elevation and the session-wide installer
lock share one audited task pipeline.

| toolId | Application | Pinned official release | Main executable | License / mirror classification |
| --- | --- | --- | --- | --- |
| `slimevr` | SlimeVR Server | `SlimeVR/SlimeVR-Installer` `v5.0.0` (`slimevr_web_installer.exe`) | `SlimeVR.exe` | `MIT AND Apache-2.0` — mirror-eligible with license and attribution notices |
| `vrct` | VRCT | `misyaguziya/VRCT` `v3.6.1` (`VRCT_3.6.1_x64-setup.exe`) | `VRCT.exe` | `MIT` code except VRCT-exclusive chat-bubble model — official GitHub release only; no mirroring without written permission |
| `vrcx` | VRCX | `vrcx-team/VRCX` `v2026.09.16` (`VRCX_2026.09.16_Setup.exe`) | `VRCX.exe` | `MIT` — mirror-eligible with MIT license; VRCX owns its own VRChat sign-in and local database |
| `vrcs` | VRCS | `Dreaminko/VRCS` `0.3.7` (`VRCS-0.3.7-windows-x64.exe`) | `VRCS.exe` | `AGPL-3.0` — official GitHub release first; any mirror must serve the unmodified release alongside the corresponding `0.3.7` source archive and `AGPL-3.0` license |

| Method | Kind | Params | Result |
| --- | --- | --- | --- |
| `tools.observeInstaller` | query | `{ toolId }` | `{ installerTool }` |
| `tools.actInstaller` | command | `{ toolId, action }` | `{ installerTool }` |

`toolId` is one of `slimevr`, `vrct`, `vrcx` or `vrcs`. Requests use Application Contract `0.1`.
Commands carry a closed `commandId` (`1..=128` ASCII `[A-Za-z0-9_.:-]+`) at envelope level; Desktop
Gateway moves it from IPC params without changing identity. Actions are `start`, `stop` and
`cancel`. Replaying an admitted command ID with the same action returns current facts without
redispatch; a different action is rejected (`vua.installer_tool.invalid_command`). The
process-local admission ledger is bounded to 256 commands and resets on restart.

`installerTool` contains `schemaVersion: "vua.installer-tool/v0.1"`, `toolId`, `capturedAt`,
`presence` (`installed`, `missing`, `incomplete`, `unknown`), nullable `location`, nullable
`version`, `running`, `canStop`, `activity` (`idle`, `starting`, `stopping`) and nullable `issue`
(`not_installed`, `handoff_failed`, `start_timeout`, `close_failed`, `close_timeout`). `installed`
requires an observed registry or known vendor directory entry with a valid Windows PE (`MZ`) main
executable; an existing directory without a valid executable reports `incomplete`.

Start launches the observed main executable directly with `CREATE_BREAKAWAY_FROM_JOB` from its own
working directory (no shell, no renderer-supplied path or arguments) and observes a new matching
process for at most 60 seconds. Stop sends a normal window close only to an instance started and
adopted by this service (rechecking PID, creation time and image path) and waits at most eight
seconds; pre-existing instances are never closed and no force termination is exposed. Cancel stops
observing a pending start without terminating the upstream application.

## Document changelog

- 0.1.0 (2026-11-11): introduce the four-tool official-installer observation and launch face alongside deployment v0.1 `install_official_tool`.
