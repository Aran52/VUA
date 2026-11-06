import { useEffect, useRef, useState } from "react";
import { useGateway, useTaskCenter } from "../../gateway/index.ts";
import { libraryTermsCopy as copy } from "../../i18n/library-terms-copy.ts";
import { Button } from "../../components/primitives/Button.tsx";
const ended = new Set(["completed", "completedWithWarnings", "failed", "cancelled"]);
/** Explicit network work, kept out of tab entry and filter queries. */
export function LibraryTermsRead({ onUpdated, label }: { onUpdated(): void; label?: string }) {
  const port = useGateway().terms, { tasks } = useTaskCenter();
  const [taskId, setTaskId] = useState<string | null>(null), [starting, setStarting] = useState(false);
  const [notice, setNotice] = useState<"done" | "stopped" | "failed" | null>(null);
  const update = useRef(onUpdated); update.current = onUpdated;
  const task = tasks.find(item => item.id === taskId);
  const active = tasks.find(item => item.title === copy.read && !ended.has(item.status));
  useEffect(() => {
    if (!taskId || !task || !ended.has(task.status)) return;
    setTaskId(null); setNotice(task.status === "completed" ? "done" : "stopped"); update.current();
  }, [taskId, task]);
  if (!port?.refreshLibrary) return null;
  const busy = starting || taskId !== null || !!active;
  return <section className="vua-library-terms-read" aria-label={copy.read}>
    <Button disabled={busy} aria-busy={busy} onClick={() => {
      setStarting(true); setNotice(null);
      void port.refreshLibrary!(copy.read).then(value => setTaskId(value.taskId), () => setNotice("failed")).finally(() => setStarting(false));
    }}>{busy ? copy.reading : label ?? copy.read}</Button>
    {notice ? <p role={notice === "failed" ? "alert" : "status"} className="vua-caption">{copy[notice]}</p> : null}
  </section>;
}
