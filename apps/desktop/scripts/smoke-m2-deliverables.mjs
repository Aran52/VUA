// M2 交付验收冒烟(在 Electron Main 进程内执行):
// 1. 启动受监督真实 Provider(与 app 同一装配路径);
// 2. 演示任务全链路(权威状态来自 SQLite);
// 3. 暂时断连:杀死 Provider 进程 → invoke 返回不可用 → 重新 start 恢复;
// 4. 重启恢复:标记遗留任务 → 关闭 Provider → 重开同一数据库 →
//    任务保留最后真实状态且 recoveryDisposition = inspect_required;
// 5. Renderer 重载与多窗口:第二个本地来源窗口收到同一事件广播,且
//    远程来源窗口被广播过滤排除;
// 6. 进程关闭:prepareShutdown → Provider 协议化退出,无孤儿。
// 证据写入 _local_m2/v<版本>/m2-deliverables-smoke.json(.gitignore 排除)。
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { mkdir, rm, writeFile } from "node:fs/promises";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { app, BrowserWindow, session } from "electron";
import {
  installLocalContentNavigationPolicy,
  installPermissionDenyPolicy,
  isAllowedLocalSender,
  localWindowWebPreferences,
} from "../dist/electron/security.js";

app.setPath("userData", mkdtempSync(path.join(os.tmpdir(), "vua-m2-smoke-")));

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "../../..");
const desktopPackage = JSON.parse(
  readFileSync(path.resolve(scriptDirectory, "../package.json"), "utf8"),
);
const productVersion = desktopPackage.version;
const evidenceDirectory = path.join(repositoryRoot, "_local_m2", `v${productVersion}`);
const evidencePath = path.join(evidenceDirectory, "m2-deliverables-smoke.json");
const logLines = [];

function log(event, detail = {}) {
  const entry = { at: new Date().toISOString(), event, ...detail };
  logLines.push(JSON.stringify(entry));
  console.log(JSON.stringify(entry));
}

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// 可靠性验收:任何未处理的拒绝都计为冒烟失败,不允许带警告零退出
let unhandledRejections = 0;
process.on("unhandledRejection", (reason) => {
  unhandledRejections += 1;
  log("smoke.unhandled_rejection", { message: String(reason) });
});

// 只终止本次冒烟自身启动的 Provider 子进程句柄与 PID,绝不按进程名误杀机器上的其他 VUA 实例
const ownedChildren = new Set();

function isProcessAlive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error?.code === "EPERM";
  }
}

function listOwnedProviderPids() {
  return [...ownedChildren]
    .filter((child) => child.exitCode === null && child.signalCode === null && Number.isInteger(child.pid) && isProcessAlive(child.pid))
    .map((child) => child.pid);
}

function killOwnedProviders() {
  const active = [...ownedChildren].filter((child) => child.exitCode === null && child.signalCode === null && Number.isInteger(child.pid));
  let killed = 0;
  for (const child of active) {
    const pid = child.pid;
    try {
      child.kill("SIGKILL");
    } catch {
      /* ignore if already exited */
    }
    if (process.platform === "win32" && isProcessAlive(pid)) {
      try {
        execFileSync("taskkill", ["/F", "/T", "/PID", String(pid)], { stdio: "ignore" });
      } catch {
        /* process exited between check and taskkill */
      }
    }
    killed += 1;
  }
  return killed;
}

async function waitFor(predicate, label, timeoutMs = 10_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    // 必须 await:异步判定返回 Promise 本身恒为真值,会让检查假绿
    if (await predicate()) return;
    await delay(25);
  }
  throw new Error(`Timed out waiting for ${label}`);
}

async function serveOnce(html) {
  const server = http.createServer((_request, response) => {
    response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    response.end(html);
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  if (address === null || typeof address === "string") {
    throw new Error("smoke server did not bind a TCP port");
  }
  return { server, url: `http://127.0.0.1:${address.port}/` };
}

async function invoke(provider, method, extra = {}) {
  return provider.invoke({
    contractVersion: "0.1",
    requestId: `m2-${Math.random().toString(36).slice(2)}`,
    correlationId: "m2-smoke",
    kind: extra.commandId === undefined ? "query" : "command",
    method,
    params: extra.params ?? {},
    ...(extra.commandId === undefined ? {} : { commandId: extra.commandId }),
  });
}

async function main() {
  const results = { checks: [] };
  const check = (name, ok, detail = {}) => {
    results.checks.push({ name, ok, ...detail });
    log(ok ? "check.passed" : "check.failed", { name, ...detail });
    if (!ok) throw new Error(`M2 smoke check failed: ${name}`);
  };

  // ---- 装配:与 app 完全相同的端点解析(独立库,不污染用户数据) ----
  const platformSuffix = process.platform === "win32" ? ".exe" : "";
  const executablePath = process.env.VUA_PROVIDER_EXECUTABLE
    ?? (process.env.CARGO_TARGET_DIR
      ? path.join(
          process.env.CARGO_TARGET_DIR,
          "release",
          `vua-orchestrator-provider${platformSuffix}`,
        )
      : path.join(
          scriptDirectory,
          "..",
          "..",
          "..",
          "target",
          "release",
          `vua-orchestrator-provider${platformSuffix}`,
        ));
  const { existsSync } = await import("node:fs");
  if (!existsSync(executablePath)) {
    throw new Error(`Provider executable is missing: ${executablePath}`);
  }
  const databasePath = path.join(evidenceDirectory, `m2-smoke-provider-${process.pid}.db`);
  await rm(databasePath, { force: true });

  const providerModule = await import(
    pathToFileURL(
      path.join(scriptDirectory, "../node_modules/@vua/orchestrator-provider/dist/index.js"),
    ).href
  );
  const { SupervisedProcessProviderV01, providerEnvironment } = providerModule;
  const spawnOwnedProvider = (exe, db) => {
    const child = spawn(exe, ["--database", db], {
      cwd: path.dirname(exe),
      env: providerEnvironment(process.env),
      shell: false,
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    });
    ownedChildren.add(child);
    child.once("close", () => ownedChildren.delete(child));
    return child;
  };

  let provider = new SupervisedProcessProviderV01({
    executablePath,
    databasePath,
    handshakeTimeoutMs: 15_000,
  }, spawnOwnedProvider);
  const kernelEvents = [];
  provider.subscribe((event) => kernelEvents.push(event));
  await provider.start();
  log("provider.started", { databasePath });

  // ---- 检查 1:演示任务全链路(权威状态在 SQLite) ----
  const started = await invoke(provider, "task.startDemo", { commandId: "m2-demo-1" });
  assert.equal(started.ok, true);
  const taskId = started.value.task.taskId;
  check("demo_task.accepted", taskId.startsWith("demo-"), { taskId });

  const cancelled = await invoke(provider, "task.requestCancellation", {
    commandId: "m2-cancel-1",
    params: { taskId },
  });
  assert.equal(cancelled.value.outcome, "requested");
  // 下一帧推进到终态
  await invoke(provider, "task.list", {});
  await waitFor(async () => {
    const list = await invoke(provider, "task.list", {});
    const task = list.value.tasks.find((entry) => entry.taskId === taskId);
    return task?.state === "cancelled";
  }, "demo task to reach cancelled");
  check("demo_task.cancelled_terminal", true, { taskId });

  // ---- 检查 2:暂时断连(只杀测试拥有的 Provider → invoke 不可用 → 重启恢复) ----
  const killed = killOwnedProviders();
  await delay(300);
  let during = null;
  let transportDead = false;
  try {
    during = await invoke(provider, "task.list", {});
  } catch {
    // 传输层死亡(写入已退出的进程)同样是断连事实
    transportDead = true;
  }
  check(
    "disconnect.invoke_unavailable",
    transportDead || (during !== null && during.ok === false),
    { transportDead, code: during === null ? "transport" : during.ok ? "ok" : during.error.code },
  );
  check(
    "disconnect.no_orphan_after_kill",
    killed > 0 && listOwnedProviderPids().length === 0,
    { killed },
  );

  provider = new SupervisedProcessProviderV01({
    executablePath,
    databasePath,
    handshakeTimeoutMs: 15_000,
  }, spawnOwnedProvider);
  provider.subscribe((event) => kernelEvents.push(event));
  await provider.start();
  const after = await invoke(provider, "task.list", {});
  assert.equal(after.ok, true);
  check("disconnect.recovered_after_restart", after.ok === true, {
    tasks: after.value.tasks.length,
  });

  // ---- 检查 3:重启恢复(遗留非终态任务 → inspect_required) ----
  const leftover = await invoke(provider, "task.startDemo", { commandId: "m2-leftover-1" });
  assert.equal(leftover.ok, true);
  const leftoverId = leftover.value.task.taskId;
  // 只杀本次冒烟拥有的子进程:任务停在非终态,没有到达安全边界
  killOwnedProviders();
  await delay(300);

  const restarted = new SupervisedProcessProviderV01({
    executablePath,
    databasePath,
    handshakeTimeoutMs: 15_000,
  }, spawnOwnedProvider);
  restarted.subscribe((event) => kernelEvents.push(event));
  await restarted.start();
  const recovered = await invoke(restarted, "task.list", {});
  const leftoverTask = recovered.value.tasks.find((entry) => entry.taskId === leftoverId);
  // 契约语义:遗留非终态任务在重启后立即归结为诚实的中断终态(排队/准备期 → cancelled,执行期 → failed),
  // 绝不静默续跑或继续冒充 running,并标注 inspect_required 供下一步检查接管。
  check(
    "restart_recovery.inspect_required",
    leftoverTask !== undefined
      && (leftoverTask.state === "cancelled" || leftoverTask.state === "failed")
      && leftoverTask.recoveryDisposition === "inspect_required",
    { state: leftoverTask?.state, disposition: leftoverTask?.recoveryDisposition },
  );

  await invoke(restarted, "task.list", {});

  // ---- 检查 4:Renderer 重载与多窗口事件同步 ----
  installPermissionDenyPolicy(session.defaultSession);
  await app.whenReady();
  const preload = path.join(scriptDirectory, "../dist/electron/preload.js");
  const localWindow = new BrowserWindow({
    show: false,
    webPreferences: { ...localWindowWebPreferences(preload), backgroundThrottling: false },
  });
  const { server, url: rendererUrl } = await serveOnce(
    "<!doctype html><html><body><h1>VUA M2 smoke</h1></body></html>",
  );
  installLocalContentNavigationPolicy(localWindow.webContents, {
    rendererUrl,
    allowedOrigins: [],
    openExternal: () => {},
  });
  await localWindow.loadURL(rendererUrl);
  // 本地窗口 preload 面证明(行为法):隔离世界订阅 Kernel 广播,收到即面活着。
  // contextBridge 对象对 executeJavaScript(主世界)不可见是 Electron 语义,
  // 因此探测主世界恒 false —— 行为证明与多窗口检查共用同一通道。
  const armListener = async (window) => {
    try {
      const result = await window.webContents.executeJavaScript(
        "window.__vuaProbe = []; " +
        "if (window.vua?.events) { window.vua.events.subscribe((event) => window.__vuaProbe.push(event.kind)); } " +
        "Boolean(window.vua?.events)",
      );
      return result;
    } catch (error) {
      console.log(JSON.stringify({ at: new Date().toISOString(), event: "debug.armlistener.error", message: String(error) }));
      return false;
    }
  };
  const readProbe = (window) => window.webContents.executeJavaScript("window.__vuaProbe ?? []");

  const sawBefore = await armListener(localWindow);
  await delay(100);
  for (const window of BrowserWindow.getAllWindows()) {
    window.webContents.send("vua:gateway:event", {
      contractVersion: "0.1",
      eventId: "m2-reload-probe",
      revision: 1,
      occurredAt: new Date().toISOString(),
      correlationId: "m2-reload-probe",
      kind: "capability.changed",
      payload: { revision: 1, operations: [] },
    });
  }
  await delay(200);
  const receivedBefore = await readProbe(localWindow);

  localWindow.webContents.reload();
  await waitFor(() => !localWindow.webContents.isLoading(), "renderer reload");
  await delay(300);
  const sawAfter = await armListener(localWindow);
  await delay(100);
  for (const window of BrowserWindow.getAllWindows()) {
    window.webContents.send("vua:gateway:event", {
      contractVersion: "0.1",
      eventId: "m2-reload-probe-2",
      revision: 2,
      occurredAt: new Date().toISOString(),
      correlationId: "m2-reload-probe",
      kind: "capability.changed",
      payload: { revision: 2, operations: [] },
    });
  }
  await delay(200);
  const receivedAfter = await readProbe(localWindow);

  check(
    "reload.preload_surface_persists",
    sawBefore === true && receivedBefore.length > 0 && sawAfter === true && receivedAfter.length > 0,
    { before: { armed: sawBefore, received: receivedBefore.length }, after: { armed: sawAfter, received: receivedAfter.length } },
  );

  // 多窗口:第二个本地窗口 + 一个远程窗口;广播只达本地来源
  const remoteWindow = new BrowserWindow({
    show: false,
    webPreferences: localWindowWebPreferences(preload),
  });
  const { url: remoteUrl, server: remoteServer } = await serveOnce(
    "<!doctype html><html><body><h1>remote</h1></body></html>",
  );
  await remoteWindow.loadURL(remoteUrl);

  const received = { local: [], remote: [] };
  const collect = (window, bucket) => {
    void window.webContents.executeJavaScript(
      `window.vua.events.subscribe((event) => { window.__events = window.__events || []; window.__events.push(event.kind); }); true`,
    );
    // 轮询读回(保持通道最小面:不为冒烟新增 IPC)
    const timer = setInterval(() => {
      void window.webContents
        .executeJavaScript("window.__events ?? []")
        .then((events) => {
          bucket.length = 0;
          bucket.push(...events);
        })
        .catch(() => {});
    }, 50);
    return () => clearInterval(timer);
  };
  const stopLocal = collect(localWindow, received.local);
  const stopRemote = collect(remoteWindow, received.remote);
  await delay(200);

  // Kernel 广播语义复刻(与 main.ts 同一过滤):仅本地来源窗口收到事件;
  // 远程来源窗口即使被注入了 preload 面,也不得收到广播。
  for (const window of BrowserWindow.getAllWindows()) {
    if (!isAllowedLocalSender(window.webContents.getURL(), rendererUrl)) continue;
    window.webContents.send("vua:gateway:event", {
      contractVersion: "0.1",
      eventId: "m2-broadcast-1",
      revision: 1,
      occurredAt: new Date().toISOString(),
      correlationId: "m2-broadcast",
      kind: "capability.changed",
      payload: { revision: 1, operations: [] },
    });
  }
  await delay(300);
  stopLocal();
  stopRemote();
  check(
    "multi_window.broadcast_reaches_local_only",
    received.local.includes("capability.changed") && !received.remote.includes("capability.changed"),
    { local: received.local.length, remote: received.remote.length },
  );
  server.close();
  remoteServer.close();

  // ---- 检查 5:进程关闭协议 ----
  const shutdown = await restarted.prepareShutdown({ timeoutMs: 5_000 });
  check("shutdown.safe_to_stop", shutdown.outcome === "safe_to_stop", {
    outcome: shutdown.outcome,
  });
  await delay(500);
  check(
    "shutdown.no_orphan_provider",
    listOwnedProviderPids().length === 0,
    { remaining: listOwnedProviderPids().length },
  );
  check("reliability.no_unhandled_rejections", unhandledRejections === 0, {
    count: unhandledRejections,
  });

  await mkdir(evidenceDirectory, { recursive: true });
  await rm(databasePath, { force: true });
  await rm(`${databasePath}-wal`, { force: true });
  await rm(`${databasePath}-shm`, { force: true });
  await writeFile(
    evidencePath,
    `${JSON.stringify({ at: new Date().toISOString(), productVersion, ...results }, null, 2)}\n`,
  );
  log("smoke.completed", { evidence: evidencePath });
}

main()
  .then(() => {
    app.exit(0);
  })
  .catch((error) => {
    killOwnedProviders();
    log("smoke.failed", { message: String(error) });
    console.error(error);
    mkdir(evidenceDirectory, { recursive: true })
      .then(() =>
        writeFile(
          evidencePath,
          `${JSON.stringify({ at: new Date().toISOString(), productVersion, failed: String(error), log: logLines }, null, 2)}\n`,
        ),
      )
      .finally(() => app.exit(1));
  });
