/** Signed guide content channel v0.1 (`guide-content v0.1`).
 * Unknown fields are tolerated; incompatible structural changes raise `minClient`. */

export const GUIDE_CONTENT_SCHEMA_V01 = "0.1" as const;
export const MAX_GUIDE_CONTENT_BYTES = 1_048_576;

export const GUIDE_CONTENT_IDS = [
  "pixiv-account",
  "booth-prices",
  "paying-abroad",
  "buying",
  "terms-of-use",
  "why-licensed",
] as const;
export type GuideContentIdV01 = (typeof GUIDE_CONTENT_IDS)[number];

export const GUIDE_CONTENT_LOCALES = ["en", "zh-CN", "zh-TW", "ja", "ko"] as const;
export type GuideContentLocaleV01 = (typeof GUIDE_CONTENT_LOCALES)[number];

/** Official HTTPS hostnames permitted in guide links, sources, and asset mirrors. */
export const GUIDE_ALLOWED_LINK_HOSTS = [
  "ph-r.com",
  "pixiv.net",
  "www.pixiv.net",
  "accounts.pixiv.net",
  "www.help.pixiv.net",
  "booth.pm",
  "accounts.booth.pm",
  "manage.booth.pm",
  "paypal.com",
  "www.paypal.com",
  "vrchat.com",
  "hello.vrchat.com",
  "docs.vrchat.com",
  "creators.vrchat.com",
  "help.vrchat.com",
  "wiki.vrchat.com",
  "jinxxy.com",
  "gumroad.com",
  "payhip.com",
  "help.payhip.com",
  "store.steampowered.com",
  "help.steampowered.com",
  "unity.com",
  "login.unity.com",
] as const;

export interface GuideLocalizedTextV01 {
  readonly en: string;
  readonly "zh-CN"?: string;
  readonly "zh-TW"?: string;
  readonly ja?: string;
  readonly ko?: string;
}

export type GuidePerformerV01 = "vua" | "vua_opens" | "player";

export interface GuideLinkV01 {
  readonly label: GuideLocalizedTextV01;
  readonly url: string;
}

export interface GuideSopStepV01 {
  readonly id: string;
  readonly performer: GuidePerformerV01;
  readonly action: GuideLocalizedTextV01;
  readonly expectedResult: GuideLocalizedTextV01;
  readonly verification: GuideLocalizedTextV01;
  readonly onFailure: GuideLocalizedTextV01;
  readonly sites: readonly string[];
  readonly links: readonly GuideLinkV01[];
}

export interface GuideRegionalVerificationV01 {
  readonly date: string;
  readonly network: string;
  readonly cardIssuingRegion: string;
}

export interface GuideRegionalNoteV01 {
  readonly region: string;
  readonly text: GuideLocalizedTextV01;
  readonly verification: GuideRegionalVerificationV01;
}

export interface GuideSopVerificationV01 {
  readonly date: string;
  readonly environment: string;
  readonly softwareVersions: readonly string[];
}

export interface GuideSourceRefV01 {
  readonly label: string;
  readonly url: string;
}

export interface GuideSopV01 {
  readonly title: GuideLocalizedTextV01;
  readonly appliesTo: {
    readonly software: readonly string[];
    readonly devices: readonly string[];
    readonly region: string;
  };
  readonly prerequisites: readonly GuideLocalizedTextV01[];
  readonly steps: readonly GuideSopStepV01[];
  readonly regionalNotes: readonly GuideRegionalNoteV01[];
  readonly lastVerification: GuideSopVerificationV01;
  readonly sources: readonly GuideSourceRefV01[];
}

export interface GuideAssetObjectV01 {
  readonly sha256: string;
  readonly bytes: number;
  readonly sources: readonly string[];
}

export interface GuideContentPayloadV01 {
  readonly schemaVersion: typeof GUIDE_CONTENT_SCHEMA_V01;
  readonly id: GuideContentIdV01;
  readonly sequence: number;
  readonly version: string;
  readonly minClient: string;
  readonly sop: GuideSopV01;
  readonly assets: Readonly<Record<string, GuideAssetObjectV01>>;
}

export interface SignedGuideEnvelopeV01 {
  readonly schemaVersion: typeof GUIDE_CONTENT_SCHEMA_V01;
  readonly algorithm: "ed25519";
  readonly payload: string;
  readonly signature: string;
}

export type GuideContentSourceV01 = "bundled" | "cached" | "remote";

export interface GuideContentReadResultV01 {
  readonly document: GuideContentPayloadV01;
  readonly source: GuideContentSourceV01;
  readonly checkedAt: string;
}

export interface DesktopGuideContentApiV01 {
  read(id: GuideContentIdV01): Promise<GuideContentReadResultV01>;
}

const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const nonEmpty = (v: unknown): v is string => typeof v === "string" && v.trim().length > 0 && v.length <= 16_384;
const semverText = (v: unknown): v is string =>
  typeof v === "string" && /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[A-Za-z0-9.-]+)?(?:\+[A-Za-z0-9.-]+)?$/u.test(v);
const isoDate = (v: unknown): v is string => typeof v === "string" && /^\d{4}-\d{2}-\d{2}$/u.test(v);

export function isGuideContentIdV01(v: unknown): v is GuideContentIdV01 {
  return typeof v === "string" && (GUIDE_CONTENT_IDS as readonly string[]).includes(v);
}

export function isAllowedGuideHttpsUrl(v: unknown): v is string {
  if (typeof v !== "string" || v.length === 0 || v.length > 2048 || /\p{Cs}/u.test(v)) return false;
  try {
    const parsed = new URL(v);
    if (parsed.protocol !== "https:" || parsed.username !== "" || parsed.password !== "") return false;
    return (GUIDE_ALLOWED_LINK_HOSTS as readonly string[]).includes(parsed.hostname.toLowerCase());
  } catch {
    return false;
  }
}

export function isGuideLocalizedTextV01(v: unknown): v is GuideLocalizedTextV01 {
  if (!record(v) || !nonEmpty(v.en)) return false;
  for (const locale of GUIDE_CONTENT_LOCALES) {
    if (locale === "en") continue;
    const text = v[locale];
    if (text !== undefined && typeof text !== "string") return false;
    if (typeof text === "string" && text.length > 16_384) return false;
  }
  return true;
}

export function resolveGuideLocalizedText(text: GuideLocalizedTextV01, locale: GuideContentLocaleV01): string {
  const candidate = text[locale];
  return typeof candidate === "string" && candidate.trim().length > 0 ? candidate : text.en;
}

function isStep(v: unknown): v is GuideSopStepV01 {
  if (!record(v)) return false;
  if (typeof v.id !== "string" || !/^[a-z0-9][a-z0-9-]{0,63}$/u.test(v.id)) return false;
  if (v.performer !== "vua" && v.performer !== "vua_opens" && v.performer !== "player") return false;
  if (!isGuideLocalizedTextV01(v.action) || !isGuideLocalizedTextV01(v.expectedResult)
    || !isGuideLocalizedTextV01(v.verification) || !isGuideLocalizedTextV01(v.onFailure)) return false;
  if (!Array.isArray(v.sites) || v.sites.length > 16 || !v.sites.every(s => typeof s === "string" && s.length > 0 && s.length <= 128)) return false;
  if (!Array.isArray(v.links) || v.links.length > 16) return false;
  return v.links.every(link => record(link) && isGuideLocalizedTextV01(link.label) && isAllowedGuideHttpsUrl(link.url));
}

function isRegionalNote(v: unknown): v is GuideRegionalNoteV01 {
  if (!record(v) || typeof v.region !== "string" || !/^[a-z0-9][a-z0-9-]{0,63}$/u.test(v.region)) return false;
  if (!isGuideLocalizedTextV01(v.text) || !record(v.verification)) return false;
  return isoDate(v.verification.date) && nonEmpty(v.verification.network) && nonEmpty(v.verification.cardIssuingRegion);
}

function isSop(v: unknown): v is GuideSopV01 {
  if (!record(v) || !isGuideLocalizedTextV01(v.title) || !record(v.appliesTo)) return false;
  const applies = v.appliesTo;
  if (!Array.isArray(applies.software) || !applies.software.every(nonEmpty)
    || !Array.isArray(applies.devices) || !applies.devices.every(nonEmpty)
    || typeof applies.region !== "string" || applies.region.length === 0) return false;
  if (!Array.isArray(v.prerequisites) || v.prerequisites.length > 32 || !v.prerequisites.every(isGuideLocalizedTextV01)) return false;
  if (!Array.isArray(v.steps) || v.steps.length < 1 || v.steps.length > 32 || !v.steps.every(isStep)) return false;
  if (new Set(v.steps.map(s => s.id)).size !== v.steps.length) return false;
  if (!Array.isArray(v.regionalNotes) || v.regionalNotes.length > 32 || !v.regionalNotes.every(isRegionalNote)) return false;
  if (!record(v.lastVerification) || !isoDate(v.lastVerification.date) || !nonEmpty(v.lastVerification.environment)
    || !Array.isArray(v.lastVerification.softwareVersions) || !v.lastVerification.softwareVersions.every(nonEmpty)) return false;
  if (!Array.isArray(v.sources) || v.sources.length < 1 || v.sources.length > 32) return false;
  return v.sources.every(src => record(src) && nonEmpty(src.label) && isAllowedGuideHttpsUrl(src.url));
}

function isAssetMap(v: unknown): v is Readonly<Record<string, GuideAssetObjectV01>> {
  if (!record(v)) return false;
  const entries = Object.entries(v);
  if (entries.length > 128) return false;
  return entries.every(([key, item]) =>
    /^[a-z0-9][a-z0-9._/-]{0,127}$/u.test(key) && !key.includes("..") && record(item)
    && typeof item.sha256 === "string" && /^[a-f0-9]{64}$/u.test(item.sha256)
    && Number.isSafeInteger(item.bytes) && (item.bytes as number) >= 1 && (item.bytes as number) <= 20 * 1024 * 1024
    && Array.isArray(item.sources) && item.sources.length >= 1 && item.sources.length <= 4
    && item.sources.every(isAllowedGuideHttpsUrl));
}

export function isGuideContentPayloadV01(v: unknown): v is GuideContentPayloadV01 {
  if (!record(v) || v.schemaVersion !== GUIDE_CONTENT_SCHEMA_V01 || !isGuideContentIdV01(v.id)) return false;
  if (!Number.isSafeInteger(v.sequence) || (v.sequence as number) < 1) return false;
  if (!semverText(v.version) || !semverText(v.minClient)) return false;
  return isSop(v.sop) && isAssetMap(v.assets);
}

export function isSignedGuideEnvelopeV01(v: unknown): v is SignedGuideEnvelopeV01 {
  return record(v)
    && v.schemaVersion === GUIDE_CONTENT_SCHEMA_V01
    && v.algorithm === "ed25519"
    && typeof v.payload === "string"
    && v.payload.length > 0
    && typeof v.signature === "string"
    && /^[a-f0-9]{128}$/u.test(v.signature);
}

export function isGuideContentReadResultV01(v: unknown): v is GuideContentReadResultV01 {
  return record(v)
    && isGuideContentPayloadV01(v.document)
    && (v.source === "bundled" || v.source === "cached" || v.source === "remote")
    && typeof v.checkedAt === "string" && v.checkedAt.length > 0;
}
