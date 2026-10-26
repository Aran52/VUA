import { describe, expect, it, vi } from "vitest";
import type { ApplicationRequestV01, ApplicationResponseV01 } from "@vua/contracts";
import { MockOrchestratorProviderV01 } from "@vua/orchestrator-provider";
import { amfUpdateFixture } from "../../scripts/fixtures/amf-update-native.js";
import { AmfGenerations } from "./amf-generations.js";
import { AmfHotUpdater } from "./amf-hot-updater.js";
import { ModuleProvider } from "./module-provider.js";
import type { ComponentRun } from "./component-updater.js";
class IdlePort extends MockOrchestratorProviderV01 {
  stops = 0;
  override async invoke(request: ApplicationRequestV01): Promise<ApplicationResponseV01> {
    if (request.method === "task.list") return { contractVersion: "0.1", requestId: request.requestId, ok: true, value: { contractVersion: "0.1", revision: 1, tasks: [] } };
    return super.invoke(request);
  }
  override async prepareShutdown(request: { timeoutMs: number }) { this.stops++; return super.prepareShutdown(request); }
}
async function setup(options: { enabled?: boolean; rejectUi?: boolean; rejectQuiesce?: boolean; loseFinish?: boolean; restart?: boolean } = {}) {
  const fixture = amfUpdateFixture(); const generations = new AmfGenerations(fixture.context);
  const host = new IdlePort(), old = new IdlePort(), next = new IdlePort(); const persist = vi.fn();
  const provider = new ModuleProvider({ host, enabled: false, persist, createAmf: () => generations.backend?.id === fixture.next.id ? next : old });
  await provider.start(); if (options.enabled !== false) await provider.setAmfEnabled(true);
  let selected = fixture.old; let committed = false; const commands: string[] = [];
  const run: ComponentRun = async command => {
    commands.push(command);
    if (command === "component-amf-preflight" || command === "component-amf-begin") {
      if (command === "component-amf-begin") selected = fixture.next;
      return { schemaVersion: "0.1", manifestId: fixture.next.id, version: fixture.next.version, build: fixture.next.build, activation: options.restart ? "restart" : "reload" };
    }
    if (command === "component-amf-finish") { committed = true; if (options.loseFinish) throw Error("helper_reply_lost"); return { committed: true, manifestId: fixture.next.id }; }
    if (command === "component-amf-recover") { if (!committed) selected = fixture.old; return { schemaVersion: "0.1", manifestId: selected.id, version: selected.version, build: selected.build, healthy: committed }; }
    return {};
  };
  const updater = new AmfHotUpdater({ context: fixture.context, generations, provider, run, timeoutMs: 500 });
  updater.subscribe(view => {
    if (view.phase === "quiescing" || view.phase === "loading") void updater.respond(view.transactionId!, view.phase,
      view.phase === "quiescing" ? !options.rejectQuiesce : !options.rejectUi,
      view.phase === "quiescing" && options.rejectQuiesce ? "busy" : options.rejectUi ? "failed" : undefined);
  });
  return { fixture, generations, host, old, next, persist, provider, updater, commands, selected: () => selected };
}
describe("AMF provider/UI generation transaction", () => {
  it("verifies the startup UI before a data trial commits without selecting or stopping providers", async () => {
    for (const rejectUi of [false, true]) {
      const state = await setup({ rejectUi });
      try {
        expect(await state.updater.verifyInitialUi()).toBe(!rejectUi);
        expect(state.commands).toEqual([]); expect(state.old.stops).toBe(0); expect(state.host.stops).toBe(0);
        expect(state.generations.active?.id).toBe(state.fixture.old.id); expect((await state.updater.snapshot()).phase).toBe("idle");
      } finally { await state.provider.prepareShutdown({ timeoutMs: 100 }); state.fixture.dispose(); }
    }
  });
  it("commits one verified generation and keeps host provider and preference alive", async () => {
    const state = await setup();
    try {
      expect(await state.updater.apply(false, state.fixture.next.id)).toBe("updated");
      expect(state.generations.active?.id).toBe(state.fixture.next.id); expect(state.selected().id).toBe(state.fixture.next.id);
      expect(state.host.stops).toBe(0); expect(state.old.stops).toBe(1); expect(state.persist.mock.calls).toEqual([[true]]);
      const snapshot = await state.updater.snapshot(); expect(snapshot.phase).toBe("idle"); expect(snapshot.active?.version).toBe("0.2.1");
      expect(JSON.stringify(snapshot)).not.toContain(state.fixture.root); expect(JSON.stringify(snapshot)).not.toContain(".exe");
    } finally { await state.provider.prepareShutdown({ timeoutMs: 100 }); state.fixture.dispose(); }
  });
  it("refuses a busy/unsaved UI before stopping or switching AMF", async () => {
    const state = await setup({ rejectQuiesce: true });
    try {
      expect(await state.updater.apply(false, state.fixture.next.id)).toBe("busy");
      expect(state.old.stops).toBe(0); expect(state.commands).not.toContain("component-amf-begin"); expect(state.generations.active?.id).toBe(state.fixture.old.id);
    } finally { await state.provider.prepareShutdown({ timeoutMs: 100 }); state.fixture.dispose(); }
  });
  it("restores the previous UI/provider together if candidate UI health fails", async () => {
    const state = await setup({ rejectUi: true });
    try {
      expect(await state.updater.apply(false, state.fixture.next.id)).toBe("failed");
      expect(state.commands).toContain("component-amf-recover"); expect(state.commands).not.toContain("component-amf-finish");
      expect(state.next.stops).toBe(1); expect(state.provider.amfReady()).toBe(true); expect(state.generations.active?.id).toBe(state.fixture.old.id);
      expect(state.selected().id).toBe(state.fixture.old.id); expect(state.host.stops).toBe(0); expect(state.persist.mock.calls).toEqual([[true]]);
    } finally { await state.provider.prepareShutdown({ timeoutMs: 100 }); state.fixture.dispose(); }
  });
  it("keeps a committed target coherent if the helper reply is lost after health commit", async () => {
    const state = await setup({ loseFinish: true });
    try {
      expect(await state.updater.apply(false, state.fixture.next.id)).toBe("updated");
      expect(state.generations.active?.id).toBe(state.selected().id); expect(state.generations.active?.id).toBe(state.fixture.next.id);
      expect(state.provider.amfReady()).toBe(true); expect(state.host.stops).toBe(0);
    } finally { await state.provider.prepareShutdown({ timeoutMs: 100 }); state.fixture.dispose(); }
  });
  it("validates and stores a disabled module without enabling its provider", async () => {
    const state = await setup({ enabled: false });
    try {
      expect(await state.updater.apply(false, state.fixture.next.id)).toBe("updated");
      expect(state.provider.moduleSnapshot().state).toBe("absent"); expect(state.old.status().state).toBe("stopped"); expect(state.next.status().state).toBe("stopped"); expect(state.persist).not.toHaveBeenCalled();
      await expect(state.updater.respond("x".repeat(32), "loading", true)).rejects.toThrow("invalid_amf_transition");
    } finally { await state.provider.prepareShutdown({ timeoutMs: 100 }); state.fixture.dispose(); }
  });
  it("requires a second explicit restart action for a legacy or changed-format target", async () => {
    const state = await setup({ restart: true });
    try {
      expect(await state.updater.apply(false, state.fixture.next.id)).toBe("restart_required"); expect(state.commands).toEqual(["component-amf-preflight"]); expect(state.old.stops).toBe(0);
    } finally { await state.provider.prepareShutdown({ timeoutMs: 100 }); state.fixture.dispose(); }
  });
});
