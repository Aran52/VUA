import { build, mergeConfig } from "vite";
import react from "@vitejs/plugin-react";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import hostConfig from "../vite.config.mts";
const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const renderer = path.join(desktop, "src/renderer");
const require = createRequire(path.join(desktop, "package.json"));
const amfVersion = JSON.parse(readFileSync(path.join(desktop, "distribution/amf-version.json"))).version;
const bindings = {
  "react": ["React", Object.keys(require("react"))],
  "react-dom": ["ReactDOM", Object.keys(require("react-dom"))],
  "react/jsx-runtime": ["JSX", Object.keys(require("react/jsx-runtime"))],
  "gateway/GatewayProvider.tsx": ["gateway"],
  "gateway/task-identity.ts": ["tasks"],
  "components/primitives/modal-layer.tsx": ["modals"],
  "app/login-browser-store.ts": ["browser"],
};
for (const [source, binding] of Object.entries(bindings)) {
  binding[1] ??= [...readFileSync(path.join(renderer, source), "utf8").matchAll(/export (?:function|const|class) (\w+)/g)].map(match => match[1]);
}
const sharedBindings = {
  name: "vua-first-party-amf-bindings",
  enforce: "pre",
  async resolveId(source, importer) {
    if (bindings[source]) return `\0amf-host:${source}`;
    if (source.startsWith(".")) {
      const resolved = await this.resolve(source, importer, { skipSelf: true });
      const relative = resolved && path.relative(renderer, resolved.id).replaceAll("\\", "/");
      if (relative && bindings[relative]) return `\0amf-host:${relative}`;
    }
  },
  load(id) {
    if (!id.startsWith("\0amf-host:")) return;
    const [name, names] = bindings[id.slice(10)];
    return `const host = globalThis.__VUA_AMF_UI_HOST_V01__; if (host?.abi !== "amf-ui/0.1") throw new Error("amf_host_binding_unavailable"); const value = host.${name};\n${names.map(key => `export const ${key} = value.${key};`).join("\n")}\nexport default value;`;
  },
};
await build(mergeConfig(hostConfig, { root: desktop, configFile: false,
  plugins: [{ name: "vua-module-csp", transformIndexHtml: html => html.replace("script-src 'self'", "script-src 'self' vua-module:").replace("style-src 'self'", "style-src 'self' vua-module:").replace("connect-src 'self'", "connect-src 'self' vua-module:") }],
  define: { __VUA_SPLIT_AMF__: "true" }, build: { outDir: "dist/components/host-renderer" } }));
await build({ root: desktop, configFile: false, base: "./", plugins: [sharedBindings, react()],
  define: { ...hostConfig.define, __VUA_AMF_VERSION__: JSON.stringify(amfVersion), "process.env.NODE_ENV": JSON.stringify("production") }, build: { outDir: "dist/components/amf-ui", emptyOutDir: true,
    lib: { entry: path.join(renderer, "features/amf/amf-entry.ts"), formats: ["es"], fileName: () => "amf-entry.js", cssFileName: "style" },
    rolldownOptions: { output: { codeSplitting: false } }, sourcemap: false } });
