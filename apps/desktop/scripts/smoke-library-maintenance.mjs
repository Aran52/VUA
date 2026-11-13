// Real Chromium DOM, isolated temporary profile, synthetic Gateway only.
import { app, BrowserWindow } from "electron";
import { createServer } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import os from "node:os";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
let server, window;
// Keep cleanup asynchronous without Electron quitting successfully on the last window.
app.on("window-all-closed", () => {});
async function main() {
try {
  const output = await mkdtemp(path.join(os.tmpdir(), "vua-n5-library-ui-"));
  const profile = path.join(output, "isolated-profile");
  await mkdir(profile);
  app.setPath("userData", profile);
  await app.whenReady();
  server = await createServer({ configFile: false, appType: "custom", root, plugins: [react()],
    server: { host: "127.0.0.1", port: 0, strictPort: false },
    // Scan this fixture's graph, not every unrelated HTML fixture in the checkout.
    optimizeDeps: { entries: ["scripts/fixtures/library-maintenance.tsx"], force: true, include: ["react", "react-dom/client", "@vua/contracts"] } });
  server.middlewares.use(async (req, res, next) => {
    if (req.url !== "/__library-maintenance") return next();
    res.setHeader("Content-Type", "text/html");
    res.end(await server.transformIndexHtml(req.url, '<html><body><div id="root"></div><script type="module" src="/scripts/fixtures/library-maintenance.tsx"></script></body></html>'));
  });
  await server.listen();
  window = new BrowserWindow({ show: false, width: 1280, height: 900,
    webPreferences: { contextIsolation: true, nodeIntegration: false, sandbox: true, backgroundThrottling: false } });
  window.webContents.on("console-message", (event) => { if (event.level === "error") console.error(event.message); });
  window.webContents.on("render-process-gone", (_event, details) => console.error("Library fixture renderer exited", details.reason, details.exitCode));
  window.webContents.on("did-fail-load", (_event, code, description) => console.error("Library fixture load failed", code, description));
  await window.loadURL(`http://127.0.0.1:${server.httpServer.address().port}/__library-maintenance`);
  await window.webContents.executeJavaScript('new Promise((resolve, reject) => { let attempts = 0; const timer = setInterval(() => { if(window.n5MaintenanceReview) { clearInterval(timer); resolve(); } else if (++attempts > 200) { clearInterval(timer); reject(new Error("fixture load timeout")); } }, 50); })');
  const checks = await window.webContents.executeJavaScript("window.n5MaintenanceReview.run()");
  await window.webContents.executeJavaScript('new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))');
  await writeFile(path.join(output, "preview.png"), (await window.webContents.capturePage()).toPNG());
  window.webContents.sendInputEvent({ type: "keyDown", keyCode: "Escape" });
  window.webContents.sendInputEvent({ type: "keyUp", keyCode: "Escape" });
  await new Promise((resolve) => setTimeout(resolve, 60));
  checks.splice(0, checks.length, ...await window.webContents.executeJavaScript("window.n5MaintenanceReview.checkNativeClose()"));
  const evidence = { date: new Date().toISOString(), browser: process.versions.chrome,
    scope: "Actual Chromium DOM with synthetic Gateway; no real materials, remote/account access or human UI acceptance", checks, passed: checks.length };
  await writeFile(path.join(output, "checks.json"), JSON.stringify(evidence, null, 2));
  console.log(JSON.stringify({ passed: checks.length, scope: evidence.scope, evidence: output }, null, 2));
  window.destroy(); await server.close(); app.exit(0);
} catch (error) {
  console.error(error);
  if (window && !window.isDestroyed()) {
    const state = await window.webContents.executeJavaScript('({path:location.pathname,readyState:document.readyState,fixtureReady:!!window.n5MaintenanceReview,rootChildren:document.getElementById("root")?.childElementCount,scripts:[...document.scripts].map(s=>new URL(s.src,location.href).pathname)})').catch(() => null);
    console.error(JSON.stringify({ fixtureState: state }));
  }
  window?.destroy(); await server?.close(); app.exit(1);
}
}
void main();
