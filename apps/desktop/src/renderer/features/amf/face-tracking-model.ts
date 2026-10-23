/** The face-tracking add-on station: capability reading and refusal classification. */
import type { FaceTrackingFactsV01 } from "@vua/contracts";

/** The station's one-line answer: a complete reference set on any mesh, some matches, or none. */
export type FtCapability = "full" | "part" | "none";
export function ftCapability(facts: FaceTrackingFactsV01): FtCapability {
  const profiles = facts.meshes.flatMap(mesh => mesh.profiles);
  if (profiles.some(profile => profile.matched.length === profile.referenceCount)) return "full";
  if (profiles.some(profile => profile.matched.length > 0)) return "part";
  return "none";
}

/** A known refusal of a menu review or a patch start, named by the step that resolves it. */
export type FtFailure = "inspect" | "menu" | "drift" | "projectOpen";

/**
 * Maps an engine refusal to its known next step. The Gateway throws the engine's sentence when
 * one travels with the code, so the bare code and the sentence's stable words both count.
 */
export function ftFailure(message: string): FtFailure | null {
  if (/project_open|project open/u.test(message)) return "projectOpen";
  if (/face_tracking_menu_unavailable|unambiguous imported menu/u.test(message)) return "menu";
  if (/plan_drift|[Rr]eview the current creator patch again/u.test(message)) return "drift";
  if (/inspect_required|[Ii]nspect a completed line|identity is missing/u.test(message)) return "inspect";
  return null;
}
