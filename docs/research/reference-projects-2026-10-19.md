# Reference projects: what AMF adopts

> Updated: 2026-10-19
> Scope: lessons from five VRChat Avatar tools for AMF, its Bridge and the MCP adapter
> Status: Research, awaiting the author's review

On 2026-10-19 Claude read five public projects at their default branches and compared them with
AMF's current path: [material-first production](../protocols/amf-production-v0.1.md),
[Bridge v5](../protocols/unity-bridge-v5.md) and the [external Agent tools](../protocols/ai-tools-v0.1.md).
This record lists what AMF learns from each project and where it lands in the
[current plan](../development/current-plan.md). The ideas are reimplemented in VUA; their code is
studied and not copied.

## The restraint filter

VUA's guiding word is **restraint** (author, 2026-10-19). The core of the VRChat Avatar ecosystem
is the Japanese player and creator community, which is highly sensitive to copyright and wary of
AI. VUA serves it by keeping AI external and supervised and by supporting legitimate platforms
only; the [product boundary](../product-boundary.md#stable-product-principles) owns the rule. Every lesson below passed this filter: in-app model providers, built-in agent
harnesses and grey-market sources stay with their projects.

## Projects and licenses

| Project | What it is | License | Use in VUA |
| --- | --- | --- | --- |
| [VRCForge](https://github.com/ayyitong888/VRCForge) v1.9.1 source | AI-assisted Avatar editor: an MCP core inside the open Unity Editor, a Python gateway with a built-in agent, signed Skill packs | GPL-3.0-only | Design ideas |
| [VRCFury](https://github.com/VRCFury/VRCFury) | Non-destructive Avatar build toolkit, an alternative and complement to Modular Avatar | Source-available; its commercial license forbids bundling, redistribution and patching, and requires download from its official channel | Detection and guidance; installed by the player from the official VCC feed |
| [VRChatEditorSkill](https://github.com/XiaoboooOvO/VRChatEditorSkill) | Codex Skill that inspects and documents an Avatar project through Unity MCP | MIT | Design ideas |
| [vrchat-avatar-acceptance-workflow](https://github.com/CHOCK1337/vrchat-avatar-acceptance-workflow) 0.3.0-rc1 | Codex workflow for batch outfit and plugin assembly with a fail-closed evidence checker | MIT | Design ideas |
| [MioVRC_AssetManager](https://github.com/CokoIya/MioVRC_AssetManager) 1.8.1 | Asset manager with BOOTH, netdisk and Xianyu acquisition, an in-app AI pipeline and Unity import | GPL-3.0-only with §7 terms, or PolyForm Noncommercial 1.0.0; core modules closed since 1.8.0 | Design ideas from its earlier public design only |

## VRCForge: the Agent interface

VRCForge places its MCP server inside the open Editor and exposes about 90 fine-grained Editor
tools, each write paired with a preview. VUA's adapter lives in Electron Main and exposes
use-case tools over Bridge v5 batch runs and durable tasks. AMF adopts:

1. **Preview and apply pairs for small edits.** A preview returns a digest; the apply call
   carries the expected preview, scene and hierarchy digests and refuses drift. AMF already binds
   a whole line's plan; material tuning, menu edits and removing one outfit use the same pairing.
2. **Commit-aware results.** Results distinguish whether a mutation started, committed, persisted
   and was read back, with `rejectedBeforeMutation`, `retryable` and
   `checkpointRecoveryRequired`. An agent that lost its connection then knows whether to retry or
   to read the result first.
3. **MCP Resources and Prompts.** Run facts and Build Records become versioned, immutable
   Resources; [versioned SOPs](../north-star.md#sops) become Prompts carrying a content hash of
   their text and support files.
4. **A Skill manifest.** An allowed-tools list, permission mode, risk level, entry tool, support
   files and a workflow file with an uncertainty policy. This is the data shape for versioned SOPs
   and the [permission modes](../development/current-plan.md#7--permission-modes-and-simpler-choices).
5. **Client setup by configuration file.** Help writes the connection into the chosen AI tool's
   configuration file (Claude Code, Codex and others), with STDIO next to Streamable HTTP, as an
   option the player starts.
6. **Supervised hand adjustment.** The agent prepares a proxy object, the player drags it in the
   Editor, and the agent finalizes; any identity drift aborts. This joins the
   [Editor handoff](../development/current-plan.md#5--unity-editor-handoff) and bone attachment.
7. **Reflection-only adapters for optional tools.** Gesture Manager sets parameters in Play Mode
   to exercise switches; NDMF's parameter API counts parameter use the way MA Information does.
   The Bridge keeps no compile-time dependency on either.
8. **An A/B checkpoint timeline** for frequent tuning steps such as material brightness.

## VRCFury and Modular Avatar: assembly features

Modular Avatar builds on NDMF with one component per function (Merge Armature, Bone Proxy, Menu
Installer, Merge Animator, Blendshape Sync, Mesh Settings, Scale Adjuster, Object Toggle, Shape
Changer) and offers a real-time preview under the MIT license. VRCFury collects features in one
component: parameter compression, gesture drivers, blinking and visemes, SPS, security lock,
rich toggles, a test copy of the built Avatar, many automatic fixes and SDK workarounds. AMF keeps
assembling through MA and adopts:

9. **Richer switch options for players**: saved state, default on, an exclusive group for main
   outfits, a slider, built with MA's Object Toggle and Menu Item. Today a selection carries only
   `toggleName` and `active`.
10. **Evidence from the built Avatar.** Previews, the turntable and the menu check run on the
    NDMF-processed clone, as VRCFury's test copy and MA's manual bake produce it.
11. **Quality-check items** drawn from VRCFury's fix-ups: mixed Write Defaults, empty layers,
    uneven anchor override and bounds across renderers (applied through MA Mesh Settings),
    duplicate PhysBones, unused bones, and parameter order across PC and mobile (MA Sync
    Parameter Sequence).
12. **An early parameter budget.** The plan shows synced bits against VRChat's 256 before
    assembly; when compression is needed, VUA names VRCFury as a known package.
13. **VRCFury-carrying outfits.** Many BOOTH outfits ship VRCFury components. The intake check
    recognizes them, and the [known packages](../development/known-packages.md) table lists VRCFury
    with `guide` delivery from its official VCC feed, consistent with its license.

## VRChatEditorSkill: inspection and evidence

14. **Evidence layers.** Static source, Unity-resolved, provider preview, NDMF-built, SDK build,
    client runtime and confirmed upload. A conclusion carries its layer, and a lower layer never
    stands in for a higher one. AMF's checks and release page label their facts this way.
15. **A toolchain profile.** The intake check records which providers the Avatar and outfits
    actually use (MA, VRCFury, lilycalInventory, FaceEmo, Avatar Optimizer) as `confirmed`,
    `needs_choice`, `missing_tool` or `stale`. An installed package and a chosen tool are separate
    facts.
16. **A menu-first audit.** The result's Expressions Menu tree is traced to parameters, Animator
    layers and bound objects. The menu map joins the Build Record and the Recipe and is marked
    stale after a change.
17. **Ownership classes before a change**: scene override, shared asset, tool-owned source,
    generated output, build clone. Generated output stays untouched; shared materials are copied
    after their users are listed.
18. **Symptom playbooks** for menu missing, parameter without effect, merged Animator missing,
    wrong armature binding and wrong switch default, each with a fixed diagnostic order. They seed
    the first symptom-to-solution SOPs.
19. **Limits before optimization.** VUA proposes optimization when a hard limit is exceeded or the
    player asks; values near a limit are within it.

## vrchat-avatar-acceptance-workflow: flow and acceptance

20. **One start card.** Delivery layer, size metric, quality and performance trade-off, menu
    structure and language, and upload authority are asked once, with "use the recommendations"
    as one answer, and frozen as the line's contract. It becomes the single-page form of "ask
    before changes".
21. **Preflight before import.** From package members VUA predicts size, and after inspection each
    outfit's added triangles, material slots, texture memory, PhysBones and parameter bits, before
    assembly.
22. **Baseline-to-final attribution.** The release page shows what each outfit added, and the SDK's
    uncompressed size against its limit with a warning line at 90 %.
23. **The creator's parts menu first.** No parallel wardrobe tree; main outfits are mutually
    exclusive, accessories are free; at most 8 controls per page and 6 at the root; only visible
    labels of generated copies are localized.
24. **Body-shape variants.** When the Avatar offers shape variants such as chest sizes, each outfit
    covering them needs the creator's support or MA Blendshape Sync for every variant before
    assembly.
25. **Bounded repair.** One targeted fix and recheck per defect, then restore or isolate the
    material. Agent prompts and full access follow this rule.
26. **Upload authority per task**, private by default; an unclear remote state blocks resubmission.
27. **Scoped lessons.** A player's correction becomes a local lesson (recipe, avoid, preference,
    capability gap) keyed by input fingerprints such as BOOTH product combinations, active only
    after an observed result and suspended on a counterexample. Lessons feed Recipes and SOPs and
    stay on the computer.
28. **A closure check before "done".** A finished state requires the Build Record's evidence to
    close: candidate, platform, SDK version and report source match.

## MioVRC_AssetManager: lessons and its limits

MioVRCA solves the same newcomer path as VUA's library and AMF: one library across folders,
BOOTH purchases and shares, one-click import, assembly and pre-upload checks. Its Unity side also
talks through request and result files. The author deferred it on 2026-10-10 for product fit
([host and modules](../architecture/modules.md#deferred-external-editing-module)). Since then its
core moved to closed source (1.8.0, 2026-10-18), with its history rewritten, and its license
became GPL or noncommercial. Its copyright stance (Xianyu pages, netdisk transfer) is the decisive
difference from VUA, whose players are served through legitimate platforms; the
[licensed entry guide](../development/licensed-entry-guide.md) is VUA's answer for players in
mainland China. Its public design offers two library ideas for N5: a product's color and base
variants grouped as one material, and each material showing which work projects use it.

## Placement

| Lessons | Plan slice |
| --- | --- |
| 1–8 | [32 Agent interface refinements](../development/current-plan.md#32--agent-interface-refinements) |
| 9–13 | [33 Switch options and built-result evidence](../development/current-plan.md#33--switch-options-and-built-result-evidence) |
| 14–19 | [34 Evidence layers and toolchain profile](../development/current-plan.md#34--evidence-layers-and-toolchain-profile) |
| 20–28 | [35 Start card, preflight and attribution](../development/current-plan.md#35--start-card-preflight-and-attribution) |
| Licensed path for new players | [36 Licensed entry guide](../development/current-plan.md#36--licensed-entry-guide) and [37 Creator credits](../development/current-plan.md#37--creator-credits) |
