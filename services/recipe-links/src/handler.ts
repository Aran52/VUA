import { createHash, randomBytes } from "node:crypto";
import { errorResponse } from "./errors.js";
import { canonicalize, MAX_BODY_BYTES, normalizeCode, parseJson, shareCode } from "./json.js";
import { languageFor, materialPage, PAGE_CSP } from "./page.js";
import { PublishLimiter } from "./rate-limit.js";
import { CapacityExceeded, CodeCollision, type ShareStore } from "./storage.js";
import { validateShare } from "./validation.js";

export const tokenHash = (token: string): string => createHash("sha256").update(token).digest("hex");
class TooLarge extends Error {}

async function bodyText(request: Request): Promise<string> {
  const length = request.headers.get("content-length");
  if (length && (!/^\d+$/u.test(length) || Number(length) > MAX_BODY_BYTES)) throw new TooLarge();
  if (!request.body) return "";
  const reader = request.body.getReader();
  let size = 0;
  const chunks: Uint8Array[] = [];
  while (true) {
    const chunk = await reader.read();
    if (chunk.done) break;
    size += chunk.value.byteLength;
    if (size > MAX_BODY_BYTES) {
      void reader.cancel().catch(() => {});
      throw new TooLarge();
    }
    chunks.push(chunk.value);
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
  return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
}

export interface HandlerOptions { store: ShareStore; publicOrigin: string; limiter?: PublishLimiter }

/** Request -> Response; persistence and abuse counters are supplied at composition time. */
export function createHandler({ store, publicOrigin, limiter = new PublishLimiter() }: HandlerOptions): (request: Request) => Promise<Response> {
  const origin = new URL(publicOrigin);
  if (origin.protocol !== "https:" || origin.username || origin.password || origin.pathname !== "/" || origin.search || origin.hash) throw new Error("PUBLIC_ORIGIN must be an HTTPS origin");
  return async request => {
    try {
      const path = new URL(request.url).pathname;
      if (/^\/api\/shares\/?$/iu.test(path)) {
        if (request.method !== "POST") return errorResponse(405, "method_not_allowed", "Use POST to publish a share.", { Allow: "POST" });
        const decision = limiter.take(request.headers.get("x-vua-source-ip") ?? "unknown");
        if (!decision.allowed) return errorResponse(429, "rate_limited", "Publishing limit reached. Try again later.", { "Retry-After": String(decision.retryAfterSeconds) });
        if (request.headers.get("content-type")?.split(";")[0]?.trim().toLowerCase() !== "application/json") return errorResponse(415, "invalid_declaration", "Send an application/json share declaration.");
        let canonical: string;
        try {
          const value = parseJson(await bodyText(request));
          if (!validateShare(value)) return errorResponse(400, "invalid_declaration", "The share declaration is not valid.");
          canonical = canonicalize(value);
          if (Buffer.byteLength(canonical) > MAX_BODY_BYTES) throw new TooLarge();
        } catch (error) {
          if (error instanceof TooLarge) return errorResponse(413, "too_large", "The share declaration exceeds 64 KiB.");
          return errorResponse(400, "invalid_declaration", "The share declaration is not valid JSON or exceeds its limits.");
        }
        const code = shareCode(canonical);
        const deleteToken = randomBytes(32).toString("base64url");
        const { created } = await store.publish(code, canonical, tokenHash(deleteToken));
        return new Response(JSON.stringify({ code, url: `${origin.origin}/r/${code}`, deleteToken }), {
          status: created ? 201 : 200,
          headers: { "Content-Type": "application/json; charset=utf-8", "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff" },
        });
      }
      const api = /^\/api\/shares\/([^/]+)\/?$/iu.exec(path);
      const page = /^\/r\/([^/]+)\/?$/iu.exec(path);
      const code = normalizeCode(api?.[1] ?? page?.[1] ?? "");
      if (!code) return errorResponse(404, "not_found", "This share does not exist.");
      if (api && request.method === "DELETE") {
        const token = /^Bearer ([A-Za-z0-9_-]{43})$/u.exec(request.headers.get("authorization") ?? "")?.[1];
        if (!token || !await store.withdraw(code, tokenHash(token))) return errorResponse(403, "forbidden", "The withdrawal credential is not valid.");
        return new Response(null, { status: 204, headers: { "Cache-Control": "no-store" } });
      }
      if (request.method !== "GET" && request.method !== "HEAD") return errorResponse(405, "method_not_allowed", "This method is not supported.", { Allow: api ? "GET, HEAD, DELETE" : "GET, HEAD" });
      const canonical = await store.get(code);
      if (canonical === null) return errorResponse(404, "not_found", "This share does not exist.");
      if (api) {
        const etag = `"${createHash("sha256").update(canonical).digest("hex")}"`;
        const headers = { "Content-Type": "application/json; charset=utf-8", "Cache-Control": "public, max-age=0, must-revalidate", ETag: etag, "X-Content-Type-Options": "nosniff" };
        if ((request.headers.get("if-none-match") ?? "").split(",").some(value => value.trim().replace(/^W\//u, "") === etag || value.trim() === "*")) return new Response(null, { status: 304, headers });
        return new Response(request.method === "HEAD" ? null : canonical, { headers });
      }
      const share = validateShare(parseJson(canonical));
      if (!share) throw new Error("Corrupt share");
      const language = languageFor(request.headers.get("accept-language") ?? "");
      return new Response(request.method === "HEAD" ? null : materialPage(share, code, language), {
        headers: {
          "Content-Type": "text/html; charset=utf-8", "Content-Language": language, Vary: "Accept-Language",
          "Cache-Control": "no-store", "Content-Security-Policy": PAGE_CSP, "X-Content-Type-Options": "nosniff",
          "Referrer-Policy": "no-referrer", "X-Frame-Options": "DENY", "Permissions-Policy": "camera=(), microphone=(), geolocation=()",
        },
      });
    } catch (error) {
      if (error instanceof CodeCollision) return errorResponse(409, "code_collision", "This code is already assigned to different content.");
      if (error instanceof CapacityExceeded) return errorResponse(503, "capacity_exceeded", "This share has reached its publisher limit.");
      return errorResponse(503, "unavailable", "The share service is temporarily unavailable.");
    }
  };
}
