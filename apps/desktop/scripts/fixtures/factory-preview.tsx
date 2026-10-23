// Synthetic preview of the AMF base map and workshop for visual review; no production claims.
import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GatewayProvider } from "../../src/renderer/gateway/GatewayProvider.tsx";
import { emptyGateway } from "../../src/renderer/gateway/empty-gateway.ts";
import { FactoryMap, type Room } from "../../src/renderer/features/amf/FactoryMap.tsx";
import { AmfRoom } from "../../src/renderer/features/amf/AmfRoom.tsx";
import { AmfProductionPage } from "../../src/renderer/features/amf/AmfProductionPage.tsx";
import { MaterialsStartDialog } from "../../src/renderer/features/amf/MaterialsStartDialog.tsx";
import { controlPreview } from "./control-center-preview.tsx";
import { AmfProductionError, type AmfProductionPort, type AmfProductionRun, type LibraryBrowserPort } from "../../src/renderer/gateway/index.ts";
import { ReleasePreview, releasePorts, vrchatPreviewPort } from "./release-preview.tsx";
import { NetworkPanel } from "../../src/renderer/features/deployer/NetworkPanel.tsx";
import "../../src/renderer/features/deployer/website-tests.css";
import "../../src/renderer/features/home/home.css";
import { VrchatSignInSection } from "../../src/renderer/features/settings/vrchat-sign-in.tsx";
import { UnityLicenseGuide } from "../../src/renderer/features/settings/UnityLicenseGuide.tsx";
import { ProductTerms } from "../../src/renderer/features/warehouse/ProductTerms.tsx";
import { termsPreviewPort } from "./terms-preview.tsx";
import "../../src/renderer/features/warehouse/warehouse.css";
import type { NetworkIntent, NetworkReport, UnityLicenseSnapshotV01, WebsiteObservation } from "@vua/contracts";
import { REGIONAL_REFERENCES } from "../../src/renderer/features/deployer/regional-reference.ts";
import "../../src/renderer/features/deployer/deployer.css";
import { Card } from "../../src/renderer/components/primitives/Card.tsx";
import { Button } from "../../src/renderer/components/primitives/Button.tsx";
import { strings } from "../../src/renderer/i18n/index.ts";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";
import "../../src/renderer/app-shell.css";
import "../../src/renderer/features/settings/settings.css";

const view = location.hash.slice(1) || "map";
const sources = [
  { sourceId: "avatar", itemId: "a", itemName: "Meiyun_v2.1.0.zip", name: "Meiyun_v2.1.0/Meiyun.unitypackage", productIds: ["booth:1"] },
  { sourceId: "outfit", itemId: "b", itemName: "TshirtShorts_Meiyun_v1.00.zip", name: "TshirtShorts_Meiyun_v1.00.unitypackage", productIds: ["booth:2"] },
  { sourceId: "costume", itemId: "c", itemName: "Meiyun_Costume4_v1.0.0.zip", name: "Meiyun_Costume4.unitypackage" },
];
const base = (runId: string, phase: AmfProductionRun["phase"], steps: [string, string, string?][], outfit = "outfit"): AmfProductionRun => ({
  schemaVersion: "0.1", runId, phase, taskId: "task-" + runId, editor: { path: "C:/Unity/Unity.exe", version: "2022.3.22f1" }, projectRoot: "C:/work/" + runId,
  choices: [{ sourceId: "avatar", role: "auto" }, { sourceId: outfit, role: "auto" }], sources: [sources[0]!, sources.find(s => s.sourceId === outfit)!].map(s => ({ ...s, role: s.sourceId === "avatar" ? "avatar" as const : "outfit" as const, sha256: "a".repeat(64) })),
  prefabs: [{ sourceId: "avatar", path: "Assets/Meiyun/Meiyu_改変.prefab", name: "Meiyu_改変", isAvatar: true, hasSkinnedMesh: true, hasMergeArmature: false, hasMissingScripts: false, humanoidBones: ["Head"] },
    { sourceId: outfit, path: "Assets/Outfit/Outfit.prefab", name: outfit === "outfit" ? "TshirtShorts_Meiyun" : "Meiyun_Costume4", isAvatar: false, hasSkinnedMesh: true, hasMergeArmature: true, hasMissingScripts: false, humanoidBones: [] }],
  steps: steps.map(([key, state, detail]) => ({ key, name: key, state, detail: detail ?? "" })), selection: null, error: null, recovery: null, result: null, snapshotId: null,
});
const runs: Record<string, AmfProductionRun> = {
  running: { ...base("running", "preparing", [["project", "succeeded"], ["materials", "succeeded"], ["inspect", "running"]], "costume"),
    progress: { step: "inspect", name: "Read actual prefabs", startedAt: new Date(Date.now() - 161_000).toISOString(), elapsedSeconds: 161, usualSeconds: 310,
      unity: { stage: "importing", current: 123, total: 379, lastActivityAt: new Date().toISOString() } } } as AmfProductionRun,
  waiting: base("waiting", "ready", [["project", "succeeded"], ["materials", "succeeded"], ["inspect", "succeeded"]]),
  attention: { ...base("attention", "failed", [["project", "succeeded"], ["materials", "succeeded"], ["inspect", "failed", "Unity could not get a license."]]), error: "vua.amf_production.unity_license_unavailable — detail: Activate this Editor's license in Unity Hub, then retry preparation. Selected materials are retained." },
  interrupted: { ...base("interrupted", "inspect_required", [["project", "succeeded"], ["materials", "succeeded"], ["inspect", "succeeded"], ["snapshot", "succeeded"], ["assembly", "interrupted"]]), snapshotId: "snapshot-1" },
  finished: { ...base("finished", "succeeded", [["project", "succeeded"], ["materials", "succeeded"], ["inspect", "succeeded"], ["snapshot", "succeeded"], ["assembly", "succeeded"]]),
    result: { facts: { sdkDescriptorRecognized: true, missingReferences: 0, avatarName: "Meiyu_改変", instances: [{ sourceId: "outfit", bindingVerified: true, menuVerified: true, menuModified: true }], missingShaderMaterials: [], sdkSignedIn: "no" } } },
};
// A finished line whose package also carries the standard Meiyun prefab; the preview picks it to show the rebuild note.
const outfitChoice = { sourceId: "outfit", prefabPath: "Assets/Outfit/Outfit.prefab", mode: "merge_armature" as const, bone: "", toggleName: "TshirtShorts", active: true };
// Two main outfits worn one at a time: the T-shirt on by default, the costume off and reset per world.
const costumeChoice = { sourceId: "costume", prefabPath: "Assets/Costume4/Costume4.prefab", mode: "merge_armature" as const, bone: "", toggleName: "Costume4", active: false, saved: false, group: "main" };
const mainSelection = { avatarPrefab: "Assets/Meiyun/Meiyu_改変.prefab", outfits: [{ ...outfitChoice, group: "main" }, costumeChoice] };
runs.switchline = { ...runs.finished!, runId: "switchline",
  sources: [...runs.finished!.sources, { ...runs.finished!.sources[1]!, sourceId: "costume", itemName: "Meiyun_Costume4_v1.0.0.zip", name: "Meiyun_Costume4.unitypackage" }],
  prefabs: [...runs.finished!.prefabs, { ...runs.finished!.prefabs[1]!, sourceId: "costume", path: "Assets/Costume4/Costume4.prefab", name: "Costume4" }],
  selection: mainSelection, result: { facts: { ...runs.finished!.result!.facts!, observedSelection: mainSelection } } } as AmfProductionRun;
runs.rebuild = { ...runs.finished!, runId: "rebuild",
  prefabs: [...runs.finished!.prefabs, { ...runs.finished!.prefabs[0]!, path: "Assets/Meiyun/Meiyun.prefab", name: "Meiyun" }],
  selection: { avatarPrefab: "Assets/Meiyun/Meiyu_改変.prefab", outfits: [outfitChoice] },
  result: { facts: { ...runs.finished!.result!.facts!, observedSelection: { avatarPrefab: "Assets/Meiyun/Meiyu_改変.prefab", outfits: [outfitChoice] } } } } as AmfProductionRun;
// Face-tracking lines: full coverage with a creator patch menu, a patched line's lock, and no coverage at all.
const ftMenu = { id: "a".repeat(64), sourceId: "avatar", scriptPath: "Assets/Meiyun/Editor/FacePatch.cs", menuPath: "Meiyun/Face Tracking Setup" };
const ftFacts = (matched: number, menus: boolean, manual = false) => ({
  schemaVersion: "0.1" as const, evidence: "unity_source" as const, observedAt: "2026-10-22T06:00:00Z",
  state: "observed" as const, runtime: "unknown" as const, builtParameterBudget: "unknown" as const, truncated: false,
  manualReplayRequired: manual, expressionParametersObserved: matched > 0, sourceSyncedParameterBits: matched > 0 ? 24 : 0,
  meshes: [{ renderer: "Body", blendShapeCount: 60, missingMesh: false, profiles: [
    { id: "arkit52-v1" as const, referenceCount: 52, matched: Array.from({ length: matched }, (_, i) => `bs_face_${i}`) },
    { id: "unified-core12-v1" as const, referenceCount: 12, matched: matched === 52 ? Array.from({ length: 12 }, (_, i) => `bs_ue_${i}`) : [] },
    { id: "sranipal-core12-v1" as const, referenceCount: 12, matched: [] }] }],
  parameters: matched > 0 ? ["FaceTrackingActive"] : [], components: matched > 0 ? ["ModularAvatarMergeArmature"] : [],
  patchMenus: menus ? [ftMenu] : [],
});
runs.ftline = { ...runs.finished!, runId: "ftline", result: { facts: { ...runs.finished!.result!.facts!, faceTracking: ftFacts(52, true) } } } as AmfProductionRun;
runs.ftlocked = { ...runs.finished!, runId: "ftlocked", snapshotId: "snapshot-ft", result: { facts: { ...runs.finished!.result!.facts!, faceTracking: ftFacts(52, false, true) } } } as AmfProductionRun;
runs.ftnone = { ...runs.finished!, runId: "ftnone", result: { facts: { ...runs.finished!.result!.facts!, faceTracking: ftFacts(0, false) } } } as AmfProductionRun;
// A finished line with version records: the newest kept the state before a restore, one carries Unity edits.
const versionSummary = { avatar: "Meiyu_改変", outfits: ["TshirtShorts_Meiyun"], sources: ["Meiyun_v2.1.0.zip", "TshirtShorts_Meiyun_v1.00.zip"], lightingMaterials: 4, sdkFixes: ["Avatar Descriptor"] };
runs.versions = { ...runs.finished!, runId: "versions", revisions: [
  { id: "rev-5", before: "restore", takenAt: "2026-10-22T05:12:00Z", editedInUnity: false, summary: versionSummary },
  { id: "rev-4", before: "lighting", takenAt: "2026-10-22T04:58:00Z", editedInUnity: true, summary: versionSummary },
  { id: "rev-3", before: "sdk_fix", takenAt: "2026-10-21T16:40:00Z", editedInUnity: false, summary: versionSummary },
  { id: "rev-2", before: "rebuild", takenAt: "2026-10-21T15:02:00Z", editedInUnity: false, summary: { ...versionSummary, avatar: "Meiyun" } },
  { id: "rev-1", before: "materials", takenAt: "2026-10-21T13:30:00Z", editedInUnity: false, summary: { avatar: null, outfits: [], sources: ["Meiyun_v2.1.0.zip"], lightingMaterials: 0, sdkFixes: [] } },
] } as AmfProductionRun;
// A finished line with unsaved Unity-side adjustments on its outfit, and a stored fit the next assembly applies.
const fitObject = (path: string) => ({ path, localPosition: { x: 0.01, y: 0, z: 0 }, localRotation: { x: 0, y: 0, z: 0, w: 1 }, localScale: { x: 1, y: 1, z: 1 } });
runs.fitline = { ...runs.finished!, runId: "fitline",
  selection: { avatarPrefab: "Assets/Meiyun/Meiyu_改変.prefab", outfits: [outfitChoice] },
  fitChanges: [{ sourceId: "outfit", moved: [fitObject("ChestRibbon"), fitObject("Skirt/Bow#1")], missing: ["Clips#2"], changesDigest: "d".repeat(64) }],
  availableFits: [{ sourceId: "outfit", fitId: "fit-1", name: "TshirtShortsForMeiyun" }],
  result: { facts: { ...runs.finished!.result!.facts!, fitSkipped: [{ sourceId: "outfit", paths: ["Clips#3", "Skirt/Bow#2"] }] } } } as AmfProductionRun;
// A finished line measured at 85 MiB on PC: two outfit textures can drop to BC7, the body stays.
const texture = (id: string, path: string, size: number, format: string, bytes: number, editable = true) => ({ id, path, width: size, height: size, format, mipmap: true, faces: 1,
  estimatedBytes: bytes, known: true, maxSize: size, editable, reason: editable ? "" : "shared_with_avatar" });
const pcTextures = [texture("body", "Assets/Meiyun/Texture/Body.png", 4096, "DXT5", 22369621, false),
  texture("shirt", "Assets/Amenbo/TshirtShorts/Texture/Shirt.png", 4096, "RGBA32", 89478484), texture("shorts", "Assets/Amenbo/TshirtShorts/Texture/Shorts.png", 2048, "RGBA32", 22369620)];
const pcTotal = pcTextures.reduce((sum, item) => sum + item.estimatedBytes, 0);
runs.texline = { ...runs.finished!, runId: "texline", result: { facts: { ...runs.finished!.result!.facts!, textureMemory: [
  { platform: "pc", textures: pcTextures, totalBytes: pcTotal, complete: true, rank: "poor" },
  { platform: "android", textures: pcTextures, totalBytes: pcTotal, complete: true, rank: "very_poor" }] } } } as AmfProductionRun;
const pixel = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
const api = {
  // The capacity view shows an idle factory, where the setting can change; the others have a line running.
  async list() { const idle = view === "workshop-capacity"; return { sources, editors: [{ path: "C:/Unity/Unity.exe", version: "2022.3.22f1" }], busy: !idle, queue: [],
    capacity: idle ? 2 : 1, running: idle ? [] : [{ taskId: "task-running", runId: "running", operation: "amf.production.prepare" }],
    runs: Object.values(runs).map(run => ({ runId: run.runId, phase: run.phase, sourceNames: run.sources.map(s => s.itemName) })) }; },
  async get(runId: string) { return structuredClone(runs[runId]!); },
  async listRecipes() { return [1, 2, 3].map(i => ({ recipeId: "r" + i, title: "Recipe " + i, revision: 1, updatedAt: "2026-10-17T00:00:00Z" })); },
  async previewImage() { return pixel; },
  // The close-offer view: Unity holds the project, so opening it is refused with the save-and-close action.
  async open(runId: string) {
    if (view === "workshop-close-offer") throw new AmfProductionError("vua.amf_production.project_open", "Unity has this work project open. You can save and close Unity first.", "amf.production.closeEditor");
    return { runId, taskId: "task-open" };
  },
  // The real case: the T-shirt materials use 0.05 while the body uses 0.1.
  async planLighting(runId: string) { return { runId, planDigest: "a".repeat(64), reference: { min: "0.1", max: "1" }, referenceReason: null,
    entries: ["ts_Black", "ts_Blue", "ts_Pink", "ts_White"].map(name => ({ material: `Assets/Amenbo/TshirtShorts/Material/${name}.mat`, sourceId: "outfit", current: { min: "0.05", max: "1" }, proposed: { min: "0.1", max: "1" } })),
    skipped: [{ material: "Assets/Nigerunaya/Meiyun/Material/Hair.mat", reason: "shared_with_avatar" }] }; },
  // The face-tracking views: the review names the menu and carries the engine's trust warning.
  async faceTrackingPlan(runId: string, menuId: string) { return { schemaVersion: "0.1", runId, revision: 3, planDigest: "e".repeat(64),
    projectFingerprint: "f".repeat(64), unityFingerprint: "0".repeat(64),
    menu: { ...ftMenu, id: menuId }, avatarGlobalObjectId: "gid-avatar", execution: "interactive_editor", replay: "manual", requiresTrustedCodeConfirmation: true,
    warning: "Creator code runs with Editor privileges. The restore point covers only the work project. Finish on the same Avatar. Recipe and version replay will require manual reapplication." }; },
  async faceTrackingPatch(runId: string) { return { runId, taskId: "task-ft" }; },
  async planTextureMemory(runId: string, platform: "pc" | "android", targetRank: "excellent" | "good" | "medium" | "poor") {
    const current = runs.texline!.result!.facts!.textureMemory!.find(report => report.platform === platform)!;
    const entries = current.textures.filter(item => item.editable).map(item => ({ current: item, estimatedBytes: Math.round(item.estimatedBytes / 4),
      proposed: { path: item.path, platform, maxSize: item.maxSize, format: platform === "pc" ? "BC7" : "ASTC_4x4" } }));
    const total = current.totalBytes - entries.reduce((sum, entry) => sum + entry.current.estimatedBytes - entry.estimatedBytes, 0);
    return { schemaVersion: "0.1", runId, planDigest: "9".repeat(64), plan: { platform, targetRank, current, entries, estimatedTotalBytes: total,
      estimatedRank: platform === "pc" ? "good" : "poor", reachesTarget: platform === "pc" } };
  },
  // The versions views: rev-1's original is gone from the library, rev-4 reviews with a Unity-edit warning, rev-2 rebuilds on the recorded base.
  async revisionPlan(runId: string, revisionId: string) {
    const revision = runs.versions!.revisions!.find(item => item.id === revisionId)!;
    if (revisionId === "rev-1") throw new Error("This version's original packages are unavailable: OldSchool_Meiyun.zip is no longer in the library. Supply these exact original packages in the library, then retry.");
    return { schemaVersion: "0.1", runId, revisionId, planDigest: "b".repeat(64), before: revision.before, takenAt: revision.takenAt, summary: revision.summary,
      editedInUnity: revision.editedInUnity, warning: revision.editedInUnity ? "The project changed in Unity since the last check. Restoring this version replaces those edits." : null,
      steps: revisionId === "rev-2"
        ? [{ key: "materials", skipped: true }, { key: "assembly", skipped: false, rebuild: true }, { key: "lighting", skipped: true, materials: [] }, { key: "sdk_fix", skipped: false, fixes: ["Avatar Descriptor"] }]
        : [{ key: "materials", skipped: true }, { key: "assembly", skipped: false }, { key: "lighting", skipped: false, materials: ["ts_Black", "ts_White"].map(name => ({ material: `Assets/Amenbo/TshirtShorts/Material/${name}.mat`, min: "0.1", max: "1" })) }, { key: "sdk_fix", skipped: true, fixes: [] }] };
  },
  async restoreRevision(runId: string) { return { runId, taskId: "task-restore" }; },
  // The fits views: the plan names the stored fit the assembly applies, and saving keeps the reported adjustments.
  async plan(runId: string, selection: unknown) { return { runId, planDigest: "c".repeat(64), selection,
    inputs: [{ sourceId: "avatar", name: "Meiyu_改変", role: "avatar" }, { sourceId: "outfit", name: "TshirtShorts", role: "outfit" }],
    fits: [{ sourceId: "outfit", fitId: "fit-1", name: "TshirtShortsForMeiyun" }] }; },
  async saveFit(runId: string, sourceId: string, name: string) { return { fitId: "fit-new", name, createdAt: "2026-10-23T01:00:00Z", updatedAt: "2026-10-23T01:00:00Z" }; },
} as unknown as AmfProductionPort;
// The materials-start views: the library's products mapped to the AMF sources above,
// plus one unrelated dress so the "other outfits" fold has a row.
const materialProducts = [
  { productId: "booth:1", title: "Meiyun formula", entry: "a" },
  { productId: "booth:2", title: "T-shirt & shorts for Meiyun", entry: "b" },
  { productId: "booth:3", title: "Meiyun Costume 4", entry: "c" },
  { productId: "booth:4", title: "Dress for Kikyu", entry: "d" },
];
const materialFacts = new Map(materialProducts.map(product => [product.productId, {
  storage: { state: "present", storedCopies: 1, presentCopies: 1, missingCopies: 0, changedCopies: 0, unreadableCopies: 0, supersededGeneratedCopies: 0, currentGeneratedCopies: 0, productionQualification: "not_evaluated" },
  sources: ["bought" as const], operation: null,
  localEntries: [{ entryId: product.entry, displayName: product.productId === "booth:1" ? "Meiyun_v2.1.0.zip" : product.title }],
}]));
const libraryApi: LibraryBrowserPort = {
  async list() {
    const items = materialProducts.map(product => ({
      productId: product.productId, title: product.title, libraryType: "bought" as const, importedArtifacts: 1, shopName: "Sample shop", variantName: null,
      price: { currency: "JPY", amount: "1000" }, imageUrl: pixel, imageUrls: [pixel], availability: "available" as const, availabilityRaw: null, entityCount: 0, entityTypes: [],
    }));
    return { catalog: { schemaVersion: 1 as const, kind: "results" as const, items, total: items.length, vocabulary: { availabilities: ["available" as const], entityTypes: [], relationKinds: [] } },
      localEntries: [], facts: materialFacts, sourceCategories: [], total: items.length, offset: 0, limit: 200 };
  },
  async productFiles(productId: string) { return { productId, items: [] }; },
};
const materialSources = [...sources, { sourceId: "dress", itemId: "d", itemName: "Kikyu_Dress_v1.2.zip", name: "Kikyu_Dress.unitypackage" }];
const materialsApi: AmfProductionPort = {
  ...api,
  async list() {
    const listed = await api.list();
    return { ...listed, sources: view.endsWith("-missing") ? materialSources.filter(source => source.itemId !== "a") : materialSources };
  },
} as AmfProductionPort;
const readyModule = { snapshot: async () => ({ schemaVersion: "0.1" as const, moduleId: "amf" as const, installed: true, state: "ready" as const }),
  subscribe: () => () => {}, setEnabled: async () => ({ outcome: "failed" as const, snapshot: { schemaVersion: "0.1" as const, moduleId: "amf" as const, installed: true, state: "ready" as const } }) };
const empty = emptyGateway(null);
// The open views: Unity still starting, and Unity holding the project with the Avatar selected.
const gateway = view === "workshop-opening" || view === "workshop-opened" ? { ...empty, task: { ...empty.task, async snapshot() {
  return { schemaVersion: 1 as const, tasks: [{ id: "task-open", title: "Open Unity", status: view === "workshop-opened" ? "completed" as const : "running" as const, originPage: "workshop", cancellable: false }] };
} } } as typeof empty : empty;
const selected = view.startsWith("workshop-") ? view.slice("workshop-".length) : "";
try { localStorage.setItem("vua-amf-production-run-v01", selected); localStorage.setItem("vua-amf-permission-mode-v01", view.endsWith("auto") ? "full" : "ask"); } catch { /* preview only */ }
// Views that open a station or a line card the way the player does, and the line they show.
const clickThrough: Record<string, { run: string; clicks: string[] }> = {
  materials: { run: "finished", clicks: ["workshop-station-prepare", "workshop-change-materials"] },
  inspect: { run: "waiting", clicks: ["workshop-station-inspect"] },
  "line-menu": { run: "finished", clicks: ["workshop-line-close"] },
  delete: { run: "finished", clicks: ["workshop-line-close", "workshop-line-delete"] },
  capacity: { run: "finished", clicks: [] },
  rebuild: { run: "rebuild", clicks: ["workshop-station-plan", "workshop-plan-adjust"] },
  opening: { run: "finished", clicks: ["workshop-open"] },
  opened: { run: "finished", clicks: ["workshop-open"] },
  ft: { run: "ftline", clicks: ["workshop-station-face"] },
  "ft-dialog": { run: "ftline", clicks: ["workshop-station-face", "workshop-ft-menu-aaaaaaaa"] },
  "ft-locked": { run: "ftlocked", clicks: ["workshop-station-face"] },
  "ft-none": { run: "ftnone", clicks: ["workshop-station-face"] },
  "close-offer": { run: "finished", clicks: ["workshop-open"] },
  texture: { run: "texline", clicks: [] },
  switch: { run: "switchline", clicks: ["workshop-station-plan"] },
  "switch-adjust": { run: "switchline", clicks: ["workshop-station-plan", "workshop-plan-adjust"] },
  versions: { run: "versions", clicks: ["workshop-station-assemble"] },
  "versions-plan": { run: "versions", clicks: ["workshop-station-assemble", "workshop-revision-rev-4"] },
  "versions-confirm": { run: "versions", clicks: ["workshop-station-assemble", "workshop-revision-rev-2", "workshop-revision-restore"] },
  "versions-error": { run: "versions", clicks: ["workshop-station-assemble", "workshop-revision-rev-1"] },
  fitsave: { run: "fitline", clicks: [] },
  fitplan: { run: "fitline", clicks: ["workshop-station-plan"] },
  fitreview: { run: "fitline", clicks: ["workshop-station-plan", "workshop-plan-adjust", "workshop-plan-review"] },
};
const runOf = selected === "full-access" || selected === "mode-menu" ? "waiting" : clickThrough[selected]?.run ?? selected.replace(/-auto$/u, "");
function ClickThrough({ clicks }: { clicks: readonly string[] }) {
  React.useEffect(() => {
    let at = 0;
    const timer = setInterval(() => {
      const target = document.querySelector<HTMLButtonElement>(`[data-nav-id=${clicks[at]}]`);
      if (!target) return;
      target.click();
      at += 1;
      if (at >= clicks.length) clearInterval(timer);
    }, 350);
    return () => clearInterval(timer);
  }, [clicks]);
  return null;
}
// Chooses another Avatar in the plan's Avatar list the way the player does.
function PickAvatar({ path }: { path: string }) {
  React.useEffect(() => {
    const timer = setInterval(() => {
      const select = [...document.querySelectorAll<HTMLSelectElement>("select")].find(item => [...item.options].some(option => option.value === path));
      if (!select) return;
      clearInterval(timer);
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!.call(select, path);
      select.dispatchEvent(new Event("change", { bubbles: true }));
    }, 150);
    return () => clearInterval(timer);
  }, [path]);
  return null;
}
// Opens the line's mode menu the way the player does, and for full access chooses it to show the warning.
function ChooseMode({ full }: { full: boolean }) {
  React.useEffect(() => {
    const timer = setInterval(() => {
      const button = document.querySelector<HTMLButtonElement>("[data-nav-id=workshop-mode]");
      if (!button) return;
      clearInterval(timer);
      button.click();
      if (full) setTimeout(() => document.querySelector<HTMLButtonElement>("[data-nav-id=workshop-mode-full]")?.click(), 400);
    }, 100);
    return () => clearInterval(timer);
  }, [full]);
  return null;
}
try { localStorage.setItem("vua-amf-production-run-v01", runOf); } catch { /* preview only */ }
const control = view.startsWith("control-") ? controlPreview(view, { ...gateway, amfProduction: api }, api) : null;
const release = view.startsWith("release-") ? releasePorts(view) : null;
const signedIn = view === "release-check" || view === "release-uploaded" || view === "release-vrchat";
const vrchatOfRelease = signedIn ? vrchatPreviewPort({ enabled: true, state: "signed_in", account: null, rememberPassword: false, pendingMethods: [], canCheckUploads: true })
  : view === "release-vrchat-signin" ? vrchatPreviewPort({ enabled: true, state: "signed_out", account: null, rememberPassword: false, pendingMethods: [], canCheckUploads: false }) : undefined;
const settingsVrchat = view.startsWith("settings-vrchat")
  ? vrchatPreviewPort(view === "settings-vrchat-account"
    ? { enabled: true, state: "signed_in", account: { displayName: "Aran", thumbnail: pixel }, rememberPassword: true, pendingMethods: [], canCheckUploads: true }
    : { enabled: true, state: "signed_out", account: null, rememberPassword: false, pendingMethods: [], canCheckUploads: false })
  : undefined;
/** The materials-start preview: a stand-in for the library drawer's action row and
 * the real "Start production" panel over it. */
function MaterialsStartPreview() {
  const [open, setOpen] = useState(true);
  return <div className="vua-page">
    <div className="vua-warehouse-detail__content" style={{ maxWidth: 420 }}>
      <Button variant="primary" className="vua-warehouse-detail__start" onClick={() => setOpen(true)}>{strings.factory.materialsStart.startCta}</Button>
    </div>
    {open ? <MaterialsStartDialog avatar={{ productId: "booth:1", title: "Meiyun formula", imageUrl: pixel }} onNavigate={() => {}} onClose={() => setOpen(false)} /> : null}
  </div>;
}
// The Unity license status above the account guide: Hub needed, a license to check, and a passed check.
const licenseFiles = (msix: boolean) => [{ kind: "classic_entitlement" as const, present: false, modifiedAt: null },
  { kind: "msix_entitlement" as const, present: msix, modifiedAt: msix ? "2026-10-19T00:00:00.000Z" : null }, { kind: "legacy_ulf" as const, present: false, modifiedAt: null }];
const licenseSnapshots: Record<string, UnityLicenseSnapshotV01> = {
  "license-hub": { schemaVersion: "0.1", files: licenseFiles(true), hubType: "msix", hubRunning: false, licensingClientRunning: false, hubCliPresent: false, lastCheck: null, state: "needs_hub_client", nextAction: "start_hub" },
  "license-check": { schemaVersion: "0.1", files: licenseFiles(true), hubType: "msix", hubRunning: true, licensingClientRunning: true, hubCliPresent: false, lastCheck: null, state: "likely_active", nextAction: "run_check" },
  "license-active": { schemaVersion: "0.1", files: licenseFiles(true), hubType: "msix", hubRunning: true, licensingClientRunning: true, hubCliPresent: false,
    lastCheck: { state: "active", editorVersion: "2022.3.22f1", checkedAt: "2026-10-19T09:30:00.000Z" }, state: "likely_active", nextAction: "none" },
};
const licensePort = licenseSnapshots[view] ? { snapshot: async () => licenseSnapshots[view]!, check: async () => ({ taskId: "task-license" }) } : undefined;
// The regional tier preview: one finished measurement with an unsteady region and
// a proxy note, so the collapsed tile and the expanded rows are both reviewable.
const regionObservation = (id: string, elapsedMs: number, p90Ms: number): WebsiteObservation => ({
  url: REGIONAL_REFERENCES.find(region => region.id === id)!.url,
  status: "http_error", elapsedMs, httpStatus: 404,
  regional: { medianMs: elapsedMs + 2, p90Ms, samples: 9, connections: 3, viaProxy: true },
});
const regionRows = [regionObservation("jp", 28, 34), regionObservation("usw", 40, 95), regionObservation("use", 120, 124), regionObservation("eu", 240, 246)];
const networkPreviewPort = {
  websiteCapability: async () => ({ state: "ready" as const }),
  capability: async () => ({ state: "ready" as const }),
  testWebsites: async (urls: readonly string[]) => urls.map(url => regionRows.find(row => row.url === url) ?? { url, status: "reachable" as const, elapsedMs: 120, httpStatus: 200 as const }),
  check: async (intent: NetworkIntent): Promise<NetworkReport> => ({ schemaVersion: "0.1", intent, detectedRegion: "unknown", effectiveRegion: "unknown", capturedAt: "2026-10-22T00:00:00Z", durationMs: 0, results: [] }),
};
/** Runs the regional test the way the player does, with the details expanded on
 * the details view. */
function NetworkPreview({ details }: { details: boolean }) {
  React.useEffect(() => {
    const timer = setInterval(() => {
      const action = document.querySelector<HTMLButtonElement>("[data-nav-id=play-network-test]");
      if (!action || action.disabled) return;
      clearInterval(timer);
      if (details) document.querySelector<HTMLButtonElement>("[data-nav-id=play-network-details]")?.click();
      action.click();
    }, 300);
    return () => clearInterval(timer);
  }, [details]);
  return <div className="vua-play-page vua-compact-environments" style={{ maxWidth: 720 }}><NetworkPanel compact /></div>;
}
createRoot(document.getElementById("root")!).render(view.startsWith("terms-") ? <GatewayProvider gateway={{ ...gateway, terms: termsPreviewPort(view) }}>
  <div style={{ maxWidth: 420, height: "100%" }}><aside className="vua-warehouse__drawer"><div className="vua-warehouse-detail__content">
    <ProductTerms productId="booth:1" title="Sample Avatar" />
  </div></aside></div>
</GatewayProvider> : licensePort ? <GatewayProvider gateway={{ ...gateway, environment: { ...gateway.environment, unityLicense: licensePort }, amfProduction: api }}>
  <div className="vua-page" style={{ maxWidth: 760 }}>
    <section className="vua-page__hero"><h1 className="vua-title">{strings.unityLicense.title}</h1></section>
    <Card><UnityLicenseGuide /></Card>
  </div>
</GatewayProvider> : release ? <GatewayProvider gateway={{ ...gateway, amfProduction: release.production, amfRelease: release.release, terms: termsPreviewPort(view), ...(vrchatOfRelease ? { vrchat: vrchatOfRelease } : {}) }}>
  <ReleasePreview view={view} />
</GatewayProvider> : view.startsWith("settings-") ? <GatewayProvider gateway={{ ...gateway, ...(settingsVrchat ? { vrchat: settingsVrchat } : {}) }}>
  <div className="vua-page" style={{ maxWidth: 760 }}>
    <section className="vua-page__hero"><h1 className="vua-title">设置 · 实验性</h1></section>
    <Card><VrchatSignInSection /></Card>
  </div>
</GatewayProvider> : view.startsWith("network-") ? <GatewayProvider gateway={{ ...gateway, environment: { ...gateway.environment, network: networkPreviewPort } }}>
  <NetworkPreview details={view === "network-tiers-details"} />
</GatewayProvider> : view.startsWith("materials-start") ? <GatewayProvider gateway={{ ...gateway, amfProduction: materialsApi, libraryBrowser: libraryApi, amfModule: readyModule }}>
  <AmfRoom page="warehouse" navigate={() => {}}><MaterialsStartPreview /></AmfRoom>
</GatewayProvider> : <GatewayProvider gateway={control?.gateway ?? { ...gateway, amfProduction: api, libraryBrowser: libraryApi }}>
  {control ? control.page : view.startsWith("map") ? <FactoryMap navigate={() => {}} initialRoom={(view === "map-flight" ? null : view.slice(4) || null) as Room | null} /> : <AmfRoom page="workshop" navigate={() => {}}><AmfProductionPage onNavigate={() => {}} onPrepareEnv={() => {}} />{(view === "workshop-full-access" || view === "workshop-mode-menu") && <ChooseMode full={view === "workshop-full-access"} />}{clickThrough[selected] && <ClickThrough clicks={clickThrough[selected]!.clicks} />}{selected === "rebuild" && <PickAvatar path="Assets/Meiyun/Meiyun.prefab" />}</AmfRoom>}
</GatewayProvider>);

// The items view opens the list of every VN3 item the way the player does.
if (view === "terms-items") setTimeout(() => document.querySelector<HTMLElement>(".vua-terms__items summary")?.click(), 1200);
// The terms view scrolls the sheet to the terms check the way the player does.
if (view === "release-terms") setTimeout(() => document.querySelector<HTMLElement>(".vua-line-terms")?.scrollIntoView({ block: "center" }), 1800);
// Mid-flight frame: the capture runs 2.5 s after load; the camera move takes 0.65 s.
if (view === "map-flight") setTimeout(() => document.querySelector<HTMLElement>("[data-nav-id=door-workshop], [data-nav-id=room-workshop]")?.click(), 2200);
