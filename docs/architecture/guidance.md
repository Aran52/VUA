# Guidance architecture: language-first onboarding, VUA instructions, encyclopedia and game guide

> Updated: 2026-10-28
> Scope: First-use language selection, contextual guidance, Help and the VRChat game guide
> Status: C+A adopted by the author on 2026-10-27; validation continues on this branch

VUA teaches three activities in different places: using VUA, learning VRChat and acting inside a
game. First-run language selection and module introductions support these activities; they are not
another long wizard. Reading never installs software, enables AMF, accepts a plan, starts a game or
uploads content.

The [product boundary](../product-boundary.md) owns scope and permissions. Delivery acceptance
belongs to the [N sequence](../development-outline.md#first-play-release-acceptance). Real game-window
and headset acceptance remains separate from renderer and synthetic UI checks.

## 1. Three teaching contexts

| Context | Player question | Presentation | Progress and source |
| --- | --- | --- | --- |
| VUA instructions | How do I use this page and recover the actual workflow? | VUA guide inside Help, optional highlight tour and small module introductions | VUA guide bookmark, explicit tour progress and separate contextual dismissals |
| VRChat encyclopedia | What does a VRChat concept mean, or how do I diagnose a problem? | Local full-text search, task entries, a topic tree and source-linked articles in the main Help window | Encyclopedia bookmark, stable topic/section IDs and cited official/community sources |
| VRChat game guide | What do I do in VRChat now? | A short adjustable-transparency guide associated with the Windows VRChat window | Independently confirmed/skipped game instructions and guide preferences |

The encyclopedia is useful before VRChat is installed or running. The current game guide observes a
Windows game window; it is not a delivered native in-headset VUA overlay. Headset reading through
SteamVR's desktop view is a separate acceptance path.

## 2. First run and contextual introductions

A new profile first chooses only a UI language. Confirming it stores the language and the existing
onboarding completion marker, then enters the fixed Home. Skipping uses the current language without
inventing business goals or environment facts. Legacy `vua-goals` data is still parsed and migrated;
old route-wizard bookmarks do not reopen that wizard. `help-wizard` remains as an internal route but
now presents the VUA guide, not a global production wizard.

The feature tour never starts automatically. A fresh profile, an upgraded profile and an old active
tour bookmark do not force it open. Help or command search starts it explicitly, with the existing
anchors, Back/Next/Skip and focus restoration. The tour navigates and explains only; it does not
execute business work.

Main modules can offer one small introduction on their first visit: Home, Play, Tools, Avatar
editing, Warehouse, Recipe, Workshop, Release and Packages. These introductions are non-modal, have
no backdrop and do not take focus. The underlying page remains usable. They offer Later, a permanent
dismissal and, where useful, a link to an actual article or the explicit feature tour. An active
feature tour suppresses them. They do not appear over language selection or Settings.

- Permanent dismissal is versioned by stable module ID and revision in local storage.
- Later is session-local and independent of permanent dismissal, task facts and readiness.
- Replay in the VUA guide clears only these two kinds of introduction state.
- A source association, detected runtime, installation, account authentication, production plan or
  upload is never completed by an introduction.

The `features/guidance/context-guidance.ts` model owns page mapping and guarded state. The component
presents the selected localized copy. The inactive tutorial service is not a prerequisite.

## 3. Automation and necessary choices

New and invalid production automation preferences resolve to **full auto**. Existing `ask` and
`smart` preferences are preserved. `factory-model.ts` keeps the existing permission-mode key and
one-time execution conditions; changing the default does not make a finished or changed line run
again automatically.

Before production starts, the materials dialog exposes the same permission picker as the workshop.
It explains that full auto prepares the work project and assembles the confirmed selection once,
without automatically uploading. Starting the dialog alone only reads materials and editors. An
actual accepted preparation and a chosen Avatar/outfit remain necessary. Entering full auto from a
less automatic mode retains the explicit warning.

The default does not infer uncertain asset roles, a target Avatar, external-project ownership,
installation consent, account authentication, destructive choices or upload rights. Those decisions
stay in the actual task. Detailed package controls remain available through the relevant details or
mode rather than a first-run preferences interview. Task-level questions replace the retired global
wizard; they are not a new chain of page popups.

## 4. Help, common questions and the encyclopedia

Help opens with common VRChat questions, then VUA instructions, the optional tour, game assistant and
external AI tools. The encyclopedia is a separate destination, not a prerequisite for every answer.

Common entries cover trust rank, Shield Level, Personal Space and Allow Untrusted URLs. They explain
what each setting changes and, just as importantly, what it does not change: account trust is not
Avatar performance, and Shield is not an upload permission. Unknown links are not declared safe merely
because they came from an acquaintance.

The local answer comes first; a matching official source follows through the existing external-link
boundary. A `guide-creator` topic holds `upload-eligibility`, `trust-rank` and `improve-trust`. The
official [first-Avatar guide](https://creators.vrchat.com/avatars/creating-your-first-avatar/) is the
source for upload prerequisites and local testing. The official [Safety and Trust
System](https://docs.vrchat.com/docs/vrchat-safety-and-trust-system) is the source for Trust Rank and
Shield behavior. The existing **Official VRChat Wiki** action opens `wiki.vrchat.com` in the isolated
browser, without requiring AMF, and returns to the same reading position. No external page receives
VUA privileges.

### Catalog, search and sources

`guide-catalog.ts` expands the existing stable topic/section content into typed articles. It retains
the legacy target mapping and marks older articles `legacy`; only individually checked articles carry
`source-checked`, their exact source links and review dates. The `guide-avatars` topic has eight
original articles in all five UI languages, using the separate `guide-avatar-copy.ts` content table
rather than enlarging every legacy language table.

A first-time encyclopedia visit opens the catalog without writing a reading bookmark. The catalog
contains six task entries and an eight-topic article tree. `searchGuideArticles` normalizes and
searches the current language's title, aliases, topic, summary and complete body, with topic and
article-kind filters. It is entirely local and requires neither AMF nor an account, remote search or
an AI answer service. Search/filter state survives returning from an article. Catalog, article and
chapter targets participate in browser Back/Forward without changing the VUA-guide bookmark.

The reader uses a vertical topic tree on wide windows and an accessible disclosure in narrow
containers. Arrow Up/Down and Left/Right follow its vertical tablist semantics. Sources remain with
their article and show the factual-review boundary; opening a source uses the existing isolated
browser and does not execute a task.

`guide-media-manifest.ts` maps selected VUA-guide sections to real VUA screenshots captured from the
production Main/preload/renderer in a disposable profile. The local files record their measured
dimensions, build commit/dirty state, capture time, controlled-production scope and privacy crop.
These are not packaged-installer, real-game, private-material, Unity-write or account-upload
acceptance. Existing SVG figures remain labeled illustrations.

### Upload eligibility in the task path

Avatar environment preparation, the upload guide and the final confirmation include the same
read-only eligibility note. Uploading a custom Avatar with Unity and the official SDK requires a
full VRChat account and New User or higher. Steam/Meta accounts must first be linked to a VRChat
account. Local build and test remain available below that rank.

The note links separately to what a trust rank is and how to raise it. Normal exploration, social
play and eligible creation are the documented direction. AFKing, mass-friending and bulk low-effort
uploads are not presented as ways to gain rank. No promotion time is promised, and VUA does not infer
a rank from a nickname, playtime or friend count. A technical check is not upload permission and does
not guarantee appearance or behavior. Final upload remains a user-confirmed official-SDK action.

### Encyclopedia and VUA-guide separation

`GuideOverlayView` accepts a topic set, initial topic/fallback, reading key and whether to record
encyclopedia history. Defaults preserve the existing reader. The encyclopedia shows hardware, play,
basics, safety, creator, devices and tutorials/reference content. The VUA guide uses only `guide-vua`,
an independent `vua-vua-guide-reading-v1` bookmark and no encyclopedia hash writes. Legacy VUA guide
targets open `help-wizard`; old `guide-vua` IDs remain valid for migration. The legacy `guide-tutorials`
meta-reading topic is retained as a valid bookmark ID but is not a main encyclopedia category.

Articles use stable topic/section targets, reading recovery, keyboard navigation and original diagrams
with captions. The VUA guide offers replay of module introductions, not a replay of the old global
wizard. All five supported UI languages are structurally checked; additional wording review is not a
claim of native-speaker acceptance. Real software screenshots are added only with explicit source and
version evidence. SVG illustrations are labeled as diagrams, not game or application screenshots.

## 5. State and routing

| State | Owner and persistence |
| --- | --- |
| Language and onboarding completion | Existing renderer preference and legacy `vua-goals`; no inferred goal from language |
| App-tour position, completion and skip | Local UI preferences, entered only after an explicit start |
| Encyclopedia topic/section | Existing `vua-guide-reading` bookmark |
| VUA-guide topic/section | Independent `vua-vua-guide-reading-v1` bookmark |
| Module dismissal and revision | Versioned local context-guide preference; session snooze is separate |
| Game-guide step and confirmation/skip | Separate local game-guide progress |
| Follow preference and background opacity | Renderer preferences; Main enforces following |
| Dragged relative position | Main's local `game-guide-placement.json`, normalized per axis; no game/process/monitor identifiers |
| Transient target and window observations | Consumed request / current observation, not permanent reading progress |
| Installation and other business tasks | Existing Orchestrator authority; guide windows only display or navigate |

Opening a chapter does not start a tour or game following. Native mouse Back/Forward traverses
encyclopedia chapters and parent/child navigation without another reader window or an implicit task.
Account help opens Settings → Accounts and returns to the source workflow. Closing a guide does not
cancel a task or invalidate an accepted receipt. Preparation still requires fresh inspection when it
is reopened; reading a guide never proves completion.

## 6. VRChat game guide

### Display and window following

The initial controls are **Follow VRChat window**, enabled by default, and a transparency adjustment
starting at **50%**. Persist subsequent choices. This default belongs to the game guide only; it does
not change the encyclopedia's appearance or determine the app-tour mask. The slider changes only the
panel's matrix/background opacity; text and controls remain fully readable. The outer Chromium canvas
and native window are transparent, so reducing the panel background reveals the application beneath.

Electron Main owns a Windows window observer behind a narrow desktop interface. Identify the visible
game window belonging to `VRChat.exe`; a running process without a usable game window is a waiting
state. Observe geometry, foreground state and minimization through Windows APIs. No client injection,
game-memory reading or automated settings editing is allowed.

With following enabled:

- Keep the guide associated with the game window as it moves or resizes, including monitor/DPI
  changes. Place instructions inside its usable bounds without covering the whole game.
- Initially place it toward the right and vertically centered. A player drag replaces that default
  with a remembered relative position within the available travel area. Retain it across app
  switches, minimize/restore, manual reopen, new game sessions and VUA restart. Clamp it to the game
  bounds and use the guide's actual current size when resizing.
- Show/restore it when the game becomes active and usable; hide it when minimized, closed or another
  application becomes active. Do not minimize or activate VRChat on the player's behalf.
- Treat interaction with the guide itself as part of its context, so clicking a control does not
  immediately hide it. Automatic showing never takes keyboard focus.
- A manual hide/close wins for the current game session. Window changes do not repeatedly bring it
  back; an explicit reopen resumes it. The encyclopedia remains available throughout.

Disabling following stops automatic game-driven showing, hiding and repositioning. Manual show/hide
controls remain, without a globally pinned window over unrelated applications. This is distinct from
the persisted transparency preference.

### Guided game steps

Present one short instruction at a time: where to open a menu, what to change, what should be visible
and what to do if it differs. Cover basic controls, audio/microphone, Personal Space and Allow
Untrusted URLs. Explain the effect of a suggested setting and let players keep their preference.
Confirmation/skip records reading progress; it does not claim VUA inspected or changed a setting.

Offer tutorial worlds by the player's requested learning language, independently of VUA's UI
language. Start with a small checked list of world identifiers, names, language and review date. Open
a selected world's official destination through the normal external-link boundary. If no suitable
entry exists, provide local instructions and ordinary world-search guidance. Do not invent a world
for every locale or require a VRChat account session in VUA.

## 7. SteamVR and the default OpenXR runtime

The encyclopedia's device/runtime topic includes `guide-devices/steamvr-openxr` in all supported
languages. SteamVR's contextual preparation entry and the PICO preparation article open this section.
It explains desktop SteamVR settings, advanced-settings visibility, the current runtime field, the
manual "Set SteamVR as OpenXR Runtime" action and confirming the result before restarting a VR game.
A legacy Developer-tab variant and missing-control recovery are described.

Desktop play does not require this setting. A streaming route that explicitly requires another
runtime follows that route's official instructions. Installation detection is not proof of the active
runtime. VUA does not silently write the system's OpenXR selection.

Source: [VIVE's official SteamVR/OpenXR setup instructions](https://developer.vive.com/resources/openxr/unity/tutorials/direct-preview/).
Static section/target and i18n consistency were checked; actual SteamVR UI and physical-route
acceptance remain separate from reading the instructions.
