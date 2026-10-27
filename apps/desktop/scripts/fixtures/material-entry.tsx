// Chromium regression using the production adapters and synthetic wire records.
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import original from "../../../../schemas/library-view/v0.1/examples/local-source.response.json";
import { strings, currentLocale } from "../../src/renderer/i18n/index.ts";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";
import "../../src/renderer/app-shell.css";

const library = structuredClone(original) as any;
document.documentElement.lang = currentLocale;
// Keep fixture screenshots deterministic; animation behavior is tested separately.
document.documentElement.dataset.effects = "off";
const product = library.items[0];
product.product.title = "Synthetic Avatar";
const local = library.items[1];
local.entry.displayName = "Local Avatar";
local.entry.artifacts[0].relativePath = "Avatar.unitypackage";
local.sourceMatch.basis = "mapping";
local.metadata = { schemaVersion: "0.1", entryId: local.entry.warehouseItemId, revision: 1,
  displayName: "Local Avatar", productId: "booth:90", thumbnailRef: null };
const savedMetadata = structuredClone(local.metadata), savedSource = structuredClone(local.sourceMatch);
const outfit = structuredClone(local);
outfit.entry.warehouseItemId = "wh-outfit"; outfit.entry.folderName = "wh-outfit";
outfit.entry.displayName = "Local outfit"; outfit.entry.artifacts[0].relativePath = "Avatar_outfit.unitypackage";
delete outfit.metadata; delete outfit.sourceMatch;
const duplicate = structuredClone(local);
duplicate.entry.warehouseItemId = "wh-duplicate"; duplicate.entry.folderName = "wh-duplicate";
duplicate.entry.displayName = "Another copy"; duplicate.metadata.entryId = "wh-duplicate";
duplicate.metadata.displayName = "Another copy";
let rows = [product, local, outfit, duplicate];
const calls: any[] = [];
let navigated = "";
let failPrepare = false;
let hasEditor = true;
let hasAvatar = true;
let amfEnabled = true;
const invoke = async (request: any) => {
  calls.push(structuredClone(request));
  const { method, params } = request;
  if (method === "library.list") {
    const offset = params.offset ?? 0, limit = params.limit ?? 50;
    return { ok: true, value: { ...library, items: rows.slice(offset, offset + limit), total: rows.length, offset, limit } };
  }
  if (method === "catalog.detail") return { ok: true, value: { schemaVersion: "0.6", operation: method, result: { product: {
    ...product.product, productId: params.productId, description: "Synthetic official description", sourceCategory: "Recorded category",
    imageUrls: [], videoUrls: [], subproducts: [],
  } } } };
  if (method === "warehouse.listEntries") return { ok: true, value: { schemaVersion: "0.6", operation: method, result: { entries: rows.filter(row => row.kind === "local").map(row => row.entry) } } };
  if (method === "warehouse.entryDetail") {
    const entry = rows.find(row => row.entry?.warehouseItemId === params.warehouseItemId)!.entry;
    return { ok: true, value: { schemaVersion: "0.6", operation: method, result: { entry: { ...entry,
      artifacts: entry.artifacts.map((artifact: any) => ({ ...artifact, suggestedFileName: null, inspectedAt: null,
        rejectionReason: null, sourceCorrelated: false, mappedProductIds: [] })),
    } } } };
  }
  if (method === "amf.production.list") return { ok: true, value: { schemaVersion: "0.1", capacity: 1, running: [], queue: [], busy: false, runs: [],
    editors: hasEditor ? [{ path: "C:/Synthetic/Unity.exe", version: "2022.3.22f1" }] : [],
    sources: rows.filter(row => row.kind === "local" && (hasAvatar || row !== local)).map(row => ({ sourceId: `src-${row.entry.warehouseItemId}`,
      itemId: row.entry.warehouseItemId, itemName: row.entry.displayName, name: row.entry.artifacts[0].relativePath })),
  } };
  if (method === "amf.production.prepare") {
    if (failPrepare) throw Error("synthetic preparation failure");
    return { ok: true, value: { schemaVersion: "0.1", runId: "synthetic-run", taskId: "synthetic-task" } };
  }
  if (method === "terms.product") return { ok: false, error: { code: "unavailable" } };
  throw Error(`Unexpected synthetic method: ${method}`);
};
Object.defineProperty(window, "vua", { configurable: true, value: { gateway: { invoke }, events: { subscribe: () => () => {} } } });
const { GatewayProvider, emptyGateway, createGatewayClient } = await import("../../src/renderer/gateway/index.ts");
const { createLiveAcquire } = await import("../../src/renderer/gateway/live-acquire-port.ts");
const { createAmfProductionPort } = await import("../../src/renderer/gateway/amf-production-port.ts");
const { WarehousePage } = await import("../../src/renderer/features/warehouse/WarehousePage.tsx");
const client = createGatewayClient(window.vua);
const gateway = { ...emptyGateway(), dataSource: () => "live" as const, acquire: createLiveAcquire(client), amfProduction: createAmfProductionPort(client) };
gateway.amfModule = { ...gateway.amfModule, snapshot: async () => ({ schemaVersion: "0.1", moduleId: "amf", installed: true, state: amfEnabled ? "ready" : "disabled" }) };
const root = createRoot(document.getElementById("root")!);
const pause = () => new Promise<void>(resolve => setTimeout(resolve, 30));
const text = () => document.body.textContent ?? "";
const checks: string[] = [];
const check = (condition: unknown, label: string) => { if (!condition) throw Error(label); checks.push(label); };
async function until(predicate: () => boolean, label: string) {
  const deadline = performance.now() + 5000;
  while (!predicate()) { if (performance.now() > deadline) throw Error(`${label}: ${text()}`); await pause(); }
}
const startButton = () => document.querySelector<HTMLButtonElement>(".vua-warehouse-detail__start");
const drawerText = () => document.querySelector(".vua-warehouse__drawer")?.textContent ?? "";
const inViewport = (element: Element | null) => {
  const bounds = element?.getBoundingClientRect();
  return bounds !== undefined && bounds.width > 0 && bounds.height > 0 && bounds.top >= 0 && bounds.bottom <= innerHeight;
};
const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>("button")].find(node => node.textContent?.trim() === label)!;
let generation = 0;
async function open(id: string) {
  const version = ++generation;
  root.render(<StrictMode><div key={version} data-review-generation={version} className="vua-shell" style={{ height: "100vh", padding: 24 }}><GatewayProvider gateway={gateway}><WarehousePage onNavigate={page => { navigated = page; }} /></GatewayProvider></div></StrictMode>);
  await until(() => document.querySelector(`[data-review-generation="${version}"]`) !== null, "new fixture mounts");
  await until(() => document.querySelector(`[data-selection-id="${id}"]`) !== null, "library mounts");
  document.querySelector(`[data-selection-id="${id}"]`)!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  await until(() => document.querySelector(".vua-warehouse__drawer") !== null && !document.querySelector(".vua-warehouse__drawer .vua-skeleton"), "detail mounts");
  await pause();
}
async function probe() {
  await open(local.entry.warehouseItemId);
  await until(() => text().includes(strings.warehouse.acquire.previewNoAssociation) || text().includes(strings.warehouse.acquire.previewNoImages), "preview resolved");
  const localEntry = !!startButton();
  const linkedSourceDenied = text().includes(strings.warehouse.acquire.previewNoAssociation);
  await open("booth:90");
  return { localEntry, linkedSourceDenied, unclassifiedProductEntry: !!startButton() };
}
async function run() {
  const observed = await probe();
  check(observed.localEntry, "local materials have an explicit Avatar entry");
  check(!observed.linkedSourceDenied, "saved entry source is not reported as unassociated");
  check(observed.unclassifiedProductEntry, "live catalog detail with empty entities permits choosing its Avatar role");
  await open(local.entry.warehouseItemId);
  check(inViewport(startButton()), "local Avatar action is visible in the details column");
  check(drawerText().includes(strings.warehouse.libraryState.contentUnverified), "source association retains unverified-content disclosure in the detail panel");
  startButton()!.click();
  await until(() => !!document.querySelector(".vua-start__picks"), "start dialog resolves local materials");
  check(calls.every(call => call.method !== "amf.production.prepare"), "opening the dialog never starts Unity work");
  document.querySelector<HTMLInputElement>('.vua-start__pick input')!.click(); await pause();
  button(strings.factory.materialsStart.startFull).click();
  await until(() => navigated === "workshop", "accepted preparation opens its workshop");
  const prepared = calls.find(call => call.method === "amf.production.prepare");
  check(prepared.params.sources[0].sourceId === "src-wh-imported" && prepared.params.sources[0].role === "avatar"
    && !prepared.params.sources.some((source: any) => source.sourceId === "src-wh-duplicate"), "local selection uses the exact entry, not every copy of its BOOTH product");
  delete local.metadata; delete local.sourceMatch; rows = [local, outfit]; calls.length = 0; navigated = "";
  await open(local.entry.warehouseItemId); startButton()!.click();
  await until(() => !!document.querySelector(".vua-start__picks"), "unlinked local Avatar needs no BOOTH identity");
  document.querySelector<HTMLInputElement>('.vua-start__pick input')!.click(); await pause();
  failPrepare = true; button(strings.factory.materialsStart.startFull).click();
  await until(() => text().includes(strings.factory.materialsStart.startFailed), "preparation failure stays visible");
  check(navigated === "" && !!document.querySelector(".vua-start"), "failed preparation keeps the selected materials and does not navigate");
  failPrepare = false; button(strings.factory.materialsStart.startFull).click();
  await until(() => navigated === "workshop", "explicit retry succeeds");
  hasEditor = false; await open(local.entry.warehouseItemId); startButton()!.click();
  await until(() => text().includes(strings.amfProduction.editorMissing), "missing Editor is explained");
  check(button(strings.factory.materialsStart.startFull).disabled, "missing Editor cannot start a line");
  hasEditor = true; hasAvatar = false; await open(local.entry.warehouseItemId); startButton()!.click();
  await until(() => text().includes(strings.factory.materialsStart.notSourceTitle), "unsupported or missing local source is explained");
  check(button(strings.factory.materialsStart.startFull).disabled, "non-admitted materials cannot start production");
  amfEnabled = false; await open(local.entry.warehouseItemId);
  check(startButton() === null, "disabled AMF does not offer a production action");
  await showLocal();
  return checks;
}
async function showLocal() {
  amfEnabled = true; hasAvatar = true; hasEditor = true;
  local.metadata = savedMetadata; local.sourceMatch = savedSource; rows = [product, local, outfit, duplicate];
  await open(local.entry.warehouseItemId);
  await until(() => drawerText().includes(strings.warehouse.acquire.previewNoImages), "linked source preview resolves");
  await new Promise(requestAnimationFrame);
}
async function showStart() {
  startButton()!.click();
  await until(() => !!document.querySelector(".vua-start__picks"), "confirmation is ready");
  await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
}
(window as any).materialEntryReview = { probe, run, showLocal, showStart };
