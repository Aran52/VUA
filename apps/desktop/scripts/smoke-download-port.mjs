// F4-3 下载端口冒烟(在 Electron Main 进程内执行):
// 真实分区 Session + 本地 HTTP fixture,验证冻结词表的事件规范化——
// 1. 完整下载:started(storedPath 在暂存根)→ progress → completed(字节数
//    与文件长度一致);
// 2. 未知长度:无 content-length 的下载 → expectedBytes null;
// 3. 策略拒绝:允许清单外来源在文件创建前 failed/policy(storedPath 空);
// 4. 弃件(abandon 意图):cancelled 事件 + 部分文件弃除;同 URL 重发起
//    重绑原 downloadId 且 attempt 递增。
// 5. 手动暂停与继续:部分字节稳定、自动续传不覆盖手动暂停,同一 attempt 完成。
// 范围注记:interrupted(可续传中断)在 Chromium 中默认被静默自动续传,
// 真实网络下的浮现时机不可控——该路径的状态机逻辑由单元测试覆盖
// (假 DownloadItem 驱动),真实网络中断归 I-1 真机矩阵。
// 每个事件都过 isDownloadEventV01 守卫(deny-unknown-fields 镜像)。
// 证据写入 _local_m4/v<版本>/download-port-smoke.json(.gitignore 排除)。
import assert from "node:assert/strict";
import { existsSync, readFileSync, mkdtempSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";
import { app, session } from "electron";
import { DownloadPort } from "../dist/electron/download-port.js";
import { isDownloadEventV01 } from "../../../packages/contracts/dist/index.js";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "../../..");
const desktopPackage = JSON.parse(readFileSync(path.resolve(scriptDirectory, "../package.json"), "utf8"));
const productVersion = desktopPackage.version;
const evidenceDirectory = path.join(repositoryRoot, "_local_m4", `v${productVersion}`);
const evidencePath = path.join(evidenceDirectory, "download-port-smoke.json");
const rawLogPath = path.join(evidenceDirectory, "download-port-smoke.log");
const logLines = [];
app.setPath("userData", mkdtempSync(path.join(os.tmpdir(), "vua-download-profile-")));

function log(event, detail = {}) {
  const entry = { at: new Date().toISOString(), event, ...detail };
  logLines.push(JSON.stringify(entry));
  console.log(JSON.stringify(entry));
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

async function waitFor(predicate, label, timeoutMs = 8_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate()) return;
    await delay(25);
  }
  throw new Error(`Timed out waiting for ${label}`);
}

function closeServer(server) {
  if (!server.listening) return Promise.resolve();
  return new Promise((resolve) => server.close(() => resolve()));
}

/** unknown-size 路由不回 content-length(unknown size 语义) */
function createServer() {
  return http.createServer((request, response) => {
    if (request.url === "/paced.bin") {
      response.writeHead(200, { "content-type": "application/octet-stream", "content-length": String(32 * 65536) });
      let sent = 0;
      const timer = setInterval(() => { response.write(Buffer.alloc(65536, 9)); if (++sent === 32) { clearInterval(timer); response.end(); } }, 60);
      response.on("close", () => clearInterval(timer));
      return;
    }
    if (request.url === "/unknown-size.bin") {
      response.writeHead(200, { "content-type": "application/octet-stream" });
      response.end(Buffer.alloc(512, 5));
      return;
    }
    response.writeHead(200, {
      "content-type": "application/octet-stream",
      "content-length": String(1024),
    });
    response.end(Buffer.alloc(1024, 7));
  });
}

async function listen(server) {
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  if (address === null || typeof address === "string") throw new Error("smoke server did not bind");
  return `http://127.0.0.1:${address.port}`;
}

let evidence = {
  schemaVersion: 1,
  productVersion,
  status: "failed",
  evidencePath,
  rawLogPath,
};

async function run() {
  const servers = [];
  try {
    log("process.start", { electronVersion: process.versions.electron });
    await app.whenReady();

    const allowedServer = createServer();
    const blockedServer = createServer();
    servers.push(allowedServer, blockedServer);
    const allowedOrigin = await listen(allowedServer);
    const blockedOrigin = await listen(blockedServer);
    log("server.ready", { allowedOrigin, blockedOrigin });

    const events = [];
    const stagingRoot = path.join(app.getPath("temp"), `vua-download-smoke-${process.pid}`);
    const downloadSession = session.fromPartition(`vua-download-smoke-${process.pid}`);
    const port = new DownloadPort({
      stagingRoot,
      allowedOrigins: [allowedOrigin],
      // F4-4 retry 语义:弃件后端口经同一 Session downloadURL 重发起
      partitionSession: downloadSession,
      progressIntervalMs: 0,
      sink: {
        emit: (event) => {
          assert.equal(isDownloadEventV01(event), true, `event failed the frozen guard: ${JSON.stringify(event)}`);
          events.push(event);
          log("download.event", {
            kind: event.kind,
            downloadId: event.downloadId,
            attempt: event.attempt,
            receivedBytes: event.receivedBytes,
            expectedBytes: event.expectedBytes,
          });
        },
      },
    });

    downloadSession.on("will-download", (event, item, webContents) => {
      port.handleWillDownload(event, item, webContents);
    });

    // 1. 完整下载
    downloadSession.downloadURL(`${allowedOrigin}/full.bin`);
    await waitFor(() => events.some((event) => event.kind === "download.completed"), "completed download");
    const started = events.find((event) => event.kind === "download.started");
    const completed = events.find((event) => event.kind === "download.completed");
    assert.ok(started && completed);
    assert.ok(completed.storedPath.startsWith(stagingRoot));
    assert.equal(completed.receivedBytes, 1024);
    assert.equal(readFileSync(completed.storedPath).length, 1024);
    log("happy.path.passed", { storedPath: completed.storedPath });

    // 2. 未知长度:expectedBytes 如实 null(不以 0 注水)
    events.length = 0;
    downloadSession.downloadURL(`${allowedOrigin}/unknown-size.bin`);
    await waitFor(() => events.some((event) => event.kind === "download.completed"), "unknown-size download");
    const unknownStarted = events.find((event) => event.kind === "download.started");
    assert.equal(unknownStarted.expectedBytes, null);
    log("unknown.size.passed");

    // Native pause retains the attempt and file; automatic resume must respect the user.
    events.length = 0;
    downloadSession.downloadURL(`${allowedOrigin}/paced.bin`);
    await waitFor(() => events.some(event => event.kind === "download.progress" && event.receivedBytes > 0), "paced native transfer");
    const paced = events.find(event => event.kind === "download.started");
    assert.equal(port.setPaused(paced.downloadId, true), true);
    await delay(250);
    const count = events.length;
    port.applyIntent(paced.downloadId, "resume");
    await delay(600);
    assert.equal(events.length, count, "user pause stops progress and survives automatic resume");
    assert.equal(port.hasActiveTransfers(), true);
    assert.equal(port.setPaused(paced.downloadId, false), true);
    await waitFor(() => events.some(event => event.kind === "download.completed"), "resumed native transfer");
    const resumed = events.find(event => event.kind === "download.completed");
    assert.equal(resumed.downloadId, paced.downloadId); assert.equal(resumed.attempt, 1);
    assert.equal(readFileSync(resumed.storedPath).length, 32 * 65536);
    assert.equal(events.some(event => event.kind === "download.cancelled"), false);
    log("user.pause.resume.passed");

    // 3. 策略拒绝:允许清单外来源,文件创建前 failed/policy
    events.length = 0;
    downloadSession.downloadURL(`${blockedOrigin}/blocked.bin`);
    await waitFor(() => events.some((event) => event.kind === "download.failed"), "policy rejection");
    const rejected = events.find((event) => event.kind === "download.failed");
    assert.equal(rejected.failureKind, "policy");
    assert.equal(rejected.storedPath, null);
    log("policy.rejection.passed");

    // 4a. 弃件放弃:abandon 意图 = 终局放弃(取消 + 弃除部分文件,不 Emit
    //     cancelled);其后同 URL 重发起 = 用户新授权,全新 downloadId、
    //     attempt 从 1(三值词汇裁定:abandon = terminal give-up)
    events.length = 0;
    downloadSession.downloadURL(`${allowedOrigin}/abandon.bin`);
    await waitFor(() => events.some((event) => event.kind === "download.started"), "abandon download started");
    const abandonStarted = events.find((event) => event.kind === "download.started");
    assert.ok(existsSync(abandonStarted.storedPath));
    port.applyIntent(abandonStarted.downloadId, "abandon");
    await waitFor(() => !existsSync(abandonStarted.storedPath), "partial file discarded");
    assert.equal(events.some((event) => event.kind === "download.cancelled"), false, "abandon is not terminal");

    events.length = 0;
    downloadSession.downloadURL(`${allowedOrigin}/abandon.bin`);
    await waitFor(() => events.some((event) => event.kind === "download.started"), "post-abandon download started");
    const fresh = events.find((event) => event.kind === "download.started");
    assert.notEqual(fresh.downloadId, abandonStarted.downloadId, "post-abandon restart is a fresh download");
    assert.equal(fresh.attempt, 1, "post-abandon restart starts at attempt 1");

    // 4b. 弃件重试:retry 意图 = 弃件 + 端口经 downloadURL 重发起,新 item
    //     重绑原 downloadId 且 attempt 递增(冻结裁定:放弃部分文件从零重启
    //     才递增,重绑原 id)
    events.length = 0;
    downloadSession.downloadURL(`${allowedOrigin}/retry.bin`);
    await waitFor(() => events.some((event) => event.kind === "download.started"), "retry download started");
    const retryStarted = events.find((event) => event.kind === "download.started");
    assert.ok(existsSync(retryStarted.storedPath));
    port.applyIntent(retryStarted.downloadId, "retry");
    await waitFor(() => events.some((event) => event.kind === "download.started" && event.downloadId === retryStarted.downloadId && event.attempt === 2), "rebound download started");
    const rebound = events.find((event) => event.kind === "download.started" && event.attempt === 2);
    assert.equal(rebound.downloadId, retryStarted.downloadId, "downloadId stable across retry restart");
    assert.equal(rebound.attempt, 2, "retry restart increments attempt");
    port.applyIntent(rebound.downloadId, "abandon");
    log("abandon.rebind.passed");

    evidence = {
      schemaVersion: 1,
      productVersion,
      status: "passed",
      electronVersion: process.versions.electron,
      stagingRoot,
      eventKinds: [...new Set(events.map((event) => event.kind))],
      evidencePath,
      rawLogPath,
    };
  } catch (error) {
    evidence = {
      ...evidence,
      error: error instanceof Error ? { name: error.name, message: error.message, stack: error.stack } : String(error),
    };
    process.exitCode = 1;
    log("smoke.failed", { error: evidence.error });
  } finally {
    for (const server of servers) await closeServer(server);
    await mkdir(evidenceDirectory, { recursive: true });
    await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
    await writeFile(rawLogPath, `${logLines.join("\n")}\n`, "utf8");
    console.log(`Evidence: ${evidencePath}`);
    console.log(`Raw log: ${rawLogPath}`);
    app.exit(process.exitCode ?? 0);
  }
}

void run();
