/** The migration grouping model (freeze row F4 / slice 46): the player confirms material
 *  boundaries over a read-only structural review, and every observed file ends with exactly
 *  one disposition — in one confirmed material, explicitly excluded, or unresolved (which
 *  blocks the import). Groups never overlap, and nothing here touches the disk. */
import type {
  DesktopLibraryDirectoryPreviewV01, LibraryMigrationManifestV01, MigrationGroupV01, MigrationSourceFileV01,
} from "@vua/contracts";

export type MigrationDecision = "material" | "collection" | "excluded";
export interface MigrationLooseGroup {
  readonly id: string;
  /** Directory holding the files; the group's files are its immediate children. */
  readonly parent: string;
  readonly name: string;
  readonly files: readonly string[];
}
interface MigrationSnapshot {
  readonly decisions: ReadonlyMap<string, MigrationDecision>;
  readonly names: ReadonlyMap<string, string>;
  readonly looseGroups: readonly MigrationLooseGroup[];
  readonly excludedFiles: ReadonlySet<string>;
}
export interface MigrationState extends MigrationSnapshot {
  readonly roots: readonly string[];
  readonly previews: ReadonlyMap<string, DesktopLibraryDirectoryPreviewV01>;
  /** Past states, oldest first; each edit appends one so `undo` can step back. */
  readonly history: readonly MigrationSnapshot[];
}

export const emptyMigrationState = (roots: readonly string[]): MigrationState => ({
  roots: [...roots], previews: new Map(), decisions: new Map(), names: new Map(),
  looseGroups: [], excludedFiles: new Set(), history: [],
});
const snapshot = (state: MigrationState): MigrationSnapshot => ({
  decisions: state.decisions, names: state.names, looseGroups: state.looseGroups, excludedFiles: state.excludedFiles,
});
const edited = (state: MigrationState, patch: Partial<MigrationSnapshot>): MigrationState =>
  ({ ...state, ...patch, history: [...state.history, snapshot(state)].slice(-50) });

export function withPreview(state: MigrationState, preview: DesktopLibraryDirectoryPreviewV01): MigrationState {
  const previews = new Map(state.previews); previews.set(preview.path, preview);
  return { ...state, previews };
}
/** `null` clears the decision, returning the directory to unconfirmed. Deciding inside a
 *  directory that was itself a material turns that ancestor into a collection, so the
 *  descendant's boundary takes effect and the ancestor's own loose files become visible as
 *  unresolved — the design's rule instead of two silently overlapping groups. */
export function decide(state: MigrationState, path: string, decision: MigrationDecision | null): MigrationState {
  const decisions = new Map(state.decisions);
  const names = new Map(state.names);
  if (decision === null) { decisions.delete(path); names.delete(path); } else { decisions.set(path, decision); }
  if (decision === "material" || decision === "collection") {
    for (const [ancestor, ancestorDecision] of decisions) {
      if (ancestorDecision === "material" && isUnder(ancestor, path)) { decisions.set(ancestor, "collection"); names.delete(ancestor); }
    }
  }
  // Choosing a boundary for a directory makes the loose groups inside it unreachable.
  const looseGroups = decision === null ? state.looseGroups
    : state.looseGroups.filter(group => group.parent !== path && !isUnder(path, group.parent));
  return edited(state, { decisions, names, looseGroups });
}
export function renameMaterial(state: MigrationState, path: string, name: string): MigrationState {
  const names = new Map(state.names); names.set(path, name);
  return edited(state, { names });
}
/** The bulk command: each named sibling folder becomes its own material. */
export function separateSiblings(state: MigrationState, children: readonly string[]): MigrationState {
  if (children.length === 0) return state;
  const decisions = new Map(state.decisions); const names = new Map(state.names);
  for (const child of children) {
    decisions.set(child, "material");
    if (!names.has(child)) names.set(child, baseName(child));
  }
  return edited(state, { decisions, names });
}
export function groupFiles(state: MigrationState, parent: string, files: readonly string[], name: string): MigrationState {
  if (files.length === 0 || name.trim() === "") return state;
  const excludedFiles = new Set(state.excludedFiles); for (const file of files) excludedFiles.delete(file);
  const looseGroups = [...state.looseGroups.filter(group => !group.files.some(file => files.includes(file))),
    { id: `group-${parent}-${files.length}-${state.looseGroups.length}`, parent, name: name.trim(), files: [...files] }];
  return edited(state, { looseGroups, excludedFiles });
}
export function setFileExcluded(state: MigrationState, path: string, excluded: boolean): MigrationState {
  const excludedFiles = new Set(state.excludedFiles);
  if (excluded) {
    excludedFiles.add(path);
    return edited(state, { excludedFiles, looseGroups: state.looseGroups.map(group => ({ ...group, files: group.files.filter(file => file !== path) })).filter(group => group.files.length > 0) });
  }
  excludedFiles.delete(path);
  return edited(state, { excludedFiles });
}
export function undo(state: MigrationState): MigrationState {
  const previous = state.history[state.history.length - 1];
  if (previous === undefined) return state;
  return { ...state, ...previous, history: state.history.slice(0, -1) };
}
export const canUndo = (state: MigrationState): boolean => state.history.length > 0;

/* ---- Structure ---- */

export const normalizePath = (value: string): string => value.replaceAll("\\", "/");
export function baseName(path: string): string {
  const parts = normalizePath(path).split("/").filter(part => part.length > 0);
  return parts[parts.length - 1] ?? path;
}
/** The path inside `root`, or null when `path` is not inside it. */
export function relativeUnder(root: string, path: string): string | null {
  const base = normalizePath(root).replace(/\/+$/u, "").toLowerCase();
  const target = normalizePath(path);
  const lower = target.toLowerCase();
  if (lower === base) return "";
  if (!lower.startsWith(`${base}/`)) return null;
  return target.slice(base.length + 1);
}
function isUnder(parent: string, child: string): boolean {
  const relative = relativeUnder(parent, child);
  return relative !== null && relative !== "";
}
/** The migration root that contains a path, or null. */
export function rootOf(state: MigrationState, path: string): string | null {
  return state.roots.find(root => relativeUnder(root, path) !== null) ?? null;
}

/* ---- Coverage ---- */

export type MigrationProblem = "not_reviewed" | "truncated" | "unreadable" | "empty" | "no_root";
export interface MigrationMaterial {
  readonly id: string;
  readonly name: string;
  readonly root: string;
  readonly files: readonly MigrationSourceFileV01[];
  readonly problem: MigrationProblem | null;
}
export interface MigrationUnassigned { readonly path: string; readonly kind: "directory" | "file"; readonly name: string }
export interface MigrationExcluded { readonly root: string; readonly path: string }
export interface MigrationUnreadable { readonly path: string; readonly reason: "link" | "other" | "unavailable" | "truncated" }
export interface MigrationCoverage {
  readonly materials: readonly MigrationMaterial[];
  readonly unassigned: readonly MigrationUnassigned[];
  readonly excluded: readonly MigrationExcluded[];
  readonly unreadable: readonly MigrationUnreadable[];
  /** True only when every observed file has a disposition and every confirmed material is
   *  complete; the import button stays disabled otherwise. */
  readonly complete: boolean;
}

interface Walk {
  materials: MigrationMaterial[];
  unassigned: MigrationUnassigned[];
  excluded: MigrationExcluded[];
  unreadable: MigrationUnreadable[];
}

/** Files under a directory that the review has loaded, with their paths relative to the
 *  migration root. A directory whose contents were never loaded, or whose read failed or hit
 *  the limit, makes the material incomplete rather than silently smaller. */
function collectFiles(state: MigrationState, root: string, directory: string, walk: Walk, material: MigrationMaterial,
  seen: Set<string>): void {
  const preview = state.previews.get(directory);
  const fail = (problem: MigrationProblem, reason: MigrationUnreadable["reason"] | null) => {
    walk.unreadable.push({ path: directory, reason: reason ?? "unavailable" });
    if (material.problem === null) (material as { problem: MigrationProblem | null }).problem = problem;
  };
  if (preview === undefined) { fail("not_reviewed", "unavailable"); return; }
  if (preview.state !== "ready") { fail("unreadable", "unavailable"); return; }
  if (preview.truncated) { fail("truncated", "truncated"); }
  for (const item of preview.items) {
    if (item.kind === "link") { walk.unreadable.push({ path: item.path, reason: "link" }); if (material.problem === null) (material as { problem: MigrationProblem | null }).problem = "unreadable"; continue; }
    if (item.kind === "other") { walk.unreadable.push({ path: item.path, reason: "other" }); if (material.problem === null) (material as { problem: MigrationProblem | null }).problem = "unreadable"; continue; }
    if (item.kind === "directory") { collectFiles(state, root, item.path, walk, material, seen); continue; }
    const relative = relativeUnder(root, item.path);
    if (relative === null || relative === "") { if (material.problem === null) (material as { problem: MigrationProblem | null }).problem = "no_root"; continue; }
    if (seen.has(item.path)) continue;
    seen.add(item.path);
    (material.files as MigrationSourceFileV01[]).push({ path: relative, bytes: item.sizeBytes ?? 0, modifiedMs: item.modifiedMs ?? 0 });
  }
}

/** Walks one collection level: every child directory needs a decision, every loose file a
 *  group or an exclusion. */
function walk(state: MigrationState, root: string, directory: string, walk_: Walk, seen: Set<string>): void {
  const preview = state.previews.get(directory);
  if (preview === undefined || preview.state !== "ready") {
    walk_.unreadable.push({ path: directory, reason: "unavailable" });
    walk_.unassigned.push({ path: directory, kind: "directory", name: baseName(directory) });
    return;
  }
  if (preview.truncated) walk_.unreadable.push({ path: directory, reason: "truncated" });
  const grouped = new Map<string, MigrationLooseGroup>();
  for (const group of state.looseGroups) for (const file of group.files) grouped.set(file, group);
  for (const item of preview.items) {
    if (item.kind === "link") { walk_.unreadable.push({ path: item.path, reason: "link" }); continue; }
    if (item.kind === "other") { walk_.unreadable.push({ path: item.path, reason: "other" }); continue; }
    if (item.kind === "directory") {
      const decision = state.decisions.get(item.path);
      if (decision === "material") {
        const material: MigrationMaterial = { id: item.path, name: state.names.get(item.path) ?? item.name, root,
          files: [], problem: null };
        const materialSeen = new Set<string>();
        collectFiles(state, root, item.path, walk_, material, materialSeen);
        if (material.files.length === 0 && material.problem === null) (material as { problem: MigrationProblem | null }).problem = "empty";
        for (const file of materialSeen) seen.add(file);
        walk_.materials.push(material);
      } else if (decision === "collection") {
        walk(state, root, item.path, walk_, seen);
      } else if (decision === "excluded") {
        walk_.excluded.push({ root, path: relativeUnder(root, item.path) ?? item.name });
      } else {
        walk_.unassigned.push({ path: item.path, kind: "directory", name: item.name });
      }
      continue;
    }
    const group = grouped.get(item.path);
    if (group !== undefined) continue; // accounted for below, once per group
    if (state.excludedFiles.has(item.path)) {
      walk_.excluded.push({ root, path: relativeUnder(root, item.path) ?? item.name });
      continue;
    }
    walk_.unassigned.push({ path: item.path, kind: "file", name: item.name });
  }
  for (const group of state.looseGroups) {
    if (group.parent !== directory) continue;
    const material: MigrationMaterial = { id: group.id, name: group.name, root, files: [], problem: null };
    for (const file of group.files) {
      const item = preview.items.find(candidate => candidate.path === file);
      const relative = relativeUnder(root, file);
      if (item === undefined || item.kind !== "file" || relative === null || relative === "") {
        (material as { problem: MigrationProblem | null }).problem = "not_reviewed";
        continue;
      }
      (material.files as MigrationSourceFileV01[]).push({ path: relative, bytes: item.sizeBytes ?? 0, modifiedMs: item.modifiedMs ?? 0 });
    }
    if (material.problem === null && material.files.length === 0) (material as { problem: MigrationProblem | null }).problem = "empty";
    walk_.materials.push(material);
  }
}

export function coverage(state: MigrationState): MigrationCoverage {
  const walk_: Walk = { materials: [], unassigned: [], excluded: [], unreadable: [] };
  for (const root of state.roots) walk(state, root, root, walk_, new Set());
  // A directory chosen as a collection but never opened is incomplete: the design reports it
  // instead of counting its contents as absent.
  const complete = walk_.unassigned.length === 0
    && walk_.unreadable.every(item => item.reason === "link" || item.reason === "other")
    && walk_.materials.every(material => material.problem === null && material.files.length > 0)
    && walk_.materials.length > 0;
  return { materials: walk_.materials, unassigned: walk_.unassigned, excluded: walk_.excluded,
    unreadable: walk_.unreadable, complete };
}

/** The groups to import: confirmed, complete materials only. */
export function migrationGroups(state: MigrationState): MigrationGroupV01[] {
  return coverage(state).materials
    .filter(material => material.problem === null && material.files.length > 0)
    .map(material => ({ name: material.name, root: material.root, files: [...material.files] }));
}

/** The reviewed collection as one versioned document (slice 47 shares this shape). */
export function migrationManifest(state: MigrationState): LibraryMigrationManifestV01 {
  const viewed = coverage(state);
  return {
    kind: "vua.migration",
    formatVersion: "0.1",
    reviewedRoots: state.roots.map(root => ({ path: root, files: countObservedFiles(state, root) })),
    materials: migrationGroups(state),
    excluded: viewed.excluded.map(item => ({ root: item.root, path: item.path.replaceAll("\\", "/") })),
  };
}
function countObservedFiles(state: MigrationState, root: string): number {
  let count = 0;
  for (const [path, preview] of state.previews) {
    if (relativeUnder(root, path) === null || preview.state !== "ready") continue;
    count += preview.items.filter(item => item.kind === "file").length;
  }
  return count;
}
