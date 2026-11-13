import { describe, expect, it } from "vitest";
import { boothCategoryLabel } from "./booth-category-copy.ts";

describe("BOOTH category names", () => {
  it("reads BOOTH's English and Japanese names in each language", () => {
    expect(boothCategoryLabel("3D Clothing", "zh-CN")).toBe(boothCategoryLabel("3D衣装", "zh-CN"));
    expect(boothCategoryLabel("3D Clothing", "ja")).toBe("3D衣装");
    expect(boothCategoryLabel("3D衣装", "en")).toBe("3D Clothing");
    expect(boothCategoryLabel("3D Characters", "ko")).not.toBe("3D Characters");
  });

  it("keeps BOOTH's words for a category outside the table", () => {
    expect(boothCategoryLabel("Software", "zh-CN")).toBe("Software");
    expect(boothCategoryLabel("Software", "ja")).toBe("Software");
  });
});
