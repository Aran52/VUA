import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import {
  isAllowedGuideHttpsUrl,
  isGuideContentIdV01,
  isGuideContentPayloadV01,
  isSignedGuideEnvelopeV01,
  resolveGuideLocalizedText,
} from "./guide-content.js";

const examplesDir = new URL("../../../schemas/guide-content/v0.1/examples/", import.meta.url);
const readExample = (name: string) => JSON.parse(readFileSync(new URL(name, examplesDir), "utf8"));

describe("guide-content v0.1 contract", () => {
  it("validates payload and envelope vectors while tolerating unknown fields", () => {
    for (const name of readdirSync(examplesDir)) {
      const value = readExample(name);
      const expected = name.includes(".valid.");
      if (name.startsWith("envelope")) {
        expect(isSignedGuideEnvelopeV01(value), name).toBe(expected);
      } else {
        expect(isGuideContentPayloadV01(value), name).toBe(expected);
      }
    }
  });

  it("enforces closed guide IDs, official HTTPS link hosts, and English locale fallback", () => {
    expect(isGuideContentIdV01("paying-abroad")).toBe(true);
    expect(isGuideContentIdV01("https://evil.example.com")).toBe(false);
    expect(isGuideContentIdV01("../paying-abroad")).toBe(false);

    expect(isAllowedGuideHttpsUrl("https://booth.pm/payment")).toBe(true);
    expect(isAllowedGuideHttpsUrl("https://accounts.pixiv.net/login")).toBe(true);
    expect(isAllowedGuideHttpsUrl("http://booth.pm/payment")).toBe(false);
    expect(isAllowedGuideHttpsUrl("https://user:pass@booth.pm/payment")).toBe(false);
    expect(isAllowedGuideHttpsUrl("https://untrusted.example.org/payment")).toBe(false);

    expect(resolveGuideLocalizedText({ en: "English fallback", "zh-CN": "简体中文" }, "zh-CN")).toBe("简体中文");
    expect(resolveGuideLocalizedText({ en: "English fallback", ja: "   " }, "ja")).toBe("English fallback");
    expect(resolveGuideLocalizedText({ en: "English fallback" }, "ko")).toBe("English fallback");
  });
});
