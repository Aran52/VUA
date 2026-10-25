import { describe, expect, it, vi } from "vitest";
import type { InstalledContext } from "./installed-context.js";
import { ComponentUpdater, isUpdateComponent, type ComponentRun } from "./component-updater.js";
import type { ChildProcessWithoutNullStreams } from "node:child_process";

const context = { root: "C:\\isolated\\VUA", host: { host: { version: "0.7.0", build: "a".repeat(40), files: [] } }, amf: { amf: { version: "0.2.0", build: "b".repeat(40) } } } as unknown as InstalledContext;
const result = (component: string) => ({ schemaVersion: "0.2", component, available: true, state: "idle", installedVersion: component === "host" ? "0.7.0" : "0.2.0", installedBuild: "a".repeat(40), rollbackAvailable: false, latestVersion: component === "host" ? "0.7.1" : "0.2.1",
  target: { manifestId: (component === "host" ? "c" : "d").repeat(64), version: component === "host" ? "0.7.1" : "0.2.1", build: "e".repeat(40), releaseUrl: `https://ph-r.com/vua/components/${component}/v${component === "host" ? "0.7.1" : "0.2.1"}/`, activation: component === "host" ? "restart" : "reload", downloadBytes: 12, reusedBytes: 80, runtimeBytes: 0 } });
describe("independent component discovery", () => {
  it("keeps a completely staged update available if a subsequent check is offline", async () => {
    let offline = false;
    const run: ComponentRun = async (command, args) => { if (offline && command === "component-check") throw Error("feed_unavailable"); return { ...result(args[1]!), state: command === "component-stage" ? "staged" : "idle" }; };
    const updates = new ComponentUpdater({ context, run }); await updates.check("host"); await updates.stage("host", result("host").target.manifestId); offline = true;
    const view = await updates.check("host"); expect(view.state).toBe("staged"); expect(view.target?.manifestId).toBe(result("host").target.manifestId);
    expect(view.error).toBe("feed_unavailable"); expect(view.latestVersion).toBeUndefined(); expect(view.checkedAt).toBeUndefined();
  });
  it("stages only the checked host and waits for lifecycle admission before restarting", async () => {
    let ready = false; const restart = vi.fn(); const run = vi.fn<ComponentRun>(async (command, args) => command === "component-request" ? {} : { ...result(args[1]!), state: command === "component-stage" ? "staged" : "idle" });
    const updates = new ComponentUpdater({ context, run, prepareRestart: async () => ready, restart });
    await updates.check("host");
    await expect(updates.stage("host", "f".repeat(64))).rejects.toThrow("invalid_update_identity");
    expect((await updates.stage("host", result("host").target.manifestId)).state).toBe("staged");
    expect((await updates.apply("host")).state).toBe("waiting-for-idle"); expect(restart).not.toHaveBeenCalled();
    ready = true; expect((await updates.apply("host")).state).toBe("activating"); expect(restart).toHaveBeenCalledWith("host", false);
    expect(run.mock.calls.filter(([command]) => command === "component-stage").map(([, args]) => args)).toEqual([["--component", "host", "--manifest", result("host").target.manifestId]]);
  });
  it("preserves interrupted host staging without hiding the independent AMF result", async () => {
    const run: ComponentRun = async (command, args) => { if (command === "component-stage") throw Error("insufficient_disk_space"); return result(args[1]!); };
    const updates = new ComponentUpdater({ context, run }); await updates.check("host"); await updates.check("amf");
    expect((await updates.stage("host", result("host").target.manifestId)).error).toBe("insufficient_disk_space");
    expect((await updates.snapshot("amf")).target?.version).toBe("0.2.1");
  });
  it("keeps host and AMF results separate, including disabled-module checks", async () => {
    const run = vi.fn<ComponentRun>(async (_command, args) => result(args[1]!));
    const updates = new ComponentUpdater({ context, run });
    await Promise.all([updates.check("host"), updates.check("amf")]);
    expect((await updates.snapshot("host")).target?.version).toBe("0.7.1");
    expect((await updates.snapshot("amf")).target?.version).toBe("0.2.1");
    expect(run.mock.calls.filter(([command]) => command === "component-check").map(([, args]) => args)).toEqual([["--component", "host"], ["--component", "amf"]]);
  });
  it("cancels only the chosen job and rejects its late successful result", async () => {
    let finishHost!: (value: Record<string, unknown>) => void; let finishAmf!: (value: Record<string, unknown>) => void;
    const hostWrite = vi.fn(); const amfWrite = vi.fn();
    const run: ComponentRun = async (_command, args, _report, onChild) => {
      const component = args[1];
      onChild({ stdin: { write: component === "host" ? hostWrite : amfWrite } } as unknown as ChildProcessWithoutNullStreams);
      return new Promise(resolve => { if (component === "host") finishHost = resolve; else finishAmf = resolve; });
    };
    const updates = new ComponentUpdater({ context, run });
    const host = updates.check("host"); const amf = updates.check("amf");
    await updates.cancel("amf"); expect(amfWrite).toHaveBeenCalledWith("cancel\n"); expect(hostWrite).not.toHaveBeenCalled();
    finishHost(result("host")); finishAmf(result("amf"));
    expect((await host).target?.version).toBe("0.7.1"); expect((await amf).target).toBeUndefined();
    expect((await amf).error).toBe("cancelled");
  });
  it("never restores a stale latest claim after a failed check or corrupt native reply", async () => {
    let bad = false;
    const run: ComponentRun = async (command, args) => { if (command === "component-check" && bad) throw Error("component_feed_replayed"); return result(args[1]!); };
    const updates = new ComponentUpdater({ context, run }); await updates.check("host"); await updates.check("amf"); bad = true;
    const failed = await updates.check("host"); expect(failed.state).toBe("failed"); expect(failed.target).toBeUndefined(); expect(failed.latestVersion).toBeUndefined(); expect(failed.checkedAt).toBeUndefined();
    expect((await updates.snapshot("amf")).target?.version).toBe("0.2.1");
    const corrupt = new ComponentUpdater({ context, run: async () => ({ ...result("host"), component: "amf" }) });
    expect((await corrupt.check("host")).state).toBe("failed");
  });
  it("requires a signed installer helper and accepts no renderer-selected component", async () => {
    const run = vi.fn(); const unavailable = new ComponentUpdater({ run });
    expect((await unavailable.check("host")).available).toBe(false); expect(run).not.toHaveBeenCalled();
    expect((await new ComponentUpdater({ context }).snapshot("amf")).available).toBe(false);
    expect(isUpdateComponent("host")).toBe(true); expect(isUpdateComponent("https://untrusted.test")).toBe(false);
  });
  it("rejects target URLs, AMF runtime downloads and wrong activation semantics", async () => {
    for (const target of [ { ...result("amf").target, runtimeBytes: 1 }, { ...result("host").target, activation: "reload" }, { ...result("amf").target, releaseUrl: "https://untrusted.test/" } ]) {
      const component = target.version === "0.7.1" ? "host" : "amf";
      const updates = new ComponentUpdater({ context, run: async () => ({ ...result(component), target }) });
      expect((await updates.check(component)).error).toBe("invalid_update_result");
    }
  });
});
