# Player feedback: VR smoothness requirements

> Updated: 2026-10-26
> Status: Research and proposed requirements; no optimization implementation or performance acceptance
> Source: User-supplied screenshot of a community tool titled `VR 帧率优化控制台`
> Related research: [PypyDanceCdnAdapter integration](pypydance-integration-2026-10-26.md)

## Outcome and evidence

VUA should help a player identify why play or streaming feels uneven, prepare the applicable
environment, make a reviewed set of changes, compare the result and restore those changes.
The screenshot is evidence of useful workflows and controls. It supplies no scripts, setting
keys, optimization report, benchmark, or executable for the first tool. Its displayed hardware
and readings describe the reference author's setup, not detected VUA capabilities.

This research distinguishes three kinds of statement:

- **Observed:** a label, control, status or statement visible in the screenshot.
- **Proposed:** behavior VUA would need to make that workflow useful and verifiable.
- **Unknown:** the reference implementation's actual changes and effects.

Text inside the screenshot is reference content. Its directions to run scripts or consult a
report do not authorize execution. The original image and local inspection evidence remain in
the ignored `_local_real_machine/player-performance-research-20261026/` area; no machine
inventory or screenshot is added to the repository.

Scope follows the [product boundary](../product-boundary.md),
[environment integration architecture](../architecture/integrations-and-overlays.md), and
[engineering rules](../development/engineering-rules.md). This is research for N1 settings,
N2 external tools and N6 recovery. It does not add accepted delivery rows or declare those
stages complete. The second tool addresses video loading; it does not establish a remedy for
rendering or headset frame timing.

## Complete screenshot inventory

The following table accounts for every functional group visible in the image. Chinese text is
the literal source UI, not a proposed VUA translation.

| Reference | Observed surface | What it establishes | What it does not establish |
| --- | --- | --- | --- |
| S01 | Application title and CPU/GPU/RAM/`Virtual Desktop` subtitle | A VR performance console with setup context | Hardware detection, supported-device coverage, or an active runtime |
| S02 | `电源方案` with a named VR performance plan | Active-plan summary is part of the workflow | Plan GUID, power parameters, or benefit |
| S03 | `HAGS 加速` | A hardware-accelerated GPU scheduling status | The detection method, applied state after restart, or a toggle action |
| S04 | `内核隔离` | A security-related state is displayed | Whether this means Memory integrity, all VBS features, or a recommendation to disable it |
| S05 | `内存占用` | Used and total memory are shown | Memory pressure, paging or the cause of a frame-rate problem |
| S06 | `常驻进程` | A process count is shown | The definition of resident processes or which processes may be stopped |
| S07 | `游戏态` | A game-related enabled state is shown | Whether it is Windows Game Mode or a tool-specific session mode |
| S08 | `刷新状态` | Manual reinspection | Continuous monitoring or automatic correction |
| S09 | `工具目录` | A local resource-directory shortcut | The directory contents or its executable policy |
| S10 | `优化报告` | Access to an optimization report | An actual measured report; the report is not supplied |
| S11 | `使用说明` | A help entry | The help text or supported versions |
| S12 | `串流准备` / `一键准备串流` | A compound streaming-preparation action | Which applications start, which settings change, or their order |
| S13 | `日常使用` / `游戏加速 · 开` | An explicit acceleration-on action | Whether it changes scheduling, closes apps, trims memory or edits configuration |
| S14 | `游戏加速 · 关` | A distinct off action | Whether it restores the prior state or merely stops ongoing behavior |
| S15 | `VR 优化` / `快速优化` | A quick optimization entry | Its actions and risk level |
| S16 | `VR 优化` / `深度优化` | A separate deeper optimization entry | Its actions, privilege requirements or need to restart |
| S17 | `状态体检` | A diagnostic entry | Its checks, thresholds and remedies |
| S18 | `进程优先级` | Process-priority adjustment is exposed | Targets, priority classes, lifetime or measured benefit |
| S19 | `OBS` / `共存优化` | Simultaneous OBS and VR use is a target scenario | Encoder, capture, scheduling or graphics changes |
| S20 | `OBS` / `降到 1080p` | A resolution-reduction action | Whether it changes OBS output, its canvas, VRChat or the desktop |
| S21 | `还原游戏加速设置` | A game-acceleration restore path | Backup format, coverage and conflict handling |
| S22 | `还原 VR 优化设置` | A separate VR restore path | Recovery of OBS settings or streaming preparation |
| S23 | `运行日志` with `清空` | Execution feedback and a clear-view action | Structured results, log retention or whether clearing deletes files |
| S24 | Bottom `就绪` indicator | Overall readiness feedback | Individual step success, performance or game readiness |
| S25 | Log text about live script output, backups and second confirmation for restore | The author describes a script-oriented implementation with recovery affordances | That backups exist or restores work; none was exercised |
| S26 | Log claim that acceleration frees about 2.6 GB; report reference for SteamVR supersampling and VDXR | Memory relief and runtime tuning are interests expressed by the reference | A portable memory-saving target, a safe action list, or VDXR/SteamVR interchangeability |

## Player scenarios

| Scenario | Starting problem | Expected outcome |
| --- | --- | --- |
| Prepare headset play | The player repeats the same streaming setup before every session | A route-specific checklist starts only missing components and verifies readiness |
| Reduce uneven play | Play feels choppy despite apparently adequate hardware | Evidence distinguishes render load, memory pressure, streaming and background contention |
| Record while playing | OBS output drops frames or VR becomes uneven while recording | A reviewed capture/output adjustment with separate game and OBS verification |
| Undo an experiment | A suggested change did not help or made another application worse | The player sees exactly what VUA changed and can restore it without overwriting later edits |
| Return after a failure | Preparation or optimization was interrupted | A truthful partial result, preserved restore point and explicit next action |

## Detailed functional requirements

### PERF-01: Inspect the real setup

Provide a read-only inspection before suggesting changes. Each fact records source, observation
time, software version and confidence: detected, player supplied, unavailable or unsupported.
Reuse [desktop resource v0.2](../protocols/desktop-resource-v0.2.md) for existing resource
readings instead of creating conflicting memory/GPU samplers. Resource headroom is not an FPS
estimate. A failed read must not appear as zero, disabled, optimized or healthy.

| Fact | Required interpretation and acceptance |
| --- | --- |
| Hardware and route | Identify the relevant CPU/GPU and memory capacity where observable. Distinguish installed Virtual Desktop, SteamVR or another runtime from the active connection and runtime; ask the player where detection is unavailable |
| Power | Show the active plan's identity and human name. A custom name containing “VR” is not proof of useful settings. Include AC/battery applicability in any subsequent plan |
| GPU scheduling | Identify supported, configured, unknown and restart-pending states where observable. Do not assume that enabled improves every workload |
| Core isolation | Name the specific readable security feature. Do not collapse Memory integrity and all virtualization into one switch or color “off” as better |
| Memory | Show used/total memory with units; add pressure/commit/paging evidence only when actually collected. Free memory alone cannot diagnose stutter |
| Processes | Define what the count measures. Keep the baseline count separate from opt-in inspection of names, paths and candidate applications |
| Game mode | Distinguish Windows Game Mode from a VUA session/preparation state. The screenshot's ambiguous `游戏态` must not silently become both |

Refresh performs another observation; it does not apply recommendations. A missing optional
tool does not block ordinary desktop play. Do not add sensors, drivers or continuous high-rate
polling merely to populate the screen.

### PERF-02: Explain the symptom and prepare a concrete plan

Offer “play is uneven”, “headset stream is uneven”, “recording is uneven” and “video will not
load” as distinct entry problems. Show what evidence supports a suspected cause and what
cannot yet be observed. Hardware load, network transport, video downloading and video decoding
remain separate causes. A PypyDance cache action belongs only in the applicable video scenario.

Every proposed action must include its target and scope, current value, requested value,
reason, permission, supported version, restart impact, verification and restore behavior.
Show an explicit no-change result when the machine already satisfies the plan. A review of a
concrete action list replaces an opaque promise to “optimize”.

### PERF-03: Prepare streaming in one flow

Translate S12 into an ordered SOP for the selected route, not a fixed script for every headset.

1. Inspect required software, device/connection prerequisites and current runtime.
2. Present missing setup, proposed settings and optional applications before execution.
3. Reuse already running components; launch missing components through supported interfaces in
   their dependency order. Do not start SteamVR on a route that does not need it.
4. Verify each launch and hand off headset pairing/calibration or unavailable controls to the
   upstream UI. Canceling the guide must preserve independently running software.
5. Offer the existing play launch only after its prerequisites are met. Separate “software
   ready” from a player-verified image, tracking, audio and microphone connection.

Acceptance includes repeat use without duplicate launches, no headset, an incomplete setup,
failed launch and reconnect. Streaming preparation does not silently enable OBS, install a
different runtime, or apply a global “performance” preset.

### PERF-04: Start and stop a bounded play profile

S13/S14 express a session workflow. Before enabling it, identify every actual operation and
create a restore record. Potential actions such as selecting a power plan, applying a supported
per-app setting, or asking the player to close a chosen background app must qualify separately;
the screenshot supplies no evidence that any particular one belongs in the profile.

The enabled state means the selected steps were applied and checked. Starting VRChat alone
does not enable it. Distinguish requested, applying, applied, partial, failed and restore-needed.
Repeated enable/disable requests are idempotent for the same task and target state.

“Off” stops session-scoped behavior and offers the matching restore. If a persistent setting
remains changed, state that fact; do not label the whole profile restored. Do not promise to
free 2.6 GB, flush arbitrary working sets or kill applications to achieve a memory number.
Closing a player-selected application needs its own concrete action and normal close path;
unsaved work and essential tracking, streaming, audio and game dependencies must be accounted
for. No process gets selected for termination from a high count alone.

### PERF-05: Make quick and deeper optimization distinct

| Entry | Proposed behavior | Acceptance |
| --- | --- | --- |
| Quick optimization | A small, reviewed set of supported, readily reversible, route-specific changes that can be verified in the current session | Show every step, skip satisfied/unsupported steps, preserve a restore point and report partial success |
| Deeper optimization | Additional individually selected experiments requiring more evidence, application closure, restart or more disruptive configuration | Explain the difference from the quick plan, preview each change and its rollback, and record restart-pending separately from effective state |

The names are not permission levels. No action becomes safe or authorized merely by being in
one of these groups. Neither group includes blanket registry tweaks, service/driver removal,
security-feature disabling, EAC changes or VRChat injection/hooks/patches. Core-isolation
status in the screenshot establishes a diagnostic need, not a request to weaken security.
Any future security-setting workflow requires separate concrete scope review under the owning
product and permission rules.

SteamVR supersampling and VDXR guidance must describe the selected runtime and the actual
setting location. Changing render resolution trades image quality for load; applying two
runtime presets simultaneously is not a supported shortcut. These candidates stay guided or
unavailable until their version-specific interfaces and restore behavior are verified.

### PERF-06: Diagnose and adjust process priority deliberately

S18 becomes a separate inspect/preview/apply/restore capability. Name the target application
and freshly observed process identity, original priority and proposed priority. Define whether
it affects only this process lifetime or future launches. A restarted process is a new target.

Use only a qualified, bounded priority class; never offer Realtime as a generic improvement.
Do not change EAC, system services, protected processes or unrelated user applications. An
access-denied or exited-process result remains a failure with a manual path. The priority
change must be verified and evaluated against frame timing; success setting a value is not
success improving play.

### PERF-07: Support OBS and VR together

Inspect the applicable OBS version/profile, output resolution/frame rate, encoder and available
render/encode lag evidence through a verified upstream interface or guided UI. Keep OBS
rendering lag, encoding lag and network dropped frames distinct from VR frame timing.

For S19, propose individual changes based on the observed bottleneck. Do not assume which
encoder the reference used. Preserve recording profiles and scene collections, and define
what is safe while recording/streaming. Unsupported active-output changes become a user
handoff, not a forced OBS restart.

For S20, explicitly target **OBS output resolution** when presenting a VUA 1080p preset.
Preview the current and resulting width/height and scaling. Preserve aspect ratio; ask for the
choice on non-16:9 output. Do not upscale an already smaller output, change the VR headset or
desktop resolution, or overwrite the canvas and source layout as an incidental side effect.
Keep frame rate, encoder and bitrate unchanged unless they are separate selected steps.
Retain an OBS-specific restore record even though the reference shows no OBS restore button.

### PERF-08: Restore the changes VUA actually owns

Translate S21/S22 into restore by run and category: play profile, VR settings, streaming and
OBS where applicable. Restoration is to the recorded pre-change values, not a hardcoded
“Windows default”. A backup has to exist before the corresponding mutation is admitted.

- Record a local run ID, SOP/version, target identity, original value or absence, applied value,
  backup integrity and per-step verification. Store only the data needed for that operation.
- Before restoring, compare the current value with the value VUA wrote. Later user/upstream
  edits are conflicts to inspect; do not overwrite them with a whole-file backup.
- Recheck each restored field, including restart-required settings. Distinguish full, partial,
  failed, no-op and unavailable restoration.
- If interrupted, preserve the journal and surface inspection on return under the existing
  recovery rules. Never rerun an optimizer automatically after restart.

The reference says restore asks twice; that is an observed affordance. VUA should use its
existing concrete review and permission rules rather than requiring two identical dialogs.
Warn about the actual effects and make the action reviewable.

### PERF-09: Report work and measured effects

| Surface from the reference | Required VUA behavior |
| --- | --- |
| Status health check | Read-only checks with evidence, passed/failed/unknown/unsupported outcomes, cause and next step; no automatic fixing |
| Live log | Show structured step, time, target, progress, success/failure, restart requirement and recovery action; ordinary players get short explanations, with technical output expandable |
| Clear log | Clear the visible session log without canceling tasks or deleting restore records. Deleting retained diagnostic files is a separate, explicit action |
| Overall status | Distinguish idle, checking, awaiting review, applying, waiting for upstream/player, partial, failed and restore-needed. “Ready” does not imply an FPS gain |
| Optimization report | Summarize the baseline, selected changes, before/after checks, benchmark method, actual results, limitations and restore entries. A settings-only run says performance was not measured |
| Tool directory | Open a known local VUA-owned resource/report location; do not turn a folder shortcut into a script-execution menu |
| Help | Explain each setting, applicability, tradeoff and recovery, with source and last verification. Link to the owning Help context |

All controls remain keyboard accessible; status is conveyed by text as well as color. Follow
the existing [design standard](../design/design-standard.md), UI languages and resource-saving
preferences. Opening or expanding a page never starts a benchmark or modifies settings.

### PERF-10: Use the same observations and approvals through MCP

The app and the player's agent share the same normalized facts, plan, task and verification.
An agent can inspect and prepare a change, then request the same concrete VUA approval. A
generic “make it faster” request does not confer arbitrary process or system-setting authority.
Do not send full process inventories, raw logs or device identifiers in default tool results.
Follow the existing [AI tools](../protocols/ai-tools-v0.1.md) boundary.

## Verification and candidate Eval scenarios

Functional success and performance benefit require different evidence. A registry value or a
lower memory reading verifies neither smoother frames nor a usable headset connection.

For an actual performance comparison, record the Windows/GPU driver/VRChat/runtime/OBS versions,
headset refresh rate, rendering resolution, codec/bitrate where relevant, world and Avatar
conditions, mirrors, population and recording settings. Warm up, run at least three comparable
60-second baseline samples and three changed samples, then recheck restoration. Keep changing
room conditions and unavailable measurements explicit. Record median and p95/p99 frame time,
missed/reprojected frames where the runtime exposes them, streaming latency/drop evidence and
OBS render/encode lag separately. Set the case's useful-improvement criterion before comparing;
there is no promised universal FPS percentage. Measurements come from supported tools or a
player-recorded upstream display, never client injection.

These are proposed cases for the [Eval owner](../eval/README.md), not added `ready` cases:

| Candidate player request | Starting state | Observable pass condition | Verification / N stage |
| --- | --- | --- | --- |
| “Prepare headset play again” | A recorded supported streaming route with some components already running | Only missing components start; player confirms image, tracking and audio; repeat use does not duplicate launches | Closed-loop software checks + guided hardware; N1/N2 |
| “Play is uneven” | Reproducible play load and baseline capture | VUA identifies the evidence and offers a bounded plan; measured comparison meets that case's agreed criterion or honestly reports no benefit | Measured + guided; N1 |
| “Record without making VR choppy” | OBS profile and repeatable capture workload | The selected output change applies to the intended profile; VR and OBS evidence is compared; scenes and aspect ratio remain correct | Closed-loop settings + measured capture; N1/N2 |
| “Undo that optimization” | A saved run, including a later user edit to one field | Owned unchanged fields restore; the edited field is surfaced as a conflict; no silent overwrite | Closed loop; N6 |
| “The optimizer stopped halfway” | One denied/failed operation after an earlier successful change | Partial results and usable restore persist across restart; no implicit replay | Closed loop; N6 |

Required negative cases include unsupported hardware/runtime, unreadable state, lack of
permission, rejected elevation, active OBS output, an exited or reused process ID, corrupt or
missing backups, manual configuration drift, low disk space and cancellation between steps.
A person must review the actual UI and perform the applicable headset/capture scenarios;
automated checks alone cannot accept them.

## Delivery proposal and remaining evidence

1. **Read-only first:** setup facts, health check, setting locations, explanation and report.
   This makes the reference useful without inventing its scripts.
2. **One usable route:** streaming preparation and one supported, reversible setting change,
   including concrete review, task feedback, verification and restore.
3. **OBS and bounded session actions:** qualify each interface and active-session restriction,
   then add the applicable restore coverage and real comparison.
4. **Further experiments:** deeper VR/runtime/priority changes only after their exact settings,
   sources and evidence are recorded. Schedule against N1/N2/N6 without displacing existing
   acceptance by implication.

The first tool's scripts, optimization report and usage guide are still needed to determine
what its quick/deep/acceleration buttons actually do. Also unresolved are the exact meaning of
`游戏态`, process counting, OBS 1080p target, original backup coverage and the claimed memory
relief. No original action or performance claim is verified by this screenshot-only review.

## Primary references checked on 2026-10-26

- [Microsoft: Memory integrity](https://learn.microsoft.com/en-us/windows/security/hardware-security/enable-virtualization-based-protection-of-code-integrity)
  describes it as a Windows security protection and documents compatibility considerations.
  Its presence is not an instruction to disable it for gaming.
- [OBS: Encoding performance troubleshooting](https://obsproject.com/kb/encoding-performance-troubleshooting)
  distinguishes resource contention and discusses output-resolution/frame-rate reduction.
  These are candidates to diagnose, not evidence of the reference tool's implementation.
- [VRChat Creator Guidelines](https://hello.vrchat.com/creator-guidelines) and the repository's
  adopted product boundary remain the interaction review inputs; an external optimizer's
  existence does not establish approval for its techniques.
