import { describe, expect, it } from "vitest";
import {
  isLibraryImportGroupsAcceptedV01, isLibraryImportGroupsParamsV01, isLibraryMigrationManifestV01,
  isMigrationGroupV01, isMigrationMaterialName, isMigrationRelativePath,
} from "./library-migration-v01.js";

const file = { path: "Coat/Coat.unitypackage", bytes: 1024, modifiedMs: 1_760_000_000_000 };
const group = { name: "Coat", root: "D:\\Materials", files: [file] };

describe("library migration v0.1", () => {
  it("accepts groups of ordinary relative files and refuses unsafe names or paths", () => {
    expect(isMigrationMaterialName("Coat v2")).toBe(true);
    expect(isMigrationMaterialName(" ")).toBe(false);
    expect(isMigrationMaterialName("Coat.")).toBe(false);
    expect(isMigrationMaterialName("Coat/Edition")).toBe(false);
    expect(isMigrationRelativePath("Coat/Textures/a.png")).toBe(true);
    expect(isMigrationRelativePath("../escape")).toBe(false);
    expect(isMigrationRelativePath("C:\\\\absolute")).toBe(false);
    expect(isMigrationRelativePath("Coat//a.png")).toBe(false);
    expect(isMigrationRelativePath("Coat/./a.png")).toBe(false);
    expect(isMigrationGroupV01(group)).toBe(true);
    expect(isMigrationGroupV01({ ...group, files: [file, file] })).toBe(false);
    expect(isMigrationGroupV01({ ...group, name: "" })).toBe(false);
    expect(isMigrationGroupV01({ ...group, root: "relative/root" })).toBe(false);
    expect(isMigrationGroupV01({ ...group, files: [] })).toBe(false);
  });

  it("refuses duplicate group names and the same file in two groups", () => {
    expect(isLibraryImportGroupsParamsV01({ schemaVersion: "0.1", groups: [group] })).toBe(true);
    expect(isLibraryImportGroupsParamsV01({ schemaVersion: "0.1", groups: [group, { ...group, name: "coat" }] })).toBe(false);
    expect(isLibraryImportGroupsParamsV01({ schemaVersion: "0.1", groups: [
      group, { ...group, name: "Coat copy", files: [{ ...file, bytes: 2048 }] },
    ] })).toBe(false);
    // The same relative path under a different root is a different file.
    expect(isLibraryImportGroupsParamsV01({ schemaVersion: "0.1", groups: [group, { ...group, name: "Other root", root: "E:\\Materials" }] })).toBe(true);
    expect(isLibraryImportGroupsParamsV01({ schemaVersion: "0.1", groups: [] })).toBe(false);
    expect(isLibraryImportGroupsParamsV01({ schemaVersion: "0.1", groups: [group], extra: true })).toBe(false);
  });

  it("pins the accepted identity and the manifest shape", () => {
    expect(isLibraryImportGroupsAcceptedV01({ schemaVersion: "0.1", operation: "library.importGroups", taskId: "t1", correlationId: "c1" })).toBe(true);
    expect(isLibraryImportGroupsAcceptedV01({ schemaVersion: "0.1", operation: "library.importPaths", taskId: "t1", correlationId: "c1" })).toBe(false);
    const manifest = { kind: "vua.migration", formatVersion: "0.1",
      reviewedRoots: [{ path: "D:\\Materials", files: 12 }], materials: [group],
      excluded: [{ root: "D:\\Materials", path: "README.txt" }] };
    expect(isLibraryMigrationManifestV01(manifest)).toBe(true);
    expect(isLibraryMigrationManifestV01({ ...manifest, formatVersion: "0.2" })).toBe(false);
    expect(isLibraryMigrationManifestV01({ ...manifest, reviewedRoots: [] })).toBe(false);
    expect(isLibraryMigrationManifestV01({ ...manifest, excluded: [{ root: "D:\\Materials", path: "/etc/passwd" }] })).toBe(false);
  });
});
