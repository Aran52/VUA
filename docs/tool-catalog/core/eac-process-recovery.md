---
catalog_schema: "vua.tool-entry/v3"
id: "core.eac-process-recovery"
boundary: "core"
status: "experimental"
risk: "high"
risk_rule: "vua.risk-gate/v1"
delivery: "unscheduled"
maintainer: "VUA-Project"
distribution: "core"
platforms: ["windows"]
capabilities: ["process.inspect", "process.terminate.allowlisted", "installer.launch.official.elevated"]
---

# EAC Conflict Detection and Residual-Process Recovery

> **Experimental high-risk feature: terminating EAC- or VRChat-related processes can interrupt the
> game or leave EAC in an abnormal state. In extreme cases it may trigger platform or account safety
> checks and account anomalies. VUA cannot guarantee platform-side outcomes. It is disabled by
> default and requires explicit opt-in and confirmation on every execution.**


It uses high-risk Orchestrator system capabilities to recover conflicts that prevent VRChat launch,
so it can only be a core module and is never exposed to web content, overlays, or community plugins.
Read-only diagnosis lives separately in [`core.eac-status`](eac-status.md); prefer normal exit,
waiting for driver unload, restarting Windows, or official EAC repair before process termination.

Official EAC repair launch holds a read-locked handle (`FILE_SHARE_READ`) across WinVerifyTrust
signature verification (`EasyAntiCheat Oy` / `Epic Games, Inc.`) and invokes
`EasyAntiCheat_EOS_Setup.exe` via Windows UAC (`runas`) with no arguments only after per-invocation
user confirmation and only when no VRChat session is active.

Only a reviewed allowlist of user-mode residual processes is eligible for termination. Verify and
display process name, PID, creation timestamp, absolute path, available signer/publisher, and reason;
refuse during active gameplay. Never stop services/drivers, change EAC/VRChat files/configuration,
inject/hook/hide processes, bypass anti-cheat, or terminate by broad name matching. Unknown evidence
permits diagnosis only.
