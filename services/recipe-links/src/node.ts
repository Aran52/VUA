import { createServer, type IncomingMessage, type Server } from "node:http";
import { isIP } from "node:net";
import { isAbsolute, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { errorResponse } from "./errors.js";
import { createHandler } from "./handler.js";
import { MAX_BODY_BYTES } from "./json.js";
import { PublishLimiter } from "./rate-limit.js";
import { DirectoryStore } from "./storage.js";

export function normalizeIp(value: string): string | null {
  if (value.startsWith("::ffff:") && isIP(value.slice(7)) === 4) return value.slice(7);
  if (isIP(value) === 4) return value;
  if (isIP(value) === 6) return new URL(`http://[${value}]/`).hostname.slice(1, -1);
  return null;
}

export function sourceIp(peer: string, forwarded: string | undefined, trustProxy: boolean): string | null {
  const normalized = normalizeIp(peer);
  if (!normalized) return null;
  const loopback = normalized === "::1" || normalized.startsWith("127.");
  return trustProxy && loopback ? (forwarded ? normalizeIp(forwarded) : null) : normalized;
}

class BodyLimit extends Error {}
function boundedBody(request: IncomingMessage): Promise<Buffer> {
  return new Promise((resolveBody, reject) => {
    const chunks: Buffer[] = [];
    let size = 0;
    let exceeded = false;
    request.on("data", (chunk: Buffer) => {
      if (exceeded) return;
      size += chunk.length;
      if (size > MAX_BODY_BYTES) {
        exceeded = true;
        chunks.length = 0;
        reject(new BodyLimit());
      } else chunks.push(chunk);
    });
    request.on("end", () => { if (!exceeded) resolveBody(Buffer.concat(chunks)); });
    request.on("error", reject);
    request.on("aborted", () => reject(new Error("Aborted")));
  });
}

export interface ServerOptions { publicOrigin: string; dataDirectory: string; port: number; trustProxy: boolean; publishesPerHour: number }
export async function startServer(options: ServerOptions): Promise<Server> {
  const store = new DirectoryStore(options.dataDirectory);
  await store.initialize();
  const handler = createHandler({ store, publicOrigin: options.publicOrigin, limiter: new PublishLimiter(options.publishesPerHour) });
  const server = createServer({ requestTimeout: 15_000, headersTimeout: 10_000, keepAliveTimeout: 5000, maxHeaderSize: 8192 }, (incoming, outgoing) => {
    const send = async (response: Response) => {
      if (outgoing.destroyed) return;
      outgoing.writeHead(response.status, Object.fromEntries(response.headers));
      outgoing.end(Buffer.from(await response.arrayBuffer()));
    };
    void (async () => {
      const ip = sourceIp(incoming.socket.remoteAddress ?? "", typeof incoming.headers["x-real-ip"] === "string" ? incoming.headers["x-real-ip"] : undefined, options.trustProxy);
      if (!ip || !incoming.url?.startsWith("/") || incoming.url.startsWith("//")) {
        incoming.resume();
        return send(errorResponse(400, "invalid_declaration", "The request is not valid."));
      }
      const headers = new Headers();
      for (const name of ["content-type", "content-length", "authorization", "accept-language", "if-none-match"]) {
        const value = incoming.headers[name];
        if (typeof value === "string") headers.set(name, value);
      }
      // The internal source header is always reconstructed, never forwarded from the caller.
      headers.set("x-vua-source-ip", ip);
      const length = incoming.headers["content-length"];
      if (length && Number(length) > MAX_BODY_BYTES) {
        incoming.resume();
        return send(errorResponse(413, "too_large", "The request body exceeds 64 KiB."));
      }
      const method = incoming.method ?? "GET";
      try {
        const body = method === "GET" || method === "HEAD" ? undefined : await boundedBody(incoming);
        if (!body) incoming.resume();
        const request = new Request(new URL(incoming.url, options.publicOrigin), { method, headers, ...(body ? { body: new Uint8Array(body) } : {}) });
        await send(await handler(request));
      } catch (error) {
        incoming.resume();
        await send(error instanceof BodyLimit
          ? errorResponse(413, "too_large", "The request body exceeds 64 KiB.")
          : errorResponse(400, "invalid_declaration", "The request could not be read."));
      }
    })().catch(() => send(errorResponse(503, "unavailable", "The share service is temporarily unavailable."))).catch(() => outgoing.destroy());
  });
  server.maxConnections = 64;
  server.on("clientError", (_error, socket) => {
    if (socket.writable) socket.end('HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{"error":"invalid_declaration","message":"The request is not valid."}');
  });
  await new Promise<void>((resolveListening, reject) => {
    server.once("error", reject);
    server.listen(options.port, "127.0.0.1", () => { server.off("error", reject); resolveListening(); });
  });
  return server;
}

export function serverOptions(environment: NodeJS.ProcessEnv): ServerOptions {
  const integer = (name: string, fallback: number, maximum: number): number => {
    const value = Number(environment[name] ?? fallback);
    if (!Number.isSafeInteger(value) || value < 1 || value > maximum) throw new Error(`Invalid ${name}`);
    return value;
  };
  if (!environment.DATA_DIR || !isAbsolute(environment.DATA_DIR)) throw new Error("DATA_DIR must be an absolute path");
  if (environment.TRUST_PROXY && !["0", "1"].includes(environment.TRUST_PROXY)) throw new Error("Invalid TRUST_PROXY");
  return {
    publicOrigin: environment.PUBLIC_ORIGIN ?? "https://ph-r.com",
    dataDirectory: resolve(environment.DATA_DIR),
    port: integer("PORT", 6191, 65535),
    trustProxy: environment.TRUST_PROXY === "1",
    publishesPerHour: integer("PUBLISHES_PER_HOUR", 20, 1000),
  };
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const server = await startServer(serverOptions(process.env));
    const stop = () => {
      server.close(() => process.exit(0));
      setTimeout(() => { server.closeAllConnections(); process.exit(0); }, 10_000).unref();
    };
    process.once("SIGTERM", stop);
    process.once("SIGINT", stop);
    console.info("VUA share-link service is listening on loopback.");
  } catch {
    console.error("VUA share-link service could not start. Check configuration and data-directory access.");
    process.exitCode = 1;
  }
}
