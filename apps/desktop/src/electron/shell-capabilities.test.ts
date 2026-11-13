import { describe, expect, it, vi } from "vitest";

/**
 * 壳能力面同源钉死(#36 缺陷4′ 能力面对齐切片,2026-09-19):
 * preload 是沙箱 preload,运行时不能导入本地模块,壳自报值按
 * DESKTOP_GATEWAY_VERSION 先例持本地字面量——本测试把守该字面量与
 * electron/shell-capabilities 单一事实源同值,信封(gateway-router,同源
 * 引用)与壳自报永不再分裂(#22/#36 live 形状分裂教训同族)。
 * 第三面(provider 不报告不转述)由 015 §11 (a) 移除死常量 DESKTOP_
 * CAPABILITIES 落实,provider 面无 desktop.remoteBrowser 行可断言。
 */

const { exposed, on, removeListener } = vi.hoisted(() => ({ exposed: vi.fn(), on: vi.fn(), removeListener: vi.fn() }));

vi.mock("electron", () => ({
  contextBridge: { exposeInMainWorld: (...args: unknown[]) => exposed(...args) },
  ipcRenderer: {
    invoke: vi.fn(),
    on,
    removeListener,
  },
}));

import { SHELL_CAPABILITIES } from "./shell-capabilities.js";
import "./preload.js";

describe("shell capability single source of truth", () => {
  it("keeps the preload self-report identical to the shell capability face", () => {
    expect(exposed).toHaveBeenCalledTimes(1);
    const [channel, api] = exposed.mock.calls[0] as unknown as [
      string,
      { capabilities: { remoteBrowser: boolean } },
    ];
    expect(channel).toBe("vua");
    expect(api.capabilities.remoteBrowser).toBe(SHELL_CAPABILITIES.remoteBrowser);
  });

  it("passes only bounded enrichment counts and removes its exact subscription", () => {
    const api = exposed.mock.calls[0][1] as {
      catalogSync: { subscribeEnrichment: (listener: (event: { schemaVersion: "0.1"; updated: number }) => void) => () => void };
    };
    const listener = vi.fn();
    const unsubscribe = api.catalogSync.subscribeEnrichment(listener);
    const [channel, wrapped] = on.mock.calls.at(-1)! as [string, (event: unknown, value: unknown) => void];
    expect(channel).toBe("vua:catalog-sync:enriched");
    for (const value of [null, [], {}, { schemaVersion: "0.2", updated: 1 }, { schemaVersion: "0.1", updated: 1, productIds: ["booth:1"] }, ...[0, 61, 1.5, NaN, "1"].map(updated => ({ schemaVersion: "0.1", updated }))]) {
      wrapped({}, value);
    }
    expect(listener).not.toHaveBeenCalled();
    wrapped({}, { schemaVersion: "0.1", updated: 1 });
    wrapped({}, { schemaVersion: "0.1", updated: 60 });
    expect(listener.mock.calls).toEqual([[{ schemaVersion: "0.1", updated: 1 }], [{ schemaVersion: "0.1", updated: 60 }]]);
    unsubscribe();
    expect(removeListener).toHaveBeenCalledWith(channel, wrapped);
  });
});
