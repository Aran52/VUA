/** Package page by permission mode (plan 8 points 3–4): full auto reads conclusions
 * only, limited auto adds the next step and a handle-it action, and manual review
 * lists every package, version and feed. Pure helpers; the page wires ports. */
import type { AmfRecipeDocument } from "@vua/contracts";
import type { InstalledPackageRowV01, InstalledPackageRowV02 } from "../../gateway/index.ts";
import type { AutomationLevel } from "./factory-model.ts";

/** How much the packages page shows: "conclusion" (full auto), "next_step"
 * (limited auto) or "table" (manual review lists everything). */
export type PackagePageDetail = "conclusion" | "next_step" | "table";
export function packagePageDetail(level: AutomationLevel): PackagePageDetail {
  return level === "full" ? "conclusion" : level === "smart" ? "next_step" : "table";
}

/** A Recipe's version pin on one package. */
export interface PackagePin { readonly recipeId: string; readonly title: string; readonly version: string }

/** Package id → the Recipes pinning it, in Recipe title order. */
export function recipePinsByPackage(recipes: readonly AmfRecipeDocument[]): ReadonlyMap<string, readonly PackagePin[]> {
  const map = new Map<string, PackagePin[]>();
  for (const recipe of recipes) {
    for (const [packageId, version] of Object.entries(recipe.packages)) {
      const list = map.get(packageId) ?? [];
      list.push({ recipeId: recipe.recipeId, title: recipe.title, version });
      map.set(packageId, list);
    }
  }
  for (const list of map.values()) list.sort((a, b) => a.title.localeCompare(b.title) || a.recipeId.localeCompare(b.recipeId));
  return map;
}

/** One line of pinned display: one package at one pinned version, with the Recipe titles. */
export interface PackagePinLine { readonly packageId: string; readonly version: string; readonly recipes: readonly string[] }

/** The pins touching an installed set, grouped per package and version, sorted for display. */
export function packagePinLines(pins: ReadonlyMap<string, readonly PackagePin[]>, installedIds: ReadonlySet<string>): readonly PackagePinLine[] {
  const lines: PackagePinLine[] = [];
  for (const [packageId, list] of pins) {
    if (!installedIds.has(packageId)) continue;
    const byVersion = new Map<string, string[]>();
    for (const pin of list) {
      const titles = byVersion.get(pin.version) ?? [];
      titles.push(pin.title);
      byVersion.set(pin.version, titles);
    }
    for (const [version, recipes] of byVersion) lines.push({ packageId, version, recipes });
  }
  return lines.sort((a, b) => a.packageId.localeCompare(b.packageId) || a.version.localeCompare(b.version));
}

/** One guide diagnostic's message and its official page. */
export interface GuideNeed { readonly message: string; readonly url: string | null }

/** The backend's fixed `amf.package.guide` message ends with "get it from <url>.";
 * a message off that shape keeps its text but no link. */
export function guideNeeds(diagnostics: readonly { readonly code: string; readonly message: string }[] | undefined): readonly GuideNeed[] {
  return (diagnostics ?? []).filter(diagnostic => diagnostic.code === "amf.package.guide")
    .map(diagnostic => ({ message: diagnostic.message, url: /get it from (\S+?)\.?$/u.exec(diagnostic.message)?.[1] ?? null }));
}

/** The conclusion numbers of an installed list. `updates` is null when the
 * backend answered without the judgment pair (v0.1 rows or a null judgment):
 * the page then makes no up-to-date claim either way. */
export function packageConclusion(rows: readonly (InstalledPackageRowV01 | InstalledPackageRowV02 | { readonly updateAvailable: boolean | null })[]): { readonly installed: number; readonly updates: number | null } {
  if (rows.length === 0) return { installed: 0, updates: 0 };
  const judged = rows.map(row => "updateAvailable" in row && typeof row.updateAvailable === "boolean" ? row.updateAvailable : null);
  if (judged.some(value => value === null)) return { installed: rows.length, updates: null };
  return { installed: rows.length, updates: judged.filter(value => value === true).length };
}
