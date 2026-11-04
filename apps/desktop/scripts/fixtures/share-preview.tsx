// Synthetic, offline views of the actual sharing dialogs for visual inspection.
import React from "react";
import { TurntableExportDialog } from "../../src/renderer/features/amf/release/TurntableExportDialog.tsx";
import { ControlRecipesPage } from "../../src/renderer/features/amf/ControlRecipesPage.tsx";
import type { DesktopRecipeSharingV01 } from "@vua/contracts";

const sharing: DesktopRecipeSharingV01 = {
  exportImage:async()=>({saved:true,code:null,url:null}),readInput:async()=>{throw new Error("share_network");},readImage:async()=>{throw new Error("share_image_no_code");},list:async()=>[],withdraw:async()=>{},hasPending:async()=>false,takePending:async()=>null,subscribe:()=>()=>{},
};
export function SharePreview({view}:{view:string}) {
  let pending = true;
  Object.defineProperty(window,"vua",{value:{dialog:{recipeSharing:{...sharing,takePending:async()=>{if(!pending)return null;pending=false;return {error:"share_image_no_code"};}}}},configurable:true});
  return view === "share-export" ? <TurntableExportDialog runId="release-1" onClose={()=>{}}/> : <ControlRecipesPage onNavigate={()=>{}}/>;
}
