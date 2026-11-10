import { afterEach, describe, expect, it, vi } from "vitest";
import { createCatalogEnricher, readCatalogProduct, type CatalogEnrichmentOptions, type CatalogProductReader } from "./catalog-enrich.js";

const productId = "booth:99990001";
const receipt = (id = productId) => ({ok:true as const,value:{schemaVersion:"0.1",productId:id}});
function reader(overrides: Partial<CatalogProductReader> = {}): CatalogProductReader {
  return {fetch:vi.fn(async url => ({status:200,body:"synthetic",finalUrl:url})),observe:vi.fn(async params => receipt(params.productId)),...overrides};
}
function scheduler(overrides: Partial<CatalogEnrichmentOptions> = {}) {
  let clock = 0;
  const options: CatalogEnrichmentOptions = {
    canRun:()=>true, queue:vi.fn(async () => [productId]), claim:vi.fn(async () => true),
    read:vi.fn(async (_id, _signal, beforeRead) => { await beforeRead(); return "updated"; }),
    refreshed:vi.fn(), now:()=>clock, sleep:vi.fn(async ms => {clock += ms;}), ...overrides,
  };
  return {options,worker:createCatalogEnricher(options)};
}
afterEach(() => {vi.useRealTimers();});

describe("single product JSON/HTML transport", () => {
  it("uses the canonical JSON first and persists the caller's observation time", async () => {
    const options = reader({now:()=>Date.parse("2026-11-10T00:00:00Z")});
    expect(await readCatalogProduct(productId,options,new AbortController().signal)).toBe("updated");
    expect(options.fetch).toHaveBeenCalledTimes(1);
    expect(options.observe).toHaveBeenCalledWith(expect.objectContaining({productId,format:"json",sourceUrl:"https://booth.pm/zh-cn/items/99990001.json",fetchedAt:"2026-11-10T00:00:00.000Z"}));
  });
  it.each(["unrecognized", "404"])("falls back to HTML for %s JSON", async cause => {
    const options = reader({
      fetch:vi.fn(async url => ({status:cause === "404" && url.endsWith(".json") ? 404 : 200,body:"synthetic",finalUrl:url})),
      observe:vi.fn(async params => params.format === "json" ? {ok:false as const,error:{code:"vua.catalog.invalid_product"}} : receipt()),
    });
    const before = vi.fn(async () => {});
    expect(await readCatalogProduct(productId,options,new AbortController().signal,before)).toBe("updated");
    expect(options.fetch).toHaveBeenCalledTimes(2); expect(before).toHaveBeenCalledTimes(2);
    expect(options.observe).toHaveBeenLastCalledWith(expect.objectContaining({format:"html",sourceUrl:"https://booth.pm/zh-cn/items/99990001"}));
  });
  it.each([403,429,500,503])("stops without an HTML retry on HTTP %s", async status => {
    const options = reader({fetch:vi.fn(async url => ({status,body:"synthetic",finalUrl:url}))});
    expect(await readCatalogProduct(productId,options,new AbortController().signal)).toBe("stop");
    expect(options.fetch).toHaveBeenCalledTimes(1); expect(options.observe).not.toHaveBeenCalled();
  });
  it("keeps explicit HTML refresh available with an older AMF", async () => {
    const legacy = vi.fn(async () => true);
    const older = reader({observe:async()=>({ok:false,error:{code:"vua.provider.unknown_method"}}),observeLegacyHtml:legacy});
    expect(await readCatalogProduct(productId,older,new AbortController().signal)).toBe("updated");
    expect(older.fetch).toHaveBeenCalledTimes(2);
    expect(legacy).toHaveBeenCalledOnce(); expect(legacy).toHaveBeenCalledWith(expect.objectContaining({format:"html",productId}));
  });
  it("stops after network errors and rejects misleading receipts or redirects", async () => {
    for (const options of [
      reader({fetch:async () => {throw Error("private transport detail");}}),
      reader({observe:async () => receipt("booth:99990002")}),
      reader({fetch:async () => ({status:200,body:"synthetic",finalUrl:"https://accounts.booth.pm/login"})}),
      reader({fetch:async () => ({status:200,body:"synthetic",finalUrl:"https://booth.pm/ja/items/99990002.json"})}),
      reader({fetch:async () => ({status:200,body:"synthetic",finalUrl:"https://booth.pm.synthetic.example/ja/items/99990001.json"})}),
    ]) expect(await readCatalogProduct(productId,options,new AbortController().signal)).toBe("stop");
  });
  it("bounds bytes and does not start cancelled or arbitrary product requests", async () => {
    const options = reader({fetch:vi.fn(async url => ({status:200,body:"猫".repeat(1_666_667),finalUrl:url}))});
    expect(await readCatalogProduct(productId,options,new AbortController().signal)).toBe("stop");
    expect(options.observe).not.toHaveBeenCalled();
    await expect(readCatalogProduct(productId,options,AbortSignal.abort())).rejects.toBeDefined();
    expect(await readCatalogProduct("https://other.example/",options,new AbortController().signal)).toBe("missing");
    expect(options.fetch).toHaveBeenCalledTimes(1);
  });
});

describe("application-lifetime enrichment scheduling", () => {
  it("does nothing at construction and waits for sixty seconds of startup idle", async () => {
    vi.useFakeTimers();
    const {options,worker} = scheduler();
    expect(options.queue).not.toHaveBeenCalled();
    worker.scheduleIdle(); await vi.advanceTimersByTimeAsync(59_999);
    expect(options.queue).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(options.queue).toHaveBeenCalledOnce(); expect(options.refreshed).toHaveBeenCalledWith(1);
    worker.dispose();
  });
  it("paces every background request, including JSON-to-HTML fallback", async () => {
    let clock = 0; const times: number[] = [];
    const product = reader({fetch:async url => {times.push(clock);return {status:200,body:"synthetic",finalUrl:url};},observe:async params => params.format === "json" ? {ok:false,error:{code:"vua.catalog.invalid_product"}} : receipt(params.productId)});
    const {worker,options} = scheduler({queue:async()=>[productId,"booth:99990002"],now:()=>clock,sleep:async ms=>{clock+=ms;},read:(id,signal,before)=>readCatalogProduct(id,product,signal,before)});
    await worker.afterSync();
    expect(times).toEqual([0,5000,10000,15000]); expect(options.refreshed).toHaveBeenCalledWith(2);
    worker.dispose();
  });
  it("never exceeds sixty claimed products when repeatedly triggered or remounted", async () => {
    const {worker,options} = scheduler({queue:async limit => Array.from({length:limit},(_,i)=>`booth:${i+1}`)});
    await worker.afterSync(); worker.pause(); worker.scheduleIdle(); await worker.afterSync();
    expect(options.claim).toHaveBeenCalledTimes(60); expect(options.read).toHaveBeenCalledTimes(60);
    expect(options.refreshed).toHaveBeenCalledTimes(1); expect(options.refreshed).toHaveBeenCalledWith(60);
    worker.dispose();
  });
  it("rechecks membership after waiting for the next request slot", async () => {
    let clock = 0; let owned = true;
    const {worker,options} = scheduler({
      queue:async()=>[productId,"booth:99990002"], now:()=>clock,
      sleep:async ms=>{clock+=ms;owned=false;}, claim:vi.fn(async()=>owned),
    });
    await worker.afterSync();
    expect(options.claim).toHaveBeenCalledTimes(2);
    expect(options.read).toHaveBeenCalledTimes(1);
    expect(options.refreshed).toHaveBeenCalledWith(1);
    worker.dispose();
  });
  it("shares one active run and does not fetch unclaimed products", async () => {
    let release!: (ids: readonly string[]) => void;
    const {worker,options} = scheduler({queue:vi.fn(()=>new Promise(resolve=>{release=resolve;})),claim:vi.fn(async()=>false)});
    const first = worker.afterSync(); const second = worker.afterSync();
    release([productId]); await Promise.all([first,second]);
    expect(options.queue).toHaveBeenCalledOnce(); expect(options.read).not.toHaveBeenCalled();
    expect(options.refreshed).not.toHaveBeenCalled(); worker.dispose();
  });
  it("keeps a failure stop across later syncs and idle triggers", async () => {
    vi.useFakeTimers();
    const {worker,options} = scheduler({read:vi.fn(async()=>"stop")});
    await worker.afterSync(); worker.pause(); worker.scheduleIdle(); await worker.afterSync();
    await vi.advanceTimersByTimeAsync(120_000);
    expect(options.read).toHaveBeenCalledOnce(); expect(options.refreshed).not.toHaveBeenCalled(); worker.dispose();
  });
  it("waits while downloads or AMF readiness prevent reads", async () => {
    vi.useFakeTimers(); let ready = false;
    const {worker,options} = scheduler({canRun:()=>ready});
    await worker.afterSync(); await vi.advanceTimersByTimeAsync(60_000);
    expect(options.queue).not.toHaveBeenCalled(); ready = true;
    await vi.advanceTimersByTimeAsync(60_000);
    expect(options.read).toHaveBeenCalledOnce(); worker.dispose();
  });
  it.each(["pause", "dispose"] as const)("%s aborts an in-flight request and prevents the next product", async action => {
    let began!: () => void; const started = new Promise<void>(resolve=>{began=resolve;}); let aborted = false;
    const {worker,options} = scheduler({queue:async()=>[productId,"booth:99990002"],read:vi.fn(async (_id, signal, before) => {
      await before(); began();
      return new Promise((_resolve,reject)=>signal.addEventListener("abort",()=>{aborted=true;reject(Error("aborted"));},{once:true}));
    })});
    const running = worker.afterSync(); await started; worker[action](); await running;
    expect(aborted).toBe(true); expect(options.read).toHaveBeenCalledOnce(); expect(options.refreshed).not.toHaveBeenCalled(); worker.dispose();
  });
});
