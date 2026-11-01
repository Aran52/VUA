import { describe, expect, it, vi } from "vitest";
import { createTermsPort } from "./terms-port.ts";
import type { GatewayClient } from "./gateway-client.ts";
const accepted = { ok: true as const, value: { schemaVersion: "0.1", taskId: "terms-test" } };
describe("terms reading Gateway", () => {
  it("captures the product's current official description before reading its license links", async () => {
    const order: string[] = [];
    const invoke = vi.fn<GatewayClient["invoke"]>(async request => { order.push(request.method); return accepted as Awaited<ReturnType<GatewayClient["invoke"]>>; });
    const capture = vi.fn(async (id: string) => { order.push(id); return { ok: true }; });
    const port = createTermsPort({ invoke, subscribe: () => () => {} }, capture);
    expect(await port.refreshProduct("booth:1", "Sample terms")).toEqual({ taskId: "terms-test" });
    expect(order).toEqual(["booth:1", "terms.refreshProduct"]);
    expect(invoke).toHaveBeenCalledWith(expect.objectContaining({ params: { schemaVersion: "0.1", productId: "booth:1" } }));
  });
  it("does not turn a failed page capture into a successful stale terms refresh", async () => {
    const invoke = vi.fn<GatewayClient["invoke"]>();
    const port = createTermsPort({ invoke, subscribe: () => () => {} }, async () => ({ ok: false }));
    await expect(port.refreshProduct("booth:1", "Sample")).rejects.toThrow("terms_capture_failed");
    expect(invoke).not.toHaveBeenCalled();
  });
  it("rejects malformed product identities before asking the capture service", async () => {
    const invoke = vi.fn<GatewayClient["invoke"]>(), capture = vi.fn(async () => ({ ok: true }));
    const port = createTermsPort({ invoke, subscribe: () => () => {} }, capture);
    await expect(port.refreshProduct("booth:invalid", "Sample")).rejects.toThrow("terms_invalid");
    expect(capture).not.toHaveBeenCalled(); expect(invoke).not.toHaveBeenCalled();
  });
  it("starts an explicit library terms task without changing query behavior", async () => {
    const capture = vi.fn(async () => ({ ok: true }));
    const invoke = vi.fn<GatewayClient["invoke"]>(async () => accepted as Awaited<ReturnType<GatewayClient["invoke"]>>);
    const port = createTermsPort({ invoke, subscribe: () => () => {} }, capture);
    expect(await port.refreshLibrary!("Read recorded terms")).toEqual({ taskId: "terms-test" });
    expect(invoke).toHaveBeenCalledWith(expect.objectContaining({ method: "terms.refreshLibrary", params: { schemaVersion: "0.1" } }));
    expect(capture).not.toHaveBeenCalled();
  });
});
