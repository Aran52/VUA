import fs from "node:fs/promises";
import path from "node:path";
import { APPLICATION_CONTRACT_VERSION } from "@vua/contracts";
import type { OrchestratorProviderV01 } from "@vua/orchestrator-provider";

/** Resolve an existing entry through AMF; Renderer never chooses a local path. */
export async function openLibraryEntryFolder(
  provider: Pick<OrchestratorProviderV01, "invoke">,
  root: string,
  entryId: unknown,
  openPath: (folder: string) => Promise<string>,
): Promise<{ readonly opened: boolean }> {
  if (typeof entryId !== "string" || entryId.length === 0 || entryId.length > 200 || entryId.includes("\0")) throw new Error("invalid library entry");
  try {
    const response = await provider.invoke({ contractVersion: APPLICATION_CONTRACT_VERSION,
      requestId: crypto.randomUUID(), correlationId: crypto.randomUUID(), kind: "query",
      method: "warehouse.entryDetail", params: { warehouseItemId: entryId } });
    if (!response.ok) return { opened: false };
    const value = response.value as { schemaVersion?: unknown; operation?: unknown; result?: { entry?: { warehouseItemId?: unknown; folderName?: unknown } } };
    const entry = value?.result?.entry;
    const name = entry?.folderName;
    if (value?.schemaVersion !== "0.6" || value.operation !== "warehouse.entryDetail" || entry?.warehouseItemId !== entryId
      || typeof name !== "string" || name === "" || name === "." || name === ".." || /[\\/:\0]/.test(name)) return { opened: false };
    const folder = path.join(root, name);
    const [rootInfo, folderInfo, realRoot, realFolder] = await Promise.all([fs.lstat(root), fs.lstat(folder), fs.realpath(root), fs.realpath(folder)]);
    if (!rootInfo.isDirectory() || rootInfo.isSymbolicLink() || !folderInfo.isDirectory() || folderInfo.isSymbolicLink()) return { opened: false };
    const relative = path.relative(realRoot, realFolder);
    if (relative === "" || relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) return { opened: false };
    return { opened: await openPath(folder) === "" };
  } catch { return { opened: false }; }
}
