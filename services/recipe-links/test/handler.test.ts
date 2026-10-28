import assert from "node:assert/strict";
import { test } from "node:test";
import { createHandler } from "../src/handler.js";
import { canonicalize, MAX_BODY_BYTES } from "../src/json.js";
import { MemoryStore } from "../src/storage.js";
import { PublishLimiter } from "../src/rate-limit.js";
import { app, origin, publish, recipe, request, selection, type Receipt } from "./helpers.js";

for (const [name, declaration] of [["selection", selection()], ["Recipe v0.4", recipe("0.4")], ["Recipe v0.5", recipe("0.5")]] as const) {
  test(`${name}: publish, canonical read, material page, cache and withdrawal`, async () => {
    const handler = app();
    const response = await handler(request("/api/shares", "POST", JSON.stringify(declaration)));
    assert.equal(response.status, 201);
    assert.equal(response.headers.get("cache-control"), "no-store");
    const receipt = await response.json() as Receipt;
    assert.match(receipt.code, /^[0-9A-HJKMNP-TV-Z]{12}$/u);
    assert.match(receipt.deleteToken, /^[A-Za-z0-9_-]{43}$/u);
    assert.equal(receipt.url, `${origin}/r/${receipt.code}`);
    const read = await handler(request(`/api/shares/${receipt.code.toLowerCase()}`));
    assert.equal(read.status, 200);
    assert.match(read.headers.get("content-type")!, /application\/json/u);
    assert.equal(await read.text(), canonicalize(declaration));
    const page = await handler(request(`/R/${receipt.code.toLowerCase()}`));
    assert.equal(page.status, 200);
    const html = await page.text();
    assert.ok(html.includes(`vua://share/${receipt.code}`));
    assert.ok(html.includes("https://booth.pm/ja/items/1000001"));
    assert.equal(html.includes("This is an Avatar assembly plan."), name !== "selection");
    const cached = await handler(request(`/api/shares/${receipt.code}`, "GET", undefined, { "if-none-match": read.headers.get("etag")! }));
    assert.equal(cached.status, 304);
    assert.equal((await handler(request(`/api/shares/${receipt.code}`, "DELETE", undefined, { authorization: `Bearer ${receipt.deleteToken}` }))).status, 204);
    assert.equal((await handler(request(`/api/shares/${receipt.code}`, "GET", undefined, { "if-none-match": read.headers.get("etag")! }))).status, 404);
    assert.equal((await handler(request(`/r/${receipt.code}`))).status, 404);
  });
}

test("duplicate publish returns independent credentials; only the last withdrawal removes content", async () => {
  const handler = app();
  const first = await publish(handler);
  const response = await handler(request("/API/SHARES", "POST", canonicalize(selection())));
  assert.equal(response.status, 200);
  const second = await response.json() as Receipt;
  assert.equal(first.code, second.code);
  assert.notEqual(first.deleteToken, second.deleteToken);
  const withdraw = (token: string) => handler(request(`/API/SHARES/${first.code.toLowerCase()}`, "DELETE", undefined, { authorization: `Bearer ${token}` }));
  assert.equal((await withdraw(second.deleteToken.slice(1) + "x")).status, 403);
  assert.equal((await withdraw(first.deleteToken)).status, 204);
  assert.equal((await withdraw(first.deleteToken)).status, 403);
  assert.equal((await handler(request(`/api/shares/${first.code}`))).status, 200);
  assert.equal((await withdraw(second.deleteToken)).status, 204);
  assert.equal((await handler(request(`/api/shares/${first.code}`))).status, 404);
  assert.equal((await handler(request(`/api/shares/${first.code}`, "DELETE"))).status, 403);
  const republished = await publish(handler);
  assert.equal(republished.code, first.code);
  assert.equal((await withdraw(second.deleteToken)).status, 403);
});

test("body limit covers declared, actual streamed and UTF-8 byte sizes", async () => {
  const handler = app();
  const json = JSON.stringify(selection());
  assert.equal((await handler(request("/api/shares", "POST", json.padEnd(MAX_BODY_BYTES, " ")))).status, 201);
  for (const body of [json.padEnd(MAX_BODY_BYTES + 1, " "), "猫".repeat(25_000)]) assert.equal((await handler(request("/api/shares", "POST", body))).status, 413);
  assert.equal((await handler(request("/api/shares", "POST", json, { "content-length": "65537" }))).status, 413);
  const stream = new ReadableStream<Uint8Array>({ start(controller) { controller.enqueue(new Uint8Array(MAX_BODY_BYTES + 1)); controller.close(); } });
  const streamed = new Request(origin + "/api/shares", { method: "POST", body: stream, headers: { "content-type": "application/json" }, duplex: "half" } as RequestInit & { duplex: "half" });
  assert.equal((await handler(streamed)).status, 413);
  const invalidUtf8 = new Request(origin + "/api/shares", { method: "POST", headers: { "content-type": "application/json" }, body: new Uint8Array([0xff]) });
  assert.equal((await handler(invalidUtf8)).status, 400);
});

test("invalid documents and malformed JSON return stable, non-reflecting errors", async () => {
  const handler = app();
  for (const body of ["{}", "null", "[1]", '{"title":"secret","title":"other"}', '{"x":' + "[".repeat(33) + "0" + "]".repeat(33) + "}"]) {
    const response = await handler(request("/api/shares", "POST", body));
    assert.equal(response.status, 400);
    const error = await response.json() as { error: string; message: string };
    assert.equal(error.error, "invalid_declaration");
    assert.ok(!error.message.includes("secret"));
  }
  assert.equal((await handler(request("/api/shares", "POST", "{}", { "content-type": "text/plain" }))).status, 415);
  assert.equal((await handler(request("/api/shares"))).status, 405);
  assert.equal((await handler(request("/r/OOOOOOOOOOOO"))).status, 404);
  assert.equal((await handler(request("/"))).status, 404);
  assert.throws(() => createHandler({ store: new MemoryStore(), publicOrigin: "http://example.test" }));
  assert.throws(() => createHandler({ store: new MemoryStore(), publicOrigin: "https://example.test/arbitrary" }));
});

test("publish attempts are limited by source, including invalid declarations", async () => {
  const handler = createHandler({ store: new MemoryStore(), publicOrigin: origin, limiter: new PublishLimiter(1) });
  assert.equal((await handler(request("/api/shares", "POST", "{}"))).status, 400);
  const limited = await handler(request("/api/shares", "POST", JSON.stringify(selection())));
  assert.equal(limited.status, 429);
  assert.ok(Number(limited.headers.get("retry-after")) > 0);
  assert.equal((await limited.json() as { error: string }).error, "rate_limited");
  assert.equal((await handler(request("/api/shares", "POST", JSON.stringify(selection()), { "x-vua-source-ip": "192.0.2.2" }))).status, 201);
});
