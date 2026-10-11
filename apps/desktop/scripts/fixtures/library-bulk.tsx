// Isolated synthetic Gateway; this file is never imported by the application.
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import type { DesktopGatewayRequestV1, LibraryRemoveFilesParamsV01 } from "@vua/contracts";
import original from "../../../../schemas/library-view/v0.1/examples/local-source.response.json";
import { strings, format, currentLocale } from "../../src/renderer/i18n/index.ts";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";
import "../../src/renderer/app-shell.css";
import "../../src/renderer/features/task-center/taskbar.css";
import "../../src/renderer/features/task-center/notification-popover.css";

const copy = strings.warehouse;
const bulk = copy.bulkFiles;
const library = structuredClone(original);
const product = library.items[0]! as any;
product.product.imageUrl = null; product.product.imageUrls = []; product.sourceCategory = "3D衣装";
product.copyIds = ["cpy-original"]; product.product.importedArtifacts = 1;
Object.assign(product.storage, { state: "present", storedCopies: 1, presentCopies: 1 });
const local = library.items[1]! as any;
local.sourceCategory = null; local.sourceMatch.basis = "name";
local.sourceMatch.product.imageUrl = null; local.sourceMatch.product.imageUrls = [];
const other = structuredClone(product);
other.product.productId = "booth:91"; other.product.title = "Synthetic second product"; other.product.importedArtifacts = 0;
other.copyIds = []; delete other.localEntries; other.sourceCategory = "3Dモデル";
Object.assign(other.storage, { state: "cloud_only", storedCopies: 0, presentCopies: 0, missingCopies: 0, changedCopies: 0, unreadableCopies: 0, supersededGeneratedCopies: 0, currentGeneratedCopies: 0 });
library.items.push(other); library.total = 3;
const calls: DesktopGatewayRequestV1[] = [];
const downloads: { productId: string; ids: readonly number[]; targets: unknown }[] = [];
const removals: LibraryRemoveFilesParamsV01[] = [];
const accepted = new Map<string, LibraryRemoveFilesParamsV01>();
let deletionMode: "normal" | "lost" | "drift" | "previewFailure" | "interrupted" = "normal";
let lost = false;
const error = (code: string) => ({ ok: false, error: { code: "application", application: { contractVersion: "0.1", code, category: "conflict", messageKey: "errors.library.removalFailed", recoverable: true, retryable: false, correlationId: "synthetic" } } });
const ok = (value: unknown) => ({ ok: true, value });
const sha = `sha256:${"a".repeat(64)}`;
const inventory = (id: string) => ({ schemaVersion: "0.1", productId: id, items: id === "booth:90" ? [
  { downloadableId: 901, fileName: "installed.zip", managedCopyId: "cpy-original", copies: [{ copyId: "cpy-original", entryId: "whi-synthetic", fileName: "installed.zip", artifactSha256: sha, presence: "present" }] },
  { downloadableId: 902, fileName: "missing.pdf", managedCopyId: null, copies: ["legacy-a", "legacy-b"].map(copyId => ({ copyId, entryId: "whi-legacy", fileName: "missing.pdf", artifactSha256: sha, presence: "missing" })) },
] : [{ downloadableId: 911, fileName: "new.zip", managedCopyId: null, copies: [] }] });
const snapshot = (request: LibraryRemoveFilesParamsV01) => ({ schemaVersion: "0.1", removalId: request.removalId, target: request.target, taskId: request.removalId, taskState: "running", state: "running", revision: 3, cancelRequested: false, recoveryDisposition: "none", files: request.copyIds.map(copyId => ({ copyId, entryId: "whi-synthetic", fileName: `${copyId}.zip`, phase: "pending", errorCode: null })) });
const invoke = async (request: DesktopGatewayRequestV1): Promise<unknown> => {
  calls.push(structuredClone(request));
  if (request.method === "library.list") {
    const params = request.params;
    const items = library.items.filter((row: any) => params.sourceCategory === undefined || row.sourceCategory === params.sourceCategory);
    return ok({ ...library, total: items.length, items, sourceCategories: ["3Dモデル", "3D衣装"] });
  }
  if (request.method === "library.productFiles") return ok(inventory(request.params.productId));
  if (request.method === "catalog.detail") return error("vua.catalog.product_not_found");
  if (request.method === "library.pendingRemovals") return ok({ schemaVersion: "0.1", target: request.params.target, items: deletionMode === "interrupted" ? [{ ...snapshot({ schemaVersion: "0.1", removalId: "library-removal-old", target: request.params.target, copyIds: ["cpy-original"], previewHash: sha }), state: "unconfirmed", recoveryDisposition: "inspect_required" }] : [] });
  if (request.method === "library.removalPreview") {
    if (deletionMode === "previewFailure") return error("vua.library.reference_read_failed");
    const ids = request.params.copyIds ?? [];
    return ok({ schemaVersion: "0.1", target: request.params.target, previewHash: sha, referenceCoverage: "drafts_and_recipes", unresolvedRecipeAssets: 0,
      files: ids.map(copyId => ({ copyId, entryId: "whi-synthetic", fileName: `${copyId}.zip`, role: "original", artifactSha256: sha, sizeBytes: 1024, presence: "present", superseded: false })),
      references: [{ kind: "recipe", id: "synthetic-recipe", title: "Synthetic affected recipe", revision: 1, copyIds: ids, missingAfterRemoval: true }],
    });
  }
  if (request.method === "library.removeFiles") {
    const params = request.params; removals.push(structuredClone(params));
    if (deletionMode === "drift") return error("vua.library.preview_changed");
    if (!accepted.has(params.removalId)) accepted.set(params.removalId, structuredClone(params));
    if (deletionMode === "lost" && !lost) { lost = true; throw Error("Synthetic lost receipt"); }
    return ok(snapshot(params));
  }
  if (request.method === "library.removalStatus") { const request = accepted.get((request.params as any).removalId); return request ? ok(snapshot(request)) : error("vua.library.target_not_found"); }
  throw Error(`Unexpected synthetic method ${request.method}`);
};
Object.defineProperty(window, "vua", { configurable: true, value: { gateway: { invoke }, events: { subscribe: () => () => {} },
  silentDownload: { start: async (productId: string, ids: readonly number[], targets: unknown) => { downloads.push({ productId, ids: [...ids], targets: structuredClone(targets) }); return { batchId: `library-download-${downloads.length}`, taskId: `task-download-${downloads.length}`, accepted: ids.length }; } },
} });
const { GatewayProvider, emptyGateway } = await import("../../src/renderer/gateway/index.ts");
const { WarehousePage } = await import("../../src/renderer/features/warehouse/WarehousePage.tsx");
const { NotificationList } = await import("../../src/renderer/features/task-center/NotificationList.tsx");
const gateway = { ...emptyGateway(), dataSource: () => "live" as const };
document.documentElement.lang = currentLocale; document.body.dataset.module = "production";
const root = createRoot(document.getElementById("root")!);
const checks: string[] = [];
const pause = () => new Promise<void>(resolve => setTimeout(resolve, 35));
const text = () => document.body.textContent ?? "";
const check = (value: unknown, label: string) => { if (!value) throw Error(label); checks.push(label); };
const until = async (test: () => boolean, label: string) => { const deadline = performance.now() + 6000; while (!test()) { if (performance.now() > deadline) throw Error(`${label}: ${text()}`); await pause(); } };
const button = (label: string) => { const found = [...document.querySelectorAll<HTMLButtonElement>("button")].find(node => node.textContent?.trim() === label); if (!found) throw Error(`Missing ${label}`); return found; };
const click = async (label: string) => { const node = button(label); if (node.disabled) throw Error(`Disabled ${label}`); node.click(); await pause(); };
const cards = () => [...document.querySelectorAll<HTMLElement>("[data-selection-id]")];
async function menu() {
  const card = cards()[0]!; card.focus(); card.dispatchEvent(new KeyboardEvent("keydown", { key: "a", ctrlKey: true, bubbles: true, cancelable: true })); await pause();
  card.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 220, clientY: 210 })); await pause();
}
async function closeDialog() { document.querySelector<HTMLButtonElement>('[role="dialog"] button[aria-label]')!.click(); await until(() => !document.querySelector('[role="dialog"]'), "dialog closes"); }
let generation = 0;
async function showLibrary() {
  root.render(<StrictMode><GatewayProvider gateway={gateway}><WarehousePage key={++generation} /></GatewayProvider></StrictMode>);
  await until(() => cards().length === 3, "library loaded"); await pause();
}
async function run() {
  checks.length = 0; await showLibrary();
  const category = document.querySelector<HTMLSelectElement>(`select[aria-label="${copy.filters.category}"]`)!;
  category.value = "category:3D衣装"; category.dispatchEvent(new Event("change", { bubbles: true }));
  await until(() => cards().length === 1, "native category applies");
  check(calls.some(call => call.method === "library.list" && call.params.sourceCategory === "3D衣装"), "category is filtered by the provider before pagination");
  category.value = "unknown"; category.dispatchEvent(new Event("change", { bubbles: true })); await until(() => cards().length === 1 && cards()[0]!.dataset.selectionId !== "booth:90", "unknown category keeps the independent local card");
  check(calls.some(call => call.method === "library.list" && call.params.sourceCategory === null), "unknown native category is explicit rather than inferred from a title");
  category.value = "all"; category.dispatchEvent(new Event("change", { bubbles: true })); await until(() => cards().length === 3, "all categories restored");
  await menu();
  check([copy.cardMenu.download, copy.cardMenu.reDownload, copy.cardMenu.deleteLocal].every(label => !button(label).disabled), "multi-selection exposes download, re-download and delete");
  await click(copy.cardMenu.download);
  await until(() => text().includes("missing.pdf"), "bulk inventory loaded");
  check(text().includes(format(bulk.skipLocal, { count: 1 })) && !text().includes("installed.zip"), "download excludes present files and explains local items without confirmed source");
  const confirmDownload = format(bulk.confirmDownload, { count: 2 });
  check(button(confirmDownload).disabled && downloads.length === 0, "ambiguous replacement cannot start until a target is explicitly chosen");
  const replacement = document.querySelector<HTMLSelectElement>('[role="dialog"] select')!;
  replacement.value = "legacy-b"; replacement.dispatchEvent(new Event("change", { bubbles: true })); await until(() => !button(confirmDownload).disabled, "replacement chosen");
  await click(confirmDownload); await until(() => downloads.length === 2, "both products queued");
  check(JSON.stringify(downloads.map(download => download.ids)) === "[[902],[911]]", "download requests only selected missing captured files");
  check(JSON.stringify(downloads[0]!.targets) === '[{"downloadableId":902,"copyId":"legacy-b"}]', "replacement identity reaches the native queue without being guessed");
  await closeDialog(); await menu(); await click(copy.cardMenu.reDownload); await until(() => text().includes("installed.zip"), "redownload inventory loaded");
  check(document.querySelectorAll('[role="dialog"] input[type="checkbox"]:checked').length === 3, "re-download explicitly includes existing local files"); await closeDialog();

  await menu(); await click(copy.cardMenu.deleteLocal); await until(() => text().includes("Synthetic affected recipe"), "removal preview loaded");
  const confirmRemove = format(bulk.confirmRemove, { count: 2 });
  await until(() => !button(confirmRemove).disabled, "removal confirmation ready");
  check(removals.length === 0 && text().includes(format(bulk.titleRemove, { count: 2 })), "no deletion occurs before the count-bearing confirmation");
  await closeDialog(); check(removals.length === 0, "closing a preview never deletes files");
  deletionMode = "lost"; await menu(); await click(copy.cardMenu.deleteLocal); await until(() => !button(confirmRemove).disabled, "new preview ready");
  await click(confirmRemove); await until(() => text().includes(bulk.unknown), "lost deletion receipt remains unknown");
  check(accepted.size === 2 && removals.length === 2, "a batch records each scoped product intent independently");
  await click(bulk.retryUnknown); await until(() => removals.length === 3 && !text().includes(bulk.unknown), "same-request recovery");
  check(JSON.stringify(removals[0]) === JSON.stringify(removals[2]) && accepted.size === 2, "receipt retry retains the exact removal ID, hash and copies");
  check(removals[0]!.copyIds.every(id => product.copyIds.includes(id)) && removals[1]!.copyIds.every(id => local.copyIds.includes(id)), "bulk deletion retains both visible scopes without expanding to unseen files");
  await closeDialog();
  for (const mode of ["previewFailure", "interrupted", "drift"] as const) {
    deletionMode = mode; await menu(); await click(copy.cardMenu.deleteLocal);
    if (mode === "drift") { await until(() => !button(confirmRemove).disabled, "drift preview ready"); await click(confirmRemove); await until(() => text().includes(bulk.previewChanged), "changed preview refused"); check(text().includes(bulk.previewChanged), "changed files require a fresh user-reviewed preview"); }
    else { await until(() => text().includes(mode === "previewFailure" ? bulk.loadFailed : bulk.pending), "unsafe deletion disabled"); check(button(confirmRemove).disabled, mode === "previewFailure" ? "failed reference reads prevent batch deletion" : "interrupted removal is inspected before another deletion"); }
    await closeDialog();
  }
  root.render(<div className="vua-notify-panel" style={{ position: "relative", top: 0, right: 0 }}><NotificationList notifications={[
    { id: "transfer-known", title: "Download · synthetic-material-with-a-long-filename.zip", status: "running", originPage: "warehouse", cancellable: true, download: { receivedBytes: 5 * 1024 ** 2, expectedBytes: 20 * 1024 ** 2, bytesPerSecond: 2 * 1024 ** 2, sampledAt: Date.now() } },
    { id: "transfer-unknown", title: "Download · unknown-size.zip", status: "running", originPage: "warehouse", cancellable: true, download: { receivedBytes: 1024, expectedBytes: null, bytesPerSecond: null, sampledAt: Date.now() } },
  ]} rejectedId={null} showCompleted={false} onShowCompletedChange={() => {}} onCancel={() => {}} onRetry={() => {}} dismissible={() => undefined} onBackToOrigin={() => {}} emptyText={null} replay={null} /></div>);
  await pause();
  check(document.querySelectorAll("progress").length === 2 && document.querySelectorAll("progress:indeterminate").length === 1, "known size renders measured progress; unknown size remains indeterminate");
  check(text().includes("MiB/s") && text().includes(strings.taskCenter.speedUnavailable), "notification rows show transfer speed and an honest measuring state");
  return [...checks];
}
(window as any).libraryBulkReview = { run, showLibrary };
