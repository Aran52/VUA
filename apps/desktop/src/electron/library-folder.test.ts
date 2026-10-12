import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { expect, it, vi } from "vitest";
import type { ApplicationRequestV01, ApplicationResponseV01 } from "@vua/contracts";
import { openLibraryEntryFolder } from "./library-folder.js";

it("opens only the existing provider-resolved directory and contains native failures", async () => {
  const base = await fs.mkdtemp(path.join(os.tmpdir(), "vua-library-folder-"));
  const root = path.join(base, "warehouse"); await fs.mkdir(root);
  await fs.mkdir(path.join(root, "whi-existing")); await fs.writeFile(path.join(root, "file.txt"), "synthetic");
  const entry = { warehouseItemId: "entry-1", folderName: "whi-existing" };
  let value: unknown = { schemaVersion: "0.6", operation: "warehouse.entryDetail", result: { entry } };
  const provider = { invoke: vi.fn(async (_request: ApplicationRequestV01) => ({ ok: true, value } as ApplicationResponseV01)) };
  const opened = vi.fn(async () => "");
  try {
    expect(await openLibraryEntryFolder(provider, root, "entry-1", opened)).toEqual({ opened: true });
    expect(opened).toHaveBeenCalledWith(path.join(root, "whi-existing"));
    expect(provider.invoke.mock.calls[0]?.[0]).toMatchObject({ method: "warehouse.entryDetail", kind: "query", params: { warehouseItemId: "entry-1" } });
    for (const name of ["", ".", "..", "../outside", "C:\\outside", "sub/folder", "sub\\folder", "file.txt", "missing"]) {
      entry.folderName = name;
      expect(await openLibraryEntryFolder(provider, root, "entry-1", opened)).toEqual({ opened: false });
    }
    const outside = path.join(base, "outside"); await fs.mkdir(outside); await fs.symlink(outside, path.join(root, "linked"), "junction");
    entry.folderName = "linked";
    expect(await openLibraryEntryFolder(provider, root, "entry-1", opened)).toEqual({ opened: false });
    entry.folderName = "whi-existing";
    expect(await openLibraryEntryFolder(provider, root, "other-entry", opened)).toEqual({ opened: false });
    expect(opened).toHaveBeenCalledTimes(1);
    expect(await openLibraryEntryFolder(provider, root, "entry-1", async () => "native failure")).toEqual({ opened: false });
    expect(await openLibraryEntryFolder(provider, root, "entry-1", async () => { throw Error("private native error"); })).toEqual({ opened: false });
    value = null;
    expect(await openLibraryEntryFolder(provider, root, "entry-1", opened)).toEqual({ opened: false });
    for (const invalid of [null, "", "a\0b", "x".repeat(201)]) await expect(openLibraryEntryFolder(provider, root, invalid, opened)).rejects.toThrow("invalid library entry");
  } finally {
    const relative = path.relative(os.tmpdir(), base);
    if (relative.startsWith("vua-library-folder-") && !path.isAbsolute(relative) && !relative.includes(path.sep)) await fs.rm(base, { recursive: true });
  }
});
