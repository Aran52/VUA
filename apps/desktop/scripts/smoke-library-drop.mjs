// Actual Main/preload/providers plus Chromium's native File drag store. Every
// source and profile is temporary and synthetic; no user material/account is used.
import { app, BrowserWindow, dialog, session, ipcMain } from "electron";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const output = fs.mkdtempSync(path.join(os.tmpdir(), "vua-library-drop-"));
const home = path.join(output, "home"), profile = path.join(output, "profile");
for (const directory of [home, profile, path.join(home, "AppData", "Roaming"), path.join(home, "AppData", "Local")]) fs.mkdirSync(directory, { recursive: true });
app.setPath("home", home); app.setPath("appData", path.join(home, "AppData", "Roaming")); app.setPath("userData", profile);
process.env.LOCALAPPDATA = path.join(home, "AppData", "Local"); process.env.VUA_DEV_USER_DATA = profile;
process.env.VUA_PROVIDER_EXECUTABLE = process.env.VUA_TEST_HOST_EXECUTABLE ?? path.resolve(desktop, "../../target/debug/vua-orchestrator-provider.exe");
process.env.VUA_AMF_EXECUTABLE = process.env.VUA_TEST_AMF_EXECUTABLE ?? path.resolve(desktop, "../../target/debug/vua-amf-provider.exe");
delete process.env.VUA_RENDERER_URL;
app.getPreferredSystemLanguages = () => ["zh-CN"];
BrowserWindow.prototype.show = function () {}; BrowserWindow.prototype.focus = function () {};
dialog.showErrorBox = () => console.error("Desktop bootstrap failed in isolated drop smoke");
const checks = []; let window, exitCode = 1;
const intakeReceipts = [];
const intakeCommandIds = []; let loseNextReceipt = false;
const handle = ipcMain.handle.bind(ipcMain);
ipcMain.handle = (channel, listener) => handle(channel, async (event, ...args) => {
  const result = await listener(event, ...args);
  if (channel === "vua:gateway:invoke" && args[0]?.method === "library.importPaths") {
    intakeReceipts.push(result); intakeCommandIds.push(args[0].params.commandId);
    // Controlled transport fixture: the provider persists acceptance, then this
    // one receipt is hidden from Renderer. Retry must retain the same command.
    if (loseNextReceipt && result.ok) {
      loseNextReceipt = false;
      return { schemaVersion: 1, requestId: args[0].requestId, ok: false, error: { code: "internal", messageKey: "errors.gateway.invalidProviderResponse" } };
    }
  }
  return result;
});
app.on("will-quit", () => { if (exitCode) app.exit(exitCode); });
const deadline = setTimeout(() => { console.error("Library drop smoke timed out"); app.exit(1); }, 180_000);
const assert = (ok, label) => { if (!ok) throw Error(label); checks.push(label); };
async function waitFor(test, label, limit = 15000) {
  const start = Date.now(); while (Date.now() - start < limit) { if (await test()) return; await delay(60); }
  throw Error(`Timeout: ${label}`);
}
async function run() { try {
  createRequire(import.meta.url)(path.join(desktop, "dist/packaged-electron/main.js"));
  await app.whenReady();
  const remote = session.fromPartition("persist:vua-remote");
  await remote.protocol.handle("https", () => new Response('<html><body><form action="/users/auth/pixiv"></form></body></html>', { headers: { "content-type": "text/html" } }));
  await waitFor(() => { window = BrowserWindow.getAllWindows().find(w => w.webContents.getURL().startsWith("file:") && !w.webContents.getURL().includes("surface=")); return window && !window.webContents.isLoading(); }, "renderer");
  const js = source => window.webContents.executeJavaScript(source, true);
  window.webContents.setBackgroundThrottling(false); window.webContents.debugger.attach("1.3");
  await waitFor(() => js('document.querySelector(".vua-tour")?.dataset.tourStep === "welcome"'), "welcome");
  await js('document.querySelector(".vua-tour__actions button").click()');
  await waitFor(() => js('!!document.querySelector(".vua-onboarding") && !document.querySelector(".vua-boot-splash")'), "guide");
  await js('document.querySelector(".vua-journey-top button").click()');
  const list = () => js('window.vua.gateway.invoke({schemaVersion:1,requestId:crypto.randomUUID(),method:"library.list",params:{schemaVersion:"0.1",source:"local"}})');
  const drop = async files => {
    await js('window.__nativeDrop=null;window.addEventListener("drop",event=>{window.__nativeDrop={types:[...event.dataTransfer.types],count:event.dataTransfer.files.length}},{once:true,capture:true})');
    const data = { items: [], files, dragOperationsMask: 1 };
    for (const type of ["dragEnter", "dragOver", "drop"]) await window.webContents.debugger.sendCommand("Input.dispatchDragEvent", { type, x: 500, y: 380, data });
  };
  let total = 0, folderCount = 0;
  for (const locale of ["zh-CN", "en", "ja", "ko"]) {
    await js(`localStorage.setItem('vua-locale', ${JSON.stringify(locale)})`);
    await window.loadURL(window.webContents.getURL());
    await waitFor(() => js('!!document.querySelector(".vua-shell") && !document.querySelector(".vua-boot-splash")'), `${locale} shell`);
    assert(await js('window.vua.library.resolveDroppedFiles([new File(["synthetic"],"pretend.zip")]).then(()=>false,()=>true)'), `${locale}: JavaScript-created files cannot supply native paths`);
    const sources = path.join(output, `sources-${locale}`); fs.mkdirSync(sources);
    const single = path.join(sources, "single.txt"), folder = path.join(sources, "folder");
    fs.writeFileSync(single, "synthetic selected file"); fs.writeFileSync(path.join(sources, "unselected.txt"), "must stay out of the library");
    fs.mkdirSync(folder); fs.mkdirSync(path.join(folder, "sub")); fs.writeFileSync(path.join(folder, "sub", "companion.txt"), "synthetic companion");
    const before = new URL(window.webContents.getURL());
    await drop([single, folder, path.join(folder, "sub")]); total += 2; folderCount++;
    await waitFor(async () => { const result = await list(); return result.ok && result.value.total === total && result.value.items.every(item => item.storage.presentCopies > 0); }, `${locale} mixed intake`);
    const after = new URL(window.webContents.getURL());
    assert(after.protocol === before.protocol && after.pathname === before.pathname, `${locale}: native drops never navigate away from VUA`);
    await waitFor(() => js('document.querySelector("[data-nav-id=nav-warehouse]")?.getAttribute("aria-current") === "page" && !!document.querySelector(".vua-warehouse")'), `${locale} imported library`);
    assert(await js('document.querySelector("[data-nav-id=nav-warehouse]")?.getAttribute("aria-current") === "page"'), `${locale}: accepted import opens the library through the AMF route guard`);
    const result = await list();
    assert(result.value.items.filter(item => item.entry.displayName === "folder").length === folderCount, `${locale}: dropped folder descendants do not make duplicate entries`);
    assert(!result.value.items.some(item => item.entry.displayName === "unselected.txt" || item.entry.displayName === "sub"), `${locale}: single-file intake does not copy siblings or duplicate dropped descendants`);
    assert(fs.readFileSync(single, "utf8") === "synthetic selected file" && fs.readFileSync(path.join(folder, "sub", "companion.txt"), "utf8") === "synthetic companion", `${locale}: source bytes and folder layout stay intact`);
    if (locale === "zh-CN") {
      const lost = path.join(sources, "lost-receipt.txt"); fs.writeFileSync(lost, "synthetic receipt-loss input");
      loseNextReceipt = true; await drop([lost]); total++;
      await waitFor(() => js('!!document.querySelector("[data-drop-action=import]") && !!document.querySelector("[role=dialog] [role=status]")'), "lost receipt stays actionable");
      await waitFor(async () => (await list()).value?.total === total, "provider accepted the hidden receipt");
      await js('document.querySelector("[data-drop-action=import]").click()');
      await waitFor(() => js('!document.querySelector("[role=dialog]")'), "same-command receipt retry");
      assert(intakeCommandIds.at(-1) === intakeCommandIds.at(-2), "controlled lost-receipt retry retains its exact command ID");
      assert((await list()).value.total === total, "controlled lost-receipt retry never imports the selection twice");
    }
    await waitFor(async () => (await js('window.vua.amfModule.setEnabled(false)')).outcome === "updated", `${locale} disable idle AMF`);
    await waitFor(() => js('!!document.querySelector("[data-nav-id=nav-enable-amf]")'), `${locale} disabled navigation`);
    const disabledFile = path.join(sources, "disabled.txt"); fs.writeFileSync(disabledFile, "retained until explicitly enabled");
    await drop([disabledFile]);
    await waitFor(() => js('!!document.querySelector("[data-drop-action=import]")'), `${locale} AMF prompt`);
    assert(!(await js('window.vua.amfModule.snapshot()')).installed, `${locale}: a drop never implicitly enables AMF`);
    assert(await js('document.querySelector("[role=dialog]")?.textContent.includes("AMF")'), `${locale}: the prompt explains the built-in material manager prerequisite`);
    await js('document.querySelector(".vua-content-dialog__close").click()');
    assert(!(await js('window.vua.amfModule.snapshot()')).installed, `${locale}: cancelling the prompt leaves AMF disabled`);
    await drop([disabledFile]); await waitFor(() => js('!!document.querySelector("[data-drop-action=import]")'), `${locale} second prompt`);
    for (const theme of ["dark", "light"]) {
      await js(`document.documentElement.dataset.theme=${JSON.stringify(theme)}`);
      await window.webContents.capturePage({}, { stayHidden: true, stayAwake: true }); await delay(350);
      fs.writeFileSync(path.join(output, `drop-${locale}-${theme}.png`), (await window.webContents.capturePage({}, { stayHidden: true, stayAwake: true })).toPNG());
    }
    await js('document.querySelector("[data-drop-action=import]").click()'); total++;
    await waitFor(async () => { const result = await list(); return result.ok && result.value.total === total && result.value.items.every(item => item.storage.presentCopies > 0); }, `${locale} explicit enable and import`);
    await waitFor(() => js('!document.querySelector("[role=dialog]") && document.querySelector("[data-nav-id=nav-warehouse]")?.getAttribute("aria-current") === "page"'), `${locale} settled prompt and library`);
    assert((await js('window.vua.amfModule.snapshot()')).state === "ready", `${locale}: explicit enable imports the retained native File after its drag event ends`);
    assert((await list()).value.total === total, `${locale}: cancelled drops never add an entry`);
    // Text and website drags remain ordinary text; they are never intake commands.
    await js('window.dispatchEvent(new DragEvent("drop",{bubbles:true,dataTransfer:(()=>{const data=new DataTransfer();data.setData("text/plain","https://booth.pm/");return data})()}))');
    await delay(100); assert((await list()).value.total === total, `${locale}: text/URL drops do not import materials`);
  }
  fs.writeFileSync(path.join(output, "checks.json"), JSON.stringify({ scope: "Actual Electron/Main/preload/providers and synthetic native files; not real-material or human UI acceptance", checks }, null, 2));
  console.log(JSON.stringify({ passed: checks.length, output })); exitCode = 0; clearTimeout(deadline); app.quit();
} catch (error) {
  console.error(error.stack ?? String(error));
  const diagnostic = await window?.webContents.executeJavaScript('({drop:window.__nativeDrop,text:document.body.textContent.slice(-1800),dialogs:[...document.querySelectorAll("[role=dialog]")].map(node=>node.textContent)})').catch(() => null);
  console.error(JSON.stringify({ output, checks, diagnostic, intakeReceipts })); clearTimeout(deadline); app.quit();
} }
void run();
