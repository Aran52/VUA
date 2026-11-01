// Actual Chromium geometry and React controls; isolated profile and synthetic ports only.
import { app, BrowserWindow } from "electron";
import { createServer } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import assert from "node:assert/strict";
import { FIXTURES } from "./design-audit/views.mjs";
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const evidence = path.resolve(root, "../../_local_eval/2026-11-01-mcp-theme");
const profile = await mkdtemp(path.join(os.tmpdir(), "vua-mcp-theme-"));
app.setPath("userData", profile); app.setPath("sessionData", profile);
app.commandLine.appendSwitch("force-device-scale-factor", "1");
app.on("window-all-closed", () => {});
const deadline = setTimeout(() => { console.error("MCP/theme UI smoke timed out"); app.exit(1); }, 90_000);
let server, win, family = "grid", theme = "dark";
const checks = [], geometry = [];
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const evaluate = source => win.webContents.executeJavaScript(source, true);
async function until(source) { for (let i = 0; i < 250; i++) { if (await evaluate(source)) return; await pause(40); } throw Error(`UI timeout: ${source}`); }
async function main() { try {
  await app.whenReady(); await mkdir(evidence, { recursive: true });
  server = await createServer({ configFile: false, appType: "custom", root, plugins: [react()], logLevel: "error",
    define: { __VUA_BUILD_INFO__: JSON.stringify({ version: "test", commit: "synthetic", dirty: false, builtAt: "2026-11-01T00:00:00Z" }) },
    server: { host: "127.0.0.1", port: 0 }, optimizeDeps: { include: ["react", "react-dom/client", "@vua/contracts"] } });
  server.middlewares.use(async (req, res, next) => {
    const route = req.url?.split("?")[0];
    if (!["/__themes", "/__mcp"].includes(route)) return next();
    res.setHeader("Content-Type", "text/html");
    const html = route === "/__themes" ? FIXTURES["first-run"].shell({ locale: "en", theme, themeFamily: family })
      : '<html data-theme="dark"><head><style>body{margin:0}#root{padding:24px;max-width:1000px;margin:auto}</style></head><body><div id="root"></div><script type="module" src="/scripts/fixtures/mcp-theme-review.tsx"></script></body></html>';
    res.end(await server.transformIndexHtml(req.url, html));
  });
  await server.listen();
  win = new BrowserWindow({ show: false, width: 1280, height: 1000, webPreferences: { contextIsolation: true, sandbox: true, offscreen: true, backgroundThrottling: false } });
  win.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  const base = server.resolvedUrls.local[0];
  console.log("Checking isolated MCP and tool controls");
  await win.loadURL(`${base}__mcp`); await until("!!window.mcpReview");
  checks.push(...await evaluate("window.mcpReview.run()"));
  for (family of ["grid", "frutiger-aero", "tenshi", "jirai"]) {
    console.log(`Checking Play layout: ${family}`);
    theme = family === "tenshi" ? "light" : "dark";
    win.setContentSize(1280, 1000);
    await win.loadURL(`${base}__themes?themeFamily=${family}`);
    await until("!!window.firstRunReview");
    await evaluate('window.firstRunReview.previewEnvironment("play")');
    await until(`document.documentElement.dataset.themeFamily === ${JSON.stringify(family)}`);
    const entry = await evaluate(`({top:document.querySelector('[data-nav-id="shell-ai"]')?.textContent?.trim()??null,
      menu:[...document.querySelectorAll('.vua-theme-commandbar button')].filter(button=>button.textContent.trim()==='MCP').length})`);
    assert.deepEqual(entry, family === "frutiger-aero" ? { top: null, menu: 1 } : { top: "MCP", menu: 0 }, `${family}: one MCP entry`);
    checks.push(`${family}: a single MCP entry is in the correct bar`);
    for (const width of [1280, 680]) {
      win.setContentSize(width, 1000); await pause(300);
      await evaluate("document.fonts.ready");
      await evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))");
      const cards = await evaluate(`([...document.querySelectorAll('.vua-play-page [data-card-entry] .vua-environment-card')].map(card=>{
        const rect=node=>{const r=node.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,right:r.right,bottom:r.bottom}};
        return {id:card.dataset.card,card:rect(card),details:rect(card.querySelector('.vua-environment-card__details')),
          icon:rect(card.querySelector('.vua-route-tile__icon')),logo:rect(card.querySelector('.vua-route-tile__icon > svg')),
          title:rect(card.querySelector('strong')),action:rect(card.querySelector('.vua-environment-card__action'))};
      }))`);
      geometry.push({ family, width, cards });
      await writeFile(path.join(evidence, "geometry.json"), JSON.stringify(geometry, null, 2));
      await writeFile(path.join(evidence, `play-${family}-${width}.png`), (await win.webContents.capturePage()).toPNG());
      assert.ok(cards.length >= 5);
      const titleInset = cards[0].title.x - cards[0].card.x;
      for (const item of cards) {
        assert.ok(Math.abs(item.title.x - item.card.x - titleInset) < 1, `${family}/${width}/${item.id}: title column`);
        assert.ok(Math.abs(item.icon.y + item.icon.height / 2 - item.title.y - item.title.height / 2) < 1, `${family}/${width}/${item.id}: icon/title center`);
        assert.ok(item.icon.x >= item.details.x && item.icon.right <= item.details.right && item.title.right <= item.details.right, `${family}/${width}/${item.id}: details fit (${JSON.stringify(item)})`);
        assert.ok(item.logo.x >= item.icon.x - 0.5 && item.logo.right <= item.icon.right + 0.5 && item.logo.y >= item.icon.y - 0.5 && item.logo.bottom <= item.icon.bottom + 0.5, `${family}/${width}/${item.id}: logo fits its frame (${JSON.stringify(item)})`);
        if (family === "jirai") assert.ok(item.logo.width <= item.icon.width * 0.65 && item.logo.height <= item.icon.height * 0.65, `${width}/${item.id}: heart has room around logo`);
      }
      checks.push(`${family}/${width}: every Play card aligns its icon and title, with logos contained`);
    }
  }
  await writeFile(path.join(evidence, "ui-result.json"), JSON.stringify({ scope: "Actual Chromium; synthetic data only", checks, geometry }, null, 2));
  console.log(JSON.stringify({ passed: checks.length, checks }));
  clearTimeout(deadline); win.destroy(); await server.close(); app.exit(0);
} catch (error) { clearTimeout(deadline); console.error(error); win?.destroy(); await server?.close(); app.exit(1); } }
void main();
