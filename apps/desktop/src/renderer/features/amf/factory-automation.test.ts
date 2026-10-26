import assert from "node:assert/strict";
import { afterEach, expect, test, vi } from "vitest";
import { DEFAULT_AUTOMATION_LEVEL, rememberAutomationLevel, storedAutomationLevel } from "./factory-model.ts";

const key = "vua-amf-permission-mode-v01";
afterEach(() => { vi.unstubAllGlobals(); });

function storage(raw: string | null) {
  let value = raw;
  return {
    getItem: (name: string) => name === key ? value : null,
    setItem: (name: string, next: string) => { if (name === key) value = next; },
    read: () => value,
  };
}

test("new, missing and invalid preferences default to full auto", () => {
  expect(DEFAULT_AUTOMATION_LEVEL).toBe("full");
  for (const raw of [null, "", "ask-later", '{"mode":"ask"}']) {
    const store = storage(raw);
    vi.stubGlobal("localStorage", store);
    expect(storedAutomationLevel()).toBe("full");
  }
});

test("existing ask and smart preferences are not migrated to full", () => {
  for (const level of ["ask", "smart", "full"] as const) {
    const store = storage(level);
    vi.stubGlobal("localStorage", store);
    expect(storedAutomationLevel()).toBe(level);
  }
});

test("a deliberate mode change is saved in the existing key", () => {
  const store = storage("full");
  vi.stubGlobal("localStorage", store);
  rememberAutomationLevel("ask");
  expect(store.read()).toBe("ask");
  expect(storedAutomationLevel()).toBe("ask");
});

test("unavailable storage falls back to the declared default without throwing", () => {
  vi.stubGlobal("localStorage", { getItem: () => { throw new Error("blocked"); } });
  expect(storedAutomationLevel()).toBe("full");
});
