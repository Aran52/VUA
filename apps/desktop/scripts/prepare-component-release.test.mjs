import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { createPrivateKey, createPublicKey } from "node:crypto";
import { spawnSync } from "node:child_process";
import { digest, sealEnvelope, verifyIndex, verifyInventory } from "../distribution/component-index.mjs";
import { prepareComponentRelease, verifyBuildReceipt, fileDigest, verifyAmfIdentity, verifyHostRuntimeIdentity } from "./prepare-component-release.mjs";
import { publishComponentRelease } from "./publish-component-release.mjs";
import { exportComponentPublisher } from "./export-component-publisher.mjs";

const privateKey = createPrivateKey({ key: Buffer.concat([Buffer.from("302e020100300506032b657004220420", "hex"), Buffer.alloc(32, 7)]), format: "der", type: "pkcs8" });
const publicKey = createPublicKey(privateKey).export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
const vectors = JSON.parse(fs.readFileSync(new URL("../../../schemas/windows-distribution/v0.1/vectors/cases.json", import.meta.url)));
test("Full installers and component exports reject different UI/provider/runtime identities", () => {
  const expected = { version: "0.2.1-qa.1", build: "a".repeat(40), bdlWrite: "0.11", uiAbi: "amf-ui/0.1" };
  const native = { schemaVersion: "0.1", component: "amf", ...expected, serviceBuildId: `amf/${expected.version}@${expected.build}`, dataRecoveryAbi: "amf-data/0.1" };
  const ui = { schemaVersion: "0.1", component: "amf", ...expected, hotReloadAbi: "amf-hot/0.1", dataRecoveryAbi: "amf-data/0.1" };
  verifyAmfIdentity(native, ui, expected);
  for (const field of ["version", "build", "bdlWrite", "uiAbi", "hotReloadAbi", "dataRecoveryAbi"]) assert.throws(() => verifyAmfIdentity(native, { ...ui, [field]: "mismatch" }, expected));
  for (const field of ["version", "build", "bdlWrite", "serviceBuildId", "dataRecoveryAbi"]) assert.throws(() => verifyAmfIdentity({ ...native, [field]: "mismatch" }, ui, expected));
  verifyHostRuntimeIdentity({ schemaVersion: "0.1", build: expected.build, dataTrialAbi: "amf-data/0.1" }, expected);
  assert.throws(() => verifyHostRuntimeIdentity({ schemaVersion: "0.1", build: "b".repeat(40), dataTrialAbi: "amf-data/0.1" }, expected));
  assert.throws(() => verifyHostRuntimeIdentity({ schemaVersion: "0.1", build: expected.build }, expected));
});
function baseline() { return structuredClone(vectors.find(v => v.schema === "manifest" && v.valid).value); }
function fixture(component = "amf") {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "vua-component-publication-"));
  const source = path.join(root, "source"); fs.mkdirSync(source);
  const paths = component === "amf" ? ["ui/amf-entry.js", "ui/style.css", "provider/vua-amf-provider.exe"]
    : ["dist/electron/main.js", "dist/electron/preload.js", "dist/renderer/index.html", "provider/vua-orchestrator-provider.exe", "bootstrap/app.asar", "launcher/VUA.exe"];
  const fileNames = [...paths, "notices/README.md", "LICENSE", "NOTICE", "THIRD_PARTY_NOTICES.md"];
  for (const name of fileNames) {
    fs.mkdirSync(path.dirname(path.join(source, name)), { recursive: true }); fs.writeFileSync(path.join(source, name), `synthetic ${name}\n`);
  }
  const original = baseline();
  return { root, source, original, fileNames, options: { component, version: component === "amf" ? "0.1.1" : "0.6.8", build: "b".repeat(40), componentRoot: source,
    baseline: sealEnvelope(original, "0.1", privateKey, publicKey), output: path.join(root, "prepared"), privateKey, publicKey } };
}
const readIndex = (root, component) => fs.readFileSync(path.join(root, "updates/beta", `${component}.json`));

test("AMF-only preparation preserves host/runtime inventories and publishes only its own channel", () => {
  const f = fixture(); const record = prepareComponentRelease(f.options);
  const index = verifyIndex(readIndex(f.options.output, "amf"), publicKey, "amf");
  const manifest = verifyInventory(fs.readFileSync(path.join(f.options.output, "updates/objects", `${record.manifestSha256}.json`)), publicKey);
  assert.deepEqual(manifest.host, f.original.host); assert.deepEqual(manifest.runtime, f.original.runtime);
  assert.equal(manifest.releaseVersion, f.original.releaseVersion); assert.equal(manifest.amf.version, "0.1.1");
  assert.equal(index.releases[0].activation, "reload"); assert.equal(index.releases[0].runtimeVersion, undefined);
  assert.equal(fs.existsSync(path.join(f.options.output, "updates/beta/host.json")), false);
  assert.deepEqual(fs.readdirSync(path.join(f.options.output, "updates/objects")).sort(), [`${record.packSha256}.bin`, `${record.manifestSha256}.json`].sort());
});

test("Host-only preparation preserves AMF and the exact runtime", () => {
  const f = fixture("host"); const record = prepareComponentRelease(f.options);
  const manifest = verifyInventory(fs.readFileSync(path.join(f.options.output, "updates/objects", `${record.manifestSha256}.json`)), publicKey);
  assert.equal(manifest.host.version, "0.6.8"); assert.equal(manifest.releaseVersion, "0.6.8");
  assert.deepEqual(manifest.amf, f.original.amf); assert.deepEqual(manifest.runtime, f.original.runtime);
  assert.equal(fs.existsSync(path.join(f.options.output, "updates/beta/amf.json")), false);
});

test("An existing release seeds a channel only with byte-identical inventory and pack order", () => {
  const f = fixture(); let offset = 0;
  const parts = [];
  f.original.amf.files = f.fileNames.slice().reverse().map(name => {
    const bytes = fs.readFileSync(path.join(f.source, name));
    const file = { path: name, bytes: bytes.length, sha256: digest(bytes), offset }; offset += bytes.length; parts.push(bytes); return file;
  });
  const originalPack = Buffer.concat(parts);
  f.original.amf.pack = { bytes: offset, sha256: digest(originalPack), sources: ["https://example.test/original.bin"] };
  const options = { ...f.options, version: f.original.amf.version, build: f.original.amf.build, baseline: sealEnvelope(f.original, "0.1", privateKey, publicKey) };
  const record = prepareComponentRelease(options);
  assert.equal(record.packSha256, f.original.amf.pack.sha256);
  assert.deepEqual(fs.readFileSync(path.join(options.output, "updates/objects", `${record.packSha256}.bin`)), originalPack);
  fs.appendFileSync(path.join(f.source, "ui/amf-entry.js"), "changed");
  assert.throws(() => prepareComponentRelease({ ...options, output: path.join(f.root, "repacked") }), /published_component_changed/);
});

test("Writer-format changes are published as restart updates even if reload was requested", () => {
  const f = fixture(); const compatibility = structuredClone(f.original.compatibility);
  compatibility.bdlWrite = "0.99"; compatibility.bdlRead.push("0.99");
  prepareComponentRelease({ ...f.options, compatibility, activation: "reload" });
  assert.equal(verifyIndex(readIndex(f.options.output, "amf"), publicKey, "amf").releases[0].activation, "restart");
});

test("Build receipts reject stale revisions, dirty builds, changed files and missing notices", () => {
  const f = fixture();
  const expected = { component: "amf", version: "0.1.1", build: "b".repeat(40) };
  const receipt = { schemaVersion: "0.2", ...expected, dirty: false, files: f.fileNames.map(name => ({ path: name, ...fileDigest(path.join(f.source, name)) })) };
  assert.equal(verifyBuildReceipt(f.source, receipt, expected), receipt);
  assert.throws(() => verifyBuildReceipt(f.source, { ...receipt, dirty: true }, expected), /clean_source|clean source/);
  assert.throws(() => verifyBuildReceipt(f.source, receipt, { ...expected, build: "c".repeat(40) }), /clean source/);
  assert.throws(() => verifyBuildReceipt(f.source, { ...receipt, files: receipt.files.slice(1) }, expected), /differ/);
  fs.appendFileSync(path.join(f.source, "ui/amf-entry.js"), "stale");
  assert.throws(() => verifyBuildReceipt(f.source, receipt, expected), /differ/);
});

test("Publication makes verified objects visible before one atomic channel switch and is retryable", () => {
  const f = fixture(); const record = prepareComponentRelease(f.options); const web = path.join(f.root, "web");
  const first = publishComponentRelease(f.options.output, web, publicKey);
  assert.equal(first.version, "0.1.1"); assert.equal(first.sequence, 1);
  assert.deepEqual(readIndex(web, "amf"), readIndex(f.options.output, "amf"));
  assert.equal(digest(fs.readFileSync(path.join(web, "updates/objects", `${record.packSha256}.bin`))), record.packSha256);
  assert.equal(publishComponentRelease(f.options.output, web, publicKey).unchanged, true);
  fs.unlinkSync(path.join(web, "updates/objects", `${record.packSha256}.bin`));
  assert.equal(publishComponentRelease(f.options.output, web, publicKey).unchanged, true);
  assert.equal(fs.existsSync(path.join(web, "updates/objects", `${record.packSha256}.bin`)), true);
});

test("Independent channels retain each other's files and previous compatible releases", () => {
  const f = fixture(); prepareComponentRelease(f.options); const web = path.join(f.root, "web");
  publishComponentRelease(f.options.output, web, publicKey);
  const before = readIndex(web, "amf");
  const host = fixture("host"); prepareComponentRelease(host.options); publishComponentRelease(host.options.output, web, publicKey);
  assert.deepEqual(readIndex(web, "amf"), before);
  const next = { ...f.options, version: "0.1.2", build: "c".repeat(40), previous: before, output: path.join(f.root, "next") };
  prepareComponentRelease(next); publishComponentRelease(next.output, web, publicKey);
  const index = verifyIndex(readIndex(web, "amf"), publicKey, "amf");
  assert.equal(index.sequence, 2); assert.deepEqual(index.releases.map(v => v.version), ["0.1.2", "0.1.1"]);
  assert.equal(verifyIndex(readIndex(web, "host"), publicKey, "host").sequence, 1);
  assert.throws(() => prepareComponentRelease({ ...next, output: path.join(f.root, "duplicate"), previous: readIndex(web, "amf") }), /already_published/);
});

test("A competing publication cannot replace the channel without its exact previous index", () => {
  const f = fixture(); prepareComponentRelease(f.options); const web = path.join(f.root, "web");
  publishComponentRelease(f.options.output, web, publicKey); const before = readIndex(web, "amf");
  const stale = { ...f.options, version: "0.1.2", output: path.join(f.root, "stale") }; prepareComponentRelease(stale);
  assert.throws(() => publishComponentRelease(stale.output, web, publicKey), /channel_changed/);
  assert.deepEqual(readIndex(web, "amf"), before);
});

test("Process death after object upload cannot expose a new channel; inspected retry recovers", () => {
  const f = fixture(); prepareComponentRelease(f.options); const web = path.join(f.root, "web");
  publishComponentRelease(f.options.output, web, publicKey); const before = readIndex(web, "amf");
  const next = { ...f.options, version: "0.1.2", previous: before, output: path.join(f.root, "next") }; const record = prepareComponentRelease(next);
  const destination = path.join(web, "updates/beta/amf.json");
  const child = spawnSync(process.execPath, ["--input-type=module", "-e", `
    import fs from 'node:fs';
    import {publishComponentRelease} from ${JSON.stringify(new URL("./publish-component-release.mjs", import.meta.url).href)};
    const rename=fs.renameSync;
    fs.renameSync=(source,destination)=>{if(destination===${JSON.stringify(destination)})process.exit(73);return rename(source,destination);};
    publishComponentRelease(${JSON.stringify(next.output)},${JSON.stringify(web)},${JSON.stringify(publicKey)});
  `], { windowsHide: true, encoding: "utf8" });
  assert.equal(child.status, 73, child.stderr);
  assert.deepEqual(readIndex(web, "amf"), before);
  assert.equal(fs.existsSync(path.join(web, "updates/objects", `${record.manifestSha256}.json`)), true);
  assert.deepEqual(fs.readdirSync(path.join(web, "updates/beta")), ["amf.json"]);
  assert.throws(() => publishComponentRelease(next.output, web, publicKey), /inspect_lock/);
  // The exact child is terminal above; this is the operator's explicit inspected recovery.
  fs.unlinkSync(path.join(web, "updates/.publish-amf.lock"));
  assert.equal(publishComponentRelease(next.output, web, publicKey).sequence, 2);
});

test("Exported operator tool runs outside the workspace with only its included public dependency", () => {
  const f = fixture(); prepareComponentRelease(f.options);
  const tool = path.join(f.root, "export/nested/operator"); const exported = exportComponentPublisher(tool);
  assert.ok(exported.files > 10);
  const trust = path.join(tool, "apps/desktop/distribution/trust.json");
  const shipped = JSON.parse(fs.readFileSync(new URL("../distribution/trust.json", import.meta.url)));
  assert.equal(JSON.parse(fs.readFileSync(trust)).publicKey, shipped.publicKey);
  // Only this isolated test copy gets the synthetic vector anchor; no real key is read.
  fs.writeFileSync(trust, JSON.stringify({ algorithm: "ed25519", publicKey }));
  const web = path.join(f.root, "operator-web");
  const result = spawnSync(process.execPath, [path.join(tool, "apps/desktop/scripts/publish-component-release.mjs"), f.options.output, web], { cwd: tool, windowsHide: true, encoding: "utf8", env: { ...process.env, NODE_PATH: "" } });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(result.stdout).component, "amf");
  assert.equal(verifyIndex(readIndex(web, "amf"), publicKey, "amf").sequence, 1);
});

test("Corrupt pack, index, inventory or unsigned page never becomes the channel target", () => {
  for (const type of ["pack", "index", "inventory", "page"]) {
    const f = fixture(); const record = prepareComponentRelease(f.options); const web = path.join(f.root, "web");
    const target = type === "pack" ? `updates/objects/${record.packSha256}.bin` : type === "inventory" ? `updates/objects/${record.manifestSha256}.json`
      : type === "index" ? "updates/beta/amf.json" : "components/amf/v0.1.1/index.html";
    fs.appendFileSync(path.join(f.options.output, target), "corruption");
    assert.throws(() => publishComponentRelease(f.options.output, web, publicKey));
    assert.equal(fs.existsSync(path.join(web, "updates/beta/amf.json")), false);
  }
});

test("No keys, user files, links or repacked same-version code are accepted", () => {
  const wrongKey = fixture(); assert.throws(() => prepareComponentRelease({ ...wrongKey.options, publicKey: "a".repeat(64) }));
  assert.equal(fs.existsSync(wrongKey.options.output), false);
  const f = fixture(); fs.writeFileSync(path.join(f.source, "credentials.env"), "synthetic not a secret");
  assert.throws(() => prepareComponentRelease(f.options), /unexpected_component_file/);
  const missing = fixture(); fs.unlinkSync(path.join(missing.source, "NOTICE")); assert.throws(() => prepareComponentRelease(missing.options), /notices_missing/);
  const same = fixture(); assert.throws(() => prepareComponentRelease({ ...same.options, version: same.original.amf.version }), /version_must_advance/);
  const linked = fixture(); const outside = path.join(linked.root, "outside"); fs.mkdirSync(outside);
  fs.symlinkSync(outside, path.join(linked.source, "ui/escape"), process.platform === "win32" ? "junction" : "dir");
  assert.throws(() => prepareComponentRelease(linked.options), /path_is_link/);
});

test("Unowned or locked destinations remain intact, and corrupt immutable objects are reported", () => {
  const f = fixture(); const record = prepareComponentRelease(f.options); const web = path.join(f.root, "web");
  fs.mkdirSync(path.join(web, "updates"), { recursive: true }); fs.writeFileSync(path.join(web, "updates/existing"), "unrelated");
  assert.throws(() => publishComponentRelease(f.options.output, web, publicKey), /unowned/);
  assert.equal(fs.readFileSync(path.join(web, "updates/existing"), "utf8"), "unrelated");
  const fresh = path.join(f.root, "owned"); publishComponentRelease(f.options.output, fresh, publicKey);
  const before = readIndex(fresh, "amf"); fs.writeFileSync(path.join(fresh, "updates/.publish-amf.lock"), "inspect");
  assert.throws(() => publishComponentRelease(f.options.output, fresh, publicKey), /inspect_lock/);
  assert.deepEqual(readIndex(fresh, "amf"), before);
  fs.unlinkSync(path.join(fresh, "updates/.publish-amf.lock")); fs.appendFileSync(path.join(fresh, "updates/objects", `${record.packSha256}.bin`), "bad");
  assert.throws(() => publishComponentRelease(f.options.output, fresh, publicKey), /published_object_conflict/);
  assert.deepEqual(readIndex(fresh, "amf"), before);
});
