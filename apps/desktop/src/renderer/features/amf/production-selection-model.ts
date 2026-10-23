import { recommendAssembly, type AmfAssemblySelection, type AmfOutfitSelection } from "@vua/contracts";
import type { AmfProductionRun } from "../../gateway/index.ts";

/** What Unity observes, else the confirmed selection, else the stated recommendation of
 * `recommendAssembly` over Unity-observed candidates; equally plausible candidates stay for the player. */
export function initialProductionSelection(run: AmfProductionRun): AmfAssemblySelection {
  const observed = run.result?.facts?.observedSelection;
  if (observed) return observed;
  if (run.selection !== null) return run.selection;
  return recommendAssembly(run.prefabs, run.sources).selection;
}

/** Two selections assemble the same Avatar: the same Avatar and the same outfits, in any order. */
export function sameSelection(a: AmfAssemblySelection, b: AmfAssemblySelection): boolean {
  const key = (selection: AmfAssemblySelection) => JSON.stringify([selection.avatarPrefab,
    selection.outfits.map(o => [o.sourceId, o.prefabPath, o.mode, o.bone, o.toggleName, o.active]).sort()]);
  return key(a) === key(b);
}

export function updateOutfitSelection(selection: AmfAssemblySelection, sourceId: string, update: Partial<AmfOutfitSelection> | null): AmfAssemblySelection {
  const previous = selection.outfits.find(o => o.sourceId === sourceId);
  const others = selection.outfits.filter(o => o.sourceId !== sourceId);
  if (update === null) return { ...selection, outfits: others };
  const merged: AmfOutfitSelection = { sourceId, prefabPath: "", mode: "merge_armature", bone: "", toggleName: "", active: true, ...previous, ...update };
  // Switch options stay out of the selection at their defaults, as the engine writes them.
  const { saved, group, ...plain } = merged;
  const grouped = group && plain.toggleName.trim() ? group : "";
  const next: AmfOutfitSelection = { ...plain, ...(saved === false ? { saved: false } : {}), ...(grouped ? { group: grouped } : {}) };
  // A group's switches are worn one at a time: turning one on by default turns the others off.
  const rest = grouped && next.active ? others.map(o => o.group === grouped && o.active ? { ...o, active: false } : o) : others;
  return { ...selection, outfits: [...rest, next].sort((a,b) => a.sourceId.localeCompare(b.sourceId)) };
}
