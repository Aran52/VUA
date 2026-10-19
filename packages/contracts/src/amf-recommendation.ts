/**
 * Recommends the Avatar and outfit prefabs of a prepared AMF run with stated rules (plan slice 7).
 * A recommendation is where the plan starts; the player confirms or adjusts it. When two
 * candidates are equally plausible, nothing is chosen for that part and the player picks.
 *
 * Avatar, in order: the creator's base for outfits (a prefab named "改変"), the prefab named after
 * its package, the package's top-level prefab; variants such as "2P" or "ChangeColor" give way to
 * the plain prefab at each step. Outfit: the only wearable prefab of its package, else the one
 * whose name names the chosen Avatar.
 */
import type { AmfAssemblySelection, AmfOutfitSelection, AmfSourceRole } from "./amf-production-v01.js";

export interface AmfCandidateFacts {
  readonly sourceId: string; readonly path: string; readonly name: string; readonly isAvatar: boolean;
  readonly hasSkinnedMesh: boolean; readonly hasMergeArmature: boolean; readonly hasMissingScripts: boolean;
}
export interface AmfSourceFacts { readonly sourceId: string; readonly role: AmfSourceRole; readonly itemName: string }
export type AmfAvatarReason = "only" | "creator_base" | "package_name" | "top_level";
export type AmfOutfitReason = "only" | "names_avatar";
export interface AmfAvatarRecommendation { readonly path: string | null; readonly reason: AmfAvatarReason | null; readonly tied: readonly string[] }
export interface AmfOutfitRecommendation { readonly sourceId: string; readonly path: string | null; readonly reason: AmfOutfitReason | null; readonly tied: readonly string[] }
export interface AmfRecommendation {
  readonly selection: AmfAssemblySelection;
  readonly avatar: AmfAvatarRecommendation;
  readonly outfits: readonly AmfOutfitRecommendation[];
}

const fold = (text: string) => text.normalize("NFKC").toLowerCase().replace(/[\s_\-.()（）［］[\]]+/gu, "");
/** "Meiyun_v2.1.0.zip" → "meiyun", "Manuka_ver1.02.zip" → "manuka": a package's own name without
 * its version and extension. */
export function packageBaseName(itemName: string): string {
  const stem = itemName.replace(/\.(zip|unitypackage|7z|rar)$/iu, "")
    .replace(/[\s_-]*(?:ver(?:sion)?\.?|v)?\s*\d+(?:\.\d+)+[a-z]?$/iu, "")
    .replace(/[\s_-]*(?:ver(?:sion)?\.?|v)\s*\d+$/iu, "");
  return fold(stem);
}
const VARIANT = /(?:^|[\s_-])(?:\d+p|change\s*colou?r|colou?r\s*\d*|costume\s*\d*|ver\s*\d+|quest|android|pc|lite|mobile)$/iu;
const isVariant = (name: string) => VARIANT.test(name.trim());
const CREATOR_BASE = /改変|kaihen/iu;
const depth = (path: string) => path.split("/").length;

interface Narrowed<T> { readonly chosen: T | null; readonly rule: number; readonly pool: readonly T[] }
/** Applies the rules in order: a rule matching exactly one candidate (or exactly one plain, non-variant
 * candidate) chooses it; a rule matching several narrows the pool for the next rule. */
function narrow<T extends { readonly name: string }>(items: readonly T[], rules: readonly ((item: T) => boolean)[]): Narrowed<T> {
  let pool = items;
  for (const [index, rule] of rules.entries()) {
    const matches = pool.filter(rule);
    const plain = matches.filter(item => !isVariant(item.name));
    if (plain.length === 1) return { chosen: plain[0]!, rule: index, pool };
    if (matches.length === 1) return { chosen: matches[0]!, rule: index, pool };
    if (matches.length > 1) pool = plain.length > 1 ? plain : matches;
  }
  return { chosen: null, rule: -1, pool };
}

export function recommendAssembly(prefabs: readonly AmfCandidateFacts[], sources: readonly AmfSourceFacts[]): AmfRecommendation {
  // A material the player named as the Avatar holds it; otherwise any automatic material may.
  const named = sources.filter(source => source.role === "avatar");
  const avatarSources = named.length > 0 ? named : sources.filter(source => source.role === "auto");
  const avatarCandidates = [...new Map(prefabs
    .filter(prefab => prefab.isAvatar && !prefab.hasMissingScripts && avatarSources.some(source => source.sourceId === prefab.sourceId))
    .map(prefab => [prefab.path, prefab])).values()];
  // Only packages that hold an Avatar name one; an outfit package's own name is not an Avatar's.
  const nameOf = (sourceId: string) => packageBaseName(sources.find(source => source.sourceId === sourceId)?.itemName ?? "");
  const packageNames = new Set(avatarCandidates.map(candidate => nameOf(candidate.sourceId)).filter(name => name.length > 0));

  let avatar: AmfAvatarRecommendation;
  let chosenAvatar: AmfCandidateFacts | null = null;
  if (avatarCandidates.length === 1) {
    chosenAvatar = avatarCandidates[0]!;
    avatar = { path: chosenAvatar.path, reason: "only", tied: [] };
  } else if (avatarCandidates.length === 0) {
    avatar = { path: null, reason: null, tied: [] };
  } else {
    const shallowest = Math.min(...avatarCandidates.map(candidate => depth(candidate.path)));
    const reasons: readonly AmfAvatarReason[] = ["creator_base", "package_name", "top_level"];
    const result = narrow(avatarCandidates, [
      candidate => CREATOR_BASE.test(candidate.name),
      candidate => packageNames.has(fold(candidate.name)),
      candidate => depth(candidate.path) === shallowest && !isVariant(candidate.name),
    ]);
    chosenAvatar = result.chosen;
    avatar = result.chosen
      ? { path: result.chosen.path, reason: reasons[result.rule] ?? null, tied: [] }
      : { path: null, reason: null, tied: result.pool.map(candidate => candidate.path) };
  }

  // The chosen Avatar's own names: its package name and its prefab name without decorations.
  const avatarNames = new Set((chosenAvatar ? [nameOf(chosenAvatar.sourceId), fold(chosenAvatar.name.replace(CREATOR_BASE, "").replace(VARIANT, ""))] : [...packageNames])
    .filter(name => name.length >= 3));
  const outfits: AmfOutfitRecommendation[] = [];
  const selected: AmfOutfitSelection[] = [];
  for (const source of sources) {
    if (source.role === "dependency" || source.role === "avatar" || source.sourceId === chosenAvatar?.sourceId) continue;
    const wearable = prefabs.filter(prefab => prefab.sourceId === source.sourceId && !prefab.isAvatar && !prefab.hasMissingScripts
      && (prefab.hasSkinnedMesh || prefab.hasMergeArmature));
    if (wearable.length === 0) continue;
    let choice: AmfCandidateFacts | null = null;
    let reason: AmfOutfitReason | null = null;
    let tied: readonly AmfCandidateFacts[] = [];
    if (wearable.length === 1) { choice = wearable[0]!; reason = "only"; }
    else {
      const result = narrow(wearable, [candidate => [...avatarNames].some(name => fold(candidate.name).includes(name))]);
      if (result.chosen) { choice = result.chosen; reason = "names_avatar"; } else tied = result.pool;
    }
    outfits.push({ sourceId: source.sourceId, path: choice?.path ?? null, reason, tied: tied.map(candidate => candidate.path) });
    if (choice) selected.push({ sourceId: source.sourceId, prefabPath: choice.path, mode: "merge_armature", bone: "", toggleName: switchName(choice.name, avatarNames), active: true });
  }
  return { selection: { avatarPrefab: avatar.path ?? "", outfits: selected }, avatar, outfits };
}

/** "TshirtShorts_Meiyun" → "TshirtShorts": the outfit's prefab name without the Avatar's name. */
function switchName(name: string, avatarNames: ReadonlySet<string>): string {
  const parts = name.split(/[\s_-]+/u).filter(part => part.length > 0 && !avatarNames.has(fold(part)));
  const joined = parts.join("_").trim();
  return (joined.length > 0 ? joined : name).slice(0, 80);
}
