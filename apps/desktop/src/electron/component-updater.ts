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
function validTarget(value: unknown, component: UpdateComponentV02): value is ComponentUpdateTargetV02 {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const v = value as Record<string, unknown>;
  return identity(v.manifestId) && version(v.version) && typeof v.build === "string" && /^[a-f0-9]{40}$/.test(v.build)
    && typeof v.releaseUrl === "string" && v.releaseUrl === `https://ph-r.com/vua/components/${component}/v${encodeURIComponent(v.version)}/`
    && (v.activation === "restart" || (component === "amf" && v.activation === "reload"))
    && size(v.downloadBytes) && size(v.reusedBytes) && size(v.runtimeBytes) && (component !== "amf" || v.runtimeBytes === 0);
}
function validSnapshot(value: Record<string, unknown>, component: UpdateComponentV02): value is Record<string, unknown> & ComponentUpdateSnapshotV02 {
  return value.schemaVersion === "0.2" && value.component === component && value.available === true && value.state === "idle"
    && version(value.installedVersion) && typeof value.installedBuild === "string" && /^[a-f0-9]{40}$/.test(value.installedBuild)
    && typeof value.rollbackAvailable === "boolean" && (value.latestVersion === undefined || version(value.latestVersion))
    && (value.requiresHost === undefined || (typeof value.requiresHost === "string" && value.requiresHost.length <= 512))
    && (value.target === undefined || validTarget(value.target, component));
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
interface Options { readonly context?: InstalledContext | undefined; readonly run?: ComponentRun }
/** One job/state per component. Renderer supplies a closed role and checked digest, never URLs. */
export class ComponentUpdater implements DesktopComponentUpdatesApiV02 {
  private readonly views: Record<UpdateComponentV02, ComponentUpdateSnapshotV02>;
  private readonly jobs = new Map<UpdateComponentV02, Job>();
  private readonly listeners = new Set<(view: ComponentUpdateSnapshotV02) => void>();
  private readonly run: ComponentRun;
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
    return this.run(command, ["--component", component, ...args], progress => this.set(component, { progress }), child => { job.child = child; if (job.cancelled) child.stdin.write("cancel\n"); });
  }
  async snapshot(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02> {
    if (!this.views[component].available || this.jobs.has(component)) return this.views[component];
    const job: Job = { cancelled: false }; this.jobs.set(component, job);
    try {
      const native = await this.invoke(component, "component-snapshot", [], job);
      if (!validSnapshot(native, component)) throw Error("invalid_update_result");
      return this.set(component, { installedVersion: native.installedVersion, installedBuild: native.installedBuild, rollbackAvailable: native.rollbackAvailable });
    } catch { return this.set(component, { state: "failed", error: "installed_files_need_repair" }); }
    finally { this.jobs.delete(component); }
  }
  async check(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02> {
    if (!this.views[component].available || this.jobs.has(component)) return this.views[component];
    const job: Job = { cancelled: false }; this.jobs.set(component, job);
    this.set(component, { state: "checking", target: undefined, latestVersion: undefined, requiresHost: undefined, checkedAt: undefined, error: undefined, progress: undefined });
    try {
      const native = await this.invoke(component, "component-check", [], job);
      if (!validSnapshot(native, component)) throw Error("invalid_update_result");
      if (job.cancelled) return this.set(component, { state: "idle", error: "cancelled" });
      return this.set(component, { ...native, checkedAt: new Date().toISOString(), error: undefined });
    } catch (error) {
      const code = error instanceof Error && /^[a-z_]{1,80}$/.test(error.message) ? error.message : "feed_unavailable";
      return this.set(component, { state: job.cancelled || code === "cancelled" ? "idle" : "failed", error: job.cancelled ? "cancelled" : code });
    } finally { this.jobs.delete(component); }
  }
  async cancel(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02> { const job = this.jobs.get(component); if (job) { job.cancelled = true; job.child?.stdin.write("cancel\n"); } return this.views[component]; }
  // U4/U5 supply activation; discovery never starts an unfinished activation path.
  async stage(_component: UpdateComponentV02, _id: string): Promise<ComponentUpdateSnapshotV02> { throw Error("update_activation_unavailable"); }
  async apply(_component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02> { throw Error("update_activation_unavailable"); }
  async restore(_component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02> { throw Error("update_activation_unavailable"); }
}
