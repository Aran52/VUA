// Actual Chromium module graph/cleanup checks with synthetic metadata/code and no providers.
import { app, BrowserWindow, protocol } from "electron";
import { createServer } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import path from "node:path";
import fs from "node:fs";
import os from "node:os";
protocol.registerSchemesAsPrivileged([{ scheme: "vua-module", privileges: { standard: true, secure: true, supportFetchAPI: true, corsEnabled: true } }]);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const output = path.join(root, "out/amf-generation-review"); fs.mkdirSync(output, { recursive: true });
const reportFile = path.join(output, "report.json"); if (fs.existsSync(reportFile)) fs.unlinkSync(reportFile);
app.setPath("userData", fs.mkdtempSync(path.join(os.tmpdir(), "vua-amf-generation-review-")));
let window, server;
const deadline = setTimeout(() => { console.error("AMF generation UI check timed out"); app.exit(1); }, 45_000);
function entry(version, build) {
  return `import {label} from './chunks/helper.js';
const host=globalThis.__VUA_AMF_UI_HOST_V01__;
export const uiAbi='amf-ui/0.1', hotReloadAbi='amf-hot/0.1', componentVersion=${JSON.stringify(version)}, componentBuild=${JSON.stringify(build)};
export const canUnload=()=>!globalThis.__amfFixtureDirty;
globalThis.__amfFixtureEvents??={};
const listener=()=>{globalThis.__amfFixtureEvents[componentVersion]=(globalThis.__amfFixtureEvents[componentVersion]??0)+1;};
window.addEventListener('amf-fixture-event',listener);
host.lifecycle.registerCleanup(new URL(import.meta.url).pathname.split('/')[1],()=>window.removeEventListener('amf-fixture-event',listener));
if(componentVersion==='0.2.1'&&globalThis.__amfFixtureFail)throw new Error('Expected controlled AMF import failure');
export default function Page(){return host.React.createElement('p',{'data-amf-fixture':componentVersion},label+' '+componentVersion);}`;
}
async function main() {
  try {
    await app.whenReady();
    const moduleRequest = request => {
      const url = new URL(request.url); const parts = url.pathname.slice(1).split("/"); const mount = parts.shift(); const id = parts.shift(); const name = parts.join("/");
      const old = mount === "1".repeat(32) && id === "a".repeat(64); const newer = ["2".repeat(32), "3".repeat(32)].includes(mount) && id === "b".repeat(64);
      if (url.hostname !== "amf" || (!old && !newer)) return new Response(null, { status: 404 });
      const version = old ? "0.2.0" : "0.2.1"; let body, type = "text/javascript";
      if (name === "ui/amf-entry.js") body = entry(version, (old ? "a" : "b").repeat(40));
      else if (name === "ui/chunks/helper.js") body = `export const label=${JSON.stringify(old ? "old helper" : "new helper")}`;
      else if (name === "ui/style.css") { type = "text/css"; body = `[data-amf-fixture]{padding:24px;border:1px solid currentColor}`; }
      else return new Response(null, { status: 404 });
      return new Response(body, { headers: { "Content-Type": type, "Access-Control-Allow-Origin": "*", "Cache-Control": "no-store" } });
    };
    server = await createServer({ configFile: false, appType: "custom", root, plugins: [react()], resolve: { alias: { "@vua/contracts": path.resolve(root, "../../packages/contracts/src/index.ts") } },
      define: { __VUA_SPLIT_AMF__: "true", __VUA_BUILD_INFO__: JSON.stringify({ version: "synthetic-ui", commit: "synthetic", dirty: false }) }, cacheDir: path.join(output, "vite-cache"), server: { host: "127.0.0.1", port: 0 } });
    server.middlewares.use(async (request, response, next) => {
      if (request.url !== "/__amf-generation-review") return next(); response.setHeader("Content-Type", "text/html");
      response.end(await server.transformIndexHtml(request.url, '<html lang="en"><head><script>localStorage.clear();localStorage.setItem("vua-locale","en")</script></head><body><div id="root"></div><script type="module" src="/scripts/fixtures/amf-generation-review.tsx"></script></body></html>'));
    });
    await server.listen();
    window = new BrowserWindow({ show: false, width: 1280, height: 900, webPreferences: { offscreen: true, sandbox: true, contextIsolation: true, nodeIntegration: false, backgroundThrottling: false, partition: `amf-generation-${Date.now()}` } });
    window.webContents.session.protocol.handle("vua-module", moduleRequest);
    window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
    window.webContents.on("console-message", event => { if (event.level === "error") console.error(`Renderer: ${event.message}`); });
    await window.loadURL(`http://127.0.0.1:${server.httpServer.address().port}/__amf-generation-review`);
    const checks = await window.webContents.executeJavaScript('new Promise((resolve,reject)=>{const start=Date.now();const timer=setInterval(()=>{if(window.amfGenerationReview){clearInterval(timer);window.amfGenerationReview.run().then(resolve,reject);}else if(Date.now()-start>15000){clearInterval(timer);reject(new Error("Fixture timeout"));}},50);})');
    const report = { scope: "Actual Chromium module imports and cleanup; synthetic metadata/code, no native-provider, user-data or remote-delivery acceptance", passed: checks.length, checks };
    fs.writeFileSync(reportFile, JSON.stringify(report, null, 2)); console.log(JSON.stringify(report));
    clearTimeout(deadline); window.destroy(); await server.close(); app.exit(0);
  } catch (error) { console.error(error); clearTimeout(deadline); window?.destroy(); await server?.close(); app.exit(1); }
}
void main();
