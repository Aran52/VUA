import fs from "node:fs";
import path from "node:path";
import { createHash, createPrivateKey } from "node:crypto";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { gzipSync, zstdCompressSync, constants as zlibConstants } from "node:zlib";
import { COMPONENTS, appendRelease, digest, relativeName, sealEnvelope, signingKey, validateIndex, validateIndexV03, validateInventory, validateTransfer, validVersion, verifyIndex, verifyIndexV03, verifyInventory } from "../distribution/component-index.mjs";
import semver from "semver";

export const OFFICIAL_UPDATES = "https://ph-r.com/vua/updates";
export function ownedPath(root, relative) {
  if (!path.isAbsolute(root) || !relativeName(relative)) throw Error("unsafe_publication_path");
  const absolute = path.resolve(root);
  if (absolute === path.parse(absolute).root) throw Error("invalid_publication_root");
  const rejectLink = target => {
    try { if (fs.lstatSync(target).isSymbolicLink()) throw Error("publication_path_is_link"); }
    catch (error) { if (error.code !== "ENOENT") throw error; }
  };
  for (let parent = absolute; ; parent = path.dirname(parent)) {
    rejectLink(parent);
    if (parent === path.dirname(parent)) break;
  }
  let target = absolute;
  for (const part of relative.split("/")) {
    target = path.join(target, part);
    rejectLink(target);
  }
  return target;
}
export function fileDigest(file) {
  const hash = createHash("sha256"); const fd = fs.openSync(file, "r"); const chunk = Buffer.alloc(1024 * 1024);
  let bytes = 0;
  try { for (let count; (count = fs.readSync(fd, chunk)) > 0;) { hash.update(chunk.subarray(0, count)); bytes += count; } }
  finally { fs.closeSync(fd); }
  return { sha256: hash.digest("hex"), bytes };
}
function inventory(root, component) {
  const names = [];
  function walk(relative = "") {
    const directory = relative ? ownedPath(root, relative) : root;
    if (!fs.statSync(directory).isDirectory() || fs.lstatSync(directory).isSymbolicLink()) throw Error("invalid_component_directory");
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const name = relative ? `${relative}/${entry.name}` : entry.name;
      const target = ownedPath(root, name);
      if (entry.isDirectory()) walk(name);
      else if (entry.isFile() && fs.lstatSync(target).isFile()) names.push(name);
      else throw Error("component_contains_non_regular_file");
    }
  }
  ownedPath(root, "component-root-check"); walk(); names.sort();
  const roots = component === "host" ? ["dist/", "provider/", "bootstrap/", "launcher/", "notices/"] : ["ui/", "provider/", "notices/"];
  const licenses = ["LICENSE", "NOTICE", "THIRD_PARTY_NOTICES.md"];
  if (!names.length || names.length > 20_000 || names.some(name => !roots.some(prefix => name.startsWith(prefix)) && !licenses.includes(name))) throw Error("unexpected_component_file");
  if (licenses.some(name => !names.includes(name)) || !names.some(name => name.startsWith("notices/"))) throw Error("component_notices_missing");
  const folded = new Set(); let offset = 0;
  return names.map(name => {
    const foldedName = name.replace(/[A-Z]/g, value => value.toLowerCase());
    if (folded.has(foldedName)) throw Error("duplicate_component_path"); folded.add(foldedName);
    const size = fs.statSync(ownedPath(root, name)).size;
    if (size > 512 * 1024 ** 2 || offset + size > 2 * 1024 ** 3) throw Error("component_pack_too_large");
    const file = { path: name, bytes: size, offset }; offset += size; return file;
  });
}
function write(root, relative, bytes) {
  const file = ownedPath(root, relative); fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, bytes, { flag: "wx" });
}
function escape(value) { return String(value).replace(/[&<>"']/g, char => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[char]); }
export function componentPage(component, entry) {
  return `<!doctype html><html lang="en"><meta charset="utf-8"><title>VUA ${escape(component)} ${escape(entry.version)}</title><main><h1>${escape(component === "host" ? "VUA" : "AMF")} ${escape(entry.version)}</h1><p>Build ${escape(entry.build)}. Update from inside VUA.</p><p>Manifest SHA-256: <code>${entry.manifest.sha256}</code></p><p><a href="/vua/downloads/">VUA installation and recovery</a></p></main></html>\n`;
}
export function componentProbeEnvironment() { return Object.fromEntries(Object.entries(process.env).filter(([name]) => !name.toUpperCase().startsWith("VUA_"))); }
export function verifyAmfIdentity(native, ui, expected) {
  if (native.schemaVersion !== "0.1" || ui.schemaVersion !== "0.1" || native.component !== "amf" || ui.component !== "amf"
      || native.version !== expected.version || ui.version !== expected.version || native.build !== expected.build || ui.build !== expected.build
      || native.bdlWrite !== expected.bdlWrite || ui.bdlWrite !== expected.bdlWrite || ui.uiAbi !== expected.uiAbi
      || native.serviceBuildId !== `amf/${expected.version}@${expected.build}` || ui.hotReloadAbi !== "amf-hot/0.1"
      || native.dataRecoveryAbi !== "amf-data/0.1" || ui.dataRecoveryAbi !== "amf-data/0.1") throw Error("AMF UI/provider source or data identity differs from this build");
}
export function verifyHostRuntimeIdentity(value, expected) {
  if (value.schemaVersion !== "0.1" || value.build !== expected.build || value.dataTrialAbi !== "amf-data/0.1") throw Error("Host data-trial identity differs from this build");
}
export function verifyBuildReceipt(componentRoot, receipt, expected) {
  if (receipt.schemaVersion !== "0.2" || receipt.component !== expected.component || receipt.version !== expected.version || receipt.build !== expected.build
      || receipt.dirty !== false || !Array.isArray(receipt.files)) throw Error("Component build receipt does not match the clean source revision");
  const actualNames = inventory(path.resolve(componentRoot), expected.component).map(file => file.path);
  if (actualNames.length !== receipt.files.length || new Set(receipt.files.map(file => file.path)).size !== receipt.files.length
      || receipt.files.some(file => !actualNames.includes(file.path) || !isDeepStrictEqual(fileDigest(ownedPath(path.resolve(componentRoot), file.path)), { sha256: file.sha256, bytes: file.bytes }))) throw Error("Component files differ from the build receipt");
  return receipt;
}

function compressFileBytes(bytes, encoding) {
  if (bytes.length === 0) return Buffer.alloc(0);
  if (encoding === "zstd") return zstdCompressSync(bytes, { params: { [zlibConstants.ZSTD_c_compressionLevel]: 19 } });
  if (encoding === "gzip") return gzipSync(bytes, { level: 9 });
  throw Error("unsupported_component_transfer");
}

function writeAllSync(fd, bytes) {
  let written = 0;
  while (written < bytes.length) {
    const result = fs.writeSync(fd, bytes, written, bytes.length - written);
    if (result <= 0) throw Error("component_write_failed");
    written += result;
  }
}

/** Prepare only one component, never rewrite a published object or the other channel. */
export function prepareComponentRelease({ component, version, build, componentRoot, baseline, previous, previousV03, output, privateKey, publicKey, compatibility, activation, sequence, retainVersions, compressionEncoding = "zstd" }) {
  if (!COMPONENTS.includes(component) || !validVersion(version) || !/^[a-f0-9]{40}$/.test(build)) throw Error("invalid_component_identity");
  if (!["zstd", "gzip"].includes(compressionEncoding)) throw Error("unsupported_component_transfer");
  const original = verifyInventory(baseline, publicKey);
  signingKey(privateKey, publicKey);
  const before = previous ? verifyIndex(previous, publicKey, component) : null;
  const beforeV03 = previousV03 ? verifyIndexV03(previousV03, publicKey, component) : null;
  if (before && beforeV03 && (before.sequence !== beforeV03.sequence || !isDeepStrictEqual(before.releases.map(r => r.version), beforeV03.releases.map(r => r.version)))) throw Error("publication_sequence_mismatch");
  const next = structuredClone(original);
  next.compatibility = compatibility ? structuredClone(compatibility) : next.compatibility;
  let files = inventory(path.resolve(componentRoot), component);
  const sourceVersion = component === "host" ? next.host.version : next.amf.version;
  if (semver.lt(version, sourceVersion) || (semver.eq(version, sourceVersion) && build !== original[component].build)) throw Error("component_version_must_advance");
  if (semver.eq(version, sourceVersion)) {
    const byName = new Map(files.map(file => [file.path, file]));
    if (files.length !== original[component].files.length || original[component].files.some(file => !byName.has(file.path))) throw Error("published_component_changed");
    files = original[component].files.map(file => byName.get(file.path));
    let offset = 0; for (const file of files) { file.offset = offset; offset += file.bytes; }
  }
  // The caller chooses a new component release explicitly; source baseline versions stay independent.
  if (before?.releases.some(entry => entry.version === version)) throw Error("component_version_already_published");
  if (component === "host") next.releaseVersion = version;
  fs.mkdirSync(output, { recursive: false });
  const temporary = ownedPath(output, "component.bin"); const fd = fs.openSync(temporary, "wx");
  const compressedTemporary = ownedPath(output, "component.pack"); const compressedFd = fs.openSync(compressedTemporary, "wx");
  const packHash = createHash("sha256"); const compressedPackHash = createHash("sha256");
  const compressedFiles = [];
  let total = 0; let compressedTotal = 0;
  try {
    for (const file of files) {
      const rawBytes = fs.readFileSync(ownedPath(path.resolve(componentRoot), file.path));
      if (rawBytes.length !== file.bytes) throw Error("component_changed_during_packaging");
      writeAllSync(fd, rawBytes);
      file.sha256 = digest(rawBytes);
      packHash.update(rawBytes);
      total += rawBytes.length;

      const compressedFrame = compressFileBytes(rawBytes, compressionEncoding);
      writeAllSync(compressedFd, compressedFrame);
      compressedPackHash.update(compressedFrame);
      compressedFiles.push({
        path: file.path,
        bytes: file.bytes,
        sha256: file.sha256,
        compressedOffset: compressedTotal,
        compressedBytes: compressedFrame.length,
        compressedSha256: digest(compressedFrame),
      });
      compressedTotal += compressedFrame.length;
    }
    fs.fsyncSync(fd);
    fs.fsyncSync(compressedFd);
  } finally {
    fs.closeSync(fd);
    fs.closeSync(compressedFd);
  }
  const packSha = packHash.digest("hex");
  const compressedPackSha = compressedPackHash.digest("hex");
  if (semver.eq(version, sourceVersion) && (packSha !== original[component].pack.sha256 || total !== original[component].pack.bytes)) throw Error("published_component_changed");
  next[component] = { version, build, files, pack: { bytes: total, sha256: packSha, sources: [`${OFFICIAL_UPDATES}/objects/${packSha}.bin`] } };
  validateInventory(next);
  const raw = sealEnvelope(next, "0.1", privateKey, publicKey);
  const manifestSha = digest(raw);
  const transferPayload = validateTransfer({
    schemaVersion: "0.3",
    product: "VUA",
    platform: "windows",
    arch: "x64",
    component,
    manifestSha256: manifestSha,
    encoding: compressionEncoding,
    pack: { bytes: compressedTotal, sha256: compressedPackSha, sources: [`${OFFICIAL_UPDATES}/objects/${compressedPackSha}.pack`] },
    files: compressedFiles,
  });
  const rawTransfer = sealEnvelope(transferPayload, "0.3", privateKey, publicKey);
  const transferSha = digest(rawTransfer);
  const pagePath = `components/${component}/v${version}/index.html`;
  const entry = { version, build, releaseUrl: `https://ph-r.com/vua/components/${component}/v${encodeURIComponent(version)}/`,
    manifest: { sha256: manifestSha, bytes: raw.length, sources: [`${OFFICIAL_UPDATES}/objects/${manifestSha}.json`] },
    compatibility: next.compatibility, activation: component === "host" || next.compatibility.bdlWrite !== original.compatibility.bdlWrite ? "restart" : activation ?? "reload",
    ...(component === "host" ? { runtimeVersion: next.runtime.version } : {}) };
  const entryV03 = {
    ...entry,
    compressedTransfer: { sha256: transferSha, bytes: rawTransfer.length, sources: [`${OFFICIAL_UPDATES}/objects/${transferSha}.json`] },
  };
  const index = before ? appendRelease(before, entry) : validateIndex({ schemaVersion: "0.2", product: "VUA", platform: "windows", arch: "x64", channel: "beta", component, sequence: 1, releases: [entry] });
  const seedV03 = beforeV03 ?? (before ? { ...structuredClone(before), schemaVersion: "0.3" } : null);
  const indexV03 = seedV03 ? appendRelease(seedV03, entryV03) : validateIndexV03({ schemaVersion: "0.3", product: "VUA", platform: "windows", arch: "x64", channel: "beta", component, sequence: 1, releases: [entryV03] });
  if (sequence !== undefined) {
    if (!Number.isSafeInteger(sequence) || sequence <= (before?.sequence ?? 0)) throw Error("publication_sequence_mismatch");
    index.sequence = sequence;
  }
  indexV03.sequence = index.sequence;
  if (retainVersions !== undefined) {
    if (!Array.isArray(retainVersions) || retainVersions.some(value => typeof value !== "string")) throw Error("invalid_retained_versions");
    index.releases = index.releases.filter(item => item.version === entry.version || retainVersions.includes(item.version));
    indexV03.releases = indexV03.releases.filter(item => item.version === entryV03.version || retainVersions.includes(item.version));
  }
  validateIndex(index);
  validateIndexV03(indexV03);
  const signedIndex = sealEnvelope(index, "0.2", privateKey, publicKey);
  const signedIndexV03 = sealEnvelope(indexV03, "0.3", privateKey, publicKey);
  const packPath = `updates/objects/${packSha}.bin`; const manifestPath = `updates/objects/${manifestSha}.json`;
  const compressedPackPath = `updates/objects/${compressedPackSha}.pack`; const transferManifestPath = `updates/objects/${transferSha}.json`;
  fs.mkdirSync(path.dirname(ownedPath(output, packPath)), { recursive: true });
  fs.copyFileSync(temporary, ownedPath(output, packPath), fs.constants.COPYFILE_EXCL);
  fs.unlinkSync(temporary);
  fs.copyFileSync(compressedTemporary, ownedPath(output, compressedPackPath), fs.constants.COPYFILE_EXCL);
  fs.unlinkSync(compressedTemporary);
  write(output, manifestPath, raw);
  write(output, transferManifestPath, rawTransfer);
  write(output, `updates/beta/${component}.json`, signedIndex);
  write(output, `updates/v0.3/beta/${component}.json`, signedIndexV03);
  write(output, pagePath, componentPage(component, entry));
  const record = { schemaVersion: "0.2", component, version, build, manifestSha256: manifestSha,
    previousIndexSha256: previous ? digest(previous) : null, indexSha256: digest(signedIndex), packSha256: packSha, packBytes: total, baselineComponentVersion: sourceVersion,
    v03: { previousIndexSha256: previousV03 ? digest(previousV03) : null, indexSha256: digest(signedIndexV03), transferManifestSha256: transferSha,
      compressedPackSha256: compressedPackSha, compressedPackBytes: compressedTotal, encoding: compressionEncoding } };
  write(output, "publication.json", JSON.stringify(record, null, 2) + "\n");
  return record;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [component, baselinePath, componentRoot, destination] = process.argv.slice(2);
  if (!COMPONENTS.includes(component) || !baselinePath || !componentRoot || !destination) throw Error("Usage: prepare-component-release.mjs <host|amf> <signed-baseline.json> <built-component-root> <new-output> [--previous=<signed-index.json>] [--previous-v03=<signed-v03-index.json>]");
  const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const workspace = path.resolve(desktop, "../..");
  const git = args => execFileSync("git", args, { cwd: workspace, windowsHide: true, encoding: "utf8" }).trim();
  if (git(["status", "--porcelain"])) throw Error("Commit and verify component code before preparing publication artifacts");
  const pkg = JSON.parse(fs.readFileSync(path.join(desktop, "package.json")));
  const amf = JSON.parse(fs.readFileSync(path.join(desktop, "distribution/amf-version.json")));
  const trust = JSON.parse(fs.readFileSync(path.join(desktop, "distribution/trust.json")));
  const baseline = fs.readFileSync(path.resolve(baselinePath)); const original = verifyInventory(baseline, trust.publicKey);
  const version = component === "host" ? pkg.version : amf.version;
  const receipt = JSON.parse(fs.readFileSync(path.join(path.dirname(path.resolve(componentRoot)), "component-build.json")));
  verifyBuildReceipt(path.resolve(componentRoot), receipt, { component, version, build: git(["rev-parse", "HEAD"]) });
  const provider = path.join(path.resolve(componentRoot), "provider", component === "host" ? "vua-orchestrator-provider.exe" : "vua-amf-provider.exe");
  if (component === "amf" && execFileSync(provider, ["--version"], { encoding: "utf8", windowsHide: true }).trim() !== version) throw Error("AMF provider and declared version disagree");
  if (component === "amf") {
    const native = JSON.parse(execFileSync(provider, ["--component-identity"], { encoding: "utf8", windowsHide: true, env: componentProbeEnvironment() }));
    const ui = JSON.parse(fs.readFileSync(path.join(path.resolve(componentRoot), "ui/component.json")));
    verifyAmfIdentity(native, ui, { ...amf, build: receipt.build });
  }
  if (component === "host") verifyHostRuntimeIdentity(JSON.parse(fs.readFileSync(path.join(path.resolve(componentRoot), "dist/renderer/update-runtime.json"))), { build: receipt.build });
  if (component === "host" && original.runtime.version !== pkg.devDependencies.electron) throw Error("Provide a signed baseline with the exact configured Electron runtime");
  const previousPath = process.argv.find(value => value.startsWith("--previous="))?.slice(11);
  const previousV03Path = process.argv.find(value => value.startsWith("--previous-v03="))?.slice(15);
  const sequenceArg = process.argv.find(value => value.startsWith("--sequence="))?.slice(11);
  const retainedArg = process.argv.find(value => value.startsWith("--retain="))?.slice(9);
  const privateKeyPath = process.env.VUA_RELEASE_KEY ?? path.join(process.env.USERPROFILE, ".codex/private/vua-release-ed25519.pem");
  const privateKey = createPrivateKey(fs.readFileSync(privateKeyPath));
  console.log(JSON.stringify(prepareComponentRelease({ component, version, build: git(["rev-parse", "HEAD"]), componentRoot: path.resolve(componentRoot), baseline,
    previous: previousPath ? fs.readFileSync(path.resolve(previousPath)) : undefined,
    previousV03: previousV03Path ? fs.readFileSync(path.resolve(previousV03Path)) : undefined,
    sequence: sequenceArg === undefined ? undefined : Number(sequenceArg),
    retainVersions: retainedArg === undefined ? undefined : retainedArg ? retainedArg.split(",") : [], output: path.resolve(destination), privateKey, publicKey: trust.publicKey,
    compatibility: component === "amf" ? Object.fromEntries(Object.entries(amf).filter(([key]) => key !== "version")) : undefined })));
}
