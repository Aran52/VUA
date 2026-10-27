import { createRoot, type Root } from "react-dom/client";
import { scenarioRequiredComponents, type DeploymentPurpose, type DesktopShellCommandV1, type VuaDesktopApiV2 } from "@vua/contracts";
import { App } from "../../src/renderer/App.tsx";
import { currentLocale, format, strings } from "../../src/renderer/i18n/index.ts";
import { storageKeys } from "../../src/renderer/app/storage-keys.ts";
import { TOUR_STEPS } from "../../src/renderer/features/tour/tour-model.ts";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";

const copy = strings.journey;
// Capture the shell's requested theme before App mounting can resolve or persist a default.
const previewTheme = document.documentElement.dataset.theme === "light" ? "light" : "dark";
let root: Root | undefined;
let scenario: "installed" | "missing" | "restored" = "installed";
let creatorInventory: "all" | "standard" = "all";
const editorVersions = () => creatorInventory === "standard" ? ["2022.3.22f1"] : ["2022.3.22f1", "2019.4.31f1", "2021.3.45f1"];
let guideId: string | null = null;
const intents: string[][] = [];
const checks: string[] = [];
let shellCommand: ((command: DesktopShellCommandV1) => void) | undefined;
let updateChecks = 0;
let playUnavailable = false, installationRunning = false, websiteCalls = 0, borrowedSteam = true;
const playStates: Record<string, string> = { desktop_play: "idle", pico_pcvr: "idle" };
const playCalls: string[] = [];
let playObservationCount = 0;
function syntheticPlay(route: string) {
  const state = playStates[route]; const active = state !== "idle";
  return { playSession: { schemaVersion: "vua.play-session/v0.2", capturedAt: new Date().toISOString(), route, state, issue: null, canStop: active,
    software: (route === "desktop_play" ? ["steam", "vrchat"] : ["steam", "pico_runtime", "steamvr", "vrchat"]).map(component => ({ component,
      presence: scenario === "missing" ? "missing" : "verified", running: component === "steam" ? borrowedSteam || active : state === "running" || state === "stopping",
      owned: component === "steam" ? !borrowedSteam && active : state === "running" || state === "stopping" })) } };
}
const check = (name: string, value: unknown) => { if (!value) throw new Error(name); checks.push(name); };
const wait = async (predicate: () => unknown, label = "") => {
  const start = Date.now();
  while (!predicate()) { if (Date.now() - start > 10000) throw new Error(`UI condition timed out: ${label || predicate.toString().slice(0, 160)}; wizard=${document.querySelector("[data-first-run-language]")?.getAttribute("data-first-run-language")}`); await new Promise(r => setTimeout(r, 25)); }
};
const visible = (el: Element) => el.getClientRects().length > 0 && !el.closest("[hidden]");
const detailsBelowCard = (control: Element) => {
  const entry = control.closest('[data-card-entry]')!;
  const card = entry.firstElementChild!, detail = document.getElementById(control.getAttribute('aria-controls')!)!;
  const cardRect = card.getBoundingClientRect(), detailRect = detail.getBoundingClientRect();
  return detail.parentElement === entry && card.nextElementSibling === detail && visible(detail)
    && Math.abs(detailRect.left - cardRect.left) < 1 && Math.abs(detailRect.width - cardRect.width) < 1
    && detailRect.top >= cardRect.bottom && detailRect.top - cardRect.bottom <= 14 && detail.scrollWidth <= detail.clientWidth + 2;
};
const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>("button")].find(el => visible(el) && (el.textContent?.trim() === label || el.querySelector("strong")?.textContent === label));
const click = async (label: string) => { await wait(() => button(label) && !button(label)!.disabled, `button ${label}`); button(label)!.focus(); button(label)!.click(); };
const settlePage = async () => {
  await wait(() => { const page = document.querySelector(".vua-page-enter"); return page && parseFloat(getComputedStyle(page).opacity) > 0.99; }, "page transition completed");
  document.querySelector(".vua-page")!.scrollTop = 0;
  await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
};

window.vua = {
  gateway: { invoke: async (req: { method: string; params?: { intent?: { purposes: string[] }; route?: string; urls?: string[]; path?: string }; }) => {
    let value: unknown;
    if (req.method === "app.snapshot") value = { capabilities: { tasks: true, operations: ["environment.planDeployment", "environment.executeDeployment", "environment.testWebsites", "environment.observePlay", "environment.startPlay", "environment.stopPlay", "environment.inspectManagerApps"].map(operationId => ({ operationId, availability: "available" })) } };
    if (req.method === "task.list") value = { revision: 1, tasks: [] };
    if (req.method === "environment.getSnapshot") value = { contractVersion: "0.1", revision: 0, capturedAt: new Date().toISOString(), items: [{ checkId: "unity_editors", zone: "create", presence: "detected", facts: { editors: editorVersions().map(version => ({ version, path: `C:/Synthetic/Unity/${version}`, classification: "other_unity_version" })) } }] };
    if (req.method === "environment.testWebsites") { websiteCalls += 1; value = { websiteTests: req.params!.urls!.map(url => ({ url, status: "reachable", elapsedMs: 8, httpStatus: 200 })) }; }
    if (["environment.observePlay", "environment.startPlay", "environment.stopPlay"].includes(req.method)) {
      if (playUnavailable) return { ok: false, error: { code: "unsupported_method", messageKey: "synthetic" } };
      const route = req.params!.route!;
      if (req.method === "environment.observePlay") playObservationCount += 1;
      if (req.method !== "environment.observePlay") { const stopping = req.method === "environment.stopPlay"; playCalls.push(req.method + ":" + route); playStates[route] = stopping ? "stopping" : "starting"; setTimeout(() => { playStates[route] = stopping ? "idle" : "running"; }, 100); }
      value = syntheticPlay(route);
    }
    if (req.method === "project.environmentManagers") value = { operation: req.method, result: { schemaVersion: "vua.environment-managers-snapshot/v0.1", editors: editorVersions().map(version => ({ version, path: `C:/Synthetic/Unity/${version}`, classification: "other_unity_version" })), vcc: { presence: "found" }, alcom: { presence: "not_found" } } };
    if (req.method === "environment.verifyEditor") value = { verdict: "verified", version: req.params!.path!.split("/").at(-1), exePath: req.params!.path! + "/Editor/Unity.exe", classification: "other_unity_version" };
    if (req.method === "environment.inspectManagerApps") value = { managerApps: { schemaVersion: "vua.manager-apps/v0.1", capturedAt: new Date().toISOString(), apps: ["unity_hub", "vcc", "alcom"].map(component => ({ component, presence: component === "alcom" ? "not_found" : "found", path: component === "alcom" ? null : `C:/Synthetic/${component}.exe` })) } };
    if (req.method === "environment.planDeployment") {
      intents.push([...req.params!.intent!.purposes]);
      value = { deploymentPlan: { schemaVersion: "vua.environment-deployment/v0.1", intent: req.params!.intent, digest: "a".repeat(64), prerequisitesReady: scenario === "installed", installer: null,
        steps: ["steam", "vrchat"].map(component => ({ component, action: scenario === "installed" ? "retain" : "manual_install", reason: scenario === "installed" ? "verified" : "missing", location: null, version: null,
          officialUrl: component === "steam" ? "https://store.steampowered.com/about/" : "https://store.steampowered.com/app/438100/" })) } };
    }
    if (req.method === "environment.checkScenarioEnvironment") {
      const intent = req.params!.intent!, ready = scenario === "installed";
      value = { schemaVersion: "vua.scenario-environment/v0.1", intent, softwareReady: ready, licenseState: "not_checked", performed: "software_check_only",
        nextAction: ready ? "follow_sop" : "plan_installation", installationStarted: false,
        observations: scenarioRequiredComponents(intent.purposes as DeploymentPurpose[]).map(component => ({ component, presence: ready ? "verified" : "missing", location: null, version: null })) };
    }
    if (req.method === "environment.executeDeployment") value = { schemaVersion: "vua.environment-deployment/v0.1", operation: req.method, taskId: "synthetic-deployment", correlationId: "synthetic" };
    if (req.method === "task.get") value = { contractVersion: "0.1", taskId: "synthetic-deployment", correlationId: "synthetic", revision: 1, state: installationRunning ? "running" : "succeeded_with_warnings", recoveryDisposition: "none", cancellationRequested: false, updatedAt: new Date().toISOString(),
      result: scenario === "restored" ? { outcome: "prerequisites_verified" } : { outcome: "manual_required", nextStep: { component: "vrchat", action: "manual_install", officialUrl: "https://store.steampowered.com/app/438100/" } } };
    return value ? { ok: true, value } : { ok: false, error: { code: "unsupported_method", messageKey: "synthetic" } };
  } }, events: { subscribe: () => () => {} },
  window: { showReader: async () => {}, showGameGuide: async () => {}, showOverlay: async () => {},
    showEncyclopedia: async (target: unknown) => { window.dispatchEvent(new CustomEvent("vua:open-encyclopedia", { detail: target })); return { visible: true }; }, encyclopediaTargetEvents: { subscribe: () => () => {} },
    shellCommandEvents: { subscribe: (listener: typeof shellCommand) => { shellCommand = listener; return () => { shellCommand = undefined; }; } } },
  system: {
    readResourceUsage: async () => ({ schemaVersion: 1, ramUsedBytes: 1024, ramTotalBytes: 4096, vramUsedBytes: null, vramTotalBytes: null, sampledAt: new Date().toISOString() }),
    checkUpdate: async () => { updateChecks += 1; return { schemaVersion: 1, state: "up-to-date", currentVersion: "synthetic", latestVersion: "synthetic", releaseUrl: null, checkedAt: new Date().toISOString() }; },
  },
  remoteContent: { openAccountGuideInBrowser: async (id: string) => { guideId = id; }, authProbe: async () => ({ authOk: false, accountName: null }) },
} as unknown as VuaDesktopApiV2;

let autoConfirmFirstRun = true;
async function completeFirstRun(keepGuide = false) {
  await wait(() => document.querySelector("[data-first-run-language]"));
  if (!autoConfirmFirstRun) return;
  const select = document.querySelector<HTMLSelectElement>(".vua-onboarding__language select")!;
  Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!.call(select, currentLocale);
  select.dispatchEvent(new Event("change", { bubbles: true }));
  await click(strings.onboarding.continue);
  await wait(() => document.querySelector(".vua-home") && !document.querySelector("[data-first-run-language]"));
  await wait(() => !document.querySelector(".vua-tour"));
  if (!keepGuide) {
    const guide = document.querySelector('[data-context-guide="home"]');
    if (guide) [...guide.querySelectorAll<HTMLButtonElement>("button")].find(button => button.textContent?.trim() === strings.contextGuide.dismiss)?.click();
  }
}
async function mount(mode: typeof scenario = "installed", legacyMode = "desktop", theme?: "dark" | "light", inventory: typeof creatorInventory = "all", keepGuide = false) {
  creatorInventory = inventory;
  const previousGoals = localStorage.getItem(storageKeys.goals);
  root?.unmount(); localStorage.clear(); sessionStorage.clear(); location.hash = ""; scenario = mode; playUnavailable = false; installationRunning = false; websiteCalls = 0; borrowedSteam = true; playStates.desktop_play = "idle"; playStates.pico_pcvr = "idle"; playCalls.length = 0;
  localStorage.setItem(storageKeys.locale, currentLocale);
  if (previousGoals !== null) localStorage.setItem(storageKeys.goals, previousGoals);
  if (theme) localStorage.setItem(storageKeys.theme, theme);
  localStorage.setItem(storageKeys.retiredDisplayMode, legacyMode);
  if (mode === "restored") {
    localStorage.setItem(storageKeys.goals, JSON.stringify({ version: 1, onboarding: "completed", goals: ["env"], environments: ["play"] }));
    localStorage.setItem(storageKeys.firstRunJourney, JSON.stringify({ v: 1, step: "prepare", purpose: "desktop_play", connection: null }));
    localStorage.setItem(storageKeys.deploymentReceiptPlay, JSON.stringify({ v: 1, taskId: "synthetic-deployment", startedAt: Date.now() - 60000, intent: { purposes: ["desktop_play"], editorRoot: "C:/Synthetic/Editor" } }));
  }
  root = createRoot(document.getElementById("root")!); root.render(<App />);
  await wait(() => document.querySelector(".vua-shell"));
  await wait(() => !document.querySelector(".vua-boot-splash"));
  if (mode === "restored" || previousGoals !== null) {
    await wait(() => !document.querySelector("[data-first-run-language]"));
    if (!keepGuide) {
      const guide = document.querySelector('[data-context-guide="home"]');
      if (guide) [...guide.querySelectorAll<HTMLButtonElement>("button")].find(button => button.textContent?.trim() === strings.contextGuide.dismiss)?.click();
    }
  } else await completeFirstRun(keepGuide);
}
async function prepare() {
  await click(copy.play);
  await click(copy.desktop);
  await wait(() => document.querySelector('[data-route-stage="prepare"]'));
  await click(strings.scenarioEnvironment.plan);
}
const review = {
  async run() {
    await mount("installed", "desktop", undefined, undefined, true);
    check("a new session defaults to Dark with the system set to Light", localStorage.getItem(storageKeys.theme) === "dark" && document.documentElement.dataset.theme === "dark");
    await wait(() => document.querySelector('[data-context-guide="home"]'));
    check("language-first startup has no mandatory tour or retired wizard", !document.querySelector(".vua-tour") && !document.querySelector("[data-first-run-language]"));
    check("language selection and page introductions request no deployment plan", intents.length === 0);
    await click(strings.contextGuide.later);
    check("later snoozes only for this session", localStorage.getItem(storageKeys.contextGuide) === null && sessionStorage.getItem(storageKeys.contextGuideSnoozed)?.includes("home"));
    await click(copy.help); await click(copy.wizard);
    await wait(() => document.querySelector(".vua-vua-guide"));
    await click(strings.contextGuide.replay);
    check("replaying page tips clears only their own state", localStorage.getItem(storageKeys.contextGuide) === null && sessionStorage.getItem(storageKeys.contextGuideSnoozed) === null && localStorage.getItem(storageKeys.theme) === "dark");
    document.querySelector<HTMLButtonElement>('[data-nav-id="logo-home"]')!.click();
    await wait(() => document.querySelector('[data-context-guide="home"]'));
    await click(strings.contextGuide.dismiss);
    check("dismissal is remembered without a wizard or forced tour", JSON.parse(localStorage.getItem(storageKeys.contextGuide)!).dismissed.home === 1 && !document.querySelector(".vua-tour"));
    await prepare();
    check("desktop plans only desktop prerequisites", intents.at(-1)?.join() === "desktop_play");
    await wait(() => document.querySelector('[data-card="route-desktop"]')?.dataset.action === "start");
    document.querySelector<HTMLButtonElement>('[data-nav-id="shell-settings"]')!.click();
    await wait(() => document.querySelector('[data-nav-id="nav-settings-accounts"]'));
    document.querySelector<HTMLButtonElement>('[data-nav-id="nav-settings-accounts"]')!.click();
    await wait(() => document.querySelector(".vua-account-grid")); await click(copy.linking);
    await click(copy.official); check("registration handoff sends a closed guide ID", guideId === "linking");
    check("settings sidebar contains only settings pages", document.querySelectorAll(".vua-shell__sidebar [data-module='settings']").length === 1 && !document.querySelector(".vua-shell__sidebar [data-module='env'], .vua-shell__sidebar [data-module='production']"));
    document.querySelector<HTMLButtonElement>('[data-nav-id="shell-back"]')!.click();
    await wait(() => document.querySelector('[data-route-stage="prepare"]'));
    check("account return preserves the originating environment workflow", !!document.querySelector('[data-route-stage="prepare"]'));
    document.querySelector<HTMLButtonElement>('[data-nav-id="logo-home"]')!.click();
    await wait(() => document.querySelector(".vua-home"));
    check("home is reachable without completing a global wizard", !document.querySelector("[data-first-run-language]"));
    check("only the logo is the Home action", document.querySelectorAll('[data-nav-id="logo-home"]').length === 1 && !document.querySelector(".vua-shell__tabs"));
    check("home has no tile subtitles", !document.querySelector(".vua-home .vua-route-tile__description"));
    check("header no longer repeats the page title beside the logo", !document.querySelector(".vua-shell__location strong"));
    const headerLabels = [...document.querySelectorAll(".vua-shell__header button")].map(el => el.textContent?.trim());
    check("header has no Tasks, theme or search button", [copy.tasks, strings.settings.theme.dark, strings.settings.theme.light, strings.commandPalette.cta].every(label => !headerLabels.some(text => text?.includes(label))));
    await wait(() => document.querySelector(".vua-shell__notify"));
    check("ready task capability retains the bell without mounting a bottom taskbar", !!document.querySelector(".vua-shell__notify") && !document.querySelector(".vua-shell__taskbar"));
    await mount("installed", "desktop");
    document.querySelector<HTMLButtonElement>('[data-nav-id="logo-home"]')!.click();
    await wait(() => !document.querySelector("[data-first-run-language]"));
    check("AMF remains inactive until explicit enablement", !document.querySelector('.vua-shell__sidebar [data-module="production"]') && !!document.querySelector('[data-nav-id="nav-enable-amf"]'));
    check("bottom-left duplicate settings and help entries are gone", !document.querySelector(".vua-shell__sidebar-global") && !document.querySelector('.vua-shell__sidebar [data-nav-id="nav-settings-theme"]'));
    check("desktop sidebar is narrower and retains frosted glass", document.querySelector(".vua-shell__sidebar")!.getBoundingClientRect().width === 176 && getComputedStyle(document.querySelector(".vua-shell__sidebar")!).backdropFilter.includes("blur"));
    check("feature search has no sidebar button outside Settings", !document.querySelector('[data-nav-id="settings-search"]'));
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "p", ctrlKey: true, bubbles: true }));
    await wait(() => document.querySelector(".vua-palette"));
    check("Ctrl+P still opens feature search outside Settings", !!document.querySelector(".vua-palette"));
    document.querySelector<HTMLInputElement>(".vua-palette__input")!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await wait(() => !document.querySelector(".vua-palette"));
    await click(copy.play); await click(copy.desktop);
    await wait(() => document.querySelector('[data-route-stage="prepare"]'));
    document.querySelector<HTMLButtonElement>('[data-nav-id="shell-settings"]')!.click();
    await wait(() => document.querySelector('[data-nav-id="nav-settings-accounts"]'));
    const search = document.querySelector<HTMLButtonElement>('[data-nav-id="settings-search"]')!;
    const sidebar = document.querySelector<HTMLElement>(".vua-shell__sidebar")!;
    check("Settings places its single search button at the sidebar bottom", search.closest(".vua-shell__sidebar-footer") && sidebar.lastElementChild === search.parentElement && Math.abs(search.getBoundingClientRect().bottom - (sidebar.getBoundingClientRect().bottom - parseFloat(getComputedStyle(sidebar).paddingBottom))) < 2);
    search.click(); await wait(() => document.querySelector(".vua-palette"));
    check("Settings search button opens the command palette", !!document.querySelector(".vua-palette"));
    document.querySelector<HTMLInputElement>(".vua-palette__input")!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await wait(() => !document.querySelector(".vua-palette"));
    check("appearance has ordered connected Dark / Light / System choices", [...document.querySelectorAll(".vua-theme-choice__button")].map(el => el.getAttribute("data-nav-id")).join() === "theme-dark,theme-light,theme-system,theme-hc-auto,theme-hc-on");
    await review.setThemePreference("light");
    check("Light applies immediately and only its appearance button is selected", document.documentElement.dataset.theme === "light" && document.querySelectorAll('.vua-theme-choice:not(.vua-theme-choice--pair) [aria-pressed="true"]').length === 1 && document.querySelector('[data-nav-id="theme-light"][aria-pressed="true"]'));
    await review.setThemePreference("system");
    check("System keeps its own saved selection even when resolved to Light", document.documentElement.dataset.theme === "light" && localStorage.getItem(storageKeys.theme) === "system" && document.querySelector('[data-nav-id="theme-system"][aria-pressed="true"]'));
    await review.setThemePreference("dark");
    document.querySelector<HTMLButtonElement>('[data-nav-id="nav-settings-accounts"]')!.click();
    await wait(() => document.querySelector(".vua-account-grid"));
    document.querySelector<HTMLButtonElement>('[data-nav-id="shell-settings"]')!.click();
    await wait(() => document.querySelector('[data-route-stage="prepare"]') && visible(document.querySelector('[data-route-stage="prepare"]')!));
    check("clicking Settings again returns to the source workflow without resetting its step", !!document.querySelector('[data-route-stage="prepare"]'));
    localStorage.setItem(storageKeys.updateCheckEnabled, "off");
    const beforeChecks = updateChecks;
    shellCommand!("check-updates");
    await wait(() => document.querySelector('[data-nav-id="nav-settings-about"][aria-current="page"]') && updateChecks > beforeChecks);
    check("tray update gesture opens About and checks even when automatic checks are disabled", updateChecks > beforeChecks);
    shellCommand!("bigscreen");
    await new Promise(resolve => requestAnimationFrame(resolve));
    check("legacy big-screen gesture cannot resurrect the retired mode", !document.querySelector('.vua-shell[data-display-mode]') && !!document.querySelector('[data-nav-id="nav-settings-about"][aria-current="page"]'));
    document.querySelector<HTMLButtonElement>('[data-nav-id="shell-back"]')!.click();
    await wait(() => !document.querySelector('[data-nav-id="nav-settings-about"]'));
    check("tray-opened settings retains the same source page", !!document.querySelector('[data-route-stage="prepare"]'));
    await mount("missing"); await prepare(); await wait(() => button(strings.deployment.execute));
    check("missing software cannot continue", document.querySelector('[data-card="route-desktop"]')?.dataset.action !== "start");
    await click(strings.deployment.execute); await wait(() => document.body.textContent?.includes(strings.deployment.manualRequired));
    check("manual handoff cannot imply ready", document.querySelector('[data-card="route-desktop"]')?.dataset.action !== "start");
    scenario = "installed"; await click(strings.scenarioEnvironment.plan);
    await wait(() => document.querySelector('[data-card="route-desktop"]')?.dataset.action === "start");
    check("fresh detection enables continuation", document.querySelector('[data-card="route-desktop"]')?.dataset.action === "start");
    const observationsBeforeRestore = playObservationCount;
    await mount("restored"); await click(copy.play); await click(copy.desktop);
    await wait(() => playObservationCount > observationsBeforeRestore && document.querySelector('[data-card="route-desktop"]')?.dataset.action === "start", "fresh observation after restoring a receipt");
    check("an old terminal receipt cannot replace fresh observation or launch software", playObservationCount > observationsBeforeRestore
      && document.querySelector('[data-card="route-desktop"]')?.dataset.action === "start" && playCalls.length === 0);
    scenario = "installed"; await click(strings.scenarioEnvironment.plan);
    await wait(() => document.querySelector('[data-card="route-desktop"]')?.dataset.action === "start");
    check("restored work remains usable after an explicit environment review", document.querySelector('[data-card="route-desktop"]')?.dataset.action === "start" && playCalls.length === 0);
    await mount("installed", "bigscreen");
    await wait(() => document.querySelector('[data-nav-id="home-help"]'));
    check("a saved big-screen preference is cleared and retains the ordinary Home directory", localStorage.getItem(storageKeys.retiredDisplayMode) === null && !document.querySelector('.vua-shell[data-display-mode]'));
    await click(copy.help); document.querySelector<HTMLButtonElement>('[data-nav-id="help-tour"]')!.click();
    await wait(() => document.querySelector('[data-nav-id="help-start-tour"]'));
    document.querySelector<HTMLButtonElement>('[data-nav-id="help-start-tour"]')!.click();
    for (let index = 0; index < TOUR_STEPS.findIndex(step => step.id === "tasks"); index += 1) {
      await wait(() => document.querySelector('.vua-tour .vua-button--primary'));
      document.querySelector<HTMLButtonElement>('.vua-tour .vua-button--primary')!.click();
      await wait(() => JSON.parse(localStorage.getItem(storageKeys.tourProgress)!).step === index + 1);
    }
    await wait(() => document.querySelector('[data-nav-id="shell-notifications"]') && document.querySelector(".vua-tour__highlight") && document.querySelector(".vua-tour")?.textContent?.includes(strings.tour.steps.tasks.body), "notification tour page and highlight");
    const taskTile = document.querySelector('[data-nav-id="shell-notifications"]')!.getBoundingClientRect();
    const taskHighlight = document.querySelector(".vua-tour__highlight")!.getBoundingClientRect();
    check("explicit task-tour step highlights the notification entry", document.querySelector(".vua-tour")?.textContent?.includes(strings.tour.steps.tasks.body) && Math.abs((taskTile.x + taskTile.width / 2) - (taskHighlight.x + taskHighlight.width / 2)) < 1 && Math.abs((taskTile.y + taskTile.height / 2) - (taskHighlight.y + taskHighlight.height / 2)) < 1);
    await mount();
    return checks;
  },
  async runEnvironmentCards() {
    const startAt = checks.length;
    await mount("installed", "desktop");  document.querySelector<HTMLButtonElement>('[data-nav-id="logo-home"]')!.click(); await wait(() => !document.querySelector("[data-first-run-language]"));
    check("Software & connections is absent from the fixed directory", !document.querySelector('[data-nav-id="nav-software"]') && !document.querySelector('[data-nav-id="home-software"]'));
    await click(copy.play); const card = () => document.querySelector<HTMLElement>('[data-card="route-desktop"]')!;
    await wait(() => card()?.dataset.action === "start", "desktop card ready to start");
    const halves = card().querySelectorAll<HTMLButtonElement>(":scope > button"); const network = document.querySelector<HTMLElement>(".vua-network-tile")!;
    const separatorWidth = parseFloat(getComputedStyle(halves[1]!).borderLeftWidth);
    check("play card has two independent halves and a thin separator", halves.length === 2 && !halves[0]!.disabled && separatorWidth > 0 && separatorWidth <= 1);
    check("network is above the environments and has the same height", network.getBoundingClientRect().bottom <= card().getBoundingClientRect().top && Math.abs(network.getBoundingClientRect().height - card().getBoundingClientRect().height) < 1);
    check("entering Play makes no website test requests", websiteCalls === 0);
    await wait(() => !document.querySelector<HTMLButtonElement>('[data-nav-id="play-network-test"]')!.disabled);
    check("regional reference testing is explicit and keeps both US coasts", network.textContent?.includes(strings.websiteTests.regionTitle) && network.textContent?.includes(strings.websiteTests.regionShort.eu) && network.textContent?.includes(strings.websiteTests.regionShort.usw) && network.textContent?.includes(strings.websiteTests.regionShort.use));
    document.querySelector<HTMLButtonElement>('[data-nav-id="play-network-details"]')!.click();
    await wait(() => visible(document.querySelector(".vua-network")!));
    document.querySelector<HTMLButtonElement>(`.vua-network [aria-label="${strings.websiteTests.testAll}"]`)!.click(); await wait(() => websiteCalls === 1 && document.querySelector(".vua-network")!.textContent?.includes("ms"));
    check("website tests live only in expanded details and cannot become regional ping", websiteCalls === 1 && !network.textContent?.includes("ms"));
    document.querySelector<HTMLButtonElement>('[data-nav-id="route-desktop"]')!.click(); await wait(() => document.querySelector('[data-route-stage="prepare"]'));
    check("left half opens details without launching software", playCalls.length === 0 && document.querySelector(".vua-environment-software"));
    document.querySelector<HTMLButtonElement>('[data-nav-id="route-desktop-action"]')!.click(); await wait(() => card().dataset.action === "stop");
    check("pending start offers scoped close rather than a second start", card().querySelector(".vua-environment-card__action")!.getAttribute("aria-label") === format(strings.environmentCards.stop, { name: card().querySelector(".vua-environment-card__details strong")!.textContent ?? "" }));
    await wait(() => card().textContent?.includes(strings.environmentCards.close));
    check("observed running software identifies pre-existing Steam as retained", document.querySelector(".vua-environment-software")!.textContent?.includes(strings.environmentCards.borrowed));
    document.querySelector<HTMLButtonElement>('[data-nav-id="route-desktop-action"]')!.click(); await wait(() => card().dataset.action === "start", "close returns the card to start");
    check("close returns the environment to play after observation", playCalls.join() === "environment.startPlay:desktop_play,environment.stopPlay:desktop_play");
    borrowedSteam = false;
    document.querySelector<HTMLButtonElement>('[data-nav-id="route-desktop-action"]')!.click(); await wait(() => playStates.desktop_play === "running", "owned session running");
    playStates.desktop_play = "finished";
    await click(strings.environmentCards.inspect);
    await wait(() => card().textContent?.includes(strings.environmentCards.close) && document.querySelector(".vua-environment-software li")!.textContent?.includes(strings.environmentCards.owned)
      && document.querySelectorAll(".vua-environment-software li")[1]!.querySelectorAll("span").length === 1, "finished session observed with only owned Steam still running");
    check("normal game exit keeps Close for owned Steam without a warning or failure", card().dataset.action === "stop" && card().textContent?.includes(strings.environmentCards.close) && !card().querySelector(".vua-environment-card__warning") && !document.querySelector(".vua-play-page [role=alert]"));
    playStates.desktop_play = "idle";
    await click(strings.environmentCards.inspect); await wait(() => card().dataset.action === "start", "all owned software exited");
    check("exit of all owned software restores Play without an extra close request", playCalls.filter(call => call.startsWith("environment.stopPlay:")).length === 1);
    check("Quest, Vive and Index remain development peers", ["quest", "vive", "index"].every(id => document.querySelector(`[data-card="route-${id}"] .vua-route-tile__tag`)?.textContent === copy.developing));
    playUnavailable = true; await click(strings.environmentCards.inspect); await wait(() => card().dataset.action === "unknown", "provider failure shown as unknown");
    check("provider failure is unknown instead of missing or ready", card().textContent?.includes(strings.environmentCards.unknown));
    await click(copy.create); await wait(() => document.querySelector('[data-nav-id="editor-2019.4.31f1"]'));
    check("creator lists complete discovered versions beside the fixed Unity entries", document.querySelector('[data-nav-id="route-unity2022"]') && document.querySelector('[data-nav-id="editor-2021.3.45f1"]') && document.querySelector('[data-nav-id="route-unity6"] .vua-route-tile__tag')?.textContent === copy.developing);
    check("Hub, VCC and ALCOM have separate detected statuses", ["unity_hub", "vcc", "alcom"].every(id => document.querySelector(`[data-manager="${id}"] [role="status"]`)) && document.querySelector('[data-manager="alcom"]')!.textContent?.includes(strings.environmentCards.notFound));
    document.querySelector<HTMLButtonElement>('[data-nav-id="manager-vcc"]')!.click(); await wait(() => !document.querySelector<HTMLElement>("#manager-vcc-details")!.hidden);
    check("manager configuration is shown separately from executable detection", document.querySelector("#manager-vcc-details")!.textContent?.includes(strings.environmentCards.configFound));
    document.querySelector<HTMLButtonElement>('[data-nav-id="manager-vcc"]')!.click();
    for (const control of document.querySelectorAll<HTMLButtonElement>('.vua-creator-page .vua-card-entry [aria-controls]')) {
      control.click();
      await wait(() => control.getAttribute('aria-expanded') === 'true');
      check(`${control.dataset.navId} opens details immediately below its own card`, detailsBelowCard(control));
      control.click();
      await wait(() => control.getAttribute('aria-expanded') === 'false');
    }
    await mount("missing", "desktop");  document.querySelector<HTMLButtonElement>('[data-nav-id="logo-home"]')!.click(); await wait(() => !document.querySelector("[data-first-run-language]")); await click(copy.play);
    await wait(() => card()?.dataset.action === "prepare"); document.querySelector<HTMLButtonElement>('[data-nav-id="route-desktop-action"]')!.click();
    await wait(() => document.querySelector(".vua-play-page .vua-scenario-preparation [role=status]"), "plus shows the software check");
    check("plus checks software without planning, installing or launching", playCalls.length === 0 && !button(strings.deployment.execute));
    await click(strings.scenarioEnvironment.plan); await wait(() => button(strings.deployment.execute), "installation plan after an explicit request");
    check("the installation plan follows an explicit request without launching software", playCalls.length === 0 && !!button(strings.deployment.execute));
    installationRunning = true; await click(strings.deployment.execute); await wait(() => card().dataset.action === "busy");
    check("accepted installation shows a spinner and cannot launch", card().querySelector(".vua-environment-card__spin") && card().textContent?.includes(strings.environmentCards.installing) && document.querySelector<HTMLButtonElement>(".vua-play-page .vua-environment-detail:not([hidden]) .vua-route-platform > button")!.disabled);
    check("details remain available while installation is running", !card().querySelector<HTMLButtonElement>(".vua-environment-card__details")!.disabled);
    const panel = document.querySelector('#route-desktop-details .vua-deployment');
    const receipt = localStorage.getItem(storageKeys.deploymentReceiptPlay);
    document.querySelector<HTMLButtonElement>('[data-nav-id="route-desktop"]')!.click();
    await wait(() => document.querySelector<HTMLElement>('#route-desktop-details')!.hidden);
    document.querySelector<HTMLButtonElement>('[data-nav-id="route-pico"]')!.click();
    await wait(() => !document.querySelector<HTMLElement>('#route-pico-details')!.hidden);
    document.querySelector<HTMLButtonElement>('[data-nav-id="route-desktop"]')!.click();
    await wait(() => !document.querySelector<HTMLElement>('#route-desktop-details')!.hidden);
    check("closing and switching card details retains the live installation panel and receipt", panel === document.querySelector('#route-desktop-details .vua-deployment') && receipt === localStorage.getItem(storageKeys.deploymentReceiptPlay) && card().dataset.action === 'busy');
    return checks.slice(startAt);
  },
  async previewEnvironment(page: "play" | "create", inventory: typeof creatorInventory = "all") {
    await mount("installed", "desktop", previewTheme, inventory);  document.querySelector<HTMLButtonElement>('[data-nav-id="logo-home"]')!.click(); await wait(() => !document.querySelector("[data-first-run-language]"));
    await click(copy[page]);
    await wait(() => page === "play" ? document.querySelector<HTMLElement>('[data-card="route-desktop"]')?.dataset.action === "start" : document.querySelector('[data-editor-status]')?.textContent?.includes("2022.3.22f1"));
    await settlePage();
  },
  async previewSettings() {
    await mount("installed", "desktop", previewTheme);
    document.querySelector<HTMLButtonElement>('[data-nav-id="logo-home"]')!.click();
    await wait(() => !document.querySelector("[data-first-run-language]"));
    document.querySelector<HTMLButtonElement>('[data-nav-id="shell-settings"]')!.click();
    await wait(() => location.hash === "#settings-theme" && document.querySelector(".vua-theme-choice"));
    await settlePage();
    check("settings capture owns the settings-theme route and actual appearance controls",
      location.hash === "#settings-theme" && document.querySelectorAll(".vua-theme-choice").length === 2
      && document.querySelectorAll(".vua-theme-choice__button").length === 5
      && !document.querySelector("[data-first-run-language]"));
    check("settings capture preserves its requested appearance preference",
      document.documentElement.dataset.theme === previewTheme
      && document.querySelector(`[data-nav-id="theme-${previewTheme}"][aria-pressed="true"]`));
    check("theme settings groups family, appearance, contrast and resource preferences",
      document.querySelectorAll(".vua-settings-group .vua-preference-row").length === 4
      && document.querySelectorAll(".vua-theme-family__option").length === 4
      && document.querySelectorAll('.vua-settings-group [role="switch"]').length === 1
      && !document.querySelector(".vua-settings-group .vua-button--primary"));
    const previews = [...document.querySelectorAll<HTMLImageElement>(".vua-theme-family__preview .vua-skin-motif")];
    await wait(() => previews.every(image => image.complete && image.naturalWidth > 0), "skin preview assets");
    check("each non-Grid preset renders its own motif and Grid keeps its color swatches",
      previews.length === 3 && previews.every(image => image.complete && image.naturalWidth > 0)
      && new Set(previews.map(image => image.dataset.motifFamily)).size === 3
      && document.querySelectorAll('.vua-theme-family__preview[data-theme-family="grid"] > span').length === 3);
    const initialFamily = document.documentElement.dataset.themeFamily === "frutiger-aero" ? "frutiger-aero" : "grid";
    const nextFamily = initialFamily === "grid" ? "frutiger-aero" : "grid";
    const initialMode = document.documentElement.dataset.theme;
    const storedFamily = localStorage.getItem(storageKeys.themeFamily);
    const storedMode = localStorage.getItem(storageKeys.theme);
    const familyOverride = new URLSearchParams(location.search).has("themeFamily");
    document.querySelector<HTMLButtonElement>(`[data-nav-id="theme-family-${nextFamily}"]`)!.click();
    await wait(() => document.documentElement.dataset.themeFamily === nextFamily
      && document.querySelector(`[data-nav-id="theme-family-${nextFamily}"][aria-pressed="true"]`));
    const storedFamilyMatches = familyOverride
      ? localStorage.getItem(storageKeys.themeFamily) === storedFamily
      : localStorage.getItem(storageKeys.themeFamily) === nextFamily;
    check("theme family switches independently and preserves preview and storage semantics",
      storedFamilyMatches && document.documentElement.dataset.theme === initialMode);
    document.querySelector<HTMLButtonElement>(`[data-nav-id="theme-family-${initialFamily}"]`)!.click();
    await wait(() => document.documentElement.dataset.themeFamily === initialFamily
      && document.querySelector(`[data-nav-id="theme-family-${initialFamily}"][aria-pressed="true"]`));
    for (const [family, expectedMode] of [["tenshi", "light"], ["jirai", "dark"]] as const) {
      document.querySelector<HTMLButtonElement>(`[data-nav-id="theme-family-${family}"]`)!.click();
      await wait(() => document.documentElement.dataset.themeFamily === family
        && document.documentElement.dataset.theme === expectedMode
        && document.querySelector(`[data-nav-id="theme-${expectedMode}"][aria-pressed="true"]`)
        && localStorage.getItem(storageKeys.theme) === (familyOverride ? storedMode : expectedMode)
        && localStorage.getItem(storageKeys.themeFamily) === (familyOverride ? storedFamily : family),
        `${family} appearance and storage`);
      check(`${family} locks its supported appearance without changing unrelated preferences`,
        (["dark", "light", "system"] as const).every(mode => document.querySelector<HTMLButtonElement>(`[data-nav-id="theme-${mode}"]`)!.disabled)
        && document.querySelector(".vua-theme-choice")?.getAttribute("aria-disabled") === "true"
        && localStorage.getItem(storageKeys.theme) === (familyOverride ? storedMode : expectedMode)
        && localStorage.getItem(storageKeys.themeFamily) === (familyOverride ? storedFamily : family));
      await wait(() => {
        const motif = document.querySelector<HTMLImageElement>(".vua-shell__skin-motif");
        return motif?.dataset.motifFamily === family && motif.complete && motif.naturalWidth > 0;
      }, `${family} shell motif`);
    }
    document.querySelector<HTMLButtonElement>(`[data-nav-id="theme-family-${initialFamily}"]`)!.click();
    await wait(() => document.documentElement.dataset.themeFamily === initialFamily
      && [...document.querySelectorAll<HTMLButtonElement>(".vua-theme-choice__button")].every(button => !button.disabled));
    if (initialMode === "light" || initialMode === "dark") await review.setThemePreference(initialMode);
    await wait(() => {
      const motif = document.querySelector<HTMLImageElement>(".vua-shell__skin-motif");
      return initialFamily === "grid" ? motif === null
        : motif?.dataset.motifFamily === initialFamily && motif.complete && motif.naturalWidth > 0;
    }, "restored shell motif");
    const toggle = document.querySelector<HTMLButtonElement>('.vua-settings-group [role="switch"]')!;
    const activeMotif = document.querySelector<HTMLImageElement>(".vua-shell__skin-motif");
    const previous = toggle.getAttribute("aria-checked");
    toggle.click();
    await wait(() => toggle.getAttribute("aria-checked") !== previous);
    check("resource-saving removes skin motifs and decorative canvas textures",
      (!activeMotif || getComputedStyle(activeMotif).display === "none")
      && !getComputedStyle(document.body).backgroundImage.includes("url(")
      && !getComputedStyle(document.body).backgroundImage.includes("gradient"));
    toggle.click();
    await wait(() => toggle.getAttribute("aria-checked") === previous);
    document.querySelector<HTMLButtonElement>('[data-nav-id="theme-hc-on"]')!.click();
    await wait(() => document.documentElement.dataset.hc === "on");
    check("high contrast removes skin motifs and decorative canvas textures",
      (!activeMotif || getComputedStyle(activeMotif).display === "none")
      && !getComputedStyle(document.body).backgroundImage.includes("url(")
      && !getComputedStyle(document.body).backgroundImage.includes("gradient"));
    document.querySelector<HTMLButtonElement>('[data-nav-id="theme-hc-auto"]')!.click();
    await wait(() => !document.documentElement.dataset.hc);
    await settlePage();
  },
  async previewTools() {
    await mount("installed", "desktop", previewTheme);
    document.querySelector<HTMLButtonElement>('[data-nav-id="logo-home"]')!.click();
    await wait(() => !document.querySelector("[data-first-run-language]"));
    await click(copy.tools);
    await wait(() => document.querySelector(".vua-tools-hub"));
    await settlePage();
  },
  settlePage,
  async beforeKey(key: string) {
    if (key === "Enter") {
      autoConfirmFirstRun = false;
      localStorage.removeItem(storageKeys.goals);
      await mount();
      autoConfirmFirstRun = true;
      await wait(() => document.querySelector('[data-nav-id="first-run-language-continue"]'), "language confirmation for native Enter");
      sessionStorage.setItem("vua-test-native-first-run-enter", "pending");
      document.querySelector<HTMLButtonElement>('[data-nav-id="first-run-language-continue"]')!.focus();
    }
  },
  async afterKey(key: string) {
    if (key === "Enter") {
      const confirmed = sessionStorage.getItem("vua-test-native-first-run-enter") === "pending"
        && JSON.parse(localStorage.getItem(storageKeys.goals) ?? "null")?.onboarding === "completed";
      await mount();
      check("native Enter confirms language and reaches Home after the actual reload", confirmed
        && !!document.querySelector(".vua-home") && !document.querySelector("[data-first-run-language]") && !document.querySelector(".vua-tour"));
    } else await wait(() => !document.querySelector(".vua-shell[data-display-mode]"));
    return `Chromium ${key}: ${key === "Enter" ? "language confirmation reaches Home" : "no retired wizard or directional shell navigation"}`;
  },
  async setThemePreference(theme: "light" | "dark" | "system") {
    if (!document.querySelector('[data-nav-id="nav-settings-theme"]')) {
      document.querySelector<HTMLButtonElement>('[data-nav-id="shell-settings"]')!.click();
    } else if (!document.querySelector('[data-nav-id="nav-settings-theme"][aria-current="page"]')) {
      document.querySelector<HTMLButtonElement>('[data-nav-id="nav-settings-theme"]')!.click();
    }
    await wait(() => document.querySelector(`[data-nav-id="theme-${theme}"]`));
    document.querySelector<HTMLButtonElement>(`[data-nav-id="theme-${theme}"]`)!.click();
    await wait(() => localStorage.getItem(storageKeys.theme) === theme);
    await wait(() => {
      const selected = document.querySelector('.vua-theme-choice [aria-pressed="true"]')!.getBoundingClientRect();
      const frame = document.querySelector(".vua-theme-choice__selection")!.getBoundingClientRect();
      return Math.abs(selected.x - frame.x) < 1 && Math.abs(selected.width - frame.width) < 1;
    }, "appearance selection frame matches its button");
  },
  async setResourceSaver(enabled: boolean) {
    const toggle = document.querySelector<HTMLButtonElement>('.vua-preference-row [role="switch"]');
    if (!toggle) throw new Error("Resource-saving preference switch is missing");
    if ((toggle.getAttribute("aria-checked") === "true") !== enabled) toggle.click();
    await wait(() => (document.documentElement.dataset.effects === "off") === enabled);
  },
  expectFlattenedThemeMotion(mode: "reduced-motion" | "resource-saving") {
    const style = getComputedStyle(document.querySelector(".vua-theme-choice__selection")!);
    const flattened = mode === "reduced-motion" ? !style.transitionProperty.includes("transform") : parseFloat(style.transitionDuration) <= 0.001;
    if (!flattened) throw new Error(`Theme frame does not flatten for ${mode}`);
    return `Appearance selection remains usable with ${mode}`;
  },
  async expectSearchFooterVisible() {
    await wait(() => window.innerWidth === 960 && window.innerHeight === 600);
    const search = document.querySelector('[data-nav-id="settings-search"]')!.getBoundingClientRect();
    const categories = document.querySelector(".vua-shell__sidebar-group--settings")!;
    const language = document.querySelector('[data-nav-id="settings-language-select"]')!.getBoundingClientRect();
    if (search.bottom > window.innerHeight || search.top < 0 || language.bottom + 12 > search.top || categories.getBoundingClientRect().bottom > language.top) throw new Error("Language and Search overlap Settings categories in a small window");
    return "Small Settings keeps language and Search separate below its categories";
  },
};
(window as unknown as { firstRunReview: typeof review }).firstRunReview = review;
