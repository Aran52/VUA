import { installAmfUiHost } from "../../gateway/index.ts";
import type { AmfUiGenerationV01 } from "@vua/contracts";
import { disposeAmfGeneration } from "../../app/amf-generation-cleanup.ts";
declare const __VUA_SPLIT_AMF__: boolean;

type Pages = typeof import("../amf/AmfPages.tsx");
interface Entry extends Pages { uiAbi?: string; componentVersion?: string; componentBuild?: string; hotReloadAbi?: string; canUnload?: () => boolean }
interface Loaded { promise: Promise<Entry>; css: HTMLLinkElement; module?: Entry; failed?: boolean }
const entries = new Map<string, Loaded>();
export function splitAmfUi(): boolean { return __VUA_SPLIT_AMF__; }
export function canUnloadAmfUi(generation?: AmfUiGenerationV01): "ready" | "busy" | "restart_required" {
  if (!generation) return "restart_required";
  const entry = entries.get(generation.instanceId);
  if (!entry || entry.failed) return "ready";
  if (!entry.module) return "busy";
  if (entry.module.hotReloadAbi !== "amf-hot/0.1" || typeof entry.module.canUnload !== "function") return "restart_required";
  return entry.module.canUnload() ? "ready" : "busy";
}
export function disposeAmfUi(generation: AmfUiGenerationV01): void {
  const entry = entries.get(generation.instanceId); entries.delete(generation.instanceId);
  entry?.css.remove(); disposeAmfGeneration(generation.instanceId);
}
export async function loadAmfPages(generation?: AmfUiGenerationV01, prepare = false): Promise<Pages> {
  if (!__VUA_SPLIT_AMF__) return import("../amf/AmfPages.tsx");
  installAmfUiHost();
  if (generation && (!/^[a-f0-9]{64}$/.test(generation.id) || !/^[a-f0-9]{32}$/.test(generation.instanceId))) throw Error("invalid_amf_generation");
  const key = generation?.instanceId ?? "legacy";
  const existing = entries.get(key);
  if (existing) { if (!prepare) { existing.css.media = "all"; if (existing.module?.componentVersion) document.documentElement.dataset.amfUiVersion = existing.module.componentVersion; } return existing.promise; }
  const base = generation ? `vua-module://amf/${generation.instanceId}/${generation.id}` : "vua-module://amf";
  const css = document.createElement("link");
  css.rel = "stylesheet"; css.href = `${base}/ui/style.css`; css.media = prepare ? "not all" : "all"; css.dataset.amfGeneration = key;
  const style = new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(() => reject(Error("amf_style_unavailable")), 10_000);
    css.onload = () => { clearTimeout(timeout); resolve(); };
    css.onerror = () => { clearTimeout(timeout); reject(Error("amf_style_unavailable")); };
  });
  document.head.append(css);
  const entryUrl = `${base}/ui/amf-entry.js`;
  const loaded: Loaded = { css, promise: Promise.resolve(undefined as unknown as Entry) };
  loaded.promise = Promise.all([import(/* @vite-ignore */ entryUrl), style]).then(([value]) => {
    const module = value as Entry;
    if (module.uiAbi !== "amf-ui/0.1" || typeof module.default !== "function"
        || (generation && module.componentVersion !== generation.version)
        || (generation && module.componentBuild !== undefined && module.componentBuild !== generation.build)
        || (prepare && (!generation || module.componentBuild !== generation.build || module.hotReloadAbi !== "amf-hot/0.1" || typeof module.canUnload !== "function"))) throw Error("incompatible_amf_ui");
    loaded.module = module;
    if (!prepare) document.documentElement.dataset.amfUiVersion = module.componentVersion;
    return module;
  }).catch(error => { loaded.failed = true; css.remove(); if (generation) disposeAmfGeneration(generation.instanceId); throw error; });
  entries.set(key, loaded);
  return loaded.promise;
}
