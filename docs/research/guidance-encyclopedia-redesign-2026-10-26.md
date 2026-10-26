# Guidance choices and the VRChat encyclopedia

> Updated: 2026-10-26
> Scope: first-run guidance, VUA's own instructions, and a broader VRChat knowledge surface
> Status: Draft research and recommendation; the new flows and encyclopedia are not implemented

The [guidance architecture](../architecture/guidance.md) remains the implementation baseline until
this proposal is reviewed. This draft does not change a navigation, storage or browser contract,
and it does not fix the production-entry or source-detail defects found in the local walkthrough.
Those defects must not be hidden by publishing an imaginary working Avatar flow.

## Current structure and the separation to preserve

A new profile currently receives a ten-step host tour followed by the independent goal wizard.
The Avatar material branch opens Warehouse and ends there. Help contains that wizard, the feature
tour, the encyclopedia, the game assistant and external AI tools. The encyclopedia mixes hardware,
play, safety, devices and a VUA tutorial. Its horizontal topic chips are a navigation limit, not a
knowledge model.

Three teaching contexts need different content and behavior:

| Context | Question answered | Form | Must not become |
| --- | --- | --- | --- |
| Game guide | What do I do in VRChat now? | One short instruction beside the Windows VRChat window, with a verified expected result and a link to more detail | A long article, a claim that VUA inspected or changed a game setting, or a delivered native VR overlay |
| VRChat encyclopedia | What does this VRChat concept mean, or how do I understand and solve this problem? | Searchable, source-backed articles, task paths, glossary and troubleshooting | A VUA manual, a scraped mirror, or an unreviewed AI answer feed |
| VUA guide | How do I use this VUA page and recover its actual workflow? | Task-oriented VUA instructions, contextual coach marks and the optional feature tour | A VRChat concept encyclopedia or a guided path through unavailable functionality |

Content may be shared by stable article/section IDs, but each context keeps its own presentation and
progress. Opening an article never starts a tour, a production task, installation, authentication
or game following. A native in-headset VUA overlay remains unscheduled.

## The three onboarding options

### A: remove the wizard and simplify every page

Retain only a first-use feature introduction, expose one or two principal actions per page, put
complexity elsewhere and default production to full automation.

**Value:** the least upfront questioning; a player sees the actual application rather than a
questionnaire. Material-first production benefits from a small number of goal-oriented actions.

**Cost:** the current ten-step host tour still teaches locations before a player has a task. A
universal two-button limit can hide state, recovery and object actions rather than simplify them.
Automation cannot infer an uncertain asset role, target Avatar, project ownership, installation
consent, account authentication, destructive change or upload confirmation. Full automation also
cannot repair a blocked production entry by presentation alone.

**Verdict:** adopt its page simplification, not wizard removal as a standalone solution. Prefer one
primary action plus contextual secondary actions over an arbitrary global button quota. Automatic
mode may inspect and prepare a plan by default; its explicit review/approval and recovery remain.

### B: expand the wizard into a detailed preferences interview

Ask the player's goals and detailed page preferences, including production automation, before use.

**Value:** can establish useful defaults for a player who already knows the tools, and can make
later preferences discoverable.

**Cost:** the target novice cannot reliably answer questions about MA, Recipes, targets or
production automation before seeing materials and real findings. Early answers become stale and
produce more branches to test. A saved preference must not masquerade as a fresh environment,
compatibility or readiness observation. A long wizard also optimizes completion of the wizard
rather than the player's result.

**Verdict:** do not use a large upfront interview. Preserve useful choices as optional task-level
and page-level preferences with a safe default and a way to change them later.

### C: split first-run guidance from contextual guidance

Ask only the language on first launch. When a module is first opened, offer non-blocking setup and
teaching in context.

**Value:** questions arrive when the player can understand them, with the real object and controls
visible. It supports different goals without permanently hiding other modules, and lets experienced
players skip familiar explanations.

**Cost:** a popup on every page would recreate the long wizard in fragments. Repeated prompts can
obscure controls, compete for focus and annoy players. Preferences still need versioning and replay.

**Verdict:** recommended, with strict limits. A module gets at most one small entry invitation and
at most three coach marks when needed. They are non-modal, anchored to existing controls, dismissible
with Escape, and offer **Later** and **Do not show again**. They never use a blocking backdrop or
force a configuration choice. Help can replay them. Only a missing choice that changes the actual
task outcome becomes a task-level question.

### Proposed combination

Use **C as the guidance model and A as the page-simplification constraint**. Keep B's detailed
options in advanced settings or the current task, not in first-run.

1. A new profile asks for language, then lands on the unchanged Home directory. The feature tour is
   an optional invitation, not a mandatory ten-step preamble. Existing profiles are not forced to
   replay anything.
2. Play asks about desktop versus headset, and asks for the model/connection only when required by
   the real route. Do not ask every player to configure Avatar production.
3. Avatar work starts from the material or existing project. Local files, BOOTH purchases and an
   existing Unity project are different entry paths. Ask for missing asset role or target only when
   observed facts cannot determine them; do not guess from a filename.
4. Production defaults to a safe automatic path with a visible Inspect → Plan → Confirm → Execute →
   Validate sequence. Advanced automation and package controls remain available without hiding the
   current object, changes, blocker or recovery. No automatic account, purchase, authorization or
   upload.
5. Store coach-mark dismissal separately from task facts and environment observations. Version the
   introduction by stable module/anchor identity, preserve reading position and never reset a task
   when teaching is replayed.

The evaluation scenarios are first desktop play, first PICO play, BOOTH-owned clothing, local Avatar
and clothing without BOOTH sign-in, an existing VCC/ALCOM project, and recovery after a failed run.
Measure whether the player reaches a real next action and can recover, not whether a wizard was
completed.

## Encyclopedia information architecture

The encyclopedia is a **VRChat knowledge surface**, not VUA's product manual. Its home offers search
and task entry points before a large category tree. Suggested searches and filters never infer a
player's account rank, hardware, location or skill.

| Area | Local knowledge to provide |
| --- | --- |
| Start playing | Hardware and play-area recognition, desktop/standalone/PC-streaming differences, accounts, installation concepts, controls, microphone, comfort and accessibility |
| Worlds and social safety | Worlds, instances and joining, personal space, mute/block/hide/show, reporting, trust ranks and Shield Levels, media/URL choices, and the distinction between account trust and Avatar performance |
| Using Avatars | Finding and changing Avatars, favorites, public/private visibility and cloning, fallbacks, performance display filters, platform compatibility, and why another player may see a different Avatar |
| Avatar foundations | Unity/VCC/SDK roles, `.unitypackage`, FBX, Prefab, materials/textures/shaders, rigs and Humanoid, Avatar Descriptor, viewpoint, visemes, and the basic Animator → parameters → states → clips model |
| Clothing and Avatar features | Asset instructions and dependencies, non-destructive outfit assembly, Modular Avatar concepts and simple outfit/toggle examples, materials, Expressions Menu/Parameters, Playable Layers, PhysBones, constraints, contacts, OSC and tracking relationships |
| Platforms and optimization | PC versus mobile/Android constraints, texture/mesh/material costs, performance ranks and their limits, per-platform overrides, impostors/fallbacks, and testable optimization steps rather than universal numeric promises |
| Build, publish and troubleshoot | Full-account and New User upload eligibility, SDK Builder validation, Build & Test versus Build & Publish, content tags and visibility, local testing, symptom-led errors, recovery and official support |
| Worlds and external creation | World components and publishing concepts, Udon/UdonSharp, networking and persistence, OSC and Avatar ↔ World interaction; curated official paths for deeper creation rather than a claim that VUA automates World development |

The last area is deliberately a reference path, not an excuse to import hundreds of World/API pages
into the first release. Core depth follows VUA's play and Avatar audience. General concepts remain
available without implying corresponding VUA capabilities.

### Entry paths

The home should expose questions such as **I am new to VRChat**, **I want to wear an outfit I
obtained**, **Others cannot see my Avatar**, **My clothing does not follow or looks pink**, **I want
to build and test**, and **I want to understand a term**. Each path leads to a short sequence of
articles, not an all-topic dump.

The first Avatar articles are:

1. Avatar, clothing, accessories and target: what each object is and why compatibility needs evidence.
2. FBX, Prefab, `.unitypackage` and VPM: what can be imported where; a file extension is not a
   compatibility verdict.
3. From an obtained outfit to a local test: prerequisites, source instructions, non-destructive
   assembly, visible checks and recovery, with VUA operations kept in the separate VUA guide.
4. Clothing does not follow, is misplaced or clips: distinguish assembly, armature, pose, blend-shape
   and PhysBone causes rather than offering one universal fix.
5. Pink or missing materials: distinguish missing assets, shader/package dependencies and platform
   restrictions; do not promise that installing one shader repairs every asset.
6. Trust rank versus performance rank: upload eligibility, social display controls, static performance
   estimates and fallback visibility are separate systems.
7. PC and mobile: why a separate compatible build is needed, what cannot carry over and how overrides
   or fallbacks affect what another player sees.
8. Build & Test versus Build & Publish: local visibility, upload eligibility, validation, content
   warnings and visibility choices.
9. Modular Avatar: what build-time processing means, a simple outfit and object toggle, and where to
   read the current official component reference.
10. Animator, Expressions and motion: the minimum mental model needed before advanced toggles,
    facial expressions, OSC or tracking.

## Reading interface

- Put search, a small set of task entries and topic navigation in the encyclopedia page, not in the
  global wizard. Use a sticky topic tree on wide windows and an accessible disclosure on narrow
  windows instead of the current horizontal chip strip.
- Give the article a readable measure and its own scroll container. A local table of contents,
  prerequisites, expected results and source/revision information stay reachable. Avoid nested cards
  and decorative artwork behind text.
- Search titles, aliases, summaries and body text locally. Filters can include topic, audience,
  platform and article type. Do not require sign-in, AMF, a remote AI model or a remote search service.
- Keep existing deep links and bookmarks through an explicit legacy topic/section mapping. Browser
  Back/Forward traverses topic and section selections without triggering work.
- External sources use the existing isolated browsing boundary and return to the same reading
  position. They do not silently replace the local article or gain application privileges.
- The game guide links to an article section; the VUA guide links to a real application page. These
  are explicit navigation actions, not business execution.

## Content model and authoring rules

Do not continue growing the monolithic language tables with whole articles. Use a typed local article
catalog with stable IDs, aliases and structured sections rendered by a small React reader. No MDX or
remote-document dependency is assumed by this proposal.

Each article records:

- `id`, `kind` (concept, procedure, reference or troubleshooting), `topics`, `audience`, `platforms`,
  `aliases` and prerequisite article IDs;
- localized title, summary and structured body, with explicit translation/fallback status;
- `sourceRefs`: exact URL, publisher, source kind, checked date and applicable version;
- independent `reviewStatus`, factual-review date and any real-machine verification, rather than
  treating a source's self-declared confidence as VUA verification;
- license/attribution for reused text and media, and deprecation/replacement information.

A procedure leads with the result and prerequisites. Each step has one action, an expected visible
result and a branch for when that result is absent. A troubleshooting article starts with observable
symptoms, offers the smallest safe checks and explains what evidence distinguishes the causes.
Version numbers, limits and software UI labels are dated, not universal truths. Do not invent time
estimates, confidence scores, success claims or screenshots.

## Source review and licensing

Sources were read on 2026-10-26. Their role in the new catalog is:

| Source | What to reuse | Boundary |
| --- | --- | --- |
| [VRChat documentation hub](https://docs.vrchat.com/) and [Safety and Trust](https://docs.vrchat.com/docs/vrchat-safety-and-trust-system) | Player controls, safety, trust, platform systems and links to current support | Version-sensitive instructions need direct review; AFK, mass-friending and bulk low-effort uploads are not rank-up instructions |
| [VRChat Creator documentation](https://creators.vrchat.com/), [Creating Your First Avatar](https://creators.vrchat.com/avatars/creating-your-first-avatar/) and [Performance Ranks](https://creators.vrchat.com/avatars/avatar-performance-ranking-system/) | SDK concepts, Avatar structure, upload prerequisites, local testing, optimization and current limits | Performance rank is not an FPS guarantee; Unity upload and avatar-creation-tool routes are different |
| [Official VRChat Wiki](https://wiki.vrchat.com/wiki/Main_Page) | Community-maintained terminology, play and ecosystem references | The direct read in this research session reached a security-verification page. Its article content was not reviewed here and is not claimed as verified |
| [VRChat Agent Creator Library](https://github.com/RhineLab-magellan/VRChat-Agent-Creator-Library) | Domain taxonomy, symptom-led diagnosis, progressive teaching, aliases and machine-readable source metadata | Community interpretation, not an authority to copy wholesale; its reviewed `modular-avatar` entry is marked `source_type: community`, `confidence: Medium`, with a 2026-08-13 review date |
| [Modular Avatar documentation](https://modular-avatar.nadena.dev/docs/intro) | Current installation, tutorials, component reference and build-time behavior | The tool's own documentation is authoritative for its current behavior; separate tool licensing and branding from article licensing |
| Unity, device/tool manufacturer documentation and reviewed Japanese/Chinese beginner tutorials | Unity fundamentals, physical routes, alternative explanations, illustrations and terminology | Each article and asset needs an individual factual and rights review; community preference never overrides current official constraints |

The Agent Creator Library's root license is MIT, requiring preservation of its copyright and license
for covered substantial reuse. That does not establish rights to all linked third-party text or
media, nor verify its technical claims. Its repository states 414 knowledge pages at the time read;
the count is a repository description, not a VUA content target. Use selected original ideas and
independently verified explanations, with attribution where required. Do not ship its training data
or scripts, or add an automatically generated answer feed.

Until official text/media rights are established, write original concise explanations, use original
figures and link to official sources. Do not assume that public availability permits bundling or
translation of a page or screenshot. Record attribution and checked dates in the catalog, not in
unversioned chat output alone.

## Delivery order and acceptance

1. Resolve the observed Avatar production-entry and source-fact blockers before publishing a VUA
   end-to-end clothing procedure. Leave the current defects recorded; this draft changes none.
2. Introduce the typed article catalog and split `guide-vua` into the VUA guide. Preserve useful
   hardware/play articles, stable targets, reading recovery and browser behavior.
3. Implement encyclopedia search/navigation and the first reviewed task paths, including distinction
   between trust and performance. Link unavailable workflows honestly rather than inventing buttons.
4. Replace mandatory first-run sequencing with language-first and bounded contextual invitations.
   Simplify actual page actions and add safe automatic defaults separately from the content work.
5. Review all five UI languages, keyboard/focus behavior, narrow windows, reduced motion and source
   attribution. Exercise the real game-window guide separately; desktop DOM checks are not physical
   headset or native-overlay acceptance.

Acceptance means a player can find a relevant answer, distinguish fact from community interpretation,
complete a checked step or reach an honest handoff, and return to the same task without losing work.
It does not mean the encyclopedia has a target page count or that a VUA capability exists merely
because an article describes it elsewhere.
