import fs from "node:fs/promises";
import path from "node:path";
import type { DesktopLibraryDirectoryPreviewV01 } from "@vua/contracts";

/** One bounded structural read. Never expands archives or follows directory links. */
export async function previewLibraryDirectory(target: string, warehouseRoot: string, limit = 1000): Promise<DesktopLibraryDirectoryPreviewV01> {
  const unavailable: DesktopLibraryDirectoryPreviewV01 = { path: target, state: "unavailable", truncated: false, items: [] };
  if (target.includes("\0") || !path.isAbsolute(target)) return unavailable;
  try {
    const meta = await fs.lstat(target);
    if (!meta.isDirectory() || meta.isSymbolicLink()) return unavailable;
    const actual = await fs.realpath(target);
    const root = await fs.realpath(warehouseRoot).catch(() => path.resolve(warehouseRoot));
    const contained = (a: string, b: string) => { const relative = path.relative(a, b); return relative === "" || relative !== ".." && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative); };
    if (contained(root, actual) || contained(actual, root)) return unavailable;
    const items: DesktopLibraryDirectoryPreviewV01["items"][number][] = [];
    let truncated = false;
    const directory = await fs.opendir(actual);
    for await (const entry of directory) {
      if (items.length >= limit) { truncated = true; break; }
      const itemPath = path.join(target, entry.name);
      const itemMeta = await fs.lstat(itemPath).catch(() => null);
      const kind = itemMeta?.isSymbolicLink() ? "link" : itemMeta?.isDirectory() ? "directory" : itemMeta?.isFile() ? "file" : "other";
      items.push({ name: entry.name, path: itemPath, kind, sizeBytes: kind === "file" && itemMeta !== null ? itemMeta.size : null });
    }
    items.sort((a, b) => a.kind === b.kind ? a.name.localeCompare(b.name) : a.kind === "directory" ? -1 : b.kind === "directory" ? 1 : a.name.localeCompare(b.name));
    return { path: target, state: "ready", truncated, items };
  } catch { return unavailable; }
}
