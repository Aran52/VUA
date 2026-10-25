// End-to-end test for the design-audit collector and rules: synthetic HTML
// pages rendered in an offscreen Electron window, collected in-page, evaluated
// node-side. Covers every rule's trigger and its documented exceptions,
// including [data-audit-skip]. Run: electron scripts/design-audit-e2e.test.mjs

import { app, BrowserWindow } from "electron";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { collectAuditSnapshot } from "./design-audit/collector.js";
import { finishThemeTransitions } from "./design-audit/settle.js";
import { captureAuditPage } from "./design-audit/capture.mjs";
import { applyAnnotations, removeAnnotations } from "./design-audit/annotate.js";
import { evaluateSnapshot, classifyCapture } from "./design-audit/rules.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
let gridPage = "";

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
.local { --vua-local-surface:color-mix(in srgb, blue 40%, white); background:var(--vua-local-surface); }
</style></head><body>
<div class="camera"><div id="scene" class="scene"></div><div class="hidden"><button id="hidden-control">hidden</button></div></div>
<div id="bleed" class="bleed">actual viewport overflow</div>
<div class="caption-clip"><span id="caption" class="caption">actual clipped caption</span></div>
<label id="large-label" class="label"><input id="labelled" type="checkbox">a usable label</label>
<input id="unlabelled" type="checkbox" aria-label="tiny independent control">
<div inert><button id="inert-control" style="width:16px;height:16px;padding:0">inactive background action</button></div>
<div role="dialog"><button id="scoped-control">dialog action</button></div>
<span id="assistive" style="position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%)">assistive label</span>
<select id="native-select" style="font:400 12px/16px sans-serif"><option>native choice</option></select>
<details><summary id="closed-summary">collapsed details</summary><button id="collapsed-control">hidden child</button></details>
<div style="content-visibility:hidden"><button id="content-hidden-control">hidden content</button></div>
<div id="mixed" style="background:color-mix(in srgb, white 20%, black);color:#888">dim text on a mixed surface</div>
<div id="local-token" class="local">a locally scoped token</div>
<input id="text-field" type="text" style="font:400 13px/18px sans-serif">
<span id="icon-line" style="line-height:0"><svg width="16" height="16"></svg></span>
<button id="status-control" data-vua-control-role="indicator">status</button>
<div data-vua-primary-group=""><button id="split-main">main</button><button id="split-menu">menu</button></div>
<div style="width:100px;height:24px;overflow:hidden"><span id="fixed-escape" style="position:fixed;left:500px;top:100px;width:100px;height:20px">fixed overlay</span></div>
<div style="position:relative;transform:translateX(0);width:100px;height:24px;overflow:hidden"><span id="fixed-trapped" style="position:fixed;left:80px;top:0;width:200px;height:20px;white-space:nowrap">truly clipped fixed text</span></div>
<div style="position:relative;width:250px;height:24px"><div style="width:40px;height:24px;overflow:hidden"><span id="absolute-escape" style="position:absolute;left:100px;top:0;width:100px;height:20px">absolute overlay</span></div></div>
<div style="visibility:hidden"><span id="visibility-override" style="visibility:visible">a visibly restored child</span></div>
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
  const pixels = (await captureAuditPage(win, true)).toBitmap();
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
  assert.equal(element("assistive"), undefined, "fully clipped assistive labels have no paint");
  assert.equal(element("collapsed-control"), undefined, "closed details retain layout without painting their children");
  assert.equal(element("content-hidden-control"), undefined, "content-visibility hidden is not painted content");
  assert.ok(element("closed-summary"), "a closed details summary remains a visible action");
  assert.equal(flagged("contrast", "mixed"), true, "modern computed colours retain real contrast failures");
  assert.ok(Math.abs(element("mixed").backgroundColor.r - 51) < 0.1);
  assert.ok(snapshot.palette.some(entry => entry.name === "--vua-local-surface" && Math.abs(entry.color.r - 153) < 0.1), "local tokens resolve in their feature scope");
  assert.equal(flagged("type-scale", "native-select"), false, "native select leading is controlled by Chromium");
  assert.equal(element("native-select").isChoice, true);
  assert.equal(flagged("type-scale", "icon-line"), false);
  assert.equal(flagged("type-scale", "text-field"), true, "empty text fields still retain their type check");
  assert.equal(element("status-control").indicatorRole, true);
  assert.equal(element("split-main").primaryGroup, element("split-menu").primaryGroup);
  assert.ok(element("split-main").primaryGroup);
  assert.equal(Math.round(element("fixed-escape").paintedRect.w), 100, "fixed content escapes an unrelated overflow ancestor");
  assert.equal(flagged("overflow", "fixed-escape"), false);
  assert.equal(flagged("overflow", "fixed-trapped"), true, "a transformed containing block still clips fixed content");
  assert.equal(Math.round(element("absolute-escape").paintedRect.w), 100, "absolute content follows its containing block");
  assert.ok(element("visibility-override"), "inherited visibility may be explicitly restored");
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

test("settling theme colours preserves camera and activity motion", async win => {
  await win.loadURL(`http://127.0.0.1:${server.address().port}/calibration`);
  await win.webContents.executeJavaScript(`(async () => {
    document.body.style.transition = 'color 60s linear';
    getComputedStyle(document.body).color;
    await new Promise(resolve => requestAnimationFrame(resolve));
    document.body.style.color = 'black';
    getComputedStyle(document.body).color;
    window.camera = document.getElementById('scene').animate([{transform:'translateX(0)'}, {transform:'translateX(100px)'}], {duration:60000});
    window.camera.pause(); window.camera.currentTime = 30000;
    window.activity = document.getElementById('bleed').animate([{opacity:1}, {opacity:.5}], {duration:1000, iterations:Infinity});
  })()`, true);
  await win.webContents.executeJavaScript(`${finishThemeTransitions.toString()};finishThemeTransitions()`, true);
  const state = await win.webContents.executeJavaScript(`({colour:getComputedStyle(document.body).color, camera:window.camera.currentTime, activity:window.activity.playState})`, true);
  assert.equal(state.colour, "rgb(0, 0, 0)");
  assert.equal(state.camera, 30000);
  assert.equal(state.activity, "running");
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

test("hidden compositor advances scripted exits across repeated opens", async win => {
  await win.loadURL(`http://127.0.0.1:${server.address().port}/`);
  const completed = await win.webContents.executeJavaScript(`(async () => {
    for (let index = 0; index < 2; index += 1) {
      const panel = document.createElement('div');
      panel.textContent = 'closing dialog';
      document.body.append(panel);
      const exit = panel.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 120, fill: 'forwards' });
      await Promise.race([exit.finished, new Promise((_, reject) => setTimeout(() => reject(new Error('hidden exit stopped advancing')), 2000))]);
      panel.remove();
    }
    return true;
  })()`, true);
  assert.equal(completed, true);
});

test("actual Grid controls paint and hit-test a stroked upper-left 120-degree bevel", async win => {
  await win.loadURL(`http://127.0.0.1:${server.address().port}/grid`);
  const geometry = await win.webContents.executeJavaScript(`(() => {
    const button = document.querySelector('#grid-button'); const r = button.getBoundingClientRect();
    const hit = (x,y) => document.elementFromPoint(r.left+x,r.top+y)?.closest('#grid-button') !== null;
    return { supported: CSS.supports('corner-shape','bevel'), rect:{x:r.x,y:r.y,w:r.width,h:r.height},
      viewport:{w:innerWidth,h:innerHeight}, clipped:!hit(1,1), inside:hit(1,10),
      right:hit(r.width-1,1), bottom:hit(1,r.height-1),
      childRole:getComputedStyle(document.querySelector('#grid-label')).getPropertyValue('--vua-shape-role').trim(),
      host:['host-button','host-sidebar','host-settings'].map(id => { const s=getComputedStyle(document.getElementById(id));
        return {role:s.getPropertyValue('--vua-shape-role').trim(),corner:s.getPropertyValue('corner-top-left-shape'),radius:s.borderTopLeftRadius,
          corners:[s.borderTopLeftRadius,s.borderTopRightRadius,s.borderBottomRightRadius,s.borderBottomLeftRadius]}; }),
      portal:{role:getComputedStyle(document.getElementById('owned-portal')).getPropertyValue('--vua-shape-role').trim(),
        radius:getComputedStyle(document.getElementById('owned-portal')).borderTopLeftRadius} };
  })()`, true);
  assert.equal(geometry.supported, true);
  assert.equal(geometry.clipped, true);
  assert.equal(geometry.inside, true);
  assert.equal(geometry.right, true);
  assert.equal(geometry.bottom, true);
  assert.equal(geometry.childRole, "none", "text never inherits its parent's control role");
  assert.ok(geometry.host.every(style => style.role === "none" && style.corner !== "superellipse(0)" && new Set(style.corners).size === 1),
    "header, sidebar and settings retain host corners even in the production jurisdiction");
  assert.equal(geometry.host[0].radius, "6px", "the shared host button retains its rounded rectangle");
  assert.equal(geometry.portal.role, "menu", "an AMF-owned portal retains its own scope");
  assert.equal(geometry.portal.radius, "0px", "small owned menus stay quiet rectangles");
  const capture = await captureAuditPage(win, false, 40);
  const pixels = capture.toBitmap(); const size = capture.getSize();
  const scaleX = size.width / geometry.viewport.w, scaleY = size.height / geometry.viewport.h;
  let diagonalPixels = 0;
  for (let y = 3; y <= 9; y += 1) for (let x = 2; x <= 5; x += 1) {
    const px = Math.round((geometry.rect.x + x) * scaleX), py = Math.round((geometry.rect.y + y) * scaleY);
    const index = (py * size.width + px) * 4;
    if (pixels[index + 2] > 180 && pixels[index + 1] < 120 && pixels[index] < 120) diagonalPixels += 1;
  }
  assert.ok(diagonalPixels >= 2, "the slanted edge itself carries the border");
  const snapshot = await win.webContents.executeJavaScript(`${collectAuditSnapshot.toString()};collectAuditSnapshot({ proposed:true, overflowChecks:true })`, true);
  const findings = evaluateSnapshot(snapshot, { rules:["grid-shape", "shadow-panel"] });
  assert.deepEqual(findings.filter(f => f.rule === "grid-shape").map(f => f.cls[0]), ["broken-grid"]);
  assert.equal(findings.filter(f => f.rule === "shadow-panel").length, 0, "selected and resting steps both stay flat");
});

async function main() {
  try {
    await app.whenReady();
    const root = path.resolve(scriptDir, "../../..");
    const files = ["packages/design-system/src/tokens.css", "packages/design-system/src/base.css", "packages/design-system/src/shapes.css",
      "apps/desktop/src/renderer/components/primitives/button.css", "apps/desktop/src/renderer/components/primitives/badge.css",
      "apps/desktop/src/renderer/components/primitives/toggle.css", "apps/desktop/src/renderer/features/amf/factory.css"];
    const css = (await Promise.all(files.map(file => readFile(path.join(root, file), "utf8")))).join("\n").replace(/@import\s+[^;]+;/g, "");
    gridPage = `<!doctype html><html data-theme="dark"><head><style>${css}
      body{padding:32px}#grid-button{width:160px;height:40px;border-color:rgb(255,0,0)}
      .vua-line__stations{margin-top:24px;width:500px}.broken-grid{border-radius:999px!important}
    </style></head><body><div data-vua-shape-scope="amf">
      <button id="grid-button" class="vua-button vua-button--primary"><span id="grid-label">Grid action</span></button>
      <span class="vua-badge" data-tone="neutral">Status</span><input aria-label="Field"><button class="vua-toggle" aria-label="Switch"></button>
      <ol class="vua-line__stations"><li><button class="vua-line__station" data-vua-surface-role="workshop-step" aria-pressed="true">Active</button></li>
      <li><button class="vua-line__station" data-vua-surface-role="workshop-step" aria-pressed="false">Rest</button></li></ol>
      <button class="broken-grid vua-button--primary">Capsule regression</button></div>
      <div class="vua-shell" data-module="production"><header><button id="host-button" class="vua-button">Host action</button></header>
        <aside><button id="host-sidebar">Host sidebar</button></aside><section><input id="host-settings" aria-label="Host settings"></section></div>
      <div id="owned-portal" class="vua-tooltip" data-vua-shape-scope="amf">Owned portal</div></body></html>`;
    server = createServer((req, res) => {
      res.setHeader("Content-Type", "text/html");
      res.end(req.url === "/overlay" ? OVERLAY_PAGE : req.url === "/calibration" ? CALIBRATION_PAGE : req.url === "/grid" ? gridPage : PAGE);
    });
    await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
    const win = new BrowserWindow({
      show: false, width: 800, height: 600,
      webPreferences: { contextIsolation: true, sandbox: true, backgroundThrottling: false, offscreen: true },
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
