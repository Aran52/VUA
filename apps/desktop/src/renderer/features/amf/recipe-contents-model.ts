import type { AmfRecipeDocument, AmfRecipeSource, TermsUseDecision, LibraryUseStatus } from "@vua/contracts";
export type RecipeSources = Readonly<Record<string,string|AmfRecipeSource|undefined>>;
export function recipeProducts(recipe:AmfRecipeDocument,sources:RecipeSources) {
  return recipe.materials.map(material=>{
    const source=sources[material.id];const native=typeof source==="string"?source:source?.boothProductId;
    return {materialId:material.id,packageName:material.packageName,productId:native&&/^[1-9][0-9]{0,19}$/u.test(native)?`booth:${native}`:null,shopName:typeof source==="object"?source.shopName??null:null};
  });
}
export function recipeSettings(recipe:AmfRecipeDocument) {
  return {bindings:recipe.outfits.length,merge:recipe.outfits.filter(v=>v.mode==="merge_armature").length,attach:recipe.outfits.filter(v=>v.mode==="attach_to_bone").length,
    switches:recipe.outfits.filter(v=>v.toggleName.trim()!=="").length,fits:recipe.adjustments?.fits?.reduce((sum,fit)=>sum+fit.objects.length,0)??0,lighting:recipe.adjustments?.lighting.length??0,sdkFixes:recipe.adjustments?.sdkFixes.length??0};
}
export function useStatus(value:TermsUseDecision):LibraryUseStatus {
  return value.decision==="conforms"?(value.conditions.length?"conditional":"allowed"):value.decision==="contact_licensor"?"contact":value.decision==="nonconforming"?"forbidden":"unknown";
}
