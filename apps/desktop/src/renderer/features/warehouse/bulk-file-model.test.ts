import { describe, expect, it } from "vitest";
import { downloadableFiles, removalProductCount, removalScopes } from "./bulk-file-model.ts";
import type { LibraryFileInventory } from "../../gateway/index.ts";
import type { RemovalDialogTarget } from "./RemoveFilesDialog.tsx";

describe("bulk file scopes", () => {
  it("does not silently expand missing scopes and deduplicates shared copies", () => {
    const items: RemovalDialogTarget[] = [
      { target: { kind: "product", id: "booth:1" }, title: "one", copyIds: ["one", "shared"] },
      { target: { kind: "entry", id: "whi-two" }, title: "two", copyIds: ["shared", "two"] },
      { target: { kind: "product", id: "booth:3" }, title: "no scope" },
    ];
    const result = removalScopes(items);
    expect(result.map(item => item.copyIds)).toEqual([["one", "shared"], ["two"]]);
    expect(removalProductCount(result)).toBe(2);
  });
  it("splits the provider's 200-file bound without losing files or inflating product count", () => {
    const ids = Array.from({ length: 405 }, (_, n) => `synthetic-${n}`);
    const result = removalScopes([{ target: { kind: "product", id: "booth:1" }, title: "one", copyIds: ids }]);
    expect(result.map(item => item.copyIds!.length)).toEqual([200, 200, 5]);
    expect(result.flatMap(item => item.copyIds!)).toEqual(ids);
    expect(removalProductCount(result)).toBe(1);
  });
  it("download fills missing files while re-download explicitly includes present files", () => {
    const inventory: LibraryFileInventory = { productId: "booth:1", items: [
      { downloadableId: 11, fileName: "present.zip", managedCopyId: "p", copies: [{ copyId: "p", entryId: "whi", fileName: "present.zip", artifactSha256: "sha256:a", presence: "present" }] },
      { downloadableId: 12, fileName: "missing.zip", managedCopyId: "m", copies: [{ copyId: "m", entryId: "whi", fileName: "missing.zip", artifactSha256: "sha256:b", presence: "missing" }] },
      { downloadableId: 13, fileName: "new.zip", managedCopyId: null, copies: [] },
    ] };
    expect(downloadableFiles(inventory, "download").map(file => file.downloadableId)).toEqual([12, 13]);
    expect(downloadableFiles(inventory, "redownload").map(file => file.downloadableId)).toEqual([11, 12, 13]);
  });
});
