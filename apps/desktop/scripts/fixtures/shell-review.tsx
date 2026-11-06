// Whole-shell review: the real App over synthetic wire records, for walking every page and flow in a
// plain browser or a hidden Electron window. Nothing here reaches real accounts, installers or Unity,
// and the production build never imports it.
//
// URL parameters:
//   profile=new      first run: empty storage, the first-use notice opens
//   env=missing      Steam and VRChat are not installed (default: installed)
//   amf=off          AMF is switched off (default: ready)
//   lines=none       no production lines (default: one in each state)
//   library=empty    no materials (default: a small library)
//   mode=ask         permission mode (default: full auto, the setting for new players)
//   lang=<locale>    UI language (default: zh-CN)
import type { VuaDesktopApiV2 } from "@vua/contracts";
import { createRoot } from "react-dom/client";
import { scenarioRequiredComponents, type DeploymentPurpose } from "@vua/contracts";
import { storageKeys } from "../../src/renderer/app/storage-keys.ts";

const params = new URLSearchParams(location.search);
const lang = params.get("lang") ?? "zh-CN";
const newProfile = params.get("profile") === "new";
const envMissing = params.get("env") === "missing";
const noLines = params.get("lines") === "none";
const emptyLibrary = params.get("library") === "empty";
let amfOn = params.get("amf") !== "off";

// A plain reload replays the same profile, so a walk can start again from a known state.
if (sessionStorage.getItem("shell-review-profile") !== location.search) {
  localStorage.clear();
  sessionStorage.setItem("shell-review-profile", location.search);
  if (!newProfile) {
    localStorage.setItem(storageKeys.goals, JSON.stringify({ version: 1, onboarding: "completed", goals: [], environments: [] }));
    localStorage.setItem(storageKeys.tourProgress, JSON.stringify({ v: 2, status: "skipped", step: 0 }));
    if (params.get("guidance") !== "on") {
      localStorage.setItem(storageKeys.contextGuide, JSON.stringify({ v: 1, dismissed: { home: 9, play: 9, tools: 9, creator: 9, warehouse: 9, recipe: 9, workshop: 9, release: 9, packages: 9 } }));
    }
  }
}
localStorage.setItem(storageKeys.locale, lang);
if (params.get("mode")) localStorage.setItem("vua-amf-permission-mode-v01", params.get("mode")!);

const now = () => new Date().toISOString();
const label = (text: string, hue: number) => `data:image/svg+xml;base64,${btoa(unescape(encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="480" height="480"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${hue} 60% 62%)"/><stop offset="1" stop-color="hsl(${(hue + 40) % 360} 55% 36%)"/></linearGradient></defs><rect width="480" height="480" fill="url(#g)"/><circle cx="240" cy="170" r="72" fill="#ffffff33"/><rect x="150" y="262" width="180" height="150" rx="34" fill="#ffffff26"/><text x="240" y="452" text-anchor="middle" font-family="sans-serif" font-size="26" fill="#fff">${text}</text></svg>`,
)))}`;

/* ---- library ---- */
const storagePresent = { state: "present", storedCopies: 1, presentCopies: 1, missingCopies: 0, changedCopies: 0, unreadableCopies: 0, supersededGeneratedCopies: 0, currentGeneratedCopies: 0, productionQualification: "not_evaluated" };
const storageCloud = { ...storagePresent, state: "cloud_only", storedCopies: 0, presentCopies: 0 };
const productCard = (id: number, title: string, shop: string, hue: number, stored: boolean) => ({
  kind: "product", sources: ["bought"], storage: stored ? storagePresent : storageCloud, copyIds: stored ? [`cpy-${id}`] : [], operation: null,
  ...(stored ? { localEntries: [{ entryId: `wh-${id}`, displayName: `${title}.zip` }] } : {}),
  product: { productId: `booth:${id}`, title, libraryType: "bought", importedArtifacts: stored ? 1 : 0, shopName: shop, variantName: null,
    price: { currency: "JPY", amount: String(1000 + id % 7 * 500) }, imageUrl: label(title.slice(0, 12), hue), imageUrls: [label(title.slice(0, 12), hue)],
    availabilityRaw: null, availabilityStatus: "available", entityCount: 0, entityTypes: [] },
});
const catalog = [
  productCard(101, "Meiyun formula", "Sample shop", 220, true),
  productCard(102, "T-shirt & shorts for Meiyun", "Amenbo", 340, true),
  productCard(103, "Meiyun Costume 4", "Sample shop", 280, true),
  productCard(104, "Karin v1.4", "Sample shop", 20, true),
  productCard(105, "Sailor dress for Karin", "Amenbo", 180, true),
  productCard(106, "Rusk v3.0", "Forest", 120, false),
  productCard(107, "Hoodie for Rusk", "Forest", 60, false),
  productCard(108, "Ribbon accessories", "Forest", 310, false),
];
const localEntry = {
  kind: "local", storage: storagePresent, copyIds: ["cpy-local"],
  entry: { warehouseItemId: "wh-local", folderName: "wh-local", displayName: "My own hair.zip", kind: "imported_material", createdAt: "2026-10-08T00:00:00Z", artifactMode: null, effectiveArtifactMode: "use_original_unitypackage",
    artifacts: [{ relativePath: "My own hair.unitypackage", artifactSha256: `sha256:${"a".repeat(64)}`, state: "clean", sizeBytes: 123456, role: "original" }] },
};
const libraryRows = emptyLibrary ? [] : [...catalog, localEntry];
const productionSources = catalog.filter(card => card.storage.state === "present").map(card => ({
  sourceId: `src-${card.product.productId}`, itemId: `wh-${card.product.productId.slice(6)}`, itemName: `${card.product.title}.zip`, name: `${card.product.title}.unitypackage`, productIds: [card.product.productId],
}));
const sourcesForList = emptyLibrary ? [] : productionSources;

/* ---- production lines ---- */
const editor = { path: "C:/Synthetic/Unity/2022.3.22f1/Editor/Unity.exe", version: "2022.3.22f1" };
const runBase = (runId: string, phase: string, avatar: number, outfit: number, steps: [string, string, string?][]) => {
  const a = productionSources[avatar]!, o = productionSources[outfit]!;
  return {
    schemaVersion: "0.1", runId, phase, taskId: `task-${runId}`, editor, projectRoot: `C:/Synthetic/work/${runId}`,
    choices: [{ sourceId: a.sourceId, role: "auto" }, { sourceId: o.sourceId, role: "auto" }],
    sources: [{ ...a, role: "avatar", sha256: "a".repeat(64) }, { ...o, role: "outfit", sha256: "b".repeat(64) }],
    prefabs: [
      { sourceId: a.sourceId, path: `Assets/${runId}/Avatar.prefab`, name: a.itemName.replace(".zip", ""), isAvatar: true, hasSkinnedMesh: true, hasMergeArmature: false, hasMissingScripts: false, humanoidBones: ["Head", "Chest"] },
      { sourceId: o.sourceId, path: `Assets/${runId}/Outfit.prefab`, name: o.itemName.replace(".zip", ""), isAvatar: false, hasSkinnedMesh: true, hasMergeArmature: true, hasMissingScripts: false, humanoidBones: [] },
    ],
    steps: steps.map(([key, state, detail]) => ({ key, name: key, state, detail: detail ?? "", startedAt: "2026-11-02T03:00:00Z", endedAt: state === "running" ? null : "2026-11-02T03:05:00Z" })),
    selection: null, error: null, recovery: null, result: null, snapshotId: null,
  };
};
const finishedFacts = (name: string) => ({ sdkDescriptorRecognized: true, missingReferences: 0, avatarName: name, instances: [{ sourceId: productionSources[1]!.sourceId, bindingVerified: true, menuVerified: true, menuModified: true }], missingShaderMaterials: [], sdkSignedIn: "no" });
const lineRecords: Record<string, ReturnType<typeof runBase> & Record<string, unknown>> = noLines || emptyLibrary ? {} : {
  "n3-running": { ...runBase("n3-running", "preparing", 0, 2, [["project", "succeeded"], ["materials", "succeeded"], ["inspect", "running"]]),
    progress: { step: "inspect", name: "Read prefabs", startedAt: new Date(Date.now() - 161_000).toISOString(), elapsedSeconds: 161, usualSeconds: 310, unity: { stage: "importing", current: 123, total: 379, lastActivityAt: now() } } },
  "n3-waiting": runBase("n3-waiting", "ready", 0, 1, [["project", "succeeded"], ["materials", "succeeded"], ["inspect", "succeeded"]]),
  "n3-attention": { ...runBase("n3-attention", "failed", 3, 4, [["project", "succeeded"], ["materials", "succeeded"], ["inspect", "failed", "Unity could not get a license."]]),
    error: "vua.amf_production.unity_license_unavailable — detail: Activate this Editor's license in Unity Hub, then retry preparation. Selected materials are retained." },
  "n3-finished": { ...runBase("n3-finished", "succeeded", 0, 1, [["project", "succeeded"], ["materials", "succeeded"], ["inspect", "succeeded"], ["snapshot", "succeeded"], ["assembly", "succeeded"]]),
    selection: { avatarPrefab: "Assets/n3-finished/Avatar.prefab", outfits: [{ sourceId: productionSources[1]!.sourceId, prefabPath: "Assets/n3-finished/Outfit.prefab", mode: "merge_armature", bone: "", toggleName: "T-shirt", active: true }] },
    snapshotId: "snap-1", completedAt: "2026-11-02T03:20:00Z", result: { facts: finishedFacts("Meiyun formula") } },
};
const hex = (seed: number) => seed.toString(16).padStart(4, "0").repeat(16);
const material = (seed: number, packageName: string) => ({ id: `m-${hex(seed)}`, packageName, sha256: hex(seed) });
const recipeDocument = (recipeId: string, title: string) => {
  const avatar = material(1, "Meiyun formula.unitypackage"), outfit = material(2, "T-shirt & shorts for Meiyun.unitypackage"), shader = material(3, "lilToon_1.8.5.unitypackage");
  return { revision: 1, updatedAt: "2026-10-30T03:00:00Z", recipe: { formatVersion: "0.4", recipeId, title, editorVersion: "2022.3.22f1",
    materials: [avatar, outfit, shader], avatar: { materialId: avatar.id, prefabPath: "Assets/Meiyun/Meiyun.prefab" },
    outfits: [{ materialId: outfit.id, prefabPath: "Assets/Outfit/Outfit.prefab", mode: "merge_armature", bone: "", toggleName: "T-shirt", active: true }],
    packages: { "com.vrchat.avatars": "3.7.4", "nadena.dev.modular-avatar": "1.10.3" } } };
};

/* ---- environment ---- */
const playSoftware = (route: string) => route === "desktop_play" ? ["steam", "vrchat"] : ["steam", "pico_runtime", "steamvr", "vrchat"];
const playStates: Record<string, string> = { desktop_play: "idle", pico_pcvr: "idle" };
const playSession = (route: string) => {
  const state = playStates[route]!; const active = state !== "idle";
  return { playSession: { schemaVersion: "vua.play-session/v0.2", capturedAt: now(), route, state, issue: null, canStop: active,
    software: playSoftware(route).map(component => ({ component, presence: envMissing ? "missing" : "verified", running: active, owned: active })) } };
};
const editors = ["2022.3.22f1", "2019.4.31f1"].map(version => ({ version, path: `C:/Synthetic/Unity/${version}`, classification: version === "2022.3.22f1" ? "production_target" : "other_unity_version" }));
const sopFor = (selection: { scenario: string; connection?: string }) => ({
  schemaVersion: "vua.scenario-environment/v0.1", scenario: selection.scenario, connection: selection.connection ?? null,
  requirements: scenarioRequiredComponents([selection.scenario as DeploymentPurpose]),
  steps: ["check", "install", "steam-account", "launch", "vrchat-account"].map((id, index) => ({ id, title: id, explanation: id, expectedResult: id, automatic: index < 2 })),
  performed: "sop_only", nextAction: "check_environment", licenseRequiredFor: "none",
});
const toolConnection = (toolId: string) => ({ toolConnection: { schemaVersion: "vua.external-tool/v0.3", toolId, capturedAt: now(), presence: envMissing ? "missing" : ["vrcft", "space", "chilloutvr", "vrchat", "steamvr"].includes(toolId) ? "installed" : "missing", steamReady: !envMissing, buildId: null, running: false, canStop: false, activity: "idle", issue: null } });

const unknownMethods = new Set<string>();
async function reply(method: string, input: Record<string, any>): Promise<unknown> {
  switch (method) {
    case "app.snapshot": return { capabilities: { tasks: true, operations: [{ operationId: "environment.testWebsites", availability: "available" }] } };
    case "task.list": return { revision: 1, tasks: [] };
    case "task.get": return null;
    case "environment.getSnapshot": return { contractVersion: "0.1", revision: 0, capturedAt: now(), items: [
      { checkId: "unity_editors", zone: "create", presence: "detected", facts: { editors } },
      { checkId: "steam", zone: "play", presence: envMissing ? "missing" : "detected", facts: {} },
      { checkId: "vrchat", zone: "play", presence: envMissing ? "missing" : "detected", facts: {} },
      { checkId: "pico_runtime", zone: "play", presence: "detected", facts: {} },
    ] };
    case "environment.getScenarioSop": return sopFor(input.scenario);
    case "environment.testWebsites": return { websiteTests: input.urls.map((url: string, index: number) => ({ url, status: "reachable", elapsedMs: 80 + index * 70, httpStatus: 200 })) };
    case "environment.observePlay": return playSession(input.route);
    case "environment.startPlay": playStates[input.route] = "running"; return playSession(input.route);
    case "environment.stopPlay": playStates[input.route] = "idle"; return playSession(input.route);
    case "project.environmentManagers": return { operation: method, result: { schemaVersion: "vua.environment-managers-snapshot/v0.1", editors, vcc: { presence: "found" }, alcom: { presence: "not_found" } } };
    case "environment.verifyEditor": return { verdict: "verified", version: input.path.split("/").at(-1), exePath: `${input.path}/Editor/Unity.exe`, classification: "other_unity_version" };
    case "environment.inspectManagerApps": return { managerApps: { schemaVersion: "vua.manager-apps/v0.1", capturedAt: now(), apps: ["unity_hub", "vcc", "alcom"].map(component => ({ component, presence: component === "alcom" ? "not_found" : "found", path: component === "alcom" ? null : `C:/Synthetic/${component}.exe` })) } };
    case "environment.checkScenarioEnvironment": {
      const ready = !envMissing;
      return { schemaVersion: "vua.scenario-environment/v0.1", intent: input.intent, softwareReady: ready, licenseState: "not_checked", performed: "software_check_only", nextAction: ready ? "follow_sop" : "plan_installation", installationStarted: false,
        observations: scenarioRequiredComponents(input.intent.purposes as DeploymentPurpose[]).map(component => ({ component, presence: ready ? "verified" : "missing", location: null, version: null })) };
    }
    case "environment.planDeployment": return { deploymentPlan: { schemaVersion: "vua.environment-deployment/v0.1", intent: input.intent, digest: "a".repeat(64), prerequisitesReady: !envMissing, installer: null,
      steps: scenarioRequiredComponents(input.intent.purposes as DeploymentPurpose[]).map(component => ({ component, action: envMissing ? "manual_install" : "retain", reason: envMissing ? "missing" : "verified", location: null, version: null, officialUrl: "https://store.steampowered.com/" })) } };
    case "environment.executeDeployment": return { schemaVersion: "vua.environment-deployment/v0.1", operation: method, taskId: "synthetic-deployment", correlationId: "synthetic" };
    case "environment.unityLicense": case "environment.checkUnityLicense": return { schemaVersion: "0.1", state: "active", checkedAt: now(), hubRunning: true, detail: null };
    case "library.list": {
      const offset = input.offset ?? 0, limit = input.limit ?? 50;
      const rows = libraryRows.filter(row => input.source !== "imported" || row.kind === "local")
        .filter(row => !input.text || (row.kind === "product" ? (row as ReturnType<typeof productCard>).product.title : (row as typeof localEntry).entry.displayName).toLowerCase().includes(String(input.text).toLowerCase()));
      return { schemaVersion: "0.1", items: rows.slice(offset, offset + limit), total: rows.length, offset, limit, sourceCategories: [] };
    }
    case "catalog.status": return { schemaVersion: "0.6", operation: method, result: { total: catalog.length, health: "ok" } };
    case "catalog.detail": {
      const card = catalog.find(item => item.product.productId === input.productId);
      if (!card) return null;
      return { schemaVersion: "0.6", operation: method, result: { product: { ...card.product, description: "Synthetic description of this product.", sourceCategory: "3D Avatar", videoUrls: [], subproducts: [] } } };
    }
    case "catalog.productDownloadables": return { schemaVersion: "0.6", operation: method, result: { downloadables: [] } };
    case "warehouse.listEntries": return { schemaVersion: "0.6", operation: method, result: { entries: libraryRows.filter(row => row.kind === "local").map(row => (row as typeof localEntry).entry) } };
    case "warehouse.entryDetail": return { schemaVersion: "0.6", operation: method, result: { entry: { ...localEntry.entry, artifacts: localEntry.entry.artifacts.map(artifact => ({ ...artifact, suggestedFileName: null, inspectedAt: null, rejectionReason: null, sourceCorrelated: false, mappedProductIds: [] })) } } };
    case "library.entryMetadata": return { schemaVersion: "0.1", metadata: null };
    case "terms.getProduct": return null;
    case "amf.production.list": return { schemaVersion: "0.1", sources: sourcesForList, editors: [editor], busy: lineRecords["n3-running"] !== undefined, capacity: 1, queue: [],
      running: lineRecords["n3-running"] ? [{ taskId: "task-n3-running", runId: "n3-running", operation: "amf.production.prepare" }] : [],
      runs: Object.values(lineRecords).map(run => ({ runId: run.runId, phase: run.phase, sourceNames: run.sources.map(source => source.itemName) })) };
    case "amf.production.get": return lineRecords[input.runId] ?? null;
    case "amf.production.listRecipes": return { schemaVersion: "0.1", recipes: noLines ? [] : [{ recipeId: "r-1", title: "Meiyun T恤短裤", revision: 1, updatedAt: "2026-10-30T03:00:00Z" }] };
    case "amf.production.getRecipe": return { schemaVersion: "0.1", document: recipeDocument(input.recipeId, "Meiyun T恤短裤") };
    case "amf.production.listFits": return { schemaVersion: "0.1", fits: [] };
    case "amf.production.previewImage": return null;
    case "amf.production.releaseState": return { schemaVersion: "0.1", runId: input.runId, editorOpen: false, turntable: null, uploads: [] };
    case "amf.production.plan": {
      const line = lineRecords[input.runId]!;
      return { schemaVersion: "0.1", runId: input.runId, planDigest: "b".repeat(64), selection: input.selection, preserveOriginals: true, officialSdkChecks: "not_run", editor: line.editor, projectRoot: line.projectRoot,
        inputs: line.sources.map((source, index) => ({ sourceId: source.sourceId, name: source.itemName, role: index === 0 ? "avatar" : "outfit" })) };
    }
    case "amf.production.prepare": return { schemaVersion: "0.1", runId: "n3-running", taskId: "task-n3-running" };
    case "vrchat.status": return { schemaVersion: "0.1", enabled: false, signedIn: false, account: null };
    case "packages.listRepos": return { operation: method, result: { repos: [] } };
    case "packages.repoCatalog": return { operation: method, result: { repos: [] } };
    case "project.listProjects": return { operation: method, result: { projects: [] } };
    case "inspection.list": return { operation: method, result: { items: [] } };
    case "tools.observeConnection": return toolConnection(input.toolId);
    default:
      if (!unknownMethods.has(method)) { unknownMethods.add(method); console.info("[shell-review] no synthetic reply for", method, input); }
      return null;
  }
}

const amfListeners = new Set<(snapshot: unknown) => void>();
const amfSnapshot = () => ({ schemaVersion: "0.1", moduleId: "amf", installed: amfOn, state: amfOn ? "ready" : "absent" });

const host = {
  gateway: { invoke: async (request: { method: string; params?: Record<string, unknown> }) => {
    const value = await reply(request.method, request.params ?? {});
    return value === null
      ? { ok: false, error: { code: "application", application: { contractVersion: "0.1", code: "vua.synthetic.unavailable", category: "unavailable", messageKey: "errors.synthetic", recoverable: false } } }
      : { ok: true, value };
  } },
  events: { subscribe: () => () => {} },
  amfModule: {
    snapshot: async () => amfSnapshot(),
    setEnabled: async (enabled: boolean) => { amfOn = enabled; amfListeners.forEach(listener => listener(amfSnapshot())); return { outcome: "updated", snapshot: amfSnapshot() }; },
    subscribe: (listener: (snapshot: unknown) => void) => { amfListeners.add(listener); return () => amfListeners.delete(listener); },
  },
  window: { showReader: async () => {}, showGameGuide: async () => {}, showOverlay: async () => {},
    showEncyclopedia: async (target: unknown) => { window.dispatchEvent(new CustomEvent("vua:open-encyclopedia", { detail: target })); return { visible: true }; },
    encyclopediaTargetEvents: { subscribe: () => () => {} }, shellCommandEvents: { subscribe: () => () => {} },
    minimize: async () => {}, toggleMaximize: async () => {}, close: async () => {} },
  system: {
    readResourceUsage: async () => ({ schemaVersion: 1, ramUsedBytes: 4 * 1024 ** 3, ramTotalBytes: 16 * 1024 ** 3, vramUsedBytes: null, vramTotalBytes: null, sampledAt: now() }),
    checkUpdate: async () => ({ schemaVersion: 1, state: "up-to-date", currentVersion: "1.0.0", latestVersion: "1.0.0", releaseUrl: null, checkedAt: now() }),
  },
  remoteContent: { openAccountGuideInBrowser: async () => {}, authProbe: async () => ({ authOk: false, accountName: null }) },
  silentDownload: { transfers: async () => [] },
  dialog: { pickWarehouseFolders: async () => null },
} as unknown as VuaDesktopApiV2;
Object.defineProperty(window, "vua", { configurable: true, value: host });

(async () => {
  await import("@vua/design-system/tokens.css");
  await import("@vua/design-system/base.css");
  const { App } = await import("../../src/renderer/App.tsx");
  createRoot(document.getElementById("root")!).render(<App />);
})();
