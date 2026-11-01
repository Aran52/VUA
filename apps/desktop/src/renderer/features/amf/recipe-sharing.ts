import { isPublishedRecipeLink, isReceivedRecipeLink, isShareImageSaved, type DesktopRecipeSharingV01 } from "@vua/contracts";

/** The sandboxed preload may require only Electron, so share results are checked here, where the
 * renderer bundle carries the contract guards. Every caller reaches recipe sharing through this. */
export function checkedRecipeSharing(api: DesktopRecipeSharingV01 | undefined): DesktopRecipeSharingV01 | undefined {
  if (!api) return undefined;
  const invalid = () => new Error("share_invalid");
  return {
    exportImage: options => api.exportImage(options).then(value => { if (!isShareImageSaved(value)) throw invalid(); return value; }),
    readInput: text => api.readInput(text).then(value => { if (!isReceivedRecipeLink(value)) throw invalid(); return value; }),
    readImage: files => api.readImage(files).then(value => { if (!isReceivedRecipeLink(value)) throw invalid(); return value; }),
    list: recipeId => api.list(recipeId).then(value => { if (!Array.isArray(value) || !value.every(isPublishedRecipeLink)) throw invalid(); return value; }),
    withdraw: code => api.withdraw(code),
    hasPending: () => api.hasPending(),
    takePending: () => api.takePending(),
    subscribe: listener => api.subscribe(listener),
  };
}

export function recipeSharing(): DesktopRecipeSharingV01 | undefined {
  return checkedRecipeSharing(window.vua?.dialog?.recipeSharing);
}
