import { describe, expect, it } from "vitest";
import { compactSourceLabel } from "./line-presentation.ts";

describe("source fallback labels", () => {
  it("drops archive and version noise without joining every material into a title", () => {
    expect(compactSourceLabel("Avatar_Base_v1.2.3_release.zip")).toBe("Avatar Base");
    expect(compactSourceLabel("Coat_2.0.unitypackage")).toBe("Coat");
  });
  it("preserves meaningful digits, names and non-version suffixes", () => {
    expect(compactSourceLabel("Avatar_52.zip")).toBe("Avatar 52");
    expect(compactSourceLabel("めいゆん_衣装.zip")).toBe("めいゆん 衣装");
    expect(compactSourceLabel("My_Avatar_Notes")).toBe("My Avatar Notes");
  });
});
