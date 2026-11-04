// Actual Chromium and production adapters; synthetic wire data, isolated profile.
import { app, BrowserWindow } from "electron";
import { createServer } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import path from "node:path";
import os from "node:os";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
let server, window;
const deadline = setTimeout(() => { console.error("material-entry smoke timed out"); app.exit(1); }, 45000);
app.on("window-all-closed", () => {});
async function main() {
try {
  const output = await mkdtemp(path.join(os.tmpdir(), "vua-material-entry-"));
  await mkdir(path.join(output, "profile")); app.setPath("userData", path.join(output, "profile"));
  await app.whenReady();
  server = await createServer({ configFile: false, appType: "custom", root, plugins: [react()],
    server: { host: "127.0.0.1", port: 0 }, optimizeDeps: { include: ["react", "react-dom/client", "@vua/contracts"] } });
  server.middlewares.use(async (req, res, next) => {
    if (req.url !== "/__material-entry") return next();
    res.setHeader("Content-Type", "text/html");
    res.end(await server.transformIndexHtml(req.url, '<html><body><div id="root"></div><script type="module" src="/scripts/fixtures/material-entry.tsx"></script></body></html>'));
  });
  await server.listen();
  window = new BrowserWindow({ show: false, width: 1280, height: 900,
    webPreferences: { contextIsolation: true, nodeIntegration: false, sandbox: true, backgroundThrottling: false } });
  window.webContents.on("console-message", event => { if (event.level === "error") console.error(event.message); });
  await window.loadURL(`http://127.0.0.1:${server.httpServer.address().port}/__material-entry`);
  await window.webContents.executeJavaScript('new Promise((resolve,reject) => { let tries=0; const timer=setInterval(() => { if(window.materialEntryReview){clearInterval(timer);resolve();}else if(++tries>200){clearInterval(timer);reject(Error("fixture timeout"));}},50); })');
  const result = await window.webContents.executeJavaScript(`window.materialEntryReview.${process.argv.includes("--probe") ? "probe" : "run"}()`);
  const evidence = { date: new Date().toISOString(), scope: "Chromium with production adapters and synthetic wire data; no Unity or account execution", result };
  await writeFile(path.join(output, "evidence.json"), JSON.stringify(evidence, null, 2));
  await writeFile(path.join(output, "preview.png"), (await window.webContents.capturePage()).toPNG());
  if (!process.argv.includes("--probe")) {
    await window.webContents.executeJavaScript("window.materialEntryReview.showStart()");
    await writeFile(path.join(output, "confirmation.png"), (await window.webContents.capturePage()).toPNG());
  }
  console.log(JSON.stringify({ ...evidence, output }, null, 2));
  window.destroy(); await server.close(); clearTimeout(deadline); app.exit(0);
} catch (error) { console.error(error); window?.destroy(); await server?.close(); clearTimeout(deadline); app.exit(1); }
}
void main();
