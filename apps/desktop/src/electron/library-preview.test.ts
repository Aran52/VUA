import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { expect, test } from "vitest";
import { previewLibraryDirectory } from "./library-preview.ts";
test("migration previews preserve mixed structure, report limits, avoid links and warehouse recursion, and never modify sources", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "vua-preview-test-"));
  try {
    const source = path.join(root, "source"); const warehouse = path.join(root, "warehouse");
    await fs.mkdir(source); await fs.mkdir(warehouse);
    await fs.mkdir(path.join(source, "Versions")); await fs.mkdir(path.join(root, "outside"));
    await fs.writeFile(path.join(source, "original.zip"), "unchanged invalid zip");
    await fs.writeFile(path.join(source, "README"), "companion");
    await fs.symlink(path.join(root, "outside"), path.join(source, "linked"), "junction");
    const result = await previewLibraryDirectory(source, warehouse);
    expect(result.state).toBe("ready"); expect(result.truncated).toBe(false);
    expect(result.items.map(item => [item.name, item.kind])).toEqual([["Versions", "directory"], ["linked", "link"], ["original.zip", "file"], ["README", "file"]]);
    // The freshness re-check needs the review's own size and modification time, never a guess.
    const zip = result.items.find(item => item.name === "original.zip");
    const zipStat = await fs.stat(path.join(source, "original.zip"));
    expect(zip?.sizeBytes).toBe(zipStat.size);
    expect(zip?.modifiedMs).toBe(Math.trunc(zipStat.mtimeMs));
    expect(result.items.find(item => item.name === "Versions")?.modifiedMs).toBeNull();
    expect((await previewLibraryDirectory(source, warehouse, 2)).truncated).toBe(true);
    for (const target of [root, warehouse, path.join(source, "linked"), path.join(source, "original.zip"), path.join(root, "missing"), "relative"]) expect((await previewLibraryDirectory(target, warehouse)).state).toBe("unavailable");
    expect(await fs.readFile(path.join(source, "original.zip"), "utf8")).toBe("unchanged invalid zip");
    expect(await fs.readdir(path.join(root, "outside"))).toEqual([]);
  } finally {
    const resolved = path.resolve(root);
    if (path.dirname(resolved) !== path.resolve(os.tmpdir()) || !path.basename(resolved).startsWith("vua-preview-test-")) throw Error("Invalid test cleanup target");
    await fs.rm(resolved, { recursive: true, force: true });
  }
});
