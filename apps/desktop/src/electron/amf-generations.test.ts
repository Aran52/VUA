import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { AmfGenerations } from "./amf-generations.js";
import { amfUpdateFixture } from "../../scripts/fixtures/amf-update-native.js";
describe("Main-owned AMF generations", () => {
  it("binds metadata bytes and facts, and gives every retry an independent import graph", () => {
    const fixture = amfUpdateFixture();
    try {
      const generations = new AmfGenerations(fixture.context);
      const first = generations.load(fixture.next.id, fixture.next.version, fixture.next.build);
      const retry = generations.load(fixture.next.id, fixture.next.version, fixture.next.build);
      expect(first.instanceId).not.toBe(retry.instanceId);
      const url = (instanceId: string, id = first.id) => `vua-module://amf/${instanceId}/${id}/ui/amf-entry.js`;
      expect(generations.read(url(first.instanceId))?.bytes.toString()).toContain("0.2.1");
      expect(generations.read(url("e".repeat(32)))).toBeUndefined();
      expect(generations.read(url(first.instanceId, "f".repeat(64)))).toBeUndefined();
      expect(() => generations.load(fixture.next.id, "9.9.9", fixture.next.build)).toThrow("invalid_module_inventory");
      fs.appendFileSync(path.join(fixture.root, "manifests", `${fixture.next.id}.json`), " ");
      expect(() => generations.load(fixture.next.id, fixture.next.version, fixture.next.build)).toThrow("module_inventory_integrity_failed");
    } finally { fixture.dispose(); }
  });
  it("keeps UI/backend switching explicit and verifies every served/executed file", () => {
    const fixture = amfUpdateFixture();
    try {
      const generations = new AmfGenerations(fixture.context); const old = generations.active!;
      const next = generations.load(fixture.next.id, fixture.next.version, fixture.next.build);
      generations.selectBackend(next); expect(generations.active).toBe(old); expect(generations.executable()).toBe(path.join(next.root, "provider/vua-amf-provider.exe"));
      generations.commit(next); generations.forget(old);
      expect(generations.read(`vua-module://amf/${old.instanceId}/${old.id}/ui/amf-entry.js`)).toBeUndefined();
      const url = `vua-module://amf/${next.instanceId}/${next.id}/ui/amf-entry.js`;
      fs.appendFileSync(path.join(next.root, "ui/amf-entry.js"), "tampered"); expect(() => generations.read(url)).toThrow("module_file_integrity_failed");
      fs.appendFileSync(path.join(next.root, "provider/vua-amf-provider.exe"), "tampered"); expect(() => generations.executable()).toThrow("amf_program_files_need_repair");
    } finally { fixture.dispose(); }
  });
});
