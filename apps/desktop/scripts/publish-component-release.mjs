import fs from "node:fs";
import path from "node:path";
import { randomBytes } from "node:crypto";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { COMPONENTS, bindInventory, bindTransfer, digest, verifyIndex, verifyIndexV03 } from "../distribution/component-index.mjs";
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
    if (!fs.existsSync(directory)) fs.mkdirSync(directory, { mode: 0o755 });
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

function atomicIndexWrite(webRoot, destination, rawBytes) {
  publicDirectory(webRoot, path.relative(webRoot, path.dirname(destination)).split(path.sep).join("/"));
  const temporary = ownedPath(webRoot, `updates/.staging/index-${randomBytes(12).toString("hex")}.json`);
  try {
    const pending = fs.openSync(temporary, "wx");
    try { fs.writeFileSync(pending, rawBytes); fs.fchmodSync(pending, 0o644); fs.fsyncSync(pending); } finally { fs.closeSync(pending); }
    fs.renameSync(temporary, destination);
  } finally {
    if (fs.existsSync(temporary)) fs.unlinkSync(temporary);
  }
  if (process.platform !== "win32") {
    const directory = fs.openSync(path.dirname(destination), "r");
    try { fs.fsyncSync(directory); } finally { fs.closeSync(directory); }
  }
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
  const entry = indexOnly ? null : index.releases.find(item => item.manifest.sha256 === record.manifestSha256 && item.version === record.version && item.build === record.build);
  let manifestPath, rawManifest, manifest, pack, packPath, packFile, pagePath, pageFile;
  if (!indexOnly) {
    if (!entry) throw Error("publication_identity_mismatch");
    manifestPath = `updates/objects/${entry.manifest.sha256}.json`;
    rawManifest = read(prepared, manifestPath);
    manifest = bindInventory(entry, record.component, rawManifest, publicKey);
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

  const hasV03 = record.v03 !== undefined && record.v03 !== null;
  const channelPathV03 = `updates/v0.3/beta/${record.component}.json`;
  let rawIndexV03, indexV03, entryV03, transferPath, rawTransfer, transferDoc, compressedPackPath, compressedPackFile;
  if (hasV03) {
    if (typeof record.v03 !== "object" || !/^[a-f0-9]{64}$/.test(record.v03.indexSha256 ?? "")) throw Error("invalid_publication_record");
    rawIndexV03 = read(prepared, channelPathV03);
    if (digest(rawIndexV03) !== record.v03.indexSha256) throw Error("publication_index_corrupt");
    indexV03 = verifyIndexV03(rawIndexV03, publicKey, record.component);
    if (indexV03.sequence !== index.sequence || !isDeepStrictEqual(indexV03.releases.map(r => r.version), index.releases.map(r => r.version))) {
      throw Error("publication_sequence_mismatch");
    }
    if (!indexOnly) {
      entryV03 = indexV03.releases.find(item => item.manifest.sha256 === record.manifestSha256 && item.version === record.version && item.build === record.build);
      if (!entryV03?.compressedTransfer || entryV03.compressedTransfer.sha256 !== record.v03.transferManifestSha256) throw Error("publication_identity_mismatch");
      transferPath = `updates/objects/${entryV03.compressedTransfer.sha256}.json`;
      rawTransfer = read(prepared, transferPath);
      transferDoc = bindTransfer(entryV03, record.component, manifest, rawTransfer, publicKey);
      compressedPackPath = `updates/objects/${transferDoc.pack.sha256}.pack`;
      compressedPackFile = ownedPath(prepared, compressedPackPath);
      if (!fs.lstatSync(compressedPackFile).isFile()
          || !isDeepStrictEqual(fileDigest(compressedPackFile), { sha256: transferDoc.pack.sha256, bytes: transferDoc.pack.bytes })) {
        throw Error("publication_pack_corrupt");
      }
      if (entryV03.compressedTransfer.sources[0] !== `https://ph-r.com/vua/updates/objects/${entryV03.compressedTransfer.sha256}.json`
          || transferDoc.pack.sources[0] !== `https://ph-r.com/vua/updates/objects/${transferDoc.pack.sha256}.pack`) {
        throw Error("publication_source_mismatch");
      }
    }
  }

  if (!fs.existsSync(webRoot)) { fs.mkdirSync(webRoot, { recursive: true, mode: 0o755 }); fs.chmodSync(webRoot, 0o755); }
  ownedTree(webRoot, "updates"); ownedTree(webRoot, "components");
  const lock = ownedPath(webRoot, `updates/.publish-${record.component}.lock`);
  let lease;
  try { lease = fs.openSync(lock, "wx"); }
  catch (error) { if (error.code === "EEXIST") throw Error("publication_busy_inspect_lock"); throw error; }
  try {
    fs.writeFileSync(lease, JSON.stringify({ component: record.component, sequence: index.sequence, pid: process.pid }));
    const live = ownedPath(webRoot, options.candidate ? `updates/candidates/${record.component}.json` : channelPath);
    const formal = ownedPath(webRoot, channelPath);
    const previous = fs.existsSync(formal) ? read(webRoot, channelPath) : null;
    const liveV03 = hasV03 ? ownedPath(webRoot, options.candidate ? `updates/v0.3/candidates/${record.component}.json` : channelPathV03) : null;
    const formalV03 = hasV03 ? ownedPath(webRoot, channelPathV03) : null;
    const previousV03 = hasV03 && fs.existsSync(formalV03) ? read(webRoot, channelPathV03) : null;

    const alreadyCommittedV02 = fs.existsSync(live) && digest(fs.readFileSync(live)) === digest(rawIndex);
    const alreadyCommittedV03 = !hasV03 || (fs.existsSync(liveV03) && digest(fs.readFileSync(liveV03)) === digest(rawIndexV03));
    const alreadyCommitted = alreadyCommittedV02 && alreadyCommittedV03;
    if (!alreadyCommittedV02 && (previous ? digest(previous) : null) !== record.previousIndexSha256) throw Error("publication_channel_changed");
    const before = previous ? verifyIndex(previous, publicKey, record.component) : null;
    const beforeV03 = previousV03 ? verifyIndexV03(previousV03, publicKey, record.component) : null;
    if (hasV03 && !alreadyCommittedV03) {
      if (record.v03.previousIndexSha256 !== null) {
        if ((previousV03 ? digest(previousV03) : null) !== record.v03.previousIndexSha256) throw Error("publication_channel_changed");
      } else if (beforeV03 && beforeV03.sequence !== (before?.sequence ?? 0)) {
        throw Error("publication_channel_changed");
      }
    }
    if (!alreadyCommitted && index.sequence <= Math.max(before?.sequence ?? 0, beforeV03?.sequence ?? 0, options.minimumSequence ?? 0)) throw Error("publication_sequence_mismatch");
    if (before && before.releases.some(old => {
      const retained = index.releases.find(item => item.version === old.version);
      return retained && !isDeepStrictEqual(retained, old);
    })) throw Error("published_component_changed");
    if (beforeV03 && hasV03 && record.v03.previousIndexSha256 !== null && beforeV03.releases.some(old => {
      const retained = indexV03.releases.find(item => item.version === old.version);
      return retained && !isDeepStrictEqual(retained, old);
    })) throw Error("published_component_changed");
    if (indexOnly && index.releases.some(item => !before?.releases.some(old => isDeepStrictEqual(item, old)))) throw Error("index_revision_must_only_remove");
    const staging = ownedPath(webRoot, "updates/.staging"); fs.mkdirSync(staging, { recursive: true, mode: 0o700 });
    const pageStaging = ownedPath(webRoot, "components/.staging"); fs.mkdirSync(pageStaging, { recursive: true, mode: 0o700 });
    if (!indexOnly) {
      immutableCopy(packFile, ownedPath(webRoot, packPath), { sha256: pack.sha256, bytes: pack.bytes }, staging, webRoot);
      immutableCopy(ownedPath(prepared, manifestPath), ownedPath(webRoot, manifestPath), { sha256: entry.manifest.sha256, bytes: entry.manifest.bytes }, staging, webRoot);
      if (hasV03) {
        immutableCopy(compressedPackFile, ownedPath(webRoot, compressedPackPath), { sha256: transferDoc.pack.sha256, bytes: transferDoc.pack.bytes }, staging, webRoot);
        immutableCopy(ownedPath(prepared, transferPath), ownedPath(webRoot, transferPath), { sha256: entryV03.compressedTransfer.sha256, bytes: entryV03.compressedTransfer.bytes }, staging, webRoot);
      }
      // Each ReadWritePaths tree can be a separate bind mount, even on the same disk.
      immutableCopy(pageFile, ownedPath(webRoot, pagePath), fileDigest(pageFile), pageStaging, webRoot);
    }
    // Validate every retained inventory/object too, using the frozen v1.0.0 rules (plus v0.3 compressed objects when declared).
    for (const item of index.releases) {
      const inventory = bindInventory(item, record.component, read(webRoot, `updates/objects/${item.manifest.sha256}.json`), publicKey);
      const object = inventory[record.component].pack;
      const objectFile = ownedPath(webRoot, `updates/objects/${object.sha256}.bin`);
      if (!fs.lstatSync(objectFile).isFile() || !isDeepStrictEqual(fileDigest(objectFile), { sha256: object.sha256, bytes: object.bytes })) throw Error("publication_pack_corrupt");
    }
    if (hasV03) {
      for (const item of indexV03.releases) {
        const inventory = bindInventory(item, record.component, read(webRoot, `updates/objects/${item.manifest.sha256}.json`), publicKey);
        if (item.compressedTransfer) {
          const transfer = bindTransfer(item, record.component, inventory, read(webRoot, `updates/objects/${item.compressedTransfer.sha256}.json`), publicKey);
          const packObject = ownedPath(webRoot, `updates/objects/${transfer.pack.sha256}.pack`);
          if (!fs.lstatSync(packObject).isFile() || !isDeepStrictEqual(fileDigest(packObject), { sha256: transfer.pack.sha256, bytes: transfer.pack.bytes })) throw Error("publication_pack_corrupt");
        }
      }
    }
    if (alreadyCommitted) return { component: record.component, sequence: index.sequence, unchanged: true };
    if (!alreadyCommittedV02) atomicIndexWrite(webRoot, live, rawIndex);
    if (hasV03 && !alreadyCommittedV03) atomicIndexWrite(webRoot, liveV03, rawIndexV03);
    return {
      component: record.component,
      sequence: index.sequence,
      version: entry?.version ?? null,
      manifestSha256: entry?.manifest.sha256 ?? null,
      indexSha256: digest(rawIndex),
      ...(hasV03 ? { v03IndexSha256: digest(rawIndexV03) } : {}),
      candidate: !!options.candidate,
    };
  } finally {
    fs.closeSync(lease); fs.unlinkSync(lock);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [prepared, webRoot] = process.argv.slice(2);
  if (!prepared || !webRoot) throw Error("Usage: publish-component-release.mjs <private-prepared-directory> <owned-vua-web-root>");
  const trust = JSON.parse(fs.readFileSync(new URL("../distribution/trust.json", import.meta.url)));
  console.log(JSON.stringify(publishComponentRelease(path.resolve(prepared), path.resolve(webRoot), trust.publicKey)));
}
