import fs from "node:fs";
import path from "node:path";
import { createHash, randomBytes } from "node:crypto";
import type { InstalledContext } from "./installed-context.js";

interface VerifiedFile { readonly path: string; readonly bytes: number; readonly sha256: string }
export interface VerifiedAmfGeneration {
  readonly id: string;
  readonly instanceId: string;
  readonly version: string;
  readonly build: string;
  readonly root: string;
  readonly files: readonly VerifiedFile[];
  readonly available: boolean;
}
const digest = (bytes: Buffer) => createHash("sha256").update(bytes).digest("hex");
const identity = (value: unknown): value is string => typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
function owned(root: string, relative: string): string {
  if (!relative || relative.includes("\\") || relative.split("/").some(part => !part || part === "." || part === ".." || /[:\x00-\x1f]/.test(part))) throw Error("unsafe_module_path");
  let target = root;
  if (fs.lstatSync(root).isSymbolicLink()) throw Error("unsafe_module_path");
  for (const part of relative.split("/")) { target = path.join(target, part); if (fs.existsSync(target) && fs.lstatSync(target).isSymbolicLink()) throw Error("unsafe_module_path"); }
  return target;
}
/** Main owns every generation and mount. No renderer-supplied URL or path enters this map. */
export class AmfGenerations {
  readonly #generations = new Map<string, VerifiedAmfGeneration>();
  #active: VerifiedAmfGeneration | undefined;
  #backend: VerifiedAmfGeneration | undefined;
  constructor(private readonly context: InstalledContext | undefined) {
    if (!context) return;
    const id = context.amfManifestId ?? digest(Buffer.from(JSON.stringify(context.amf)));
    this.#active = { id, instanceId: randomBytes(16).toString("hex"), version: context.amf.amf.version, build: context.amf.amf.build,
      root: context.amfRoot, files: context.amf.amf.files, available: context.amfAvailable };
    this.#backend = this.#active; this.remember(this.#active);
  }
  get active(): VerifiedAmfGeneration | undefined { return this.#active; }
  get backend(): VerifiedAmfGeneration | undefined { return this.#backend; }
  private remember(value: VerifiedAmfGeneration): void { this.#generations.set(value.instanceId, value); }
  load(id: string, version: string, build: string): VerifiedAmfGeneration {
    const context = this.context;
    if (!context || !identity(id)) throw Error("invalid_update_identity");
    const file = owned(context.root, `manifests/${id}.json`);
    if (fs.statSync(file).size > 4 * 1024 * 1024) throw Error("invalid_module_inventory");
    const raw = fs.readFileSync(file);
    if (digest(raw) !== id) throw Error("module_inventory_integrity_failed");
    // The trusted native preflight has authenticated this exact digest and facts.
    const envelope = JSON.parse(raw.toString("utf8")) as { payload?: unknown };
    if (typeof envelope.payload !== "string") throw Error("invalid_module_inventory");
    const manifest = JSON.parse(envelope.payload) as { schemaVersion?: unknown; product?: unknown; amf?: { version?: unknown; build?: unknown; pack?: { sha256?: unknown }; files?: VerifiedFile[] } };
    const component = manifest.amf;
    if (manifest.schemaVersion !== "0.1" || manifest.product !== "VUA" || component?.version !== version || component.build !== build
        || !identity(component.pack?.sha256) || !Array.isArray(component.files) || component.files.length > 20000
        || component.files.some(file => !file || typeof file.path !== "string" || !Number.isSafeInteger(file.bytes) || file.bytes < 0 || !identity(file.sha256))) throw Error("invalid_module_inventory");
    const root = owned(context.root, `payloads/amf/${component.pack.sha256}`);
    const generation = { id, instanceId: randomBytes(16).toString("hex"), root, version, build, files: component.files, available: true };
    this.remember(generation); return generation;
  }
  selectBackend(value: VerifiedAmfGeneration): void { this.requireKnown(value); this.#backend = value; }
  commit(value: VerifiedAmfGeneration): void { this.requireKnown(value); this.#active = value; this.#backend = value; }
  forget(value: VerifiedAmfGeneration): void {
    if (value === this.#active || value === this.#backend) return;
    this.#generations.delete(value.instanceId);
  }
  private requireKnown(value: VerifiedAmfGeneration): void { if (this.#generations.get(value.instanceId) !== value) throw Error("unknown_module_generation"); }
  read(requestUrl: string): { readonly bytes: Buffer; readonly type: string } | undefined {
    const url = new URL(requestUrl);
    if (url.hostname !== "amf" || url.username || url.password || url.search || url.hash) return;
    const parts = decodeURIComponent(url.pathname).slice(1).split("/");
    const instanceId = parts.shift(); const id = parts.shift();
    if (!instanceId || !/^[a-f0-9]{32}$/.test(instanceId) || !identity(id)) return;
    const generation = this.#generations.get(instanceId);
    const name = parts.join("/");
    if (!generation?.available || generation.id !== id || !name.startsWith("ui/")) return;
    const file = generation.files.find(file => file.path === name); if (!file) return;
    const target = owned(generation.root, name); const bytes = fs.readFileSync(target);
    if (bytes.length !== file.bytes || digest(bytes) !== file.sha256) throw Error("module_file_integrity_failed");
    const type = name.endsWith(".js") ? "text/javascript" : name.endsWith(".css") ? "text/css" : name.endsWith(".svg") ? "image/svg+xml" : name.endsWith(".png") ? "image/png" : "application/octet-stream";
    return { bytes, type };
  }
  executable(): string | undefined {
    const generation = this.#backend; if (!generation) return;
    const file = generation.files.find(file => file.path === "provider/vua-amf-provider.exe");
    if (!generation.available || !file) throw Error("amf_program_files_need_repair");
    const target = owned(generation.root, file.path); const bytes = fs.readFileSync(target);
    if (bytes.length !== file.bytes || digest(bytes) !== file.sha256) throw Error("amf_program_files_need_repair");
    return target;
  }
}
