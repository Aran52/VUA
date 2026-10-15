// A pending selection survives page remounts, without persisting an execution request.
let requestedRecipe: string | null = null;
export function requestProductionRecipe(recipeId: string): void { requestedRecipe = recipeId; }
export function takeRequestedProductionRecipe(): string | null {
  const recipeId = requestedRecipe; requestedRecipe = null; return recipeId;
}
