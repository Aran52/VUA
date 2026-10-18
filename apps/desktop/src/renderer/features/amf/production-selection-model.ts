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

export function updateOutfitSelection(selection: AmfAssemblySelection, sourceId: string, update: Partial<AmfOutfitSelection> | null): AmfAssemblySelection {
  const previous = selection.outfits.find(o => o.sourceId === sourceId);
  const others = selection.outfits.filter(o => o.sourceId !== sourceId);
  if (update === null) return { ...selection, outfits: others };
  const next: AmfOutfitSelection = { sourceId, prefabPath: "", mode: "merge_armature", bone: "", toggleName: "", active: true, ...previous, ...update };
  return { ...selection, outfits: [...others, next].sort((a,b) => a.sourceId.localeCompare(b.sourceId)) };
}
