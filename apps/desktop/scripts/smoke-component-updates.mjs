// Actual Chromium controls with synthetic update ports; no real update is applied.
import { app, BrowserWindow } from "electron";
import { createServer } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import path from "node:path";
import fs from "node:fs";
import os from "node:os";
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
let server, window;
const output = path.join(root, "out/component-update-review"); fs.mkdirSync(output, { recursive: true });
app.setPath("userData", fs.mkdtempSync(path.join(os.tmpdir(), "vua-update-ui-profile-")));
const deadline = setTimeout(() => { console.error("Component update UI check timed out"); app.exit(1); }, 45_000);
async function main() {
try {
  await app.whenReady();
  server = await createServer({ configFile: false, appType: "custom", root, plugins: [react()],
    resolve: { alias: { "@vua/contracts": path.resolve(root, "../../packages/contracts/src/index.ts") } },
    define: { __VUA_BUILD_INFO__: JSON.stringify({ version: "synthetic-ui", commit: "synthetic", dirty: false }) },
    cacheDir: path.join(output, "vite-cache"), server: { host: "127.0.0.1", port: 0 } });
  server.middlewares.use(async (request, response, next) => {
    if (request.url !== "/__component-update-review") return next();
    response.setHeader("Content-Type", "text/html");
    response.end(await server.transformIndexHtml(request.url, '<html lang="en"><head><script>localStorage.clear();localStorage.setItem("vua-locale","en")</script></head><body><div id="root"></div><script type="module" src="/scripts/fixtures/component-update-review.tsx"></script></body></html>'));
  });
  await server.listen();
  window = new BrowserWindow({ show: false, width: 1360, height: 1120, webPreferences: { offscreen: true, sandbox: true, contextIsolation: true, nodeIntegration: false, backgroundThrottling: false, partition: `update-review-${Date.now()}` } });
  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  const errors = []; window.webContents.on("console-message", event => { if (event.level === "error" && !event.message.includes("404 (Not Found)")) errors.push(event.message); });
  await window.loadURL(`http://127.0.0.1:${server.httpServer.address().port}/__component-update-review`);
  const checks = await window.webContents.executeJavaScript('new Promise((resolve,reject)=>{const start=Date.now();const timer=setInterval(()=>{if(window.componentUpdateReview){clearInterval(timer);window.componentUpdateReview.run().then(resolve,reject);}else if(Date.now()-start>15000){clearInterval(timer);reject(new Error("Fixture timeout"));}},50);})');
  if (errors.length) throw Error(errors.join("\n"));
  await window.webContents.executeJavaScript("window.componentUpdateReview.present()");
  await window.webContents.executeJavaScript('new Promise(resolve=>{const timer=setInterval(()=>{if(document.querySelector("[data-update-plan]")?.textContent.includes("Download both updates")&&document.querySelector("[data-component-update=host]")?.textContent.includes("Update ready")){clearInterval(timer);resolve();}},25);})');
  for (const theme of ["dark", "light"]) {
    await window.webContents.executeJavaScript(`document.documentElement.dataset.theme=${JSON.stringify(theme)}`);
    await window.webContents.executeJavaScript('document.fonts.ready.then(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))))');
    await new Promise(resolve => setTimeout(resolve, 250));
    fs.writeFileSync(path.join(output, `${theme}.png`), (await window.webContents.capturePage()).toPNG());
  }
  const report = { scope: "Actual Chromium controls, synthetic update ports; no real installation, process restart or remote delivery acceptance", checks, passed: checks.length };
  fs.writeFileSync(path.join(output, "report.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report)); window.destroy(); await server.close(); clearTimeout(deadline); app.exit(0);
} catch (error) { console.error(error); window?.destroy(); await server?.close(); clearTimeout(deadline); app.exit(1); }
}
void main();
