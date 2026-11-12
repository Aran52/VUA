import { createHash, createPrivateKey, createPublicKey, sign, verify } from "node:crypto";
import { isDeepStrictEqual } from "node:util";
import semver from "semver";

export const MAX_METADATA_BYTES = 4 * 1024 * 1024;
export const COMPONENTS = ["host", "amf"];
export const digest = bytes => createHash("sha256").update(bytes).digest("hex");
const hash = (value, bytes) => typeof value === "string" && new RegExp(`^[a-f0-9]{${bytes * 2}}$`).test(value);
const integer = (value, min, max) => Number.isSafeInteger(value) && value >= min && value <= max;
const fail = code => { throw Error(code); };
function keys(value, required, optional = []) {
  if (!value || typeof value !== "object" || Array.isArray(value)
      || required.some(key => !Object.hasOwn(value, key))
      || Object.keys(value).some(key => !required.includes(key) && !optional.includes(key))) fail("invalid_update_fields");
}
export function validVersion(value) {
  return typeof value === "string" && /^[0-9]+\.[0-9]+\.[0-9]+(?:-[A-Za-z0-9.-]+)?(?:\+[A-Za-z0-9.-]+)?$/.test(value) && semver.valid(value) !== null;
}
export function httpsSource(value) {
  try { if (typeof value !== "string" || !value.isWellFormed()) return false; const u = new URL(value); return u.protocol === "https:" && !!u.hostname && !u.username && !u.password && !u.hash; }
  catch { return false; }
}
const indexSource = value => httpsSource(value) && [...value].length <= 2048;
function rustRange(value) {
  if (typeof value !== "string" || !value.length) fail("invalid_host_range");
  const parts = value.split(",").map(part => part.trim());
  if (parts.some(part => !/^(?:(?:>=|<=|>|<|=|~|\^)\s*)?(?:\*|[0-9]+(?:\.(?:[0-9]+|[xX*])){0,2}(?:-[A-Za-z0-9.-]+)?(?:\+[A-Za-z0-9.-]+)?)$/.test(part))) fail("invalid_host_range");
  const result = parts.map(part => /^[0-9]/.test(part) && !/[xX*]/.test(part) ? `^${part}` : part).join(" ");
  if (semver.validRange(result) === null) fail("invalid_host_range");
  return result;
}
function compatibility(value) {
  keys(value, ["host", "uiAbi", "react", "application", "bridge", "bdlWrite", "bdlRead"]);
  rustRange(value.host);
  if (value.uiAbi !== "amf-ui/0.1" || value.application !== "0.1" || !validVersion(value.react)
      || typeof value.bridge !== "string" || !value.bridge.length || typeof value.bdlWrite !== "string" || !value.bdlWrite.length
      || !Array.isArray(value.bdlRead) || !value.bdlRead.length || value.bdlRead.length > 32
      || value.bdlRead.some(v => typeof v !== "string" || !v.length) || new Set(value.bdlRead).size !== value.bdlRead.length
      || !value.bdlRead.includes(value.bdlWrite)) fail("invalid_component_compatibility");
}
const EMPTY_SHA256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

function metadataObject(value) {
  keys(value, ["sha256", "bytes", "sources"]);
  if (!hash(value.sha256, 32) || !integer(value.bytes, 1, MAX_METADATA_BYTES)
      || !Array.isArray(value.sources) || !value.sources.length || value.sources.length > 4
      || !value.sources.every(indexSource)) fail("invalid_component_manifest");
}

export function validateIndex(index, expectedSchema = "0.2") {
  if (!["0.2", "0.3"].includes(expectedSchema)) fail("unsupported_component_feed");
  keys(index, ["schemaVersion", "product", "platform", "arch", "channel", "component", "sequence", "releases"]);
  if (index.schemaVersion !== expectedSchema || index.product !== "VUA" || index.platform !== "windows" || index.arch !== "x64"
      || index.channel !== "beta" || !COMPONENTS.includes(index.component) || !integer(index.sequence, 1, Number.MAX_SAFE_INTEGER)
      || !Array.isArray(index.releases) || index.releases.length > 64) fail("unsupported_component_feed");
  const versions = new Set();
  const optionalReleaseKeys = expectedSchema === "0.3" ? ["runtimeVersion", "compressedTransfer"] : ["runtimeVersion"];
  for (const entry of index.releases) {
    keys(entry, ["version", "build", "releaseUrl", "manifest", "compatibility", "activation"], optionalReleaseKeys);
    if (!validVersion(entry.version) || !hash(entry.build, 20) || !indexSource(entry.releaseUrl)
        || !["restart", "reload"].includes(entry.activation)) fail("invalid_component_release");
    const precedence = semver.valid(entry.version);
    if (versions.has(precedence)) fail("duplicate_component_version");
    versions.add(precedence);
    metadataObject(entry.manifest);
    if (Object.hasOwn(entry, "compressedTransfer")) metadataObject(entry.compressedTransfer);
    compatibility(entry.compatibility);
    if (index.component === "host") {
      if (entry.activation !== "restart" || !validVersion(entry.runtimeVersion)
          || !semver.satisfies(entry.version, rustRange(entry.compatibility.host))) fail("invalid_host_release");
    } else if (Object.hasOwn(entry, "runtimeVersion")) fail("amf_must_not_select_runtime");
  }
  return index;
}
export function validateIndexV03(index) {
  return validateIndex(index, "0.3");
}
function publicKey(value) {
  if (!hash(value, 32)) fail("invalid_trust_key");
  return createPublicKey({ key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), Buffer.from(value, "hex")]), type: "spki", format: "der" });
}
export function openEnvelope(raw, expectedSchema, trustedKey) {
  if (!Buffer.isBuffer(raw) || raw.length > MAX_METADATA_BYTES) fail("metadata_too_large");
  const envelope = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(raw));
  keys(envelope, ["schemaVersion", "algorithm", "payload", "signature"]);
  if (envelope.schemaVersion !== expectedSchema || envelope.algorithm !== "ed25519" || typeof envelope.payload !== "string"
      || !hash(envelope.signature, 64) || !verify(null, Buffer.from(envelope.payload), publicKey(trustedKey), Buffer.from(envelope.signature, "hex"))) fail("invalid_release_signature");
  return JSON.parse(envelope.payload);
}
export function signingKey(privateKey, trustedKey) {
  const key = privateKey?.type === "private" ? privateKey : createPrivateKey(privateKey);
  if (createPublicKey(key).export({ type: "spki", format: "der" }).subarray(-32).toString("hex") !== trustedKey) fail("release_key_differs_from_shipped_anchor");
  return key;
}
export function sealEnvelope(payload, schema, privateKey, trustedKey) {
  const key = signingKey(privateKey, trustedKey);
  const text = JSON.stringify(payload);
  const raw = Buffer.from(JSON.stringify({ schemaVersion: schema, algorithm: "ed25519", payload: text, signature: sign(null, Buffer.from(text), key).toString("hex") }));
  if (raw.length > MAX_METADATA_BYTES) fail("metadata_too_large");
  return raw;
}
export function verifyIndex(raw, trustedKey, component, minimumSequence = 0) {
  const index = validateIndex(openEnvelope(raw, "0.2", trustedKey), "0.2");
  if (index.component !== component) fail("wrong_component_feed");
  if (index.sequence < minimumSequence) fail("component_feed_replayed");
  return index;
}
export function verifyIndexV03(raw, trustedKey, component, minimumSequence = 0) {
  const index = validateIndex(openEnvelope(raw, "0.3", trustedKey), "0.3");
  if (index.component !== component) fail("wrong_component_feed");
  if (index.sequence < minimumSequence) fail("component_feed_replayed");
  return index;
}
export function relativeName(value) {
  return typeof value === "string" && value.length > 0 && Buffer.byteLength(value) <= 240 && !/[\\:\x00-\x1f\x7f?*<>|"]/.test(value)
    && value.split("/").every(part => part && part !== "." && part !== ".." && !/[ .]$/.test(part)
      && !/^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$/i.test(part.split(".")[0]));
}
function pack(value) {
  keys(value, ["bytes", "sha256", "sources"]);
  if (!integer(value.bytes, 1, 2 * 1024 ** 3) || !hash(value.sha256, 32) || !Array.isArray(value.sources)
      || !value.sources.length || value.sources.length > 4 || !value.sources.every(httpsSource)) fail("invalid_release_source");
}
function files(value, bytes, required) {
  if (!Array.isArray(value) || !value.length || value.length > 20_000) fail("invalid_release_inventory");
  let end = 0; const names = new Set();
  for (const file of value) {
    keys(file, ["path", "bytes", "sha256", "offset"]);
    const name = typeof file.path === "string" ? file.path.replace(/[A-Z]/g, c => c.toLowerCase()) : "";
    if (!relativeName(file.path) || names.has(name) || !integer(file.bytes, 0, 512 * 1024 ** 2)
        || !integer(file.offset, 0, 2 * 1024 ** 3) || !hash(file.sha256, 32)) fail("invalid_release_file");
    names.add(name);
    if (bytes !== undefined) {
      if (file.offset !== end || file.offset + file.bytes > bytes) fail("invalid_release_range");
      end += file.bytes;
    }
  }
  if (bytes !== undefined && end !== bytes) fail("incomplete_release_pack");
  if (required.some(name => !value.some(file => file.path === name))) fail("incomplete_release_layout");
}
export function validateInventory(value) {
  keys(value, ["schemaVersion", "product", "platform", "arch", "releaseVersion", "releaseUrl", "runtime", "host", "amf", "compatibility"]);
  if (value.schemaVersion !== "0.1" || value.product !== "VUA" || value.platform !== "windows" || value.arch !== "x64"
      || !validVersion(value.releaseVersion) || !httpsSource(value.releaseUrl)) fail("unsupported_release");
  compatibility(value.compatibility);
  keys(value.runtime, ["version", "archive", "files"]);
  if (!validVersion(value.runtime.version)) fail("invalid_runtime_version");
  pack(value.runtime.archive);
  files(value.runtime.files, undefined, ["electron.exe", "resources/default_app.asar", "LICENSE", "LICENSES.chromium.html"]);
  for (const component of COMPONENTS) {
    const payload = value[component]; keys(payload, ["version", "build", "files", "pack"]);
    if (!validVersion(payload.version) || !hash(payload.build, 20)) fail("invalid_component_identity");
    pack(payload.pack);
    files(payload.files, payload.pack.bytes, component === "host"
      ? ["dist/electron/main.js", "dist/electron/preload.js", "dist/renderer/index.html", "provider/vua-orchestrator-provider.exe", "bootstrap/app.asar"]
      : ["ui/amf-entry.js", "ui/style.css", "provider/vua-amf-provider.exe"]);
  }
  if (value.host.version !== value.releaseVersion || !semver.satisfies(value.host.version, rustRange(value.compatibility.host))) fail("incompatible_host");
  return value;
}
export function verifyInventory(raw, trustedKey) {
  return validateInventory(openEnvelope(raw, "0.1", trustedKey));
}
export function bindInventory(entry, component, raw, trustedKey) {
  if (raw.length !== entry.manifest.bytes || digest(raw) !== entry.manifest.sha256) fail("component_manifest_integrity_failed");
  const inventory = verifyInventory(raw, trustedKey);
  if (inventory[component].version !== entry.version || inventory[component].build !== entry.build
      || !isDeepStrictEqual(inventory.compatibility, entry.compatibility)
      || (component === "host" && inventory.runtime.version !== entry.runtimeVersion)) fail("component_manifest_identity_mismatch");
  return inventory;
}
export function validateTransfer(value) {
  keys(value, ["schemaVersion", "product", "platform", "arch", "component", "manifestSha256", "encoding", "pack", "files"]);
  if (value.schemaVersion !== "0.3" || value.product !== "VUA" || value.platform !== "windows" || value.arch !== "x64"
      || !COMPONENTS.includes(value.component) || !hash(value.manifestSha256, 32)
      || !["zstd", "gzip"].includes(value.encoding)) fail("unsupported_component_transfer");
  pack(value.pack);
  if (!Array.isArray(value.files) || !value.files.length || value.files.length > 20_000) fail("invalid_component_transfer_files");
  let offset = 0; const names = new Set();
  for (const file of value.files) {
    keys(file, ["path", "bytes", "sha256", "compressedOffset", "compressedBytes", "compressedSha256"]);
    const name = typeof file.path === "string" ? file.path.replace(/[A-Z]/g, c => c.toLowerCase()) : "";
    if (!relativeName(file.path) || names.has(name) || !integer(file.bytes, 0, 512 * 1024 ** 2)
        || !hash(file.sha256, 32) || !integer(file.compressedOffset, 0, 2 * 1024 ** 3)
        || !integer(file.compressedBytes, 0, 512 * 1024 ** 2) || !hash(file.compressedSha256, 32)
        || file.compressedOffset !== offset) fail("invalid_component_transfer_file");
    if (file.bytes === 0) {
      if (file.compressedBytes !== 0 || file.compressedSha256 !== EMPTY_SHA256) fail("invalid_component_transfer_file");
    } else if (file.compressedBytes === 0) fail("invalid_component_transfer_file");
    names.add(name);
    offset += file.compressedBytes;
    if (offset > value.pack.bytes) fail("invalid_component_transfer_file");
  }
  if (offset !== value.pack.bytes) fail("incomplete_component_transfer_pack");
  return value;
}
export function verifyTransfer(raw, trustedKey) {
  return validateTransfer(openEnvelope(raw, "0.3", trustedKey));
}
export function bindTransfer(entry, component, inventory, raw, trustedKey) {
  if (!entry.compressedTransfer) fail("missing_compressed_transfer");
  if (raw.length !== entry.compressedTransfer.bytes || digest(raw) !== entry.compressedTransfer.sha256) fail("component_transfer_integrity_failed");
  const transfer = verifyTransfer(raw, trustedKey);
  if (transfer.component !== component || transfer.manifestSha256 !== entry.manifest.sha256) fail("component_transfer_identity_mismatch");
  const expectedFiles = inventory[component].files;
  if (transfer.files.length !== expectedFiles.length
      || transfer.files.some((file, index) => file.path !== expectedFiles[index].path
        || file.bytes !== expectedFiles[index].bytes || file.sha256 !== expectedFiles[index].sha256)) fail("component_transfer_files_mismatch");
  return transfer;
}
export function appendRelease(previous, entry) {
  const expectedSchema = previous?.schemaVersion === "0.3" ? "0.3" : "0.2";
  validateIndex(previous, expectedSchema);
  validateIndex({ ...previous, releases: [entry] }, expectedSchema);
  const releases = [...previous.releases, entry];
  const next = { ...previous, sequence: previous.sequence + 1, releases };
  // Validate before truncation so a duplicate cannot disappear at the history boundary.
  validateIndex({ ...next, releases: releases.slice(0, 64) }, expectedSchema);
  if (previous.releases.some(old => semver.eq(old.version, entry.version))) fail("component_version_already_published");
  next.releases.sort((a, b) => semver.rcompare(a.version, b.version));
  next.releases = next.releases.slice(0, 64);
  return validateIndex(next, expectedSchema);
}
