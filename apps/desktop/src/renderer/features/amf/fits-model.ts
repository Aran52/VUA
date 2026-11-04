/** Saved fits: the presentation logic of the workshop's save prompt and the control center's list. */

/**
 * The default name of a fit saved without a chosen name, mirroring the engine's rule:
 * the garment's item name plus `For` plus the Avatar's item name, without whitespace,
 * capped at 80 characters, `Fit` when nothing remains.
 */
export function defaultFitName(garmentName: string, avatarName: string): string {
  const name = [...`${garmentName}For${avatarName}`].filter(char => !/\s/u.test(char)).slice(0, 80).join("");
  return name.length > 0 ? name : "Fit";
}

/** A fit name is a single line of 1–80 characters without control characters that names something. */
export function validFitName(name: string): boolean {
  const chars = [...name];
  return chars.length >= 1 && chars.length <= 80 && !chars.some(char => /\p{Cc}/u.test(char)) && chars.some(char => !/\s/u.test(char));
}

/** A known refusal of saving, renaming or deleting a fit, named by the step that resolves it. */
export type FitFailure = "invalid" | "drift" | "missing" | "baseline" | "limit" | "gone";

/**
 * Maps an engine refusal to its known next step. The Gateway throws the engine's sentence when
 * one travels with the code, so the bare code and the sentence's stable words both count.
 */
export function fitFailure(message: string): FitFailure | null {
  if (/fit_invalid|single line of 1-80 characters/u.test(message)) return "invalid";
  if (/plan_drift|adjustments changed/u.test(message)) return "drift";
  if (/fit_changes_missing|no adjustments to save|not part of the line|not fully observed/u.test(message)) return "missing";
  if (/fit_baseline_missing|baseline (?:is missing|does not cover)/u.test(message)) return "baseline";
  if (/fit_limit|At most \d+ fits are kept/u.test(message)) return "limit";
  if (/fit_not_found|no saved fit with this ID/u.test(message)) return "gone";
  return null;
}
