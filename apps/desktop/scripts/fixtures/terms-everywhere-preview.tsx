import { termsPreviewPort } from "./terms-preview.tsx";
// Offline synthetic views of the actual filter and import components.
import React from "react";
import type { AmfSharedRecipe } from "@vua/contracts";
import { LibraryVn3Filter } from "../../src/renderer/features/warehouse/LibraryVn3Filter.tsx";
import { NO_VN3_CHOICE } from "../../src/renderer/features/warehouse/library-view-model.ts";
import { AmfRecipeImportDialog } from "../../src/renderer/features/amf/AmfRecipeImportDialog.tsx";
import { Card } from "../../src/renderer/components/primitives/Card.tsx";
import vectors from "../../../../schemas/share-links/v0.1/code-vectors.json";
const declaration=structuredClone(vectors.find(value=>value.name==="recipe-v05")!.declaration) as AmfSharedRecipe;
const sample:AmfSharedRecipe={...declaration,recipe:{...declaration.recipe,materials:declaration.recipe.materials.map((material,index)=>({...material,boothProductId:String(index+1)}))},sources:Object.fromEntries(declaration.recipe.materials.map((material,index)=>[material.id,{boothProductId:String(index+1),shopName:"Sample Studio"}]))};
export function TermsEverywherePreview({view}:{view:string}) {
  const [choice,setChoice]=React.useState({...NO_VN3_CHOICE,items:["C","J"] as ("C"|"J")[]});
  React.useEffect(()=>{if(!view.endsWith("import"))return;const timer=setTimeout(()=>{const select=document.querySelector<HTMLSelectElement>(".vua-recipe-use select");if(select){select.value="game_development";select.dispatchEvent(new Event("change",{bubbles:true}));}},500);return()=>clearTimeout(timer);},[view]);
  return view.endsWith("filter")?<div className="vua-page vua-warehouse" style={{maxWidth:900}}><div className="vua-warehouse__filters"><LibraryVn3Filter value={choice} disabled={false} open onOpenChange={()=>{}} onUpdated={()=>{}} onChange={next=>setChoice(next as typeof choice)}
      counts={{readable:18,noVn3:6,unread:21,unreadable:2,items:{A:18,B:4,C:17,D:9,E:2,F:6,G:12,H:1,I:18,J:16,K:11,L:13,M:0,N:1,O:15,P:7,Q:3,R:2,S:5,T:9,U:14,V:10,W:0}}}/></div></div>
    :<AmfRecipeImportDialog initial={{received:{document:sample,text:JSON.stringify(sample),code:null,preview:null}}} onClose={()=>{}} onOpen={()=>{}} onImported={()=>{}}/>;
}

export const recipeTermsPreviewPort = {
  ...termsPreviewPort("terms-parsed"),
  async checkUse(preset:Parameters<ReturnType<typeof termsPreviewPort>["checkUse"]>[0],ids:readonly string[]){
    const check=await termsPreviewPort("terms-parsed").checkUse(preset,ids);
    const products=check.products.map(product=>preset==="game_development"&&product.decision==="contact_licensor"?{...product,blockingItems:[{productId:product.productId,item:"R",termKey:"vn3.R.software_integration",reason:"contact_licensor"}]}:product);
    return {...check,products,combination:{...check.combination,blockingItems:products.flatMap(product=>product.blockingItems)}};
  },
};
