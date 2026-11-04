// Rendered browser/notification and module-shape regression; synthetic ports only.
import { app, BrowserWindow } from "electron";
import { createServer } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { mkdtemp } from "node:fs/promises";
import os from "node:os";
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const profile = await mkdtemp(path.join(os.tmpdir(), "vua-browser-shell-"));
app.setPath("userData", profile);
app.setPath("sessionData", profile);
let server, win;
app.on("window-all-closed", () => {});
async function main() {
  try {
    await app.whenReady();
    server = await createServer({ configFile:false, appType:"custom", root, plugins:[react()], logLevel:"error",
      server:{host:"127.0.0.1",port:0}, optimizeDeps:{include:["react","react-dom/client","@vua/contracts"]} });
    server.middlewares.use(async (req,res,next) => {
      if (req.url !== "/__browser-shell-review") return next();
      res.setHeader("Content-Type","text/html");
      res.end(await server.transformIndexHtml(req.url, '<html data-theme="dark"><body><div id="root"></div><script type="module" src="/scripts/fixtures/browser-shell-review.tsx"></script></body></html>'));
    });
    await server.listen();
    win = new BrowserWindow({show:false,width:1280,height:900,webPreferences:{contextIsolation:true,sandbox:true,offscreen:true,backgroundThrottling:false}});
    await win.loadURL(`http://127.0.0.1:${server.httpServer.address().port}/__browser-shell-review`);
    await win.webContents.executeJavaScript('new Promise((resolve,reject)=>{const deadline=Date.now()+15000;const timer=setInterval(()=>{if(window.browserShellReview){clearInterval(timer);resolve()}else if(Date.now()>deadline){clearInterval(timer);reject(Error("browser shell fixture failed to load"))}},30)})');
    const checks = await win.webContents.executeJavaScript("window.browserShellReview.run()", true);
    for (const check of checks) console.log(`✔ ${check}`);
    win.destroy(); await server.close(); app.exit(0);
  } catch (error) { console.error(error); win?.destroy(); await server?.close(); app.exit(1); }
}
void main();
