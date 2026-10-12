import { EventEmitter } from "node:events";
import { Readable } from "node:stream";
import type { Session } from "electron";
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("electron", () => ({ net: { request: vi.fn() } }));
import { net } from "electron";
import { readSessionPage } from "./session-page.js";

const url = "https://accounts.booth.pm/library?page=1";
const origins = ["https://accounts.booth.pm", "https://accounts.pixiv.net"];
const partition = {} as Session;
class Request extends EventEmitter {
  end = vi.fn(); abort = vi.fn(); followRedirect = vi.fn();
  respond(body = "Synthetic library", statusCode = 200) {
    const response = Object.assign(Readable.from([Buffer.from(body)]), { statusCode });
    this.emit("response", response);
  }
}
let request: Request;
beforeEach(() => { vi.clearAllMocks(); request = new Request(); vi.mocked(net.request).mockReturnValue(request as unknown as ReturnType<typeof net.request>); });
describe("private session HTML reads", () => {
  it("uses the supplied partition and returns a real URL without fetch Response.url", async () => {
    const result = readSessionPage(partition, url, origins);
    request.respond();
    expect(await result).toEqual({ status: 200, body: "Synthetic library", finalUrl: url });
    expect(net.request).toHaveBeenCalledWith(expect.objectContaining({ session: partition, credentials: "include", redirect: "manual", cache: "no-store" }));
  });
  it("reports an allowed login redirect instead of pretending the library responded", async () => {
    const result = readSessionPage(partition, url, origins);
    request.emit("redirect", 302, "GET", "https://accounts.pixiv.net/login");
    request.respond("Synthetic sign-in");
    expect((await result).finalUrl).toBe("https://accounts.pixiv.net/login");
    expect(request.followRedirect).toHaveBeenCalledOnce();
  });
  it("rejects a disallowed target before contacting it", async () => {
    await expect(readSessionPage(partition, "https://other.test/", origins)).rejects.toThrow("origin_not_allowed");
    expect(net.request).not.toHaveBeenCalled();
  });
  it("blocks disallowed redirects without forwarding session credentials", async () => {
    const result = readSessionPage(partition, url, origins);
    request.emit("redirect", 302, "GET", "https://other.test/library");
    await expect(result).rejects.toThrow("origin_not_allowed");
    expect(request.followRedirect).not.toHaveBeenCalled();
    expect(request.abort).toHaveBeenCalledOnce();
  });
  it("bounds redirect loops", async () => {
    const result = readSessionPage(partition, url, origins);
    for (let i = 0; i < 11; i++) request.emit("redirect", 302, "GET", `${url}&repeat=${i}`);
    await expect(result).rejects.toThrow("session_redirect_limit");
    expect(request.followRedirect).toHaveBeenCalledTimes(10);
  });
  it("cancels pending reads and ignores a late successful response", async () => {
    const abort = new AbortController();
    const result = readSessionPage(partition, url, origins, abort.signal);
    abort.abort(); request.emit("redirect", 302, "GET", "https://accounts.pixiv.net/login"); request.respond();
    await expect(result).rejects.toThrow("session_read_aborted");
    expect(request.abort).toHaveBeenCalledOnce();
    expect(request.followRedirect).not.toHaveBeenCalled();
  });
  it("does not start an already cancelled read", async () => {
    const result = readSessionPage(partition, url, origins, AbortSignal.abort());
    await expect(result).rejects.toThrow("session_read_aborted");
    expect(request.end).not.toHaveBeenCalled();
  });
  it("fails on an interrupted response", async () => {
    const result = readSessionPage(partition, url, origins);
    request.emit("error", new Error("synthetic private detail"));
    await expect(result).rejects.toThrow("session_read_failed");
  });
  it("bounds the private HTML response before retaining an oversized page", async () => {
    const result = readSessionPage(partition, url, origins);
    request.respond("x".repeat(16 * 1024 * 1024 + 1));
    await expect(result).rejects.toThrow("session_page_too_large");
    expect(request.abort).toHaveBeenCalledOnce();
  });
});
