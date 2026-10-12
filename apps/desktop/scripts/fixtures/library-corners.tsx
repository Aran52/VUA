// Real DOM with synthetic public search/download facts; no real account, files or network.
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import original from "../../../../schemas/library-view/v0.1/examples/local-source.response.json";
import detailVector from "../../../../schemas/bdl-queries/v0.4/examples/catalog-detail.result.json";
import { strings, currentLocale } from "../../src/renderer/i18n/index.ts";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";
import "../../src/renderer/app-shell.css";
const copy = strings.warehouse;
const library = structuredClone(original) as any;
const cloud = library.items[0]; cloud.sourceCategory = null;
cloud.product.variantName = "Synthetic purchased variant";
const local = library.items[1]; delete local.sourceMatch; local.sourceCategory = null;
local.entry.displayName = "Synthetic Coat_v1.2.zip";
const ambiguous = structuredClone(local); ambiguous.entry.warehouseItemId = "wh-ambiguous";
ambiguous.entry.folderName = "wh-ambiguous"; ambiguous.entry.displayName = "Synthetic Ambiguous";
ambiguous.entry.artifacts[0].artifactSha256 = `sha256:${"b".repeat(64)}`;
const missing = structuredClone(cloud); missing.product.productId = "booth:91"; missing.product.title = "Synthetic missing files";
Object.assign(missing.storage, { state: "missing", storedCopies: 1, missingCopies: 1 });
missing.copyIds = ["cpy-missing"];
const downloaded = structuredClone(cloud); downloaded.product.productId = "booth:92";
downloaded.product.title = "Synthetic downloaded product with a longer title";
Object.assign(downloaded.storage, { state: "present", storedCopies: 1, presentCopies: 1 });
downloaded.copyIds = ["cpy-downloaded"]; downloaded.product.importedArtifacts = 1;
library.items.push(ambiguous, missing, downloaded); library.total = 5;
const records = new Map([local, ambiguous].map(row => [row.entry.warehouseItemId, { schemaVersion: "0.1", entryId: row.entry.warehouseItemId, revision: 0, displayName: row.entry.displayName, productId: null, thumbnailRef: null }]));
const calls: any[] = []; const opened: string[] = []; const searched: string[] = []; const fetched: string[] = [];
const controls: { batchId: string; paused: boolean }[] = [];
let transfers: { batchId: string; productId: string; paused: boolean }[] = [];
let releaseSearch: (() => void) | undefined;
let holdSearch = false; let libraryReads = 0;
let fastDownload = false; let transferReads = 0;
const publicSummary = (id: string) => ({ ...cloud.product, productId: id, title: "Synthetic Coat", libraryType: null });
const ok = (value: unknown) => ({ ok: true, value });
const invoke = async (request: any) => {
  calls.push(structuredClone(request));
  if (request.method === "library.list") { libraryReads++; return ok({ ...structuredClone(library), sourceCategories: cloud.sourceCategory ? [cloud.sourceCategory] : [] }); }
  if (request.method === "library.entryMetadata") return ok(structuredClone(records.get(request.params.entryId)));
  if (request.method === "library.productFiles") return ok({ schemaVersion: "0.1", productId: request.params.productId, items: [{ downloadableId: 901, fileName: "synthetic.zip", managedCopyId: null, copies: [] }] });
  if (request.method === "library.downloadStatus") {
    // The fast regression keeps status unreadable to exercise queue reconciliation
    // on its own: both native polls are empty, with no remount or terminal shortcut.
    if (fastDownload) return { ok: false, error: { kind: "unavailable" } };
    const stored = cloud.storage.state === "present";
    return ok({ schemaVersion: "0.1", batchId: request.params.batchId, taskId: request.params.batchId, productId: "booth:90", revision: stored ? 2 : 1,
      state: stored ? "succeeded" : "running", cancellationRequested: false, recoveryDisposition: "none", files: [{ downloadableId: 901, fileName: "synthetic.zip", phase: stored ? "stored" : "downloading", downloadId: "dl-synthetic", errorCode: null, entryId: stored ? "wh-completed" : null, copyId: stored ? "cpy-completed" : null, artifactSha256: stored ? `sha256:${"a".repeat(64)}` : null, replaced: false }] });
  }
  if (request.method === "library.updateEntryMetadata") {
    const p = request.params; const record = records.get(p.entryId)!;
    if (p.expectedRevision !== record.revision) throw Error("unexpected stale source write");
    const next = { ...record, revision: record.revision + 1, productId: p.productId };
    records.set(p.entryId, next);
    const row = library.items.find((row: any) => row.kind === "local" && row.entry.warehouseItemId === p.entryId);
    row.metadata = next; row.sourceMatch = { product: publicSummary(p.productId), sources: [], basis: "mapping", content: "unverified" };
    return ok(next);
  }
  if (request.method === "catalog.detail") {
    const detail = structuredClone(detailVector); detail.schemaVersion = "0.6";
    Object.assign(detail.result.product, { productId: request.params.productId, title: publicSummary(request.params.productId).title, sourceCategory: request.params.productId === "booth:90" ? cloud.sourceCategory : request.params.productId === "booth:92" ? null : "3D衣装", imageUrl: null, imageUrls: [], shopUrl: null });
    return ok(detail);
  }
  if (request.method === "catalog.productDownloadables") return ok({ schemaVersion: "0.7", operation: request.method, result: { productId: request.params.productId, downloads: [{ downloadableId: 901, productId: request.params.productId, fileName: "synthetic.zip", sizeBytes: 1000, sourceUrl: "https://booth.pm/downloadables/901", available: true }] } });
  throw Error(`Unexpected corner request ${request.method}`);
};
Object.defineProperty(window, "vua", { configurable: true, value: {
  gateway: { invoke }, events: { subscribe: () => () => {} },
  shell: { openExternal: async (url: string) => { opened.push(url); return true; } },
  window: { openBrowseWindow: async (url: string) => { opened.push(url); return { opened: true }; } },
  library: { openBoothProduct: async (productId: string) => { opened.push(`https://booth.pm/items/${productId.slice(6)}`); return { opened: true }; } },
  catalogSync: {
    fetchProduct: async (id: string) => { fetched.push(id); if (id === "booth:90") cloud.sourceCategory = "3D衣装"; return { ok: true }; },
    searchProducts: async (query: string) => {
      searched.push(query); if (holdSearch) await new Promise<void>(resolve => { releaseSearch = resolve; });
      return { schemaVersion: "0.1", query, truncated: query.includes("Ambiguous"), items: [
        { productId: "booth:99", title: query, shopName: "Synthetic public shop", sourceCategory: "3D衣装" },
        ...(query.includes("Ambiguous") ? [{ productId: "booth:98", title: query, shopName: "Another public shop", sourceCategory: null }] : []),
      ] };
    },
  },
  silentDownload: {
    start: async (productId: string) => {
      if (fastDownload) { Object.assign(cloud.storage, { state: "present", presentCopies: 1, storedCopies: 1 }); cloud.copyIds = ["cpy-completed"]; cloud.product.importedArtifacts = 1; }
      else transfers.push({ productId, batchId: "library-download-corner", paused: false });
      return { batchId: "library-download-corner", taskId: "task-corner", accepted: 1 };
    },
    transfers: async () => { transferReads++; return structuredClone(transfers); },
    control: async (batchId: string, paused: boolean) => { controls.push({ batchId, paused }); transfers = transfers.map(item => item.batchId === batchId ? { ...item, paused } : item); return { ok: true }; },
  },
} });
const { GatewayProvider, emptyGateway } = await import("../../src/renderer/gateway/index.ts");
const { WarehousePage } = await import("../../src/renderer/features/warehouse/WarehousePage.tsx");
const gateway = { ...emptyGateway(), dataSource: () => "live" as const };
document.documentElement.lang = currentLocale; document.body.dataset.module = "production";
document.documentElement.dataset.effects = "off";
const root = createRoot(document.getElementById("root")!); const checks: string[] = [];
const wait = () => new Promise<void>(resolve => setTimeout(resolve, 40));
const text = () => document.body.textContent ?? "";
const check = (value: unknown, label: string) => { if (!value) throw Error(label); checks.push(label); };
const until = async (test: () => boolean, label: string) => { const end = performance.now() + 7000; while (!test()) { if (performance.now() > end) throw Error(`${label}: ${text()}`); await wait(); } };
const card = (id: string) => [...document.querySelectorAll<HTMLElement>("[data-selection-id]")].find(node => node.dataset.selectionId!.includes(id))!;
const corner = (id: string) => card(id)?.querySelector<HTMLButtonElement>(".vua-library-corner")!;
const button = (label: string, within: ParentNode = document) => { const found = [...within.querySelectorAll<HTMLButtonElement>("button")].find(node => node.textContent?.trim() === label); if (!found) throw Error(`Missing ${label}`); return found; };
let generation = 0;
async function showLibrary() { root.render(<StrictMode><GatewayProvider gateway={gateway}><WarehousePage key={++generation} /></GatewayProvider></StrictMode>); await until(() => document.querySelectorAll("[data-selection-id]").length === 5, "five library cards"); await wait(); }
async function closeDialog() { document.querySelector<HTMLButtonElement>('[role="dialog"] button[aria-label]')!.click(); await until(() => !document.querySelector('[role="dialog"]'), "match dialog closed"); }
async function run() {
  await showLibrary();
  const bodyRows = card("booth:90").querySelector(".vua-warehouse-card__body")!;
  check([...bodyRows.children].slice(0, 3).map(node => node.className).join("|").includes("__title|vua-caption vua-text-secondary vua-warehouse-card__shop|vua-warehouse-card__variant"), "card text order is title, shop, then variant");
  check(corner("booth:90").dataset.cornerState === "cloud" && corner("wh-imported").dataset.cornerState === "local", "cloud/local facts use green cloud and purple computer corners");
  check(corner("booth:91").dataset.cornerState === "error" && corner("booth:92").dataset.cornerState === "linked", "missing files use red cross; stored linked files use orange computer");
  for (const node of document.querySelectorAll<HTMLElement>("[data-selection-id]")) {
    const menu = node.querySelector<HTMLElement>(".vua-warehouse-card__menu")!;
    check(Math.abs(node.getBoundingClientRect().bottom - menu.getBoundingClientRect().bottom - 13) < 6, "card menu stays at the card bottom regardless of title/badges");
  }
  corner("booth:90").dispatchEvent(new MouseEvent("mouseover", { bubbles: true })); await until(() => !!document.querySelector('[role="tooltip"]'), "corner explanation");
  check(document.querySelector('[role="tooltip"]')!.textContent === copy.corner.cloud, "corner has localized tooltip");
  corner("booth:90").click(); await until(() => corner("booth:90")?.dataset.cornerState === "downloading", "download begins");
  check(!card("booth:90").getAttribute("aria-selected") || card("booth:90").getAttribute("aria-selected") === "false", "corner action does not select the card or open details");
  corner("booth:90").click(); await until(() => corner("booth:90")?.dataset.cornerState === "paused", "native pause receipt");
  corner("booth:90").click(); await until(() => corner("booth:90")?.dataset.cornerState === "downloading", "native continuation");
  check(controls.map(c => c.paused).join() === "true,false" && new Set(controls.map(c => c.batchId)).size === 1, "pause/resume controls the same batch rather than cancellation or a new start");
  transfers = []; Object.assign(cloud.storage, { state: "present", presentCopies: 1, storedCopies: 1 }); cloud.copyIds = ["cpy-completed"]; cloud.product.importedArtifacts = 1;
  await until(() => corner("booth:90")?.dataset.cornerState === "linked", "terminal status clears the in-page optimistic transfer without remounting");
  check(corner("booth:90").dataset.cornerState === "linked", "actual completion is shown as orange stored files");
  Object.assign(cloud.storage, { state: "cloud_only", presentCopies: 0, storedCopies: 0 }); cloud.copyIds = []; cloud.product.importedArtifacts = 0;
  fastDownload = true; await showLibrary(); const readsBeforeFast = transferReads;
  corner("booth:90").click(); await until(() => corner("booth:90")?.dataset.cornerState === "downloading", "optimistic fast start");
  await until(() => corner("booth:90")?.dataset.cornerState === "linked", "unchanged empty native queue reconciles a fast completion without remounting");
  check(transferReads > readsBeforeFast && transfers.length === 0, "empty-before/empty-after queue never leaves a completed card offering pause");
  fastDownload = false;
  card("booth:90").dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 200, clientY: 200 })); await wait();
  button(copy.sourceSearch.openBooth).click(); await until(() => opened.length > 0, "menu opens source");
  check(opened.at(-1) === "https://booth.pm/items/90", "context menu opens a canonical BOOTH page");
  const reads = libraryReads;
  card("booth:90").dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  await until(() => fetched.includes("booth:90") && libraryReads > reads, "missing category enriches and refreshes joined list despite existing description");
  const drawer = document.querySelector<HTMLElement>(".vua-warehouse__drawer")!;
  button(copy.sourceSearch.openBooth, drawer).click(); await until(() => opened.length === 2, "detail opens source");
  check(!drawer.textContent!.includes(copy.libraryState.productionUnchecked), "detail drops the retired qualification sentence");
  check([...document.querySelectorAll("select option")].some(node => node.textContent === "3D衣装"), "new category reaches filter options immediately");
  button(copy.detail.close, drawer).click(); await wait();
  card("booth:92").dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  await until(() => fetched.includes("booth:92"), "uncategorized official page observed");
  await new Promise(resolve => setTimeout(resolve, 400));
  check(fetched.filter(id => id === "booth:92").length === 1, "genuinely uncategorized product is fetched once per open instead of a reload loop");
  button(copy.detail.close, document.querySelector(".vua-warehouse__drawer")!).click(); await wait();
  holdSearch = true; corner("wh-imported").click(); await until(() => releaseSearch !== undefined, "public search started");
  check(corner("wh-imported").dataset.cornerState === "searching", "local lookup shows a spinning search state");
  holdSearch = false; releaseSearch!(); await until(() => corner("wh-imported").dataset.cornerState === "linked", "exact public match saved");
  check(searched.at(-1) === "Synthetic Coat", "only leaf keywords without version, path or file bytes are searched");
  check(records.get("wh-imported")!.productId === "booth:99" && !library.items.some((row: any) => row.kind === "product" && row.product.productId === "booth:99"), "public source not owned by this account is associated without adding an account-library row");
  await showLibrary(); check(corner("wh-imported").dataset.cornerState === "linked", "confirmed source association survives library reentry");
  corner("wh-ambiguous").click(); await until(() => !!document.querySelector('[role="dialog"]'), "ambiguous search needs confirmation");
  check(corner("wh-ambiguous").dataset.cornerState === "uncertain" && records.get("wh-ambiguous")!.productId === null, "ambiguous or paginated hits never auto-select a first match");
  await closeDialog(); await showLibrary();
  check(corner("wh-ambiguous").dataset.cornerState === "local", "uncertainty clears back to purple on next library entry");
  corner("wh-ambiguous").click(); await until(() => !!document.querySelector('[role="dialog"]'), "candidate review reopens");
  button(copy.sourceSearch.confirm).click(); await until(() => !document.querySelector('[role="dialog"]') && corner("wh-ambiguous").dataset.cornerState === "linked", "explicit candidate saved");
  check(records.get("wh-ambiguous")!.productId === "booth:99", "explicit public candidate confirmation persists through metadata face");
  // Reset synthetic records solely for a resting screenshot containing all corner colors.
  Object.assign(cloud.storage, { state: "cloud_only", storedCopies: 0, presentCopies: 0 }); cloud.copyIds = []; cloud.product.importedArtifacts = 0;
  for (const row of [local, ambiguous]) { delete row.metadata; delete row.sourceMatch; records.get(row.entry.warehouseItemId)!.productId = null; }
  await showLibrary(); corner("wh-ambiguous").click(); await until(() => !!document.querySelector('[role="dialog"]'), "resting color preview"); await closeDialog();
  check(["cloud", "local", "linked", "uncertain", "error"].every(state => document.querySelector(`[data-corner-state="${state}"]`)), "resting theme preview retains distinct cloud/local/linked/uncertain/error colors");
  return [...checks];
}
(window as any).libraryBulkReview = { run, showLibrary };
