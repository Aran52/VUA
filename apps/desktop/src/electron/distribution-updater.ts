import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import path from "node:path";
import type { DesktopDistributionApiV01, DistributionProgressV01, DistributionSnapshotV01, DistributionTargetV01 } from "@vua/contracts";
import type { InstalledContext } from "./installed-context.js";

type Run = (command: string, args: readonly string[], report: (progress: DistributionProgressV01) => void, child: (process: ChildProcessWithoutNullStreams) => void) => Promise<Record<string, unknown>>;
interface Options {
  context?: InstalledContext | undefined; run?: Run;
  prepareRestart: () => Promise<boolean>; restart: (command: "apply-update" | "rollback-launch") => void;
}
export function validProgress(value: unknown): value is DistributionProgressV01 {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return typeof v.phase === "string" && v.phase.length < 40 && typeof v.fullPack === "boolean"
    && ["completed", "total", "bytesPerSecond", "reusedBytes"].every(key => typeof v[key] === "number" && Number.isFinite(v[key]) && (v[key] as number) >= 0);
}
function validTarget(value: Record<string, unknown>): value is Record<string, unknown> & DistributionTargetV01 {
  return typeof value.manifestId === "string" && /^[a-f0-9]{64}$/.test(value.manifestId)
    && typeof value.amfVersion === "string" && typeof value.hostVersion === "string" && typeof value.compatible === "boolean" && typeof value.newer === "boolean"
    && typeof value.downloadBytes === "number" && value.downloadBytes >= 0 && typeof value.releaseUrl === "string"
    && /^https:\/\/github\.com\/VUA-Project\/VUA\/releases\/tag\/v[\d.]+$/.test(value.releaseUrl);
}
/** The release helper owns signature/path/content validation. Main owns the fixed
 * command/root, Renderer admission, lifecycle gates and progress projection. */
export class DistributionUpdater implements DesktopDistributionApiV01 {
  private view: DistributionSnapshotV01;
  private child: ChildProcessWithoutNullStreams | undefined;
  private readonly listeners = new Set<(snapshot: DistributionSnapshotV01) => void>();
  private readonly run: Run;
  private applying = false;
  constructor(private readonly options: Options) {
    this.view = { schemaVersion: "0.1", available: !!options.context, state: "idle",
      ...(options.context ? { hostVersion: options.context.host.host.version, amfVersion: options.context.amf.amf.version, runtimeVersion: options.context.host.runtime.version } : {}) };
    this.run = options.run ?? ((command, args, report, child) => this.native(command, args, report, child));
  }
  private native(command: string, args: readonly string[], report: (progress: DistributionProgressV01) => void, onChild: (child: ChildProcessWithoutNullStreams) => void): Promise<Record<string, unknown>> {
    const context = this.options.context;
    if (!context) return Promise.reject(Error("installer_required"));
    return new Promise((resolve, reject) => {
      const child = spawn(path.join(context.root, "VUA.exe"), [command, "--root", context.root, ...args, "--headless"], { windowsHide: true, stdio: "pipe" });
      onChild(child); let buffer = ""; let result: Record<string, unknown> | undefined; let error = "distribution_failed";
      child.stdout.setEncoding("utf8");
      child.stdout.on("data", (chunk: string) => {
        buffer += chunk;
        if (buffer.length > 128 * 1024) { child.stdin.write("cancel\n"); error = "invalid_update_progress"; return; }
        for (let end = buffer.indexOf("\n"); end >= 0; end = buffer.indexOf("\n")) {
          const line = buffer.slice(0, end); buffer = buffer.slice(end + 1);
          try {
            const event = JSON.parse(line) as { schemaVersion?: unknown; kind?: unknown; progress?: unknown; value?: Record<string, unknown>; code?: unknown };
            if (event.schemaVersion !== "0.1") continue;
            if (event.kind === "progress" && validProgress(event.progress)) report(event.progress);
            else if (event.kind === "result" && event.value && typeof event.value === "object") result = event.value;
            else if (event.kind === "error" && typeof event.code === "string" && /^[a-z_]{1,80}$/.test(event.code)) error = event.code;
          } catch { error = "invalid_update_progress"; }
        }
      });
      child.stderr.resume(); child.on("error", () => reject(Error("update_helper_unavailable")));
      child.on("close", code => code === 0 && result ? resolve(result) : reject(Error(error)));
    });
  }
  private set(next: Partial<DistributionSnapshotV01>): DistributionSnapshotV01 {
    this.view = { ...this.view, ...next };
    for (const listener of this.listeners) listener(this.view);
    return this.view;
  }
  subscribe(listener: (snapshot: DistributionSnapshotV01) => void): () => void { this.listeners.add(listener); return () => this.listeners.delete(listener); }
  private busy(): boolean { return this.applying || this.view.state === "checking" || this.view.state === "downloading"; }
  private async invoke(command: string, args: readonly string[] = []): Promise<Record<string, unknown>> {
    try { return await this.run(command, args, progress => this.set({ progress }), child => { this.child = child; }); }
    finally { this.child = undefined; }
  }
  async snapshot(): Promise<DistributionSnapshotV01> {
    if (!this.view.available || this.busy()) return this.view;
    try {
      const native = await this.invoke("snapshot");
      if (native.schemaVersion !== "0.1" || typeof native.hostVersion !== "string" || typeof native.amfVersion !== "string" || typeof native.runtimeVersion !== "string") throw Error("invalid_installed_snapshot");
      return this.set({ hostVersion: native.hostVersion, amfVersion: native.amfVersion, runtimeVersion: native.runtimeVersion,
        ...(typeof native.pendingManifestId === "string" ? { pendingManifestId: native.pendingManifestId } : {}),
        ...(typeof native.targetAmfVersion === "string" ? { targetAmfVersion: native.targetAmfVersion } : {}),
        rollbackAvailable: native.rollbackAvailable === true, state: native.pending === "staged" ? "staged" : native.pending === "interrupted" ? "interrupted" : this.view.state });
    } catch { return this.set({ state: "failed", error: "installed_files_need_repair" }); }
  }
  async check(): Promise<DistributionSnapshotV01> {
    if (!this.view.available || this.busy()) return this.view;
    this.set({ state: "checking", error: undefined, progress: undefined, target: undefined });
    try {
      const target = await this.invoke("check"); if (!validTarget(target)) throw Error("invalid_release_manifest");
      this.set({ state: "idle", target }); return this.snapshot();
    } catch { return this.set({ state: "failed", error: "feed_unavailable" }); }
  }
  async stage(id: string): Promise<DistributionSnapshotV01> {
    if (!this.view.available || this.busy()) return this.view;
    if (!/^[a-f0-9]{64}$/.test(id) || (this.view.target && id !== this.view.target.manifestId && id !== this.view.pendingManifestId)) throw Error("invalid_update_identity");
    if (this.view.target?.manifestId === id && !this.view.target.compatible) return this.set({ state: "failed", error: "amf_requires_host_update" });
    this.set({ state: "downloading", error: undefined, progress: undefined });
    try { await this.invoke("stage", ["--manifest", id]); return this.set({ state: "staged" }); }
    catch (error) { return this.set({ state: "interrupted", error: error instanceof Error && /^[a-z_]{1,80}$/.test(error.message) ? error.message : "download_failed" }); }
  }
  async cancel(): Promise<DistributionSnapshotV01> { this.child?.stdin.write("cancel\n"); return this.view; }
  async apply(): Promise<DistributionSnapshotV01> { return this.restart("apply-update"); }
  async restore(): Promise<DistributionSnapshotV01> { return this.restart("rollback-launch"); }
  private async restart(command: "apply-update" | "rollback-launch"): Promise<DistributionSnapshotV01> {
    if (!this.view.available || this.busy() || (command === "apply-update" ? this.view.state !== "staged" : !this.view.rollbackAvailable)) return this.view;
    this.applying = true;
    try {
      if (!(await this.options.prepareRestart())) return this.set({ error: "work_is_running" });
      if (command === "apply-update") await this.invoke("request-activation");
      this.options.restart(command); return this.view;
    } catch { return this.set({ error: "restart_failed" }); }
    finally { this.applying = false; }
  }
}
