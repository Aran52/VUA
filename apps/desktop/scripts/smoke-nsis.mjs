import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const build = JSON.parse(fs.readFileSync(path.join(desktop, "out/distribution/latest-build.json"))).output;
const metadata = JSON.parse(fs.readFileSync(path.join(build, "build.json")));
const directory = path.join(desktop, "out/nsis-smoke", String(Date.now())); fs.mkdirSync(directory, { recursive: true });
const root = path.join(directory, "installed"); const profile = path.join(directory, "profile");
const prefix = `VUA-${metadata.version}-windows-x64-${process.argv.includes("--preview") ? "preview-" : ""}`;
function run(file, args, timeout = 90000) {
  const result = spawnSync(file, args, { windowsHide: true, encoding: "utf8", timeout, maxBuffer: 4 * 1024 * 1024 });
  if (result.error || result.status !== 0) throw Error(`NSIS diagnostic failed: ${path.basename(file)} (${result.status})`);
  return result.stdout;
}
run(path.join(build, `${prefix}offline.exe`), ["/S", "/ISOLATED", `/D=${root}`]);
if (!fs.existsSync(path.join(root, "nsis-isolated"))) throw Error("Installer did not isolate registry/shortcuts");
run(path.join(root, "VUA.exe"), ["launch", `--vua-smoke-test=${profile}`]);
const result = JSON.parse(fs.readFileSync(path.join(profile, "packaged-smoke.json")));
if (result.status !== "passed" || !result.packagedIdentity || !result.amfPageMounted) throw Error("Installed host/AMF bootstrap failed");
// Same runtime, deliberately no runtime.zip in the second installer. This proves
// a component-aware repeat install can complete without fetching a new runtime.
run(path.join(build, `${prefix}setup.exe`), ["/S", "/ISOLATED", `/D=${root}`]);
run(path.join(root, "VUA.exe"), ["launch", `--vua-smoke-test=${profile}`]);
const replay = JSON.parse(fs.readFileSync(path.join(profile, "packaged-smoke.json")));
if (replay.previousMarker !== "persisted") throw Error("Installer did not retain the isolated profile");
run(path.join(root, "Uninstall.exe"), ["/S", `_?=${root}`]);
if (fs.existsSync(path.join(root, "payloads")) || !fs.existsSync(path.join(profile, "host/tasks.db"))) throw Error("Uninstall ownership failed");
const summary = { status: "passed", version: metadata.version, amfVersion: metadata.amfVersion, offlineInstall: true, onlineReuse: true,
  packagedIdentity: true, independentAmfPage: true, profileReplay: true, ownedUninstall: true, cleanMachineAcceptance: false };
fs.writeFileSync(path.join(desktop, "out/nsis-smoke.json"), JSON.stringify(summary, null, 2));
console.log(JSON.stringify(summary));
