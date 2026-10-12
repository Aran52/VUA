import { describe, expect, it } from "vitest";
import type { TaskEventV01 } from "@vua/contracts";
import { TaskProgressCache, recentDownloadSpeed } from "./task-progress.ts";
import type { TaskItem } from "./task-port.ts";

const task: TaskItem = { id: "synthetic-transfer", title: "task", status: "running", originPage: "warehouse", cancellable: true };
const event = (revision: number, time: number, receivedBytes: number, expectedBytes?: number): TaskEventV01 => ({
  kind: "task.progressed", contractVersion: "0.1", eventId: `event-${revision}`, taskId: task.id, revision, occurredAt: new Date(time).toISOString(), correlationId: "synthetic", state: "running",
  payload: { completed: receivedBytes, messageKey: "tasks.download.transfer", params: { operation: "download.transfer", fileName: "synthetic.zip", receivedBytes, ...(expectedBytes === undefined ? {} : { expectedBytes }) } },
});
describe("real transfer presentation", () => {
  it("uses byte/time deltas, never infers progress from task state", () => {
    const cache = new TaskProgressCache();
    expect(cache.project(task).download).toBeUndefined();
    cache.observe(event(1, 1000, 100), 5000);
    expect(cache.project(task).download).toMatchObject({ receivedBytes: 100, expectedBytes: null, bytesPerSecond: null });
    cache.observe(event(2, 1500, 612, 2048), 5500);
    const view = cache.project(task);
    expect(view.title).toContain("synthetic.zip");
    expect(view.download).toMatchObject({ receivedBytes: 612, expectedBytes: 2048, bytesPerSecond: 1024 });
    expect(recentDownloadSpeed(view.download!, 8501)).toBeNull();
    expect(cache.project({ ...task, status: "completed" }).download!.bytesPerSecond).toBeNull();
    expect(cache.project({ ...task, errorText: "inspect_required" }).download!.bytesPerSecond).toBeNull();
  });
  it("ignores duplicate/reordered events and resets speed on rollback or gaps", () => {
    const cache = new TaskProgressCache();
    cache.observe(event(2, 1000, 500));
    cache.observe(event(1, 500, 20));
    expect(cache.project(task).download!.receivedBytes).toBe(500);
    cache.observe(event(3, 1500, 100));
    expect(cache.project(task).download!.bytesPerSecond).toBeNull();
    cache.observe(event(4, 20000, 900));
    expect(cache.project(task).download!.bytesPerSecond).toBeNull();
    cache.observe(event(5, 19000, 1000));
    expect(cache.project(task).download!.bytesPerSecond).toBeNull();
  });
  it("does not manufacture a size or speed for zero/unknown totals", () => {
    const cache = new TaskProgressCache();
    cache.observe(event(1, 1000, 100, 0));
    expect(cache.project(task).download!.expectedBytes).toBeNull();
  });
  it("labels stored-file progress separately from transferred bytes", () => {
    const cache = new TaskProgressCache();
    // The existing library worker checkpoint is an operation payload; consume it defensively.
    cache.observe({ ...event(1, 1000, 0), payload: { operation: "library.download", files: [{ phase: "downloaded" }, { phase: "stored" }, { phase: "failed" }] } } as unknown as TaskEventV01);
    expect(cache.project(task).progress).toEqual({ done: 1, total: 3 });
    expect(cache.project(task).status).toBe("running");
    expect(cache.project(task).progressUnit).toBe("files");
  });
});
