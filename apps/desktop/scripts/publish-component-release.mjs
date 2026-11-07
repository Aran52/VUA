import fs from "node:fs";
import path from "node:path";
import { randomBytes } from "node:crypto";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { COMPONENTS, bindInventory, digest, verifyIndex } from "../distribution/component-index.mjs";
import { ownedPath, fileDigest, componentPage } from "./prepare-component-release.mjs";

const read = (root, name) => {
  const target = ownedPath(root, name);
  if (!fs.lstatSync(target).isFile()) throw Error("publication_file_not_regular");
  return fs.readFileSync(target);
};
function publicDirectory(root, relative) {
  ownedPath(root, relative);
  let directory = root;
  for (const part of relative.split("/")) {
    directory = path.join(directory, part);
    if (!fs.existsSync(directory)) fs.mkdirSync(directory, {mode: 0o755});
    if (!fs.lstatSync(directory).isDirectory()) throw Error("publication_directory_not_regular");
    fs.chmodSync(directory, 0o755);
  }
}
function ownedTree(root, name) {
  const target = ownedPath(root, name); const marker = path.join(target, ".vua-owned.json");
  if (!fs.existsSync(target)) publicDirectory(root, name);
  if (!fs.existsSync(marker)) {
    if (fs.readdirSync(target).length) throw Error("unowned_publication_tree");
    fs.writeFileSync(marker, JSON.stringify({ schemaVersion: "0.2", product: "VUA", tree: name }), { flag: "wx" });
  } else if (!isDeepStrictEqual(JSON.parse(read(root, `${name}/.vua-owned.json`)), { schemaVersion: "0.2", product: "VUA", tree: name })) throw Error("unowned_publication_tree");
  publicDirectory(root, name);
}
function immutableCopy(source, destination, expected, staging, webRoot) {
  if (fs.existsSync(destination)) {
    if (!isDeepStrictEqual(fileDigest(destination), expected)) throw Error("published_object_conflict");
    return;
  }
  publicDirectory(webRoot, path.relative(webRoot, path.dirname(destination)).split(path.sep).join("/"));
  const temporary = path.join(staging, `object-${randomBytes(12).toString("hex")}`);
  try {
    fs.copyFileSync(source, temporary, fs.constants.COPYFILE_EXCL);
    if (!isDeepStrictEqual(fileDigest(temporary), expected)) throw Error("publication_object_corrupt");
    fs.chmodSync(temporary, 0o644);
    const complete = fs.openSync(temporary, "a");
    try { fs.fsyncSync(complete); } finally { fs.closeSync(complete); }
    // A hard link makes a complete immutable object visible atomically without replacement.
    try { fs.linkSync(temporary, destination); }
    catch (error) {
      if (error.code !== "EEXIST" || !isDeepStrictEqual(fileDigest(destination), expected)) throw error;
    }
    if (process.platform !== "win32") {
      const directory = fs.openSync(path.dirname(destination), "r");
      try { fs.fsyncSync(directory); } finally { fs.closeSync(directory); }
    }
  } finally { if (fs.existsSync(temporary)) fs.unlinkSync(temporary); }
}

/** Run on the official filesystem (or an isolated test root), after private staging upload. */
export function publishComponentRelease(prepared, webRoot, publicKey, options = {}) {
  const record = JSON.parse(read(prepared, "publication.json"));
  const indexOnly = record.kind === "index";
  if (record.schemaVersion !== "0.2" || !COMPONENTS.includes(record.component) || (record.kind !== undefined && !["release", "index"].includes(record.kind))
      || (!indexOnly && !/^[a-f0-9]{64}$/.test(record.manifestSha256))) throw Error("invalid_publication_record");
  const channelPath = `updates/beta/${record.component}.json`;
  const rawIndex = read(prepared, channelPath);
  if (digest(rawIndex) !== record.indexSha256) throw Error("publication_index_corrupt");
  const index = verifyIndex(rawIndex, publicKey, record.component);
  const entry = indexOnly ? null : index.releases.find(entry => entry.manifest.sha256 === record.manifestSha256 && entry.version === record.version && entry.build === record.build);
  let manifestPath, rawManifest, pack, packPath, packFile, pagePath, pageFile;
  if (!indexOnly) {
  if (!entry) throw Error("publication_identity_mismatch");
  manifestPath = `updates/objects/${entry.manifest.sha256}.json`;
  rawManifest = read(prepared, manifestPath);
  const manifest = bindInventory(entry, record.component, rawManifest, publicKey);
  pack = manifest[record.component].pack;
  packPath = `updates/objects/${pack.sha256}.bin`;
  packFile = ownedPath(prepared, packPath);
  if (!fs.lstatSync(packFile).isFile() || !isDeepStrictEqual(fileDigest(packFile), { sha256: pack.sha256, bytes: pack.bytes })) throw Error("publication_pack_corrupt");
  if (entry.releaseUrl !== `https://ph-r.com/vua/components/${record.component}/v${encodeURIComponent(entry.version)}/`
      || entry.manifest.sources[0] !== `https://ph-r.com/vua/updates/objects/${entry.manifest.sha256}.json`
      || pack.sources[0] !== `https://ph-r.com/vua/updates/objects/${pack.sha256}.bin`) throw Error("publication_source_mismatch");
  pagePath = `components/${record.component}/v${entry.version}/index.html`;
  pageFile = ownedPath(prepared, pagePath);
  if (!read(prepared, pagePath).equals(Buffer.from(componentPage(record.component, entry)))) throw Error("publication_page_corrupt");
  }
  if (!fs.existsSync(webRoot)) {fs.mkdirSync(webRoot, { recursive: true, mode: 0o755 }); fs.chmodSync(webRoot, 0o755);}
  ownedTree(webRoot, "updates"); ownedTree(webRoot, "components");
  const lock = ownedPath(webRoot, `updates/.publish-${record.component}.lock`);
  let lease;
  try { lease = fs.openSync(lock, "wx"); }
  catch (error) { if (error.code === "EEXIST") throw Error("publication_busy_inspect_lock"); throw error; }
  let indexTemporary;
  try {
    fs.writeFileSync(lease, JSON.stringify({ component: record.component, sequence: index.sequence, pid: process.pid }));
    const live = ownedPath(webRoot, options.candidate ? `updates/candidates/${record.component}.json` : channelPath);
    const formal = ownedPath(webRoot, channelPath);
    const previous = fs.existsSync(formal) ? read(webRoot, channelPath) : null;
    const alreadyCommitted = fs.existsSync(live) && digest(fs.readFileSync(live)) === digest(rawIndex);
    if (!alreadyCommitted && (previous ? digest(previous) : null) !== record.previousIndexSha256) throw Error("publication_channel_changed");
    const before = previous ? verifyIndex(previous, publicKey, record.component) : null;
    if (!alreadyCommitted && index.sequence <= Math.max(before?.sequence ?? 0, options.minimumSequence ?? 0)) throw Error("publication_sequence_mismatch");
    if (before && before.releases.some(old => {
      const retained = index.releases.find(entry => entry.version === old.version);
      return retained && !isDeepStrictEqual(retained, old);
    })) throw Error("published_component_changed");
    if (indexOnly && index.releases.some(item => !before?.releases.some(old => isDeepStrictEqual(item, old)))) throw Error("index_revision_must_only_remove");
    const staging = ownedPath(webRoot, "updates/.staging"); fs.mkdirSync(staging, { recursive: true });
    if (!indexOnly) {
    immutableCopy(packFile, ownedPath(webRoot, packPath), { sha256: pack.sha256, bytes: pack.bytes }, staging, webRoot);
    immutableCopy(ownedPath(prepared, manifestPath), ownedPath(webRoot, manifestPath), { sha256: entry.manifest.sha256, bytes: entry.manifest.bytes }, staging, webRoot);
    immutableCopy(pageFile, ownedPath(webRoot, pagePath), fileDigest(pageFile), staging, webRoot);
    }
    // Validate every retained inventory/object too, using the frozen v1.0.0 rules.
    for (const item of index.releases) {
      const inventory = bindInventory(item, record.component, read(webRoot, `updates/objects/${item.manifest.sha256}.json`), publicKey);
      const object = inventory[record.component].pack;
      const objectFile = ownedPath(webRoot, `updates/objects/${object.sha256}.bin`);
      if (!fs.lstatSync(objectFile).isFile() || !isDeepStrictEqual(fileDigest(objectFile), {sha256: object.sha256, bytes: object.bytes})) throw Error("publication_pack_corrupt");
    }
    if (alreadyCommitted) return { component: record.component, sequence: index.sequence, unchanged: true };
    publicDirectory(webRoot, path.relative(webRoot, path.dirname(live)).split(path.sep).join("/"));
    indexTemporary = ownedPath(webRoot, `updates/.staging/index-${record.component}-${randomBytes(12).toString("hex")}.json`);
    const pending = fs.openSync(indexTemporary, "wx");
    try { fs.writeFileSync(pending, rawIndex); fs.fchmodSync(pending, 0o644); fs.fsyncSync(pending); } finally { fs.closeSync(pending); }
    fs.renameSync(indexTemporary, live); indexTemporary = undefined;
    if (process.platform !== "win32") {
      const directory = fs.openSync(path.dirname(live), "r");
      try { fs.fsyncSync(directory); } finally { fs.closeSync(directory); }
    }
    return { component: record.component, sequence: index.sequence, version: entry?.version ?? null, manifestSha256: entry?.manifest.sha256 ?? null, indexSha256: digest(rawIndex), candidate: !!options.candidate };
  } finally {
    if (indexTemporary && fs.existsSync(indexTemporary)) fs.unlinkSync(indexTemporary);
    fs.closeSync(lease); fs.unlinkSync(lock);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [prepared, webRoot] = process.argv.slice(2);
  if (!prepared || !webRoot) throw Error("Usage: publish-component-release.mjs <private-prepared-directory> <owned-vua-web-root>");
  const trust = JSON.parse(fs.readFileSync(new URL("../distribution/trust.json", import.meta.url)));
  console.log(JSON.stringify(publishComponentRelease(path.resolve(prepared), path.resolve(webRoot), trust.publicKey)));
}
