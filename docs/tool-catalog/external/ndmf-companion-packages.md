---
catalog_schema: "vua.tool-entry/v3"
id: "external.ndmf-companion-packages"
boundary: "external"
status: "experimental"
risk: "low"
risk_rule: "vua.risk-gate/v1"
delivery: "unscheduled"
maintainer: "external-upstream"
distribution: "external-connection"
platforms: ["windows"]
capabilities: ["external.unity-package.detect", "external.vpm-registry.read"]
---

# NDMF Companion & VRM Avatar Packages

Non-destructive Unity packages running on NDMF (`nadena.dev.ndmf`) and official VRM conversion
tooling resolved from upstream VPM feeds without bundling or forking upstream archives:

- **[LilyCalInventory](https://lilxyzw.github.io/LilyCalInventory/)** (`jp.lilxyzw.lilycalinventory`,
  MIT, feed `https://lilxyzw.github.io/vpm-repos/vpm.json`) — non-destructive inventory, outfit and
  item toggle builder on NDMF.
- **[lilAvatarUtils](https://lilxyzw.github.io/lilAvatarUtils/)** (`jp.lilxyzw.avatar-utils`, MIT,
  feed `https://lilxyzw.github.io/vpm-repos/vpm.json`) — Unity Editor avatar lighting, material and
  texture inspection utility.
- **[TexTransTool](https://ttt.rs64.net/)** (`net.rs64.tex-trans-tool`, MIT, feed
  `https://vpm.rs64.net/vpm.json`) — non-destructive build-time decal, atlas, and color-modification
  toolkit on NDMF.
- **[FaceEmo](https://suzuryg.github.io/face-emo/)** (`jp.suzuryg.face-emo`, MIT, feed
  `https://suzuryg.github.io/vpm-repos/vpm.json`) — non-destructive facial expression menu and
  animation generator.
- **[FloorAdjuster](https://github.com/Narazaka/FloorAdjuster)** (`net.narazaka.vrchat.floor-adjuster`,
  Zlib, feed `https://vpm.narazaka.net/index.json`) — NDMF build-time avatar floor height and
  viewpoint alignment component.
- **[VRCQuestTools](https://kurotu.github.io/VRCQuestTools/)** (`com.github.kurotu.vrc-quest-tools`,
  MIT, feed `https://kurotu.github.io/vpm-repos/vpm.json`) — NDMF-capable PC-to-Android/Quest/PICO
  avatar conversion toolkit.
- **[VRM Converter for VRChat](https://docs.esperecyan.net/vrm-converter-for-vrchat/)**
  (`jp.pokemori.vrm-converter-for-vrchat`, MIT, feed
  `https://esperecyan.github.io/VRMConverterForVRChat/registry.json`, `guide` delivery) — upstream
  converter workflow referenced when players inspect `.vrm` avatars for VRChat use.

VUA registers the official VPM feeds in the Control Center package repository quick-add list and in
[known-packages](../../development/known-packages.md) so Recipe-pinned projects and explicit user
additions resolve verified upstream releases inside isolated VUA work projects. VUA never installs
these packages heuristically from asset names and never writes to external ALCOM/VCC projects.
