// Fit rule: an outfit fits an Avatar when one of the Avatar's names appears in the outfit's own
// text, compared ignoring case, character width and spaces. The Avatar's names are its package
// names without version and extension ("Meiyun_v2.1.0.zip" → "meiyun") and a name its product
// title quotes in 「」 or 『』. Anything unmatched lands with the other outfits and stays selectable.
import { packageBaseName, type AmfSourceChoice } from "@vua/contracts";

/** What names the Avatar: its library product title and the names of its packages. */
export interface AvatarFitNames {
  readonly productName: string | null;
  readonly packageNames: readonly string[];
}
/** The outfit's text as the library knows it; `text` carries anything else observable. */
export interface OutfitFitText {
  readonly productName: string | null;
  readonly materialName: string | null;
  readonly text?: string | null;
}

/** Names shorter than this never mark a fit; a stray letter matches everything. */
const MIN_FIT_NAME_LENGTH = 2;
const QUOTED = /[「『]([^」』]{2,40})[」』]/gu;

/** One comparison form: full-width folds to half-width, case folds, spaces drop. */
export function normalizeFitText(value: string): string {
  return value.normalize("NFKC").toLocaleLowerCase().replace(/\s+/gu, "");
}

/** Every name the Avatar answers to, in comparison form. */
export function avatarFitNames(avatar: AvatarFitNames): readonly string[] {
  const fromPackages = avatar.packageNames.map(name => packageBaseName(name.split("/").pop() ?? name));
  const fromTitle = [...(avatar.productName ?? "").matchAll(QUOTED)].map(match => normalizeFitText(match[1] ?? ""));
  return [...new Set([...fromPackages, ...fromTitle].filter(name => name.length >= MIN_FIT_NAME_LENGTH))];
}

/** True when the outfit's text mentions one of the Avatar's names. */
export function outfitFitsAvatar(avatar: AvatarFitNames, outfit: OutfitFitText): boolean {
  const text = [outfit.productName, outfit.materialName, outfit.text].filter((value): value is string => typeof value === "string" && value.length > 0);
  // Package-style text compares in the package-name form too, so "Tshirt_Meiyun" meets "meiyun".
  const forms = text.flatMap(value => [normalizeFitText(value), packageBaseName(value)]).join(" ");
  return avatarFitNames(avatar).some(name => forms.includes(name));
}

/** Split outfit candidates into the fitted ones and the rest, keeping order. */
export function groupOutfitsByFit<T extends OutfitFitText>(avatar: AvatarFitNames, outfits: readonly T[]): { fitted: readonly T[]; others: readonly T[] } {
  const fitted: T[] = []; const others: T[] = [];
  for (const outfit of outfits) (outfitFitsAvatar(avatar, outfit) ? fitted : others).push(outfit);
  return { fitted, others };
}

/** The preparation request for a line started from the materials room, in the roles the engine
 * accepts: a single Avatar package is named as the Avatar; checked outfits come as automatic
 * materials, as the workshop sends them, so a package without a wearable prefab (a bundled
 * shader, say) never has to be dressed. An Avatar with several packages, or a line without
 * outfits yet, sends everything as automatic; outfits can join the line later. */
export function buildStartChoices(avatarSourceIds: readonly string[], outfitSourceIds: readonly string[]): readonly AmfSourceChoice[] {
  const avatar = [...new Set(avatarSourceIds)];
  const outfits = [...new Set(outfitSourceIds)].filter(sourceId => !avatar.includes(sourceId));
  const named = avatar.length === 1 && outfits.length > 0;
  return [
    ...avatar.map(sourceId => ({ sourceId, role: named ? "avatar" as const : "auto" as const })),
    ...outfits.map(sourceId => ({ sourceId, role: "auto" as const })),
  ];
}
