// First-party packaged entry. Electron starts without an external application
// argument, so app.isPackaged and the normal production guards remain effective.
const { app, dialog } = require("electron");
const fs = require("original-fs");
const path = require("node:path");
const crypto = require("node:crypto");
const key = "__VUA_RELEASE_PUBLIC_KEY__";
const hash = bytes => crypto.createHash("sha256").update(bytes).digest("hex");
const smokeDirectory = process.argv.find(arg => arg.startsWith("--vua-smoke-test="))?.slice("--vua-smoke-test=".length);
function smokePhase(phase) {
  if (!smokeDirectory || !path.isAbsolute(smokeDirectory)) return;
  fs.mkdirSync(smokeDirectory, { recursive: true });
  fs.writeFileSync(path.join(smokeDirectory, "bootstrap-progress.json"), JSON.stringify({ phase, at: new Date().toISOString() }));
}
function owned(root, relative) {
  if (!relative || relative.includes("\\") || relative.split("/").some(part => !part || part === "." || part === ".." || /[:\x00-\x1f]/.test(part))) throw Error("unsafe_release_path");
  let target = root;
  for (const part of relative.split("/")) {
    target = path.join(target, part);
    if (fs.existsSync(target) && fs.lstatSync(target).isSymbolicLink()) throw Error("release_path_is_link");
  }
  return target;
}
function manifest(root, id) {
  if (!/^[0-9a-f]{64}$/.test(id)) throw Error("invalid_release_identity");
  const raw = fs.readFileSync(owned(root, `manifests/${id}.json`));
  if (raw.length > 4194304 || hash(raw) !== id) throw Error("invalid_release_identity");
  const envelope = JSON.parse(raw);
  const publicKey = crypto.createPublicKey({ key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), Buffer.from(key, "hex")]), format: "der", type: "spki" });
  if (envelope.schemaVersion !== "0.1" || envelope.algorithm !== "ed25519" || !/^[0-9a-f]{128}$/.test(envelope.signature)
      || !crypto.verify(null, Buffer.from(envelope.payload), publicKey, Buffer.from(envelope.signature, "hex"))) throw Error("invalid_release_signature");
  const result = JSON.parse(envelope.payload);
  if (result.schemaVersion !== "0.1" || result.product !== "VUA" || result.platform !== "windows" || result.arch !== "x64" || result.compatibility.uiAbi !== "amf-ui/0.1") throw Error("unsupported_release");
  return result;
}
function verifyFiles(root, files) {
  for (const file of files) {
    const target = owned(root, file.path);
    if (fs.statSync(target).size !== file.bytes || hash(fs.readFileSync(target)) !== file.sha256) throw Error("installed_files_need_repair");
  }
}
try {
  smokePhase("bootstrap-entered");
  if (!app.isPackaged || process.defaultApp) throw Error("invalid_packaged_identity");
  const root = path.resolve(process.resourcesPath, "../../..");
  for (let directory = root; directory !== path.dirname(directory); directory = path.dirname(directory)) {
    if (fs.lstatSync(directory).isSymbolicLink()) throw Error("install_root_is_link");
  }
  const active = JSON.parse(fs.readFileSync(owned(root, "active.json")));
  if (active.schemaVersion !== "0.1") throw Error("unsupported_installed_layout");
  const host = manifest(root, active.hostManifest); const amf = manifest(root, active.amfManifest);
  for (const value of [host.host.pack.sha256, host.runtime.archive.sha256, amf.amf.pack.sha256]) if (!/^[0-9a-f]{64}$/.test(value)) throw Error("invalid_payload_identity");
  const hostRoot = owned(root, `payloads/host/${host.host.pack.sha256}`);
  const amfRoot = owned(root, `payloads/amf/${amf.amf.pack.sha256}`);
  const runtimeRoot = owned(root, `runtime/${host.runtime.archive.sha256}`);
  if (path.resolve(process.resourcesPath) !== path.join(runtimeRoot, "resources")) throw Error("invalid_runtime_identity");
  verifyFiles(hostRoot, host.host.files);
  smokePhase("host-files-verified");
  let amfAvailable = true;
  try { verifyFiles(amfRoot, amf.amf.files); } catch { amfAvailable = false; }
  smokePhase("amf-files-checked");
  app.setName("VUA"); app.setAppUserModelId("org.vua-project.desktop");
  Object.defineProperty(globalThis, "__VUA_DISTRIBUTION_V01__", { writable: false, configurable: false,
    value: Object.freeze({ schemaVersion: "0.1", root, hostRoot, amfRoot, runtimeRoot, host, amf, amfAvailable, hostManifestId: active.hostManifest, amfManifestId: active.amfManifest }) });
  smokePhase("main-entry-loading");
  require(owned(hostRoot, "dist/electron/main.js"));
} catch (error) {
  const smoke = process.argv.find(arg => arg.startsWith("--vua-smoke-test="))?.slice("--vua-smoke-test=".length);
  if (smoke && path.isAbsolute(smoke)) {
    fs.mkdirSync(smoke, { recursive: true });
    fs.writeFileSync(path.join(smoke, "packaged-smoke.json"), JSON.stringify({ status: "failed", scope: "Packaged bootstrap", error: error.message }));
  } else dialog.showErrorBox("VUA", "VUA could not start. Run the VUA installer again to repair program files. Personal data will be retained.");
  app.exit(1);
}
