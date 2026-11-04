import { readFileSync, readdirSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { isAmfRecipeDocument, isAmfSharedRecipe, decodeAmfRecipeShare } from "./amf-recipe-v05.js";
import { isDesktopGatewayRequestV1 } from "./desktop-gateway.js";
import { isAmfProductionParams } from "./amf-production-v01.js";
import { hasValidAmfRecipeReview, isAmfRecipeMaterials } from "./amf-recipe-materials.js";

describe("Recipe v0.5 with retained v0.4 reads", () => {
  for (const version of ["v0.4","v0.5"]) {
    for (const [folder,validate] of [["examples",isAmfRecipeDocument],["share-examples",isAmfSharedRecipe]] as const) {
      const root = new URL(`../../../schemas/amf-recipe/${version}/${folder}/`,import.meta.url);
      for (const name of readdirSync(root)) it(`${version}/${folder}/${name}`, () => {
        const text = readFileSync(new URL(name,root),"utf8"); const valid = name.includes(".valid.");
        expect(validate(JSON.parse(text))).toBe(valid);
        if (folder === "share-examples") {
          expect(decodeAmfRecipeShare(text) !== null).toBe(valid);
          expect(decodeAmfRecipeShare(`vuar${version.slice(1)}.${Buffer.from(text).toString("base64url")}`) !== null).toBe(valid);
        }
      });
    }
  }
  it("binds bounded explicit choices and difference confirmation to the request", () => {
    const materialChoices={"material-2":"source-2"};
    expect(isAmfProductionParams("amf.production.recipeMaterialPlan",{schemaVersion:"0.1",recipeId:"recipe-1",materialChoices})).toBe(true);
    expect(isDesktopGatewayRequestV1({schemaVersion:1,requestId:"materials",method:"amf.production.recipeMaterialPlan",params:{schemaVersion:"0.1",recipeId:"recipe-1",materialChoices}})).toBe(true);
    const params={schemaVersion:"0.1",runId:"n3-test",recipeId:"recipe-1",choice:"recipe_wins",planDigest:"a".repeat(64),materialChoices,recipeDifferencesConfirmed:true};
    expect(isAmfProductionParams("amf.production.applyRecipe",params)).toBe(true);
    for (const bad of [{...params,materialChoices:{"../local":"source"}},{...params,materialChoices:{"material-2":2}},{...params,recipeDifferencesConfirmed:false}]) {
      expect(isAmfProductionParams("amf.production.applyRecipe",bad)).toBe(false);
    }
  });
  it("accepts omitted review fields and rejects malformed new ones", () => {
    expect(hasValidAmfRecipeReview({})).toBe(true);
    expect(hasValidAmfRecipeReview({materials:[{materialId:"material-1",status:"ready"}]})).toBe(false);
    const missing={materialId:"material-1",packageName:"Coat.unitypackage",boothProductId:"1001",status:"material_not_downloaded",resolved:null,candidates:[],downloadNames:["Coat.zip"]};
    expect(isAmfRecipeMaterials([missing])).toBe(true);
    expect(isAmfRecipeMaterials([missing,{...missing}])).toBe(false);
    expect(isAmfRecipeMaterials([{...missing,boothProductId:"100x"}])).toBe(false);
    expect(hasValidAmfRecipeReview({recipeDifferences:[{kind:"fit_objects_missing",materialId:"material-1",before:"",after:null,rule:"path",paths:["Clip"],count:1}]})).toBe(true);
    expect(hasValidAmfRecipeReview({recipeDifferences:[{kind:"fit_objects_missing",materialId:"material-1",before:"",after:null,rule:"path",paths:["Clip"],count:2}]})).toBe(false);
  });
});
