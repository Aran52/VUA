import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { createHash, createPrivateKey, createPublicKey, sign } from "node:crypto";
import { canonicalReleaseUrl, componentSources } from "../distribution/release-sources.mjs";

const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const workspace = path.resolve(desktop, "../..");
const require = createRequire(path.join(desktop, "package.json"));
const builder = createRequire(require.resolve("electron-builder"));
const library = createRequire(builder.resolve("app-builder-lib"));
const electron = createRequire(require.resolve("electron"));
const pkg = JSON.parse(fs.readFileSync(path.join(desktop, "package.json")));
const trust = JSON.parse(fs.readFileSync(path.join(desktop, "distribution/trust.json")));
const amf = JSON.parse(fs.readFileSync(path.join(desktop, "distribution/amf-version.json")));
const git = args => execFileSync("git", args, { cwd: workspace, windowsHide: true, encoding: "utf8" }).trim();
const dirty = git(["status", "--porcelain"]) !== "";
if (dirty && !process.argv.includes("--preview")) throw Error("Commit and verify the release before preparing publication artifacts");
const sha256 = bytes => createHash("sha256").update(bytes).digest("hex");
const build = git(["rev-parse", "HEAD"]);
const output = path.join(desktop, "out/distribution", `${pkg.version}-${build.slice(0, 8)}-${Date.now()}`);
fs.mkdirSync(output, { recursive: true });
const hostRoot = path.join(output, "host"); const amfRoot = path.join(output, "amf");
function copy(source, target) { fs.mkdirSync(path.dirname(target), { recursive: true }); fs.cpSync(source, target, { recursive: true, dereference: false }); }
copy(path.join(desktop, "dist/packaged-electron"), path.join(hostRoot, "dist/electron"));
copy(path.join(desktop, "dist/components/host-renderer"), path.join(hostRoot, "dist/renderer"));
copy(path.join(desktop, "dist/components/amf-ui"), path.join(amfRoot, "ui"));
const native = path.join(workspace, "target/x86_64-pc-windows-msvc/release");
copy(path.join(native, "vua-orchestrator-provider.exe"), path.join(hostRoot, "provider/vua-orchestrator-provider.exe"));
copy(path.join(native, "vua-amf-provider.exe"), path.join(amfRoot, "provider/vua-amf-provider.exe"));
if (execFileSync(path.join(amfRoot, "provider/vua-amf-provider.exe"), ["--version"], { windowsHide: true, encoding: "utf8" }).trim() !== amf.version) throw Error("AMF UI and native versions disagree");
copy(path.join(native, "vua-distribution.exe"), path.join(output, "VUA.exe"));
for (const root of [hostRoot, amfRoot]) {
  copy(path.join(desktop, "out/release-notices"), path.join(root, "notices"));
  for (const name of ["LICENSE", "NOTICE", "THIRD_PARTY_NOTICES.md"]) copy(path.join(workspace, name), path.join(root, name));
}
const bootstrap = path.join(output, "bootstrap-source"); fs.mkdirSync(bootstrap);
fs.writeFileSync(path.join(bootstrap, "main.cjs"), fs.readFileSync(path.join(desktop, "distribution/bootstrap.cjs"), "utf8").replace("__VUA_RELEASE_PUBLIC_KEY__", trust.publicKey));
fs.writeFileSync(path.join(bootstrap, "package.json"), JSON.stringify({ name: "vua-desktop", productName: "VUA", version: pkg.version, main: "main.cjs", private: true }));
fs.mkdirSync(path.join(hostRoot, "bootstrap"), { recursive: true });
await library("@electron/asar").createPackage(bootstrap, path.join(hostRoot, "bootstrap/app.asar"));

export function inventory(root) {
  const files = [];
  const walk = relative => {
    for (const entry of fs.readdirSync(path.join(root, relative), { withFileTypes: true }).sort((a,b) => a.name.localeCompare(b.name, "en"))) {
      const name = [relative, entry.name].filter(Boolean).join("/");
      if (entry.isSymbolicLink()) throw Error("Release contains a symlink");
      if (entry.isDirectory()) walk(name);
      else if (entry.isFile()) { const bytes = fs.readFileSync(path.join(root, name)); files.push({ path: name, bytes: bytes.length, sha256: sha256(bytes), offset: 0 }); }
      else throw Error("Unsupported release file");
    }
  };
  walk(""); return files;
}
function pack(root, target) {
  const files = inventory(root); let offset = 0; const digest = createHash("sha256"); const fd = fs.openSync(target, "wx");
  try { for (const file of files) { const bytes = fs.readFileSync(path.join(root, file.path)); file.offset = offset; fs.writeSync(fd, bytes); digest.update(bytes); offset += file.bytes; } }
  finally { fs.closeSync(fd); }
  return { files, bytes: offset, sha256: digest.digest("hex") };
}
const hostPack = pack(hostRoot, path.join(output, "host.bin")); const amfPack = pack(amfRoot, path.join(output, "amf.bin"));
const electronVersion = JSON.parse(fs.readFileSync(electron.resolve("./package.json"))).version;
const archiveName = `electron-v${electronVersion}-win32-x64.zip`;
const expected = JSON.parse(fs.readFileSync(electron.resolve("./checksums.json")))[archiveName];
// The pinned npm package's authoritative checksum is supplied explicitly. Never
// use a mirror's checksum as the sole authority for mirrored executable bytes.
const archive = await electron("@electron/get").downloadArtifact({ version: electronVersion, artifactName: "electron", platform: "win32", arch: "x64", checksums: { [archiveName]: expected } });
const archiveBytes = fs.readFileSync(archive);
if (sha256(archiveBytes) !== expected) throw Error("Electron archive failed the pinned checksum");
copy(archive, path.join(output, "runtime.zip"));
const runtimeRoot = path.join(output, "runtime");
await electron("@electron-internal/extract-zip").extract(archive, { dir: runtimeRoot });
const releaseUrl = canonicalReleaseUrl(pkg.version);
const component = (version, data, name) => ({ version, build, files: data.files, pack: { bytes: data.bytes, sha256: data.sha256, sources: componentSources(pkg.version, name) } });
const manifest = { schemaVersion: "0.1", product: "VUA", platform: "windows", arch: "x64", releaseVersion: pkg.version, releaseUrl,
  runtime: { version: electronVersion, files: inventory(runtimeRoot), archive: { bytes: archiveBytes.length, sha256: expected,
    sources: [`https://npmmirror.com/mirrors/electron/${electronVersion}/${archiveName}`, `https://mirrors.huaweicloud.com/electron/${electronVersion}/${archiveName}`, `https://github.com/electron/electron/releases/download/v${electronVersion}/${archiveName}`] } },
  host: component(pkg.version, hostPack, `vua-host-${pkg.version}.bin`), amf: component(amf.version, amfPack, `vua-amf-${amf.version}.bin`),
  compatibility: Object.fromEntries(Object.entries(amf).filter(([key]) => key !== "version")) };
const privateKeyPath = process.env.VUA_RELEASE_KEY ?? path.join(process.env.USERPROFILE, ".codex/private/vua-release-ed25519.pem");
const privateKey = createPrivateKey(fs.readFileSync(privateKeyPath));
if (createPublicKey(privateKey).export({ type: "spki", format: "der" }).subarray(-32).toString("hex") !== trust.publicKey) throw Error("Release key differs from the shipped trust anchor");
const payload = JSON.stringify(manifest);
fs.writeFileSync(path.join(output, "vua-release.json"), JSON.stringify({ schemaVersion: "0.1", algorithm: "ed25519", payload, signature: sign(null, Buffer.from(payload), privateKey).toString("hex") }));
fs.writeFileSync(path.join(output, "build.json"), JSON.stringify({ version: pkg.version, amfVersion: amf.version, build, dirty, output }, null, 2));
fs.writeFileSync(path.join(desktop, "out/distribution/latest-build.json"), JSON.stringify({ output }));
console.log(JSON.stringify({ output, hostBytes: hostPack.bytes, amfBytes: amfPack.bytes, runtimeBytes: archiveBytes.length, electronVersion, amfVersion: amf.version }));
