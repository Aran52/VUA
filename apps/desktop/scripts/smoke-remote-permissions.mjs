import assert from "node:assert/strict";
import { mkdtempSync, readFileSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { app, BrowserWindow, session } from "electron";
import {
  installLocalContentNavigationPolicy,
  installPermissionDenyPolicy,
  localWindowWebPreferences,
} from "../dist/electron/security.js";

app.setPath("userData", mkdtempSync(path.join(os.tmpdir(), "vua-remote-perm-smoke-")));

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "../../..");
const desktopPackage = JSON.parse(readFileSync(path.resolve(scriptDirectory, "../package.json"), "utf8"));
const productVersion = desktopPackage.version;
const evidenceDirectory = path.join(repositoryRoot, "_local_m1", `v${productVersion}`);
const evidencePath = path.join(evidenceDirectory, "remote-permissions-smoke.json");
const rawLogPath = path.join(evidenceDirectory, "remote-permissions-smoke.log");
const logLines = [];

function log(event, detail = {}) {
  const entry = { at: new Date().toISOString(), event, ...detail };
  logLines.push(JSON.stringify(entry));
  console.log(JSON.stringify(entry));
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

async function waitFor(predicate, label, timeoutMs = 5_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate()) return;
    await delay(25);
  }
  throw new Error(`Timed out waiting for ${label}`);
}

async function listen(server) {
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  if (address === null || typeof address === "string") throw new Error("Smoke server did not bind a TCP port");
  return `http://127.0.0.1:${address.port}/`;
}

async function closeServer(server) {
  if (!server.listening) return;
  await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
}

const page = `<!doctype html>
<html><head><meta charset="utf-8"><title>VUA remote permission smoke</title></head>
<body><p>synthetic remote surface</p></body></html>`;

const server = http.createServer((_request, response) => {
  response.writeHead(200, {
    "content-type": "text/html; charset=utf-8",
    "content-security-policy": "default-src 'self' 'unsafe-inline'",
  });
  response.end(page);
});

let remoteWindow;
let applicationWindow;
let evidence = {
  schemaVersion: 1,
  productVersion,
  status: "failed",
  evidencePath,
  rawLogPath,
};

async function run() {
  try {
    log("process.start", { electronVersion: process.versions.electron });
    await app.whenReady();
  const remoteUrl = await listen(server);
  log("server.ready", { remoteUrl });

  const permissionRequests = [];
  const remoteSession = session.fromPartition(`vua-m1-remote-smoke-${process.pid}`, { cache: false });
  installPermissionDenyPolicy(remoteSession, (permission, requestingUrl) => {
    permissionRequests.push({ permission, requestingUrl });
    log("permission.denied", { permission, requestingUrl });
  });

  remoteWindow = new BrowserWindow({
    show: false,
    webPreferences: {
      ...localWindowWebPreferences(),
      backgroundThrottling: false,
      session: remoteSession,
    },
  });
  await remoteWindow.loadURL(remoteUrl);

  const remoteResults = await remoteWindow.webContents.executeJavaScript(`(async () => {
    const notifications = await Notification.requestPermission();
    const geolocation = await new Promise((resolve) => {
      navigator.geolocation.getCurrentPosition(
        () => resolve("granted"),
        (error) => resolve("denied:" + error.code),
        { timeout: 3000 }
      );
    });
    let media;
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      stream.getTracks().forEach((track) => track.stop());
      media = "granted";
    } catch (error) {
      media = "denied:" + error.name;
    }
    return {
      notifications,
      geolocation,
      media,
      vuaType: typeof window.vua,
      gatewayCallable: typeof window.vua?.gateway?.invoke === "function"
    };
  })()`, true);

  await waitFor(
    () => ["notifications", "geolocation", "media"].every(
      (permission) => permissionRequests.some((request) => request.permission === permission),
    ),
    "all permission requests",
  );
  assert.equal(remoteResults.notifications, "denied");
  assert.match(remoteResults.geolocation, /^denied:/);
  assert.match(remoteResults.media, /^denied:/);
  assert.equal(remoteResults.vuaType, "undefined");
  assert.equal(remoteResults.gatewayCallable, false);
  log("remote.boundary.passed", remoteResults);

  const localUrl = "data:text/html,<meta charset=utf-8><title>VUA local smoke</title>";
  const viewRedirects = [];
  const shellHandoffs = [];
  let blockedNavigation = false;
  applicationWindow = new BrowserWindow({
    show: false,
    webPreferences: { ...localWindowWebPreferences(), backgroundThrottling: false },
  });
  await applicationWindow.loadURL(localUrl);
  applicationWindow.webContents.on("will-navigate", (_event, url) => {
    if (url === remoteUrl) blockedNavigation = true;
  });
  installLocalContentNavigationPolicy(applicationWindow.webContents, {
    rendererUrl: localUrl,
    allowedOrigins: [remoteUrl],
    navigateCurrentView: (url) => {
      viewRedirects.push(url);
      log("view.redirect", { url });
    },
    openExternal: (url) => {
      shellHandoffs.push(url);
      log("shell.handoff", { url });
    },
    confirmNavigation: async () => true,
  });

  const windowCountBefore = BrowserWindow.getAllWindows().length;
  await applicationWindow.webContents.executeJavaScript(`window.open(${JSON.stringify(remoteUrl)}); true`, true);
  await waitFor(() => viewRedirects.length === 1, "embedded view redirect");
  assert.deepEqual(viewRedirects, [remoteUrl]);
  assert.equal(BrowserWindow.getAllWindows().length, windowCountBefore);

  await applicationWindow.webContents.executeJavaScript(`window.open("steam://open/friends"); true`, true);
  await waitFor(() => shellHandoffs.length === 1, "confirmed external protocol handoff");
  await delay(100);
  assert.deepEqual(shellHandoffs, ["steam://open/friends"]);
  assert.equal(BrowserWindow.getAllWindows().length, windowCountBefore);

  await applicationWindow.webContents.executeJavaScript(`setTimeout(() => { location.href = ${JSON.stringify(remoteUrl)}; }, 0); true`, true);
  await waitFor(() => blockedNavigation, "blocked remote navigation");
  await delay(100);
  assert.equal(applicationWindow.webContents.getURL(), localUrl);
  log("local.navigation.passed", { blockedNavigation, newWindowCreated: false });

  evidence = {
    schemaVersion: 1,
    productVersion,
    status: "passed",
    electronVersion: process.versions.electron,
    remoteOrigin: remoteUrl,
    permissionRequests,
    remoteResults,
    blockedNavigation,
    shellHandoffs,
    newWindowCreated: false,
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
    await closeServer(server);
    await mkdir(evidenceDirectory, { recursive: true });
    await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
    await writeFile(rawLogPath, `${logLines.join("\n")}\n`, "utf8");
    console.log(`Evidence: ${evidencePath}`);
    console.log(`Raw log: ${rawLogPath}`);
    if (applicationWindow && !applicationWindow.isDestroyed()) applicationWindow.destroy();
    if (remoteWindow && !remoteWindow.isDestroyed()) remoteWindow.destroy();
    app.exit(process.exitCode ?? 0);
  }
}

void run();
