// 顶栏占用查看器 DOM 冒烟(第 182 批反向审查批,对象 cae84388):真实
// Chromium DOM,合成 system 宿主,零生产/远程服务;不宣称端到端。
// 监听泄漏钉:页面内 add/remove 调用计数对「引用失配型泄漏」失明(两版本
// 调用次数都对称),故经 CDP DOMDebugger.getEventListeners 取窗口 blur
// 监听真实注册数,断言开合循环前后零残留(修复前每轮开合泄漏 1 个必红)。
// 证据落临时目录 JSON:红绿两轮都落——失败证据带 status:"failed" 与错误
// 信息,覆盖同名文件,防止 tmp 残留上一轮绿 JSON 被误引为绿。
// 退出纪律(集成第 200 批验收退回修复):原失败臂 window.destroy() 之后
// await server?.close() 实测永不 settle,事件循环排空进程 exit 0,
// app.exit(1) 永不到达=红轮假绿。修法=关停顺序重排(先关 server,窗口
// 仍在、事件循环健康,close 可正常 settle)＋5s 超时竞速兜底＋app.exit
// 保证退出码到达;红轮 exit≠0 即 app.exit(1) 到达的自证(排空只会 0)。
import { app, BrowserWindow } from "electron";
import { createServer } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { writeFileSync } from "node:fs";
import os from "node:os";
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
let server, window;
const checks = [];
function check(ok, name) {
  if (!ok) throw new Error(name);
  checks.push(name);
}
/** 窗口 blur 监听真实注册数(CDP 地面真值,页面内不可观测) */
async function countWindowBlurListeners() {
  const evaluated = await window.webContents.debugger.sendCommand("Runtime.evaluate", {
    expression: "window",
    returnByValue: false,
    objectGroup: "smoke",
  });
  const detail = await window.webContents.debugger.sendCommand("DOMDebugger.getEventListeners", {
    objectId: evaluated.result.objectId,
  });
  return detail.listeners.filter((listener) => listener.type === "blur").length;
}
function evidenceBase(status) {
  return { date: new Date().toISOString(), status, browser: process.versions.chrome,
    scope: "Actual Chromium DOM, synthetic system host; no remote or production E2E claim" };
}
/** 关停:同步落证据 → 先关 server(窗口仍在,close 可 settle;5s 竞速兜底
 * 防挂)→ 再销毁窗口 → app.exit 保证退出码到达(顺序不可重排回 destroy
 * 在前:那正是红轮假绿的根因)。 */
async function shutdown(exitCode, evidence) {
  try {
    writeFileSync(path.join(os.tmpdir(), "vua-resource-monitor-dom.json"), JSON.stringify(evidence, null, 2));
  } catch (writeError) {
    console.error(writeError);
  }
  try {
    await Promise.race([
      server?.close(),
      new Promise((resolve) => setTimeout(resolve, 5000)),
    ]);
  } catch (closeError) {
    console.error(closeError);
  }
  window?.destroy();
  app.exit(exitCode);
}
async function main() {
  let behaviorChecks = [];
  try {
    await app.whenReady();
    server = await createServer({ configFile: false, appType: "custom", root, plugins: [react()],
      server: { host: "127.0.0.1", port: 0, strictPort: false },
      optimizeDeps: { include: ["react", "react-dom/client", "@vua/contracts"] } });
    server.middlewares.use(async (req, res, next) => {
      if (req.url !== "/__resource-monitor") return next();
      res.setHeader("Content-Type", "text/html");
      res.end(await server.transformIndexHtml(req.url, '<html><body><div id="root"></div><script type="module" src="/scripts/fixtures/resource-monitor-popover.tsx"></script></body></html>'));
    });
    await server.listen();
    window = new BrowserWindow({ show: false, width: 1280, height: 900,
      webPreferences: { offscreen: true, backgroundThrottling: false, contextIsolation: true, nodeIntegration: false, sandbox: true } });
    window.webContents.on("console-message", (event) => { if (event.level === "error") console.error(event.message); });
    await window.loadURL(`http://127.0.0.1:${server.httpServer.address().port}/__resource-monitor`);
    await window.webContents.executeJavaScript('new Promise((resolve, reject) => { let attempts = 0; const timer = setInterval(() => { if(window.resourceMonitorSmoke) { clearInterval(timer); resolve(); } else if (++attempts > 200) { clearInterval(timer); reject(new Error("fixture load timeout")); } }, 50); })');
    behaviorChecks = await window.webContents.executeJavaScript("window.resourceMonitorSmoke.base()");
    // 监听生命周期钉(CDP 地面真值):基线 → 三轮开合 → 断言零残留
    await window.webContents.debugger.attach("1.3");
    await window.webContents.debugger.sendCommand("Runtime.enable");
    const blurBeforeCycles = await countWindowBlurListeners();
    await window.webContents.executeJavaScript("window.resourceMonitorSmoke.lifecycleCycles()");
    const blurAfterCycles = await countWindowBlurListeners();
    check(
      blurAfterCycles === blurBeforeCycles,
      `开合循环后窗口 blur 监听零残留(CDP 实数:循环前 ${blurBeforeCycles},循环后 ${blurAfterCycles};` +
        "修复前 remove 传匿名新箭头按引用失配永不生效,每轮开合泄漏 1 个必红)",
    );
    const failureChecks = await window.webContents.executeJavaScript("window.resourceMonitorSmoke.failureFaces()");
    // 唯一断言计数:行为面(base 快照)＋CDP 钉＋失败面(failureFaces 快照)
    // ——夹具已改为各阶段返回本阶段快照,不再双计(集成第 200 批勘误)。
    const allChecks = [...behaviorChecks, ...checks, ...failureChecks];
    const evidence = { ...evidenceBase("passed"),
      windowBlurListeners: { beforeCycles: blurBeforeCycles, afterCycles: blurAfterCycles },
      checks: allChecks, passed: allChecks.length };
    console.log(JSON.stringify(evidence, null, 2));
    await shutdown(0, evidence);
  } catch (error) {
    console.error(error);
    const evidence = { ...evidenceBase("failed"), error: error?.message ?? String(error),
      checks: [...behaviorChecks, ...checks],
      passed: behaviorChecks.length + checks.length };
    console.error(JSON.stringify(evidence, null, 2));
    await shutdown(1, evidence);
  }
}
void main();
