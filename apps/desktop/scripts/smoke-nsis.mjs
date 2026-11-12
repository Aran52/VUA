import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const build = JSON.parse(fs.readFileSync(path.join(desktop, "out/distribution/latest-build.json"))).output;
const metadata = JSON.parse(fs.readFileSync(path.join(build, "build.json")));
const directory = path.join(desktop, "out/nsis-smoke", String(Date.now())); fs.mkdirSync(directory, { recursive: true });
const root = path.join(directory, "installed"); const profile = path.join(directory, "profile");
const prefix = `VUA-${metadata.version}-windows-x64-${process.argv.includes("--preview") ? "preview-" : ""}`;
const digest = file => createHash("sha256").update(fs.readFileSync(file)).digest("hex");
const installers = ["setup", "offline"].map(kind => {
  const name = `${prefix}${kind}.exe`, file = path.join(build, name);
  return { name, bytes: fs.statSync(file).size, sha256: digest(file) };
});
let phase = "offline-install";
const summaryIdentity = { version: metadata.version, amfVersion: metadata.amfVersion, sourceCommit: metadata.build,
  manifestSha256: digest(path.join(build, "vua-release.json")), installers, cleanMachineAcceptance: false };
function run(file, args, timeout = 90000, extra = {}) {
  const result = spawnSync(file, args, { windowsHide: true, encoding: "utf8", timeout, maxBuffer: 4 * 1024 * 1024, ...extra });
  if (result.error || result.status !== 0) {
    // Raw streams remain in the owned local diagnostic directory. CI publishes
    // only the scoped source/phase/result summary below.
    fs.writeFileSync(path.join(directory, "failed-process.txt"), `${result.stdout ?? ""}\n${result.stderr ?? ""}`);
    throw Error(`NSIS diagnostic failed: ${path.basename(file)} (${result.error?.code ?? result.status})`);
  }
  return result.stdout;
}
function installedFootprint(targetRoot) {
  const seenHardLinks = new Set();
  let files = 0, bytes = 0, logicalBytes = 0;
  function walk(current) {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (entry.isFile()) {
        files += 1;
        const stat = fs.statSync(full, { bigint: true });
        const size = Number(stat.size);
        logicalBytes += size;
        if (stat.nlink > 1n && stat.ino !== 0n) {
          const id = `${stat.dev}:${stat.ino}`;
          if (seenHardLinks.has(id)) continue;
          seenHardLinks.add(id);
        }
        bytes += size;
      }
    }
  }
  walk(targetRoot);
  return { files, bytes, logicalBytes };
}
function readStartupTiming(profileDir) {
  const launcher = JSON.parse(fs.readFileSync(path.join(profileDir, "launcher-timing.json"), "utf8"));
  const bootstrap = JSON.parse(fs.readFileSync(path.join(profileDir, "bootstrap-progress.json"), "utf8"));
  const { launcherStartedAtMs, electronSpawnedAtMs } = launcher;
  const { bootstrapCompletedAtMs } = bootstrap;
  if (![launcherStartedAtMs, electronSpawnedAtMs, bootstrapCompletedAtMs].every(Number.isSafeInteger)) throw Error("Missing launcher/bootstrap timing");
  return {
    launcherStartedAt: new Date(launcherStartedAtMs).toISOString(),
    electronSpawnedAt: new Date(electronSpawnedAtMs).toISOString(),
    bootstrapCompletedAt: new Date(bootstrapCompletedAtMs).toISOString(),
    launcherVerifyMs: electronSpawnedAtMs - launcherStartedAtMs,
    bootstrapVerifyMs: bootstrapCompletedAtMs - electronSpawnedAtMs,
    totalPreMainMs: bootstrapCompletedAtMs - launcherStartedAtMs,
  };
}
try {
run(path.join(build, `${prefix}offline.exe`), ["/S", "/ISOLATED", `/D=${root}`]);
if (!fs.existsSync(path.join(root, "nsis-isolated"))) throw Error("Installer did not isolate registry/shortcuts");
const cacheEntries = fs.existsSync(path.join(root, "cache")) ? fs.readdirSync(path.join(root, "cache")) : [];
if (cacheEntries.some(name => name.endsWith(".zip") || name.endsWith(".bin"))) throw Error(`Temporary transfer archive retained in cache: ${cacheEntries.join(",")}`);
const installed = installedFootprint(root);
phase = "offline-launch";
run(path.join(root, "VUA.exe"), ["launch", `--vua-smoke-test=${profile}`]);
if (fs.existsSync(path.join(root, "cache/launch-verified.json"))) throw Error("Single-use launch receipt was not consumed");
const result = JSON.parse(fs.readFileSync(path.join(profile, "packaged-smoke.json")));
if (result.status !== "passed" || !result.packagedIdentity || !result.amfPageMounted || !result.nativeDriveNavigation) throw Error("Installed host/AMF bootstrap failed");
const offlineLaunchTiming = readStartupTiming(profile);
phase = "direct-runtime-launch";
const runtimeDirs = fs.readdirSync(path.join(root, "runtime"));
fs.mkdirSync(path.join(root, "cache"), { recursive: true });
fs.writeFileSync(path.join(root, "cache/launch-verified.json"), JSON.stringify({ schemaVersion: "0.1", tokenSha256: "0".repeat(64) }));
const cleanEnv = Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith("ELECTRON_") && k !== "NODE_OPTIONS" && k !== "NODE_PATH" && !k.startsWith("VUA_")));
run(path.join(root, "runtime", runtimeDirs[0], "VUA.Runtime.exe"), [`--vua-smoke-test=${profile}`], 90000, { cwd: root, env: { ...cleanEnv, VUA_LAUNCH_VERIFIED: "f".repeat(64) } });
if (fs.existsSync(path.join(root, "cache/launch-verified.json"))) throw Error("Invalid launch receipt was not removed on direct launch");
const directResult = JSON.parse(fs.readFileSync(path.join(profile, "packaged-smoke.json")));
if (directResult.status !== "passed") throw Error("Direct VUA.Runtime.exe bootstrap failed");
// Same runtime, deliberately no runtime.zip in the second installer. This proves
// a component-aware repeat install can complete without fetching a new runtime.
phase = "online-runtime-reuse";
run(path.join(build, `${prefix}setup.exe`), ["/S", "/ISOLATED", `/D=${root}`]);
phase = "profile-replay";
run(path.join(root, "VUA.exe"), ["launch", `--vua-smoke-test=${profile}`]);
const replay = JSON.parse(fs.readFileSync(path.join(profile, "packaged-smoke.json")));
if (replay.previousMarker !== "persisted") throw Error("Installer did not retain the isolated profile");
const replayLaunchTiming = readStartupTiming(profile);
phase = "owned-uninstall";
run(path.join(root, "Uninstall.exe"), ["/S", `_?=${root}`]);
if (fs.existsSync(path.join(root, "payloads")) || !fs.existsSync(path.join(profile, "host/tasks.db"))) throw Error("Uninstall ownership failed");
const summary = { status: "passed", version: metadata.version, amfVersion: metadata.amfVersion, offlineInstall: true, onlineReuse: true,
  sourceCommit: metadata.build, manifestSha256: digest(path.join(build, "vua-release.json")), installers, installed,
  startupTiming: { offlineLaunch: offlineLaunchTiming, profileReplay: replayLaunchTiming }, testedAt: new Date().toISOString(),
  packagedIdentity: true, nativeDriveNavigation: true, independentAmfPage: true, profileReplay: true, ownedUninstall: true, cleanMachineAcceptance: false };
fs.writeFileSync(path.join(desktop, "out/nsis-smoke.json"), JSON.stringify(summary, null, 2));
console.log(JSON.stringify(summary));
} catch (error) {
  const readPhase = name => {
    try { const value = JSON.parse(fs.readFileSync(path.join(profile, name))); return typeof value.phase === "string" && /^[a-z-]{1,50}$/.test(value.phase) ? value.phase : null; }
    catch { return null; }
  };
  const failed = { ...summaryIdentity, status: "failed", phase, startupPhase: readPhase("packaged-smoke.json"),
    bootstrapPhase: readPhase("bootstrap-progress.json"), testedAt: new Date().toISOString(), error: error.message };
  fs.writeFileSync(path.join(desktop, "out/nsis-smoke.json"), JSON.stringify(failed, null, 2));
  throw error;
}
