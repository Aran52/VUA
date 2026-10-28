/** Public consent belongs to one export and to the reviewed saved Recipe. */
export function imageExportProblem(value: { include: boolean; recipeId: string; reviewedRecipeId: string | null; confirmed: boolean }): "recipe" | "review" | "consent" | null {
  if (!value.include) return null;
  if (!value.recipeId) return "recipe";
  if (value.reviewedRecipeId !== value.recipeId) return "review";
  return value.confirmed ? null : "consent";
}
