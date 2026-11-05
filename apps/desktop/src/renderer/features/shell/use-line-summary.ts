/** What the production lines need from the player, for the directory's badge and for Home.
 * Lines change state through tasks and on the Workshop page. Reading the list also checks the installed
 * Editors, so it runs on a task change and when Home or the Workshop opens or closes; moving between
 * other pages reads nothing, and nothing polls while the app sits idle. */
import { useEffect, useMemo, useState } from "react";
import { useGateway, useTaskCenter, type AmfProductionList } from "../../gateway/index.ts";
import type { PageId } from "../../app/nav-model.ts";
import { phaseStatus, storedHiddenLines, type LineStatus } from "../amf/factory-model.ts";

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

export const emptyLineSummary: LineSummary = { loaded: false, materials: 0, hasEditor: true, recipes: null, lines: [], running: 0, waiting: 0, attention: 0, finished: 0 };

const lineName = (names: readonly string[]) => (names[0] ?? "").replace(/\.zip$/iu, "");

export function summarizeLines(list: Pick<AmfProductionList, "runs" | "sources" | "editors">, recipes: number | null, hidden: ReadonlySet<string>): LineSummary {
  const lines: SummaryLine[] = [...new Map(list.runs.map(run => [run.runId, run] as const)).values()]
    .filter(run => !hidden.has(run.runId))
    .map(run => ({ runId: run.runId, status: phaseStatus(run.phase), name: lineName(run.sourceNames) }));
  const count = (status: LineStatus) => lines.filter(line => line.status === status).length;
  return { loaded: true, materials: list.sources.length, hasEditor: list.editors.length > 0, recipes, lines,
    running: count("running"), waiting: count("waiting"), attention: count("attention"), finished: count("finished") };
}

export function useLineSummary(enabled: boolean, page: PageId): LineSummary {
  const port = useGateway().amfProduction;
  const taskKey = useTaskCenter().tasks.map(task => `${task.id}:${task.status}`).join(" ");
  const [list, setList] = useState<AmfProductionList | null>(null);
  const [recipes, setRecipes] = useState<number | null>(null);
  const readKey = page === "home" || page === "workshop" ? page : "elsewhere";
  useEffect(() => {
    if (!enabled || !port) { setList(null); setRecipes(null); return; }
    let alive = true;
    void port.list().then(value => { if (alive) setList(value); }, () => { /* The last facts stay shown. */ });
    void port.listRecipes().then(value => { if (alive) setRecipes(value.length); }, () => { /* Unknown stays unknown. */ });
    return () => { alive = false; };
  }, [enabled, port, taskKey, readKey]);
  return useMemo(() => list === null ? emptyLineSummary : summarizeLines(list, recipes, storedHiddenLines()), [list, recipes, page]);
}
