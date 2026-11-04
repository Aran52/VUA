import { createHash, randomUUID } from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import { canonicalShareJson, parseShareJson, decodeAmfRecipeShare, isAmfSharedRecipe, normalizeShareCode, shareCodeFromLink, SHARE_CODE_ALPHABET, SHARE_MAX_BYTES,
  type AmfSharedRecipe, type PublishedRecipeLink, type ReceivedRecipeLink } from "@vua/contracts";

export function declarationCode(document: unknown): string {
  const canonical = canonicalShareJson(document);
  const hash = createHash("sha256").update(canonical, "utf8").digest("hex");
  const bits = BigInt(`0x${hash.slice(0,15)}`);
  let code = "";
  for (let shift = 55n; shift >= 0n; shift -= 5n) code += SHARE_CODE_ALPHABET[Number((bits >> shift) & 31n)];
  return code;
}

interface Claim extends PublishedRecipeLink { readonly deleteTokens: readonly string[] }
interface Receipt { code: string; url: string; deleteToken: string }
export interface ShareClientOptions {
  origin: string;
  directory: string;
  fetch: typeof fetch;
  assertReady: () => void;
}
const MAX_STORE = 4 * 1024 * 1024;

/** Main-only anonymous transport and withdrawal ownership. No token leaves this class. */
export class RecipeShareClient {
  readonly origin: string;
  private readonly file: string;
  private queue: Promise<unknown> = Promise.resolve();
  constructor(private readonly options: ShareClientOptions) {
    const origin = new URL(options.origin);
    if (origin.protocol !== "https:" || origin.username || origin.password || origin.pathname !== "/" || origin.search || origin.hash) throw new Error("share_origin_invalid");
    this.origin = origin.origin;
    this.file = path.join(options.directory, "share-links.json");
  }
  private exclusive<T>(action: () => Promise<T>): Promise<T> {
    const next = this.queue.catch(() => {}).then(action);
    this.queue = next;
    return next;
  }
  private async load(): Promise<Claim[]> {
    try {
      if ((await fs.stat(this.file)).size > MAX_STORE) throw new Error("share_store_invalid");
      const value: unknown = JSON.parse(await fs.readFile(this.file,"utf8"));
      if (!Array.isArray(value) || value.length > 10000) throw new Error("share_store_invalid");
      for (const claim of value as Claim[]) {
        if (!claim || normalizeShareCode(claim.code) !== claim.code || claim.url !== `${this.origin}/r/${claim.code}`
          || !/^[A-Za-z0-9-]{1,100}$/u.test(claim.recipeId) || !Number.isFinite(Date.parse(claim.publishedAt))
          || !Array.isArray(claim.deleteTokens) || claim.deleteTokens.length < 1 || claim.deleteTokens.some(token => !/^[A-Za-z0-9_-]{43}$/u.test(token))) throw new Error("share_store_invalid");
      }
      return value as Claim[];
    } catch (error) { if ((error as NodeJS.ErrnoException).code === "ENOENT") return []; throw new Error("share_store_invalid"); }
  }
  private async save(claims: readonly Claim[]): Promise<void> {
    const text = JSON.stringify(claims);
    if (Buffer.byteLength(text) > MAX_STORE) throw new Error("share_store_full");
    await fs.mkdir(this.options.directory,{recursive:true,mode:0o700});
    const temporary = this.file + `.${randomUUID()}.tmp`;
    try {
      const file = await fs.open(temporary,"wx",0o600);
      try { await file.writeFile(text,"utf8"); await file.sync(); } finally { await file.close(); }
      await fs.rename(temporary,this.file);
    } finally { await fs.unlink(temporary).catch(error=>{if((error as NodeJS.ErrnoException).code!=="ENOENT")throw error;}); }
  }
  private async request(route: string, init: RequestInit = {}, allowForbidden = false): Promise<unknown> {
    let response: Response;
    try {
      response = await this.options.fetch(this.origin + route,{...init,redirect:"error",credentials:"omit",signal:AbortSignal.timeout(15000)});
    } catch { throw new Error("share_network"); }
    const discard = () => { void response.body?.cancel().catch(() => {}); };
    if (allowForbidden && response.status === 403) { discard(); return null; }
    if (response.status === 204) { discard(); return null; }
    if (!response.ok) {
      discard();
      const errors: Record<number,string> = {400:"share_invalid",409:"share_invalid",422:"share_invalid",404:"share_removed",413:"share_too_large",429:"share_rate_limited"};
      throw new Error(errors[response.status] ?? "share_network");
    }
    if (!/^application\/json(?:;|$)/iu.test(response.headers.get("content-type") ?? "")) { discard(); throw new Error("share_invalid"); }
    const declared = Number(response.headers.get("content-length") ?? 0);
    if (declared > SHARE_MAX_BYTES) { discard(); throw new Error("share_too_large"); }
    const reader = response.body?.getReader();
    if (!reader) throw new Error("share_invalid");
    const chunks: Uint8Array[] = []; let size=0;
    while (true) {
      const chunk=await reader.read().catch(()=>{throw new Error("share_network");});
      if(chunk.done)break;
      size+=chunk.value.byteLength;
      if(size>SHARE_MAX_BYTES){void reader.cancel().catch(()=>{});throw new Error("share_too_large");}
      chunks.push(chunk.value);
    }
    try { return parseShareJson(new TextDecoder("utf-8",{fatal:true}).decode(Buffer.concat(chunks))); }
    catch { throw new Error("share_invalid"); }
  }
  publish(document: AmfSharedRecipe, recipeId: string): Promise<PublishedRecipeLink> {
    return this.exclusive(async()=>{
      this.options.assertReady();
      if(!isAmfSharedRecipe(document)||!/^[A-Za-z0-9-]{1,100}$/u.test(recipeId))throw new Error("share_invalid");
      const canonical=canonicalShareJson(document);
      if([...document.recipe.title].length>120)throw new Error("share_invalid");
      if(Buffer.byteLength(canonical)>SHARE_MAX_BYTES)throw new Error("share_too_large");
      const claims=await this.load();
      const code=declarationCode(document);
      const receipt=await this.request("/api/shares",{method:"POST",headers:{"content-type":"application/json"},body:canonical}) as Receipt;
      if(!receipt||Object.keys(receipt).length!==3||receipt.code!==code||receipt.url!==`${this.origin}/r/${code}`||!/^[A-Za-z0-9_-]{43}$/u.test(receipt.deleteToken))throw new Error("share_invalid");
      const existing=claims.find(claim=>claim.code===code);
      const link={code,url:receipt.url,recipeId,publishedAt:new Date().toISOString()};
      const next: Claim={...link,deleteTokens:[...(existing?.deleteTokens??[]),receipt.deleteToken]};
      try { await this.save([...claims.filter(claim=>claim.code!==code),next]); }
      catch {
        await this.request(`/api/shares/${code}`,{method:"DELETE",headers:{authorization:`Bearer ${receipt.deleteToken}`}},true).catch(()=>{});
        throw new Error("share_store_failed");
      }
      this.options.assertReady();
      return link;
    });
  }
  async read(text: string): Promise<ReceivedRecipeLink> {
    this.options.assertReady();
    if(typeof text!=="string"||text.length>349557)throw new Error("share_invalid");
    const code=shareCodeFromLink(text.trim(),this.origin);
    if(!code){const document=decodeAmfRecipeShare(text);if(!document)throw new Error("share_invalid");return {document,text,code:null,preview:null};}
    const value=await this.request(`/api/shares/${code}`);
    if(declarationCode(value)!==code||!isAmfSharedRecipe(value))throw new Error("share_invalid");
    this.options.assertReady();
    return {document:value,text:canonicalShareJson(value),code,preview:null};
  }
  list(recipeId?: string): Promise<readonly PublishedRecipeLink[]> {
    return this.exclusive(async()=>{this.options.assertReady();return(await this.load()).filter(claim=>recipeId===undefined||claim.recipeId===recipeId).map(({code,url,recipeId,publishedAt})=>({code,url,recipeId,publishedAt}));});
  }
  withdraw(code: string): Promise<void> {
    return this.exclusive(async()=>{
      this.options.assertReady();
      const normalized=normalizeShareCode(code); if(!normalized)throw new Error("share_invalid");
      let claims=await this.load();const claim=claims.find(value=>value.code===normalized);if(!claim)throw new Error("share_not_published");
      for(const token of claim.deleteTokens){
        await this.request(`/api/shares/${normalized}`,{method:"DELETE",headers:{authorization:`Bearer ${token}`}},true);
        claims=claims.map(value=>value.code===normalized?{...value,deleteTokens:value.deleteTokens.filter(item=>item!==token)}:value).filter(value=>value.deleteTokens.length>0);
        await this.save(claims);
      }
    });
  }
}
