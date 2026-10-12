import { installAmfUiHost } from "../../gateway/index.ts";
declare const __VUA_SPLIT_AMF__: boolean;

export async function loadAmfPages() {
  if (!__VUA_SPLIT_AMF__) return import("../amf/AmfPages.tsx");
  installAmfUiHost();
  const css = document.createElement("link");
  css.rel = "stylesheet"; css.href = "vua-module://amf/ui/style.css";
  document.head.append(css);
  const entry = "vua-module://amf/ui/amf-entry.js";
  const module = await import(/* @vite-ignore */ entry);
  if (module.uiAbi !== "amf-ui/0.1" || typeof module.default !== "function") throw new Error("incompatible_amf_ui");
  document.documentElement.dataset.amfUiVersion = module.componentVersion;
  return module as typeof import("../amf/AmfPages.tsx");
}
