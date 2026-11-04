import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { ownedPath, fileDigest, componentProbeEnvironment, verifyAmfIdentity, verifyHostRuntimeIdentity } from "./prepare-component-release.mjs";

const [component, destination, noticeDirectory] = process.argv.slice(2);
if (!["host", "amf"].includes(component) || !destination || !noticeDirectory) throw Error("Usage: assemble-component.mjs <host|amf> <new-output-directory> <fresh-notices-directory>");
const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const workspace = path.resolve(desktop, "../..");
const require = createRequire(path.join(desktop, "package.json"));
const pkg = JSON.parse(fs.readFileSync(path.join(desktop, "package.json")));
const amf = JSON.parse(fs.readFileSync(path.join(desktop, "distribution/amf-version.json")));
const trust = JSON.parse(fs.readFileSync(path.join(desktop, "distribution/trust.json")));
const git = args => execFileSync("git", args, { cwd: workspace, encoding: "utf8", windowsHide: true }).trim();
const build = git(["rev-parse", "HEAD"]);
const output = path.resolve(destination); ownedPath(output, "payload"); fs.mkdirSync(output, { recursive: false });
const payload = ownedPath(output, "payload"); fs.mkdirSync(payload);
const target = path.resolve(workspace, process.env.CARGO_TARGET_DIR ?? "target", "x86_64-pc-windows-msvc/release");
const copy = (source, relative) => {
  const destination = ownedPath(payload, relative); fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.cpSync(source, destination, { recursive: true, dereference: false, errorOnExist: true, force: false });
};
if (component === "amf") {
  copy(path.join(desktop, "dist/components/amf-ui"), "ui");
  copy(path.join(target, "vua-amf-provider.exe"), "provider/vua-amf-provider.exe");
  if (execFileSync(path.join(payload, "provider/vua-amf-provider.exe"), ["--version"], { encoding: "utf8", windowsHide: true }).trim() !== amf.version) throw Error("AMF UI and native versions disagree");
  const uiIdentity = JSON.parse(fs.readFileSync(path.join(payload, "ui/component.json")));
  const nativeIdentity = JSON.parse(execFileSync(path.join(payload, "provider/vua-amf-provider.exe"), ["--component-identity"], { encoding: "utf8", windowsHide: true, env: componentProbeEnvironment() }));
  verifyAmfIdentity(nativeIdentity, uiIdentity, { ...amf, build });
} else {
  copy(path.join(desktop, "dist/packaged-electron"), "dist/electron");
  copy(path.join(desktop, "dist/components/host-renderer"), "dist/renderer");
  verifyHostRuntimeIdentity(JSON.parse(fs.readFileSync(path.join(payload, "dist/renderer/update-runtime.json"))), { build });
  copy(path.join(target, "vua-orchestrator-provider.exe"), "provider/vua-orchestrator-provider.exe");
  copy(path.join(target, "vua-distribution.exe"), "launcher/VUA.exe");
  const bootstrap = ownedPath(output, "bootstrap-source"); fs.mkdirSync(bootstrap);
  fs.writeFileSync(path.join(bootstrap, "main.cjs"), fs.readFileSync(path.join(desktop, "distribution/bootstrap.cjs"), "utf8").replace("__VUA_RELEASE_PUBLIC_KEY__", trust.publicKey), { flag: "wx" });
  fs.writeFileSync(path.join(bootstrap, "package.json"), JSON.stringify({ name: "vua-desktop", productName: "Virtual Unified Assistant", version: pkg.version, main: "main.cjs", private: true }), { flag: "wx" });
  const builder = createRequire(require.resolve("electron-builder")); const library = createRequire(builder.resolve("app-builder-lib"));
  fs.mkdirSync(ownedPath(payload, "bootstrap"));
  await library("@electron/asar").createPackage(bootstrap, ownedPath(payload, "bootstrap/app.asar"));
}
copy(path.resolve(noticeDirectory), "notices");
for (const license of ["LICENSE", "NOTICE", "THIRD_PARTY_NOTICES.md"]) copy(path.join(workspace, license), license);
const files = [];
function walk(relative = "") {
  for (const entry of fs.readdirSync(relative ? ownedPath(payload, relative) : payload, { withFileTypes: true })) {
    const name = relative ? `${relative}/${entry.name}` : entry.name; const file = ownedPath(payload, name);
    if (entry.isDirectory()) walk(name);
    else if (entry.isFile()) files.push({ path: name, ...fileDigest(file) });
    else throw Error("Component output contains a link or special file");
  }
}
walk(); files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
if (git(["rev-parse", "HEAD"]) !== build) throw Error("Source revision changed during assembly");
const receipt = { schemaVersion: "0.2", component, version: component === "host" ? pkg.version : amf.version, build, dirty: git(["status", "--porcelain"]) !== "", files };
fs.writeFileSync(ownedPath(output, "component-build.json"), JSON.stringify(receipt, null, 2) + "\n", { flag: "wx" });
console.log(JSON.stringify({ component, version: receipt.version, build, dirty: receipt.dirty, files: files.length, bytes: files.reduce((n, file) => n + file.bytes, 0), output }));
