---
catalog_schema: "vua.tool-entry/v3"
id: "external.avatar-optimizer"
boundary: "external"
status: "experimental"
risk: "medium"
risk_rule: "vua.risk-gate/v1"
delivery: "unscheduled"
maintainer: "external-upstream"
distribution: "external-connection"
platforms: ["windows"]
capabilities: ["external.unity-package.detect", "avatar.optimize.plan", "avatar.optimize.validate"]
---

# Avatar Optimizer


[AAO: Avatar Optimizer](https://vpm.anatawa12.com/avatar-optimizer/) (`com.anatawa12.avatar-optimizer`,
MIT) runs as an independent upstream Unity package on NDMF at the Unity build boundary and receives
no VUA internal capability.

In [AMF production](../../protocols/amf-production-v0.1.md), Avatar Optimizer is off by default.
When the player explicitly enables it (`amf.production.setOptimizer`) or reproduces an
[`amf-recipe` v0.6](../../protocols/amf-recipe-v0.6.md) Recipe that recorded `adjustments.optimizer:
true`, VUA verifies VRChat SDK (`>= 3.7.0, < 3.11.0`) and NDMF (`>= 1.8.0, < 2.0.0`) compatibility,
installs pinned release `1.9.21` from the official VPM feed (`https://vpm.anatawa12.com/vpm.json`),
and adds or removes `Anatawa12.AvatarOptimizer.TraceAndOptimize` with default settings on the active
VUA-owned Avatar root. VUA never touches private fields on `TraceAndOptimize` and never vendors or
forks upstream optimizer code.

Local before/after measurement invokes NDMF's public `AvatarProcessor.ManualProcessAvatar` on
temporary unpacked clones inside the work scene, reads SDK performance and footprint metrics on
both processed copies, cleans up temporary objects and `Assets/ZZZ_GeneratedAssets` entries, and
labels the result `localMeasurementOnly: true` so it is never presented as an official VRChat upload
verdict. Every toggle writes a revision-backed Build Record and remains rollbackable; lines carrying
face-tracking components or parameters surface `optimizerWarnings: ["osc_gimmick_possible"]`.
Real-material acceptance requires verifying an Avatar + outfit + toggle/PhysBone/BlendShape setup on
Unity `2022.3.22f1` before promoting `status` to `supported`.
