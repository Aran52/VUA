import { isTermsProductId } from "@vua/contracts";
/** A user-requested read first records the official page, including products new to this library. */
export async function readRecipeTerms(productId:string,title:string,refresh:(id:string,title:string)=>Promise<{readonly taskId:string}>,capture?:((id:string)=>Promise<{readonly ok:boolean}>)|undefined) {
  if(!isTermsProductId(productId))throw new Error("terms_invalid");
  if(capture && !(await capture(productId)).ok)throw new Error("terms_capture_failed");
  return refresh(productId,title);
}
