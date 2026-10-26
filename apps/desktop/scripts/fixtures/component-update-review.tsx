// Controlled component panels; synthetic ports, no installation or network actions.
import { createRoot } from "react-dom/client";
import type { ComponentUpdateSnapshotV02, DesktopComponentUpdatesApiV02, DesktopUpdatePlansApiV01, UpdateComponentV02, UpdatePlanSnapshotV01 } from "@vua/contracts";
import { GatewayProvider } from "../../src/renderer/gateway/index.ts";
import { emptyGateway } from "../../src/renderer/gateway/empty-gateway.ts";
import { ComponentUpdatePanel } from "../../src/renderer/features/modules/ComponentUpdatePanel.tsx";
import { UpdatePlanPanel } from "../../src/renderer/features/modules/UpdatePlanPanel.tsx";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";
import "../../src/renderer/app-shell.css";
import "../../src/renderer/features/settings/settings.css";

const listeners = new Set<(view: ComponentUpdateSnapshotV02) => void>();
const planListeners = new Set<(view: UpdatePlanSnapshotV01) => void>();
const initial = (component: UpdateComponentV02): ComponentUpdateSnapshotV02 => ({ schemaVersion: "0.2", component, available: true, state: "idle", installedVersion: component === "host" ? "0.7.0" : "0.2.0", installedBuild: "a".repeat(40), rollbackAvailable: false });
const views = { host: initial("host"), amf: initial("amf") };
let pair: UpdatePlanSnapshotV01 = { schemaVersion: "0.1", available: true, state: "idle", rollbackAvailable: false };
let applyAllowed = false; const actions: string[] = [];
const target = (component: UpdateComponentV02) => ({ manifestId: (component === "host" ? "b" : "c").repeat(64), version: component === "host" ? "0.8.0" : "0.3.0", build: "d".repeat(40), releaseUrl: `https://ph-r.com/vua/components/${component}/v${component === "host" ? "0.8.0" : "0.3.0"}/`, activation: "restart" as const, downloadBytes: component === "host" ? 6_291_456 : 2_097_152, reusedBytes: 100_000_000, runtimeBytes: 0 });
function emit(component: UpdateComponentV02, value: Partial<ComponentUpdateSnapshotV02>) { views[component] = { ...views[component], ...value }; for (const listener of listeners) listener(views[component]); return views[component]; }
function emitPlan(value: Partial<UpdatePlanSnapshotV01>) { pair = { ...pair, ...value }; for (const listener of planListeners) listener(pair); return pair; }
const updates: DesktopComponentUpdatesApiV02 = {
  snapshot: async component => views[component],
  check: async component => emit(component, { state: "checking", error: undefined, target: undefined }),
  stage: async component => { actions.push(`stage:${component}`); return emit(component, { state: "downloading", progress: { completed: 2_097_152, total: 6_291_456, bytesPerSecond: 1_048_576, reusedBytes: 100_000_000, fullPack: false } }); },
  cancel: async component => emit(component, { state: "idle", error: "cancelled" }),
  apply: async component => { actions.push(`apply:${component}`); return emit(component, applyAllowed ? { state: "activating", error: undefined } : { state: "waiting-for-idle", error: "work_is_running" }); },
  restore: async component => views[component],
  subscribe: listener => { listeners.add(listener); return () => listeners.delete(listener); },
};
const plans: DesktopUpdatePlansApiV01 = {
  snapshot: async () => pair,
  propose: async () => emitPlan({ state: "proposed", plan: { id: "e".repeat(64), installed: { host: "0.7.0", amf: "0.2.0" }, activation: "restart", targets: [{ component: "host", target: target("host") }, { component: "amf", target: target("amf") }] } }),
  stage: async () => { actions.push("stage:pair"); return emitPlan({ state: "staged" }); },
  cancel: async () => emitPlan({ state: "interrupted", error: "cancelled" }),
  apply: async () => { actions.push("apply:pair"); return emitPlan({ state: "activating" }); },
  restore: async () => pair,
  subscribe: listener => { planListeners.add(listener); return () => planListeners.delete(listener); },
};
const gateway = { ...emptyGateway(), componentUpdates: updates, updatePlans: plans };
createRoot(document.getElementById("root")!).render(<GatewayProvider gateway={gateway}>
  <main className="vua-page" style={{ maxWidth: 1280, margin: "0 auto", padding: 32 }}>
    <header><h1 className="vua-title">Controlled update panels</h1><p>Synthetic versions and state transitions; no installation or remote delivery acceptance.</p></header>
    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 24 }}><ComponentUpdatePanel component="host" /><ComponentUpdatePanel component="amf" /></div>
    <UpdatePlanPanel />
  </main>
</GatewayProvider>);
const panel = (component: UpdateComponentV02) => document.querySelector(`[data-component-update=${component}]`)!;
const button = (root: Element, text: string) => { const result = [...root.querySelectorAll("button")].find(candidate => candidate.textContent === text); if (!result) throw Error(`Button missing: ${text}`); return result; };
async function wait(predicate: () => unknown) { for (let i = 0; i < 200; i++) { if (predicate()) return; await new Promise(resolve => setTimeout(resolve, 25)); } throw Error("UI condition timed out"); }
const review = {
  async run() {
    const checks: string[] = []; const check = (value: unknown, label: string) => { if (!value) throw Error(label); checks.push(label); };
    await wait(() => panel("host")?.querySelector("input") && document.querySelector("[data-update-plan]"));
    (panel("host").querySelector("input") as HTMLInputElement).click(); await wait(() => !(panel("host").querySelector("input") as HTMLInputElement).checked);
    check((panel("amf").querySelector("input") as HTMLInputElement).checked, "Host automatic check does not disable AMF checks");
    button(panel("host"), "Check for updates").click(); await wait(() => panel("host").textContent?.includes("Checking"));
    emit("host", { state: "idle", target: target("host"), checkedAt: "2026-10-26T00:00:00Z", latestVersion: "0.8.0" });
    await wait(() => panel("host").textContent?.includes("0.8.0 is available"));
    button(panel("amf"), "Check for updates").click(); await wait(() => panel("amf").textContent?.includes("Checking"));
    button(panel("amf"), "Cancel").click(); await wait(() => panel("amf").textContent?.includes("Check cancelled"));
    check(panel("host").textContent?.includes("0.8.0 is available"), "Cancelling AMF leaves the host target visible");
    button(panel("host"), "Download update").click(); await wait(() => panel("host").querySelector("progress"));
    check((panel("host").querySelector("[data-nav-id=host-check-updates]") as HTMLButtonElement).disabled, "Host check is disabled during its download");
    emit("host", { state: "staged", progress: undefined }); await wait(() => panel("host").textContent?.includes("Restart and apply"));
    button(panel("host"), "Restart and apply").click(); await wait(() => panel("host").textContent?.includes("tasks and downloads"));
    check(views.host.state === "waiting-for-idle", "Busy work leaves a staged update pending");
    applyAllowed = true; button(panel("host"), "Restart and apply").click(); await wait(() => views.host.state === "activating");
    check(!actions.includes("stage:amf") && !actions.includes("apply:amf"), "A host action never stages or applies AMF");
    const pairPanel = document.querySelector("[data-update-plan]")!; button(pairPanel, "Find a compatible pair").click();
    await wait(() => pairPanel.textContent?.includes("VUA 0.7.0 → 0.8.0 · AMF 0.2.0 → 0.3.0"));
    check(!actions.includes("stage:pair"), "Both versions are reviewed before pair download");
    button(pairPanel, "Download both updates").click(); await wait(() => pairPanel.textContent?.includes("Restart and apply both"));
    button(pairPanel, "Restart and apply both").click(); await wait(() => actions.includes("apply:pair"));
    check(actions.filter(action => action === "apply:pair").length === 1, "Pair activation uses its explicit action");
    return checks;
  },
  present() { emit("host", { state: "staged", error: undefined }); emit("amf", { state: "idle", requiresHost: ">=0.8.0, <0.9.0", error: undefined }); emitPlan({ state: "proposed" }); },
};
(window as unknown as { componentUpdateReview: typeof review }).componentUpdateReview = review;
