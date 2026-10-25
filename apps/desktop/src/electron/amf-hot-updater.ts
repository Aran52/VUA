import { randomBytes } from "node:crypto";
import type { AmfUiGenerationV01, AmfUiRuntimeSnapshotV01, AmfUiTransitionV01, DesktopAmfUiRuntimeApiV01 } from "@vua/contracts";
import type { InstalledContext } from "./installed-context.js";
import type { ModuleProvider } from "./module-provider.js";
import { AmfGenerations, type VerifiedAmfGeneration } from "./amf-generations.js";
import { runComponentHelper, type ComponentRun } from "./component-updater.js";

type Reply = { accepted: boolean; reason?: "busy" | "restart_required" | "failed" };
interface Options { readonly context?: InstalledContext | undefined; readonly generations: AmfGenerations; readonly provider: ModuleProvider; readonly run?: ComponentRun; readonly timeoutMs?: number }
const projection = (value: VerifiedAmfGeneration | undefined): AmfUiGenerationV01 | undefined => value ? { id: value.id, instanceId: value.instanceId, version: value.version, build: value.build, uiAbi: "amf-ui/0.1" } : undefined;
const digest = (value: unknown): value is string => typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
function validIdentity(value: Record<string, unknown>): value is Record<string, unknown> & { manifestId: string; version: string; build: string } {
  return value.schemaVersion === "0.1" && digest(value.manifestId) && typeof value.version === "string" && value.version.length <= 160 && typeof value.build === "string" && /^[a-f0-9]{40}$/.test(value.build);
}
/** Owns a single AMF generation transaction; no path or executable is projected to UI. */
export class AmfHotUpdater implements DesktopAmfUiRuntimeApiV01 {
  private view: AmfUiRuntimeSnapshotV01;
  private readonly run: ComponentRun;
  private readonly listeners = new Set<(view: AmfUiRuntimeSnapshotV01) => void>();
  private pending: { id: string; phase: AmfUiTransitionV01; resolve: (value: Reply) => void } | undefined;
  private applying = false;
  constructor(private readonly options: Options) {
    this.view = { schemaVersion: "0.1", available: !!options.context, phase: "idle", active: projection(options.generations.active) };
    this.run = options.run ?? ((command, args, report, child) => { if (!options.context) return Promise.reject(Error("installer_required")); return runComponentHelper(options.context, command, args, report, child); });
  }
  async snapshot(): Promise<AmfUiRuntimeSnapshotV01> { return this.view; }
  subscribe(listener: (view: AmfUiRuntimeSnapshotV01) => void): () => void { this.listeners.add(listener); return () => this.listeners.delete(listener); }
  private set(next: Partial<AmfUiRuntimeSnapshotV01>): void { this.view = { ...this.view, ...next, active: projection(this.options.generations.active) }; for (const listener of this.listeners) listener(this.view); }
  async respond(id: string, phase: AmfUiTransitionV01, accepted: boolean, reason?: Reply["reason"]): Promise<void> {
    if (!this.pending || this.pending.id !== id || this.pending.phase !== phase) throw Error("invalid_amf_transition");
    this.pending.resolve({ accepted, ...(reason ? { reason } : {}) });
  }
  private invoke(command: string, args: readonly string[] = []): Promise<Record<string, unknown>> { return this.run(command, args, () => {}, () => {}); }
  private request(phase: AmfUiTransitionV01, id: string, candidate: VerifiedAmfGeneration): Promise<Reply> {
    return new Promise(resolve => {
      const timeout = setTimeout(() => finish({ accepted: false, reason: phase === "quiescing" ? "busy" : "failed" }), this.options.timeoutMs ?? 15_000);
      const finish = (reply: Reply) => { clearTimeout(timeout); if (this.pending?.id === id) this.pending = undefined; resolve(reply); };
      this.pending = { id, phase, resolve: finish };
      this.set({ phase, transactionId: id, candidate: projection(candidate), error: undefined });
    });
  }
  async apply(restore: boolean, id?: string): Promise<"updated" | "busy" | "failed" | "restart_required"> {
    if (!this.view.available || this.applying) return "busy";
    if (!digest(this.options.context?.amfManifestId) || !digest(this.options.context?.hostManifestId)) return "restart_required";
    this.applying = true;
    let candidate: VerifiedAmfGeneration | undefined;
    const previous = this.options.generations.active;
    let committed = false;
    try {
      if (!previous) throw Error("amf_generation_unavailable");
      const preflight = await this.invoke("component-amf-preflight", [...(id ? ["--manifest", id] : []), ...(restore ? ["--restore"] : [])]);
      if (!validIdentity(preflight) || (id && preflight.manifestId !== id)) throw Error("amf_generation_identity_mismatch");
      if (preflight.activation === "restart") return "restart_required";
      if (preflight.activation !== "reload") throw Error("amf_generation_identity_mismatch");
      candidate = this.options.generations.load(preflight.manifestId, preflight.version, preflight.build);
      const target = candidate; const transactionId = randomBytes(16).toString("hex");
      const quiesced = await this.request("quiescing", transactionId, target);
      if (!quiesced.accepted) return quiesced.reason === "restart_required" ? "restart_required" : "busy";
      if (!restore) await this.invoke("component-request", ["--component", "amf"]);
      const outcome = await this.options.provider.replaceAmf({
        activate: async () => {
          const selected = await this.invoke("component-amf-begin", ["--manifest", target.id, ...(restore ? ["--restore"] : [])]);
          if (!validIdentity(selected) || selected.manifestId !== target.id) throw Error("amf_generation_identity_mismatch");
          this.options.generations.selectBackend(target);
        },
        verifyUi: async () => (await this.request("loading", transactionId, target)).accepted,
        complete: async () => {
          const result = await this.invoke("component-amf-finish", ["--manifest", target.id]);
          if (result.committed !== true || result.manifestId !== target.id) throw Error("amf_generation_identity_mismatch");
          this.options.generations.commit(target); committed = true;
        },
        restore: async () => {
          const restored = await this.invoke("component-amf-recover");
          if (!validIdentity(restored)) throw Error("amf_generation_identity_mismatch");
          if (restored.manifestId === target.id && restored.healthy === true) { this.options.generations.commit(target); committed = true; }
          else if (restored.manifestId === previous.id) this.options.generations.commit(previous);
          else throw Error("amf_generation_identity_mismatch");
          this.set({ phase: "restoring", candidate: projection(target), error: committed ? undefined : "amf_reload_failed" });
        },
      });
      if (outcome === "updated" || committed) return "updated";
      return outcome;
    } catch { return "failed"; }
    finally {
      if (committed && previous && previous !== this.options.generations.active) this.options.generations.forget(previous);
      if (candidate && candidate !== this.options.generations.active && candidate !== this.options.generations.backend) this.options.generations.forget(candidate);
      this.pending?.resolve({ accepted: false, reason: "failed" });
      this.set({ phase: "idle", transactionId: undefined, candidate: undefined, error: committed ? undefined : this.view.error });
      this.applying = false;
    }
  }
}
