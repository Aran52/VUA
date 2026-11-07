// The sidebar highlight travels between the items it joins, and a material's details take part in history, so the
// mouse's back and forward buttons leave and re-enter them. Synthetic Gateway.
import { app, BrowserWindow } from "electron";
import { createServer } from "vite";
import assert from "node:assert/strict";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { mkdtemp } from "node:fs/promises";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const profile = await mkdtemp(path.join(os.tmpdir(), "vua-shell-navigation-"));
app.setPath("userData", profile); app.setPath("sessionData", profile);
app.on("window-all-closed", () => {});
const deadline = setTimeout(() => { console.error("Shell navigation smoke timed out"); app.exit(1); }, 150_000);
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

/** Starts a switch, freezes the highlight's trip and reads where it is at each end. */
const trip = nav => `(async () => {
  const box = node => { const rect = node.getBoundingClientRect(); return { left: rect.left, top: rect.top, width: rect.width, height: rect.height }; };
  const from = box(document.querySelector('.vua-shell__sidebar-item[aria-current="page"]'));
  const oldItem = document.querySelector('.vua-shell__sidebar-item[aria-current="page"]');
  document.querySelector('[data-nav-id="${nav}"]').click();
  for (let i = 0; i < 200 && !document.querySelector('.vua-shell__sidebar-glide'); i += 1) await new Promise(resolve => setTimeout(resolve, 5));
  const glide = document.querySelector('.vua-shell__sidebar-glide');
  if (!glide) return { glide: false };
  const animation = glide.getAnimations()[0];
  animation.pause();
  animation.currentTime = 0;
  const start = box(glide);
  const oldBackground = getComputedStyle(oldItem).backgroundColor;
  animation.currentTime = animation.effect.getTiming().duration - 0.01;
  const end = box(glide);
  const to = box(document.querySelector('.vua-shell__sidebar-item[aria-current="page"]'));
  animation.finish();
  return { glide: true, from, to, start, end, oldBackground };
})()`;

async function main() {
  let server, win;
  try {
    await app.whenReady();
    server = await createServer({ root, configFile: path.join(root, "vite.config.mts"), logLevel: "error", server: { host: "127.0.0.1", port: 0 } });
    await server.listen();
    win = new BrowserWindow({ show: false, width: 1280, height: 800, webPreferences: { offscreen: true, backgroundThrottling: false, contextIsolation: true } });
    const evaluate = source => win.webContents.executeJavaScript(source, true);
    const until = async (source, label) => {
      for (let i = 0; i < 300; i += 1) { if (await evaluate(`!!(${source})`)) return; await pause(100); }
      assert.fail(`Timed out waiting for ${label}`);
    };
    await win.loadURL(`http://127.0.0.1:${server.httpServer.address().port}/scripts/fixtures/shell-review.html?mode=ask&lang=en#warehouse`);
    await until(`document.querySelector('.vua-warehouse-card')`, "the wall");

    // Inside one group and across the two.
    for (const nav of ["nav-release", "nav-env-play", "nav-warehouse"]) {
      const result = await evaluate(trip(nav));
      assert.equal(result.glide, true, `${nav}: the highlight travels`);
      for (const key of ["left", "top", "width", "height"]) {
        assert.ok(Math.abs(result.start[key] - result.from[key]) <= 1, `${nav}: the highlight sets off from the item left behind (${key} ${result.start[key]} vs ${result.from[key]})`);
        assert.ok(Math.abs(result.end[key] - result.to[key]) <= 1, `${nav}: the highlight lands on the chosen item (${key} ${result.end[key]} vs ${result.to[key]})`);
      }
      assert.equal(result.oldBackground, "rgba(0, 0, 0, 0)", `${nav}: the item left behind shows no second highlight while it travels`);
      await pause(800);
    }

    await until(`document.querySelector('.vua-warehouse-card')`, "the wall again");
    const detail = `!!document.querySelector('.vua-warehouse-detail')`;
    const title = `document.querySelector('.vua-warehouse-detail__title')?.textContent`;
    const hash = () => evaluate("location.hash");
    await evaluate(`document.querySelector('.vua-warehouse-card').dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))`);
    await until(detail, "the details");
    await pause(600);
    const firstTitle = await evaluate(title);
    assert.equal(await hash(), "#warehouse");

    await evaluate("history.back()");
    await until(`!${detail}`, "Back to return to the wall");
    assert.equal(await hash(), "#warehouse", "Back stays on Materials");
    await evaluate("history.forward()");
    await until(detail, "Forward to reopen the details");
    await pause(600);
    assert.equal(await evaluate(title), firstTitle, "Forward reopens the same material");

    await evaluate(`window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }))`);
    await pause(900);
    const stepped = await evaluate(title);
    assert.notEqual(stepped, firstTitle, "Arrow keys step to the neighbour");
    await evaluate("history.back()");
    await until(`!${detail}`, "Back to skip stepped neighbours and return to the wall");
    await evaluate("history.forward()");
    await until(detail, "Forward to reopen after stepping");
    await pause(600);
    assert.equal(await evaluate(title), stepped, "Forward reopens the material last viewed");

    await evaluate(`document.querySelector('[data-nav-id="warehouse-detail-back"]').click()`);
    await until(`!${detail}`, "the page's back button");
    await pause(200);
    await evaluate("history.forward()");
    await until(detail, "Forward after the page's own back button");
    await pause(400);

    await evaluate(`document.querySelector('[data-nav-id="nav-workshop"]').click()`);
    await until(`document.querySelector('.vua-workshop__line')`, "the Workshop");
    assert.equal(await hash(), "#workshop");
    await evaluate("history.back()");
    await until(detail, "Back from another page to return to the details");
    assert.equal(await hash(), "#warehouse");
    await evaluate("history.back()");
    await until(`!${detail} && document.querySelector('.vua-warehouse-card')`, "one more Back to reach the wall");
    assert.equal(await hash(), "#warehouse");

    console.log("Shell navigation smoke passed");
    clearTimeout(deadline); win.destroy(); await server.close(); app.exit(0);
  } catch (error) { clearTimeout(deadline); console.error(error); win?.destroy(); await server?.close(); app.exit(1); }
}
void main();
