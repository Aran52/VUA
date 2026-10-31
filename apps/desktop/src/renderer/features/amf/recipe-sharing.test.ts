import { describe, expect, it, vi } from "vitest";
import type { DesktopRecipeSharingV01 } from "@vua/contracts";
import { checkedRecipeSharing } from "./recipe-sharing.ts";

const api = (values: Partial<Record<keyof DesktopRecipeSharingV01, unknown>>): DesktopRecipeSharingV01 => ({
  exportImage: async () => values.exportImage as never,
  readInput: async () => values.readInput as never,
  readImage: async () => values.readImage as never,
  list: async () => values.list as never,
  withdraw: vi.fn(async () => {}),
  hasPending: async () => false,
  takePending: async () => null,
  subscribe: () => () => {},
});

describe("recipe sharing results are checked in the renderer", () => {
  it("is absent without the desktop API", () => {
    expect(checkedRecipeSharing(undefined)).toBeUndefined();
  });
  it("rejects values that are not the contract's shapes", async () => {
    const sharing = checkedRecipeSharing(api({ readInput: { code: 1 }, readImage: null, exportImage: "saved", list: [{}] }))!;
    await expect(sharing.readInput("x")).rejects.toThrow("share_invalid");
    await expect(sharing.readImage([])).rejects.toThrow("share_invalid");
    await expect(sharing.exportImage({} as never)).rejects.toThrow("share_invalid");
    await expect(sharing.list()).rejects.toThrow("share_invalid");
  });
  it("passes valid lists and the unchecked calls through", async () => {
    const raw = api({ list: [] });
    const sharing = checkedRecipeSharing(raw)!;
    await expect(sharing.list("recipe-1")).resolves.toEqual([]);
    await sharing.withdraw("ABCDEFGHJKMN");
    expect(raw.withdraw).toHaveBeenCalledWith("ABCDEFGHJKMN");
  });
});
