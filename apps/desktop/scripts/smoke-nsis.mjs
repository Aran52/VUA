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
function run(file, args, timeout = 90000) {
  const result = spawnSync(file, args, { windowsHide: true, encoding: "utf8", timeout, maxBuffer: 4 * 1024 * 1024 });
  if (result.error || result.status !== 0) {
    // Raw streams remain in the owned local diagnostic directory. CI publishes
    // only the scoped source/phase/result summary below.
    fs.writeFileSync(path.join(directory, "failed-process.txt"), `${result.stdout ?? ""}\n${result.stderr ?? ""}`);
    throw Error(`NSIS diagnostic failed: ${path.basename(file)} (${result.error?.code ?? result.status})`);
  }
  return result.stdout;
}
try {
run(path.join(build, `${prefix}offline.exe`), ["/S", "/ISOLATED", `/D=${root}`]);
if (!fs.existsSync(path.join(root, "nsis-isolated"))) throw Error("Installer did not isolate registry/shortcuts");
phase = "offline-launch";
run(path.join(root, "VUA.exe"), ["launch", `--vua-smoke-test=${profile}`]);
const result = JSON.parse(fs.readFileSync(path.join(profile, "packaged-smoke.json")));
if (result.status !== "passed" || !result.packagedIdentity || !result.amfPageMounted || !result.nativeDriveNavigation) throw Error("Installed host/AMF bootstrap failed");
// Same runtime, deliberately no runtime.zip in the second installer. This proves
// a component-aware repeat install can complete without fetching a new runtime.
phase = "online-runtime-reuse";
run(path.join(build, `${prefix}setup.exe`), ["/S", "/ISOLATED", `/D=${root}`]);
phase = "profile-replay";
run(path.join(root, "VUA.exe"), ["launch", `--vua-smoke-test=${profile}`]);
const replay = JSON.parse(fs.readFileSync(path.join(profile, "packaged-smoke.json")));
if (replay.previousMarker !== "persisted") throw Error("Installer did not retain the isolated profile");
phase = "owned-uninstall";
run(path.join(root, "Uninstall.exe"), ["/S", `_?=${root}`]);
if (fs.existsSync(path.join(root, "payloads")) || !fs.existsSync(path.join(profile, "host/tasks.db"))) throw Error("Uninstall ownership failed");
const summary = { status: "passed", version: metadata.version, amfVersion: metadata.amfVersion, offlineInstall: true, onlineReuse: true,
  sourceCommit: metadata.build, manifestSha256: digest(path.join(build, "vua-release.json")), installers, testedAt: new Date().toISOString(),
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
