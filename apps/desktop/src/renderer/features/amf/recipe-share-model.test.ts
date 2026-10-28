import { expect, it } from "vitest";
import { imageExportProblem } from "./recipe-share-model.ts";
it("requires review and consent for the currently selected Recipe only", () => {
  const reviewed={include:true,recipeId:"recipe-a",reviewedRecipeId:"recipe-a",confirmed:true};
  expect(imageExportProblem(reviewed)).toBeNull();
  expect(imageExportProblem({...reviewed,recipeId:"recipe-b"})).toBe("review");
  expect(imageExportProblem({...reviewed,confirmed:false})).toBe("consent");
  expect(imageExportProblem({...reviewed,include:false,recipeId:""})).toBeNull();
});
