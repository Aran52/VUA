// Real Main/preload/renderer, isolated UI preferences, no vendor or hardware actions.
import { app, BrowserWindow } from "electron";
import { createRequire } from "node:module";
import childProcess from "node:child_process";
import { fileURLToPath } from "node:url";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const output = fs.mkdtempSync(path.join(os.tmpdir(), "vua-startup-ui-"));
const testHome = path.join(output, "home");
for (const directory of [testHome, path.join(testHome, "AppData", "Roaming"), path.join(testHome, "AppData", "Local")]) fs.mkdirSync(directory, { recursive: true });
app.setPath("home", testHome);
app.setPath("appData", path.join(testHome, "AppData", "Roaming"));
process.env.LOCALAPPDATA = path.join(testHome, "AppData", "Local");
process.env.VUA_DEV_USER_DATA = path.join(output, "profile");
process.env.VUA_PROVIDER_EXECUTABLE = process.env.VUA_TEST_HOST_EXECUTABLE ?? path.resolve(desktop, "../../target/release/vua-orchestrator-provider.exe");
process.env.VUA_AMF_EXECUTABLE = process.env.VUA_TEST_AMF_EXECUTABLE ?? path.resolve(desktop, "../../target/release/vua-amf-provider.exe");
delete process.env.VUA_RENDERER_URL;
// Exercise a valid cold handshake that exceeds the old 7.5-second UI wait.
// Only this diagnostic delays the handshake request; the real provider and its timeout stay intact.
const slowProvider = process.argv.includes("--slow-provider");
const spawn = childProcess.spawn;
let slowedProvider = false;
if (slowProvider) childProcess.spawn = (...args) => {
  const child = spawn(...args);
  if (args[0] === process.env.VUA_PROVIDER_EXECUTABLE) {
    slowedProvider = true;
    const write = child.stdin.write;
    child.stdin.write = function (...writeArgs) {
      child.stdin.write = write;
      setTimeout(() => write.apply(this, writeArgs), 12_000);
      return true;
    };
  }
  return child;
};
const localeCase = process.argv.includes("--locale-unmatched") ? "unmatched"
  : process.argv.includes("--locale-ordered") ? "ordered" : "native";
if (localeCase === "unmatched") app.getPreferredSystemLanguages = () => ["fr-FR", "de-DE"];
if (localeCase === "ordered") app.getPreferredSystemLanguages = () => ["fr-FR", "ko-KR", "en-US"];
const shown = [];
const errors = [];
BrowserWindow.prototype.show = function () { shown.push({ window: this.id, url: this.webContents.getURL(), loading: this.webContents.isLoading() }); };
BrowserWindow.prototype.focus = function () {};
app.on("browser-window-created", (_event, window) => {
  window.webContents.setBackgroundThrottling(false);
  window.webContents.on("console-message", event => { if (event.level === "error") errors.push(event.message); });
});
let main;
let lastStep = "initialization", lastJavaScript = "";
let exitCode = 1;
app.on("will-quit", () => { if (exitCode) app.exit(exitCode); });
const deadline = setTimeout(() => { console.error(JSON.stringify({ failure: "Startup UI smoke timed out", wait: lastStep, operation: lastJavaScript, url: main?.webContents.getURL(), loading: main?.webContents.isLoading() })); app.quit(); }, 120_000);
async function waitFor(check, label, timeoutMs = 7_500) {
  lastStep = label;
  console.log("Startup UI: " + label);
  const expires = performance.now() + timeoutMs;
  while (performance.now() < expires) { if (await check()) return; await delay(50); }
  throw new Error(`Timeout: ${label}`);
}
const js = source => { lastJavaScript = source.slice(0, 180); return main.webContents.executeJavaScript(source, true); };
const welcome = () => js('document.querySelector(".vua-tour")?.dataset.tourStep==="welcome"');
const firstUseNotice = () => js('!!document.querySelector("[data-first-use-notice]") && !document.querySelector(".vua-tour")');
const home = () => js('!!document.querySelector(".vua-home") && !document.querySelector(".vua-onboarding") && !document.querySelector(".vua-tour")');
const skip = () => js('document.querySelector(".vua-tour__actions button").click()');
const next = () => js('document.querySelector(".vua-tour__nav button:last-child").click()');
const reload = async source => { const loaded = new Promise(resolve => main.webContents.once("did-finish-load", resolve)); await js(`${source};location.reload()`); await loaded; };
const fits = selector => js(`(()=>{const c=document.querySelector(${JSON.stringify(selector)}),r=c.getBoundingClientRect();return r.left>=-1&&r.top>=-1&&r.right<=innerWidth+1&&r.bottom<=innerHeight+1&&c.scrollWidth<=c.clientWidth+1})()`);
async function startTour() {
  await js('document.querySelector("[data-nav-id=shell-help]").click()');
  await waitFor(() => js('!!document.querySelector("[data-nav-id=help-tour]")'), "Help tour entry");
  await js('document.querySelector("[data-nav-id=help-tour]").click()');
  await waitFor(() => js('!!document.querySelector("[data-nav-id=help-start-tour]")'), "Help tour child");
  await js('document.querySelector("[data-nav-id=help-start-tour]").click()');
  await waitFor(welcome, "explicit tour welcome");
}
async function capture(window, name) {
  await window.webContents.executeJavaScript('document.fonts.ready.then(()=>true)');
  await window.webContents.executeJavaScript('document.getAnimations().filter(animation=>Number.isFinite(animation.effect?.getComputedTiming().endTime)).forEach(animation=>animation.finish())');
  const [width, height] = window.getSize();
  if (window.isResizable()) { window.setSize(width + 1, height); await delay(50); window.setSize(width, height); }
  await window.webContents.capturePage();
  await delay(120);
  fs.writeFileSync(path.join(output, `${name}.png`), (await window.webContents.capturePage(undefined, { stayHidden: true })).toPNG());
}
async function run() {
  const checks = [];
  const assert = (condition, label) => { if (!condition) throw new Error(label); checks.push(label); };
  try {
    createRequire(import.meta.url)(path.join(desktop, "dist/packaged-electron/main.js"));
    await app.whenReady();
    let splash;
    await waitFor(() => {
      splash = BrowserWindow.getAllWindows().find(window => window.webContents.getURL().includes("surface=splash"));
      return splash && !splash.webContents.isLoading();
    }, "native splash");
    assert(splash.getContentSize().join(",") === "480,320" && !splash.isResizable() && !splash.isMaximizable() && !splash.isMinimizable(), `Startup uses a small fixed-size native window: ${JSON.stringify({ content: splash.getContentSize(), resizable: splash.isResizable(), maximizable: splash.isMaximizable(), minimizable: splash.isMinimizable() })}`);
    assert(await splash.webContents.executeJavaScript('document.querySelectorAll("button,input,select").length===0 && !!document.querySelector(".vua-boot-splash__loader") && document.querySelector(".vua-boot-splash__logo").naturalWidth===992 && !document.querySelector(".vua-shell")'), "Splash shows only the supplied logo and loader, without the main application");
    // Inspect the actual angle even when a CI runner hides rays for reduced motion.
    // Chromium's floating-point matrix serialization is not an angle contract.
    const rayRotation = await splash.webContents.executeJavaScript(`(()=>{
      const rays=document.querySelector(".vua-boot-splash__rays"), display=rays.style.display;
      rays.style.display="block";
      const transform=getComputedStyle(document.querySelector(".vua-boot-splash__ray-track")).transform;
      const matrix=new DOMMatrixReadOnly(transform), expected=[-0.5,Math.sqrt(3)/2,-Math.sqrt(3)/2,-0.5,0,0];
      rays.style.display=display;
      return {transform,valid:matrix.is2D&&[matrix.a,matrix.b,matrix.c,matrix.d,matrix.e,matrix.f].every((value,i)=>Math.abs(value-expected[i])<0.000001)};
    })()`);
    assert(rayRotation.valid, `Black rays keep the approved 120-degree direction: ${rayRotation.transform}`);
    // Native startup permits a 15-second handshake before opening the renderer.
    // Allow that existing budget plus page loading on a cold Windows runner.
    await waitFor(() => {
      main = BrowserWindow.getAllWindows().find(window => window.webContents.getURL().startsWith("file:") && !window.webContents.getURL().includes("surface=splash"));
      return main && !main.webContents.isLoading();
    }, "main application load", 30_000);
    if (!splash.isDestroyed()) {
      const rejected = await splash.webContents.executeJavaScript('window.vua.startup.complete().then(()=>false,()=>true)');
      assert(rejected, "The splash cannot announce main-frame readiness");
    }
    await waitFor(firstUseNotice, "first-use notice");
    assert(!await js('!!document.querySelector(".vua-tour")'), "The notice waits for an explicit entry choice");
    assert(await js('document.querySelectorAll("[data-first-use-part]").length===4 && document.querySelectorAll(".vua-onboarding button").length===2 && !document.querySelector(".vua-onboarding select, .vua-onboarding input")'), "Four notice parts and two choices replace the language question");
    assert(await js('document.querySelector("[data-first-use-part=source]").textContent.includes("Apache 2.0") && document.querySelector("[data-first-use-part=licenses] a").href.endsWith("/THIRD_PARTY_NOTICES.md")'), "The first-use notice links its free source and third-party licenses");
    await waitFor(() => splash.isDestroyed(), "startup window teardown");
    assert(shown.filter(item => item.window === main.id).length === 1 && shown.filter(item => item.window === main.id).every(item => !item.loading), "Main opens once after loading and the small splash is destroyed");
    if (slowProvider) assert(slowedProvider && await js('window.vua.gateway.invoke({schemaVersion:1,requestId:"slow-startup",method:"app.snapshot",params:{}}).then(result=>result.ok)'), "A delayed native handshake reaches the notice with a working Gateway");
    const nativeLanguages = app.getPreferredSystemLanguages();
    assert(await js('JSON.stringify(window.vua.startup.systemLanguages)===' + JSON.stringify(JSON.stringify(nativeLanguages))), "Preload supplies the Windows language preference list");
    const match = nativeLanguages.map(tag => tag.trim().toLowerCase()).map(tag => /^zh-(hant|tw|hk|mo)(-|$)/.test(tag) ? "zh-TW" : tag.startsWith("zh") ? "zh-CN" : tag.startsWith("en") ? "en" : tag.startsWith("ja") ? "ja" : tag.startsWith("ko") ? "ko" : null).find(Boolean) ?? "en";
    assert(await js('document.documentElement.lang===' + JSON.stringify(match) + ' && localStorage.getItem("vua-locale")===null'), "First use adopts the first supported system language without a manual override");
    if (localeCase === "unmatched") assert(match === "en", "Unsupported system languages fall back to English");
    if (localeCase === "ordered") assert(match === "ko", "A later supported language is selected");
    await js('document.querySelector("[data-nav-id=first-use-beginner]").click()');
    await waitFor(() => js('!!document.querySelector("[data-wizard-step=vrchat]") && !document.querySelector(".vua-tour")'), "beginner entry starts with VRChat");
    assert(await js('localStorage.getItem("vua-locale")===null && JSON.parse(localStorage.getItem("vua-goals")).onboarding==="completed"'), "The beginner entry preserves system language and records completion");
    await js('document.querySelector("[data-nav-id=logo-home]").click()');
    await waitFor(home, "leave the beginner wizard");
    await reload("");
    await waitFor(home, "completed first use stays completed");
    assert(!await js('!!document.querySelector(".vua-tour")'), "A returning profile does not repeat the first-use tour");

    if (localeCase === "native") {
      main.setSize(960, 600);
      for (const locale of ["zh-CN", "zh-TW", "en", "ja", "ko"]) {
        const reset = 'localStorage.setItem("vua-locale",' + JSON.stringify(locale) + ');localStorage.removeItem("vua-goals");localStorage.removeItem("vua-context-guide-v1");sessionStorage.removeItem("vua-context-guide-snoozed-v1");localStorage.removeItem("vua-beginner-wizard-v1");localStorage.setItem("vua-theme","dark");history.replaceState(null,"",location.pathname+location.search)';
        await reload(reset);
        await waitFor(firstUseNotice, locale + " notice");
        assert(await js('document.documentElement.lang===' + JSON.stringify(locale) + ' && !document.querySelector(".vua-onboarding select")'), locale + " respects the saved language without asking again");
        await js('document.querySelector("[data-nav-id=first-use-enter]").scrollIntoView({block:"center"})');
        assert(await fits(".vua-onboarding__footer"), locale + " entry choices remain reachable in a short window");
        await js('document.querySelector("[data-nav-id=first-use-enter]").click()');
        await waitFor(welcome, locale + " Got it opens the tour");
        assert(await js('localStorage.getItem("vua-locale")===' + JSON.stringify(locale) + ' && JSON.parse(localStorage.getItem("vua-goals")).goals.length===0'), locale + " acknowledgment preserves language and invents no goals");
        assert(await fits(".vua-tour__card"), locale + " first-use tour fits the window");
        if (locale === "zh-CN") await capture(main, "welcome-zh-dark");
        await next();
        await waitFor(() => js('document.querySelector(".vua-tour")?.dataset.tourStep==="route" && !!document.querySelector(".vua-tour__highlight")'), "first highlighted feature");
        await js('document.querySelector(".vua-tour__nav button:first-child").click()');
        await waitFor(welcome, "Back to welcome");
        await skip();
        await waitFor(home, locale + " skips the light tour");
        await waitFor(() => js('!!document.querySelector("[data-context-guide=home]")'), locale + " optional Home introduction");
        assert(await fits(".vua-context-guide"), locale + " contextual guide fits");
        await js('document.querySelector(".vua-context-guide__actions button:last-child").click()');
        await reload("");
        await waitFor(home, locale + " returns Home");
        assert(await js('!document.querySelector(".vua-context-guide") && !document.querySelector(".vua-tour")'), locale + " dismissal survives reload");
        await startTour();
        assert(await fits(".vua-tour__card"), locale + " explicit replay fits");
        await skip();
        await waitFor(home, locale + " exits replay");

        await reload(reset);
        await waitFor(firstUseNotice, locale + " beginner choice");
        await js('document.querySelector("[data-nav-id=first-use-beginner]").click()');
        await waitFor(() => js('!!document.querySelector("[data-wizard-step=vrchat]")'), locale + " first concept");
        assert(await js('document.querySelectorAll(".vua-beginner-wizard__article").length===1 && !document.querySelector(".vua-tour")'), locale + " shows one concept without a tour");
        await js('document.querySelector("[data-nav-id=wizard-next]").click()');
        await waitFor(() => js('!!document.querySelector("[data-wizard-step=steam]")'), locale + " next concept");
        await js('document.querySelector("[data-nav-id=wizard-want]").click()');
        await waitFor(() => js('!!document.querySelector(".vua-account-grid")'), locale + " wants Steam account setup");
        await js('document.querySelector("[data-nav-id=shell-help]").click()');
        await waitFor(() => js('!!document.querySelector("[data-nav-id=help-wizard]")'), "Help wizard entry");
        await js('document.querySelector("[data-nav-id=help-wizard]").click()');
        await waitFor(() => js('!!document.querySelector("[data-wizard-step=steam]")'), "concept bookmark restored");
        await js('document.querySelector("[data-nav-id=wizard-previous]").click()');
        await waitFor(() => js('!!document.querySelector("[data-wizard-step=vrchat]")'), "previous concept");
        await js('document.querySelector("[data-guide-entry=what-is-vrchat]").click()');
        await waitFor(() => js('!!document.querySelector("[data-guide-section=what-is-vrchat]") && localStorage.getItem("vua-guide-reading")?.includes("what-is-vrchat")'), locale + " shares the encyclopedia explanation");
        await js('document.querySelector("[data-nav-id=logo-home]").click()');
        await waitFor(home, locale + " leaves the wizard freely");
      }
      for (const legacy of [{v:2,status:"active",step:2}, {v:1,status:"active",step:4}]) {
        await reload(`localStorage.setItem("vua-tour-progress",${JSON.stringify(JSON.stringify(legacy))});localStorage.setItem("vua-last-page","home")`);
        await waitFor(home, "returning profile with an old active tour");
        assert(!await js('!!document.querySelector(".vua-tour")'), `Tour bookmark v${legacy.v} never forces a tour on startup`);
      }
      await reload('localStorage.setItem("vua-locale","zh-CN");localStorage.setItem("vua-last-page","home")');
      await waitFor(home, "complete explicit tour baseline");
      const savedGoals = await js('localStorage.getItem("vua-goals")');
      await startTour();
      for (const step of ["welcome", "route", "network", "checks", "plan", "tools", "creator", "tasks", "guide", "settings"]) {
        await waitFor(() => js(`document.querySelector(".vua-tour")?.dataset.tourStep===${JSON.stringify(step)} && ${step === "welcome" ? "true" : '!!document.querySelector(".vua-tour__highlight")'}`), `${step} highlight`);
        assert(await js('!document.querySelector(".vua-tour").hasAttribute("data-absent")') && await fits(".vua-tour__card"), `${step} points to an available feature and fits the window`);
        if (step === "creator") await capture(main, "tour-creator-zh");
        await next();
      }
      await waitFor(() => js('!document.querySelector(".vua-tour")'), "explicit tour completes");
      assert(await js(`JSON.parse(localStorage.getItem("vua-tour-progress")).status==="completed" && localStorage.getItem("vua-goals")===${JSON.stringify(savedGoals)}`), "Completing the optional tour does not reset onboarding or task goals");
      await reload('localStorage.setItem("vua-effects","off");localStorage.setItem("vua-last-page","home");history.replaceState(null,"",location.pathname+location.search)');
      await waitFor(home, "completed tour remains closed");
      assert(await js('!document.querySelector(".vua-tour") && !document.querySelector(".vua-onboarding") && document.documentElement.dataset.effects==="off"'), "Returning profiles keep resource saving without repeating onboarding");
      const staticSplash = new BrowserWindow({ width: 480, height: 320, show: false, webPreferences: { sandbox: true, contextIsolation: true, nodeIntegration: false } });
      await staticSplash.loadFile(path.join(desktop, "dist/renderer/index.html"), { query: { surface: "splash" } });
      assert(await staticSplash.webContents.executeJavaScript('document.querySelector(".vua-boot-splash__surface").hasAttribute("data-static") && getComputedStyle(document.querySelector(".vua-boot-splash__loader")).animationName==="none"'), "Resource saving keeps startup monochrome and static with a visible loader");
      await capture(staticSplash, "splash-static");
      staticSplash.destroy();
    }
    assert(errors.length === 0, `Renderer errors: ${errors.join("; ")}`);
    fs.writeFileSync(path.join(output, "checks.json"), JSON.stringify({ checks, localeCase, scope: "Controlled UI startup, language and navigation only; no hardware/vendor or human language acceptance" }, null, 2));
    console.log(JSON.stringify({ passed: checks.length, localeCase, evidence: output }));
    exitCode = 0;
  } catch (error) {
    console.error(error);
    if (main && !main.isDestroyed()) await capture(main, "failure");
    console.error(JSON.stringify({ evidence: output, rendererErrors: errors }));
  } finally { childProcess.spawn = spawn; clearTimeout(deadline); app.quit(); }
}
void run();
