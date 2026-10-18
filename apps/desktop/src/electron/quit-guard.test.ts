import { describe, expect, it } from "vitest";
import type { ApplicationRequestV01, ApplicationResponseV01 } from "@vua/contracts";
import { quitStrings, runningTaskCount } from "./quit-guard.js";

const provider = (states: readonly string[] | null, delayMs = 0) => ({
  calls: [] as ApplicationRequestV01[],
  async invoke(request: ApplicationRequestV01): Promise<ApplicationResponseV01> {
    this.calls.push(request);
    if (delayMs > 0) await new Promise(resolve => setTimeout(resolve, delayMs));
    if (states === null) throw new Error("provider gone");
    return { contractVersion: request.contractVersion, requestId: request.requestId, ok: true,
      value: { revision: 1, tasks: states.map((state, index) => ({ taskId: `task-${index}`, state })) } } as unknown as ApplicationResponseV01;
  },
});

describe("asking before quitting", () => {
  it("counts the tasks that are still running, with a read-only task list", async () => {
    const host = provider(["running", "succeeded", "paused", "failed"]);
    expect(await runningTaskCount(host)).toBe(2);
    expect(host.calls).toHaveLength(1);
    expect(host.calls[0]).toMatchObject({ kind: "query", method: "task.list" });
  });
  it("lets the player quit when the provider is gone or slow", async () => {
    expect(await runningTaskCount(provider(null))).toBe(0);
    expect(await runningTaskCount(provider(["running"], 200), 20)).toBe(0);
    expect(await runningTaskCount(provider([]))).toBe(0);
  });
  it("speaks the app language, falling back from the system locale", () => {
    expect(quitStrings("zh-CN").stop).toBe("停止并退出");
    expect(quitStrings("ja-JP").tray).toBe("トレイで続ける");
    expect(quitStrings("fr").cancel).toBe("Cancel");
    expect(quitStrings(null).message).toBe("VUA is still working on a task.");
  });
});
