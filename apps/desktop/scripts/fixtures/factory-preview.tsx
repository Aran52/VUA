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
import type { AmfProductionPort, AmfProductionRun, LibraryBrowserPort } from "../../src/renderer/gateway/index.ts";
import { ReleasePreview, releasePorts, vrchatPreviewPort } from "./release-preview.tsx";
import { VrchatSignInSection } from "../../src/renderer/features/settings/vrchat-sign-in.tsx";
import { UnityLicenseGuide } from "../../src/renderer/features/settings/UnityLicenseGuide.tsx";
import type { UnityLicenseSnapshotV01 } from "@vua/contracts";
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
  { sourceId: "avatar", itemId: "a", itemName: "Meiyun_v2.1.0.zip", name: "Meiyun_v2.1.0/Meiyun.unitypackage" },
  { sourceId: "outfit", itemId: "b", itemName: "TshirtShorts_Meiyun_v1.00.zip", name: "TshirtShorts_Meiyun_v1.00.unitypackage" },
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
runs.rebuild = { ...runs.finished!, runId: "rebuild",
  prefabs: [...runs.finished!.prefabs, { ...runs.finished!.prefabs[0]!, path: "Assets/Meiyun/Meiyun.prefab", name: "Meiyun" }],
  selection: { avatarPrefab: "Assets/Meiyun/Meiyu_改変.prefab", outfits: [outfitChoice] },
  result: { facts: { ...runs.finished!.result!.facts!, observedSelection: { avatarPrefab: "Assets/Meiyun/Meiyu_改変.prefab", outfits: [outfitChoice] } } } } as AmfProductionRun;
const pixel = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
const api = {
  async list() { return { sources, editors: [{ path: "C:/Unity/Unity.exe", version: "2022.3.22f1" }], busy: true, queue: [],
    runs: Object.values(runs).map(run => ({ runId: run.runId, phase: run.phase, sourceNames: run.sources.map(s => s.itemName) })) }; },
  async get(runId: string) { return structuredClone(runs[runId]!); },
  async listRecipes() { return [1, 2, 3].map(i => ({ recipeId: "r" + i, title: "Recipe " + i, revision: 1, updatedAt: "2026-10-17T00:00:00Z" })); },
  async previewImage() { return pixel; },
  // The real case: the T-shirt materials use 0.05 while the body uses 0.1.
  async planLighting(runId: string) { return { runId, planDigest: "a".repeat(64), reference: { min: "0.1", max: "1" }, referenceReason: null,
    entries: ["ts_Black", "ts_Blue", "ts_Pink", "ts_White"].map(name => ({ material: `Assets/Amenbo/TshirtShorts/Material/${name}.mat`, sourceId: "outfit", current: { min: "0.05", max: "1" }, proposed: { min: "0.1", max: "1" } })),
    skipped: [{ material: "Assets/Nigerunaya/Meiyun/Material/Hair.mat", reason: "shared_with_avatar" }] }; },
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
const gateway = emptyGateway(null);
const selected = view.startsWith("workshop-") ? view.slice("workshop-".length) : "";
try { localStorage.setItem("vua-amf-production-run-v01", selected); localStorage.setItem("vua-amf-permission-mode-v01", view.endsWith("auto") ? "full" : "ask"); } catch { /* preview only */ }
// Views that open a station or a line card the way the player does, and the line they show.
const clickThrough: Record<string, { run: string; clicks: string[] }> = {
  materials: { run: "finished", clicks: ["workshop-station-prepare", "workshop-change-materials"] },
  inspect: { run: "waiting", clicks: ["workshop-station-inspect"] },
  "line-menu": { run: "finished", clicks: ["workshop-line-close"] },
  delete: { run: "finished", clicks: ["workshop-line-close", "workshop-line-delete"] },
  rebuild: { run: "rebuild", clicks: ["workshop-station-plan", "workshop-plan-adjust"] },
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
createRoot(document.getElementById("root")!).render(licensePort ? <GatewayProvider gateway={{ ...gateway, environment: { ...gateway.environment, unityLicense: licensePort }, amfProduction: api }}>
  <div className="vua-page" style={{ maxWidth: 760 }}>
    <section className="vua-page__hero"><h1 className="vua-title">{strings.unityLicense.title}</h1></section>
    <Card><UnityLicenseGuide /></Card>
  </div>
</GatewayProvider> : release ? <GatewayProvider gateway={{ ...gateway, amfProduction: release.production, amfRelease: release.release, ...(vrchatOfRelease ? { vrchat: vrchatOfRelease } : {}) }}>
  <ReleasePreview view={view} />
</GatewayProvider> : view.startsWith("settings-") ? <GatewayProvider gateway={{ ...gateway, ...(settingsVrchat ? { vrchat: settingsVrchat } : {}) }}>
  <div className="vua-page" style={{ maxWidth: 760 }}>
    <section className="vua-page__hero"><h1 className="vua-title">设置 · 实验性</h1></section>
    <Card><VrchatSignInSection /></Card>
  </div>
</GatewayProvider> : view.startsWith("materials-start") ? <GatewayProvider gateway={{ ...gateway, amfProduction: materialsApi, libraryBrowser: libraryApi, amfModule: readyModule }}>
  <AmfRoom page="warehouse" navigate={() => {}}><MaterialsStartPreview /></AmfRoom>
</GatewayProvider> : <GatewayProvider gateway={control?.gateway ?? { ...gateway, amfProduction: api }}>
  {control ? control.page : view.startsWith("map") ? <FactoryMap navigate={() => {}} initialRoom={(view === "map-flight" ? null : view.slice(4) || null) as Room | null} /> : <AmfRoom page="workshop" navigate={() => {}}><AmfProductionPage onNavigate={() => {}} onPrepareEnv={() => {}} />{(view === "workshop-full-access" || view === "workshop-mode-menu") && <ChooseMode full={view === "workshop-full-access"} />}{clickThrough[selected] && <ClickThrough clicks={clickThrough[selected]!.clicks} />}{selected === "rebuild" && <PickAvatar path="Assets/Meiyun/Meiyun.prefab" />}</AmfRoom>}
</GatewayProvider>);

// Mid-flight frame: the capture runs 2.5 s after load; the camera move takes 0.65 s.
if (view === "map-flight") setTimeout(() => document.querySelector<HTMLElement>("[data-nav-id=door-workshop], [data-nav-id=room-workshop]")?.click(), 2200);
