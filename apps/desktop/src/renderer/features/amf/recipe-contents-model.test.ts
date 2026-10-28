import { expect,it } from "vitest";
import { readFileSync } from "node:fs";
import type { AmfSharedRecipe,TermsUseDecision } from "@vua/contracts";
import { recipeProducts,recipeSettings,useStatus } from "./recipe-contents-model.ts";
const sample=(version:string)=>JSON.parse(readFileSync(new URL(`../../../../../../schemas/amf-recipe/v${version}/share-examples/share.valid.json`,import.meta.url),"utf8")) as AmfSharedRecipe;
it("reads v0.4 and v0.5 source identities without treating a package name as a BOOTH source",()=>{
  for(const version of ["0.4","0.5"]){const declaration=sample(version);const rows=recipeProducts(declaration.recipe,declaration.sources);expect(rows.every(row=>/^booth:[1-9][0-9]*$/u.test(row.productId??""))).toBe(true);expect(recipeProducts(declaration.recipe,{}).every(row=>row.productId===null)).toBe(true);}
});
it("counts actual switch and object settings while reading old missing adjustments",()=>{
  const recipe=sample("0.5").recipe;const {adjustments:_adjustments,...plain}=recipe;expect(recipeSettings(plain).lighting).toBe(0);expect(recipeSettings(plain).fits).toBe(0);
  expect(recipeSettings({...plain,outfits:recipe.outfits.map(row=>({...row,toggleName:""}))}).switches).toBe(0);
  expect(recipeSettings(recipe).bindings).toBe(recipe.outfits.length);
});
it("never presents unknown or a contact decision as usable",()=>{
  const base:TermsUseDecision={decision:"unknown",conditions:[],blockingItems:[],advisories:[],hasSpecialNote:false,specialNotes:[]};expect(useStatus(base)).toBe("unknown");expect(useStatus({...base,decision:"contact_licensor"})).toBe("contact");expect(useStatus({...base,decision:"nonconforming"})).toBe("forbidden");expect(useStatus({...base,decision:"conforms",conditions:[{productId:"booth:1",item:"J",condition:"modified_distinguishable"}]})).toBe("conditional");
});
