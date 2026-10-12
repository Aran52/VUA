// Isolated real Chromium UI; source/account/filesystem services are synthetic.
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import original from "../../../../schemas/library-view/v0.1/examples/local-source.response.json";
import { strings, format } from "../../src/renderer/i18n/index.ts";
import { storageKeys } from "../../src/renderer/app/storage-keys.ts";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";
import "../../src/renderer/app-shell.css";
const copy = strings.warehouse;
localStorage.removeItem(storageKeys.importLastFolder);
const library = structuredClone(original) as any;
const account = library.items[0]; account.product.imageUrl = null; account.product.imageUrls = []; account.operation = null;
const local = library.items[1]; local.entry.displayName = "Synthetic mixed garment"; delete local.sourceMatch;
local.entry.artifacts.push({ ...local.entry.artifacts[0], artifactSha256: `sha256:${"b".repeat(64)}`, relativePath: "Textures/companion.psd" });
local.copyIds = ["local-a", "local-b"]; Object.assign(local.storage, { state: "missing", storedCopies: 2, presentCopies: 0, missingCopies: 2, changedCopies: 0, unreadableCopies: 0, supersededGeneratedCopies: 0, currentGeneratedCopies: 0 });
const many = Array.from({ length: 36 }, (_, index) => {
  const row = structuredClone(account); row.product.productId = `booth:${1000 + index}`; row.product.title = `Synthetic product ${index}`;
  row.product.importedArtifacts = 1; row.copyIds = [`account-${index}`]; delete row.localEntries;
  Object.assign(row.storage, { state: "present", storedCopies: 1, presentCopies: 1, missingCopies: 0, changedCopies: 0, unreadableCopies: 0, supersededGeneratedCopies: 0, currentGeneratedCopies: 0 }); return row;
});
delete account.localEntries;
let rows = [account, ...many, local];
const hidden = new Set<string>(); const calls: any[] = [];
const known = { desktop: "D:\\Desktop", documents: "E:\\Documents", downloads: "F:\\Downloads", drives: ["C:\\", "D:\\", "E:\\", "F:\\"].map(path => ({ name: path, path, hidden: false })) };
const directories: string[] = []; const scans: string[] = [];
const invoke = async (request: any) => {
  calls.push(structuredClone(request));
  if (request.method === "library.list") {
    const items = rows.filter(row => !hidden.has(row.kind === "local" ? `entry:${row.entry.warehouseItemId}` : `product:${row.product.productId}`))
      .filter(row => request.params.source !== "imported" || row.kind === "local" && row.entry.kind === "imported_material");
    return { ok: true, value: { ...library, items, total: items.length, offset: 0, limit: 50 } };
  }
  if (request.method === "library.removeCards") {
    request.params.targets.forEach((target: any) => hidden.add(`${target.kind}:${target.id}`));
    return { ok: true, value: { schemaVersion: "0.2", targets: request.params.targets } };
  }
  if (request.method === "catalog.detail") return { ok: false, error: { code: "application", application: { contractVersion: "0.1", code: "vua.catalog.product_not_found", category: "validation", messageKey: "errors.catalog.notFound", recoverable: false, retryable: false, correlationId: "synthetic" } } };
  throw Error(`Unexpected method ${request.method}`);
};
Object.defineProperty(window, "vua", { configurable: true, value: { gateway: { invoke }, events: { subscribe: () => () => {} },
  fs: {
    navigation: async () => ({ ok: true, value: known }),
    listDirectory: async (target: string | null) => { const path = target ?? "C:\\Users\\Synthetic"; directories.push(path); return { ok: true, value: { path, parent: /^[A-Z]:\\$/.test(path) ? null : path.slice(0, path.lastIndexOf("\\")) || "C:\\", entries: [{ name: "Collection", path: `${path.endsWith("\\") ? path : `${path}\\`}Collection`, hidden: false }] } }; },
    createDirectory: async () => ({ ok: false, error: "failed" }),
  },
  library: { previewDirectory: async (path: string) => { scans.push(path); return { path, state: "ready", truncated: false, items: [
    { name: "Garment with versions", path: `${path}\\Garment`, kind: "directory", sizeBytes: null },
    { name: "shared.README", path: `${path}\\shared.README`, kind: "file", sizeBytes: 32 },
    { name: "bundle.zip", path: `${path}\\bundle.zip`, kind: "file", sizeBytes: 64 },
    { name: "ancestor link", path: `${path}\\link`, kind: "link", sizeBytes: null },
  ] }; } },
  dialog: { pickWarehouseFolders: async () => null },
} });
const { GatewayProvider, emptyGateway } = await import("../../src/renderer/gateway/index.ts");
const { WarehousePage } = await import("../../src/renderer/features/warehouse/WarehousePage.tsx");
const root = createRoot(document.getElementById("root")!);
const gateway = { ...emptyGateway(), dataSource: () => "live" as const };
const checks: string[] = [];
const pause = () => new Promise<void>(resolve => setTimeout(resolve, 45));
const check = (value: unknown, label: string) => { if (!value) throw Error(label); checks.push(label); };
const until = async (predicate: () => boolean, label: string) => { const deadline = performance.now() + 7000; while (!predicate()) { if (performance.now() > deadline) throw Error(`${label}: ${document.body.textContent}`); await pause(); } };
const cards = () => [...document.querySelectorAll<HTMLElement>("[data-selection-id]")];
const button = (label: string) => { const result = [...document.querySelectorAll<HTMLButtonElement>("button")].find(node => node.textContent?.trim() === label); if (!result) throw Error(`Missing button ${label}`); return result; };
const click = async (label: string) => { const node = button(label); if (node.disabled) throw Error(`Disabled ${label}; ${JSON.stringify({ checks, directories: directories.slice(-4), pickers: document.querySelectorAll('.vua-folder-picker').length, inputs: [...document.querySelectorAll<HTMLInputElement>('.vua-folder-picker input')].map(node => node.value), text: document.querySelector('.vua-folder-picker')?.textContent })}`); node.click(); await pause(); };
const inputValue = (input: HTMLInputElement, value: string) => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value); input.dispatchEvent(new Event("input", { bubbles: true })); };
async function pathTo(value: string) {
  const input = document.querySelector<HTMLInputElement>(`input[aria-label="${strings.folderPicker.pathInputAria}"]`)!;
  inputValue(input, value); await pause(); input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
  await pause();
  await until(() => directories.at(-1) === value && !!document.querySelector(".vua-folder-picker__row") && !button(strings.folderPicker.up).disabled && input.value === value, "path navigation");
}
async function showLibrary() {
  root.render(<StrictMode><GatewayProvider gateway={gateway}><WarehousePage /></GatewayProvider></StrictMode>);
  await until(() => cards().length === rows.length, "library loaded");
}
async function run() {
  await showLibrary();
  check(cards().filter(card => card.dataset.selectionId === local.entry.warehouseItemId).length === 1 && cards().length === 38, "a multi-file missing local material is exactly one removable card");
  const source = document.querySelector<HTMLSelectElement>(`select[aria-label="${copy.filters.source}"]`)!;
  source.value = "local"; source.dispatchEvent(new Event("change", { bubbles: true })); await until(() => cards().length === 1, "imported origin filter");
  check(source.selectedOptions[0]!.textContent === copy.filters.sourceLocal && calls.some(call => call.method === "library.list" && call.params.source === "imported"), "Local imports sends origin filtering and excludes downloaded account products");
  const missing = cards()[0]!; missing.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 180, clientY: 220 })); await pause();
  await click(copy.removeCards.title); await until(() => !!document.querySelector('[role="dialog"]'), "card confirmation");
  check(calls.every(call => call.method !== "library.removeCards") && !button(format(copy.removeCards.confirm, { count: 1 })).disabled, "missing-file card still requires count-confirmed removal");
  document.querySelector<HTMLButtonElement>('[role="dialog"] button[aria-label]')!.click(); await pause();
  check(calls.every(call => call.method !== "library.removeCards"), "canceling card removal changes no records");
  missing.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 180, clientY: 220 })); await pause(); await click(copy.removeCards.title); await click(format(copy.removeCards.confirm, { count: 1 }));
  await until(() => cards().length === 0 && !document.querySelector('[role="dialog"]'), "card removed");
  check(calls.some(call => call.method === "library.removeCards" && call.params.targets[0].id === local.entry.warehouseItemId) && calls.every(call => call.method !== "library.removeFiles"), "card removal preserves physical-file actions and exact entry identity");
  source.value = "all"; source.dispatchEvent(new Event("change", { bubbles: true })); await until(() => cards().length === 37, "account cards restored in all-source view");
  const wall = document.querySelector<HTMLDivElement>(".vua-warehouse__wall-scroll")!; wall.style.maxHeight = "230px"; wall.style.height = "230px"; await pause();
  const bounds = wall.getBoundingClientRect(); const pointer = { x: bounds.right - 12, y: bounds.bottom - 12 };
  wall.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, clientX: bounds.left + 3, clientY: bounds.top + 3 }));
  window.dispatchEvent(new PointerEvent("pointermove", { clientX: pointer.x, clientY: pointer.y })); await pause();
  const selectedBefore = document.querySelectorAll('[data-selection-id][data-selected="true"]').length;
  wall.scrollTop = 500; wall.dispatchEvent(new Event("scroll")); await pause();
  const selectedAfter = document.querySelectorAll('[data-selection-id][data-selected="true"]').length;
  check(selectedAfter > selectedBefore && !!document.querySelector(".vua-warehouse__marquee"), "scrolling without pointer movement extends the held marquee");
  wall.scrollTop = 0; wall.dispatchEvent(new Event("scroll")); await pause();
  check(document.querySelectorAll('[data-selection-id][data-selected="true"]').length === selectedBefore, "reversing scroll restores the original marquee extent");
  window.dispatchEvent(new PointerEvent("pointerup", { clientX: pointer.x, clientY: pointer.y })); await pause();
  check(!document.querySelector(".vua-warehouse__marquee"), "release removes the marquee and its scroll lifecycle");
  wall.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 2, clientX: bounds.left + 3, clientY: bounds.top + 3 }));
  window.dispatchEvent(new PointerEvent("pointermove", { clientX: pointer.x, clientY: pointer.y }));
  wall.scrollTop = 500; wall.dispatchEvent(new Event("scroll"));
  await pause();
  window.dispatchEvent(new PointerEvent("pointerup", { button: 2, clientX: pointer.x, clientY: pointer.y }));
  await pause(); await click(copy.removeCards.title);
  const currentCount = document.querySelectorAll('[data-selection-id][data-selected="true"]').length;
  check(currentCount > selectedBefore && !button(format(copy.removeCards.confirm, { count: currentCount })).disabled, "right-button release uses the current scrolled selection for count-confirmed card removal");
  document.querySelector<HTMLButtonElement>('[role="dialog"] button[aria-label]')!.click(); await pause();
  await click(strings.importPage.localTitle); await until(() => !!document.querySelector(".vua-folder-picker"), "single item picker");
  const pathField = document.querySelector<HTMLInputElement>(`input[aria-label="${strings.folderPicker.pathInputAria}"]`)!;
  check(getComputedStyle(pathField).borderTopStyle !== "none" && !pathField.hidden, "folder path is a persistent visible text field");
  for (const key of ["desktop", "documents", "downloads"] as const) { await until(() => !button(strings.folderPicker[key]).disabled, "known locations loaded"); await click(strings.folderPicker[key]); check(directories.at(-1) === known[key], `${key} uses the resolved known-folder path`); }
  await pathTo("C:\\"); await click(strings.folderPicker.up);
  await until(() => document.querySelectorAll(".vua-folder-picker__row").length === 4, "drive list");
  check(button(strings.folderPicker.openCta).disabled && button(`+ ${strings.folderPicker.newFolder}`).disabled, "This PC is a virtual non-importable non-creatable location");
  const disk = [...document.querySelectorAll<HTMLElement>(".vua-folder-picker__row")].find(node => node.textContent?.includes("E:\\"))!;
  disk.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })); await until(() => directories.at(-1) === "E:\\", "drive entered");
  check(directories.at(-1) === "E:\\", "drive roots are navigable without typing a letter");
  await click(strings.folderPicker.cancel); await pause();
  // ImportPage retains its parent content dialog when picker cancellation occurs.
  if (document.querySelector('[role="dialog"]')) { document.querySelector<HTMLButtonElement>('[role="dialog"] button[aria-label]')!.click(); await pause(); }
  const arrow = document.querySelector<HTMLButtonElement>(`button[aria-label="${strings.importPage.importModes}"]`)!;
  arrow.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })); await pause();
  check(!!document.querySelector('[role="menuitem"]'), "hovering the adjacent arrow reveals the migration route");
  arrow.click(); await pause(); await click(strings.importPage.migrationTitle); await until(() => !!document.querySelector(".vua-folder-picker"), "migration source picker");
  await click(strings.folderPicker.desktop); await click(strings.folderPicker.openCta);
  await until(() => !!document.querySelector(".vua-migration-preview__tree"), "migration structure preview");
  check(document.body.textContent?.includes(strings.importPage.migrationPreview.planning) && document.body.textContent?.includes("shared.README") && document.body.textContent?.includes("bundle.zip"), "migration preview retains loose companion files and archives without assigning package boundaries");
  check(scans.length > 0 && calls.every(call => call.method !== "library.importPaths" && call.method !== "library.importFolders"), "migration structural preview performs no import");
  return checks;
}
async function showPicker() {
  if (document.querySelector('[role="dialog"]')) { document.querySelector<HTMLButtonElement>('[role="dialog"] button[aria-label]')!.click(); await pause(); }
  await click(strings.importPage.localTitle); await until(() => !!document.querySelector(".vua-folder-picker__row"), "picker visual preview");
}
(window as any).libraryNavigationReview = { run, showPicker };
