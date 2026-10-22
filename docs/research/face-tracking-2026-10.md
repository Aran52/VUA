# Avatar face tracking: October 2026 research

> Reviewed: 2026-10-20; eye-only repository/license follow-up 2026-10-21
> Status: Research; product descriptions are not runtime verification
> Implementation proposal: [Face tracking adaptation](../development/face-tracking.md)

## Findings

An Avatar's geometry, its animation/parameter wiring and the headset's tracking transport are
three separate compatibility questions. Installing VRCFaceTracking (VRCFT) supplies neither
geometry nor Avatar animation. A shape's spelling is evidence of a possible mapping, not proof
that its deformation is useful. A matching controller can still address the wrong renderer.

The present plan's Meiyun case is a useful acceptance case, but a menu invocation is too narrow
as the abstraction. Model-specific patches, reusable templates, existing tracking shapes,
generated mappings and native eye tracking need different actions. They can share inspection,
review, recovery and evidence.

There is also a new direction to preserve: VRChat's
[24 September 2026 developer update](https://ask.vrchat.com/t/developer-update-24-september-2026/48972)
announced native high-fidelity tracking, networked Unified Expressions and a dedicated OSC blob.
This was described as forthcoming public testing. The
[published SDK release index](https://creators.vrchat.com/releases/) reviewed here still lists
3.10.5 as the latest stable release. Do not silently upgrade the production toolchain or assume
the announcement is a capability of SDK 3.10.5. Detect the installed SDK surface and admit a
future native adapter separately.

Links were consulted on **2026-10-20**, except the EyeTrackVR application/module repository and
license follow-up explicitly dated **2026-10-21** below. Dynamic pages and default Git branches
can change. Versions below identify the evidence where available; unknown behavior stays
unknown. No purchased package, private Discord conversation or real facial sample was read.

## Implementation families

### 1. Model-specific geometry patches

| Representative | Requirements and installation | Automation and original files | License / delivery |
| --- | --- | --- | --- |
| [yjworks Meiyun add-on](https://booth.pm/ja/items/8123435), for [Meiyun](https://booth.pm/ja/items/4431242) | The product names MA, VRCFT v5/Unified Expressions, Jerry's templates and Hash's distribution tools. It reports testing with Pico 4 Pro and Unity 2022.3.22f1. | The author reports an Editor-menu patcher. Exact menu, supplied package version, output mesh and handling of an already dressed Avatar need local verification. No batch claim follows from the product page. | Personal use; no redistribution/resale. Use the author's already owned copy. |
| [Jerry's Karin add-on](https://adjerry91.booth.pm/items/4723914) | Avatar v1.11, VRCFT v5+, custom UE shapes, VRCFury; instructions accompany the package. | Requires the original base FBX; customized models require shape transfer. The page records a migration to Hash patching. No supported headless interface established. | Paid model/add-on licenses remain separate; obtain from the creator, do not bundle. |
| [Fermata KUMALY](https://booth.pm/ja/items/7914053) | Specific base versions; MA/NDMF; two-stage patch and prefab installation, with WD On assumptions. Japanese and Korean instructions describe HDiffPatch. | Original FBX bytes and paths matter even if the visible version number is unchanged. STEP1 opens a window; STEP2 can add FT to another root. One patch per project is requested. | Add-on redistribution is prohibited; commission use requires both parties to own it. The included patch utility's license does not license the Avatar. |

[Hash EditDistributionTools](https://github.com/HashEdits/EditDistributionTools) is a real
underlying distribution mechanism, not an Avatar or universal conversion library. It builds
differences between an original and modified model so creators can distribute their changes.
Generated patchers have creator-specific names and paths. A common tool ancestry does not
establish the behavior or license of each generated patcher.

Free examples exist, including [ocian's Manuka add-on](https://booth.pm/en/items/7689034)
(listed at JPY 0; requires Manuka v1.02, MA and Jerry's templates). Free price does not waive its
no-redistribution terms or provide the separately required base Avatar.

### 2. Reusable animation and parameter templates

| Representative | Avatar prerequisites / installation | Batch / original files | License |
| --- | --- | --- | --- |
| [Adjerry91 VRCFaceTracking Templates](https://github.com/Adjerry91/VRCFaceTracking-Templates), official feed reviewed at 7.0.5 | Existing case-sensitive UE, ARKit or SRanipal shapes; choose MA or VRCFury prefab and shape convention. Animations normally address `Body`. Gesture conflicts need explicit handling. | Prefab insertion can be deterministic after mapping/target review. It does not create missing shapes. MA variants do not provide the VRCFury renderer-path rewrite. No original FBX is needed when suitable shapes already exist in the mesh. | [MIT](https://github.com/Adjerry91/VRCFaceTracking-Templates/blob/main/LICENSE.md); preserve notices. README additionally requests visible product credit and cautions against embedding detached controllers. |
| [Pawlygon VRC-Facetracking](https://github.com/PawlygonStudio/VRC-Facetracking) / [official listing](https://vcc.pawlygon.net/) | The 1.x OSC templates target UE shapes, with MA/VRCFury variants. Current default-branch documentation also describes a native-SDK generation. Select by exact version and SDK capability. | Controller merge can run at build time; interactive legacy/native-conflict prompts mean latest-branch code cannot be assumed batch-safe. No mesh generation is implied. | [MIT](https://github.com/PawlygonStudio/VRC-Facetracking/blob/main/LICENSE.md); preserve notices and review the chosen release's dependencies. |

The Pawlygon rendered repository page and cached raw README differed during research. This is
one more reason to pin a release and inspect its manifest rather than resolve “latest FT”.

[VRCFury's license](https://github.com/VRCFury/VRCFury/blob/main/LICENSE.md) is source-available
with separate license options. Its commercial option requires direct end-user acquisition from
an official channel and prohibits redistributing or modifying VRCFury. VUA should keep
`com.vrcfury.vrcfury` on **guide** delivery and point to its
[official installation instructions](https://vrcfury.com/download/). Do not apply a template's
MIT license to its dependencies.

### 3. Avatars or reference models with shapes already supplied

| Representative | What is supplied / what remains | Batch / original files | License |
| --- | --- | --- | --- |
| [Coby by GabSith](https://gabsith.gumroad.com/l/Coby) | A free base tier advertises native-to-the-model FT; its instructions require VRCFury and Pawlygon's template. “Native” in a product description does not necessarily mean VRChat's new SDK feature. | Inspect the supplied prefab and exact dependencies. A populated shape set may still need controller/parameter wiring. | Creator/store terms; no repository redistribution assumed. |
| [hinzka's 52blendshapes-for-VRoid-face](https://github.com/hinzka/52blendshapes-for-VRoid-face) | Reference VRM models and Perfect Sync expression data. Useful to understand ARKit-like geometry; not a ready-made VRChat assembly guarantee. | VRM intake/conversion and VRChat wiring are additional work. Matching topology matters when transferring data. | The repository's model-specific conditions apply; it is not an unrestricted license for arbitrary source models. |

An SDK descriptor's eye bones, blinking and visemes are useful existing capabilities. Neither
MMD expressions nor `vrc.v_*` visemes alone prove a complete FT set. Names must be evaluated per
mesh: a union across unrelated meshes can create a misleading “complete” count.

### 4. Geometry generation or shape-data transfer

| Representative | Input and generated output | Batch / original files | License |
| --- | --- | --- | --- |
| [HANA_Tool v4](https://booth.pm/ja/items/2604269) / [v5](https://booth.pm/ja/items/4607328) | Reads/writes shape data for compatible model topology; VRoid data is supplied for particular conventions. v4's page warns about VRM 1.0 incompatibility. | Interactive Unity tooling; no documented batch API verified. Vertex/topology and model-version constraints prevent treating it as a universal generator. | Paid tools, separate model and distributed shape-data conditions; guide. |
| [PerfectSync Maker Desktop](https://booth.pm/ko/items/8915492) | Vendor advertises generating 52 expressions from several model formats, with a free trial and export/transfer workflows. | External Windows application; no supported CLI established. Output quality, facial extremes and topology need review. | Store-proprietary; guide, trial conditions apply. No purchase requested for this slice. |

These are distinct from animation-only mapping tools:
[VRCFT-SettingTool](https://booth.pm/ja/items/7840725) explicitly says it does **not** generate
shapes and can choose the wrong `Body` mesh; [HatoTools' setup tool](https://booth.pm/ko/items/8105725)
advertises automatic name mapping and controller creation. Both require inspection and human
review of the mapping. Name similarity is insufficient authority to sculpt or overwrite geometry.

### 5. Eye-only tracking

| Representative | Avatar-side requirement | Installation / batch | License |
| --- | --- | --- | --- |
| [VRChat native OSC eye tracking](https://docs.vrchat.com/docs/osc-eye-tracking) | Descriptor eye-look / eyelid configuration for the intended effect. `/tracking/eye/*` addresses are separate from Avatar expression parameters. | A supported sender supplies gaze/eyelid data. Configuring an Avatar descriptor can be deterministic; physical calibration cannot. No extra FT shape library or original FBX is inherently required. | Official interface; sender and Avatar licenses apply independently. |
| [EyeTrackVR](https://github.com/EyeTrackVR/EyeTrackVR), rechecked 2026-10-21 | Appropriate eye configuration and hardware/software calibration; it supplies eye tracking, not missing mouth geometry. | External application/device workflow; use its documented setup and output. No original FBX is inherently needed. No VUA device-module writes. | [Babble Software Distribution License 1.0](https://github.com/EyeTrackVR/EyeTrackVR/blob/main/LICENSE): custom non-commercial terms, same-license/public-source conditions for derivatives and retained notices. Guide only; do not classify it as permissive open source. |
| [ETVRTrackingModule](https://github.com/EyeTrackVR/ETVRTrackingModule), reviewed 2026-10-21 | Translates EyeTrackVR eye OSC into VRCFT expressions. A parameter-template Avatar needs matching eye shapes/controller wiring; native-eye output is a different receiver route. | Install through VRCFT's supported module workflow and follow the version-specific sender configuration. No Unity batch installation or original FBX requirement for the module itself. | Module [MIT license](https://github.com/EyeTrackVR/ETVRTrackingModule/blob/main/LICENSE): preserve its notice. This does not replace the EyeTrackVR application's separate terms. |

VRCFT eye parameters are another route within the template family. Native OSC eyes support
different capabilities from per-eye Avatar parameters; for example the documented
`EyesClosedAmount` input controls both eyelids together. Zero FT expression parameters can be
valid for native eyes, and a disconnected sender can fall back to automatic eye behavior.

## Parameters, animation and evidence

[VRCFT's parameter documentation](https://docs.vrcft.io/docs/tutorial-avatars/tutorial-avatars-extras/parameters)
allows a namespace before `v2/` and both float and binary representations. Inspection must not
require names to begin exactly with `v2/`, confuse binary channels with independent expressions,
or confuse Animator parameters with synced Expression Parameters.

[Unified Expressions](https://docs.vrcft.io/docs/tutorial-avatars/tutorial-avatars-extras/unified-blendshapes)
is a device-independent expression vocabulary. Its
[standard comparison](https://docs.vrcft.io/docs/tutorial-avatars/tutorial-avatars-extras/compatibility/overview)
and [SRanipal mapping](https://docs.vrcft.io/docs/tutorial-avatars/tutorial-avatars-extras/compatibility/vive-sranipal)
include combinations and conditional relationships. Renaming ARKit 52 or SRanipal shapes is
not generally equivalent to converting their deformation semantics.

[VRChat expression parameters](https://creators.vrchat.com/avatars/expression-menu-and-controls/)
have a 256-bit sync budget: synced Bool costs 1 bit, Int/Float 8; local parameters are separate.
Report the descriptor asset's observed cost with its evidence layer. MA/VRCFury build-time
merges, compression and conflicts can change the final cost; do not present a source count as
the built Avatar's budget. Native high-fidelity FT must eventually have its own accounting.

Inspect animation bindings and controller integration separately from geometry. Common failures
include `Body` case/path changes, shape keys on an eye mesh instead of the face mesh, old
SRanipal controls, conflicting gesture/idle/blink layers, Write Defaults assumptions, duplicate
FT prefabs, tongue emulation mismatches, and stale OSC Avatar configuration after an upload.
The product and template sources above document these mechanisms; none establishes that all
avatars have the same failure.

## Patcher behavior and recovery

| Form | Likely interaction and failure point | VUA boundary |
| --- | --- | --- |
| Editor menu opening an EditorWindow | Selection, variant, original FBX, confirmation dialogs, then a separate Patch button | Open a supervised interactive session; menu return means only dispatch. |
| Direct Editor command | May write assets, import files, trigger reload or schedule delayed callbacks | Batch only after a version-specific adapter establishes completion and postconditions. No such third-party adapter is admitted by this first slice. |
| External executable / HDiffPatch | Hash mismatch, antivirus refusal, output path, child process and exit status; may be launched by an Editor script | No arbitrary executable/argument API. An external-only patcher remains a manual preparation route. |
| Raw FBX difference | Exact original bytes may be necessary; an edited FBX or silent re-release can fail | Keep source identity/content hash. Patch only a copied work-project input. |
| Build-time mesh/controller transform | Source scene may look unchanged; generated clone carries the result | Require later NDMF/SDK-built evidence rather than claiming absence or success from the source alone. |

Unity Undo is not a filesystem transaction. It need not undo FBX replacements, generated files,
external processes or importer changes. Protect `Assets`, `Packages` and `ProjectSettings`
before running a patch; stop the dedicated Unity process before restoring. A snapshot cannot
undo arbitrary writes outside that project. Unity Editor scripts run with the user's OS
authority, including during import. Work-project confinement is the boundary of VUA's own
writes, **not a sandbox for untrusted third-party code**.

## Language-community observations

These are patterns in the reviewed sources, not measured market shares.

- **Japanese:** BOOTH model-specific patches, original-version requirements and MA prefab
  installation recur in the Meiyun/Karin/KUMALY listings. Japanese users also have topology
  transfer tools such as HANA and mapping tools such as VRCFT-SettingTool. “Non-destructive”
  prefab integration does not mean the earlier FBX patch was non-mutating.
- **English:** Jerry's/Pawlygon's author documentation separates shapes from reusable animation
  templates. Renderer-path rewrites, template variants and gesture suppression are explicit
  setup concerns. Free add-ons may still require a paid base.
- **Korean:** Fermata's native Korean KUMALY/Kipfel instructions and
  [ToiShop's Milltina instructions](https://booth.pm/zh-tw/items/6561516) describe a menu window,
  a patch action and a separate prefab insertion. They warn about silent base-file updates,
  WD settings and limited tongue directions. The store language selector does not identify
  the language of the creator's actual instructions.
- **Chinese:** [regzo2's PICO module](https://github.com/regzo2/PicoStreamingAssistantFTUDP)
  and [lonelyicer's Chinese instructions](https://github.com/lonelyicer/VRCFTPicoModule/blob/master/README.zh.md)
  emphasize sender/module protocol selection. Newer
  [UnifiedPicoModule documentation](https://github.com/WolalaQAQ/UnifiedPicoModule/blob/main/README_zh.md)
  distinguishes initialization from received data and warns about competing modules/ports.
  These are alternatives to investigate, not silently install over the existing setup.

For the author's **Pico 4 Pro**, use the current
[VRCFT hardware guide](https://docs.vrcft.io/docs/hardware/vr/pico/pico4pe) and record the actual
firmware, PICO Connect and module versions. Its legacy-protocol workaround and microphone note
are version-dependent; VUA should not change hidden PICO configuration. Eye and lip permission,
live incoming data, Avatar-side wiring and VRChat OSC each need their own check. A module saying
“initialized” is not enough.

## Acquisition decision

**Purchase request: none.** The author owns the target model/add-on. Synthetic fixtures cover
the transaction and facts; public sources establish the design. Free model/add-on trials would
not replace verification with the author's precise Meiyun bytes and Pico 4 Pro. No free or paid
model content is needed in the repository.
