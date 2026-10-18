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
| 5 | Unity editor handoff | P-02 | |
| 6 | SDK step for newcomers | P-02 | in progress: the sign-in guide, the SDK's own checks (Aran52/VUA#6), one-click fixes and plain-word messages (Aran52/VUA#12) are in; the build and upload guide remains |
| 7 | Permission modes and simpler choices | P-02, P-03 | permission modes (Aran52/VUA#11) and stated recommendations (Aran52/VUA#16) in the workshop and `vua_amf_plan`; the image confirmation remains |
| 8 | Known packages | P-01, P-03 | in progress: the known-packages table with detection and installs is in (Aran52/VUA#21); the package page by permission mode (items 3–4) and two follow-ups remain |
| 9 | Material tuning | P-02, P-03 | assigned: item 2's engine, Bridge facts and contracts |
| 10 | Face tracking add-on | P-03 | |
| 11 | Unity sign-in and license | — | assigned: the detection and check engine and contracts |
| 12 | Managed upload | P-02 | done: first real upload on 2026-10-17 (Aran52/VUA#4) |
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
| 27 | Queue factory work | P-02 | built (Aran52/VUA#18): one job at a time, the rest wait in a visible queue; awaits the author's real-run check. Item 2's engine is in (Aran52/VUA#23); its workshop setting remains |
| 28 | Quitting during a task | — | done (Aran52/VUA#14) |
| 29 | Restore after an interrupted assembly | — | done (Aran52/VUA#15) |
| 30 | Workshop stations and line management | P-02 | built (Aran52/VUA#20); awaits the author's real-run check; removing an outfit through the plan remains |
| 31 | Change a finished line's base Avatar | P-02 | next |

Suggested next order: 31 Base change, 1 Integrate, the rest of 8, the rest of 6 (build and upload
guide), 5 Editor handoff, the rest of 7 (image confirmation). 31 removes a dead end on finished
lines; integration keeps the production branch close to `main`; 6 and 8 close the largest gaps a
newcomer meets in P-02 and P-01.

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

**Done when** a held project continues without closing Unity by hand.

## 6 — SDK step for newcomers

1. The Bridge reads the official SDK checks and performance statistics for the result.
2. VUA shows each item in plain words as "must fix before upload", "reminder, upload still
   works" or "affects the performance rank", with a one-click fix where VUA can make it.
3. An illustrated guide to the SDK panel in four languages: sign-in, build, warnings, upload.

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

## Open decisions

- Styles: built-in only first or shareable data from the start; how the two briefs translate
  into tokens, motifs and motion.
- Known packages beyond the shaders, SDK and Modular Avatar (for example Avatar Optimizer,
  VRCFury), each after a license check.

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
