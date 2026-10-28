import { expect,it,vi } from "vitest";
import { readRecipeTerms } from "./recipe-terms-actions.ts";
it("reads a new product's public description only after the player asks, before refreshing its terms",async()=>{
  const order:string[]=[];
  const capture=vi.fn(async(id:string)=>{order.push(id);return {ok:true};});
  const refresh=vi.fn(async()=>{order.push("terms");return {taskId:"task"};});
  expect((await readRecipeTerms("booth:1","Sample",refresh,capture)).taskId).toBe("task");expect(order).toEqual(["booth:1","terms"]);
  await expect(readRecipeTerms("booth:2","Sample",refresh,async()=>({ok:false}))).rejects.toThrow("terms_capture_failed");expect(refresh).toHaveBeenCalledTimes(1);
  await expect(readRecipeTerms("https://example.test","Sample",refresh,capture)).rejects.toThrow("terms_invalid");expect(capture).toHaveBeenCalledTimes(1);
});
