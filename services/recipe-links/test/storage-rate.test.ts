import assert from "node:assert/strict";
import { mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { createHandler, tokenHash } from "../src/handler.js";
import { canonicalize, shareCode } from "../src/json.js";
import { PublishLimiter } from "../src/rate-limit.js";
import { CodeCollision, DirectoryStore, MemoryStore } from "../src/storage.js";
import { normalizeIp, serverOptions, sourceIp } from "../src/node.js";
import { origin, publish, request, selection } from "./helpers.js";

for (const kind of ["memory", "directory"] as const) {
  test(`${kind}: concurrent publishers and withdrawals preserve every remaining claim`, async () => {
    const directory = await mkdtemp(join(tmpdir(), "vua-share-test-"));
    try {
      const store = kind === "memory" ? new MemoryStore() : new DirectoryStore(directory);
      const canonical = canonicalize(selection());
      const code = shareCode(canonical);
      const tokens = Array.from({ length: 40 }, (_, index) => tokenHash(`synthetic-${index}`));
      const results = await Promise.all(tokens.map(token => store.publish(code, canonical, token)));
      assert.equal(results.filter(result => result.created).length, 1);
      await assert.rejects(store.publish(code, "different", tokenHash("other")), CodeCollision);
      assert.equal(await store.get(code), canonical);
      assert.ok((await Promise.all(tokens.slice(1).map(token => store.withdraw(code, token)))).every(Boolean));
      assert.equal(await store.get(code), canonical);
      assert.equal(await store.withdraw(code, tokens[0]!), true);
      assert.equal(await store.get(code), null);
      assert.equal(await store.withdraw(code, tokens[0]!), false);
      await assert.rejects(store.get("../secret"));
      assert.deepEqual(await readdir(directory), []);
    } finally { await rm(directory, { recursive: true, force: true }); }
  });
}

test("directory restart preserves hashed withdrawal credentials; corrupt records fail closed", async () => {
  const directory = await mkdtemp(join(tmpdir(), "vua-share-restart-"));
  try {
    const store = new DirectoryStore(directory);
    const receipt = await publish(createHandler({ store, publicOrigin: origin }));
    const file = join(directory, `${receipt.code}.json`);
    const saved = await readFile(file, "utf8");
    assert.ok(!saved.includes(receipt.deleteToken));
    assert.ok(saved.includes(tokenHash(receipt.deleteToken)));
    const restarted = createHandler({ store: new DirectoryStore(directory), publicOrigin: origin });
    assert.equal((await restarted(request(`/api/shares/${receipt.code}`))).status, 200);
    await writeFile(file, "corrupt");
    assert.equal((await restarted(request(`/api/shares/${receipt.code}`))).status, 503);
    assert.equal((await restarted(request("/api/shares", "POST", JSON.stringify(selection())))).status, 503);
    assert.equal(await readFile(file, "utf8"), "corrupt");
    await writeFile(file, saved);
    assert.equal((await restarted(request(`/api/shares/${receipt.code}`, "DELETE", undefined, { authorization: `Bearer ${receipt.deleteToken}` }))).status, 204);
    assert.deepEqual(await readdir(directory), []);
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test("sliding hourly limits survive daily salt rotation; expired counts free capacity", () => {
  let now = Date.UTC(2026, 0, 1, 23, 59, 50);
  const limiter = new PublishLimiter(2, () => now);
  assert.equal(limiter.take("192.0.2.1").allowed, true);
  now += 20_000;
  assert.equal(limiter.take("192.0.2.1").allowed, true);
  assert.equal(limiter.take("192.0.2.1").allowed, false);
  now += 3_600_000;
  assert.equal(limiter.take("192.0.2.1").allowed, true);
  const bounded = new PublishLimiter(2, () => now, 1);
  assert.equal(bounded.take("192.0.2.1").allowed, true);
  assert.equal(bounded.take("192.0.2.2").allowed, false);
  now += 3_600_000;
  assert.equal(bounded.take("192.0.2.2").allowed, true);
});

test("proxy identity cannot be spoofed by forwarding headers from an untrusted peer", () => {
  assert.equal(sourceIp("192.0.2.1", "192.0.2.2", true), "192.0.2.1");
  assert.equal(sourceIp("::ffff:127.0.0.1", "192.0.2.2", false), "127.0.0.1");
  assert.equal(sourceIp("::ffff:127.0.0.1", "192.0.2.2", true), "192.0.2.2");
  assert.equal(sourceIp("127.0.0.1", "192.0.2.1, 192.0.2.2", true), null);
  assert.equal(sourceIp("127.0.0.1", undefined, true), null);
  assert.equal(normalizeIp("2001:0db8:0:0:0:0:0:1"), "2001:db8::1");
  assert.throws(() => serverOptions({ DATA_DIR: "relative" }));
  assert.throws(() => serverOptions({ DATA_DIR: tmpdir(), PORT: "0" }));
  assert.throws(() => serverOptions({ DATA_DIR: tmpdir(), PUBLISHES_PER_HOUR: "0" }));
  assert.throws(() => serverOptions({ DATA_DIR: tmpdir(), TRUST_PROXY: "yes" }));
  assert.equal(serverOptions({ DATA_DIR: tmpdir() }).port, 6191);
});
