// Isolated React controls with synthetic ports. No local connection key or real requests.
import { createRoot } from "react-dom/client";
import type { AiActionRequestV01, AiToolsSnapshotV01, DesktopAiToolsApiV01, ExternalToolSnapshot } from "@vua/contracts";
import { GatewayProvider } from "../../src/renderer/gateway/GatewayProvider.tsx";
import { emptyGateway } from "../../src/renderer/gateway/empty-gateway.ts";
import { AiToolsPage } from "../../src/renderer/features/help/AiToolsPage.tsx";
import { SteamToolCard } from "../../src/renderer/features/tools/SteamToolCard.tsx";
import { VrcftCard } from "../../src/renderer/features/tools/VrcftCard.tsx";
import { strings } from "../../src/renderer/i18n/index.ts";
import { LibraryTermsRead } from "../../src/renderer/features/warehouse/LibraryTermsRead.tsx";
import { libraryTermsCopy } from "../../src/renderer/i18n/library-terms-copy.ts";
import type { TaskCenterView } from "../../src/renderer/gateway/index.ts";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";
import "../../src/renderer/features/home/home.css";

const request = (id: string, state: AiActionRequestV01["state"]): AiActionRequestV01 => ({ id, state,
  tool: "vua_request_play", title: id, explanation: "Synthetic request", expectedResult: "No real work",
  createdAt: new Date().toISOString(), details: {} });
let snapshot: AiToolsSnapshotV01 = { schemaVersion: "0.1", enabled: true, endpoint: "http://127.0.0.1:32179/mcp",
  connections: 0, error: null, requests: [request("pending", "pending"), request("accepted", "accepted"), request("failed", "failed"), request("running", "running")] };
const listeners = new Set<(snapshot: AiToolsSnapshotV01) => void>();
let failClear = false;
const cleared: string[][] = [];
const aiTools: DesktopAiToolsApiV01 = {
  async snapshot() { return snapshot; }, async setEnabled() { return snapshot; },
  async prompt() { return "Synthetic instructions; no real connection key"; }, async copyPrompt() {},
  async resolve() { throw Error("Should not approve anything"); },
  subscribe(listener) { listeners.add(listener); return () => { listeners.delete(listener); }; },
  async clear(ids) {
    if (failClear) { failClear = false; throw Error("Synthetic clear failure"); }
    cleared.push([...ids]); snapshot = { ...snapshot, requests: snapshot.requests.filter(item => !ids.includes(item.id)) };
    listeners.forEach(listener => listener(snapshot)); return snapshot;
  },
};
const pendingTools: { resolve(value: ExternalToolSnapshot): void; reject(error: Error): void }[] = [];
const base = emptyGateway(null);
let termsCalls = 0, libraryReloads = 0, failTerms = false;
let tasks: TaskCenterView = { schemaVersion: 1, tasks: [] };
const taskListeners = new Set<(view: TaskCenterView) => void>();
const finishTerms = (status: "completed" | "completedWithWarnings") => {
  tasks = { ...tasks, tasks: tasks.tasks.map(task => ({ ...task, status })) }; taskListeners.forEach(listener => listener(tasks));
};
const gateway = { ...base, aiTools,
  task: { ...base.task, async snapshot() { return tasks; }, subscribe(listener: (view: TaskCenterView) => void) { taskListeners.add(listener); return () => { taskListeners.delete(listener); }; } },
  terms: { async product() { throw Error("not used"); }, async checkUse() { throw Error("not used"); }, async refreshProduct() { throw Error("not used"); },
    async refreshLibrary(title: string) {
      if (failTerms) { failTerms = false; throw Error("Synthetic terms failure"); }
      termsCalls++; tasks = { schemaVersion: 1, tasks: [{ id: `terms-${termsCalls}`, title, status: "running", originPage: "warehouse", cancellable: true }] };
      taskListeners.forEach(listener => listener(tasks)); return { taskId: `terms-${termsCalls}` };
    } }, environment: { ...base.environment, tools: {
  observe: () => new Promise<ExternalToolSnapshot>((resolve, reject) => { pendingTools.push({ resolve, reject }); }),
  async act() { throw Error("Should not act on tools"); },
} } };
localStorage.setItem("vua-vrcft-setup", JSON.stringify({ schemaVersion: 1, device: "webcam", method: "babble", completed: [] }));
createRoot(document.getElementById("root")!).render(<GatewayProvider gateway={gateway}>
  <div className="vua-tools-hub vua-compact-environments"><div className="vua-environment-grid">
    <SteamToolCard toolId="vrcs" title="VRCS" icon="globe" selected={false} onDetails={() => {}} onReveal={() => {}} onPrepareSteam={() => {}} />
    <VrcftCard selected={false} onDetails={() => {}} onReveal={() => {}} onPrepareSteam={() => {}} />
  </div></div>
  <AiToolsPage navigate={() => {}} />
  <LibraryTermsRead onUpdated={() => { libraryReloads++; }} />
</GatewayProvider>);
const wait = () => new Promise(resolve => setTimeout(resolve, 30));
async function until(predicate: () => unknown) { for (let i = 0; i < 160; i++) { if (predicate()) return; await wait(); } throw Error(`UI timeout: ${predicate}`); }
const button = (root: ParentNode, text: string) => [...root.querySelectorAll<HTMLButtonElement>("button")].find(item => item.textContent?.trim() === text);
Object.assign(window, { mcpReview: { async run() {
  const checks: string[] = [];
  const check = (value: unknown, label: string) => { if (!value) throw Error(label); checks.push(label); };
  await until(() => pendingTools.length >= 2 && document.querySelectorAll("[data-ai-request]").length === 4);
  for (const id of ["tool-vrcs", "tool-vrcft"]) {
    const action = document.querySelector<HTMLButtonElement>(`[data-nav-id="${id}-action"]`)!;
    check(action.disabled && action.getAttribute("aria-busy") === "true" && action.querySelector("svg.vua-environment-card__spin") && !action.querySelector('path[d="m16 4 13 23H3Z"]'), `${id}: first read shows a loading circle`);
  }
  pendingTools.splice(0).forEach(pending => pending.reject(Error("Synthetic observation failure")));
  await until(() => [...document.querySelectorAll(".vua-environment-card__action")].every(action => action.getAttribute("aria-busy") === "false"));
  check(document.querySelectorAll('.vua-environment-card__action path[d="m16 4 13 23H3Z"]').length === 2, "Observation failures show warning triangles");
  const section = document.querySelector(".vua-ai-tools__requests")!;
  check([...section.querySelectorAll("[data-ai-request]")].every(card => button(card, strings.aiTools.clear)), "Every request state has a clear button");
  failClear = true;
  button(section.querySelector('[data-ai-request="pending"]')!, strings.aiTools.clear)!.click();
  await until(() => [...document.querySelectorAll('.vua-ai-tools [role="alert"]')].some(alert => alert.textContent === strings.aiTools.error)
    && !button(section.querySelector('[data-ai-request="pending"]')!, strings.aiTools.clear)!.disabled);
  check(!!section.querySelector('[data-ai-request="pending"]'), "A failed clear keeps the request available");
  button(section.querySelector('[data-ai-request="pending"]')!, strings.aiTools.clear)!.click();
  await until(() => !section.querySelector('[data-ai-request="pending"]'));
  check(cleared.length === 1 && cleared[0]!.join() === "pending" && section.querySelectorAll("[data-ai-request]").length === 3, "Individual clear addresses only its request");
  button(section, strings.aiTools.clearAll)!.click();
  await until(() => section.querySelectorAll("[data-ai-request]").length === 0);
  check(cleared.length === 2 && [...cleared[1]!].sort().join() === "accepted,failed,running", "Clear all sends the currently displayed IDs");
  check(button(section, strings.aiTools.clearAll)!.disabled && section.textContent?.includes(strings.aiTools.noRequests), "Empty list disables clear all and explains the empty state");
  const termsSection = document.querySelector(".vua-library-terms-read")!;
  check(termsCalls === 0, "Opening the filter performs no background license downloads");
  button(termsSection, libraryTermsCopy.read)!.click();
  await until(() => termsCalls === 1 && !!button(termsSection, libraryTermsCopy.reading)?.disabled);
  check(true, "An explicit library terms read stays busy while its task runs");
  finishTerms("completedWithWarnings");
  await until(() => libraryReloads === 1 && termsSection.textContent?.includes(libraryTermsCopy.stopped));
  check(true, "Partial reading refreshes the filter and explains unresolved evidence");
  failTerms = true; button(termsSection, libraryTermsCopy.read)!.click();
  await until(() => termsSection.textContent?.includes(libraryTermsCopy.failed) && !button(termsSection, libraryTermsCopy.read)!.disabled);
  check(termsCalls === 1 && libraryReloads === 1, "Failed submission cannot claim a successful license refresh");
  button(termsSection, libraryTermsCopy.read)!.click();
  await until(() => termsCalls === 2 && !!button(termsSection, libraryTermsCopy.reading)?.disabled);
  finishTerms("completed");
  await until(() => libraryReloads === 2 && termsSection.textContent?.includes(libraryTermsCopy.done));
  check(true, "Completed reading refreshes use decisions and allows another explicit read");
  return checks;
} } });
