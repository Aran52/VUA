import { describe, expect, it, vi } from "vitest";
import type { AmfProductionList, AmfProductionPort } from "../../gateway/index.ts";
import { latestProductionList, readProductionList } from "./production-list-read.ts";

const list = { sources: [], editors: [], runs: [] } as unknown as AmfProductionList;
const port = (read: () => Promise<AmfProductionList>) => ({ list: vi.fn(read) }) as unknown as AmfProductionPort & { list: ReturnType<typeof vi.fn> };

describe("the shared production list read", () => {
  it("reuses a read within the age the caller accepts and shares one still running", async () => {
    const shared = port(async () => list);
    const first = readProductionList(shared, 1000);
    expect(readProductionList(shared, 1000)).toBe(first);
    await first;
    expect(shared.list).toHaveBeenCalledTimes(1);
  });
  it("asks again when the caller accepts no age", async () => {
    const shared = port(async () => list);
    await readProductionList(shared);
    await readProductionList(shared);
    expect(shared.list).toHaveBeenCalledTimes(2);
  });
  it("reads again once the tasks have changed", async () => {
    const shared = port(async () => list);
    await readProductionList(shared, 60_000, "a:running");
    await readProductionList(shared, 60_000, "a:running");
    expect(shared.list).toHaveBeenCalledTimes(1);
    await readProductionList(shared, 60_000, "a:completed");
    expect(shared.list).toHaveBeenCalledTimes(2);
  });
  it("keeps the newest list for drawing at once", async () => {
    const shared = port(async () => list);
    expect(latestProductionList(shared)).toBeNull();
    expect(latestProductionList(undefined)).toBeNull();
    await readProductionList(shared);
    expect(latestProductionList(shared)).toBe(list);
  });
  it("reads again after a failure", async () => {
    let fail = true;
    const shared = port(async () => { if (fail) throw new Error("no"); return list; });
    await expect(readProductionList(shared, 60_000)).rejects.toThrow("no");
    await Promise.resolve();
    fail = false;
    await expect(readProductionList(shared, 60_000)).resolves.toBe(list);
    expect(shared.list).toHaveBeenCalledTimes(2);
  });
});
