/** Local library migration v0.1 (freeze row F4 / slice 46): reviewed material boundaries,
 *  coverage and grouped intake. The command carries explicit, named groups of source files;
 *  the provider re-validates freshness, overlap and limits before it copies anything. The
 *  manifest is the same plan as one versioned document — slice 47's agent will write this
 *  shape, and VUA validates everything it executes again. */
export const MIGRATION_GROUPS_LIMIT = 200;
export const MIGRATION_FILES_LIMIT = 20_000;

/** One observed ordinary source file inside a group: `/`-separated path relative to the
 *  group's root, its size, and its modification time for the freshness re-check. */
export interface MigrationSourceFileV01 {
  readonly path: string;
  readonly bytes: number;
  readonly modifiedMs: number;
}
export interface MigrationGroupV01 {
  readonly name: string;
  readonly root: string;
  readonly files: readonly MigrationSourceFileV01[];
}
export interface LibraryImportGroupsParamsV01 {
  readonly schemaVersion: "0.1";
  readonly groups: readonly MigrationGroupV01[];
  readonly autoGenerate?: boolean;
}
export interface LibraryImportGroupsAcceptedV01 {
  readonly schemaVersion: "0.1";
  readonly operation: "library.importGroups";
  readonly taskId: string;
  readonly correlationId: string;
}
/** The reviewed collection as one document: the roots and their observed file counts, the
 *  confirmed materials with their files, and the explicitly excluded files. Files in neither
 *  list are unresolved, which blocks the import. */
export interface LibraryMigrationManifestV01 {
  readonly kind: "vua.migration";
  readonly formatVersion: "0.1";
  readonly reviewedRoots: readonly { readonly path: string; readonly files: number }[];
  readonly materials: readonly MigrationGroupV01[];
  readonly excluded: readonly { readonly root: string; readonly path: string }[];
}

const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const text = (v: unknown, max: number): v is string => typeof v === "string" && v.length > 0
  && [...v].length <= max && !/[\u0000-\u001f\u007f-\u009f]/u.test(v);
const absoluteRoot = (v: unknown): v is string => typeof v === "string" && v.length > 0 && v.length <= 4096
  && !v.includes("\0") && /^(?:[A-Za-z]:[\\/]|\\\\[^\\]+\\[^\\]+|\/)/.test(v);
const safeBytes = (v: unknown): v is number => Number.isSafeInteger(v) && Number(v) >= 0 && Number(v) <= Number.MAX_SAFE_INTEGER;
const safeMilliseconds = (v: unknown): v is number => Number.isSafeInteger(v) && Number(v) >= 0 && Number(v) <= 8.64e15;

/** A Windows file name the group can become: no separators, reserved characters or edge spaces. */
export function isMigrationMaterialName(value: unknown): value is string {
  return text(value, 80) && !/[<>:"/\\|?*]/u.test(value) && !/[. ]$/u.test(value) && value.trim() === value;
}
/** A `/`-separated path inside its root: no drive, no leading separator, no `..` segment. */
export function isMigrationRelativePath(value: unknown): value is string {
  return text(value, 512) && !value.startsWith("/") && !/[\\:]/u.test(value)
    && value.split("/").every(part => part.length > 0 && part.length <= 255 && part !== "." && part !== "..");
}
export function isMigrationSourceFileV01(value: unknown): value is MigrationSourceFileV01 {
  return record(value) && Object.keys(value).length === 3
    && isMigrationRelativePath(value.path) && safeBytes(value.bytes) && safeMilliseconds(value.modifiedMs);
}
export function isMigrationGroupV01(value: unknown): value is MigrationGroupV01 {
  if (!record(value) || !Object.keys(value).every(key => ["name", "root", "files"].includes(key))
    || Object.keys(value).length !== 3 || !isMigrationMaterialName(value.name) || !absoluteRoot(value.root)
    || !Array.isArray(value.files) || value.files.length < 1 || value.files.length > MIGRATION_FILES_LIMIT
    || !value.files.every(isMigrationSourceFileV01)) return false;
  const paths = value.files.map(file => (file as MigrationSourceFileV01).path);
  return new Set(paths).size === paths.length;
}
export function isLibraryImportGroupsParamsV01(value: unknown): value is LibraryImportGroupsParamsV01 {
  if (!record(value) || !Object.keys(value).every(key => ["schemaVersion", "groups", "autoGenerate"].includes(key))
    || value.schemaVersion !== "0.1" || (value.autoGenerate !== undefined && typeof value.autoGenerate !== "boolean")
    || !Array.isArray(value.groups) || value.groups.length < 1 || value.groups.length > MIGRATION_GROUPS_LIMIT
    || !value.groups.every(isMigrationGroupV01)) return false;
  const names = value.groups.map(group => (group as MigrationGroupV01).name.toLowerCase());
  if (new Set(names).size !== names.length) return false;
  if (value.groups.reduce((total, group) => total + (group as MigrationGroupV01).files.length, 0) > MIGRATION_FILES_LIMIT) return false;
  // A file belongs to exactly one group; the same root-relative path in two groups is an
  // overlap the provider must refuse rather than silently drop.
  const keys = value.groups.flatMap(group => (group as MigrationGroupV01).files
    .map(file => `${(group as MigrationGroupV01).root.replaceAll("\\", "/").toLowerCase()}\u0000${file.path.toLowerCase()}`));
  return new Set(keys).size === keys.length;
}
export function isLibraryImportGroupsAcceptedV01(value: unknown): value is LibraryImportGroupsAcceptedV01 {
  return record(value) && value.schemaVersion === "0.1" && value.operation === "library.importGroups"
    && Object.keys(value).length === 4 && text(value.taskId, 256) && text(value.correlationId, 256);
}
export function isLibraryMigrationManifestV01(value: unknown): value is LibraryMigrationManifestV01 {
  return record(value) && value.kind === "vua.migration" && value.formatVersion === "0.1"
    && Array.isArray(value.reviewedRoots) && value.reviewedRoots.length > 0 && value.reviewedRoots.length <= MIGRATION_GROUPS_LIMIT
    && value.reviewedRoots.every(root => record(root) && absoluteRoot(root.path) && safeBytes(root.files))
    && Array.isArray(value.materials) && value.materials.every(isMigrationGroupV01)
    && Array.isArray(value.excluded) && value.excluded.length <= MIGRATION_FILES_LIMIT
    && value.excluded.every(item => record(item) && absoluteRoot(item.root) && isMigrationRelativePath(item.path));
}
