import { describe, expect, it } from "vitest";
import type { DesktopLibraryDirectoryPreviewV01 } from "@vua/contracts";
import {
  canUndo, coverage, decide, emptyMigrationState, groupFiles, migrationGroups, migrationManifest,
  relativeUnder, renameMaterial, separateSiblings, setFileExcluded, undo, withPreview, type MigrationState,
} from "./migration-model.ts";

const T = 1_760_000_000_000;
const file = (name: string, bytes = 10, parent = "D:\\lib"): DesktopLibraryDirectoryPreviewV01["items"][number] =>
  ({ name, path: `${parent}\\${name}`, kind: "file", sizeBytes: bytes, modifiedMs: T });
const dir = (name: string, parent = "D:\\lib"): DesktopLibraryDirectoryPreviewV01["items"][number] =>
  ({ name, path: `${parent}\\${name}`, kind: "directory", sizeBytes: null, modifiedMs: null });
const preview = (path: string, items: DesktopLibraryDirectoryPreviewV01["items"], truncated = false): DesktopLibraryDirectoryPreviewV01 =>
  ({ path, state: "ready", truncated, items });

/** A collection with two wrapper folders, a loose pair the player groups, one excluded file
 *  and one link — the mixed shape the design's corner-case matrix names. */
function mixed(): MigrationState {
  let state = emptyMigrationState(["D:\\lib"]);
  state = withPreview(state, preview("D:\\lib", [dir("Coat"), dir("Hat"), file("README.txt"), file("LICENSE.txt"), file("extra.unitypackage"),
    { name: "linked", path: "D:\\lib\\linked", kind: "link", sizeBytes: null, modifiedMs: null }]));
  state = withPreview(state, preview("D:\\lib\\Coat", [file("Coat.unitypackage", 100, "D:\\lib\\Coat"), dir("Textures", "D:\\lib\\Coat")]));
  state = withPreview(state, preview("D:\\lib\\Coat\\Textures", [file("body.png", 50, "D:\\lib\\Coat\\Textures")]));
  state = withPreview(state, preview("D:\\lib\\Hat", [file("Hat.unitypackage", 40, "D:\\lib\\Hat")]));
  return state;
}
const materialOf = (state: MigrationState, name: string) => coverage(state).materials.find(material => material.name === name);

describe("migration grouping model", () => {
  it("keeps every file unresolved until the player confirms boundaries", () => {
    const view = coverage(mixed());
    expect(view.materials).toEqual([]);
    expect(view.unassigned.map(item => item.name)).toEqual(["Coat", "Hat", "README.txt", "LICENSE.txt", "extra.unitypackage"]);
    expect(view.unreadable.map(item => item.reason)).toEqual(["link"]);
    expect(view.complete).toBe(false);
  });

  it("turns an ancestor into a collection when a descendant boundary is chosen, never overlapping", () => {
    let state = decide(mixed(), "D:\\lib\\Coat", "material");
    const coat = materialOf(state, "Coat");
    expect(coat?.files).toEqual([
      { path: "Coat/Coat.unitypackage", bytes: 100, modifiedMs: T },
      { path: "Coat/Textures/body.png", bytes: 50, modifiedMs: T },
    ]);
    // Choosing the child as its own material makes the wrapper a collection: the child's
    // boundary takes effect and the wrapper's own loose file becomes visible as unresolved.
    state = decide(state, "D:\\lib\\Coat\\Textures", "material");
    expect(materialOf(state, "Coat")).toBeUndefined();
    expect(materialOf(state, "Textures")?.files).toEqual([{ path: "Coat/Textures/body.png", bytes: 50, modifiedMs: T }]);
    expect(coverage(state).unassigned.map(item => item.path)).toEqual(expect.arrayContaining(["D:\\lib\\Coat\\Coat.unitypackage"]));
    expect(migrationGroups(state).map(group => group.name)).toEqual(["Textures"]);
  });

  it("applies the bulk sibling command and undoes it", () => {
    const base = mixed();
    const bulk = separateSiblings(base, ["D:\\lib\\Coat", "D:\\lib\\Hat"]);
    expect(coverage(bulk).materials.map(material => material.name)).toEqual(["Coat", "Hat"]);
    expect(canUndo(bulk)).toBe(true);
    const back = undo(bulk);
    expect(coverage(back).materials).toEqual([]);
    expect(coverage(back).unassigned.map(item => item.name)).toEqual(["Coat", "Hat", "README.txt", "LICENSE.txt", "extra.unitypackage"]);
  });

  it("groups loose files into one named material and excludes the rest", () => {
    let state = separateSiblings(mixed(), ["D:\\lib\\Coat", "D:\\lib\\Hat"]);
    state = groupFiles(state, "D:\\lib", ["D:\\lib\\README.txt", "D:\\lib\\LICENSE.txt"], "Coat companion files");
    state = decide(state, "D:\\lib\\extra.unitypackage" as string, null);
    state = setFileExcluded(state, "D:\\lib\\extra.unitypackage", true);
    const view = coverage(state);
    expect(view.materials.map(material => [material.name, material.files.map(file => file.path)])).toEqual([
      ["Coat", ["Coat/Coat.unitypackage", "Coat/Textures/body.png"]],
      ["Hat", ["Hat/Hat.unitypackage"]],
      ["Coat companion files", ["README.txt", "LICENSE.txt"]],
    ]);
    expect(view.excluded).toEqual([{ root: "D:\\lib", path: "extra.unitypackage" }]);
    // A link stays reported as unreadable and does not block the import of what was reviewed.
    expect(view.unreadable.map(item => item.reason)).toEqual(["link"]);
    expect(view.complete).toBe(true);
    const groups = migrationGroups(state);
    expect(groups.map(group => [group.name, group.files.length])).toEqual([["Coat", 2], ["Hat", 1], ["Coat companion files", 2]]);
    const manifest = migrationManifest(state);
    expect(manifest.kind).toBe("vua.migration");
    expect(manifest.reviewedRoots).toEqual([{ path: "D:\\lib", files: 6 }]);
    expect(manifest.excluded).toEqual([{ root: "D:\\lib", path: "extra.unitypackage" }]);
  });

  it("blocks the import while any observed file is unresolved, unreviewed or the read hit a limit", () => {
    let state = separateSiblings(mixed(), ["D:\\lib\\Coat", "D:\\lib\\Hat"]);
    expect(coverage(state).complete).toBe(false); // loose files still unassigned
    let resolved = setFileExcluded(state, "D:\\lib\\README.txt", true);
    resolved = setFileExcluded(resolved, "D:\\lib\\LICENSE.txt", true);
    resolved = setFileExcluded(resolved, "D:\\lib\\extra.unitypackage", true);
    expect(coverage(resolved).complete).toBe(true);
    // A truncated read leaves an unresolved remainder instead of a silent, smaller material.
    let truncated = emptyMigrationState(["D:\\lib"]);
    truncated = withPreview(truncated, preview("D:\\lib", [dir("Coat")], true));
    truncated = decide(decide(truncated, "D:\\lib\\Coat", "material"), "D:\\lib", null);
    truncated = withPreview(truncated, preview("D:\\lib\\Coat", [file("Coat.unitypackage")]));
    expect(coverage(truncated).unreadable.map(item => item.reason)).toContain("truncated");
    expect(coverage(truncated).complete).toBe(false);
    // A material whose contents were never opened is reported, not counted as empty.
    let unopened = emptyMigrationState(["D:\\lib"]);
    unopened = withPreview(unopened, preview("D:\\lib", [dir("Coat")]));
    unopened = decide(unopened, "D:\\lib\\Coat", "material");
    expect(materialOf(unopened, "Coat")?.problem).toBe("not_reviewed");
    expect(coverage(unopened).complete).toBe(false);
  });

  it("renames a material and steps back through history", () => {
    let state = separateSiblings(mixed(), ["D:\\lib\\Coat", "D:\\lib\\Hat"]);
    state = renameMaterial(state, "D:\\lib\\Coat", "Meiyun coat");
    expect(materialOf(state, "Meiyun coat")?.files).toHaveLength(2);
    const back = undo(state);
    expect(materialOf(back, "Coat")?.files).toHaveLength(2);
    expect(coverage(back).materials.some(material => material.name === "Meiyun coat")).toBe(false);
  });

  it("computes relative paths across separators and refuses outside paths", () => {
    expect(relativeUnder("D:\\lib", "D:\\lib\\Coat\\a.png")).toBe("Coat/a.png");
    expect(relativeUnder("D:\\lib\\", "D:/lib/a.png")).toBe("a.png");
    expect(relativeUnder("D:\\lib", "D:\\library\\a.png")).toBeNull();
    expect(relativeUnder("D:\\lib", "D:\\lib")).toBe("");
  });
});
