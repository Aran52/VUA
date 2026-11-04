import { afterEach, describe, expect, it } from "vitest";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { readFileSync } from "node:fs";
import { isAmfSharedRecipe, type AmfSharedRecipe } from "@vua/contracts";
import { declarationCode, RecipeShareClient } from "./recipe-share-client.ts";

const vectors = JSON.parse(readFileSync(new URL("../../../../schemas/share-links/v0.1/code-vectors.json", import.meta.url),"utf8")) as {name:string;declaration:unknown;code:string}[];
const document = vectors.find(v => v.name === "recipe-v05")!.declaration as AmfSharedRecipe;
const code = declarationCode(document), origin = "https://ph-r.com";
const dirs: string[] = [];
afterEach(async () => { for (const dir of dirs.splice(0)) await rm(dir,{recursive:true,force:true}); });
async function client(fetcher: typeof fetch) {
  const directory = await mkdtemp(path.join(os.tmpdir(),"vua-share-client-")); dirs.push(directory);
  return {client:new RecipeShareClient({origin,directory,fetch:fetcher,assertReady:()=>{}}),directory};
}
const json = (value: unknown) => new Response(JSON.stringify(value),{headers:{"content-type":"application/json"}});
describe("Main-only anonymous Recipe links", () => {
  it("matches all four backend code vectors", () => { for (const vector of vectors) expect(declarationCode(vector.declaration)).toBe(vector.code); expect(isAmfSharedRecipe(document)).toBe(true); });
  it("persists every independent claim atomically and never exposes tokens", async () => {
    const calls: {url:string;init?:RequestInit}[] = []; let issued = 0;
    const {client:c,directory} = await client(async (input,init) => {
      calls.push({url:String(input),init});
      return init?.method === "POST" ? json({code,url:`${origin}/r/${code}`,deleteToken:String.fromCharCode(65+issued++).repeat(43)}) : new Response(null,{status:204});
    });
    await c.publish(document,"recipe-example"); await c.publish(document,"recipe-example");
    const listing = await c.list(); expect(listing).toHaveLength(1); expect(JSON.stringify(listing)).not.toContain("deleteToken");
    const store = JSON.parse(await readFile(path.join(directory,"share-links.json"),"utf8")); expect(store[0].deleteTokens).toHaveLength(2);
    await c.withdraw(code.toLowerCase()); expect(await c.list()).toEqual([]);
    expect(calls.filter(v => v.init?.method === "DELETE")).toHaveLength(2);
    for (const call of calls) { expect(call.url.startsWith(origin + "/api/shares")).toBe(true); expect(call.init?.credentials).toBe("omit"); expect(call.init?.redirect).toBe("error"); }
  });
  it("checks the digest before import and rejects unconfigured hosts without requests", async () => {
    let count = 0;
    const {client:c} = await client(async () => { count++; return json(document); });
    expect((await c.read(`${origin}/r/${code.toLowerCase()}`)).code).toBe(code);
    await expect(c.read(`${origin}/r/000000000000`)).rejects.toThrow("share_invalid");
    await expect(c.read(`https://example.test/r/${code}`)).rejects.toThrow("share_invalid"); expect(count).toBe(2);
  });
  it("distinguishes removed, network, duplicate-name and size failures", async () => {
    for (const [status,message] of [[404,"share_removed"],[429,"share_rate_limited"],[500,"share_network"]] as const) {
      const {client:c} = await client(async () => new Response(null,{status})); await expect(c.read(`${origin}/r/${code}`)).rejects.toThrow(message);
    }
    const {client:huge} = await client(async () => new Response(" ".repeat(65537),{headers:{"content-type":"application/json"}}));
    await expect(huge.read(`${origin}/r/${code}`)).rejects.toThrow("share_too_large");
    const {client:duplicate} = await client(async () => new Response('{"a":1,"a":2}',{headers:{"content-type":"application/json"}}));
    await expect(duplicate.read(`${origin}/r/${code}`)).rejects.toThrow("share_invalid");
  });
  it("retains withdrawal ownership when the network fails and recovers an already spent claim", async () => {
    let fail = false;
    const {client:c} = await client(async (_input,init) => { if (init?.method === "POST") return json({code,url:`${origin}/r/${code}`,deleteToken:"Z".repeat(43)}); if (fail) throw new Error("offline"); return new Response(null,{status:403}); });
    await c.publish(document,"recipe-example"); fail=true;
    await expect(c.withdraw(code)).rejects.toThrow("share_network"); expect(await c.list()).toHaveLength(1);
    fail=false; await c.withdraw(code); expect(await c.list()).toHaveLength(0);
  });
});
