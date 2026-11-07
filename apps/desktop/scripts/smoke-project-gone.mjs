// A finished line whose work project was removed is a normal state: the pages say so once, offer a new line
// from the same materials, and leave the Workshop's waiting count alone. Synthetic Gateway.
import { app, BrowserWindow } from "electron";
import { createServer } from "vite";
import assert from "node:assert/strict";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { mkdtemp } from "node:fs/promises";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const profile = await mkdtemp(path.join(os.tmpdir(), "vua-project-gone-"));
app.setPath("userData", profile); app.setPath("sessionData", profile);
app.on("window-all-closed", () => {});
const deadline = setTimeout(() => { console.error("Project gone smoke timed out"); app.exit(1); }, 120_000);
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

async function main() {
  let server, win;
  try {
    await app.whenReady();
    server = await createServer({ root, configFile: path.join(root, "vite.config.mts"), logLevel: "error", server: { host: "127.0.0.1", port: 0 } });
    await server.listen();
    win = new BrowserWindow({ show: false, width: 1280, height: 800, webPreferences: { offscreen: true, backgroundThrottling: false, contextIsolation: true } });
    const page = `http://127.0.0.1:${server.httpServer.address().port}/scripts/fixtures/shell-review.html`;
    const evaluate = source => win.webContents.executeJavaScript(source, true);
    const until = async (source, label) => {
      for (let i = 0; i < 300; i += 1) { if (await evaluate(`!!(${source})`)) return; await pause(100); }
      assert.fail(`Timed out waiting for ${label}`);
    };
    const click = navId => evaluate(`document.querySelector('[data-nav-id="${navId}"]').click()`);
    const waiting = () => evaluate(`document.querySelector('[data-nav-id="nav-workshop"]').textContent`);

    await win.loadURL(`${page}?mode=ask&lang=en#workshop`);
    await until(`document.querySelector('.vua-workshop__line')`, "the lines");
    await pause(800);
    const waitingWithProjects = await waiting();

    await win.loadURL(`${page}?mode=ask&lang=en&gone=1#release`);
    await until(`document.querySelector('[data-nav-id="release-project-gone"]')`, "the finished page to say the project is gone");
    await until(`document.querySelector('.vua-album__still--empty') && !document.querySelector('.vua-album__still--empty').hasAttribute('aria-busy')`, "the picture read to settle");
    assert.equal(await evaluate(`document.querySelector('.vua-album__still--empty').textContent`), "The picture is no longer on this computer");
    assert.equal(await evaluate(`!!document.querySelector('[data-nav-id="release-upload"]')`), false, "An upload needs the project");
    assert.equal(await evaluate(`document.querySelector('[data-nav-id="release-new-line"]').textContent`), "Start a new line with these materials");

    await click("release-new-line");
    await until(`document.querySelector('[data-nav-id="workshop-start-line"]')`, "the new line page");
    await pause(600);
    assert.equal(await evaluate(`document.querySelector('[data-nav-id="workshop-start-line"]').disabled`), false, "The new line starts with the finished line's materials chosen");

    const lines = await evaluate(`document.querySelectorAll('.vua-workshop__line').length`);
    let found = false;
    for (let index = 0; index < lines && !found; index += 1) {
      await evaluate(`document.querySelectorAll('.vua-workshop__line')[${index}].click()`);
      await pause(900);
      found = await evaluate(`!!document.querySelector('[data-nav-id="workshop-project-gone"]')`);
    }
    assert.ok(found, "A finished line says its project is gone");
    assert.equal(await evaluate(`document.querySelector('[data-nav-id="workshop-rebuild"]').textContent`), "Start a new line with these materials");
    await until(`document.querySelector('.vua-n3__views-gone')`, "the pictures to say once that they are gone");
    assert.equal(await evaluate(`document.querySelector('.vua-n3__views-gone').textContent`), "The pictures are no longer on this computer");
    assert.equal(await evaluate(`document.querySelectorAll('.vua-n3__views .vua-n3__preview-frame').length`), 0, "Empty frames go with the message");
    assert.equal(await waiting(), waitingWithProjects, "A removed project adds nothing to the Workshop's waiting count");

    console.log("Project gone smoke passed");
    clearTimeout(deadline); win.destroy(); await server.close(); app.exit(0);
  } catch (error) { clearTimeout(deadline); console.error(error); win?.destroy(); await server?.close(); app.exit(1); }
}
void main();
