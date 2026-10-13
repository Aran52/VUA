import assert from "node:assert/strict";
import { test } from "node:test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { createHash, generateKeyPairSync, sign } from "node:crypto";
import { prepareOfficialRelease } from "./prepare-official-release.mjs";
import { canonicalReleaseUrl, componentSources, officialReleaseUrl } from "../distribution/release-sources.mjs";

const digest = bytes => createHash("sha256").update(bytes).digest("hex");
function fixture(root) {
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  const host = Buffer.from("synthetic-host"); const amf = Buffer.from("synthetic-amf");
  const version = "0.6.7"; const build = "a".repeat(40);
  const payload = JSON.stringify({ product: "VUA", releaseVersion: version, host: { version, build, pack: { bytes: host.length, sha256: digest(host) } }, amf: { version: "0.1.0", pack: { bytes: amf.length, sha256: digest(amf) } } });
  const envelope = JSON.stringify({ schemaVersion: "0.1", algorithm: "ed25519", payload, signature: sign(null, Buffer.from(payload), privateKey).toString("hex") });
  const files = new Map([
    ["VUA-0.6.7-windows-x64-setup.exe", Buffer.from("synthetic-installer")],
    ["VUA-0.6.7-windows-x64-offline.exe", Buffer.from("synthetic-offline")],
    ["vua-host-0.6.7.bin", host], ["vua-amf-0.1.0.bin", amf],
    ["vua-release.json", Buffer.from(envelope)],
    ["release-manifest.json", Buffer.from(JSON.stringify({ version, sourceCommit: build }))],
  ]);
  for (const [name, bytes] of files) fs.writeFileSync(path.join(root, name), bytes);
  fs.writeFileSync(path.join(root, "SHA256SUMS.txt"), [...files].map(([name, bytes]) => `${digest(bytes)}  ${name}`).join("\n") + "\n");
  fs.writeFileSync(path.join(root, "private-do-not-export.txt"), "synthetic-private");
  return { publicKey: publicKey.export({ type: "spki", format: "der" }).subarray(-32).toString("hex"), envelope };
}

test("official mirror exports verified immutable bytes, metadata and clear planned AMF copy", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "vua-publication-test-"));
  try {
    const { publicKey, envelope } = fixture(root);
    const output = path.join(root, "output");
    const release = prepareOfficialRelease(root, output, publicKey);
    const mirror = path.join(output, "releases/v0.6.7");
    assert.equal(fs.readFileSync(path.join(mirror, "vua-release.json"), "utf8"), envelope);
    assert.equal(fs.existsSync(path.join(mirror, "private-do-not-export.txt")), false);
    assert.equal(release.assets.length, 7);
    const latest = JSON.parse(fs.readFileSync(path.join(mirror, "latest.json")));
    assert.equal(latest.html_url, "https://ph-r.com/vua/releases/v0.6.7/");
    assert.equal(latest.manifest_sha256, digest(Buffer.from(envelope)));
    const page = fs.readFileSync(path.join(output, "project-vua.html"), "utf8");
    assert.match(page, /AMF 正在开发的方向/); assert.match(page, /自动改模/); assert.match(page, /Recipe 配方/);
    assert.match(page, /v0.6.7 内置更新检查仍使用 GitHub/);
    assert.doesNotMatch(page, /<style>|style=/);
    assert.match(page, /href="\/vua\/downloads\/style.css"/);
    assert.ok(fs.statSync(path.join(output, "downloads/style.css")).size > 0);
    assert.ok(fs.statSync(path.join(output, "downloads/downloads.js")).size > 0);
    assert.match(page, /src="\/vua\/downloads\/downloads.js"/);
    assert.throws(() => prepareOfficialRelease(root, output, publicKey), /EEXIST/);
  } finally { fs.rmSync(root, { recursive: true }); }
});

test("corrupt artifacts and unauthenticated inventories cannot create a deployable output", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "vua-publication-rejection-"));
  try {
    const { publicKey } = fixture(root);
    const output = path.join(root, "output");
    fs.appendFileSync(path.join(root, "VUA-0.6.7-windows-x64-setup.exe"), "bad");
    assert.throws(() => prepareOfficialRelease(root, output, publicKey), /digest mismatch/);
    assert.equal(fs.existsSync(output), false);
    assert.throws(() => prepareOfficialRelease(root, output, "00".repeat(32)), /signature rejected/);
    assert.equal(fs.existsSync(output), false);
  } finally { fs.rmSync(root, { recursive: true }); }
});

test("future signed packs include official and GitHub sources with fixed immutable names", () => {
  assert.equal(canonicalReleaseUrl("0.6.8"), "https://github.com/VUA-Project/VUA/releases/tag/v0.6.8");
  assert.deepEqual(componentSources("0.6.8", "vua-amf-0.1.1.bin"), [
    "https://ph-r.com/vua/releases/v0.6.8/vua-amf-0.1.1.bin",
    "https://github.com/VUA-Project/VUA/releases/download/v0.6.8/vua-amf-0.1.1.bin",
  ]);
  assert.throws(() => componentSources("0.6.8", "../private.bin"));
  assert.throws(() => officialReleaseUrl("0.6.8/../../"));
});
