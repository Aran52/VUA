/** The workshop's line versions: the revisions card's presentation logic, away from the view. */
import type { AmfProductionRevisionSummaryItem } from "../../gateway/index.ts";

/** A known refusal of a version review or restore, named by the next step that resolves it. */
export type RevisionFailure = "sourceMissing" | "drift" | "faceTracking" | "inspect" | "gone";

/**
 * Maps an engine refusal to its known next step. The Gateway throws the engine's sentence when
 * one travels with the code, so the bare code and the sentence's stable words both count.
 */
export function revisionFailure(message: string): RevisionFailure | null {
  if (/revision_source_missing|original packages are unavailable/u.test(message)) return "sourceMissing";
  if (/face_tracking_manual_replay|manual replay/u.test(message)) return "faceTracking";
  if (/revision_missing|no longer retained/u.test(message)) return "gone";
  if (/plan_drift|[Rr]eview (?:the restore|it) again/u.test(message)) return "drift";
  if (/inspect_required|[Rr]einspect/u.test(message)) return "inspect";
  return null;
}

/** The names a version row carries: its Avatar and outfits, else the source packages it held. */
export function revisionNames(revision: AmfProductionRevisionSummaryItem): string {
  const named = [revision.summary.avatar, ...revision.summary.outfits]
    .filter((name): name is string => typeof name === "string" && name.length > 0);
  return (named.length > 0 ? named : revision.summary.sources).join(" · ");
}
