import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { isApplicationRequestV01 } from "./application-contract.js";
import { isDesktopGatewayRequestV1 } from "./desktop-gateway.js";
import { isLibraryImportPathsAcceptedV02, isLibraryImportPathsParamsV02 } from "./library-intake-v02.js";
const vector = (name: string) => JSON.parse(readFileSync(new URL(`../../../schemas/library-intake/v0.2/examples/${name}.json`, import.meta.url), "utf8")) as Record<string, unknown>;
describe("native mixed library intake v0.2", () => {
  for (const name of ["import", "invalid-relative", "invalid-unknown", "invalid-empty", "invalid-null", "invalid-duplicate"]) {
    it(`consumes ${name} through application and desktop Gateway guards`, () => {
      const request = vector(`${name}.request`);
      expect(isApplicationRequestV01(request)).toBe(name === "import");
      expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: "drop-1", method: request.method,
        params: { ...(request.params as object), commandId: "drop-1" } })).toBe(name === "import");
    });
  }
  it("keeps v0.1 separate and refuses case aliases, NUL, excessive paths and incorrect receipts", () => {
    const p = { schemaVersion: "0.2", sourcePaths: ["C:/A.zip"] };
    for (const invalid of [
      { ...p, schemaVersion: "0.1" }, { ...p, sourceFolders: p.sourcePaths },
      { ...p, sourcePaths: ["C:/A.zip", "c:\\a.zip"] }, { ...p, sourcePaths: ["C:/a\0.zip"] },
      { ...p, sourcePaths: Array.from({ length: 201 }, (_, i) => `C:/${i}.zip`) },
    ]) expect(isLibraryImportPathsParamsV02(invalid)).toBe(false);
    expect(isLibraryImportPathsAcceptedV02(vector("import.response"))).toBe(true);
    expect(isLibraryImportPathsAcceptedV02({ ...vector("import.response"), operation: "library.importFolders" })).toBe(false);
  });
});
