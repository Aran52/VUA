import { generateKeyPairSync, sign } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  GUIDE_CONTENT_IDS,
  MAX_GUIDE_CONTENT_BYTES,
  isGuideContentPayloadV01,
  type GuideContentPayloadV01,
} from "@vua/contracts";
import { BUNDLED_GUIDE_CONTENT } from "./bundled-guides.js";
import {
  DEFAULT_GUIDE_TRUST_PUBLIC_KEY_HEX,
  GuideContentService,
  defaultFetchGuideBytes,
  resolveAllowedGuideBaseUrl,
  verifySignedGuideEnvelope,
} from "./guide-content.js";
// @ts-expect-error -- Node ESM helper script tested directly.
import { sealGuideEnvelope, verifyGuideEnvelopeBytes } from "../../scripts/prepare-guide-content.mjs";
// @ts-expect-error -- Node ESM helper script tested directly.
import { checkAllGuideSources, evaluateSourceOutcome, htmlToNormalizedText } from "../../scripts/check-guide-sources.mjs";

function createTestKeyPair() {
  const { publicKey, privateKey } = generateKeyPairSync("ed25519");
  const publicKeyHex = publicKey.export({ type: "spki", format: "der" }).subarray(-32).toString("hex");
  return { publicKey, privateKey, publicKeyHex };
}

function signPayloadText(payloadText: string, privateKey: ReturnType<typeof createTestKeyPair>["privateKey"]): Buffer {
  const signature = sign(null, Buffer.from(payloadText, "utf8"), privateKey).toString("hex");
  return Buffer.from(
    JSON.stringify({
      schemaVersion: "0.1",
      algorithm: "ed25519",
      payload: payloadText,
      signature,
    }),
    "utf8",
  );
}

function makePayload(overrides: Partial<GuideContentPayloadV01> = {}): GuideContentPayloadV01 {
  const base = structuredClone(BUNDLED_GUIDE_CONTENT["paying-abroad"]);
  return {
    ...base,
    sequence: 2,
    version: "0.2.0",
    minClient: "1.0.0",
    ...overrides,
  };
}

const tempDirs: string[] = [];
function makeTempDir(): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vua-guide-content-"));
  tempDirs.push(dir);
  return dir;
}

afterEach(() => {
  while (tempDirs.length > 0) {
    const dir = tempDirs.pop()!;
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

describe("guide-content v0.1 channel", () => {
  it("matches the compiled trust anchor with apps/desktop/distribution/trust.json and validates all bundled guides", () => {
    const trustJson = JSON.parse(
      fs.readFileSync(new URL("../../distribution/trust.json", import.meta.url), "utf8"),
    ) as { publicKey: string };
    expect(DEFAULT_GUIDE_TRUST_PUBLIC_KEY_HEX).toBe(trustJson.publicKey);

    const locales = ["en", "zh-CN", "zh-TW", "ja", "ko"] as const;
    for (const id of GUIDE_CONTENT_IDS) {
      const guide = BUNDLED_GUIDE_CONTENT[id];
      expect(isGuideContentPayloadV01(guide), id).toBe(true);
      expect(guide.id).toBe(id);
      for (const locale of locales) {
        expect(guide.sop.title[locale]?.trim().length, `${id}.title.${locale}`).toBeGreaterThan(0);
        for (const step of guide.sop.steps) {
          expect(step.action[locale]?.trim().length, `${id}.${step.id}.action.${locale}`).toBeGreaterThan(0);
          expect(step.expectedResult[locale]?.trim().length, `${id}.${step.id}.expectedResult.${locale}`).toBeGreaterThan(0);
          expect(step.verification[locale]?.trim().length, `${id}.${step.id}.verification.${locale}`).toBeGreaterThan(0);
          expect(step.onFailure[locale]?.trim().length, `${id}.${step.id}.onFailure.${locale}`).toBeGreaterThan(0);
          for (const link of step.links ?? []) {
            expect(link.label[locale]?.trim().length, `${id}.${step.id}.link.${locale}`).toBeGreaterThan(0);
          }
        }
      }
    }
  });

  it("verifies Ed25519 signatures over raw payload bytes BEFORE parsing payload JSON", () => {
    const { privateKey, publicKeyHex } = createTestKeyPair();
    const otherKey = createTestKeyPair();

    const validEnvelope = signPayloadText(JSON.stringify(makePayload()), privateKey);
    const verified = verifySignedGuideEnvelope(validEnvelope, publicKeyHex, "paying-abroad");
    expect(verified.payload.version).toBe("0.2.0");

    // Wrong key / tampered signature fails with invalid_guide_signature even when payload is malformed JSON.
    const unparseablePayload = "{not-valid-json-at-all";
    const badSignedMalformed = signPayloadText(unparseablePayload, otherKey.privateKey);
    const parseSpy = vi.spyOn(JSON, "parse");
    try {
      expect(() => verifySignedGuideEnvelope(badSignedMalformed, publicKeyHex, "paying-abroad")).toThrow(
        "invalid_guide_signature",
      );
      // JSON.parse was called once for the outer envelope, NEVER for `unparseablePayload`.
      expect(parseSpy).toHaveBeenCalledTimes(1);
    } finally {
      parseSpy.mockRestore();
    }
  });

  it("performs zero network requests at startup/construction and serves remote -> cached -> bundled offline fallback", async () => {
    const { privateKey, publicKeyHex } = createTestKeyPair();
    const userDataRoot = makeTempDir();
    const remotePayload = makePayload({ sequence: 3, version: "0.3.0" });
    let shouldFailNetwork = false;
    const fetchBytes = vi.fn(async () => {
      if (shouldFailNetwork) throw new Error("network_down");
      return signPayloadText(JSON.stringify(remotePayload), privateKey);
    });

    const service = new GuideContentService({
      userDataRoot,
      trustedPublicKeyHex: publicKeyHex,
      clientVersion: "1.0.0",
      fetchBytes,
      now: () => new Date("2026-11-11T12:00:00Z"),
    });
    expect(fetchBytes).not.toHaveBeenCalled();

    // Uncached + offline -> bundled
    shouldFailNetwork = true;
    const initialOffline = await service.read("paying-abroad");
    expect(initialOffline.source).toBe("bundled");
    expect(initialOffline.document.version).toBe(BUNDLED_GUIDE_CONTENT["paying-abroad"].version);
    expect(fetchBytes).toHaveBeenCalledTimes(1);

    // Online -> remote (and caches verified envelope + high-water)
    shouldFailNetwork = false;
    const onlineRead = await service.read("paying-abroad");
    expect(onlineRead.source).toBe("remote");
    expect(onlineRead.document.version).toBe("0.3.0");

    // Subsequent offline -> verified cached copy
    shouldFailNetwork = true;
    const cachedRead = await service.read("paying-abroad");
    expect(cachedRead.source).toBe("cached");
    expect(cachedRead.document.version).toBe("0.3.0");

    // Corrupted local cache file + offline -> bundled fallback
    fs.writeFileSync(path.join(userDataRoot, "guide-content", "cache", "paying-abroad.json"), "corrupted");
    const corruptedFallback = await service.read("paying-abroad");
    expect(corruptedFallback.source).toBe("bundled");
  });

  it("rejects sequence replay and same-sequence equivocation, preserving verified high-water cache", async () => {
    const { privateKey, publicKeyHex } = createTestKeyPair();
    const userDataRoot = makeTempDir();
    let currentEnvelope = signPayloadText(JSON.stringify(makePayload({ sequence: 5, version: "0.5.0" })), privateKey);

    const service = new GuideContentService({
      userDataRoot,
      trustedPublicKeyHex: publicKeyHex,
      clientVersion: "1.0.0",
      fetchBytes: async () => currentEnvelope,
    });

    const first = await service.read("paying-abroad");
    expect(first.source).toBe("remote");
    expect(first.document.sequence).toBe(5);

    // Replay: valid signature, lower sequence (4 < 5)
    currentEnvelope = signPayloadText(JSON.stringify(makePayload({ sequence: 4, version: "0.4.0" })), privateKey);
    const replayed = await service.read("paying-abroad");
    expect(replayed.source).toBe("cached");
    expect(replayed.document.sequence).toBe(5);
    expect(replayed.document.version).toBe("0.5.0");

    // Equivocation: valid signature, same sequence (5 === 5), different content
    currentEnvelope = signPayloadText(JSON.stringify(makePayload({ sequence: 5, version: "0.5.1" })), privateKey);
    const equivocated = await service.read("paying-abroad");
    expect(equivocated.source).toBe("cached");
    expect(equivocated.document.version).toBe("0.5.0");

    // Identical sequence + identical payload bytes remains valid
    currentEnvelope = signPayloadText(JSON.stringify(makePayload({ sequence: 5, version: "0.5.0" })), privateKey);
    const idempotent = await service.read("paying-abroad");
    expect(idempotent.source).toBe("remote");
    expect(idempotent.document.version).toBe("0.5.0");
  });

  it("ignores unknown fields and enforces minClient SemVer fallback without advancing high-water", async () => {
    const { privateKey, publicKeyHex } = createTestKeyPair();
    const userDataRoot = makeTempDir();

    const withUnknownFields = {
      ...makePayload({ sequence: 2, version: "0.2.0", minClient: "1.0.0" }),
      futureTopLevelField: { experimental: true },
      sop: {
        ...BUNDLED_GUIDE_CONTENT["paying-abroad"].sop,
        futureSopHint: "ignored-safely",
      },
    };
    let currentEnvelope = signPayloadText(JSON.stringify(withUnknownFields), privateKey);

    const v1Client = new GuideContentService({
      userDataRoot,
      trustedPublicKeyHex: publicKeyHex,
      clientVersion: "1.0.0",
      fetchBytes: async () => currentEnvelope,
    });
    const accepted = await v1Client.read("paying-abroad");
    expect(accepted.source).toBe("remote");
    expect(accepted.document.version).toBe("0.2.0");

    // Remote bumps minClient to 1.2.0 -> 1.0.0 client falls back to cached 0.2.0 without advancing high-water to 3
    currentEnvelope = signPayloadText(
      JSON.stringify(makePayload({ sequence: 3, version: "0.3.0", minClient: "1.2.0" })),
      privateKey,
    );
    const gated = await v1Client.read("paying-abroad");
    expect(gated.source).toBe("cached");
    expect(gated.document.version).toBe("0.2.0");

    // Upgraded 1.2.0 client accepts sequence 3
    const v12Client = new GuideContentService({
      userDataRoot,
      trustedPublicKeyHex: publicKeyHex,
      clientVersion: "1.2.0",
      fetchBytes: async () => currentEnvelope,
    });
    const upgraded = await v12Client.read("paying-abroad");
    expect(upgraded.source).toBe("remote");
    expect(upgraded.document.version).toBe("0.3.0");
  });

  it("enforces the 1 MiB envelope limit on both buffers and streaming responses", async () => {
    const { publicKeyHex } = createTestKeyPair();
    const oversized = Buffer.alloc(MAX_GUIDE_CONTENT_BYTES + 1, 0x20);
    expect(() => verifySignedGuideEnvelope(oversized, publicKeyHex, "paying-abroad")).toThrow(
      "guide_envelope_too_large",
    );

    let streamCancelled = false;
    const chunk = new Uint8Array(300 * 1024);
    let sent = 0;
    const stream = new ReadableStream<Uint8Array>({
      pull(controller) {
        sent += 1;
        controller.enqueue(chunk);
        if (sent >= 5) controller.close();
      },
      cancel() {
        streamCancelled = true;
      },
    });
    await expect(
      defaultFetchGuideBytes(
        "https://ph-r.com/vua/updates/guides/paying-abroad.json",
        new AbortController().signal,
        MAX_GUIDE_CONTENT_BYTES,
        async () => new Response(stream, { status: 200 }),
      ),
    ).rejects.toThrow("guide_envelope_too_large");
    expect(streamCancelled).toBe(true);
  });

  it("restricts guide base URL overrides and seals/monitors sources without storing page text", async () => {
    expect(resolveAllowedGuideBaseUrl(undefined)).toBe("https://ph-r.com/vua/updates/guides");
    expect(resolveAllowedGuideBaseUrl("https://ph-r.com/vua/updates/guides-candidate/")).toBe(
      "https://ph-r.com/vua/updates/guides-candidate",
    );
    expect(() => resolveAllowedGuideBaseUrl("https://evil.example.org/vua/updates/guides")).toThrow(
      "invalid_guide_base_url",
    );
    expect(() => resolveAllowedGuideBaseUrl("http://127.0.0.1:8787/guides", false)).toThrow(
      "invalid_guide_base_url",
    );
    expect(resolveAllowedGuideBaseUrl("http://127.0.0.1:8787/guides/", true)).toBe("http://127.0.0.1:8787/guides");

    // prepare-guide-content round-trip
    const { privateKey, publicKeyHex } = createTestKeyPair();
    const sealed = sealGuideEnvelope(makePayload({ sequence: 7 }), privateKey, publicKeyHex);
    const opened = verifyGuideEnvelopeBytes(sealed, publicKeyHex, "paying-abroad");
    expect(opened.sequence).toBe(7);

    // check-guide-sources: 403 / Cloudflare is `unreachable`, NOT `changed`; no page text stored
    const normalized = htmlToNormalizedText("<html><head><script>evil()</script></head><body><p>Official payment guide for BOOTH buyers.</p></body></html>");
    expect(normalized).toBe("Official payment guide for BOOTH buyers.");

    const init = evaluateSourceOutcome(undefined, {
      ok: true,
      status: 200,
      body: "<article><p>Official payment guide for BOOTH buyers with credit card and PayPal notes.</p></article>",
    });
    expect(init.outcome).toBe("initialized");
    expect(init.textSha256).toMatch(/^[a-f0-9]{64}$/u);

    const blocked403 = evaluateSourceOutcome(
      { textSha256: init.textSha256 },
      { ok: false, status: 403, body: "Attention Required! | Cloudflare" },
    );
    expect(blocked403.outcome).toBe("unreachable");
    expect(blocked403.textSha256).toBe(init.textSha256);

    const fullRun = await checkAllGuideSources({
      manifest: {
        schemaVersion: "0.1",
        sources: [{ id: "booth-payment", textSha256: init.textSha256 }],
      },
      checkedDate: "2026-11-11",
      fetchPage: async (url: string) =>
        url.includes("help.vrchat.com")
          ? { ok: false, status: 403, body: "Forbidden" }
          : { ok: true, status: 200, body: "<main><p>Updated upstream policy content long enough to hash cleanly.</p></main>" },
    });
    expect(fullRun.unreachable.map((x: { id: string }) => x.id)).toContain("vrchat-account-upgrade");
    expect(fullRun.changed.map((x: { id: string }) => x.id)).toContain("booth-payment");
    expect(JSON.stringify(fullRun.nextManifest)).not.toContain("Updated upstream policy content");
  });
});
