import assert from "node:assert/strict";
import { test } from "vitest";
import { storageKeys } from "../../app/storage-keys.ts";
import {
  CONTEXT_GUIDES,
  contextGuideForPage,
  dismissContextGuide,
  readContextGuideSnoozed,
  readContextGuideState,
  resetContextGuides,
  snoozeContextGuide,
  type ContextGuideStore,
} from "./context-guidance.ts";

function memoryStore(): ContextGuideStore & { data: Map<string, string> } {
  const data = new Map<string, string>();
  return {
    data,
    getItem: key => data.get(key) ?? null,
    setItem: (key, value) => { data.set(key, value); },
    removeItem: key => { data.delete(key); },
  };
}

test("each main module has at most one contextual introduction; other pages have none", () => {
  for (const [page, id] of Object.entries({
    home: "home", "env-play": "play", "tools-discover": "tools", "env-create": "creator",
    warehouse: "warehouse", recipe: "recipe", workshop: "workshop", release: "release", packages: "packages",
  })) {
    assert.equal(contextGuideForPage(page as Parameters<typeof contextGuideForPage>[0])?.id, id);
  }
  for (const page of ["help", "help-wizard", "settings-theme", "asset-browser"] as const) {
    assert.equal(contextGuideForPage(page), null);
  }
  assert.equal(Object.keys(CONTEXT_GUIDES).length, 9);
});

test("invalid or partial stored dismissals never become a completion claim", () => {
  const store = memoryStore();
  for (const raw of [null, "not-json", "null", "[]", '{"v":2,"dismissed":{}}', '{"v":1,"dismissed":[]}']) {
    store.data.set(storageKeys.contextGuide, raw ?? "null");
    assert.deepEqual(readContextGuideState(store), { v: 1, dismissed: {} });
  }
  store.data.set(storageKeys.contextGuide, JSON.stringify({ v: 1, dismissed: { home: 1, unknown: 99, play: -1, creator: "1", warehouse: 2 } }));
  assert.deepEqual(readContextGuideState(store), { v: 1, dismissed: { home: 1, warehouse: 2 } });
});

test("permanent dismissal preserves the other preference and unrelated storage", () => {
  const store = memoryStore();
  store.data.set(storageKeys.theme, "dark");
  store.data.set(storageKeys.contextGuide, JSON.stringify({ v: 1, dismissed: { play: 1 } }));
  assert.deepEqual(dismissContextGuide(store, CONTEXT_GUIDES.home), { v: 1, dismissed: { play: 1, home: 1 } });
  assert.equal(store.data.get(storageKeys.theme), "dark");
  assert.equal(store.data.get(storageKeys.contextGuideSnoozed), undefined);
});

test("snooze is session-local, deduplicated and independent of permanent dismissal", () => {
  const session = memoryStore();
  snoozeContextGuide(session, "home");
  snoozeContextGuide(session, "home");
  snoozeContextGuide(session, "play");
  assert.deepEqual(readContextGuideSnoozed(session), ["home", "play"]);
  assert.equal(session.data.get(storageKeys.contextGuide), undefined);
  session.data.set(storageKeys.contextGuideSnoozed, '["home","bogus",4]');
  assert.deepEqual(readContextGuideSnoozed(session), ["home"]);
});

test("replay clears only guide dismissal and snooze", () => {
  const local = memoryStore();
  const session = memoryStore();
  dismissContextGuide(local, CONTEXT_GUIDES.home);
  snoozeContextGuide(session, "play");
  local.data.set(storageKeys.guideReading, '{"topic":"guide-safety"}');
  local.data.set(storageKeys.theme, "light");
  resetContextGuides(local, session);
  assert.equal(local.data.has(storageKeys.contextGuide), false);
  assert.equal(session.data.has(storageKeys.contextGuideSnoozed), false);
  assert.equal(local.data.get(storageKeys.guideReading), '{"topic":"guide-safety"}');
  assert.equal(local.data.get(storageKeys.theme), "light");
});
