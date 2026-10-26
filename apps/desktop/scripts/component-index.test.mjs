import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { createPrivateKey, createPublicKey } from "node:crypto";
import { validateIndex, verifyIndex, sealEnvelope, verifyInventory, appendRelease } from "../distribution/component-index.mjs";

const seed = Buffer.alloc(32, 7);
const privateKey = createPrivateKey({ key: Buffer.concat([Buffer.from("302e020100300506032b657004220420", "hex"), seed]), type: "pkcs8", format: "der" });
const trustedKey = createPublicKey(privateKey).export({ type: "spki", format: "der" }).subarray(-32).toString("hex");
const cases = JSON.parse(fs.readFileSync(new URL("../../../schemas/windows-distribution/v0.2/vectors/cases.json", import.meta.url)));
const old = JSON.parse(fs.readFileSync(new URL("../../../schemas/windows-distribution/v0.1/vectors/cases.json", import.meta.url)));

for (const vector of cases) test(`Component producer agrees with native vector: ${vector.name}`, () => {
  const run = () => vector.schema === "index" ? validateIndex(vector.value)
    : verifyIndex(Buffer.from(JSON.stringify(vector.value)), trustedKey, vector.component);
  if (vector.nativeValid) assert.doesNotThrow(run);
  else assert.throws(run);
});

test("Publication validates the same immutable v0.1 inventory and exact signing key", () => {
  const inventory = old.find(v => v.schema === "manifest" && v.valid).value;
  const raw = sealEnvelope(inventory, "0.1", privateKey, trustedKey);
  assert.deepEqual(verifyInventory(raw, trustedKey), inventory);
  assert.throws(() => sealEnvelope(inventory, "0.1", privateKey, "a".repeat(64)), /shipped_anchor/);
  const tampered = structuredClone(inventory); tampered.amf.files[0].path = "../project/paid-asset";
  assert.throws(() => verifyInventory(sealEnvelope(tampered, "0.1", privateKey, trustedKey), trustedKey));
});

test("One channel advances independently and retains compatible historical versions", () => {
  const previous = structuredClone(cases.find(v => v.name === "amf-index").value);
  const entry = structuredClone(previous.releases[0]); entry.version = "0.1.1"; entry.build = "b".repeat(40);
  const next = appendRelease(previous, entry);
  assert.equal(next.component, "amf"); assert.equal(next.sequence, previous.sequence + 1);
  assert.deepEqual(next.releases.map(v => v.version), ["0.1.1", "0.1.0"]);
  assert.equal(previous.sequence, 3); assert.equal(previous.releases.length, 1);
  assert.throws(() => appendRelease(next, { ...entry, build: "c".repeat(40) }), /already_published|duplicate/);
  assert.throws(() => verifyIndex(sealEnvelope(next, "0.2", privateKey, trustedKey), trustedKey, "host"), /wrong_component/);
  assert.throws(() => verifyIndex(sealEnvelope(next, "0.2", privateKey, trustedKey), trustedKey, "amf", 5), /replayed/);
});
