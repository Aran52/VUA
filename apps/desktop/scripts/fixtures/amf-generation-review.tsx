// Real module imports/React lifecycle with synthetic generation metadata and services.
import { createRoot } from "react-dom/client";
import { Suspense, useState } from "react";
import type { AmfUiGenerationV01, AmfUiRuntimeSnapshotV01, AmfUiTransitionV01, DesktopAmfUiRuntimeApiV01 } from "@vua/contracts";
import { GatewayProvider } from "../../src/renderer/gateway/index.ts";
import { emptyGateway } from "../../src/renderer/gateway/empty-gateway.ts";
import { AmfSurface, AmfUiRuntimeBridge } from "../../src/renderer/features/modules/AmfSurface.tsx";
import "@vua/design-system/tokens.css";
import "@vua/design-system/base.css";
const old: AmfUiGenerationV01 = { id: "a".repeat(64), instanceId: "1".repeat(32), version: "0.2.0", build: "a".repeat(40), uiAbi: "amf-ui/0.1" };
const failed: AmfUiGenerationV01 = { id: "b".repeat(64), instanceId: "2".repeat(32), version: "0.2.1", build: "b".repeat(40), uiAbi: "amf-ui/0.1" };
const retry: AmfUiGenerationV01 = { ...failed, instanceId: "3".repeat(32) };
let view: AmfUiRuntimeSnapshotV01 = { schemaVersion: "0.1", available: true, phase: "idle", active: old };
const listeners = new Set<(value: AmfUiRuntimeSnapshotV01) => void>();
let reply: ((value: boolean) => void) | undefined;
let show: (value: boolean) => void;
const port: DesktopAmfUiRuntimeApiV01 = {
  snapshot: async () => view,
  respond: async (id, phase, accepted) => { if (id !== view.transactionId || phase !== view.phase) throw Error("Unexpected acknowledgement"); const callback = reply; reply = undefined; callback?.(accepted); },
  subscribe: listener => { listeners.add(listener); return () => listeners.delete(listener); },
};
function emit(next: AmfUiRuntimeSnapshotV01) { view = next; for (const listener of listeners) listener(view); }
function request(phase: AmfUiTransitionV01, candidate: AmfUiGenerationV01) {
  return new Promise<boolean>(resolve => { reply = resolve; emit({ ...view, phase, candidate, transactionId: "d".repeat(32) }); });
}
function Fixture() {
  const [visible, setVisible] = useState(true); show = setVisible;
  return <GatewayProvider gateway={{ ...emptyGateway(), amfUiRuntime: port }}><AmfUiRuntimeBridge /><main style={{ padding: 24 }}>
    <h1>Controlled AMF generation review</h1><p>Synthetic module code and services; no installation, user data or remote delivery acceptance.</p>
    {visible ? <Suspense fallback={<p>Loading fixture…</p>}><AmfSurface page="warehouse" creatorReady navigate={() => {}} prepareEnv={() => {}} /></Suspense> : <p data-module-management>Module management</p>}
  </main></GatewayProvider>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
const globals = window as unknown as { __amfFixtureDirty?: boolean; __amfFixtureFail?: boolean; __amfFixtureEvents?: Record<string, number>; amfGenerationReview?: unknown };
async function wait(predicate: () => unknown) { for (let i = 0; i < 400; i++) { if (predicate()) return; await new Promise(resolve => setTimeout(resolve, 25)); } throw Error(`AMF generation UI condition timed out: ${predicate.toString().slice(0, 240)}; phase=${view.phase}; text=${document.body.innerText.slice(0, 700)}`); }
globals.amfGenerationReview = {
  async run() {
    const checks: string[] = []; const check = (value: unknown, name: string) => { if (!value) throw Error(name); checks.push(name); };
    await wait(() => document.querySelector("[data-amf-fixture]")?.textContent?.includes("old helper"));
    check(document.documentElement.dataset.amfUiVersion === "0.2.0", "Initial module version is bound to its entry");
    check(!(await request("quiescing", failed)) && document.querySelector('[data-amf-fixture="0.2.0"]'), "An open AMF surface refuses replacement without unmounting its state");
    emit({ ...view, phase: "idle", transactionId: undefined, candidate: undefined });
    show(false); await wait(() => document.querySelector("[data-module-management]") && !document.querySelector("[data-amf-surface]"));
    globals.__amfFixtureDirty = true; check(!(await request("quiescing", failed)), "Unsaved module state refuses replacement");
    emit({ ...view, phase: "idle", transactionId: undefined, candidate: undefined }); globals.__amfFixtureDirty = false;
    check(await request("quiescing", failed), "An idle unmounted module can quiesce");
    globals.__amfFixtureFail = true; check(!(await request("loading", failed)), "A failed candidate import cannot acknowledge health");
    emit({ ...view, phase: "restoring", active: old, candidate: failed });
    await new Promise(resolve => setTimeout(resolve, 25));
    emit({ schemaVersion: "0.1", available: true, phase: "idle", active: old });
    window.dispatchEvent(new Event("amf-fixture-event"));
    check((globals.__amfFixtureEvents?.["0.2.0"] ?? 0) === 1 && !globals.__amfFixtureEvents?.["0.2.1"], "Failed import effects are removed while the previous module remains intact");
    globals.__amfFixtureFail = false;
    check(await request("quiescing", retry), "A retry can quiesce the retained previous generation");
    check(await request("loading", retry), "The same artifact retries through a fresh import graph");
    emit({ schemaVersion: "0.1", available: true, phase: "idle", active: retry });
    await wait(() => !document.querySelector(`link[data-amf-generation="${old.instanceId}"]`));
    show(true); await wait(() => document.querySelector("[data-amf-fixture]")?.textContent?.includes("new helper"));
    check(document.documentElement.dataset.amfUiVersion === "0.2.1", "Relative helper imports and visible UI switch to the new version");
    const oldCount = globals.__amfFixtureEvents?.["0.2.0"];
    window.dispatchEvent(new Event("amf-fixture-event"));
    check(globals.__amfFixtureEvents?.["0.2.0"] === oldCount && globals.__amfFixtureEvents?.["0.2.1"] === 1, "Only the committed module's listener remains active");
    check(document.querySelectorAll("link[data-amf-generation]").length === 1, "Only one generation stylesheet is retained");
    return checks;
  },
};
