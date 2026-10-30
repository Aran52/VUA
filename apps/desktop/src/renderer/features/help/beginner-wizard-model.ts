import type { PageId } from "../../app/nav-model.ts";
import type { GuideTarget } from "../guide/guide-target.ts";

interface BeginnerConcept {
  readonly id: string;
  readonly target: GuideTarget;
  readonly setup: PageId;
}

// The wizard reads the same sections as the encyclopedia and VUA manual.
// Want this opens a real setup destination; that page owns its plans and consent.
export const BEGINNER_CONCEPTS: readonly BeginnerConcept[] = [
  { id: "vrchat", target: { topic: "guide-start", section: "what-is-vrchat" }, setup: "env-play" },
  { id: "steam", target: { topic: "guide-start", section: "steam-account" }, setup: "settings-accounts" },
  { id: "desktop-play", target: { topic: "guide-start", section: "install-vrchat" }, setup: "env-play" },
  { id: "hardware", target: { topic: "guide-hardware", section: "identify-device" }, setup: "env-play" },
  { id: "avatars", target: { topic: "guide-avatars", section: "outfit-assembly" }, setup: "env-create" },
  { id: "materials", target: { topic: "guide-vua", section: "import-paths" }, setup: "warehouse" },
  { id: "production", target: { topic: "guide-vua", section: "production-line" }, setup: "avatar-hub" },
  { id: "upload", target: { topic: "guide-creator", section: "upload-eligibility" }, setup: "release" },
];

export function parseBeginnerBookmark(raw: string | null): number {
  try {
    const value: unknown = raw === null ? null : JSON.parse(raw);
    if (typeof value !== "object" || value === null || !("version" in value) || value.version !== 1 || !("concept" in value)) return 0;
    const index = BEGINNER_CONCEPTS.findIndex(concept => concept.id === value.concept);
    return index < 0 ? 0 : index;
  } catch { return 0; }
}

export function serializeBeginnerBookmark(index: number): string {
  const bounded = Number.isFinite(index) ? Math.max(0, Math.min(BEGINNER_CONCEPTS.length - 1, Math.trunc(index))) : 0;
  return JSON.stringify({ version: 1, concept: BEGINNER_CONCEPTS[bounded]!.id });
}
