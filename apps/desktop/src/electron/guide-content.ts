import { createHash, createPublicKey, verify as cryptoVerify, type KeyObject } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import {
  GUIDE_CONTENT_SCHEMA_V01,
  MAX_GUIDE_CONTENT_BYTES,
  isGuideContentIdV01,
  isGuideContentPayloadV01,
  isSignedGuideEnvelopeV01,
  type DesktopGuideContentApiV01,
  type GuideContentIdV01,
  type GuideContentPayloadV01,
  type GuideContentReadResultV01,
} from "@vua/contracts";
import { BUNDLED_GUIDE_CONTENT } from "./bundled-guides.js";

export const DEFAULT_GUIDE_CONTENT_BASE_URL = "https://ph-r.com/vua/updates/guides";
export const GUIDE_FETCH_TIMEOUT_MS = 8_000;
/** Compiled Ed25519 trust anchor matching `apps/desktop/distribution/trust.json` (verified by unit tests). */
export const DEFAULT_GUIDE_TRUST_PUBLIC_KEY_HEX =
  "49e6cf56a33ed222959897b68d5dbbacce39ae2aa649f49d532e52f096e91db8";

interface ParsedSemVer {
  major: number;
  minor: number;
  patch: number;
  prerelease: readonly (number | string)[];
}

const STRICT_SEMVER_RE =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/u;

function parseStrictSemVer(raw: string): ParsedSemVer | null {
  const match = STRICT_SEMVER_RE.exec(raw);
  if (!match) return null;
  const major = Number(match[1]);
  const minor = Number(match[2]);
  const patch = Number(match[3]);
  if (!Number.isSafeInteger(major) || !Number.isSafeInteger(minor) || !Number.isSafeInteger(patch)) {
    return null;
  }
  const prerelease = match[4]
    ? match[4].split(".").map((part) => (/^\d+$/u.test(part) ? Number(part) : part))
    : [];
  return { major, minor, patch, prerelease };
}

function coerceClientSemVer(raw: string): ParsedSemVer {
  const trimmed = raw.trim().replace(/^v/iu, "");
  const strict = parseStrictSemVer(trimmed);
  if (strict) return strict;
  const numeric = /^(\d+)(?:\.(\d+))?(?:\.(\d+))?/u.exec(trimmed);
  if (numeric) {
    return {
      major: Number(numeric[1]),
      minor: Number(numeric[2] ?? "0"),
      patch: Number(numeric[3] ?? "0"),
      prerelease: [],
    };
  }
  return { major: 1, minor: 0, patch: 0, prerelease: [] };
}

function compareSemVer(a: ParsedSemVer, b: ParsedSemVer): number {
  if (a.major !== b.major) return a.major - b.major;
  if (a.minor !== b.minor) return a.minor - b.minor;
  if (a.patch !== b.patch) return a.patch - b.patch;
  if (a.prerelease.length === 0 && b.prerelease.length === 0) return 0;
  if (a.prerelease.length === 0) return 1;
  if (b.prerelease.length === 0) return -1;
  const len = Math.max(a.prerelease.length, b.prerelease.length);
  for (let i = 0; i < len; i += 1) {
    const ai = a.prerelease[i];
    const bi = b.prerelease[i];
    if (ai === undefined) return -1;
    if (bi === undefined) return 1;
    if (ai === bi) continue;
    const aNum = typeof ai === "number";
    const bNum = typeof bi === "number";
    if (aNum && bNum) return ai - bi;
    if (aNum) return -1;
    if (bNum) return 1;
    return ai < bi ? -1 : 1;
  }
  return 0;
}

interface StoredHighWaterEntry {
  sequence: number;
  payloadSha256: string;
  checkedAt: string;
}

interface StoredHighWaterFile {
  schemaVersion: "0.1";
  guides: Record<string, StoredHighWaterEntry>;
}

export interface GuideContentServiceOptions {
  userDataRoot: string;
  trustedPublicKeyHex?: string;
  clientVersion: string;
  baseUrl?: string;
  bundled?: Readonly<Record<GuideContentIdV01, GuideContentPayloadV01>>;
  fetchBytes?: (url: string, signal: AbortSignal, maxBytes: number) => Promise<Buffer>;
  fetchImpl?: (url: string, init: RequestInit) => Promise<Response>;
  now?: () => Date;
}

function sha256Hex(input: Buffer | string): string {
  return createHash("sha256").update(input).digest("hex");
}

function spkiEd25519PublicKey(hex: string): KeyObject {
  if (!/^[a-f0-9]{64}$/u.test(hex)) {
    throw new Error("invalid_guide_trust_key");
  }
  const der = Buffer.concat([
    Buffer.from("302a300506032b6570032100", "hex"),
    Buffer.from(hex, "hex"),
  ]);
  return createPublicKey({ key: der, type: "spki", format: "der" });
}

export function resolveAllowedGuideBaseUrl(candidateUrl: string | undefined, allowLoopback = false): string {
  if (!candidateUrl) return DEFAULT_GUIDE_CONTENT_BASE_URL;
  const trimmed = candidateUrl.replace(/\/+$/u, "");
  const parsed = new URL(trimmed);
  if (parsed.username !== "" || parsed.password !== "" || parsed.search !== "" || parsed.hash !== "") {
    throw new Error("invalid_guide_base_url");
  }
  if (parsed.protocol === "https:" && parsed.hostname === "ph-r.com" && parsed.pathname.startsWith("/vua/updates/guides")) {
    return trimmed;
  }
  if (allowLoopback && parsed.protocol === "http:" && (parsed.hostname === "127.0.0.1" || parsed.hostname === "localhost")) {
    return trimmed;
  }
  throw new Error("invalid_guide_base_url");
}

/** Verifies Ed25519 signature over the raw `payload` string bytes BEFORE parsing `payload` JSON. */
export function verifySignedGuideEnvelope(
  rawBytes: Buffer,
  trustedPublicKey: KeyObject | string,
  expectedId: GuideContentIdV01,
): { payload: GuideContentPayloadV01; payloadSha256: string } {
  if (!Buffer.isBuffer(rawBytes) || rawBytes.byteLength === 0 || rawBytes.byteLength > MAX_GUIDE_CONTENT_BYTES) {
    throw new Error("guide_envelope_too_large");
  }
  const text = new TextDecoder("utf-8", { fatal: true }).decode(rawBytes);
  const envelope: unknown = JSON.parse(text);
  if (!isSignedGuideEnvelopeV01(envelope)) {
    throw new Error("invalid_guide_envelope");
  }
  const key = typeof trustedPublicKey === "string" ? spkiEd25519PublicKey(trustedPublicKey) : trustedPublicKey;
  const payloadBytes = Buffer.from(envelope.payload, "utf8");
  const signatureBytes = Buffer.from(envelope.signature, "hex");
  if (!cryptoVerify(null, payloadBytes, key, signatureBytes)) {
    throw new Error("invalid_guide_signature");
  }
  const parsedPayload: unknown = JSON.parse(envelope.payload);
  if (!isGuideContentPayloadV01(parsedPayload)) {
    throw new Error("invalid_guide_payload");
  }
  if (parsedPayload.id !== expectedId) {
    throw new Error("wrong_guide_id");
  }
  return { payload: parsedPayload, payloadSha256: sha256Hex(payloadBytes) };
}

export async function defaultFetchGuideBytes(
  url: string,
  signal: AbortSignal,
  maxBytes: number,
  fetchImpl: (url: string, init: RequestInit) => Promise<Response> = fetch,
): Promise<Buffer> {
  const parsed = new URL(url);
  if (parsed.protocol !== "https:" && parsed.hostname !== "127.0.0.1" && parsed.hostname !== "localhost") {
    throw new Error("guide_https_required");
  }
  const response = await fetchImpl(url, {
    method: "GET",
    redirect: "error",
    signal,
    headers: { accept: "application/json" },
  });
  if (!response.ok) {
    throw new Error(`guide_http_${response.status}`);
  }
  const declaredLength = response.headers.get("content-length");
  if (declaredLength !== null) {
    const length = Number(declaredLength);
    if (!Number.isFinite(length) || length > maxBytes) {
      throw new Error("guide_envelope_too_large");
    }
  }
  if (!response.body) {
    throw new Error("guide_empty_body");
  }
  const chunks: Buffer[] = [];
  let total = 0;
  const reader = response.body.getReader();
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      const buf = Buffer.from(value);
      total += buf.byteLength;
      if (total > maxBytes) {
        await reader.cancel();
        throw new Error("guide_envelope_too_large");
      }
      chunks.push(buf);
    }
  } finally {
    reader.releaseLock();
  }
  return Buffer.concat(chunks, total);
}

function writeAtomic(filePath: string, bytes: Buffer | string): void {
  const dir = path.dirname(filePath);
  fs.mkdirSync(dir, { recursive: true });
  const pending = `${filePath}.${process.pid}.${Date.now()}.pending`;
  fs.writeFileSync(pending, bytes);
  fs.renameSync(pending, filePath);
}

export class GuideContentService implements DesktopGuideContentApiV01 {
  private readonly storeDir: string;
  private readonly cacheDir: string;
  private readonly statePath: string;
  private readonly publicKey: KeyObject;
  private readonly clientVersion: ParsedSemVer;
  private readonly baseUrl: string;
  private readonly bundled: Readonly<Record<GuideContentIdV01, GuideContentPayloadV01>>;
  private readonly fetchBytes: (url: string, signal: AbortSignal, maxBytes: number) => Promise<Buffer>;
  private readonly now: () => Date;

  constructor(options: GuideContentServiceOptions) {
    this.storeDir = path.join(options.userDataRoot, "guide-content");
    this.cacheDir = path.join(this.storeDir, "cache");
    this.statePath = path.join(this.storeDir, "state.json");
    this.publicKey = spkiEd25519PublicKey(options.trustedPublicKeyHex ?? DEFAULT_GUIDE_TRUST_PUBLIC_KEY_HEX);
    this.clientVersion = coerceClientSemVer(options.clientVersion);
    this.baseUrl = (options.baseUrl ?? DEFAULT_GUIDE_CONTENT_BASE_URL).replace(/\/+$/u, "");
    this.bundled = options.bundled ?? BUNDLED_GUIDE_CONTENT;
    const fetchImpl = options.fetchImpl ?? fetch;
    this.fetchBytes =
      options.fetchBytes ?? ((url, signal, maxBytes) => defaultFetchGuideBytes(url, signal, maxBytes, fetchImpl));
    this.now = options.now ?? (() => new Date());
    // Intentionally performs zero network I/O at construction/startup.
  }

  async read(id: GuideContentIdV01): Promise<GuideContentReadResultV01> {
    if (!isGuideContentIdV01(id)) {
      throw new Error("invalid_guide_id");
    }
    const checkedAt = this.now().toISOString();
    const state = this.readState();
    const highWater = state.guides[id];

    const remoteUrl = `${this.baseUrl}/${id}.json`;
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(new Error("guide_fetch_timeout")), GUIDE_FETCH_TIMEOUT_MS);
      let raw: Buffer;
      try {
        raw = await this.fetchBytes(remoteUrl, controller.signal, MAX_GUIDE_CONTENT_BYTES);
      } finally {
        clearTimeout(timer);
      }
      const { payload, payloadSha256 } = verifySignedGuideEnvelope(raw, this.publicKey, id);
      this.assertMonotonicAndConsistent(payload.sequence, payloadSha256, highWater);
      if (!this.supportsMinClient(payload.minClient)) {
        throw new Error("guide_min_client_unsatisfied");
      }
      writeAtomic(this.cachedEnvelopePath(id), raw);
      state.guides[id] = { sequence: payload.sequence, payloadSha256, checkedAt };
      this.writeState(state);
      return { document: payload, source: "remote", checkedAt };
    } catch {
      const cached = this.readVerifiedCache(id, state.guides[id]);
      if (cached) {
        return { document: cached, source: "cached", checkedAt };
      }
      return { document: this.bundled[id], source: "bundled", checkedAt };
    }
  }

  private supportsMinClient(minClient: string): boolean {
    const validMin = parseStrictSemVer(minClient);
    if (!validMin) return false;
    return compareSemVer(this.clientVersion, validMin) >= 0;
  }

  private assertMonotonicAndConsistent(
    sequence: number,
    payloadSha256: string,
    highWater: StoredHighWaterEntry | undefined,
  ): void {
    if (!highWater) return;
    if (sequence < highWater.sequence) {
      throw new Error("guide_sequence_replayed");
    }
    if (sequence === highWater.sequence && payloadSha256 !== highWater.payloadSha256) {
      throw new Error("guide_sequence_equivocation");
    }
  }

  private cachedEnvelopePath(id: GuideContentIdV01): string {
    return path.join(this.cacheDir, `${id}.json`);
  }

  private readVerifiedCache(
    id: GuideContentIdV01,
    highWater: StoredHighWaterEntry | undefined,
  ): GuideContentPayloadV01 | null {
    try {
      const raw = fs.readFileSync(this.cachedEnvelopePath(id));
      const { payload, payloadSha256 } = verifySignedGuideEnvelope(raw, this.publicKey, id);
      this.assertMonotonicAndConsistent(payload.sequence, payloadSha256, highWater);
      if (!this.supportsMinClient(payload.minClient)) {
        return null;
      }
      return payload;
    } catch {
      return null;
    }
  }

  private readState(): StoredHighWaterFile {
    try {
      const parsed: unknown = JSON.parse(fs.readFileSync(this.statePath, "utf8"));
      if (
        parsed && typeof parsed === "object" && !Array.isArray(parsed)
        && (parsed as { schemaVersion?: unknown }).schemaVersion === GUIDE_CONTENT_SCHEMA_V01
        && typeof (parsed as { guides?: unknown }).guides === "object"
        && (parsed as { guides?: unknown }).guides !== null
        && !Array.isArray((parsed as { guides?: unknown }).guides)
      ) {
        const cleanGuides: Record<string, StoredHighWaterEntry> = {};
        for (const [key, entry] of Object.entries((parsed as { guides: Record<string, unknown> }).guides)) {
          if (
            isGuideContentIdV01(key)
            && entry && typeof entry === "object" && !Array.isArray(entry)
            && Number.isSafeInteger((entry as { sequence?: unknown }).sequence)
            && ((entry as { sequence: number }).sequence >= 1)
            && typeof (entry as { payloadSha256?: unknown }).payloadSha256 === "string"
            && /^[a-f0-9]{64}$/u.test((entry as { payloadSha256: string }).payloadSha256)
            && typeof (entry as { checkedAt?: unknown }).checkedAt === "string"
          ) {
            cleanGuides[key] = {
              sequence: (entry as { sequence: number }).sequence,
              payloadSha256: (entry as { payloadSha256: string }).payloadSha256,
              checkedAt: (entry as { checkedAt: string }).checkedAt,
            };
          }
        }
        return { schemaVersion: GUIDE_CONTENT_SCHEMA_V01, guides: cleanGuides };
      }
    } catch {
      // Missing or unreadable high-water file starts empty.
    }
    return { schemaVersion: GUIDE_CONTENT_SCHEMA_V01, guides: {} };
  }

  private writeState(state: StoredHighWaterFile): void {
    try {
      writeAtomic(this.statePath, `${JSON.stringify(state, null, 2)}\n`);
    } catch {
      // Best-effort local persistence; callers still receive the verified document.
    }
  }
}
