import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { isApplicationRequestV01 } from "./application-contract.js";
import { DESKTOP_GATEWAY_METHOD_KINDS, isDesktopGatewayRequestV1 } from "./desktop-gateway.js";
import {
  isRecipeDraftLegacyCensusV01,
  isRecipeDraftLegacyMigrationV01,
  isRecipeDraftListV01,
  isRecipeDraftMaterialsForLineV01,
  isRecipeDraftParamsV01,
  isRecipeDraftReadV01,
  isRecipeDraftSelectionStatusV01,
} from "./recipe-selection-draft-v01.js";
const vector = (name: string): Record<string, unknown> => JSON.parse(readFileSync(fileURLToPath(new URL(`../../../schemas/recipe-selection-draft/v0.1/examples/${name}.json`, import.meta.url)), "utf8")) as Record<string, unknown>;
describe("selection drafts stay separate from production Recipe", () => {
  for (const name of ["list.request", "get.request", "status.request", "materials.request", "legacy-census.request", "migrate-legacy.request", "save.request", "add.request", "invalid-formal.request", "invalid-path.request", "invalid-revision.request"]) {
    it(name, () => {
      const request = vector(name); const valid = !name.startsWith("invalid");
      const kind = ["recipeDraft.list", "recipeDraft.get", "recipeDraft.selectionStatus", "recipeDraft.materialsForLine", "recipeDraft.legacyCensus"].includes(String(request.method)) ? "query" : "command";
      expect(isRecipeDraftParamsV01(String(request.method), request.params)).toBe(valid);
      expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: "synthetic", ...request })).toBe(valid);
      expect(isApplicationRequestV01({ contractVersion: "0.1", requestId: "synthetic", correlationId: "synthetic", kind, ...(kind === "command" ? { commandId: "synthetic" } : {}), ...request })).toBe(valid);
      expect(DESKTOP_GATEWAY_METHOD_KINDS[request.method as "recipeDraft.save"]).toBe(kind);
    });
  }
  it("validates complete reads and refuses misleading append receipts", () => {
    expect(isRecipeDraftListV01(vector("list.response"))).toBe(true);
    expect(isRecipeDraftReadV01(vector("read.response"))).toBe(true);
    expect(isRecipeDraftReadV01(vector("add.response"))).toBe(true);
    expect(isRecipeDraftReadV01(vector("invalid-kind.response"))).toBe(false);
    const broken = vector("add.response"); broken.addedCount = 10;
    expect(isRecipeDraftReadV01(broken)).toBe(false);
    delete broken.existingCount;
    expect(isRecipeDraftReadV01(broken)).toBe(false);
  });
  it("binds missing-file facts to sequential selections without inventing presence", () => {
    const status = vector("status.response"); expect(isRecipeDraftSelectionStatusV01(status)).toBe(true);
    const row = (status.items as Record<string, unknown>[])[0]!;
    row.state = "present"; expect(isRecipeDraftSelectionStatusV01(status)).toBe(false);
    row.state = "missing"; row.index = 2; expect(isRecipeDraftSelectionStatusV01(status)).toBe(false);
  });
  it("validates line material prefill without inferred roles and legacy migration receipts", () => {
    const materials = vector("materials.response"); expect(isRecipeDraftMaterialsForLineV01(materials)).toBe(true);
    const overlapping = structuredClone(materials);
    ((overlapping.sources as Record<string, unknown>[])[0]!.selectionIndices as number[]).push(1);
    expect(isRecipeDraftMaterialsForLineV01(overlapping)).toBe(false);
    const withRole = structuredClone(materials);
    (withRole.sources as Record<string, unknown>[])[0]!.role = "avatar";
    expect(isRecipeDraftMaterialsForLineV01(withRole)).toBe(false);
    const census = vector("legacy-census.response"); expect(isRecipeDraftLegacyCensusV01(census)).toBe(true);
    const badCensus = structuredClone(census); badCensus.parseableDocuments = 99;
    expect(isRecipeDraftLegacyCensusV01(badCensus)).toBe(false);
    const migration = vector("migrate-legacy.response"); expect(isRecipeDraftLegacyMigrationV01(migration)).toBe(true);
    const badMigration = structuredClone(migration); badMigration.migratedCount = 0;
    expect(isRecipeDraftLegacyMigrationV01(badMigration)).toBe(false);
  });
});
