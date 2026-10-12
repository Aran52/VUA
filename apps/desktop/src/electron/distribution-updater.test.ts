import { describe, expect, it, vi } from "vitest";
import { DistributionUpdater, validProgress } from "./distribution-updater.js";
import type { InstalledContext } from "./installed-context.js";
const context = { root: "C:\\isolated\\VUA", host: { host: { version: "0.6.7" }, runtime: { version: "44.1.1" } }, amf: { amf: { version: "0.1.0" } } } as InstalledContext;
const id = "a".repeat(64);
const target = { manifestId: id, hostVersion: "0.6.7", amfVersion: "0.1.1", compatible: true, newer: true, downloadBytes: 1024, releaseUrl: "https://github.com/VUA-Project/VUA/releases/tag/v0.6.7" };
describe("installed distribution updates", () => {
  it("keeps legacy ZIP/dev honest and never starts a helper", async () => {
    const run = vi.fn(); const updater = new DistributionUpdater({ run, prepareRestart: async () => true, restart: vi.fn() });
    expect((await updater.check()).available).toBe(false); expect((await updater.stage(id)).available).toBe(false); expect(run).not.toHaveBeenCalled();
  });
  it("only stages a checked opaque identity and rejects incompatible targets", async () => {
    const run = vi.fn(async command => command === "check" ? { ...target, compatible: false } : { schemaVersion: "0.1", hostVersion: "0.6.7", amfVersion: "0.1.0", runtimeVersion: "44.1.1" });
    const updater = new DistributionUpdater({ context, run, prepareRestart: async () => true, restart: vi.fn() }); await updater.check();
    await expect(updater.stage("../unsafe")).rejects.toThrow("invalid_update_identity");
    expect((await updater.stage(id)).error).toBe("amf_requires_host_update"); expect(run.mock.calls.some(([method]) => method === "stage")).toBe(false);
  });
  it("never replaces a failed check with a cached latest-version claim", async () => {
    let failed = false; const run = vi.fn(async command => { if (command === "check") { if (failed) throw Error("offline"); return target; } return { schemaVersion: "0.1", hostVersion: "0.6.7", amfVersion: "0.1.0", runtimeVersion: "44.1.1" }; });
    const updater = new DistributionUpdater({ context, run, prepareRestart: async () => true, restart: vi.fn() }); expect((await updater.check()).target?.amfVersion).toBe("0.1.1"); failed = true;
    const view = await updater.check(); expect(view.state).toBe("failed"); expect(view.target).toBeUndefined(); expect(view.amfVersion).toBe("0.1.0");
  });
  it("preserves interrupted staging and discovers its explicit continuation after restart", async () => {
    const run = vi.fn(async command => { if (command === "check") return target; if (command === "stage") throw Error("cancelled"); return { schemaVersion: "0.1", hostVersion: "0.6.7", amfVersion: "0.1.0", runtimeVersion: "44.1.1", pending: "interrupted", pendingManifestId: id, targetAmfVersion: "0.1.1" }; });
    const updater = new DistributionUpdater({ context, run, prepareRestart: async () => true, restart: vi.fn() }); await updater.check(); expect((await updater.stage(id)).state).toBe("interrupted");
    const next = new DistributionUpdater({ context, run, prepareRestart: async () => true, restart: vi.fn() }); const view = await next.snapshot(); expect(view.pendingManifestId).toBe(id); expect(view.state).toBe("interrupted");
    expect(run.mock.calls.filter(([method]) => method === "stage")).toHaveLength(1);
  });
  it("waits for lifecycle admission before requesting activation or restoring", async () => {
    const restart = vi.fn(); let ready = false;
    const run = vi.fn(async command => command === "check" ? target : command === "snapshot" ? { schemaVersion: "0.1", hostVersion: "0.6.7", amfVersion: "0.1.0", runtimeVersion: "44.1.1", rollbackAvailable: true } : {});
    const updater = new DistributionUpdater({ context, run, prepareRestart: async () => ready, restart }); await updater.check(); await updater.stage(id);
    expect((await updater.apply()).error).toBe("work_is_running"); expect(restart).not.toHaveBeenCalled(); expect(run.mock.calls.some(([method]) => method === "request-activation")).toBe(false);
    ready = true; await updater.apply(); expect(run.mock.calls.at(-1)?.[0]).toBe("request-activation"); expect(restart).toHaveBeenCalledWith("apply-update");
  });
  it("rejects unbounded/invalid progress observations", () => {
    expect(validProgress({ phase: "downloading", completed: 4, total: 10, bytesPerSecond: 1, reusedBytes: 2, fullPack: false })).toBe(true);
    expect(validProgress({ phase: "downloading", completed: Infinity, total: 10, bytesPerSecond: 1, reusedBytes: 2, fullPack: false })).toBe(false);
  });
});
