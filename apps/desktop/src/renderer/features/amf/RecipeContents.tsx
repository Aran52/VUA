import { useEffect,useState } from "react";
import type { AmfRecipeDocument } from "@vua/contracts";
import { catalogBrowser } from "../../app/catalog-browser-instance.ts";
import { openExternalUrl } from "../../app/open-external.ts";
import { termsEverywhereCopy as copy } from "../../i18n/index.ts";
import { recipeProducts,recipeSettings,type RecipeSources } from "./recipe-contents-model.ts";

export function RecipeContents({recipe,sources,revision=0}:{recipe:AmfRecipeDocument;sources:RecipeSources;revision?:number}) {
  const products=recipeProducts(recipe,sources),key=JSON.stringify(products.map(v=>v.productId));
  const [details,setDetails]=useState<Readonly<Record<string,{title:string|null;shopName:string|null}>>>({});
  useEffect(()=>{let alive=true;setDetails({});const ids=[...new Set(products.flatMap(v=>v.productId?[v.productId]:[]))];
    void Promise.all(ids.map(async id=>{try{const detail=await catalogBrowser.detail(id);return detail.kind==="detail"?[id,{title:detail.product.title,shopName:detail.product.attribution?.shopName??null}] as const:null;}catch{return null;}})).then(values=>{if(alive)setDetails(Object.fromEntries(values.filter(v=>v!==null)));});
    return()=>{alive=false;};
  },[key,revision]);
  const settings=recipeSettings(recipe);
  return <section className="vua-recipe-contents"><ul>{products.map(material=>{const detail=material.productId?details[material.productId]:null;return <li key={material.materialId}>
    <strong>{detail?.title??copy.titleUnknown}</strong><small>{material.packageName}</small><small>{detail?.shopName??material.shopName??copy.shopUnknown}</small>
    {material.productId?<button className="vua-recipe-contents__link" type="button" onClick={()=>{void openExternalUrl(`https://booth.pm/items/${material.productId!.slice(6)}`);}}>BOOTH · {material.productId.slice(6)}</button>:<small>{copy.noProducts}</small>}
  </li>;})}</ul><h4>{copy.settings}</h4><dl className="vua-recipe-contents__settings">{(["bindings","merge","attach","switches","fits","lighting","sdkFixes"] as const).map(kind=><div key={kind}><dt>{copy[kind]}</dt><dd>{settings[kind]}</dd></div>)}</dl><p className="vua-caption">{copy.files}</p></section>;
}
