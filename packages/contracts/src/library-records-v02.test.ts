import { expect, test } from "vitest";
import { readFileSync } from "node:fs";
import { isLibraryRemoveCardsV02 } from "./library-records-v02.js";
import { isDesktopGatewayRequestV1 } from "./desktop-gateway.js";
import { isApplicationRequestV01 } from "./application-contract.js";
test("card removal preserves the exact target set across both command envelopes and rejects malformed selections", () => {
  const vector = JSON.parse(readFileSync(new URL("../../../schemas/library-records/v0.2/examples/remove.request.json", import.meta.url), "utf8"));
  const params = vector.params;
  expect(isLibraryRemoveCardsV02(params)).toBe(true);
  expect(isDesktopGatewayRequestV1({ schemaVersion: 1, requestId: "r", method: "library.removeCards", params: { ...params, commandId: "remove-once" } })).toBe(true);
  const application = { contractVersion: "0.1", requestId: "r", correlationId: "c", kind: "command", method: "library.removeCards", commandId: "remove-once", params };
  expect(isApplicationRequestV01(application)).toBe(true);
  expect(isApplicationRequestV01({ ...application, extra: true })).toBe(false);
  for (const value of [{ ...params, targets: [] }, { ...params, targets: [params.targets[0], params.targets[0]] }, { ...params, targets: [{ kind: "product", id: "booth:x" }] }, { ...params, targets: [{ kind: "entry", id: "../root" }] }, { ...params, unknown: true }]) expect(isLibraryRemoveCardsV02(value)).toBe(false);
});
