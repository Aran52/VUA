import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { isOutfitRemovalFacts, isOutfitRemovalPlan, isOutfitRemovalApplied } from "./outfit-removal-v01.js";
import { isAmfProductionParams } from "./amf-production-v01.js";
import { DESKTOP_GATEWAY_METHOD_KINDS, isDesktopGatewayRequestV1 } from "./desktop-gateway.js";

const fixture = JSON.parse(readFileSync(new URL("../../../schemas/unity-bridge/v5/examples/outfit-removal.valid.result.json", import.meta.url), "utf8"));
const removal = fixture.data.outfitRemoval[0];
describe("explicit outfit removal contracts", () => {
  it("keeps identities and manual additions in the review, with strict bounded new fields", () => {
    expect(isOutfitRemovalFacts([removal])).toBe(true);
    for (const invalid of [null, {}, [removal, removal], [{...removal, objects:[...removal.objects,removal.objects[0]]}],
      [{...removal,objects:[{...removal.objects[0],manual:"yes"}]}], [{...removal,roots:[{kind:"avatar",globalObjectId:"id"}]}],
      [{...removal,avatarGlobalObjectId:removal.objects[0].globalObjectId}], [{...removal,blockedReason:[]}], [{...removal,blockedReason:"delete_anyway"}]]) {
      expect(isOutfitRemovalFacts(invalid)).toBe(false);
    }
    expect(isOutfitRemovalApplied({sourceId:"outfit",avatarGlobalObjectId:"avatar",deletedObjectIds:["id","id"]})).toBe(false);
  });
  it("requires an exact query and a confirmed digest-bound command", () => {
    const params = {schemaVersion:"0.1",runId:"n3-test",sourceId:"outfit",planDigest:"a".repeat(64),confirmed:true};
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.planOutfitRemoval"]).toBe("query");
    expect(DESKTOP_GATEWAY_METHOD_KINDS["amf.production.removeOutfit"]).toBe("command");
    expect(isAmfProductionParams("amf.production.removeOutfit",params)).toBe(true);
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"remove",method:"amf.production.removeOutfit",params})).toBe(true);
    for (const invalid of [{...params,confirmed:false},{...params,confirmed:undefined},{...params,planDigest:"old"},{...params,sourceId:""},{...params,objects:["id"]}]) {
      expect(isAmfProductionParams("amf.production.removeOutfit",invalid)).toBe(false);
    }
    const plan = {schemaVersion:"0.1",runId:"n3-test",planDigest:params.planDigest,removal};
    expect(isOutfitRemovalPlan(plan)).toBe(true);
    expect(isOutfitRemovalPlan({...plan,removal:{...removal,blockedReason:"contains_owned_object"}})).toBe(false);
  });
});
