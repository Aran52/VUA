import { LIBRARY_VN3_ITEMS } from "@vua/contracts";
import { describe, expect, it } from "vitest";
import { libraryViewTables } from "../../i18n/library-view-copy.ts";
import { EMPTY_LIBRARY_SESSION, headerSortState, libraryFiltersActive, neighboursOf, nextHeaderSort, NO_VN3_CHOICE, offeredLibrarySorts, sortFactOf, toggleVn3Item, VN3_FILTER_GROUPS, vn3ChoiceActive, vn3FilterWire } from "./library-view-model.ts";

describe("sort choices", () => {
  it("offers BOOTH's publish date once a row has one, and keeps a stored choice", () => {
    expect(offeredLibrarySorts("added_desc", [{ publishedAt: null }, {}])).not.toContain("published_desc");
    expect(offeredLibrarySorts("added_desc", [{ publishedAt: null }, { publishedAt: "2026-01-01T00:00:00Z" }])).toContain("published_desc");
    expect(offeredLibrarySorts("published_desc", [])).toContain("published_desc");
    expect(offeredLibrarySorts("added_desc", [])).toContain("size_desc");
  });
});

describe("the VN3 filter", () => {
  it("keeps letters in VN3 order and sends only what is chosen", () => {
    expect(vn3FilterWire(NO_VN3_CHOICE)).toEqual({});
    const one = toggleVn3Item(NO_VN3_CHOICE, "J");
    const two = toggleVn3Item(one, "C");
    expect(two.items).toEqual(["C", "J"]);
    expect(vn3FilterWire({ ...two, includeContact: true, state: "readable" })).toEqual({ state: "readable", items: ["C", "J"], includeContact: true });
    expect(toggleVn3Item(two, "C").items).toEqual(["J"]);
  });

  it("counts as active only when it narrows the list", () => {
    expect(vn3ChoiceActive(NO_VN3_CHOICE)).toBe(false);
    expect(vn3ChoiceActive({ ...NO_VN3_CHOICE, includeContact: true })).toBe(false);
    expect(vn3ChoiceActive({ ...NO_VN3_CHOICE, state: "unread" })).toBe(true);
    expect(vn3ChoiceActive(toggleVn3Item(NO_VN3_CHOICE, "A"))).toBe(true);
  });

  it("offers each item once, and leaves the transfer of rights to the details", () => {
    const offered = VN3_FILTER_GROUPS.flatMap(group => group.items);
    expect(new Set(offered).size).toBe(offered.length);
    expect([...offered].sort()).toEqual(LIBRARY_VN3_ITEMS.filter(item => item !== "W"));
  });

  it("names every group in every language", () => {
    for (const copy of Object.values(libraryViewTables)) for (const group of VN3_FILTER_GROUPS) expect(copy.vn3.groups[group.id]).toBeTruthy();
  });
});

describe("sorting", () => {
  it("shows the fact a non-default order goes by", () => {
    expect(sortFactOf("added_desc")).toBeNull();
    expect(sortFactOf("title_asc")).toBeNull();
    expect(sortFactOf("size_desc")).toBe("size");
    expect(sortFactOf("price_desc")).toBe("price");
    expect(sortFactOf("published_desc")).toBe("published");
    expect(sortFactOf("added_asc")).toBe("added");
  });

  it("reverses the title column on a second click", () => {
    expect(nextHeaderSort("title", "added_desc")).toBe("title_asc");
    expect(nextHeaderSort("title", "title_asc")).toBe("title_desc");
    expect(nextHeaderSort("shop", "title_asc")).toBe("shop_asc");
    expect(nextHeaderSort("size", "title_asc")).toBe("size_desc");
    expect(nextHeaderSort("size", "size_desc")).toBe("size_asc");
    expect(nextHeaderSort("added", "added_desc")).toBe("added_asc");
    expect(nextHeaderSort("added", "added_asc")).toBe("added_desc");
    expect(headerSortState("size", "size_asc")).toBe("ascending");
    expect(headerSortState("added", "added_desc")).toBe("descending");
    expect(headerSortState("shop", "title_asc")).toBe("none");
  });
});

describe("the details page's neighbours", () => {
  it("stops at both ends of the page", () => {
    const order = ["a", "b", "c"];
    expect(neighboursOf(order, item => item === "a")).toEqual({ index: 0, total: 3, previous: null, next: "b" });
    expect(neighboursOf(order, item => item === "c")).toEqual({ index: 2, total: 3, previous: "b", next: null });
    expect(neighboursOf(order, item => item === "z")).toEqual({ index: -1, total: 3, previous: null, next: null });
  });
});

describe("the session's filters", () => {
  it("starts with nothing narrowing the list", () => {
    expect(libraryFiltersActive(EMPTY_LIBRARY_SESSION)).toBe(false);
    expect(libraryFiltersActive({ ...EMPTY_LIBRARY_SESSION, text: "  " })).toBe(false);
    expect(libraryFiltersActive({ ...EMPTY_LIBRARY_SESSION, sourceCategory: "category:3D" })).toBe(true);
  });
});
