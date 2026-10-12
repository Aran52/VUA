import { app, BrowserWindow } from "electron";
import { createServer } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import path from "node:path";
import os from "node:os";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
let server, window;
const timeout = setTimeout(() => { console.error("Library navigation smoke timed out"); app.exit(1); }, 90000);
app.on("window-all-closed", () => {});
async function main() {
  try {
    const output = await mkdtemp(path.join(os.tmpdir(), "vua-library-navigation-"));
    const profile = path.join(output, "profile"); await mkdir(profile); app.setPath("userData", profile); await app.whenReady();
    server = await createServer({ configFile: false, appType: "custom", root, cacheDir: path.join(output, "vite-cache"), plugins: [react()], server: { host: "127.0.0.1", port: 0 }, optimizeDeps: { force: true, include: ["react", "react-dom/client", "@vua/contracts"] } });
    server.middlewares.use(async (req, res, next) => {
      if (!req.url?.startsWith("/__library-navigation")) return next();
      const locale = new URL(req.url, "http://localhost").searchParams.get("locale") ?? "en";
      if (!["en", "zh-CN", "ja", "ko"].includes(locale)) return next();
      res.setHeader("Content-Type", "text/html");
      res.end(await server.transformIndexHtml(req.url, `<html><body data-module="production"><div id="root"></div><script>localStorage.setItem('vua-locale',${JSON.stringify(locale)})</script><script type="module" src="/scripts/fixtures/library-navigation.tsx"></script></body></html>`));
    });
    await server.listen();
    window = new BrowserWindow({ show: false, width: 1200, height: 900, webPreferences: { contextIsolation: true, nodeIntegration: false, sandbox: true, backgroundThrottling: false, offscreen: true } });
    const errors = []; window.webContents.on("console-message", event => { if (event.level === "error") errors.push(event.message); });
    const checks = [];
    const requestedLocale = process.argv.find(arg => arg.startsWith("--locale="))?.slice(9);
    for (const locale of requestedLocale && ["en", "zh-CN", "ja", "ko"].includes(requestedLocale) ? [requestedLocale] : ["en", "zh-CN", "ja", "ko"]) {
      await window.loadURL(`http://127.0.0.1:${server.httpServer.address().port}/__library-navigation?locale=${locale}`);
      await window.webContents.executeJavaScript("new Promise((resolve,reject)=>{let n=0;const t=setInterval(()=>{if(window.libraryNavigationReview){clearInterval(t);resolve()}else if(++n>200){clearInterval(t);reject(Error('Fixture timeout'))}},50)})");
      checks.push(...(await window.webContents.executeJavaScript("window.libraryNavigationReview.run()")).map(check => `${locale}: ${check}`));
      await window.webContents.executeJavaScript("document.documentElement.dataset.effects='off'");
      for (const theme of ["dark", "light"]) {
        await window.webContents.executeJavaScript(`document.documentElement.dataset.theme=${JSON.stringify(theme)}`);
        await window.webContents.executeJavaScript("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))");
        await window.webContents.capturePage({}, { stayHidden: true, stayAwake: true });
        await new Promise(resolve => setTimeout(resolve, 350));
        await writeFile(path.join(output, `${locale}-${theme}.png`), (await window.webContents.capturePage({}, { stayHidden: true, stayAwake: true })).toPNG());
      }
      await window.webContents.executeJavaScript("window.libraryNavigationReview.showPicker()");
      await window.webContents.executeJavaScript("document.documentElement.dataset.theme='dark'");
      await window.webContents.capturePage({}, { stayHidden: true, stayAwake: true }); await new Promise(resolve => setTimeout(resolve, 350));
      await writeFile(path.join(output, `${locale}-picker.png`), (await window.webContents.capturePage({}, { stayHidden: true, stayAwake: true })).toPNG());
    }
    if (errors.length) throw Error(errors.join("\n"));
    await writeFile(path.join(output, "checks.json"), JSON.stringify(checks, null, 2));
    console.log(JSON.stringify({ checks: checks.length, output })); clearTimeout(timeout); await server.close(); window.destroy(); app.exit(0);
  } catch (error) { console.error(error); clearTimeout(timeout); if (server) await server.close(); app.exit(1); }
}
void main();
