// Page switches stay quick when the AMF module is slow. The module answers one request at a time and its
// production list takes seconds, so a page that waits behind those reads stalls. Synthetic replies with
// real-world delays, in one lane, show that stall where instant fixtures hide it.
import { app, BrowserWindow } from "electron";
import { createServer } from "vite";
import assert from "node:assert/strict";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { mkdtemp } from "node:fs/promises";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const profile = await mkdtemp(path.join(os.tmpdir(), "vua-page-speed-"));
app.setPath("userData", profile); app.setPath("sessionData", profile);
app.commandLine.appendSwitch("force-device-scale-factor", "1");
app.on("window-all-closed", () => {});
const deadline = setTimeout(() => { console.error("Page speed smoke timed out"); app.exit(1); }, 150_000);
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

/** The longest a switch may take. A stall behind the production list is at least 2500 ms. */
const BUDGET_MS = 1500;

const timed = (nav, ready) => `(async () => {
  const start = performance.now();
  document.querySelector('[data-nav-id="${nav}"]').click();
  while (performance.now() - start < 30000) { if (${ready}) return Math.round(performance.now() - start); await new Promise(resolve => setTimeout(resolve, 10)); }
  return -1;
})()`;
const cards = "document.querySelector('.vua-warehouse-card__title')?.innerText";
const lines = "document.querySelector('.vua-workshop__line')";
const recipes = "document.querySelector('.vua-recipe-card')";

async function main() {
  let server, win;
  try {
    await app.whenReady();
    server = await createServer({ root, configFile: path.join(root, "vite.config.mts"), logLevel: "error", server: { host: "127.0.0.1", port: 0 } });
    await server.listen();
    win = new BrowserWindow({ show: false, width: 1280, height: 800, webPreferences: { offscreen: true, backgroundThrottling: false, contextIsolation: true } });
    const url = `http://127.0.0.1:${server.httpServer.address().port}/scripts/fixtures/shell-review.html?mode=ask&lang=en&slow=1`;
    const evaluate = source => win.webContents.executeJavaScript(source, true);
    const open = async () => {
      await win.loadURL(`${url}#home`);
      for (let i = 0; i < 300 && !(await evaluate(`!!document.querySelector('[data-nav-id="nav-warehouse"]')`)); i += 1) await pause(100);
    };
    // One tour compiles every page, so the timed tour measures the app rather than the dev server.
    await open();
    for (const [nav, ready] of [["nav-warehouse", cards], ["nav-workshop", lines], ["nav-recipe", recipes]]) await evaluate(timed(nav, ready));
    await open();
    await pause(6000);
    const results = {};
    results["Home to Materials"] = await evaluate(timed("nav-warehouse", cards));
    await pause(300);
    results["Materials to Workshop"] = await evaluate(timed("nav-workshop", lines));
    await pause(300);
    results["Workshop to Materials"] = await evaluate(timed("nav-warehouse", cards));
    await pause(300);
    results["Materials to Recipes"] = await evaluate(timed("nav-recipe", recipes));
    console.log(JSON.stringify(results));
    for (const [name, ms] of Object.entries(results)) assert.ok(ms >= 0 && ms <= BUDGET_MS, `${name} took ${ms} ms, over ${BUDGET_MS} ms: a page is waiting behind a slow read`);
    clearTimeout(deadline); win.destroy(); await server.close(); app.exit(0);
  } catch (error) { clearTimeout(deadline); console.error(error); win?.destroy(); await server?.close(); app.exit(1); }
}
void main();
