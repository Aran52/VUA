// Operator for the explicit isolated installed-app diagnostic; no keys, URLs or source mutations.
import fs from "node:fs";
import path from "node:path";
import { randomBytes } from "node:crypto";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

function owned(directory) {
  if (!path.isAbsolute(directory)) throw Error("QA root must be absolute");
  const root = path.resolve(directory);
  if (fs.lstatSync(root).isSymbolicLink()) throw Error("QA root is a link");
  const marker = JSON.parse(fs.readFileSync(path.join(root, "vua-component-qa.json")));
  if (marker.schemaVersion !== "0.1" || marker.product !== "VUA" || marker.kind !== "component-update-qa" || !/^[a-f0-9]{32}$/.test(marker.runId)) throw Error("Unowned QA root");
  return { root, marker };
}
export function createQaRoot(directory) {
  if (!path.isAbsolute(directory)) throw Error("QA root must be absolute");
  const root = path.resolve(directory);
  if (fs.existsSync(root) && (fs.lstatSync(root).isSymbolicLink() || fs.readdirSync(root).length)) throw Error("Choose a new empty QA root");
  fs.mkdirSync(root, { recursive: true });
  const marker = { schemaVersion: "0.1", product: "VUA", kind: "component-update-qa", runId: randomBytes(16).toString("hex"), createdAt: new Date().toISOString() };
  fs.writeFileSync(path.join(root, "vua-component-qa.json"), JSON.stringify(marker, null, 2), { flag: "wx", mode: 0o600 });
  return { root, ...marker };
}
export function qaProfile(directory, name, initialize = false) {
  const { root, marker } = owned(directory);
  if (!/^[a-z][a-z0-9-]{0,31}$/.test(name)) throw Error("Invalid QA case name");
  const profiles = path.join(root, "profiles"), profile = path.join(profiles, name);
  for (const folder of [profiles, profile]) if (fs.existsSync(folder) && fs.lstatSync(folder).isSymbolicLink()) throw Error("QA profile is a link");
  if (initialize) {
    fs.mkdirSync(profile, { recursive: true });
    const config = path.join(profile, "component-update-qa.json");
    if (!fs.existsSync(config)) fs.writeFileSync(config, JSON.stringify({ schemaVersion: "0.1", runId: marker.runId }, null, 2), { flag: "wx", mode: 0o600 });
  }
  return { root, profile, runId: marker.runId };
}
export async function sendQa(directory, name, request, { dispatchOnly = false, timeoutMs = 180_000 } = {}) {
  const { profile } = qaProfile(directory, name); const id = `qa-${randomBytes(16).toString("hex")}.json`;
  const commands = path.join(profile, "component-update-commands"), replies = path.join(profile, "component-update-replies");
  if (!fs.existsSync(commands) || fs.lstatSync(commands).isSymbolicLink() || !fs.statSync(commands).isDirectory()) throw Error("Installed QA app is not ready");
  const file = path.join(commands, id); fs.writeFileSync(`${file}.pending`, JSON.stringify(request), { flag: "wx", mode: 0o600 }); fs.renameSync(`${file}.pending`, file);
  if (dispatchOnly) return { requestId: id, dispatched: true };
  const reply = path.join(replies, id); const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (fs.existsSync(reply)) { const value = JSON.parse(fs.readFileSync(reply)); if (!value.ok) throw Error(value.error); return { requestId: id, replyFile: reply, ...value }; }
    await delay(100);
  }
  throw Error(`Installed QA action timed out: ${request.method}`);
}
export async function waitQaReady(directory, name, { afterPid, hostBuild, timeoutMs = 90_000 } = {}) {
  const { profile, runId } = qaProfile(directory, name); const file = path.join(profile, "component-update-ready.json"); const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (fs.existsSync(file)) {
      const value = JSON.parse(fs.readFileSync(file));
      if (value.runId === runId && value.status === "ready" && Number.isSafeInteger(value.pid) && value.pid > 0
          && (!afterPid || value.pid !== afterPid) && (!hostBuild || value.host?.build === hostBuild)) {
        try { process.kill(value.pid, 0); return value; } catch {}
      }
    }
    await delay(100);
  }
  throw Error("Installed QA app did not become ready");
}
function request(method, first, second) {
  if (["check", "stage", "cancel", "apply", "restore"].includes(method)) {
    if (!["host", "amf"].includes(first)) throw Error("Choose host or amf");
    if (method === "stage" && !/^[a-f0-9]{64}$/.test(second ?? "")) throw Error("Stage requires a manifest digest");
    return { method, component: first, ...(method === "stage" ? { manifestId: second } : {}) };
  }
  if (method === "snapshot" || method === "capture" || method === "quit") return { method };
  if (method === "navigate") return { method, page: first };
  if (method === "set-enabled") { if (!["true", "false"].includes(first)) throw Error("Choose true or false"); return { method, enabled: first === "true" }; }
  if (method === "fail-ui") return { method, manifestId: first === "clear" ? null : first };
  if (method === "fail-host-health") return { method, build: first === "clear" ? null : first };
  throw Error("Unknown QA action");
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [operation, directory, name, method, first, second] = process.argv.slice(2);
  let value;
  if (operation === "init") value = createQaRoot(directory);
  else if (operation === "profile") value = qaProfile(directory, name, true);
  else if (operation === "ready") value = await waitQaReady(directory, name, {
    afterPid: Number(process.argv.find(arg => arg.startsWith("--after-pid="))?.slice(12)) || undefined,
    hostBuild: process.argv.find(arg => arg.startsWith("--host-build="))?.slice(13),
  });
  else if (operation === "send") value = await sendQa(directory, name, request(method, first, second), { dispatchOnly: process.argv.includes("--dispatch-only") });
  else throw Error("Use init, profile, ready or send with an explicit isolated QA root");
  console.log(JSON.stringify(value));
}
