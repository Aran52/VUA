# Face tracking adaptation

> Updated: 2026-10-21
> Status: Design and first implementation scope; real-material acceptance pending
> Research: [October 2026 review](../research/face-tracking-2026-10.md)

## Outcome and ownership

Add FT to an already assembled line while preserving its outfits and original Warehouse files.
Report what was observed and what still needs verification. Claude owns the Workshop add-on
station and current plan. This slice owns the engine, Bridge, contracts, package review and
these SOP drafts. It changes no device module, OSC receiver, account or upload authority.

The first implementation has two parts: FT facts from the active owned Avatar, and a protected
interactive session for an Editor menu supplied by one of the line's imported materials.
Existing successful runs remain valid when the new facts are absent.

This first route requires the add-on to be among the line's already prepared sources, normally
selected as a dependency alongside the Avatar before assembly. It does not add a new material
to an assembled project without rebuilding. Adding that intake path, deterministic template
installation, generated shape transfer, final built-controller analysis and device configuration
remain follow-ups. The desktop station consumes the new ports; this slice adds no renderer UI.

## Facts and adapter boundary

Use one workflow: observe → select a route → review exact inputs/target → confirm → protect →
apply or hand off → re-inspect → verify in VRChat.

An adapter describes a stable ID/version, supported inputs and dependencies, target identity,
execution mode, mutation scope, completion criterion, verification evidence and replay policy.
These are product-owned capabilities; a downloaded file cannot grant itself permissions.

| Adapter family | Apply action | Completion / replay policy |
| --- | --- | --- |
| `editor_menu` (first slice) | Select an observed menu from a prepared source; run in a dedicated interactive Unity process | Player finishes the patch and confirms completion in Unity; independent reinspection follows. Opaque edits require manual replay. |
| `template_prefab` (follow-up) | Add a pinned, reviewed MA/VRCFury prefab with an explicit renderer/shape mapping | Verify the owned instance and built controller/parameter result. Replay exact prefab content and settings after assembly. |
| `existing_shapes` | Read shape and animation facts, then choose an appropriate template | Detection alone makes no changes; a visual deformation review still applies. |
| `shape_generation` (follow-up) | Reviewed generator on copied mesh data | Per-tool topology/version constraints and output checks; retain inputs/settings only, never generated paid geometry in a Recipe. |
| `external_patch` | Player follows the creator's supported external workflow | No arbitrary exe/arguments tool. Reimport the resulting owned material or await a reviewed adapter. |
| `native_eyes` / future `native_face` | Version-specific descriptor configuration | Keep these separate from expression-parameter templates; admit native face only after SDK and client verification. |

Inspection reads each SkinnedMeshRenderer separately, reports exact case-sensitive matches to
named reference sets, and counts FT-like controller/expression parameters and known integration
components. A reference set is a detection vocabulary, not a quality score or compulsory set of
shapes. No renderer is renamed. The descriptor's sync cost is source evidence; final NDMF/SDK
cost remains unverified. Zero recognized parameters never means “FT impossible”.

Facts name their observation time and `unity_source` evidence layer. Runtime movement stays
unknown. MA or VRCFury components are integration hints, not proof of FT. A partial/truncated
observation must say so. Missing meshes, scripts or unsupported layouts remain explicit.

Patch menu discovery reads MenuItem metadata only from scripts among the prepared source's
asset paths. It does not invoke menus, validators or patcher methods. Built-in commands,
context menus, ambiguous menu identities and arbitrary caller-supplied methods are excluded.
Menu selection is bound to its source and script as well as the project/observation digest.
The list is a set of possible creator entry points, not a declaration that every listed menu
performs FT.

## Confirmation and execution

| Permission mode | Where it stops |
| --- | --- |
| Ask before changes | Show source, exact menu, work-project target, recovery, trust and replay limits; wait for the player's per-action confirmation. |
| Decide on its own | May recommend a route and prepare the same review. Third-party code execution still waits for the player. |
| Fully delegated to VUA | May automate only later reviewed deterministic adapters within granted scope. The first opaque Editor-menu route still requires the same explicit confirmation. |

The command accepts a reviewed digest and an explicit confirmation of the imported code. It
does not interpret an AI request, a material label or a standing permission as that receipt.
The current MCP read tools see the facts through the run; no autonomous patch execution tool is
added by this slice.

The work project must be closed before the session starts, so the restore point is consistent.
VUA records a version, takes the existing full-copy restore point and launches its own supervised
interactive Unity process. It selects the active owned Avatar and opens the chosen menu once.
The player performs the creator's steps and adds the FT prefab to that same Avatar if necessary.
The dedicated session offers **Finish and check** and **Cancel and restore**. Completion saves
the managed scene, closes the session and lets the engine re-inspect in batch mode.

This avoids running a window-based patcher in batch mode or restoring files under a live Editor.
An already open ordinary Editor must be closed first; it is neither adopted nor killed.
Cancellation, timeout, missing receipt and reinspection failure restore the snapshot after the
supervised process exits. Domain reload must not invoke the menu twice. Restarted VUA does not
resume a patch. Existing interrupted-change recovery remains authoritative.
If the process supervisor cannot verify that creator child processes have ended, automatic
restoration is deferred and the snapshot is retained. The player must close the creator helper
before reviewing a manual restore; replacing files under a possible writer is not recovery.
The session uses the existing 20-minute Unity process budget. A longer creator workflow needs
a separately reviewed limit; reaching this limit cancels and restores rather than accepting
an incomplete patch.

The Bridge refuses the menu operation in batch mode. Batch mode remains available for
inspection and the final recheck. A future synchronous patch adapter needs a reviewed version,
explicit inputs, no-dialog completion, deterministic output and postconditions before it can
be marked batch-capable.

The transaction covers the managed project's assets, packages and settings. VUA never writes
the Warehouse original. Imported Unity scripts already have normal Editor process privileges;
the player must trust their source. This is not an OS sandbox and cannot reverse external
side effects. Do not accept external original-FBX paths in the command.

## Versions, restoration and Recipes

The first route cannot honestly reproduce arbitrary creator dialogs or arbitrary mesh edits.
Successful completion records a local manual-replay marker in the snapshotted work-project
scope, with the selected menu and input identity. Reinspection reads it; a full snapshot rollback
restores its previous state together with the project.
It lives in `ProjectSettings/VUA.FaceTracking.json`, so the AssetDatabase does not import or
lock this host/Bridge metadata as a model asset.

Version history records the protected FT change. A version whose state contains opaque FT
changes, or a replay that would cross such a change, must report manual replay required.
Likewise, saving a Recipe of such a line must fail with a named explanation rather than silently
omit FT. The existing Recipe format remains unchanged in this slice. The latest full-copy
rollback remains usable. Explain this limit in the add-on station before confirmation.
Only the existing latest full-copy restore point is retained. A later protected edit can
replace the pre-FT restore point; version records do not turn opaque edits into replayable
geometry or create a permanent uninstall copy.

Follow-up: add a versioned Recipe adjustment only for a reviewed deterministic adapter. Record
adapter version, original material content IDs, exact dependency pins, mapping/settings and
postconditions. Reproduction resolves the receiver's own inputs, assembles, applies FT and
checks again; unavailable tools or mismatched hashes stop with a named prerequisite. Imported
Recipes never authorize scripts. Removing a patch requires rebuilding from pristine copied
inputs when it changed an FBX; rebuilding only the scene cannot undo that patch.

## Proposed replacement for current-plan item 10

Keep Meiyun/Pico 4 Pro as the first real acceptance case, and describe the capability as **FT
adaptation on a completed line**:

1. Observe geometry, parameters, wiring, dependencies and their evidence layer.
2. Choose among creator patch, reusable template, existing shapes, generation and native eyes;
   keep native full-face tracking gated by verified SDK support.
3. Apply through a reviewed adapter or a confirmed interactive handoff with a restore point.
4. Recheck the owned Avatar and built parameter cost, then verify actual movement in VRChat.
5. Record exact replay support and block incomplete Recipe reproduction.

Item 8 owns package delivery; item 9 remains material tuning; item 33 owns the final built
256-bit budget display; item 42 owns version/replay behavior. This document does not edit
`current-plan.md` or declare those items accepted.

## Decisions for the author

- **First UI scope:** recommend the protected interactive creator-patch route plus facts,
  followed by a pinned MA template adapter. It serves the owned Meiyun material while avoiding
  an unverified generic “one-click FT” promise.
- **Opaque replay:** recommend the explicit Recipe/revision block above until the Meiyun
  patcher's inputs and outputs are verified. A descriptive record cannot reproduce a mesh.
- **Native SDK tracking:** recommend a separate follow-up after stable SDK support and a tested
  migration, retaining OSC templates for existing Avatars.
- **Meiyun identity:** confirm the exact package/base version, menu and required follow-up prefab
  steps in dev VUA. If the creator only supports a new root, choose a reviewed transplant
  adapter before claiming support for an already dressed line.

No purchase is requested. UI decisions and real-device acceptance remain with the author.

## SOP draft: Add face tracking to an Avatar

| Field | Value |
| --- | --- |
| ID / version | `amf.add-face-tracking` / `0.1.0-draft` |
| Applies to | VUA-managed completed PC Avatar line; Unity 2022.3.22f1/f1c1; exact recorded SDK/MA versions; VRCFT v5 OSC route; first physical device Pico 4 Pro; all regions |
| Prerequisites | Lawfully owned base/add-on; compatible original version; trusted Editor scripts; closed work project; enough space for a restore point; device/module setup separately complete |
| Last verification | Sources reviewed 2026-10-20; software verification 2026-10-21 on Unity 2022.3.22f1c1, SDK Base/Avatars 3.10.5, MA 1.18.7, NDMF 1.14.8, using generated triangle meshes and a synthetic menu. Meiyun and hardware not yet verified. |
| Sources | [Research](../research/face-tracking-2026-10.md), creator's exact instructions, [VRCFT PICO guide](https://docs.vrcft.io/docs/hardware/vr/pico/pico4pe) |

| Step / actor | Action | Expected result | Verification | On failure |
| --- | --- | --- | --- | --- |
| 1 / VUA | Inspect the completed line | FT geometry, wiring and source budget facts | Check evidence time, target and partial flags | Reinspect; unknown remains unknown |
| 2 / Player, VUA opens guide | Choose the creator route or supported template and review exact requirements | Matching original version and dependencies | Compare material identity and installed pins | Obtain the correct owned files through the creator |
| 3 / Player | Confirm the menu, target, trust and replay limits | One concrete approved action | Digest still matches | Recreate review after drift |
| 4 / VUA | Record version and snapshot; start the dedicated patch session | Creator menu opens on the active Avatar | Bound session/command receipt | Restore and report the failed step |
| 5 / Player | Complete creator dialogs and prefab/mesh steps on the same Avatar; Finish and check | Saved result, session closed | Confirm target and completion; VUA re-inspects | Cancel and restore; do not select library originals |
| 6 / VUA, Player | Inspect, then check through the SDK | Geometry/wiring present; final budget checked | SDK/NDMF evidence and a visual expression check | Fix the named defect or restore |
| 7 / Player | Upload through the official SDK; launch VRCFT from Tools and test in VRChat | Eye, blink and mouth motion; intended gesture/viseme coexistence | Mirror and, when possible, a remote viewer; reconnect test | Use the troubleshooting SOP |

## SOP draft: Face tracking does not move

| Field | Value |
| --- | --- |
| ID / version | `amf.face-tracking-not-moving` / `0.1.0-draft` |
| Applies to | Same first PC/Pico 4 Pro route; distinguish native eyes from expression-parameter FT |
| Prerequisites | A selected Avatar, known tracking route and access to the upstream UI |
| Last verification | 2026-10-20: source review; physical run pending |
| Sources | [Research](../research/face-tracking-2026-10.md), [VRCFT parameters](https://docs.vrcft.io/docs/tutorial-avatars/tutorial-avatars-extras/parameters), creator's troubleshooting instructions |

| Step / actor | Action | Expected result | Verification | On failure |
| --- | --- | --- | --- | --- |
| 1 / Player | Identify whether eyes only, mouth only or the whole face fails | Specific symptom | Compare blink, gaze, jaw and smile separately | Record unknown channels; do not replace everything |
| 2 / Player, VUA opens guide | Check headset permissions and live module input | Fresh values change with the face | Upstream data view, not “initialized” alone | Check sender/version/connection and competing modules; leave hidden settings to upstream guidance |
| 3 / Player | Check VRChat OSC and the currently worn uploaded Avatar | Intended OSC route reaches the intended Avatar | Upstream output and VRChat behavior | Follow upstream reconnect/configuration guidance; no automatic broad cache deletion |
| 4 / VUA | Inspect the saved line | Correct mesh shapes, controller/parameter and integration evidence | Per-mesh matches and exact names; identify source versus built facts | Correct path/mapping or obtain required shapes |
| 5 / Player | Check FT toggles, gesture/idle layers, blink/viseme conflicts and built sync cost | The intended layer owns each motion | SDK/NDMF result and in-game expressions | Make one reviewed correction with a restore point |
| 6 / Player | Repeat with microphone, USB/Wi-Fi reconnect and a new session | Tracking continues or recovers | Record software/device versions and result locally | Keep the failure and restore known-good state; report sanitized findings |

## Author verification in dev VUA

- Include the owned FT add-on among the line's prepared dependencies and confirm its menu is
  discoverable. A DLL-only tool, a class MonoScript cannot resolve, or a reserved/context menu
  requires a reviewed adapter; do not enter a free-form method or executable instead.
- Run Meiyun's owned FT add-on on its completed line. The next check must show FT evidence,
  and with VRCFT started, **expressions must move in VRChat on Pico 4 Pro**.
- Confirm exact menu/window behavior, original FBX requirement, any executable/dialog and how
  the patched mesh/prefab attaches to the existing dressed Avatar.
- Verify cancel, patch error, closed session, domain reload and recheck failure restore the
  previous line; the Warehouse original's hash must remain unchanged.
- Verify gestures, blinking, eye direction, mouth/visemes and tongue capabilities that the
  actual hardware/add-on supplies. Check final parameter budget and a remote viewer if available.
- Verify USB and Wi-Fi, microphone coexistence, disconnect/reconnect and the next session.
- Verify Claude's station displays absent/partial facts, source-only budget, trust/replay limits
  and the pending physical check accurately.

## Reproducing software verification

Use a disposable local project with Unity **2022.3.22f1c1**, VRChat Base/Avatars **3.10.5**,
Modular Avatar **1.18.7**, NDMF **1.14.8** and Unity Test Framework **1.1.33**. These dependencies
were obtained from their official feeds, with package archive SHA-256 checks; dependency setup
requires the usual official services. No Avatar package or paid content is needed.

**Observed on 2026-10-21:** C# compilation and the generated-mesh/SDK-parameter checks passed.
Two consecutive interactive sessions completed successfully, each dispatching the menu once
across a domain reload and returning a manual-replay receipt. Replacing the existing metadata
marker also passed. Cancellation after reload exited without a success receipt. The actual
Unity FT receipt passed the TypeScript boundary validator. The synthetic meshes deliberately
have no display materials; their missing-material warning is expected and does not represent
a render-quality or SDK upload pass.

1. Copy the current Bridge package to the disposable project's `Packages/com.ph-r.vua`.
   Only in that copy, put `unity/Tests/Editor/AmfFaceTrackingSynthetic.cs` and
   `AmfFaceTrackingSessionSynthetic.cs` in the package's `Editor/Amf` directory. Put
   `FtSyntheticMenu.cs` in the project's `Assets/Editor`. These harness files are not staged
   into production projects by the Bridge scaffolder.
2. Start Unity in batch mode with `-executeMethod
   Vua.Editor.Bridge.AmfProductionEntryPoint.RunFaceTrackingSyntheticTests`. It generates two
   triangle meshes and a real SDK expression-parameter asset. Expected: exit 0 and
   `VUA_FT_SYNTHETIC_CHECKS_PASSED`, 52 exact ARKit names on the first mesh, no union across
   meshes, four recognized parameter names (including an MA Merge Animator controller), and
   17 source sync bits (1 + 8 + 8; local Float excluded). Inspection must not invoke the
   synthetic patch or its validator. A batch patch
   request must be refused before dispatch.
3. Run `PrepareFaceTrackingSessionSynthetic` through the same fully qualified class in batch
   mode. It creates a synthetic owned prefab, saved scene and digest-bound command under
   `.vua/bridge/ft-synthetic.request.json`.
4. Start Unity on that project **without** `-batchmode` or `-quit`, using the production
   `Vua.Editor.Bridge.AmfProductionEntryPoint.Run` entry point, `-vuaRequest` pointing to that
   command and `-vuaResult` pointing to `.vua/bridge/ft-synthetic.result.json`. The synthetic
   menu requests a domain reload; the harness simulates the player's completion after reload.
   Expected: one menu invocation, a successful receipt, manual replay required, and editor exit.
   A fresh disposable project is the simplest way to repeat the whole sequence.
5. To test cancellation, set the fixture's `ft-interactive-mode.txt` to `cancel_after_reload`
   after preparation, clear only the preceding fixture invocation/result files, and launch the
   same command. Expected: one dispatch, exit 1, and no successful result file. The harness
   calls the same cancellation method as the session button.

The harness makes no network requests and consumes only its generated shapes/menu. Compiler
logs and SDK packages stay local. Rust tests independently exercise confirmation, stale review,
open-editor refusal, missing completion, target change, cancellation, reinspection failure,
restoration and Recipe/revision restrictions. These are software checks, not evidence that
the Meiyun patch is compatible or that Pico tracking moves the Avatar.
