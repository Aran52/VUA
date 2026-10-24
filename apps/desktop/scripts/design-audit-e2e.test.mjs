// End-to-end test for the design-audit collector and rules: synthetic HTML
// pages rendered in an offscreen Electron window, collected in-page, evaluated
// node-side. Covers every rule's trigger and its documented exceptions,
// including [data-audit-skip]. Run: electron scripts/design-audit-e2e.test.mjs

import { app, BrowserWindow } from "electron";
import { createServer } from "node:http";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { collectAuditSnapshot } from "./design-audit/collector.js";
import { applyAnnotations, removeAnnotations } from "./design-audit/annotate.js";
import { evaluateSnapshot, classifyCapture } from "./design-audit/rules.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));

const PAGE = `<!doctype html><html data-theme="dark"><head><style>
:root {
  --vua-text-strong: #f5f3ff; --vua-text-secondary: #b3b0c8;
  --vua-bg-canvas: #0b0a12; --vua-bg-panel: #151322;
  --vua-border: #2e2a45; --vua-accent: #a78bfa;
  --vua-success: #4ade80; --vua-warning: #fde047; --vua-error: #fb7185;
  --vua-radius-control: 6px; --vua-radius-card: 10px; --vua-radius-overlay: 12px;
  --vua-glow-accent: 0 0 18px rgba(167,139,250,0.45);
}
body { margin: 0; background: var(--vua-bg-canvas); color: var(--vua-text-strong); font: 400 14px/20px sans-serif; }
.ok { font-size: 14px; line-height: 20px; padding: 16px; border-radius: 10px; background: #151322; color: #f5f3ff; }
.offscale { font-size: 13px; line-height: 18px; }
.oddweight { font-weight: 500; }
.hardcolor { color: #ff4400; background-color: #123456; }
.lowcontrast { color: #55555f; background-color: #151322; padding: 8px; }
.offradius { border-radius: 8px; }
.shadowed { box-shadow: 0 4px 12px rgba(0,0,0,0.4); }
.glowy { box-shadow: 0 0 18px rgba(167,139,250,0.45); }
.blurcard { backdrop-filter: blur(10px); background: rgba(21,19,34,0.8); }
.pill-in-card { border-radius: 999px; }
.circle { border-radius: 50%; width: 32px; height: 32px; }
.stepdown { border-radius: 6px; }
.clipped { overflow: hidden; width: 80px; white-space: nowrap; }
.ellipsis { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; width: 60px; }
.tiny { width: 16px; height: 16px; padding: 0; }
</style></head><body>
<div id="ok" class="ok">token-clean card</div>
<div id="bad" class="offscale oddweight" style="padding: 14px;">off scale</div>
<div id="hard" class="hardcolor">hardcoded</div>
<div id="low" class="lowcontrast">dim text</div>
<div id="rad" class="offradius">radius</div>
<div id="shadow" class="shadowed">shadow panel</div>
<div id="glow" class="glowy">accent glow</div>
<div id="blur" class="blurcard">blur card wall</div>
<div style="border-radius: 10px; padding: 4px;">
  <div id="nested" class="same-radius" style="border-radius: 10px;">same radius as parent</div>
  <div id="larger" class="larger-radius" style="border-radius: 12px;">larger radius than parent</div>
  <div id="pill" class="pill-in-card">pill</div>
  <div id="circle" class="circle"></div>
  <div id="step" class="stepdown">steps down</div>
</div>
<div id="clip" class="clipped">this long text is clipped by the container</div>
<div id="ell" class="ellipsis">this long text ends with an intentional ellipsis</div>
<button id="tinybtn" class="tiny" style="border-radius:0">x</button>
<div style="border: 1px solid #2e2a45; background: #151322; padding: 8px;">
  <div style="border: 1px solid #2e2a45; background: #1e1b30; padding: 8px;">
    <div id="deepsurface" class="deepsurface" style="border: 1px solid #2e2a45; background: #24203a; padding: 8px;">deep surface
      <span id="deeptext">plain text inside</span>
      <svg id="deepsvg" width="16" height="16"><rect width="16" height="16"/></svg>
    </div>
  </div>
</div>
<div data-audit-skip="art">
  <div id="skipped" class="offscale hardcolor" style="border-radius: 3px; box-shadow: 0 2px 6px #000;">art layer</div>
</div>
</body></html>`;

const OVERLAY_PAGE = `<!doctype html><html><body>
<div id="root"></div>
<vite-error-overlay><pre>Failed to resolve import "@vua/contracts"</pre></vite-error-overlay>
</body></html>`;

const CALIBRATION_PAGE = `<!doctype html><html><head><style>
body { margin:0; color:white; background:black; font:400 14px/20px sans-serif; }
.camera { position:relative; width:200px; height:80px; overflow:hidden; }
.scene { width:1200px; height:60px; transform:translateX(-300px); background:gray; }
.hidden { opacity:0; }
.bleed { position:absolute; top:100px; left:740px; width:200px; height:40px; }
.caption-clip { width:100px; overflow:hidden; }
.caption { display:block; width:300px; white-space:nowrap; }
.label { display:inline-flex; align-items:center; min-height:28px; padding:0 8px; gap:8px; }
input { width:16px; height:16px; padding:0; margin:0; }
.auto { width:101px; margin-left:auto; }
.fixed { margin-inline:6px; }
.separator { border-bottom:1px solid white; }
</style></head><body>
<div class="camera"><div id="scene" class="scene"></div><div class="hidden"><button id="hidden-control">hidden</button></div></div>
<div id="bleed" class="bleed">actual viewport overflow</div>
<div class="caption-clip"><span id="caption" class="caption">actual clipped caption</span></div>
<label id="large-label" class="label"><input id="labelled" type="checkbox">a usable label</label>
<input id="unlabelled" type="checkbox" aria-label="tiny independent control">
<div inert><button id="inert-control" style="width:16px;height:16px;padding:0">inactive background action</button></div>
<div role="dialog"><button id="scoped-control">dialog action</button></div>
<div id="auto" class="auto">auto alignment</div><div id="fixed" class="fixed">fixed margins</div>
<div class="separator"><div class="separator"><button id="choice">ordinary choice</button></div></div>
</body></html>`;

const tests = [];
const test = (name, fn) => tests.push({ name, fn });
let server; // assigned in main; shared with the overlay test

test("collector and rules run end-to-end on a synthetic page", async win => {
  const snapshot = await win.webContents.executeJavaScript(
    `${collectAuditSnapshot.toString()};collectAuditSnapshot({ proposed: true, overflowChecks: true })`, true);
  assert.ok(snapshot.elementCount > 15, "elements collected");
  assert.ok(snapshot.palette.length >= 8, "token palette resolved");
  const findings = evaluateSnapshot(snapshot);

  const byRule = Object.groupBy(findings, f => f.rule);
  const has = (rule, id) => (byRule[rule] ?? []).some(f => (f.cls ?? []).includes(id));

  // token-clean element is silent
  assert.ok(!has("type-scale", "ok") && !has("spacing-grid", "ok") && !has("radius-token", "ok"), "clean element passes");
  // triggered rules
  assert.ok(has("type-scale", "offscale"), "type-scale flags 13/18");
  assert.ok(has("type-weight", "offscale"), "type-weight flags 500");
  assert.ok(has("spacing-grid", "offscale"), "spacing-grid flags 14px padding");
  assert.ok((byRule["color-token"] ?? []).some(f => f.cls.includes("hardcolor")), "color-token flags #ff4400/#123456");
  assert.ok(has("contrast", "lowcontrast"), "contrast flags dim text");
  assert.ok(has("radius-token", "offradius"), "radius-token flags 8px");
  assert.ok(has("shadow-panel", "shadowed"), "shadow-panel flags ordinary panel");
  assert.ok(!has("shadow-panel", "glow"), "accent glow is exempt");
  assert.ok(has("blur-scope", "blurcard"), "blur-scope flags card-wall blur");
  assert.ok(!has("radius-nesting", "same-radius"), "equal inner corners follow the approved standard");
  assert.ok(has("radius-nesting", "larger-radius"), "larger inner corners are flagged");
  assert.ok(!has("radius-nesting", "pill") && !has("radius-nesting", "circle"), "pill/circle exempt from radius-nesting");
  assert.ok(!has("radius-nesting", "step"), "stepped-down radius passes");
  assert.ok(has("overflow", "clipped"), "overflow flags hidden clipping");
  assert.ok(!has("overflow", "ell"), "ellipsis clipping is exempt");
  assert.ok(has("target-size", "tiny"), "target-size flags 16px button");
  // surface-nesting: the deep surface itself is flagged once; text and svg
  // inside it are never flagged.
  const deepFindings = (byRule["surface-nesting"] ?? []).filter(f => (f.cls ?? []).includes("deepsurface"));
  assert.equal(deepFindings.length, 1, "surface-nesting flags the deep surface once");
  assert.equal(deepFindings[0].details.surfaceDepth, 3, "depth counts self + two ancestors");
  assert.equal((byRule["surface-nesting"] ?? []).length, 1, "text and svg inside the surface stay silent");
  // data-audit-skip subtree is invisible to every rule
  for (const rule of Object.keys(byRule)) {
    assert.ok(!(byRule[rule] ?? []).some(f => (f.cls ?? []).includes("skipped")), `skip region silent for ${rule}`);
  }
});

test("annotation overlay draws and removes cleanly", async win => {
  const snapshot = await win.webContents.executeJavaScript(
    `${collectAuditSnapshot.toString()};collectAuditSnapshot({ proposed: true, overflowChecks: true })`, true);
  const findings = evaluateSnapshot(snapshot);
  const items = findings.map(f => ({
    rule: f.rule, severity: f.severity, message: f.message, cls: f.cls, tag: f.tag,
    rect: f.index != null ? snapshot.elements[f.index].rect : null,
  })).filter(i => i.rect);
  const drawn = await win.webContents.executeJavaScript(
    `${applyAnnotations.toString()};applyAnnotations(${JSON.stringify(items)}, { view: "e2e", theme: "dark", locale: "zh-CN" })`, true);
  assert.equal(drawn, items.length);
  assert.equal(await win.webContents.executeJavaScript("document.querySelectorAll('[data-vua-audit-overlay]').length"), 1);
  win.webContents.invalidate();
  await win.webContents.executeJavaScript("new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))", true);
  const pixels = (await win.webContents.capturePage({}, { stayHidden: true, stayAwake: true })).toBitmap();
  let legendPixels = 0;
  for (let offset = 0; offset < pixels.length; offset += 4) {
    if (pixels[offset] >= 40 && pixels[offset] <= 50 && pixels[offset + 1] >= 40 && pixels[offset + 1] <= 50 && pixels[offset + 2] >= 250) legendPixels += 1;
  }
  assert.ok(legendPixels >= 20, "the captured image contains the red annotation legend, not only a DOM overlay");
  await win.webContents.executeJavaScript(`${removeAnnotations.toString()};removeAnnotations()`, true);
  assert.equal(await win.webContents.executeJavaScript("document.querySelectorAll('[data-vua-audit-overlay]').length"), 0);
});

test("paint clipping, label hit regions and auto margins preserve real defects", async win => {
  await win.loadURL(`http://127.0.0.1:${server.address().port}/calibration`);
  const snapshot = await win.webContents.executeJavaScript(
    `${collectAuditSnapshot.toString()};collectAuditSnapshot({ proposed: true, overflowChecks: true })`, true);
  const element = id => snapshot.elements.find(value => value.id === id);
  const findings = evaluateSnapshot(snapshot);
  const flagged = (rule, id) => findings.some(f => f.rule === rule && f.index === snapshot.elements.indexOf(element(id)));
  assert.equal(element("hidden-control"), undefined, "opacity-zero ancestors are not painted controls");
  assert.equal(flagged("overflow", "scene"), false, "a camera clipped inside the page is not viewport leakage");
  assert.equal(flagged("overflow", "bleed"), true, "unconfined content still fails");
  assert.equal(flagged("overflow", "caption"), true, "ordinary clipped text is not exempted with the scene");
  assert.equal(flagged("target-size", "labelled"), false, "the associated 28px label supplies the real hit region");
  assert.equal(flagged("target-size", "unlabelled"), true, "an independent tiny checkbox still fails");
  assert.equal(element("inert-control").disabled, true, "modal isolation makes background actions inactive");
  assert.equal(flagged("target-size", "inert-control"), false);
  assert.notEqual(element("scoped-control").actionScope, element("inert-control").actionScope, "dialog actions have their own primary-action scope");
  assert.equal(element("labelled").hitRect.source, "label");
  assert.equal(await win.webContents.executeJavaScript("document.getElementById('large-label').click(); document.getElementById('labelled').checked"), true);
  assert.equal(element("auto").autoMargins[3], true);
  assert.equal(flagged("spacing-grid", "auto"), false);
  assert.equal(flagged("spacing-grid", "fixed"), true);
  assert.equal(element("choice").selfSurface, false, "an action is not another nested container");
  await win.loadURL(`http://127.0.0.1:${server.address().port}/`);
});

test("vite error overlay marks the snapshot as a capture failure", async win => {
  await win.loadURL(`http://127.0.0.1:${server.address().port}/overlay`);
  const snapshot = await win.webContents.executeJavaScript(
    `${collectAuditSnapshot.toString()};collectAuditSnapshot({ proposed: true, overflowChecks: true })`, true);
  assert.equal(snapshot.errorOverlay, true, "overlay detected");
  assert.match(classifyCapture(snapshot, 20), /vite error overlay/);
  // The normal page stays clean.
  await win.loadURL(`http://127.0.0.1:${server.address().port}/`);
  const clean = await win.webContents.executeJavaScript(
    `${collectAuditSnapshot.toString()};collectAuditSnapshot({ proposed: true, overflowChecks: true })`, true);
  assert.equal(clean.errorOverlay, false, "no overlay on the normal page");
  assert.equal(classifyCapture(clean, 20), null);
});

test("e2e page CSS parses with the static block parser", () => {
  // Imported late so the node-side parser is exercised against the same source.
  return import("./design-audit/static-scan.mjs").then(({ parseCssBlocks }) => {
    const css = PAGE.match(/<style>([\s\S]*?)<\/style>/)[1];
    const blocks = parseCssBlocks(css);
    assert.ok(blocks.some(b => b.selector === ".offscale" && b.body.includes("13px")), "nested at-top-level rules parse");
    assert.equal(blocks.filter(b => b.selector === ":root").length, 1);
  });
});

async function main() {
  try {
    await app.whenReady();
    server = createServer((req, res) => {
      res.setHeader("Content-Type", "text/html");
      res.end(req.url === "/overlay" ? OVERLAY_PAGE : req.url === "/calibration" ? CALIBRATION_PAGE : PAGE);
    });
    await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
    const win = new BrowserWindow({
      show: false, width: 800, height: 600,
      webPreferences: { contextIsolation: true, sandbox: true, backgroundThrottling: false },
    });
    await win.loadURL(`http://127.0.0.1:${server.address().port}/`);
    let failed = 0;
    for (const { name, fn } of tests) {
      try {
        await fn(win);
        console.log(`✔ ${name}`);
      } catch (error) {
        failed += 1;
        console.error(`✖ ${name}\n  ${error.message}`);
      }
    }
    win.destroy();
    await new Promise(resolve => server.close(resolve));
    app.exit(failed ? 1 : 0);
  } catch (error) {
    console.error(error);
    app.exit(1);
  }
}

// node:assert deepEqual parity for groupBy availability check
if (!Object.groupBy) { console.error("Object.groupBy unavailable"); process.exit(1); }
void main();
