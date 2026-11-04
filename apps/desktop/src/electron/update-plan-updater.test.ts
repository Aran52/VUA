import { describe, expect, it, vi } from "vitest";
import type { ChildProcessWithoutNullStreams } from "node:child_process";
import type { InstalledContext } from "./installed-context.js";
import type { ComponentRun } from "./component-updater.js";
import { UpdatePlanUpdater } from "./update-plan-updater.js";

const context = { host: { host: { files: [] } } } as unknown as InstalledContext;
const id = "a".repeat(64);
const proposal = () => ({ schemaVersion: "0.1", available: true, state: "proposed", rollbackAvailable: false, plan: {
  id, installed: { host: "0.7.0", amf: "0.2.0" }, activation: "restart", targets: ["host", "amf"].map(component => ({ component,
    target: { manifestId: (component === "host" ? "b" : "c").repeat(64), version: component === "host" ? "0.8.0" : "0.3.0", build: "d".repeat(40),
      releaseUrl: `https://ph-r.com/vua/components/${component}/v${component === "host" ? "0.8.0" : "0.3.0"}/`, activation: "restart", downloadBytes: 20, reusedBytes: 100, runtimeBytes: 0 } })),
} });
const base = { context, externalBusy: () => false, prepareRestart: async () => true, restart: vi.fn() };
describe("explicit compatible pair controller", () => {
  it("requires a reviewed identity and complete staging before applying both", async () => {
    let ready = false; const restart = vi.fn();
    const run = vi.fn<ComponentRun>(async command => command === "plan-request" ? {} : { ...proposal(), state: command === "plan-stage" ? "staged" : "proposed" });
    const updates = new UpdatePlanUpdater({ ...base, run, restart, prepareRestart: async () => ready });
    await updates.propose(); await expect(updates.stage("e".repeat(64))).rejects.toThrow("invalid_update_identity");
    expect((await updates.apply(id)).state).toBe("proposed"); expect(restart).not.toHaveBeenCalled();
    expect((await updates.stage(id)).state).toBe("staged");
    expect((await updates.apply(id)).state).toBe("waiting-for-idle"); expect(restart).not.toHaveBeenCalled();
    ready = true; expect((await updates.apply(id)).state).toBe("activating"); expect(restart).toHaveBeenCalledWith("plan-apply", id);
    expect(run.mock.calls.filter(([command]) => command === "plan-stage").map(([, args]) => args)).toEqual([["--plan", id]]);
  });
  it("does not interrupt another component operation or silently use a pair on its behalf", async () => {
    const run = vi.fn<ComponentRun>(); const restart = vi.fn();
    const updates = new UpdatePlanUpdater({ ...base, run, restart, externalBusy: () => true });
    expect((await updates.propose()).error).toBe("work_is_running"); expect(run).not.toHaveBeenCalled(); expect(restart).not.toHaveBeenCalled();
  });
  it("cancels a proposal and discards even a late successful response", async () => {
    let finish!: (value: Record<string, unknown>) => void; const write = vi.fn();
    const run: ComponentRun = async (_command, _args, _report, child) => {
      child({ stdin: { write } } as unknown as ChildProcessWithoutNullStreams);
      return new Promise(resolve => { finish = resolve; });
    };
    const updates = new UpdatePlanUpdater({ ...base, run }); const pending = updates.propose(); await updates.cancel();
    expect(write).toHaveBeenCalledWith("cancel\n"); finish(proposal());
    expect((await pending).plan).toBeUndefined(); expect((await pending).error).toBe("cancelled");
  });
  it("shows errors for unavailable feeds and corrupt pair payloads, without a latest claim", async () => {
    const offline = new UpdatePlanUpdater({ ...base, run: async () => { throw Error("feed_unavailable"); } });
    expect((await offline.propose()).state).toBe("failed"); expect((await offline.propose()).plan).toBeUndefined();
    const invalid = proposal(); invalid.plan.targets[1]!.target.runtimeBytes = 100;
    const updates = new UpdatePlanUpdater({ ...base, run: async () => invalid });
    expect((await updates.propose()).error).toBe("invalid_update_plan");
  });
  it("offers pair restore separately from an independent host or AMF restore", async () => {
    const restart = vi.fn(); const run: ComponentRun = async () => ({ schemaVersion: "0.1", available: true, state: "idle", rollbackAvailable: true });
    const updates = new UpdatePlanUpdater({ ...base, run, restart }); await updates.snapshot(); await updates.restore();
    expect(restart).toHaveBeenCalledWith("plan-restore", undefined);
  });
});
