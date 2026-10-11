import { describe, expect, it } from "vitest";
import { libraryCornerState } from "./library-corner-model.ts";
import type { LibraryCardFacts } from "../../gateway/index.ts";
const facts: LibraryCardFacts = { sources: ["bought"], operation: null, storage: { state: "cloud_only", storedCopies: 0, presentCopies: 0, missingCopies: 0, changedCopies: 0, unreadableCopies: 0, supersededGeneratedCopies: 0, currentGeneratedCopies: 0, productionQualification: "not_evaluated" } };
describe("library corner states", () => {
  it("separates public source association from account ownership", () => {
    expect(libraryCornerState(facts, "product")).toBe("cloud");
    expect(libraryCornerState({ ...facts, sources: [], storage: { ...facts.storage, state: "present", presentCopies: 1 } }, "local")).toBe("local");
    expect(libraryCornerState(facts, "local", { sourceSearch: "matched" })).toBe("linked");
  });
  it("prefers actual active controls, missing files and explicit uncertainty", () => {
    expect(libraryCornerState(facts, "product", { active: true, paused: true })).toBe("paused");
    expect(libraryCornerState({ ...facts, storage: { ...facts.storage, state: "missing" } }, "product")).toBe("error");
    expect(libraryCornerState(facts, "local", { sourceSearch: "uncertain" })).toBe("uncertain");
    expect(libraryCornerState(facts, "local", { sourceSearch: "searching" })).toBe("searching");
  });
});
