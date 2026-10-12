import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createHash, createPublicKey, verify } from "node:crypto";
import { officialReleaseUrl } from "../distribution/release-sources.mjs";
import { downloadPage, officialStyles, projectPage } from "../distribution/official-pages.mjs";

const digest = bytes => createHash("sha256").update(bytes).digest("hex");
const readFile = (root, name) => {
  const target = path.join(root, name);
  if (!fs.lstatSync(target).isFile()) throw Error(`Not a regular public asset: ${name}`);
  return fs.readFileSync(target);
};

/** Export only known publication artifacts. Preserve every already-published byte. */
export function prepareOfficialRelease(source, output, publicKey) {
  const raw = readFile(source, "vua-release.json");
  if (raw.length > 4 * 1024 * 1024) throw Error("Manifest too large");
  const envelope = JSON.parse(raw);
  if (envelope.schemaVersion !== "0.1" || envelope.algorithm !== "ed25519" || typeof envelope.payload !== "string" || !/^[a-f0-9]{128}$/.test(envelope.signature)) throw Error("Invalid signed envelope");
  const key = createPublicKey({ key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), Buffer.from(publicKey, "hex")]), format: "der", type: "spki" });
  if (!verify(null, Buffer.from(envelope.payload), key, Buffer.from(envelope.signature, "hex"))) throw Error("Manifest signature rejected");
  const manifest = JSON.parse(envelope.payload);
  const version = manifest.releaseVersion;
  const base = officialReleaseUrl(version);
  const amfVersion = manifest.amf.version;
  if (!/^\d+\.\d+\.\d+$/.test(amfVersion) || manifest.product !== "VUA" || manifest.host.version !== version || !/^[a-f0-9]{40}$/.test(manifest.host.build)) throw Error("Invalid release identity");
  const names = [`VUA-${version}-windows-x64-setup.exe`, `VUA-${version}-windows-x64-offline.exe`, `vua-host-${version}.bin`, `vua-amf-${amfVersion}.bin`, "vua-release.json", "release-manifest.json"];
  const sumsRaw = readFile(source, "SHA256SUMS.txt");
  const sums = new Map();
  for (const line of sumsRaw.toString("utf8").trim().split(/\r?\n/)) {
    const match = /^([a-f0-9]{64})  ([A-Za-z0-9.-]+)$/.exec(line);
    if (!match || !names.includes(match[2]) || sums.has(match[2])) throw Error("Invalid publication checksum list");
    sums.set(match[2], match[1]);
  }
  if (sums.size !== names.length) throw Error("Incomplete publication checksums");
  const assets = names.map(name => {
    const bytes = readFile(source, name);
    const sha256 = digest(bytes);
    if (sums.get(name) !== sha256) throw Error(`Publication digest mismatch: ${name}`);
    return { name, bytes: bytes.length, sha256, url: `${base}${name}` };
  });
  for (const [name, pack] of [[`vua-host-${version}.bin`, manifest.host.pack], [`vua-amf-${amfVersion}.bin`, manifest.amf.pack]]) {
    const asset = assets.find(asset => asset.name === name);
    if (asset.sha256 !== pack.sha256 || asset.bytes !== pack.bytes) throw Error("Signed pack differs from publication");
  }
  const record = JSON.parse(readFile(source, "release-manifest.json"));
  if (record.version !== version || record.sourceCommit !== manifest.host.build) throw Error("Publication source identity mismatch");
  assets.push({ name: "SHA256SUMS.txt", bytes: sumsRaw.length, sha256: digest(sumsRaw), url: `${base}SHA256SUMS.txt` });
  const release = { version, amfVersion, build: manifest.host.build, assets };
  fs.mkdirSync(output, { recursive: false });
  const releaseDirectory = path.join(output, "releases", `v${version}`);
  fs.mkdirSync(releaseDirectory, { recursive: true });
  for (const asset of assets) fs.copyFileSync(path.join(source, asset.name), path.join(releaseDirectory, asset.name), fs.constants.COPYFILE_EXCL);
  const latest = { schemaVersion: 1, product: "VUA", tag_name: `v${version}`, html_url: base, amf_version: amfVersion, source_commit: manifest.host.build, manifest_sha256: digest(raw), assets };
  fs.writeFileSync(path.join(releaseDirectory, "latest.json"), `${JSON.stringify(latest, null, 2)}\n`, { flag: "wx" });
  fs.writeFileSync(path.join(releaseDirectory, "index.html"), downloadPage(release), { flag: "wx" });
  fs.mkdirSync(path.join(output, "downloads"));
  fs.writeFileSync(path.join(output, "downloads", "index.html"), downloadPage(release), { flag: "wx" });
  fs.writeFileSync(path.join(output, "downloads", "style.css"), officialStyles, { flag: "wx" });
  fs.writeFileSync(path.join(output, "project-vua.html"), projectPage(release), { flag: "wx" });
  return release;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (!process.argv[2] || !process.argv[3]) throw Error("Usage: prepare-official-release.mjs <public-release-directory> <new-output-directory>");
  const trust = JSON.parse(fs.readFileSync(new URL("../distribution/trust.json", import.meta.url)));
  console.log(JSON.stringify(prepareOfficialRelease(path.resolve(process.argv[2]), path.resolve(process.argv[3]), trust.publicKey)));
}
