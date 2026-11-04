import { currentLocale, format } from "../../i18n/index.ts";
import { encyclopediaCopy } from "../../i18n/encyclopedia-copy.ts";
import type { GuideTopicId } from "./guide-content.ts";

export type GuideScreenshotId = "vua-first-run" | "vua-help" | "vua-avatar-preparation" | "vua-theme-presets";
interface GuideScreenshot {
  readonly src: string;
  readonly copy: keyof typeof encyclopediaCopy.en.media;
  readonly width: number;
  readonly height: number;
  readonly sourceMode: "development" | "controlled-production";
  readonly commit: string;
  readonly dirty: boolean;
  readonly capturedAt: string;
  readonly privacy: string;
  readonly license: "Apache-2.0";
}
export interface ResolvedGuideMedia {
  readonly src: string;
  readonly alt: string;
  readonly caption: string;
  readonly kind: "screenshot";
  readonly aspectRatio: string;
}

const commit = "130d4ae94ab629331c35b9964b93e6e10822dd0f";
export const GUIDE_SCREENSHOTS: Record<GuideScreenshotId, GuideScreenshot> = {
  "vua-first-run": { src: "./guide/vua/first-run.png", copy: "firstRun", width: 2162, height: 1353, sourceMode: "controlled-production", commit, dirty: true, capturedAt: "2026-10-27T19:04:19.290Z", privacy: "Isolated profile; no accounts, credentials or private assets", license: "Apache-2.0" },
  "vua-help": { src: "./guide/vua/help-faq.png", copy: "help", width: 1781, height: 1242, sourceMode: "controlled-production", commit, dirty: true, capturedAt: "2026-10-27T19:04:28.560Z", privacy: "Isolated profile; no accounts, credentials or private assets", license: "Apache-2.0" },
  "vua-avatar-preparation": { src: "./guide/vua/avatar-preparation.png", copy: "avatarPreparation", width: 1401, height: 201, sourceMode: "controlled-production", commit, dirty: true, capturedAt: "2026-10-27T19:04:24.772Z", privacy: "Cropped to the eligibility note to exclude host software inventory; no accounts or private assets", license: "Apache-2.0" },
  "vua-theme-presets": { src: "./guide/vua/theme-presets.png", copy: "themes", width: 1773, height: 1233, sourceMode: "controlled-production", commit, dirty: true, capturedAt: "2026-10-27T19:04:35.797Z", privacy: "Isolated profile; no accounts, credentials or private assets", license: "Apache-2.0" },
};

const TARGET_MEDIA: Record<string, GuideScreenshotId> = {
  // The older language-choice capture does not illustrate the first-use notice.
  // F8 supplies current-build captures after the interface has settled.
  "guide-vua/window-tour": "vua-help",
  "guide-vua/environment-page": "vua-avatar-preparation",
  "guide-vua/settings-basics": "vua-theme-presets",
  "guide-vua/upload-prerequisites": "vua-avatar-preparation",
};

function resolve(id: GuideScreenshotId): ResolvedGuideMedia {
  const meta = GUIDE_SCREENSHOTS[id];
  const copy = encyclopediaCopy[currentLocale].media[meta.copy];
  return {
    src: meta.src,
    alt: copy.alt,
    caption: format(copy.caption, { build: `${meta.commit.slice(0, 12)}${meta.dirty ? "+dirty" : ""}`, date: meta.capturedAt.slice(0, 10) }),
    kind: "screenshot",
    aspectRatio: `${meta.width} / ${meta.height}`,
  };
}

export function resolveGuideScreenshot(id: string): ResolvedGuideMedia | null {
  return Object.hasOwn(GUIDE_SCREENSHOTS, id) ? resolve(id as GuideScreenshotId) : null;
}

export function guideMediaForSection(topic: GuideTopicId, section: string): ResolvedGuideMedia | null {
  const id = TARGET_MEDIA[`${topic}/${section}`];
  return id ? resolve(id) : null;
}
