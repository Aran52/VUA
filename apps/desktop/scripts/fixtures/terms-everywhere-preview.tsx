import { termsPreviewPort } from "./terms-preview.tsx";
// Offline synthetic views of the actual filter and import components.
import React from "react";
import type { AmfSharedRecipe,LibraryUseStatus } from "@vua/contracts";
import { LibraryUseFilter } from "../../src/renderer/features/warehouse/LibraryUseFilter.tsx";
import { AmfRecipeImportDialog } from "../../src/renderer/features/amf/AmfRecipeImportDialog.tsx";
import { Badge } from "../../src/renderer/components/primitives/Badge.tsx";
import { Card } from "../../src/renderer/components/primitives/Card.tsx";
import { termsEverywhereCopy as copy } from "../../src/renderer/i18n/index.ts";
import vectors from "../../../../schemas/share-links/v0.1/code-vectors.json";
const declaration=structuredClone(vectors.find(value=>value.name==="recipe-v05")!.declaration) as AmfSharedRecipe;
const sample:AmfSharedRecipe={...declaration,recipe:{...declaration.recipe,materials:declaration.recipe.materials.map((material,index)=>({...material,boothProductId:String(index+1)}))},sources:Object.fromEntries(declaration.recipe.materials.map((material,index)=>[material.id,{boothProductId:String(index+1),shopName:"Sample Studio"}]))};
export function TermsEverywherePreview({view}:{view:string}) {
  const [use,setUse]=React.useState<Parameters<typeof LibraryUseFilter>[0]["value"]>("vtuber_individual"),[mode,setMode]=React.useState<"available"|"all">("all");
  React.useEffect(()=>{if(!view.endsWith("import"))return;const timer=setTimeout(()=>{const select=document.querySelector<HTMLSelectElement>(".vua-recipe-use select");if(select){select.value="game_development";select.dispatchEvent(new Event("change",{bubbles:true}));}},500);return()=>clearTimeout(timer);},[view]);
  return view.endsWith("filter")?<div className="vua-page" style={{maxWidth:900}}><Card><LibraryUseFilter value={use} mode={mode} disabled={false} onChange={(purpose,display)=>{setUse(purpose);setMode(display);}}/>
    <ul style={{display:"grid",gap:16,padding:0,listStyle:"none"}}>{(["allowed","conditional","contact","forbidden","unknown","no_terms"] as LibraryUseStatus[]).filter(status=>mode==="all"||["allowed","conditional"].includes(status)).map((status,index)=><li key={status} style={{display:"flex",justifyContent:"space-between"}}><span>Sample material {index+1}</span><Badge tone={["allowed","conditional"].includes(status)?"neutral":"warning"}>{copy[status]}</Badge></li>)}</ul></Card></div>
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
