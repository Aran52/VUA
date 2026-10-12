import type { TaskEventV01 } from "@vua/contracts";
import { strings } from "../i18n/index.ts";
import type { TaskItem } from "./task-port.ts";

const record = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const bytes = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
interface Presentation { readonly revision: number; readonly eventTime: number; readonly fileName?: string; readonly download?: NonNullable<TaskItem["download"]>; readonly progress?: NonNullable<TaskItem["progress"]> }

/** UI cache only. Task state and finality always come from task.list. */
export class TaskProgressCache {
  private readonly items = new Map<string, Presentation>();
  observe(event: TaskEventV01, now = Date.now()): void {
    if (event.kind !== "task.progressed") return;
    const previous = this.items.get(event.taskId);
    if (previous !== undefined && event.revision <= previous.revision) return;
    const payload: unknown = event.payload;
    if (!record(payload)) return;
    if (payload.messageKey === "tasks.download.transfer" && record(payload.params) && payload.params.operation === "download.transfer") {
      const params = payload.params;
      const received = params.receivedBytes;
      if (!bytes(received)) return;
      const eventTime = Date.parse(event.occurredAt);
      const elapsed = previous?.download === undefined ? 0 : eventTime - previous.eventTime;
      const delta = previous?.download === undefined ? -1 : received - previous.download.receivedBytes;
      const speed = elapsed > 0 && elapsed <= 10_000 && delta >= 0 ? delta * 1000 / elapsed : null;
      this.items.set(event.taskId, { revision: event.revision, eventTime,
        ...(typeof params.fileName === "string" && params.fileName.length > 0 ? { fileName: params.fileName } : previous?.fileName === undefined ? {} : { fileName: previous.fileName }),
        download: { receivedBytes: received, expectedBytes: bytes(params.expectedBytes) && params.expectedBytes > 0 ? params.expectedBytes : null, bytesPerSecond: speed, sampledAt: now },
      });
    } else if (payload.operation === "library.download" && Array.isArray(payload.files) && payload.files.every(file => record(file) && typeof file.phase === "string")) {
      this.items.set(event.taskId, { revision: event.revision, eventTime: Date.parse(event.occurredAt),
        progress: { done: payload.files.filter(file => file.phase === "stored").length, total: payload.files.length },
      });
    }
  }
  project(task: TaskItem): TaskItem {
    const observed = this.items.get(task.id);
    if (observed === undefined) return task;
    return { ...task,
      ...(observed.fileName === undefined ? {} : { title: `${strings.taskCenter.downloadTask} · ${observed.fileName}` }),
      ...(observed.download === undefined ? {} : { download: { ...observed.download, bytesPerSecond: task.status === "running" && task.errorText !== "inspect_required" ? observed.download.bytesPerSecond : null } }),
      ...(observed.progress === undefined ? {} : { progress: observed.progress, progressUnit: "files" }),
    };
  }
}

export function recentDownloadSpeed(download: NonNullable<TaskItem["download"]>, now: number): number | null {
  return now - download.sampledAt <= 3000 && now >= download.sampledAt ? download.bytesPerSecond : null;
}
export function formatBytes(value: number): string {
  const units = ["B", "KiB", "MiB", "GiB"];
  const index = value < 1024 ? 0 : Math.min(3, Math.floor(Math.log(value) / Math.log(1024)));
  return `${new Intl.NumberFormat(undefined, { maximumFractionDigits: index === 0 ? 0 : 1 }).format(value / 1024 ** index)} ${units[index]}`;
}
