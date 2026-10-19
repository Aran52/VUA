# Current development plan

> Updated: 2026-10-19

This plan orders the next slices toward the [north star](../north-star.md). Each slice names the
[Eval cases](../eval/cases.md) it moves and what counts as done. The
[N sequence](../development-outline.md) keeps the acceptance rows; this plan sets the order.

## Where things stand

- `main` carries the Ibis releases (v0.6.7).
- `slice/north-star` (VUA-Project/VUA#83) adds the north star, the Eval and the documentation rules.
- `slice/n3-n4-ma-production` (VUA-Project/VUA#84) is the production branch of this stage. It
  carries the AMF production work, the MSIX Hub detection (#85), and the slices merged on the
  Aran52 fork:
  - from the first real runs (Aran52/VUA#1–#4): the material-first production path with Recipe
    v0.4, the local MCP adapter, six Steam tool connections, Unity Hub starting before Unity
    work, one shared product name for work projects so the SDK sign-in carries over, lilToon
    from its feed, result and candidate previews, and live Unity progress with Stop and Continue;
  - the AMF interface as an Avatar factory: a base map with animated rooms and a camera, the
    workshop's production lines with stations and permission modes, the control center with the
    Recipe graph and a VCC-style package manager, and the release room's album with the
    60-frame turntable and managed upload; token groundwork for the [skin system](skin-system.md);
  - on 2026-10-17/18 (Aran52/VUA#5–#20): the [experimental VRChat sign-in](vrchat-account.md)
    and the account's Avatar list, the SDK's own checks with its Auto Fix and plain-word
    messages, cached project fingerprints, permission modes and stated recommendations, the
    work queue, workshop stations with in-place material changes and line deletion, production
    started from the materials room, the quit prompt, restore after an interrupted assembly, and
    production records in Settings;
  - on 2026-10-19 (Aran52/VUA#21–#23): the known-packages table, which installs lilToon,
    Poiyomi Toon by version line and UnlitWF from their feeds and names the official page for
    packages players get themselves; a Bridge v5 result schema that covers every field Unity
    writes; and Unity work on different lines side by side, up to a capacity of three. The slice
    rows below link each pull request.
- VUA-Project/VUA#86 keeps notification rows in place while downloads progress.
- Real runs on 2026-10-16/17, with evidence in `_local_eval/`:
  - P-02 (Meiyun with the T-shirt & shorts) assembled with correct shading (lilToon 2.3.4); its
    Recipe "Meiyun T恤短裤" is saved; the result uploaded through the SDK as the private Avatar
    "AMF test1" (`avtr_7ce0e209…`), so the SDK build passed;
  - P-01 passed: VUA installed lilToon for the outfit's lilToon materials and found no material
    without its shader;
  - with Unity Hub quit, VUA started Hub and Unity received its license;
  - the turntable bake, Stop (Unity's whole process tree ended in 12 s) and Continue worked.
- The author's review found texture memory and lilToon brightness limits to tune, the SDK panel
  hard for newcomers, and the SDK sign-in asked again after the move to the shared product name.
- A real-machine round through the MCP adapter on 2026-10-19, with evidence in
  `_local_eval/2026-10-19-mcp-real-run.md`: an external agent drove VUA and the author approved each
  request. P-02 and P-01 on new lines, queueing and cancelling a waiting task, stop and continue, a
  material change, deleting a line and a second managed upload ("AMF test2") passed. Opening the
  project for the SDK timed out waiting for the SDK panel, and a restore after a material change
  and a new assembly returned to the later state. Slices 42–44 and items in 5 and 6 take the
  findings.
- Release: on 2026-10-19 the author postponed the next release, planned for the end of October,
  for a stage pause and a real-machine test round through the MCP adapter; it is numbered under
  the [version policy](../release/versioning.md) when it is cut.

## Order

| # | Slice | Eval | State |
| --- | --- | --- | --- |
| 1 | Integrate | — | next: the author merges #83, #84 and #86 with the organization account |
| 2 | Finish the first real production runs | P-02, P-01 | done: P-01 passed; P-02 assembled, shaded correctly and built and uploaded through the SDK |
| 3 | Unity Hub and SDK sign-in | P-02 | Hub start verified; with slice 19 the SDK starts signed in from VUA's session; the shared SDK sign-in awaits a check |
| 4 | Result previews | P-02 | done (Aran52/VUA#1) |
| 5 | Unity editor handoff | P-02 | in progress: item 3 is in (Aran52/VUA#36), the open waits through a slow first import and names the SDK panel menu; items 1–2 remain |
| 6 | SDK step for newcomers | P-02 | in progress: the sign-in guide, the SDK's own checks (Aran52/VUA#6), one-click fixes and plain-word messages (Aran52/VUA#12) are in; the build and upload guide and the SDK panel's own Auto Fix in the facts (item 4) remain |
| 7 | Permission modes and simpler choices | P-02, P-03 | permission modes (Aran52/VUA#11) and stated recommendations (Aran52/VUA#16) in the workshop and `vua_amf_plan`; the image confirmation remains |
| 8 | Known packages | P-01, P-03 | in progress: the known-packages table with detection and installs is in (Aran52/VUA#21); the package page by permission mode (items 3–4) and two follow-ups remain |
| 9 | Material tuning | P-02, P-03 | in progress: item 2's engine, Bridge facts and contracts (Aran52/VUA#29); its workshop entry and item 1 remain |
| 10 | Face tracking add-on | P-03 | |
| 11 | Unity sign-in and license | — | in progress: the detection and check engine and contracts (Aran52/VUA#28); the scenario step remains |
| 12 | Managed upload | P-02 | done: first real upload on 2026-10-17 (Aran52/VUA#4), a second through the MCP round on 2026-10-19 |
| 13 | Appearance and one-click styles | — | architecture and tokens ready; needs a design session |
| 14 | Player language and four-language review | P-02 | |
| 15 | Versioned SOPs | A-03, B-02–B-05 | |
| 16 | Eval runs | all `ready` cases | |
| 17 | Complex production and Recipe reproduction | P-03, P-04 | |
| 18 | Tool and package delivery by license | C-01, C-02 | |
| 19 | Experimental VRChat sign-in | P-02 | merged (Aran52/VUA#5); the upload check reads VRChat's real list shape since slice 26; awaits the real-account checks |
| 20 | Start production from the materials room | P-02 | built (Aran52/VUA#19); awaits the author's real-run check |
| 21 | Faster project fingerprints | — | done (Aran52/VUA#7) |
| 22 | Motion setting | — | needs the author's decision |
| 23 | Base map art | — | with slice 13 |
| 24 | Finer usual durations | — | when the estimates prove too rough |
| 25 | Production records in Settings | — | done (Aran52/VUA#13) |
| 26 | VRChat Avatar list in the release room | P-02 | built (Aran52/VUA#17); awaits the author's real-account check |
| 27 | Queue factory work | P-02 | built (Aran52/VUA#18): one job at a time, the rest wait in a visible queue; queueing and cancelling verified in the 2026-10-19 MCP round. Item 2: the engine (Aran52/VUA#23) and the workshop setting are in; a run with two lines side by side remains |
| 28 | Quitting during a task | — | done (Aran52/VUA#14) |
| 29 | Restore after an interrupted assembly | — | done (Aran52/VUA#15) |
| 30 | Workshop stations and line management | P-02 | built (Aran52/VUA#20); the material change and deletion verified in the 2026-10-19 MCP round, whose restore found slice 42's gap; removing an outfit through the plan remains |
| 31 | Change a finished line's base Avatar | P-02 | built (Aran52/VUA#27); awaits the author's real-run check |
| 32 | Agent interface refinements | P-02, P-03 | planned from the [reference study](../research/reference-projects-2026-10-19.md) |
| 33 | Switch options and built-result evidence | P-02, P-03 | planned from the reference study |
| 34 | Evidence layers and toolchain profile | P-03, B-02 | planned from the reference study |
| 35 | Start card, preflight and attribution | P-02, P-03 | planned from the reference study |
| 36 | Licensed entry guide | — | design drafted; awaits the author's network verification |
| 37 | Creator credits | P-02 | planned |
| 38 | Steadier regional latency | — | planned; calibration needs the author's in-game Ping readings |
| 39 | Resource panel that guides | P-02 | planned; items 1 and 2 need the author's decision |
| 40 | Notification center redesign | P-02 | planned; a preview for the author comes first |
| 41 | VN3 terms and filtering by use | P-02 | backend prompt written; to be assigned |
| 42 | Labeled restore points | P-02 | planned; how many points a line keeps is the author's call (three proposed) |
| 43 | Fixes from the MCP round | P-02 | built (Aran52/VUA#32): items 1, 3, 4 and 5, and item 2's banner; the transient failure behind it, the stale progress line and the duplicate card (item 6) need a reproduction |
| 44 | MCP payloads and identity | P-02 | planned |

Suggested next order: 43 Fixes from the MCP round, 42 Labeled restore points, 1 Integrate, the
rest of 8, the rest of 6 (build and upload guide), 5 Editor handoff, the rest of 7 (image
confirmation). 43 and 42 remove the traps the MCP round met; integration keeps the production
branch close to `main`; 6 and 8 close the largest gaps a newcomer meets in P-02 and P-01.

Every slice ships its player-facing text in the four UI languages; slice 14 reviews them together
with the author.

## 1 — Integrate

1. Merge #83, then #84 (which now carries #85 and Aran52/VUA#1–#4), into `main`; merge #86.
2. Keep CI green on each pull request.

**Done when** all of them are on `main`.

## 2 — Finish the first real production runs

1. P-02 with lilToon from VPM: shading confirmed by the author. Save the result as a Recipe.
2. Run the missing-shader case (P-01) on a project whose material needs a shader it lacks.

**Done when** P-02 and P-01 pass as closed-loop runs in the app, with evidence in `_local_eval/`.

## 3 — Unity Hub and SDK sign-in

Both removed friction in the first real run, so they come before the previews (author decision,
2026-10-16).

1. VUA starts Unity Hub before Unity work and waits for its licensing client, so a license held
   by the packaged Hub is available.
2. All VUA work projects share one company and product name, so the SDK sign-in, which Unity
   keeps per product name under `LocalLow`, carries over between them. Existing work projects
   take the shared name at their next Bridge run.
3. VUA reports whether the SDK is signed in as yes or no through the Bridge; the sign-in itself
   stays inside the SDK.

**Done when** a run with Hub closed reaches a licensed Unity, and a second work project opens
signed in to the SDK.

## 4 — Result previews

1. The Bridge renders the finished Avatar from the front, side and back under neutral lighting,
   and one image per Avatar and outfit candidate during inspection, using the existing preview
   renderer. Images stay in the work project.
2. The Avatar page shows them; the agent reads them to check a result before asking the author.
   The MCP adapter returns them as image content, as the author approved on 2026-10-16.

**Done when** a P-02 result can be judged from its previews, and each candidate has an image.

## 5 — Unity editor handoff

1. The Bridge inside an open Editor accepts "save and close": save scenes and assets, quit, and
   send a receipt.
2. When work needs a project an Editor holds, the request card offers "Save and close Unity,
   then continue". After the look check, the page offers "Done checking, close Unity".
3. Opening the project for the SDK succeeds once Unity holds the project with the Avatar
   selected; when the SDK panel does not open by itself, the page says how to open it (VRChat SDK
   → Show Control Panel). In the MCP round the panel stayed closed on this machine, and the open
   task failed with `sdk_handoff_timeout`.

Item 3 is in (Aran52/VUA#36, [amf-production 0.23.0](../protocols/amf-production-v0.1.md)). The MCP
round's Unity spent 82 seconds on the project's first import, and its receipt (Avatar selected, panel
requested) arrived 30 seconds after the 180-second wait ended. The open now waits up to 10 minutes,
as a managed upload does, succeeds once the Avatar is selected, and the workshop names VRChat SDK >
Show Control Panel while Unity opens and after.

**Done when** a held project continues without closing Unity by hand.

## 6 — SDK step for newcomers

1. The Bridge reads the official SDK checks and performance statistics for the result.
2. VUA shows each item in plain words as "must fix before upload", "reminder, upload still
   works" or "affects the performance rank", with a one-click fix where VUA can make it.
3. An illustrated guide to the SDK panel in four languages: sign-in, build, warnings, upload.
4. An Auto Fix the player makes in the SDK panel during an upload reaches the line's facts: VUA
   reads the SDK's check again after the upload, so the release page and `vua_amf_run` stop showing
   the fixed error.

Done so far: the sign-in guide on the release page, and the SDK's own checks. The Bridge runs
the SDK panel's Avatar check without opening the panel; the release page groups its items as
"fix before uploading", reminders and notes, shows the performance rank for the build target
with what it means for other players, and lists the categories rated below Good. The SDK's own
Auto Fix runs from the release page behind a restore point, and the most common SDK messages
read in plain words with the original one click away. Remaining: the build and upload guide.

**Eval:** P-02 outside-help count. **Done when** the author uploads the P-02 result guided by
VUA alone.

## 7 — Permission modes and simpler choices

1. The workshop offers three permission modes, like an AI agent's (author ruling 2026-10-18):
   ask before changes (default), decide on its own, and full access, which warns first that it
   may mess up the Avatar. The modes share one layout and differ in where the line stops; they
   describe what the player wants to hand over, not their experience with Unity.
2. VUA recommends the Avatar and outfit prefabs with stated rules (name match with the outfit's
   target, top-level prefab, the creator's base for outfits) and asks only when two candidates
   are equally plausible.
3. The confirmation reads as one sentence with images: BOOTH product images for materials and
   the previews from slice 4 for prefabs; binding details sit under "Adjust".
4. `vua_amf_plan` without a selection returns the recommendation.

**Done when** the author completes P-02 in full access without a confirmation, and in "ask before
changes" confirming each change with every detail adjustable.

## 8 — Known packages

1. A table of known packages: ID, feed, license, delivery policy, detection signals. Shaders
   come first:
   - lilToon (`jp.lilxyzw.liltoon`, MIT), already installed from its feed;
   - Poiyomi Toon, the free version (`https://poiyomi.github.io/vpm/index.json`, MIT per its
     repository); Poiyomi Pro stays with its own EULA;
   - UnlitWF (whiteflare's VPM feed), after its feed URL and redistribution terms are confirmed;
   - VRChat's Toon Standard ships inside the SDK (3.8.1 and later) for PC and mobile Avatars;
   - then VRChat SDK (official source), Modular Avatar with NDMF.
2. Bundled copies are recognized from each package's `legacyFolders` and `legacyFiles`
   (lilToon declares `Assets\lilToon`), generalizing the lilToon rule.
3. The package page follows the permission modes: full access resolves dependencies without
   asking; "decide on its own" shows it with "Next step" and "Handle dependencies for me"; "ask
   before changes" shows every package, version and feed.
4. The package page lists work projects with installed versions, updates and Recipe pins.

Done so far (Aran52/VUA#21): the [known packages](known-packages.md) table, detection by
material marker or shader GUID, installs from the official feed on a line's first preparation
and on a material change, Poiyomi Toon pinned to the material's version line, and the
`amf.package.guide` warning for packages the player gets themselves. Remaining: items 3 and 4,
and two follow-ups from the review:

- A bundled Poiyomi copy leaves for the official package only when it holds a single version
  line. Poiyomi releases also ship their older lines, so real bundled copies mostly import
  unchanged. Recording each release's full set of shader GUIDs would let those copies leave too.
- A preparation-level test that imports or skips a bundled copy's files as decided; the decision
  itself is tested.

**Done when** P-01 resolves its missing shader through a known package.

## 9 — Material tuning

1. Texture memory: measure each texture's memory and the total against VRChat's performance
   thresholds; lower Max Size and switch compression in the work project, reversible by
   snapshot.
2. lilToon brightness limits: align the outfit materials' lower and upper brightness limits with
   the Avatar's, so the outfit and body brighten and darken together across worlds.
3. Both follow the permission modes: applied toward a target (full access), as a reviewed list
   (decide on its own) or per material and texture (ask before changes).
4. Evaluate lilAvatarUtils (MIT) first; integrate it when it covers these steps.

**Done when** the P-02 result reaches a chosen rank, and its outfit and body match in brightness
in bright and dark worlds, without visible loss the author objects to.

## 10 — Face tracking add-on

1. Run the author's Meiyun face tracking add-on with the P-02 result; its patcher runs from an
   Editor menu, which the Bridge calls with the player's confirmation.
2. Record what the add-on needs (packages, menus, parameters) in the known packages and SOPs.
3. Check it with VRCFaceTracking through the tools page.

**Done when** the Meiyun result carries working face tracking, verified in VRChat.

## 11 — Unity sign-in and license

1. Detect the license from the license file (standard and packaged-Hub locations), whether Hub
   and its licensing client run, and a short real check in a VUA-owned project.
2. The scenario step shows the state and the one next action; a production license failure
   opens that step with the run retained.
3. Test the Unity CLI that Hub installs at `%LOCALAPPDATA%\Unity\bin\unity.exe`.

**Done when** a fresh machine reaches a licensed Unity run guided by VUA alone.

## 12 — Managed upload

1. Verify the SDK's build-and-upload interface in SDK 3.10.5, including whether it needs the
   Editor window.
2. The player confirms the content rights statement in VUA for each upload; the VRChat sign-in
   stays in the SDK.

SDK 3.10.5 hands out its builder only while the SDK control panel is open and signed in, and
shows VRChat's ownership agreement in that panel, so the upload runs in the Editor's interface
through the [Bridge v5 handoff](../protocols/unity-bridge-v5.md#managed-upload-in-the-open-editor).
The player's experimental [VRChat sign-in inside VUA](vrchat-account.md) hands its session to
the open Editor before an upload ([vrchat-session v0.1](../protocols/vrchat-session-v0.1.md)),
so the panel starts signed in, and each upload record names where its sign-in came from.

**Done when** the P-02 result uploads from VUA with the player's confirmation.

## 13 — Appearance and one-click styles

1. Design session with the author: which screens, and whether a style may recolor the module
   colors. The first two styles come from the author's briefs; the current purple style stays
   as the default:
   - **Y2K**: aqua and white, bandages, pills, angel wings, flat;
   - **Jirai-kei**: pink and black, lace, energy drinks, leather, dimensional.
2. Move remaining literal values into tokens (colors, radii, shadows, font sizes, timings).
3. A style is token data plus its motifs (patterns, icons, textures), checked against a schema
   and switched with one click in Appearance and the command palette; contrast is checked per
   style in dark and light.
4. Redo the notification panel's size and animation; build the production animation from real
   step events.

**Done when** the author switches styles in one click and approves the redone screens.

## 14 — Player language and four-language review

1. Remove internal references and process vocabulary from UI text; add a four-language
   terminology table and a check for Chinese full-width punctuation.
2. Rewrite the Japanese and Korean strings for the AMF, tour and onboarding pages natively.
3. Review every screen and message added in slices 3–13 in all four languages with the author.

**Done when** the author reviews the AMF journey and the new screens in all four languages.

## 15 — Versioned SOPs

1. Turn the scenario SOPs into versioned SOP documents with the fields in the north star.
2. Write the first symptom-to-solution SOPs and serve them through Help and `vua_sop`.
3. Record each SOP's last verification.

**Eval:** A-03, B-02, B-03, B-04, B-05. **Done when** these cases are `ready` and pass in both
channels.

## 16 — Eval runs

1. Convert the cases into machine-readable files and define the run record.
2. Run every `ready` case in the app and through an MCP agent; publish the two pass rates and
   median times with the next release.

**Done when** a release reports both metrics.

## 17 — Complex production and Recipe reproduction

1. Run the N3 complex case with the author's real materials, then repeat it in a clean project.
2. Reproduce it from a Recipe in a second project, including reapplication and conflicts.
3. Run both through the MCP agent as well.

**Eval:** P-03, P-04. **Done when** the N3 and N4 acceptance rows pass.

## 18 — Tool and package delivery by license

1. Add license, delivery-channel and verified-version fields to the tool catalog and the known
   packages.
2. Mirror license-permitted packages and tools on VUA's mainland China host, unchanged, with
   their license text, verified against the official hashes and labeled as mirrors; packages
   under their own terms (VRChat SDK) come from the official source.
3. Test installation from a mainland China network on the author's remote host.

**Eval:** C-01, C-02. **Done when** both pass, including the mainland China run.

## 19 — Experimental VRChat sign-in

Designed in [VRChat sign-in](vrchat-account.md) (author decision, 2026-10-18).

1. An Experimental setting, off by default, with the risk explained before the first sign-in;
   sign-in in VUA with VRChat's two-factor step; the session kept, the password only with
   "Remember my password", both encrypted per Windows user.
2. Before a managed upload, the SDK in the open Editor receives the session and starts signed in.
3. After an upload, VUA checks the Avatar against the player's Avatar list.

**Done when** the author signs in once in VUA, an upload starts with the SDK already signed in, and
the upload check finds "AMF test1".

## 20 — Start production from the materials room

1. Choosing an Avatar in the library offers "Start production", with the outfits that fit it
   marked, and opens a new production line with them selected.

**Done when** P-02 starts from the materials room without visiting the workshop's selection.

## 21 — Faster project fingerprints

After each Unity run VUA fingerprints the whole work project; on the development machine this takes
about 40 seconds for the Meiyun project (about 4,200 files).

1. Cache each file's hash by path, size and modification time, and hash only changed files.
2. Show the check as its own stage in the line view.

**Done when** the check after an unchanged reinspection takes a few seconds.

## 22 — Motion setting

VUA follows Windows' animation setting, which flattens CSS motion on the development machine; the
factory camera, page turns and album tabs are scripted and follow resource-saving mode instead.

1. The author decides whether all VUA motion follows its own resource-saving mode, with Windows'
   setting as the default for new installs.

**Done when** the chosen rule applies to every animated screen.

## 23 — Base map art

The base map's rooms are drawn in code from simple shapes. Art for the default and native skins
(sprites or skeletal animation for workers, props and stages) comes with slice 13's design session,
in the scene-pack format of the [skin system](skin-system.md).

**Done when** the default skin's rooms use their art pack, with the code-drawn scenes as fallback.

## 24 — Finer usual durations

"Usually … on this computer" is the median of earlier steps of the same kind. If the estimates
prove too rough, group them by project or by the number of imported assets.

**Done when** the shown usual time lands within a third of the actual time for most steps.

## 25 — Production records in Settings

The release room's "Production records" tab lists runs and their records for diagnosis.

1. Move it into Settings as debugging information; the release room keeps the album of
   finished Avatars.

**Done when** the release room shows only the album, and Settings shows the production records.

## 26 — VRChat Avatar list in the release room

1. With the [experimental VRChat sign-in](vrchat-account.md), the release room shows the
   account's own Avatars: name, image, release status, platforms and last update.
2. Finished Avatars link to their VRChat entry, and the list marks the Avatars VUA uploaded.

**Done when** the author sees "AMF test1" in the list with its current state on VRChat.

## 27 — Queue factory work

The factory runs one Unity task at a time. Unity itself can open several projects at once;
VUA keeps to one because each Unity run uses most of the machine and an upload uses the
interactive Editor. Work requested while a task runs waits for it (Candidate
[amf-production 0.16.0](../protocols/amf-production-v0.1.md#queue)).

1. Uploads and other Unity work requested while a task runs wait in a visible queue and start
   in order; the player can remove a waiting item.
2. Later, on machines with enough memory, work on different projects may run side by side.

Item 2's engine is in (Aran52/VUA#23, [amf-production 0.19.0](../protocols/amf-production-v0.1.md#queue)):
`amf.production.setCapacity` sets a capacity of 1–3 while no Unity work runs or waits, and
uploads, editor handoffs and deletions run alone. Remaining: the workshop setting, and the rooms
reading several running jobs instead of one.

**Done when** an upload requested during a reinspection starts by itself when the reinspection
ends.

## 28 — Quitting during a task

Quitting VUA from the tray during an AMF task stops the task and Unity without asking. A crash or
a forced end stops Unity too: the AMF provider keeps Unity in a Windows job that ends with it.

1. Quitting while a task runs asks first: stop the task and quit, keep VUA running in the tray,
   or cancel.

**Done when** quitting during a reinspection offers the three choices.

## 29 — Restore after an interrupted assembly

After a forced end, a line whose assembly was running shows "needs a new check", and its work
project may be half assembled. The restore point taken before that assembly is kept; today the
player reinspects first and then restores from the workshop.

1. At the next start, such a line offers "Restore to before this assembly" directly, next to
   "Continue".

**Done when** a line interrupted during assembly returns to its pre-assembly state in one step.

## 30 — Workshop stations and line management

Author request, 2026-10-18.

1. The workshop's separate checks page leaves; the product boundary dropped it on 2026-09-22, and
   the checks live in the intake-check and quality-check stations and the release room.
2. Each station on a line is a place to click: materials, intake check, plan, assembly and
   quality check each open their own panel; without a choice the page follows the line.
3. A line changes its own materials in place ([amf-production 0.17.0](../protocols/amf-production-v0.1.md#changing-a-lines-materials))
   behind a restore point that keeps the earlier materials; a finished line can change its plan
   and assemble only what changed.
4. Each line card closes from its corner: hide, or delete after a confirmation, which removes
   the work project and the line's records.

**Done when** the author swaps the outfit of a finished line on that same line, brings the earlier
outfit back from the restore point, and deletes a test line whose work folder is then gone.

## 31 — Change a finished line's base Avatar

Found in the author's use, 2026-10-19. A line's Avatar prefab comes from the stated
recommendation, which prefers the creator's base for outfits, such as Meiyun's `Meiyu_改変`.
On an assembled line, choosing another Avatar prefab in the plan, such as the standard `Meiyun`
that wears its own outfit, stops with "Choose a new Avatar to change the base prefab"
(`avatar_target_changed`), and the workshop offers no way forward. Keeping the base's own
outfit and adding to it is a common wish.

1. Choosing another Avatar prefab on a finished line rebuilds the line on that base: behind a
   restore point the work scene starts fresh, as for a material change, and the new base and
   every chosen outfit are assembled again.
2. The plan says so before assembly, and the restore point brings back the earlier base.

**Done when** the author moves a finished Meiyun line from `Meiyu_改変` to `Meiyun`, keeps its
outfits and adds one, and returns to the earlier base from the restore point.

## 32 — Agent interface refinements

Lessons 1–8 of the [reference study](../research/reference-projects-2026-10-19.md#vrcforge-the-agent-interface).

1. Small edits (material values, a menu entry, removing one outfit) as preview and apply pairs:
   the preview returns a digest, and the apply refuses a changed scene or hierarchy.
2. Tool results state whether a mutation started, committed, persisted and was read back, and
   whether a retry is safe.
3. Run facts and Build Records as versioned MCP Resources; versioned SOPs as MCP Prompts with a
   content hash.
4. Help writes the connection into the chosen AI tool's configuration file, with STDIO next to
   Streamable HTTP, when the player asks for it.
5. A supervised hand adjustment in the open Editor: the agent prepares a proxy, the player drags
   it, the agent finalizes.
6. Reflection-only checks with Gesture Manager (switch behavior in Play Mode) and NDMF's
   parameter count.

**Done when** an agent completes a P-02 material adjustment through a preview and apply pair,
reads the run as a Resource, and recovers from a dropped connection without a repeated write.

## 33 — Switch options and built-result evidence

Lessons 9–13 of the [reference study](../research/reference-projects-2026-10-19.md#vrcfury-and-modular-avatar-assembly-features).

1. Switch options in the plan: saved state, default on, an exclusive group for main outfits and a
   slider, built with MA's Object Toggle and Menu Item.
2. Previews, the turntable and the menu check run on the NDMF-processed clone.
3. The quality-check station lists mixed Write Defaults, empty layers, uneven anchor override and
   bounds (fixed through MA Mesh Settings), duplicate PhysBones, unused bones and parameter order
   across PC and mobile.
4. The plan shows synced parameter bits against 256 before assembly.
5. The intake check recognizes outfits carrying VRCFury components; the known packages list
   VRCFury with `guide` delivery from its official VCC feed after its license check.

**Done when** a P-02 line with two outfits in one exclusive group passes the quality check on its
built result, and an outfit with VRCFury components shows the guide for VRCFury.

## 34 — Evidence layers and toolchain profile

Lessons 14–19 of the [reference study](../research/reference-projects-2026-10-19.md#vrchateditorskill-inspection-and-evidence).

1. Each fact on the check stations and the release page carries its evidence layer: static
   source, Unity-resolved, NDMF-built, SDK build, client runtime or confirmed upload.
2. The intake check records the providers the Avatar and outfits use as a toolchain profile.
3. The result's Expressions Menu tree, traced to parameters and objects, joins the Build Record and
   the Recipe.
4. Changes to shared assets copy them after listing their users; generated output stays untouched.
5. The first symptom playbooks (menu missing, parameter without effect, merged Animator missing,
   wrong binding, wrong switch default) become SOPs for slice 15.

**Done when** the P-03 result shows its menu map with evidence layers, and an agent diagnoses a
missing menu item from the playbook SOP.

## 35 — Start card, preflight and attribution

Lessons 20–28 of the [reference study](../research/reference-projects-2026-10-19.md#vrchat-avatar-acceptance-workflow-flow-and-acceptance).

1. One start card per line with the recommendations prefilled: delivery layer, size limit,
   quality and performance trade-off, menu structure and language, upload authority.
2. Before assembly the plan predicts each outfit's size, triangles, material slots, texture
   memory, PhysBones and parameter bits.
3. The release page attributes the change from the base Avatar to each outfit and shows the SDK's
   uncompressed size with a warning line at 90 % of its limit.
4. Menus keep the creator's parts menu, with main outfits exclusive and at most 8 controls per
   page.
5. Body-shape variants of the Avatar need matching outfit support before assembly.
6. One targeted repair per defect, then restore or isolate; uploads private by default.
7. Local lessons from the player's corrections, keyed by the materials they concern.
8. A finished state requires the Build Record's evidence to close.

**Done when** a P-03 line runs from one start card to a release page that attributes each
outfit's cost, and a repeated correction is suggested from its lesson on the next line.

## 36 — Licensed entry guide

Designed in the [licensed entry guide](licensed-entry-guide.md) (author ruling, 2026-10-19).

1. The author verifies pixiv sign-up, BOOTH, card payment and PayPal from the remote host on a
   mainland China network.
2. The guide's six parts as versioned SOPs in four languages for every player outside Japan,
   opened from Help and the library; the mainland China payment note is chosen by the player at
   the payment step.
3. Images and animations download as content packs when a part opens.
4. A reachability check for the sites the guide's steps use, built on the website tests.

**Done when** a player in mainland China with no pixiv account completes a first BOOTH purchase
guided by VUA alone, the purchase arrives in the library, and a player who never opens the guide
downloads none of its content packs.

## 37 — Creator credits

1. The release room lists, for each finished Avatar, its materials with their creators and BOOTH
   pages, from the Build Record's provenance.

**Done when** the P-02 result's album entry names Meiyun's and the outfit's creators with links
to their BOOTH pages.

## 38 — Steadier regional latency

The Play network tile compares VRChat's four regions with
[Oracle reference origins](../architecture/network-onboarding.md#user-outcome-and-approximate-regional-references):
one warm-up and the median of three HEAD responses on one connection per region
([website-tests 0.3](../protocols/website-tests-v0.1.md)). The author finds the results unsteady.
A run from this development machine on 2026-10-19 (curl, eight requests per region) showed
steady response times after TLS, Tokyo 11–32 ms and Amsterdam 231–239 ms, and single TCP
connections up to ten times slower than the region's median. Three samples on one connection let
two such outliers decide the value, and international routes can differ per connection.

1. Measure on two or three connections per region: one warm-up each, then nine HEAD responses
   in total spaced about 150 ms apart. Report the minimum as the region's value, with the median
   and the spread from the minimum to the 90th percentile.
2. Sample the four regions in turn (Tokyo, US West, US East, Europe, then again), so all four see
   the same moments of the network; regional tests wait for running website tests.
3. The tile shows the regions in order with a tier; regions within 15 ms or 10 % of each other
   read as close, and a large spread reads as unsteady. Milliseconds stay in the expanded
   details.
4. When a reference resolves into `198.18.0.0/15` or a proxy is set for VUA's process, the result
   says it was measured through a proxy on this computer, whose route can differ from the game's.
5. A new website-tests version records the method; the four origins and the wire shape stay.
6. Calibration: on two or three networks, including the remote host on a mainland China network,
   the author joins an instance in each region and notes VRChat's own Ping; the tile's first
   region is compared with the lowest in-game Ping.

**Done when** repeated tests on one network give the same first region, and the calibration
runs agree with the in-game Ping on the first region. Real UDP measurement through VUA-hosted
echo endpoints is decided after the calibration.

## 39 — Resource panel that guides

The title bar shows 100 % minus the mean of CPU, GPU, RAM and VRAM, with a warning from 90 %.
A native helper samples PDH counters every 2 seconds and re-enumerates adapters every 30 seconds
by creating a D3D12 device on each; the renderer polls every 2 seconds while VUA runs.

1. The title bar names the tightest resource or a state (plenty, getting tight, tight) from the
   highest reading instead of the mean, at a fixed width.
2. The panel ties readings to VUA's work: the largest memory users with VUA's own tasks marked
   (for example a Unity import), a suggested production capacity from total memory, and VRAM next
   to the Avatar's texture memory from slice 9.
3. The title bar shows a reading smoothed over about 10 seconds; the panel shows the last minute
   as a sparkline kept in memory only.
4. Lighter sampling: polling stops while the window is hidden and slows to 5 seconds while the
   panel is closed; each adapter's integrated or discrete kind is cached by LUID, so a D3D12
   device is created only for a new adapter; the cost of the `GPU Engine(*)` counter is measured
   with VRChat running.

**Done when** the title bar names the bottleneck during a P-02 run, and the panel's own sampling
stays under a measured budget with VRChat running on the development machine.

## 40 — Notification center redesign

Author review, 2026-10-19. The bell is a 16 px outline icon in the secondary text color, and its
badge counts only running tasks and waiting AI requests. The panel opens over a full-window blur
and closes on an outside click, an outside scroll, Escape or when the window loses focus, so it
cannot stay open while the player works. Rows use different layouts for downloads, counted
progress, errors with raw codes, AI requests and the AMF failure line, and show no times. "Show
completed" has no effect: the notification hook never passes the set of tasks that finished in
this session, and the setting leaves with this slice.

1. The bell shows what waits: an alert color for a failure, the accent for something the player
   must confirm, a progress ring while work runs, and a dot for results not yet seen.
2. A pin turns the panel into a side drawer without a backdrop that stays open beside the page;
   unpinned, it keeps today's popover behavior.
3. One card layout for every entry: status icon, title and time; one line of state or result (for
   AMF work the step with its elapsed and usual time); an optional progress bar; actions on the
   right; error codes under "Details". AI requests and AMF failures use the same card.
4. Three sections replace "Show completed": needs you (AI requests, failures, waiting for input),
   running, and finished this session with "Clear all". Older history stays in production records
   in Settings. Each kind of work keeps its own latest finished entry, so a turntable bake no longer
   hides the assembly that preceded it.
5. A long task that finishes or fails while VUA is in the background raises a Windows
   notification.
6. The cleared-entry list keeps only entries of tasks that still exist.

**Done when** the author follows a P-02 assembly in the pinned drawer while using the workshop,
sees a failure on the bell without opening it, and gets a Windows notification for a task that
finished in the background.

## 41 — VN3 terms and filtering by use

Many Avatar bases and outfits link their VN3 license from the BOOTH description, mostly as Google
Drive or Docs PDFs. VN3 documents come from its license generator: items A–X with a small closed
set of values, a generator and VN3 version in the footer, and a free-text special note that takes
precedence over the rest. Read on 2026-10-19 from Shinano's English VN3 document (generator 1.11,
VN3 1.10). BDL's `term_observations` table exists without a writer.

1. Recognize license links in product descriptions and fetch public documents one at a time,
   without cookies, from Google's public download endpoints; cache them by content hash on this
   computer.
2. A versioned rule parser turns each document into its A–X values with conditions, such as
   permitted with a content rating; unrecognized wording or versions stay unknown. No AI service
   reads the documents.
3. BDL stores documents, items and their link to products; a terms contract answers a product's
   terms and checks one material or an Avatar's materials against use presets (VRChat personal
   use, individual and corporate VTubers, game development, outfits for sale, merchandise and
   publications, adult content), with the strictest item deciding and unknown never passing.
4. Claude adds the library and release-room views and the MCP tool afterwards.

The backend is assigned through `h-vn3-terms.md` in the task ledger. A catalog-wide index built
on VUA's servers waits for the author's decision.

**Done when** Shinano's terms read F as permitted with a content rating and R as contact the
licensor, and an Avatar's materials are checked together for a chosen use.

## 42 — Labeled restore points

Found in the MCP round, 2026-10-19. Each line keeps one restore point, and every protected change
replaces it. After a material change and a new assembly, the restore returned to the state before
that assembly; the state before the material change was gone, and the panel named neither.

1. Each line keeps its most recent restore points (three proposed), each labeled with the change
   it came before, its materials, its Avatar and when it was taken.
2. The assembly station lists them; restoring one protects the current state first, as today.
3. `vua_amf_run` lists them; restoring stays the player's action.

**Done when** the author changes a line's outfit, assembles, and brings the earlier outfit back
from its labeled point.

## 43 — Fixes from the MCP round

Issues 2–5 and 10 of the round's report, and one found while reading its records.

1. An upload an AI requests is confirmed on the release page: the request card takes the player
   there instead of approving in place, and a failed approval names its cause instead of "could
   not connect".
2. The line page clears a load error once the run loads again and keeps the field that failed
   validation; the transient failure seen twice in the round is found and fixed.
3. Reading a run that does not exist reports `run_not_found` instead of `store_failed`.
4. A request for a project that Unity holds is refused when it is made, without a failed task;
   the line page's progress line updates when a task ends.
5. Deleting a line also removes its assembly records.
6. The workshop showed two line cards for one run once; it is reproduced and the list fixed.

**Done when** the round's upload, delete and stop cases repeat without a misleading message, a
stale banner or a stray record.

## 44 — MCP payloads and identity

Agent-side findings of the round, rated per tool in its report.

1. `vua_tasks` takes a limit and a state filter, newest first.
2. `vua_amf_run` and `vua_library` stay compact: humanoid bones as a count and image URLs only
   when asked for.
3. `vua_status` reports the build: product version, commit and build time.
4. Errors carry their plain-language message next to the code and the next action.
5. Tasks and run facts say who started them: the player, the workshop's automation or an AI
   request.

**Done when** an agent repeats the round's reads at under 10 KB each and names the build it
tests.

## Open decisions

- Styles: built-in only first or shareable data from the start; how the two briefs translate
  into tokens, motifs and motion.
- Known packages beyond the shaders, SDK and Modular Avatar (for example Avatar Optimizer,
  VRCFury), each after a license check.
- Network steps for players in mainland China: which sites the licensed entry guide and Quest's
  first activation report, after the author's verification; VUA names no accelerator or proxy.

## Scheduled after these slices

Social discovery built on VRCX-0 (pain d), the remaining N1 physical rows, N5 real-account
review, and N7 clean-machine installation.

## Working arrangement

- Code, targeted Rust builds and real Unity runs happen on the development machine, one heavy
  job at a time. CI runs the full Rust, TypeScript, smoke and installer checks.
- Mainland China network cases run on the author's remote host.
- The author supplies real materials, account sign-ins and human UI review.
- Each slice is built on a `slice/<slug>` branch in its own worktree, branched from the stage's
  production branch named under "Where things stand", and returns to it through a pull request
  on the fork once CI is green. The author merges the production branch into `main` with the
  organization account.
- Claude integrates: it reviews pull requests from other assistants, fixes what blocks a merge on
  their branch, merges, and keeps this plan's status. Other assistants report status in their
  pull request.
- The main checkout runs the author's dev VUA from the production branch and is rebuilt after
  merges while VUA is idle. Electron runs in other worktrees use the main checkout's Electron
  through `ELECTRON_OVERRIDE_DIST_PATH`, because Windows asks about network access once per
  Electron path.
- Task prompts for other assistants and their ledger stay in `_local_real_machine/prompts/`;
  machine runbooks and helper scripts stay in `_local_environment/`.
