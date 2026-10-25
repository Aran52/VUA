import { afterEach, describe, expect, it, vi } from "vitest";
import type { DesktopComponentUpdatesApiV02 } from "@vua/contracts";
import { checkComponents, componentCheckEnabled, setComponentCheckEnabled } from "./component-update-store.ts";
import { storageKeys } from "./storage-keys.ts";

afterEach(() => vi.unstubAllGlobals());
describe("independent automatic update preferences", () => {
  it("migrates the host preference without coupling the AMF choice", async () => {
    const values = new Map<string, string>();
    vi.stubGlobal("localStorage", { getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => values.set(key, value) });
    vi.stubGlobal("window", { dispatchEvent: vi.fn() });
    values.set(storageKeys.updateCheckEnabled, "off");
    expect(componentCheckEnabled("host")).toBe(false); expect(componentCheckEnabled("amf")).toBe(true);
    const check = vi.fn(async () => { throw Error("offline"); });
    const port = { check } as unknown as DesktopComponentUpdatesApiV02;
    await checkComponents(port); expect(check.mock.calls).toEqual([["amf"]]);
    setComponentCheckEnabled("host", true); setComponentCheckEnabled("amf", false);
    expect(componentCheckEnabled("host")).toBe(true); expect(componentCheckEnabled("amf")).toBe(false);
    check.mockClear(); await checkComponents(port); expect(check.mock.calls).toEqual([["host"]]);
    check.mockClear(); await checkComponents(port, true); expect(check.mock.calls).toEqual([["host"], ["amf"]]);
  });
});
