import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import type { ComponentUpdateSnapshotV02, ComponentUpdateTargetV02, DesktopComponentUpdatesApiV02, DistributionProgressV01, UpdateComponentV02 } from "@vua/contracts";
import type { InstalledContext } from "./installed-context.js";
import { validProgress } from "./distribution-updater.js";

export type ComponentRun = (command: string, args: readonly string[], report: (progress: DistributionProgressV01) => void, child: (process: ChildProcessWithoutNullStreams) => void) => Promise<Record<string, unknown>>;
const identity = (value: unknown): value is string => typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
const size = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const version = (value: unknown): value is string => typeof value === "string" && value.length <= 160 && /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value);
export function isUpdateComponent(value: unknown): value is UpdateComponentV02 { return value === "host" || value === "amf"; }
export function validTarget(value: unknown, component: UpdateComponentV02): value is ComponentUpdateTargetV02 {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const v = value as Record<string, unknown>;
  return identity(v.manifestId) && version(v.version) && typeof v.build === "string" && /^[a-f0-9]{40}$/.test(v.build)
    && typeof v.releaseUrl === "string" && v.releaseUrl === `https://ph-r.com/vua/components/${component}/v${encodeURIComponent(v.version)}/`
    && (v.activation === "restart" || (component === "amf" && v.activation === "reload"))
    && size(v.downloadBytes) && size(v.reusedBytes) && size(v.runtimeBytes) && (component !== "amf" || v.runtimeBytes === 0);
}
function validSnapshot(value: Record<string, unknown>, component: UpdateComponentV02): value is Record<string, unknown> & ComponentUpdateSnapshotV02 {
  return value.schemaVersion === "0.2" && value.component === component && value.available === true && ["idle", "staged", "interrupted"].includes(String(value.state))
    && version(value.installedVersion) && typeof value.installedBuild === "string" && /^[a-f0-9]{40}$/.test(value.installedBuild)
    && typeof value.rollbackAvailable === "boolean" && (value.latestVersion === undefined || version(value.latestVersion))
    && (value.requiresHost === undefined || (typeof value.requiresHost === "string" && value.requiresHost.length <= 512))
    && (value.target === undefined || validTarget(value.target, component))
    && (value.error === undefined || (typeof value.error === "string" && /^[a-z_]{1,80}$/.test(value.error)));
}
export function componentHelper(context: InstalledContext): string {
  const relative = "launcher/VUA.exe";
  const file = context.host.host.files.find(file => file.path === relative);
  if (!file) throw Error("installer_required");
  const helper = path.join(context.hostRoot, ...relative.split("/"));
  for (const candidate of [context.hostRoot, path.dirname(helper), helper]) {
    if (fs.lstatSync(candidate).isSymbolicLink()) throw Error("update_helper_unavailable");
  }
  const bytes = fs.readFileSync(helper);
  if (bytes.length !== file.bytes || createHash("sha256").update(bytes).digest("hex") !== file.sha256) throw Error("update_helper_unavailable");
  return helper;
}
export function runComponentHelper(context: InstalledContext, command: string, args: readonly string[], report: (progress: DistributionProgressV01) => void, onChild: (child: ChildProcessWithoutNullStreams) => void): Promise<Record<string, unknown>> {
  const executable = componentHelper(context);
  return new Promise((resolve, reject) => {
    const child = spawn(executable, [command, "--root", context.root, ...args, "--headless"], { windowsHide: true, stdio: "pipe" });
    onChild(child); let buffer = ""; let result: Record<string, unknown> | undefined; let error = "update_failed";
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      buffer += chunk;
      if (buffer.length > 128 * 1024) { child.stdin.write("cancel\n"); error = "invalid_update_result"; buffer = ""; return; }
      for (let end = buffer.indexOf("\n"); end >= 0; end = buffer.indexOf("\n")) {
        const line = buffer.slice(0, end); buffer = buffer.slice(end + 1);
        try {
          const event = JSON.parse(line) as Record<string, unknown>;
          if (event.schemaVersion !== "0.1") continue;
          if (event.kind === "progress" && validProgress(event.progress)) report(event.progress);
          else if (event.kind === "result" && event.value && typeof event.value === "object" && !Array.isArray(event.value)) result = event.value as Record<string, unknown>;
          else if (event.kind === "error" && typeof event.code === "string" && /^[a-z_]{1,80}$/.test(event.code)) error = event.code;
        } catch { error = "invalid_update_result"; }
      }
    });
    child.stdin.on("error", () => {});
    child.stderr.resume(); child.on("error", () => reject(Error("update_helper_unavailable")));
    child.on("close", code => code === 0 && result ? resolve(result) : reject(Error(error)));
  });
}
interface Job { child?: ChildProcessWithoutNullStreams; cancelled: boolean }
interface Options {
  readonly context?: InstalledContext | undefined; readonly run?: ComponentRun;
  readonly prepareRestart?: () => Promise<boolean>;
  readonly restart?: (component: UpdateComponentV02, restore: boolean) => void;
  readonly externalBusy?: () => boolean;
  readonly applyAmf?: (restore: boolean, id?: string) => Promise<"updated" | "busy" | "failed" | "restart_required">;
}
/** One job/state per component. Renderer supplies a closed role and checked digest, never URLs. */
export class ComponentUpdater implements DesktopComponentUpdatesApiV02 {
  private readonly views: Record<UpdateComponentV02, ComponentUpdateSnapshotV02>;
  private readonly jobs = new Map<UpdateComponentV02, Job>();
  private readonly listeners = new Set<(view: ComponentUpdateSnapshotV02) => void>();
  private readonly run: ComponentRun;
  private applying = false;
  private amfRestoreRequiresRestart = false;
  get busy(): boolean { return this.jobs.size > 0 || this.applying; }
  constructor(private readonly options: Options) {
    const context = options.context;
    const initial = (component: UpdateComponentV02): ComponentUpdateSnapshotV02 => ({ schemaVersion: "0.2", component,
      available: !!context && (options.run !== undefined || context.host.host.files.some(file => file.path === "launcher/VUA.exe")), state: "idle", rollbackAvailable: false,
      ...(context ? { installedVersion: component === "host" ? context.host.host.version : context.amf.amf.version,
        installedBuild: component === "host" ? context.host.host.build : context.amf.amf.build } : {}) });
    this.views = { host: initial("host"), amf: initial("amf") };
    this.run = options.run ?? ((command, args, report, child) => { if (!context) return Promise.reject(Error("installer_required")); return runComponentHelper(context, command, args, report, child); });
  }
  private set(component: UpdateComponentV02, next: Partial<ComponentUpdateSnapshotV02>): ComponentUpdateSnapshotV02 {
    const view = this.views[component] = { ...this.views[component], ...next };
    for (const listener of this.listeners) listener(view);
    return view;
  }
  subscribe(listener: (view: ComponentUpdateSnapshotV02) => void): () => void { this.listeners.add(listener); return () => this.listeners.delete(listener); }
  private invoke(component: UpdateComponentV02, command: string, args: readonly string[], job: Job): Promise<Record<string, unknown>> {
    return this.run(command, ["--component", component, ...args], ({ completed, total, bytesPerSecond, reusedBytes, fullPack }) => this.set(component, { progress: { completed, total, bytesPerSecond, reusedBytes, fullPack } }), child => { job.child = child; if (job.cancelled) child.stdin.write("cancel\n"); });
  }
  async snapshot(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02> {
    if (!this.views[component].available || this.jobs.has(component) || this.applying) return this.views[component];
    const job: Job = { cancelled: false }; this.jobs.set(component, job);
    try {
      const native = await this.invoke(component, "component-snapshot", [], job);
      if (!validSnapshot(native, component)) throw Error("invalid_update_result");
      return this.set(component, { installedVersion: native.installedVersion, installedBuild: native.installedBuild, rollbackAvailable: native.rollbackAvailable,
        ...(native.state !== "idle" ? { state: native.state, target: native.target } : {}), error: native.error ?? this.views[component].error });
    } catch (error) { return this.set(component, { available: error instanceof Error && error.message === "installer_required" ? false : this.views[component].available, state: "failed", error: error instanceof Error && error.message === "installer_required" ? "installer_required" : "installed_files_need_repair" }); }
    finally { this.jobs.delete(component); }
  }
  async check(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02> {
    if (!this.views[component].available || this.jobs.has(component) || this.applying) return this.views[component];
    const retained = this.views[component];
    const pending = ["staged", "waiting-for-idle", "interrupted"].includes(retained.state) && retained.target;
    const job: Job = { cancelled: false }; this.jobs.set(component, job);
    this.set(component, { state: "checking", target: undefined, latestVersion: undefined, requiresHost: undefined, checkedAt: undefined, error: undefined, progress: undefined });
    try {
      const native = await this.invoke(component, "component-check", [], job);
      if (!validSnapshot(native, component)) throw Error("invalid_update_result");
      if (job.cancelled) return this.set(component, { state: pending ? retained.state : "idle", target: pending ? retained.target : undefined, error: "cancelled" });
      return this.set(component, { ...native, checkedAt: new Date().toISOString(), error: native.error });
    } catch (error) {
      const code = error instanceof Error && /^[a-z_]{1,80}$/.test(error.message) ? error.message : "feed_unavailable";
      return this.set(component, { available: code === "installer_required" ? false : this.views[component].available, state: pending ? retained.state : job.cancelled || code === "cancelled" ? "idle" : "failed", target: pending ? retained.target : undefined, error: job.cancelled ? "cancelled" : code });
    } finally { this.jobs.delete(component); }
  }
  async cancel(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02> { const job = this.jobs.get(component); if (job) { job.cancelled = true; job.child?.stdin.write("cancel\n"); } return this.views[component]; }
  async stage(component: UpdateComponentV02, id: string): Promise<ComponentUpdateSnapshotV02> {
    if (!this.views[component].available || this.jobs.has(component) || this.applying) return this.views[component];
    if (this.options.externalBusy?.()) return this.set(component, { error: "work_is_running" });
    if (!identity(id) || id !== this.views[component].target?.manifestId) throw Error("invalid_update_identity");
    const job: Job = { cancelled: false }; this.jobs.set(component, job);
    this.set(component, { state: "downloading", error: undefined, progress: undefined });
    try {
      const native = await this.invoke(component, "component-stage", ["--manifest", id], job);
      if (!validSnapshot(native, component) || native.state !== "staged" || native.target?.manifestId !== id) throw Error("invalid_update_result");
      return this.set(component, { ...native, state: job.cancelled ? "interrupted" : "staged", error: job.cancelled ? "cancelled" : undefined, progress: undefined });
    } catch (error) {
      return this.set(component, { state: "interrupted", error: job.cancelled ? "cancelled" : error instanceof Error && /^[a-z_]{1,80}$/.test(error.message) ? error.message : "download_failed" });
    } finally { this.jobs.delete(component); }
  }
  async apply(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02> {
    if (component === "amf" && this.views.amf.target?.activation === "reload" && this.options.applyAmf) return this.replaceAmf(false);
    return this.restart(component, false);
  }
  async restore(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02> {
    if (component === "amf" && !this.amfRestoreRequiresRestart && this.options.applyAmf) return this.replaceAmf(true);
    return this.restart(component, true);
  }
  private async replaceAmf(restore: boolean): Promise<ComponentUpdateSnapshotV02> {
    const view = this.views.amf;
    if (!view.available || this.jobs.size || this.applying || (restore ? !view.rollbackAvailable : !["staged", "waiting-for-idle"].includes(view.state))) return view;
    if (this.options.externalBusy?.()) return this.set("amf", { error: "work_is_running" });
    this.applying = true; this.set("amf", { state: "verifying", error: undefined });
    try {
      const outcome = await this.options.applyAmf!(restore, restore ? undefined : view.target?.manifestId);
      if (outcome === "busy") return this.set("amf", { state: restore ? view.state : "waiting-for-idle", error: "work_is_running" });
      if (outcome === "restart_required") {
        if (restore) this.amfRestoreRequiresRestart = true;
        return this.set("amf", { state: view.state, error: restore ? "restore_requires_restart" : "restart_required", target: view.target ? { ...view.target, activation: "restart" } : undefined });
      }
      const native = await this.invoke("amf", "component-snapshot", [], { cancelled: false });
      if (!validSnapshot(native, "amf")) throw Error("invalid_update_result");
      if (outcome === "updated") { this.amfRestoreRequiresRestart = false; return this.set("amf", { ...native, state: "idle", target: undefined, error: undefined, progress: undefined }); }
      return this.set("amf", { ...native, state: "interrupted", target: view.target, error: "amf_reload_failed" });
    } catch { return this.set("amf", { state: "interrupted", error: "amf_reload_failed" }); }
    finally { this.applying = false; }
  }
  private async restart(component: UpdateComponentV02, restore: boolean): Promise<ComponentUpdateSnapshotV02> {
    const view = this.views[component];
    if (!view.available || this.jobs.size || this.applying || (restore ? !view.rollbackAvailable : !["staged", "waiting-for-idle"].includes(view.state))) return view;
    if (this.options.externalBusy?.()) return this.set(component, { error: "work_is_running" });
    if (!this.options.prepareRestart || !this.options.restart) throw Error("update_activation_unavailable");
    if (component === "amf" && !restore && view.target?.activation === "reload") throw Error("update_activation_unavailable");
    this.applying = true;
    try {
      // Request is inert on normal launch. Validate it before stopping any providers.
      if (!restore) await this.invoke(component, "component-request", [], { cancelled: false });
      if (!(await this.options.prepareRestart())) return this.set(component, { state: restore ? view.state : "waiting-for-idle", error: "work_is_running" });
      this.options.restart(component, restore);
      return this.set(component, { state: "activating", error: undefined });
    } catch (error) { return this.set(component, { state: view.state, error: error instanceof Error && /^[a-z_]{1,80}$/.test(error.message) ? error.message : "restart_failed" }); }
    finally { this.applying = false; }
  }
}
