import { readFileSync } from "node:fs";
import { createHandler } from "../src/handler.js";
import { MemoryStore } from "../src/storage.js";
import { PublishLimiter } from "../src/rate-limit.js";
import type { Json } from "../src/json.js";

export const origin = "https://shares.example.test";
export const fixture = (relative: string): Json => JSON.parse(readFileSync(new URL(`../../../../schemas/${relative}`, import.meta.url), "utf8")) as Json;
export const selection = (): Json => fixture("selection-share/v0.1/examples/selection.valid.json");
export const recipe = (version: "0.4" | "0.5"): Json => fixture(`amf-recipe/v${version}/share-examples/share.valid.json`);
export const app = () => createHandler({ publicOrigin: origin, store: new MemoryStore(), limiter: new PublishLimiter(1000) });
export const request = (path: string, method = "GET", body?: string, headers: Record<string, string> = {}): Request => new Request(origin + path, {
  method,
  headers: { "content-type": "application/json", "x-vua-source-ip": "192.0.2.1", ...headers },
  ...(body === undefined ? {} : { body }),
});
export interface Receipt { code: string; url: string; deleteToken: string }
export async function publish(handler: (request: Request) => Promise<Response>, declaration: Json = selection()): Promise<Receipt> {
  const response = await handler(request("/api/shares", "POST", JSON.stringify(declaration)));
  if (![200, 201].includes(response.status)) throw new Error(await response.text());
  return await response.json() as Receipt;
}
