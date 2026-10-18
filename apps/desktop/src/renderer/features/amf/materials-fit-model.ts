// Fit rule: an outfit fits an Avatar when one of the Avatar's names — its product
// or material names, plus the spellings recorded below — appears in the outfit's
// own text, compared ignoring case, character width and spaces; anything unmatched
// lands with the other outfits and stays selectable.
import type { AmfSourceChoice } from "@vua/contracts";

/** The Avatar's own names: the library product's title and its material names. */
export interface AvatarFitNames {
  readonly productName: string | null;
  readonly materialName: string | null;
}
/** The outfit's text as the library knows it; `text` carries anything else
 * observable (labels, the description's target-Avatar line). */
export interface OutfitFitText {
  readonly productName: string | null;
  readonly materialName: string | null;
  readonly text?: string | null;
}

/** Alternative spellings of an Avatar's name, collected only from spellings seen
 * in real warehouse product names (Meiyun ships as "Meiyun" and "Meiyu"); never
 * invented here. The key is the spelling the Avatar's own product uses. */
const AVATAR_NAME_SPELLINGS: Readonly<Record<string, readonly string[]>> = {
  Meiyun: ["Meiyun", "Meiyu"],
};

/** Names shorter than this never mark a fit; a stray letter matches everything. */
const MIN_FIT_NAME_LENGTH = 2;

/** One comparison form: full-width folds to half-width, case folds, spaces drop. */
export function normalizeFitText(value: string): string {
  return value.normalize("NFKC").toLocaleLowerCase().replace(/\s+/gu, "");
}

function cleanNames(values: readonly (string | null | undefined)[]): string[] {
  return values.filter((value): value is string => typeof value === "string" && value.length > 0).map(normalizeFitText);
}

/** Every name the Avatar answers to, in comparison form. */
export function avatarFitNames(avatar: AvatarFitNames): readonly string[] {
  const own = cleanNames([avatar.productName, avatar.materialName]);
  const names = new Set(own.filter(name => name.length >= MIN_FIT_NAME_LENGTH));
  for (const spellings of Object.values(AVATAR_NAME_SPELLINGS)) {
    const forms = cleanNames(spellings);
    if (own.some(name => forms.some(form => name.includes(form)))) {
      for (const form of forms) if (form.length >= MIN_FIT_NAME_LENGTH) names.add(form);
    }
  }
  return [...names];
}

/** True when the outfit's text mentions one of the Avatar's names. */
export function outfitFitsAvatar(avatar: AvatarFitNames, outfit: OutfitFitText): boolean {
  const text = normalizeFitText(cleanNames([outfit.productName, outfit.materialName, outfit.text]).join(" "));
  return avatarFitNames(avatar).some(name => text.includes(name));
}

/** Split outfit candidates into the fitted ones and the rest, keeping order. */
export function groupOutfitsByFit<T extends OutfitFitText>(avatar: AvatarFitNames, outfits: readonly T[]): { fitted: readonly T[]; others: readonly T[] } {
  const fitted: T[] = []; const others: T[] = [];
  for (const outfit of outfits) (outfitFitsAvatar(avatar, outfit) ? fitted : others).push(outfit);
  return { fitted, others };
}

/** The preparation request for a line started from the materials room: the
 * Avatar's materials carry the avatar role, every checked outfit the outfit role. */
export function buildStartChoices(avatarSourceIds: readonly string[], outfitSourceIds: readonly string[]): readonly AmfSourceChoice[] {
  const choices: AmfSourceChoice[] = [];
  const avatar = new Set(avatarSourceIds);
  for (const sourceId of avatar) choices.push({ sourceId, role: "avatar" });
  for (const sourceId of new Set(outfitSourceIds)) if (!avatar.has(sourceId)) choices.push({ sourceId, role: "outfit" });
  return choices;
}
