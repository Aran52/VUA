import type { PageId } from "../../app/nav-model.ts";
import { storageKeys } from "../../app/storage-keys.ts";
import { GUIDE_TARGETS, type GuideTarget } from "../guide/guide-target.ts";

export type ContextGuideId = "home" | "play" | "tools" | "creator" | "warehouse" | "recipe" | "workshop" | "release" | "packages";
export interface ContextGuideDefinition {
  readonly id: ContextGuideId;
  readonly revision: number;
  readonly action: "tour" | "guide" | "none";
  readonly target: GuideTarget | null;
}

export const CONTEXT_GUIDES: Readonly<Record<ContextGuideId, ContextGuideDefinition>> = {
  home: { id: "home", revision: 1, action: "tour", target: null },
  play: { id: "play", revision: 1, action: "guide", target: { topic: "guide-start", section: "prepare" } },
  tools: { id: "tools", revision: 1, action: "none", target: null },
  creator: { id: "creator", revision: 1, action: "guide", target: GUIDE_TARGETS.uploadEligibility },
  warehouse: { id: "warehouse", revision: 1, action: "guide", target: { topic: "guide-vua", section: "import-paths" } },
  recipe: { id: "recipe", revision: 1, action: "guide", target: { topic: "guide-vua", section: "production-line" } },
  workshop: { id: "workshop", revision: 1, action: "none", target: null },
  release: { id: "release", revision: 1, action: "guide", target: GUIDE_TARGETS.uploadEligibility },
  packages: { id: "packages", revision: 1, action: "none", target: null },
};

export function contextGuideForPage(page: PageId): ContextGuideDefinition | null {
  const id = page === "home" ? "home"
    : page === "env-play" ? "play"
    : page === "tools-discover" ? "tools"
    : page === "env-create" ? "creator"
    : page === "warehouse" ? "warehouse"
    : page === "recipe" ? "recipe"
    : page === "workshop" ? "workshop"
    : page === "release" ? "release"
    : page === "packages" ? "packages"
    : null;
  return id === null ? null : CONTEXT_GUIDES[id];
}

export interface ContextGuideStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export interface ContextGuideState {
  readonly v: 1;
  readonly dismissed: Partial<Record<ContextGuideId, number>>;
}

const emptyState = (): ContextGuideState => ({ v: 1, dismissed: {} });

export function readContextGuideState(store: ContextGuideStore): ContextGuideState {
  try {
    const value: unknown = JSON.parse(store.getItem(storageKeys.contextGuide) ?? "null");
    if (typeof value !== "object" || value === null || !("v" in value) || value.v !== 1 || !("dismissed" in value)) return emptyState();
    const source = value.dismissed;
    if (typeof source !== "object" || source === null || Array.isArray(source)) return emptyState();
    const dismissed: ContextGuideState["dismissed"] = {};
    for (const [id, revision] of Object.entries(source)) {
      if (id in CONTEXT_GUIDES && typeof revision === "number" && Number.isInteger(revision) && revision >= 0) {
        dismissed[id as ContextGuideId] = revision;
      }
    }
    return { v: 1, dismissed };
  } catch {
    return emptyState();
  }
}

export function dismissContextGuide(store: ContextGuideStore, definition: ContextGuideDefinition): ContextGuideState {
  const current = readContextGuideState(store);
  const next = { v: 1 as const, dismissed: { ...current.dismissed, [definition.id]: definition.revision } };
  try { store.setItem(storageKeys.contextGuide, JSON.stringify(next)); } catch { /* Keep the in-memory dismissal. */ }
  return next;
}

export function readContextGuideSnoozed(store: ContextGuideStore): readonly ContextGuideId[] {
  try {
    const value: unknown = JSON.parse(store.getItem(storageKeys.contextGuideSnoozed) ?? "[]");
    if (!Array.isArray(value)) return [];
    return [...new Set(value.filter((id): id is ContextGuideId => typeof id === "string" && id in CONTEXT_GUIDES))];
  } catch {
    return [];
  }
}

export function snoozeContextGuide(store: ContextGuideStore, id: ContextGuideId): readonly ContextGuideId[] {
  const next = [...new Set([...readContextGuideSnoozed(store), id])];
  try { store.setItem(storageKeys.contextGuideSnoozed, JSON.stringify(next)); } catch { /* Keep the session dismissal. */ }
  return next;
}

export function resetContextGuides(local: ContextGuideStore, session: ContextGuideStore): void {
  try { local.removeItem(storageKeys.contextGuide); } catch { /* Other preferences and task state are untouched. */ }
  try { session.removeItem(storageKeys.contextGuideSnoozed); } catch { /* The current page can still be used. */ }
}
