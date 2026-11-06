/** What the production lines need from the player, for the directory's badge and for Home.
 * Lines change state through tasks, which read the list again, and on the Workshop page, which reads it itself.
 * Opening Home or leaving the Workshop reads only when the facts are old. The page just opened goes first, since a
 * list read holds up the module's other answers, and nothing polls while the app sits idle. */
import { useEffect, useMemo, useRef, useState } from "react";
import { useGateway, useTaskCenter, type AmfProductionList } from "../../gateway/index.ts";
import type { PageId } from "../../app/nav-model.ts";
import { phaseStatus, storedHiddenLines, type LineStatus } from "../amf/factory-model.ts";
import { PRODUCTION_LIST_FRESH_MS, readProductionList } from "../amf/production-list-read.ts";
import { lineName } from "../amf/line-presentation.ts";

export interface SummaryLine { readonly runId: string; readonly status: LineStatus; readonly name: string }
export interface LineSummary {
  readonly loaded: boolean;
  readonly materials: number;
  readonly hasEditor: boolean;
  readonly recipes: number | null;
  readonly lines: readonly SummaryLine[];
  readonly running: number;
  readonly waiting: number;
  readonly attention: number;
  readonly finished: number;
}

const PAGE_FIRST_MS = 1500;

export const emptyLineSummary: LineSummary = { loaded: false, materials: 0, hasEditor: true, recipes: null, lines: [], running: 0, waiting: 0, attention: 0, finished: 0 };


export function summarizeLines(list: Pick<AmfProductionList, "runs" | "sources" | "editors">, recipes: number | null, hidden: ReadonlySet<string>): LineSummary {
  const lines: SummaryLine[] = [...new Map(list.runs.map(run => [run.runId, run] as const)).values()]
    .filter(run => !hidden.has(run.runId))
    .map(run => ({ runId: run.runId, status: phaseStatus(run.phase), name: lineName(run.runId, run.sourceNames[0] ?? "") }));
  const count = (status: LineStatus) => lines.filter(line => line.status === status).length;
  return { loaded: true, materials: list.sources.length, hasEditor: list.editors.length > 0, recipes, lines,
    running: count("running"), waiting: count("waiting"), attention: count("attention"), finished: count("finished") };
}

export function useLineSummary(enabled: boolean, page: PageId): LineSummary {
  const port = useGateway().amfProduction;
  const taskKey = useTaskCenter().tasks.map(task => `${task.id}:${task.status}`).join(" ");
  const [list, setList] = useState<AmfProductionList | null>(null);
  const [recipes, setRecipes] = useState<number | null>(null);
  const seen = useRef<{ readonly page: PageId; readonly taskKey: string } | null>(null);
  const wanted = useRef<number | null>(null);
  const newest = useRef(0);
  useEffect(() => {
    if (!enabled || !port) { setList(null); setRecipes(null); seen.current = null; wanted.current = null; newest.current += 1; return; }
    const before = seen.current;
    seen.current = { page, taskKey };
    if (before === null || before.taskKey !== taskKey) wanted.current = 0;
    else if (before.page !== page && (page === "home" || before.page === "workshop")) wanted.current ??= PRODUCTION_LIST_FRESH_MS;
    if (wanted.current === null) return;
    const read = () => {
      const maxAge = wanted.current ?? 0, mine = ++newest.current;
      wanted.current = null;
      void readProductionList(port, maxAge).then(value => { if (newest.current === mine) setList(value); }, () => { /* The last facts stay shown. */ });
      void port.listRecipes().then(value => { if (newest.current === mine) setRecipes(value.length); }, () => { /* Unknown stays unknown. */ });
    };
    if (before === null) { read(); return; }
    const timer = setTimeout(read, PAGE_FIRST_MS);
    return () => clearTimeout(timer);
  }, [enabled, port, taskKey, page]);
  return useMemo(() => list === null ? emptyLineSummary : summarizeLines(list, recipes, storedHiddenLines()), [list, recipes, page]);
}
