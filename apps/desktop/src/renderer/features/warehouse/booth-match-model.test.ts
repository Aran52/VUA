import { describe, expect, it } from "vitest";
import { boothSearchKeyword, exactBoothMatch } from "./booth-match-model.ts";
const item = (productId: string, title: string) => ({ productId, title, shopName: null, sourceCategory: null });
describe("public BOOTH provenance matching", () => {
  it("uses a leaf label and strips a file/version suffix without disclosing a parent directory", () => {
    expect(boothSearchKeyword("C:\\private\\Synthetic_Coat_v1.2.zip")).toBe("Synthetic Coat");
  });
  it("requires a unique exact title and a complete page, never the top keyword hit", () => {
    const result = { schemaVersion: "0.1" as const, query: "Synthetic coat", truncated: false, items: [item("booth:90", "Synthetic coat"), item("booth:91", "Synthetic coat accessories")] };
    expect(exactBoothMatch(result, "Synthetic coat")?.productId).toBe("booth:90");
    expect(exactBoothMatch({ ...result, truncated: true }, result.query)).toBeNull();
    expect(exactBoothMatch({ ...result, items: [...result.items, item("booth:92", "Synthetic coat")] }, result.query)).toBeNull();
    expect(exactBoothMatch(result, "coat")).toBeNull();
  });
});
