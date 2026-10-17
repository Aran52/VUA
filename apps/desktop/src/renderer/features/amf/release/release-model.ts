/** The release room's view of finished Avatars: names, outfits, checks, uploads and page turns. */
import type { AmfProductionList, AmfProductionRun, AmfReleaseState, AmfUpload } from "../../../gateway/index.ts";

/** "Meiyun_v2.1.0/Meiyun.unitypackage" → "Meiyun". */
export const baseName = (value: string) => value.split(/[\\/]/u).pop()!.replace(/\.(unitypackage|zip|prefab)$/iu, "");

/** Finished Avatars, newest first. */
export function finishedRunIds(list: AmfProductionList | null): string[] {
  return (list?.runs ?? []).filter(run => run.phase === "succeeded").map(run => run.runId);
}

/** The Avatar's own name, as Unity reports it, else its prefab or package. */
export function avatarTitle(run: AmfProductionRun): string {
  const facts = run.result?.facts;
  const avatar = run.sources.find(source => source.role === "avatar");
  return facts?.avatarName || (run.selection ? baseName(run.selection.avatarPrefab) : "") || (avatar ? baseName(avatar.itemName) : "") || run.runId;
}

/** The packages that went into the Avatar: "Meiyun + TshirtShorts_Meiyun". */
export function materialsLine(run: AmfProductionRun): string {
  return [...new Set(run.sources.map(source => baseName(source.itemName)))].join(" + ");
}

/** A name VRChat accepts for a new Avatar, proposed from the Avatar and its outfits. */
export function proposedUploadName(run: AmfProductionRun): string {
  const outfits = outfitLines(run).map(outfit => outfit.name);
  const name = [avatarTitle(run), ...outfits].join(" ").replace(/[\p{Cc}\s]+/gu, " ").trim();
  return [...name].slice(0, 64).join("").trim() || run.runId.slice(0, 64);
}

export interface OutfitLine { readonly name: string; readonly toggle: string | null; readonly active: boolean }
/** The outfits as Unity last observed them, else as the plan chose them. */
export function outfitLines(run: AmfProductionRun): OutfitLine[] {
  const selection = run.result?.facts?.observedSelection ?? run.selection;
  return (selection?.outfits ?? []).map(outfit => ({ name: baseName(outfit.prefabPath), toggle: outfit.toggleName.trim() || null, active: outfit.active }));
}

export type CheckKey = "descriptor" | "bindings" | "menus" | "references" | "scripts" | "shaders" | "signIn";
export interface CheckLine { readonly key: CheckKey; readonly state: "ok" | "attention" | "info"; readonly count?: number; readonly signIn?: "yes" | "no" | "unknown" }
/** VUA's own checks of the finished Avatar, in the order a player reads them. */
export function releaseChecks(run: AmfProductionRun): CheckLine[] {
  const facts = run.result?.facts;
  if (!facts) return [];
  const missingShaders = facts.missingShaderMaterials?.length ?? 0;
  const signIn = facts.sdkSignedIn ?? "unknown";
  return [
    { key: "descriptor", state: facts.sdkDescriptorRecognized ? "ok" : "attention" },
    { key: "bindings", state: facts.instances.every(instance => instance.bindingVerified) ? "ok" : "attention" },
    { key: "menus", state: facts.instances.every(instance => instance.menuVerified) ? "ok" : "attention" },
    { key: "references", state: facts.missingReferences === 0 ? "ok" : "attention", count: facts.missingReferences },
    { key: "scripts", state: facts.hasMissingScripts ? "attention" : "ok" },
    { key: "shaders", state: missingShaders === 0 ? "ok" : "attention", count: missingShaders },
    { key: "signIn", state: signIn === "yes" ? "ok" : "info", signIn },
  ];
}

export const runningUpload = (state: AmfReleaseState | null): AmfUpload | null => state?.uploads.find(upload => upload.state === "running") ?? null;
/** The Avatar ID of the newest successful upload: the Avatar that later uploads update. */
export const uploadedAvatarId = (state: AmfReleaseState | null): string | null => state?.uploads.find(upload => upload.state === "succeeded")?.blueprintId ?? null;

/** The newest step with this key, e.g. the last turntable bake. */
export function lastStep(run: AmfProductionRun, key: string) {
  return [...run.steps].reverse().find(step => step.key === key) ?? null;
}
/** Unity work of this run is running now. */
export const runBusy = (run: AmfProductionRun) => run.steps.some(step => step.state === "running");

export type TurnDirection = "next" | "prev";
export interface Turn { readonly from: number; readonly to: number; readonly direction: TurnDirection }
/** One page turn from the shown spread to the target spread. */
export function turnTo(index: number, target: number, count: number): Turn | null {
  if (target === index || target < 0 || target >= count) return null;
  return { from: index, to: target, direction: target > index ? "next" : "prev" };
}
/** Turn progress from a corner drag: a full turn is the width of the open album. */
export function dragProgress(direction: TurnDirection, startX: number, x: number, width: number): number {
  const distance = direction === "next" ? startX - x : x - startX;
  return Math.min(1, Math.max(0, distance / Math.max(1, width)));
}
/** A released page completes its turn past a third of the way. */
export const turnCompletes = (progress: number) => progress >= 1 / 3;
export const easeInOut = (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);

export const STILL_VIEWS = ["front", "side", "back"] as const;
export type StillView = typeof STILL_VIEWS[number];
/** The view a drag of `steps` stills reaches from the current one, wrapping around. */
export function viewAfter(views: readonly StillView[], current: StillView, steps: number): StillView {
  const at = Math.max(0, views.indexOf(current));
  return views[(((at + steps) % views.length) + views.length) % views.length] ?? current;
}
