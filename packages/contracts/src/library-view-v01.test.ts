import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { isApplicationRequestV01 } from "./application-contract.js";
import { isDesktopGatewayRequestV1, DESKTOP_GATEWAY_METHOD_KINDS } from "./desktop-gateway.js";
import { isLibraryListV01, isLibraryProductFilesV01, isLibraryViewParamsV01 } from "./library-view-v01.js";
const vector = (name: string): Record<string, unknown> => JSON.parse(readFileSync(fileURLToPath(new URL(`../../../schemas/library-view/v0.1/examples/${name}.json`, import.meta.url)), "utf8")) as Record<string, unknown>;
describe("library-view v0.1 consumers", () => {
  it("accepts explicit imported origin without changing the retained physical-local source", () => {
    const request = vector("imported-source.request");
    expect(isLibraryViewParamsV01(String(request.method), request.params)).toBe(true);
    expect(isLibraryViewParamsV01("library.list", { schemaVersion: "0.1", source: "local" })).toBe(true);
  });
  for (const name of ["list.request", "files.request", "category.request", "unknown-category.request", "invalid-category.request", "invalid-session.request", "invalid-limit.request"]) {
    it(name, () => {
      const request = vector(name); const valid = !name.startsWith("invalid");
      expect(isLibraryViewParamsV01(String(request.method), request.params)).toBe(valid);
      expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: "request-synthetic", ...request })).toBe(valid);
      expect(isApplicationRequestV01({ contractVersion: "0.1", requestId: "request-synthetic", correlationId: "synthetic", kind: "query", ...request })).toBe(valid);
      expect(DESKTOP_GATEWAY_METHOD_KINDS[String(request.method) as "library.list" | "library.productFiles"]).toBe("query");
    });
  }
  it("accepts actual multi-source and physical-copy evidence without paths", () => {
    expect(isLibraryListV01(vector("list.response"))).toBe(true);
    expect(isLibraryListV01(vector("category.response"))).toBe(true);
    expect(isLibraryListV01(vector("local-source.response"))).toBe(true);
    expect(isLibraryProductFilesV01(vector("files.response"))).toBe(true);
    expect(isLibraryProductFilesV01(vector("invalid-path.response"))).toBe(false);
  });
  it("keeps source suggestions separate and validates each card's file scope", () => {
    const list = vector("local-source.response");
    const rows = list.items as Record<string, unknown>[];
    const local = rows[1]!;
    (local.sourceMatch as Record<string, unknown>).content = "equal";
    expect(isLibraryListV01(list)).toBe(false);
    (local.sourceMatch as Record<string, unknown>).content = "unverified";
    local.copyIds = ["cpy-imported", "cpy-other-card"];
    expect(isLibraryListV01(list)).toBe(false);
    local.copyIds = ["cpy-imported"];
    (local.sourceMatch as Record<string, unknown>).basis = "confirmed_by_title";
    expect(isLibraryListV01(list)).toBe(false);
  });
  it("rejects inflated counts and unbound managed targets", () => {
    const list = vector("list.response"); const rows = list.items as Record<string, unknown>[];
    (rows[0]?.storage as Record<string, unknown>).presentCopies = 2;
    expect(isLibraryListV01(list)).toBe(false);
    const misleading = vector("list.response");
    ((misleading.items as Record<string, unknown>[])[0]?.storage as Record<string, unknown>).state = "missing";
    expect(isLibraryListV01(misleading)).toBe(false);
    const files = vector("files.response"); (files.items as Record<string, unknown>[])[0]!.managedCopyId = "cpy-not-listed";
    expect(isLibraryProductFilesV01(files)).toBe(false);
  });
  it("accepts native category facets and explicit unknown filters, rejecting guesses of other types", () => {
    for (const sourceCategory of [null, "3D衣装", "カテゴリー名", "a".repeat(200)]) expect(isLibraryViewParamsV01("library.list", { schemaVersion: "0.1", sourceCategory })).toBe(true);
    for (const sourceCategory of ["", 3, [], "a".repeat(201)]) expect(isLibraryViewParamsV01("library.list", { schemaVersion: "0.1", sourceCategory })).toBe(false);
    const response = vector("list.response");
    response.sourceCategories = ["3D衣装"];
    (response.items as Record<string, unknown>[])[0]!.sourceCategory = "3D衣装";
    expect(isLibraryListV01(response)).toBe(true);
    response.sourceCategories = ["3D衣装", "3D衣装"];
    expect(isLibraryListV01(response)).toBe(false);
    response.sourceCategories = [3];
    expect(isLibraryListV01(response)).toBe(false);
  });
});
