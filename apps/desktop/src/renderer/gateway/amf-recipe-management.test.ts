import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { createAmfProductionPort, isAmfRecipeSummary } from "./amf-production-port.ts";
import type { GatewayClient } from "./gateway-client.ts";

function harness() {
  let response: unknown;
  const requests: unknown[] = [];
  const client = { invoke: async (request: unknown) => { requests.push(request); return { ok: true, value: response }; }, subscribe: () => () => {} } as unknown as GatewayClient;
  return { port: createAmfProductionPort(client), requests, reply: (value: unknown) => { response = value; } };
}
const summary = { recipeId: "recipe-test", title: "Avatar", revision: 1, updatedAt: "2026-11-11T00:00:00Z" };
const recipe = JSON.parse(readFileSync(new URL("../../../../../schemas/amf-recipe/v0.5/examples/recipe.valid.json", import.meta.url), "utf8"));

describe("local Recipe management boundary", () => {
  it("accepts old summaries and checks each additive material fact", async () => {
    const h = harness();
    const enriched = { ...summary, avatarName: "Avatar.unitypackage", outfitCount: 1, materialCount: 2, packageCount: 3, boothProductIds: ["1000001"] };
    h.reply({ schemaVersion: "0.1", recipes: [summary, enriched] });
    await expect(h.port.listRecipes()).resolves.toEqual([summary, enriched]);
    for (const bad of [{ outfitCount: -1 }, { materialCount: 1.1 }, { packageCount: null }, { avatarName: null }, { boothProductIds: ["0"] }, { boothProductIds: ["12", "12"] }]) {
      expect(isAmfRecipeSummary({ ...enriched, ...bad })).toBe(false);
    }
    expect(h.requests).toHaveLength(1);
    expect(h.requests[0]).toMatchObject({ method: "amf.production.listRecipes" });
  });

  it("binds rename and delete receipts to the requested identity and revision", async () => {
    const h = harness();
    const document = { recipe: { ...recipe, recipeId: "recipe-test", title: "Renamed" }, revision: 2, updatedAt: summary.updatedAt };
    h.reply({ schemaVersion: "0.1", document });
    await expect(h.port.renameRecipe!("recipe-test", 1, "Renamed")).resolves.toEqual(document);
    expect(h.requests.at(-1)).toMatchObject({ method: "amf.production.renameRecipe", params: { expectedRevision: 1, title: "Renamed" } });
    for (const wrong of [{ ...document, revision: 3 }, { ...document, recipe: { ...document.recipe, recipeId: "other" } }, { ...document, recipe: { ...document.recipe, title: "Different" } }]) {
      h.reply({ schemaVersion: "0.1", document: wrong });
      await expect(h.port.renameRecipe!("recipe-test", 1, "Renamed")).rejects.toThrow("amf_recipe_document_invalid");
    }
    h.reply({ schemaVersion: "0.1", recipeId: "other", deleted: true });
    await expect(h.port.deleteRecipe!("recipe-test", 2)).rejects.toThrow("amf_recipe_delete_invalid");
    h.reply({ schemaVersion: "0.1", recipeId: "recipe-test", deleted: true });
    await expect(h.port.deleteRecipe!("recipe-test", 2)).resolves.toBeUndefined();
  });

  it("returns null for an absent image and rejects invalid or oversized PNG receipts", async () => {
    const h = harness();
    const image = { mediaType: "image/png", data: Buffer.from("\x89PNG\r\n\x1a\n", "latin1").toString("base64") };
    const reply = (value: unknown) => h.reply({ schemaVersion: "0.1", recipeId: "recipe-test", image: value });
    reply(null); await expect(h.port.recipeImage!("recipe-test")).resolves.toBeNull();
    reply(image); await expect(h.port.recipeImage!("recipe-test")).resolves.toEqual(image);
    const large = Buffer.alloc(655361); Buffer.from("\x89PNG\r\n\x1a\n", "latin1").copy(large);
    for (const wrong of [{ ...image, mediaType: "image/jpeg" }, { ...image, data: "not base64" }, { ...image, data: large.toString("base64") }, {}, undefined]) {
      reply(wrong); await expect(h.port.recipeImage!("recipe-test")).rejects.toThrow("amf_recipe_image_invalid");
    }
    h.reply({ schemaVersion: "0.1", recipeId: "other", image });
    await expect(h.port.recipeImage!("recipe-test")).rejects.toThrow("amf_recipe_image_invalid");
  });

  it("reads optional source Recipes on line summaries without requiring them from older AMF", async () => {
    const h = harness();
    const row = { runId: "n3-old", phase: "succeeded", sourceNames: [], projectGone: true };
    const reply = (run: unknown) => h.reply({ schemaVersion: "0.1", sources: [], editors: [], runs: [run], capacity: 1, running: [], queue: [], busy: false });
    reply(row); await expect(h.port.list()).resolves.toMatchObject({ runs: [row] });
    reply({ ...row, recipeId: "recipe-test" }); await expect(h.port.list()).resolves.toMatchObject({ runs: [{ recipeId: "recipe-test" }] });
    for (const id of [null, "", "../escape", 7]) {
      reply({ ...row, recipeId: id }); await expect(h.port.list()).rejects.toThrow("amf_production_list_invalid");
    }
  });
});
