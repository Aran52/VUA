# Current development plan

> Updated: 2026-10-16

This plan orders the next slices toward the [north star](../north-star.md). Each slice names the
[Eval cases](../eval/cases.md) it moves and what counts as done. The
[N sequence](../development-outline.md) keeps the acceptance rows; this plan sets the order.

## Where things stand

- `main` carries the Ibis releases (v0.6.7).
- `slice/north-star` (VUA-Project/VUA#83) adds the north star, the Eval and the documentation rules.
- `slice/n3-n4-ma-production` (VUA-Project/VUA#84) adds the material-first AMF production path,
  Recipe v0.4, the local MCP adapter and six Steam tool connections, plus the fixes and features
  from the first real run:
  - Unity's package manager starts in the managed project; leftover Unity lock files and
    retained Bridge copies are handled; assembly runs without a Recipe target;
  - lilToon installs from its official VPM feed in place of bundled copies;
  - each AI request is confirmed on the page it changes, waits until the player decides (also
    across restarts), can be withdrawn by the AI, and explains a stale state in plain words;
  - the bell offers waiting requests; the tray icon and taskbar signal them in the background.
- VUA-Project/VUA#85 detects the packaged (MSIX) Unity Hub; VUA-Project/VUA#86 keeps notification
  rows in place while downloads progress.
- First real run (P-02, Meiyun with the T-shirt & shorts): assembly succeeded on 2026-10-16. The
  author's Unity check found three issues: lighting (bundled lilToon 1.4.0, now replaced),
  texture memory, and the SDK panel's difficulty for newcomers.

## Order

| # | Slice | Eval | State |
| --- | --- | --- | --- |
| 1 | Integrate | — | in progress |
| 2 | Finish the first real production runs | P-02, P-01 | in progress |
| 3 | Result previews | P-02 | next |
| 4 | Unity editor handoff | P-02 | |
| 5 | SDK step for newcomers | P-02 | |
| 6 | Unity familiarity levels and simpler choices | P-02, P-03 | |
| 7 | Known packages | P-01, P-03 | |
| 8 | Texture memory | P-02, P-03 | |
| 9 | Unity sign-in and license | A-01 | |
| 10 | Managed upload | P-02 | |
| 11 | Appearance and one-click styles | — | needs a design session |
| 12 | Player language | P-02 | |
| 13 | Versioned SOPs | A-03, B-02–B-05 | |
| 14 | Eval runs | all `ready` cases | |
| 15 | Complex production and Recipe reproduction | P-03, P-04 | |
| 16 | Tool and package delivery by license | C-01, C-02 | |

## 1 — Integrate

1. Merge #83, then #84, into `main`; merge #85 and #86.
2. Keep CI green on each pull request.

**Done when** all four are on `main`.

## 2 — Finish the first real production runs

1. Rerun P-02 with lilToon from VPM; the author checks the lighting in Unity.
2. Save the result as a Recipe.
3. Run the missing-shader case (P-01) on a project whose material needs a shader it lacks.

**Done when** P-02 and P-01 pass as closed-loop runs in the app, with evidence in `_local_eval/`.

## 3 — Result previews

1. The Bridge renders the finished Avatar from the front, side and back under neutral lighting,
   and one image per Avatar and outfit candidate during inspection, using the existing preview
   renderer. Images stay in the work project.
2. The Avatar page shows them; the agent reads them to check a result before asking the author.

**Done when** a P-02 result can be judged from its previews, and each candidate has an image.

## 4 — Unity editor handoff

1. The Bridge inside an open Editor accepts "save and close": save scenes and assets, quit, and
   send a receipt.
2. When work needs a project an Editor holds, the request card offers "Save and close Unity,
   then continue". After the look check, the page offers "Done checking, close Unity".

**Done when** a held project continues without closing Unity by hand.

## 5 — SDK step for newcomers

1. The Bridge reads the official SDK checks and performance statistics for the result.
2. VUA shows each item in plain words as "must fix before upload", "reminder, upload still
   works" or "affects the performance rank", with a one-click fix where VUA can make it.
3. An illustrated guide to the SDK panel in four languages: sign-in, build, warnings, upload.

**Eval:** P-02 outside-help count. **Done when** the author uploads the P-02 result guided by
VUA alone.

## 6 — Unity familiarity levels and simpler choices

1. On first entry to Avatar production VUA asks how familiar the player is with Unity: fully
   automatic, semi-automatic or fully manual; the choice is changeable in settings and sets how
   much each step asks and shows.
2. VUA recommends the Avatar and outfit prefabs with stated rules (name match with the outfit's
   target, top-level prefab, the creator's base for outfits) and asks only when two candidates
   are equally plausible.
3. The confirmation reads as one sentence with images: BOOTH product images for materials and
   the previews from slice 3 for prefabs; binding details sit under "Adjust".
4. `vua_amf_plan` without a selection returns the recommendation.

**Done when** the author completes P-02 at the fully automatic level with one confirmation per
change, and at the manual level with every detail adjustable.

## 7 — Known packages

1. A table of known packages: ID, feed, license, delivery policy, detection signals. First
   entries: VRChat SDK (official source), Modular Avatar with NDMF, lilToon.
2. Bundled copies are recognized from each package's `legacyFolders` and `legacyFiles`
   (lilToon declares `Assets\lilToon`), generalizing the lilToon rule.
3. The package page lists work projects with installed versions, updates and Recipe pins.

**Done when** P-01 resolves its missing shader through a known package.

## 8 — Texture memory

1. Measure each texture's memory and the total against VRChat's performance thresholds.
2. Lower Max Size and switch compression in the work project, reversible by snapshot: toward a
   target rank (fully automatic), as a reviewed list (semi-automatic) or per texture (manual).
3. Evaluate lilAvatarUtils (MIT) first; integrate it when it covers these steps.

**Done when** the P-02 result reaches a chosen rank without visible loss the author objects to.

## 9 — Unity sign-in and license

1. Detect the license from the license file (standard and packaged-Hub locations), whether Hub
   and its licensing client run, and a short real check in a VUA-owned project.
2. The scenario step shows the state and the one next action; a production license failure
   opens that step with the run retained.
3. With the packaged Hub, Unity work needs Hub running; see the open decisions.
4. Test the Unity CLI that Hub installs at `%LOCALAPPDATA%\Unity\bin\unity.exe`.

**Done when** a fresh machine reaches a licensed Unity run guided by VUA alone.

## 10 — Managed upload

1. Verify the SDK's build-and-upload interface in SDK 3.10.5, including whether it needs the
   Editor window.
2. The player confirms the content rights statement in VUA for each upload; the VRChat sign-in
   stays in the SDK.

**Done when** the P-02 result uploads from VUA with the player's confirmation.

## 11 — Appearance and one-click styles

1. Design session with the author: which screens, which first styles, and whether a style may
   recolor the module colors.
2. Move remaining literal values into tokens (colors, radii, shadows, font sizes, timings).
3. A style is token data checked against a schema, switched with one click in Appearance and
   the command palette; contrast is checked per style in dark and light.
4. Redo the notification panel's size and animation; build the production animation from real
   step events.

**Done when** the author switches styles in one click and approves the redone screens.

## 12 — Player language

1. Remove internal references and process vocabulary from UI text; add a four-language
   terminology table and a check for Chinese full-width punctuation.
2. Rewrite the Japanese and Korean strings for the AMF, tour and onboarding pages natively.

**Done when** the author reviews the AMF journey in all four languages.

## 13 — Versioned SOPs

1. Turn the scenario SOPs into versioned SOP documents with the fields in the north star.
2. Write the first symptom-to-solution SOPs and serve them through Help and `vua_sop`.
3. Record each SOP's last verification.

**Eval:** A-03, B-02, B-03, B-04, B-05. **Done when** these cases are `ready` and pass in both
channels.

## 14 — Eval runs

1. Convert the cases into machine-readable files and define the run record.
2. Run every `ready` case in the app and through an MCP agent; publish the two pass rates and
   median times with the next release.

**Done when** a release reports both metrics.

## 15 — Complex production and Recipe reproduction

1. Run the N3 complex case with the author's real materials, then repeat it in a clean project;
   include the Meiyun face tracking add-on, whose patcher runs from an Editor menu.
2. Reproduce it from a Recipe in a second project, including reapplication and conflicts.
3. Run both through the MCP agent as well.

**Eval:** P-03, P-04. **Done when** the N3 and N4 acceptance rows pass.

## 16 — Tool and package delivery by license

1. Add license, delivery-channel and verified-version fields to the tool catalog and the known
   packages.
2. Mirror license-permitted packages and tools on VUA's mainland China host, unchanged, with
   their license text, verified against the official hashes and labeled as mirrors; packages
   under their own terms (VRChat SDK) come from the official source.
3. Test installation from a mainland China network on the author's remote host.

**Eval:** C-01, C-02. **Done when** both pass, including the mainland China run.

## Open decisions

- Packaged Unity Hub: VUA starts Hub before Unity work, or guides the player to the regular Hub.
- Previews: the agent views local renders of the author's paid materials during development runs.
- Styles: built-in only first or shareable data from the start; first styles and reference apps.
- Known packages beyond the first three (for example Avatar Optimizer, VRCFury, Poiyomi), each
  after a license check.

## Scheduled after these slices

Social discovery built on VRCX-0 (pain d), the remaining N1 physical rows, N5 real-account
review, and N7 clean-machine installation.

## Working arrangement

- Code, targeted Rust builds and real Unity runs happen on the development machine, one heavy
  job at a time. CI runs the full Rust, TypeScript, smoke and installer checks.
- Mainland China network cases run on the author's remote host.
- The author supplies real materials, account sign-ins and human UI review.
