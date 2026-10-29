# Freeze plan

> Updated: 2026-10-29 (author's ruling)

VUA is the first step of PH-R's serial order. It freezes before PH-R headset work starts. This plan
says what must be true before the freeze, what stops now, and what may change afterwards. It
overrides the order in the [current plan](current-plan.md) until the freeze release is published;
the [N sequence](../development-outline.md) keeps its acceptance rows.

## What the freeze means

After the freeze release, VUA receives only:

- bug fixes, including fixes found by players;
- security updates;
- compatibility with new VRChat SDK, Unity, Steam, PICO and Electron versions;
- native review of the five UI languages;
- the Pharos settings and controls, when PH-R reaches that stage. This is the one planned
  feature addition and gets its own plan.

Anything else is recorded as a later idea and not started.

## Rule while the freeze is open

No new slice starts unless it is a row in this plan. A slice prompt for another assistant names
the row it serves. Work found along the way that is not a fix goes to "After the freeze".

## Before the freeze

### 1. Host checks

The author runs the host checks and may skip any of them; the release notes list what was
skipped.

| Slice | Check |
| --- | --- |
| 3 | The SDK starts signed in after the shared product name change |
| 19, 26 | Experimental VRChat sign-in and the Avatar list with a real account |

Slice 11 (Unity sign-in and license) finishes its scenario step (author, 2026-10-29): the
Avatar-editing preparation shows the license state with one next action, and a production
license failure opens that step with the line retained. Testing Hub's Unity CLI waits until
after the freeze.

### 2. Close the open branches

Every branch with work since v0.7.0 is merged or closed before the freeze. Merge the current
state; where a feature is unfinished, hide its entry and record what is missing. Do not finish
an unfinished feature to close its branch.

| Branch | Content |
| --- | --- |
| `slice/component-update-*` | Independent signed host and AMF updates; merge, because fixes after the freeze ship through it |
| `slice/theme-styles`, `slice/grid-theme-audit` | Grid, Aero, Tenshi and Jirai presets; keep what is merged, stop slice 13 here |
| `slice/terms-everywhere`, `slice/turntable-share` | Recipe terms before import, turntable image with a Recipe link |
| `slice/share-link-service` | Public share links and material pages, served by the trial server in mainland China |
| `slice/local-material-entry` | Local entry and CI fixes |

Delete branches that are fully merged.

### 3. AMF checks by the community

AMF's real-material checks move to community testers after v1.0.0 is published. They report
results as GitHub issues; fixes ship as independent AMF updates, so they do not need a new host
release. These checks do not block the freeze.

| Slice or case | Check |
| --- | --- |
| 5 | A project held open in Unity: save and close from VUA |
| 6 | The four build-and-upload guide screenshots |
| 7 | P-02 in each permission mode |
| 8 | P-01 with a Poiyomi outfit |
| 9 | Material tuning in a dark world and on a P-02 rank |
| 10 | Face tracking in VRChat |
| 20 | Start production from the materials room with real materials |
| 27 | Two lines running side by side |
| 30 | Removing two outfits |
| 31 | Changing a finished line's base Avatar |
| 41 | VN3 terms with real products |
| 42 | Restoring a revision and applying a saved fit |
| 43 | The transient failure, if it appears again |
| P-03 | The N3 minimum case: one real Avatar, at least two actively used dependencies or plugins and at least six other real materials in one project and one run, handed to the SDK |
| P-04 | The P-03 Recipe rebuilds the same relationships and settings in a second, separate project |

### 4. Fresh installations

| Case | Pass condition |
| --- | --- |
| VUA on a clean Windows VM | Online and offline installers each install, launch and reach the home screen; record the VM image |
| External tools | Each of the six connected Steam tools (VRCFT, Space Calibrator, OVR Advanced Settings, OVR Overlay Translator, OVR Toolkit, OyasumiVR) goes from not installed through VUA's Steam handoff to observed installation and launch at least once |
| Upgrade from v0.7.0 | Installing the freeze release over v0.7.0 keeps material records, Recipes, settings, run history and saved module choices, and VUA reaches the home screen |
| AMF update without GitHub | On a network where GitHub is unreachable, VUA finds, downloads and activates an independent AMF update from the mainland mirror; record how GitHub was blocked |

LIV and VRCS stay as description cards.

### 5. First run and guidance

The author's ruling of 2026-10-28 is part of v1.0.0:

- The first run asks no language question; VUA uses the detected system language.
- The first launch opens a first-use notice with four parts: what VUA is for, the changelog,
  "VUA is open source under Apache 2.0 and free to download on GitHub", and links to the
  third-party licenses. It has two buttons:
  - **知道了** enters VUA with the existing light, skippable feature tour;
  - **我完全不知道要做什么** opens a page-by-page wizard joined with the encyclopedia. It starts
    from "what is VRChat" and introduces one concept per page. Each page has two buttons:
    **我要这个** sets up or deploys that thing, **我知道了** goes to the next page.
- Where a concept matters, VUA explains it in place and links to the matching encyclopedia
  section; for example, the upload step explains the player's trust rank.
- The encyclopedia reads like a manual — neutral, accurate and concise — with a moderate
  summary and commentary from an experienced player.

### 6. Freeze release

- The product name becomes **Virtual Unified Assistant** in the app, READMEs, release notes
  and installer metadata.
- The release notes state that the UI translations have not been reviewed by native speakers.
- The release notes list every row of this plan that was skipped or failed, with its impact,
  and invite players to the community AMF checks.
- The freeze release is **v1.0.0 Brave New World**, selected by the author on 2026-10-29. Brave
  New World is Aldous Huxley's novel and the Stellaris achievement for colonizing the first
  planet. Version 1.0.0 ends the continuing-Beta policy; the
  [versioning policy](../release/versioning.md) is updated with the release.
- Share links use the trial server already deployed in mainland China. It is reachable from
  mainland China and probably not from elsewhere; the release notes say so, and a share link
  that cannot be reached fails with a clear message instead of an empty page.

## After the freeze

These items stop now. Each may come back only through a new decision.

| Slices | Item |
| --- | --- |
| 13, 22, 23 | Styles beyond the merged presets, motion setting, base map art |
| 14 | Native review of the five UI languages (allowed after the freeze as a fix) |
| 15, 16 | Versioned SOPs, Eval runs |
| 17 (beyond P-03/P-04) | Further complex-production coverage |
| 18 | Tool and package delivery by license |
| 24 | Finer usual durations |
| 32, 34, 35 | Agent interface refinements, evidence layers, start card |
| 33 (remaining) | The slider, built-result evidence, VRCFury |
| 36, 38 | Licensed entry guide, latency calibration |
| 37 | Creator credits |
| 39, 40 | Resource panel, notification center redesign |
| 45–48 | Source hints, library migration, identical-content choice |
| 49, 51 | Turntable sharing and creator-trusted Recipes beyond the merged state |
| 50 | Optional AI connection changes |
| 52 | Signed releases (already on hold) |
| 53 | Interface consistency audit beyond its first report |
| 11 (item 3) | Testing the Unity CLI that Hub installs |
| — | N7's illustrated user guide with screenshots of the tested release |
| — | Encrypting saved sign-in cookies, which needs the installer to verify a fused Electron runtime |
| — | Development dependency upgrades proposed by Dependabot (VUA-Project/VUA#87, #89) |
| — | Social discovery on VRCX-0, remaining N1 physical rows, N5 real-account review |
