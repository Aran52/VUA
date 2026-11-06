import { readdirSync, readFileSync } from "node:fs";
import { expect, it } from "vitest";
import { isLibraryListParamsV03, isLibraryListV03 } from "./library-view-v03.js";
import { isDesktopGatewayRequestV1 } from "./desktop-gateway.js";
it("consumes both faces of the versioned vectors", () => {
  const root = new URL("../../../schemas/library-view/v0.3/examples/", import.meta.url);
  for (const name of readdirSync(root)) {
    const value = JSON.parse(readFileSync(new URL(name, root), "utf8"));
    const valid = name.includes(".valid.");
    if (name.endsWith("request.json")) expect(isLibraryListParamsV03(value.params), name).toBe(valid);
    else expect(isLibraryListV03(value), name).toBe(valid);
  }
});
it("rejects the retired purpose fields and unknown words", () => {
  expect(isLibraryListParamsV03({ schemaVersion: "0.3", use: "vrchat_personal" })).toBe(false);
  expect(isLibraryListParamsV03({ schemaVersion: "0.3", sort: "newest" })).toBe(false);
  expect(isLibraryListParamsV03({ schemaVersion: "0.3", vn3: { items: ["X"] } })).toBe(false);
  expect(isLibraryListParamsV03({ schemaVersion: "0.3", vn3: { items: ["A", "A"] } })).toBe(false);
  expect(isLibraryListParamsV03({ schemaVersion: "0.2", sort: "title_asc" })).toBe(false);
});
it("reaches the gateway as a query", () => {
  expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: "a", method: "library.list", params: { schemaVersion: "0.3", sort: "added_desc", vn3: { items: ["C", "J"] } } })).toBe(true);
});
