import { expect, it, vi } from "vitest";
import { disposeAmfGeneration, registerAmfGenerationCleanup } from "./amf-generation-cleanup.ts";
it("cleans all generation effects and immediately cleans late effects from a failed import", () => {
  const first = "a".repeat(32), next = "b".repeat(32); const old = vi.fn(), fresh = vi.fn(), late = vi.fn();
  registerAmfGenerationCleanup(first, old); registerAmfGenerationCleanup(first, () => { throw Error("broken cleanup"); });
  registerAmfGenerationCleanup(next, fresh);
  disposeAmfGeneration(first); expect(old).toHaveBeenCalledOnce(); expect(fresh).not.toHaveBeenCalled();
  registerAmfGenerationCleanup(first, late); expect(late).toHaveBeenCalledOnce();
  disposeAmfGeneration(first); expect(old).toHaveBeenCalledOnce(); disposeAmfGeneration(next); expect(fresh).toHaveBeenCalledOnce();
});
