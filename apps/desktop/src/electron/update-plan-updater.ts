import type { DesktopUpdatePlansApiV01, UpdatePlanSnapshotV01 } from "@vua/contracts";
import type { ChildProcessWithoutNullStreams } from "node:child_process";
import type { InstalledContext } from "./installed-context.js";
import { runComponentHelper, validTarget, type ComponentRun } from "./component-updater.js";

interface Options {
  readonly context?: InstalledContext | undefined;
  readonly run?: ComponentRun;
  readonly externalBusy: () => boolean;
  readonly prepareRestart: () => Promise<boolean>;
  readonly restart: (command: "plan-apply" | "plan-restore", id?: string) => void;
}
function validSnapshot(value: Record<string, unknown>): value is Record<string, unknown> & UpdatePlanSnapshotV01 {
  if (value.schemaVersion !== "0.1" || value.available !== true || !["idle", "proposed", "staged", "interrupted"].includes(String(value.state)) || typeof value.rollbackAvailable !== "boolean") return false;
  if (value.plan === undefined) return value.state === "idle";
  const plan = value.plan as Record<string, unknown>;
  if (!plan || typeof plan !== "object" || typeof plan.id !== "string" || !/^[a-f0-9]{64}$/.test(plan.id) || plan.activation !== "restart") return false;
  const installed = plan.installed as Record<string, unknown> | undefined;
  const targets = plan.targets;
  return !!installed && typeof installed.host === "string" && typeof installed.amf === "string"
    && Array.isArray(targets) && targets.length === 2
    && targets[0]?.component === "host" && validTarget(targets[0].target, "host")
    && targets[1]?.component === "amf" && validTarget(targets[1].target, "amf") && targets[1].target.activation === "restart"
    && (value.error === undefined || (typeof value.error === "string" && /^[a-z_]{1,80}$/.test(value.error)));
}
export class UpdatePlanUpdater implements DesktopUpdatePlansApiV01 {
  private view: UpdatePlanSnapshotV01;
  private job: { cancelled: boolean; child?: ChildProcessWithoutNullStreams } | undefined;
  private applying = false;
  private readonly run: ComponentRun;
  private readonly listeners = new Set<(view: UpdatePlanSnapshotV01) => void>();
  get busy(): boolean { return !!this.job || this.applying; }
  constructor(private readonly options: Options) {
    const context = options.context;
    this.view = { schemaVersion: "0.1", available: !!context && (!!options.run || context.host.host.files.some(file => file.path === "launcher/VUA.exe")), state: "idle", rollbackAvailable: false };
    this.run = options.run ?? ((command, args, report, child) => { if (!context) return Promise.reject(Error("installer_required")); return runComponentHelper(context, command, args, report, child); });
  }
  private set(next: Partial<UpdatePlanSnapshotV01>): UpdatePlanSnapshotV01 { this.view = { ...this.view, ...next }; for (const listener of this.listeners) listener(this.view); return this.view; }
  subscribe(listener: (view: UpdatePlanSnapshotV01) => void): () => void { this.listeners.add(listener); return () => this.listeners.delete(listener); }
  private async invoke(command: string, args: readonly string[] = []): Promise<Record<string, unknown>> {
    let component: "host" | "amf" = "host";
    return this.run(command, args, progress => {
      if (progress.phase === "plan-host") component = "host";
      else if (progress.phase === "plan-amf") component = "amf";
      const { completed, total, bytesPerSecond, reusedBytes, fullPack } = progress;
      this.set({ progress: { completed, total, bytesPerSecond, reusedBytes, fullPack, component } });
    }, child => { if (this.job) { this.job.child = child; if (this.job.cancelled) child.stdin.write("cancel\n"); } });
  }
  async snapshot(): Promise<UpdatePlanSnapshotV01> {
    if (!this.view.available || this.busy) return this.view;
    this.job = { cancelled: false };
    try { const native = await this.invoke("plan-snapshot"); if (!validSnapshot(native)) throw Error("invalid_update_plan"); return this.set({ ...native, plan: native.plan, error: native.error, progress: undefined }); }
    catch (error) { return this.failure(error, "failed"); }
    finally { this.job = undefined; }
  }
  async propose(): Promise<UpdatePlanSnapshotV01> {
    if (!this.view.available || this.busy) return this.view;
    if (this.options.externalBusy()) return this.set({ error: "work_is_running" });
    const retained = this.view;
    const pending = ["staged", "waiting-for-idle", "interrupted"].includes(retained.state) && retained.plan;
    this.job = { cancelled: false }; this.set({ state: "checking", plan: undefined, progress: undefined, error: undefined });
    try {
      const native = await this.invoke("plan-propose"); if (!validSnapshot(native) || native.state !== "proposed") throw Error("invalid_update_plan");
      if (this.job.cancelled) return this.set({ state: pending ? retained.state : "idle", plan: pending ? retained.plan : undefined, error: "cancelled" });
      return this.set(native);
    } catch (error) { this.failure(error, "failed"); return pending ? this.set({ state: retained.state, plan: retained.plan }) : this.view; }
    finally { this.job = undefined; }
  }
  private failure(error: unknown, state: "failed" | "interrupted"): UpdatePlanSnapshotV01 {
    return this.set({ available: error instanceof Error && error.message === "installer_required" ? false : this.view.available, state, error: this.job?.cancelled ? "cancelled" : error instanceof Error && /^[a-z_]{1,80}$/.test(error.message) ? error.message : "update_plan_failed" });
  }
  async stage(id: string): Promise<UpdatePlanSnapshotV01> {
    if (!this.view.available || this.busy) return this.view;
    if (this.options.externalBusy()) return this.set({ error: "work_is_running" });
    if (!this.view.plan || id !== this.view.plan.id) throw Error("invalid_update_identity");
    this.job = { cancelled: false }; this.set({ state: "downloading", progress: undefined, error: undefined });
    try {
      const native = await this.invoke("plan-stage", ["--plan", id]);
      if (!validSnapshot(native) || native.plan?.id !== id || native.state !== "staged") throw Error("invalid_update_plan");
      return this.set({ ...native, state: this.job.cancelled ? "interrupted" : "staged", error: this.job.cancelled ? "cancelled" : undefined, progress: undefined });
    } catch (error) { return this.failure(error, "interrupted"); }
    finally { this.job = undefined; }
  }
  async cancel(): Promise<UpdatePlanSnapshotV01> { if (this.job) { this.job.cancelled = true; this.job.child?.stdin.write("cancel\n"); } return this.view; }
  async apply(id: string): Promise<UpdatePlanSnapshotV01> {
    if (!this.view.plan || this.view.plan.id !== id) throw Error("invalid_update_identity");
    return this.restart(id);
  }
  async restore(): Promise<UpdatePlanSnapshotV01> { return this.restart(); }
  private async restart(id?: string): Promise<UpdatePlanSnapshotV01> {
    if (!this.view.available || this.busy || (id ? !["staged", "waiting-for-idle"].includes(this.view.state) : !this.view.rollbackAvailable)) return this.view;
    if (this.options.externalBusy()) return this.set({ error: "work_is_running" });
    this.applying = true;
    try {
      if (id) await this.invoke("plan-request", ["--plan", id]);
      if (!(await this.options.prepareRestart())) return this.set({ state: id ? "waiting-for-idle" : this.view.state, error: "work_is_running" });
      this.options.restart(id ? "plan-apply" : "plan-restore", id); return this.set({ state: "activating", error: undefined });
    } catch (error) { return this.failure(error, "failed"); }
    finally { this.applying = false; }
  }
}
