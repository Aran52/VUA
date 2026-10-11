import { net, type Session } from "electron";
import { isAllowedRemoteOrigin } from "./security.js";

/** A private authenticated HTML read. Electron 44's fetch Response omits its URL;
 * track real redirects through ClientRequest instead of trusting an empty/fallback URL. */
export function readSessionPage(targetSession: Session, url: string, allowedOrigins: readonly string[], signal?: AbortSignal): Promise<{
  readonly status: number; readonly body: string; readonly finalUrl: string;
}> {
  if (!isAllowedRemoteOrigin(url, allowedOrigins)) return Promise.reject(new Error("origin_not_allowed"));
  return new Promise((resolve, reject) => {
    const request = net.request({ session: targetSession, method: "GET", url,
      credentials: "include", redirect: "manual", cache: "no-store",
      headers: { accept: "text/html,application/xhtml+xml" } });
    let finalUrl = url;
    let redirects = 0;
    let settled = false;
    const cleanup = () => signal?.removeEventListener("abort", abort);
    const fail = (error: Error) => {
      if (settled) return;
      settled = true;
      cleanup();
      request.abort();
      reject(error);
    };
    const abort = () => fail(new Error("session_read_aborted"));
    request.on("error", () => fail(new Error("session_read_failed")));
    request.on("redirect", (_status, _method, destination) => {
      if (settled) return;
      if (!isAllowedRemoteOrigin(destination, allowedOrigins)) { fail(new Error("origin_not_allowed")); return; }
      if (++redirects > 10) { fail(new Error("session_redirect_limit")); return; }
      finalUrl = destination;
      request.followRedirect();
    });
    request.on("response", response => {
      const chunks: Buffer[] = [];
      let bytes = 0;
      response.on("error", () => fail(new Error("session_read_failed")));
      response.on("aborted", () => fail(new Error("session_read_aborted")));
      response.on("data", (chunk: Buffer) => {
        if (settled) return;
        bytes += chunk.length;
        if (bytes > 16 * 1024 * 1024) { fail(new Error("session_page_too_large")); return; }
        chunks.push(chunk);
      });
      response.on("end", () => {
        if (settled) return;
        settled = true;
        cleanup();
        resolve({ status: response.statusCode, body: Buffer.concat(chunks).toString("utf8"), finalUrl });
      });
    });
    signal?.addEventListener("abort", abort, { once: true });
    if (signal?.aborted) abort();
    else request.end();
  });
}
