/** Mixed native file/folder intake. Folder-only v0.1 remains available unchanged. */
export interface LibraryImportPathsParamsV02 {
  readonly schemaVersion: "0.2";
  readonly sourcePaths: readonly string[];
  readonly autoGenerate?: boolean;
}
export interface LibraryImportPathsAcceptedV02 {
  readonly schemaVersion: "0.2";
  readonly operation: "library.importPaths";
  readonly taskId: string;
  readonly correlationId: string;
}
export function isLibraryImportPathsParamsV02(value: unknown): value is LibraryImportPathsParamsV02 {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  const p = value as Record<string, unknown>;
  return p.schemaVersion === "0.2"
    && Object.keys(p).every(key => ["schemaVersion", "sourcePaths", "autoGenerate"].includes(key))
    && (!("autoGenerate" in p) || typeof p.autoGenerate === "boolean")
    && Array.isArray(p.sourcePaths) && p.sourcePaths.length > 0 && p.sourcePaths.length <= 200
    && p.sourcePaths.every(s => typeof s === "string" && s.length > 0 && !s.includes("\0") && /^(?:[A-Za-z]:[\\/]|\\\\[^\\]+\\[^\\]+|\/)/.test(s))
    && new Set(p.sourcePaths.map((s: string) => s.replaceAll("\\", "/").toLowerCase())).size === p.sourcePaths.length;
}
export function isLibraryImportPathsAcceptedV02(value: unknown): value is LibraryImportPathsAcceptedV02 {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  const p = value as Record<string, unknown>;
  return p.schemaVersion === "0.2" && p.operation === "library.importPaths"
    && Object.keys(p).length === 4 && typeof p.taskId === "string" && p.taskId.length > 0
    && typeof p.correlationId === "string" && p.correlationId.length > 0;
}
