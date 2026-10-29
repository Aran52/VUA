/** AMF stations: a plain list of the four stations with their live facts. The code-drawn
 * base map, its camera flight and the production animation were removed with it (F1). */
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { Icon } from "@vua/design-system";
import { useGateway, type AmfProductionList, type AmfProductionRun } from "../../gateway/index.ts";
import type { PageId } from "../../app/nav-model.ts";
import { strings, format, formatShortDateTime } from "../../i18n/index.ts";
import { currentStation, lineBusy, lineLabels, lineStations, lineStatus, phaseStatus, type LineStatus } from "./factory-model.ts";
import "./factory.css";

const copy = strings.factory;
export type Room = "materials" | "workshop" | "control" | "release";
export const ROOM_PAGES: Record<Room, PageId> = { materials: "warehouse", workshop: "workshop", control: "recipe", release: "release" };
const ROOM_ICONS = { materials: "folder", workshop: "avatar", control: "edit", release: "anim" } as const;
const PRIORITY: readonly LineStatus[] = ["running", "waiting", "attention", "finished", "stopped", "idle"];
const lineName = (names: readonly string[]) => [...new Set(names.map(name => name.replace(/\.zip$/iu, "")))].join(" + ");

export function StationList({ navigate }: { navigate: (page: PageId) => void }) {
  const port = useGateway().amfProduction;
  const [list, setList] = useState<AmfProductionList | null>(null);
  const [recipes, setRecipes] = useState<number | null>(null);
  const [line, setLine] = useState<AmfProductionRun | null>(null);
  const statuses = useMemo(() => (list?.runs ?? []).map(run => ({ run, status: phaseStatus(run.phase) })), [list]);
  const lineId = useMemo(() => [...statuses].sort((a, b) => PRIORITY.indexOf(a.status) - PRIORITY.indexOf(b.status))[0]?.run.runId ?? null, [statuses]);
  const finished = statuses.filter(item => item.status === "finished");

  useEffect(() => {
    if (!port) return;
    let alive = true; let timer: ReturnType<typeof setTimeout> | undefined;
    const load = async () => {
      try {
        const [observed, saved] = await Promise.all([port.list(), port.listRecipes().catch(() => null)]);
        if (!alive) return;
        setList(observed); if (saved) setRecipes(saved.length);
      } catch { /* The stations keep their last facts. */ }
      if (alive) timer = setTimeout(() => { void load(); }, 4000);
    };
    void load();
    return () => { alive = false; if (timer !== undefined) clearTimeout(timer); };
  }, [port]);
  useEffect(() => {
    if (!port || !lineId) { setLine(null); return; }
    let alive = true;
    void port.get(lineId).then(run => { if (alive) setLine(run); }).catch(() => { if (alive) setLine(null); });
    return () => { alive = false; };
  }, [port, lineId, list]);

  // A reinspection keeps the run's previous phase; the busiest line's steps say what it is doing now.
  const count = (status: LineStatus) => statuses.filter(item => (item.run.runId === line?.runId ? lineStatus(line) : item.status) === status).length;
  const running = count("running"); const waiting = count("waiting"); const attention = count("attention");
  const stations = line ? lineStations(line) : null;
  const labels = lineLabels(list?.runs ?? [], date => formatShortDateTime(date));
  const label = (runId: string, fallback: string) => { const found = labels.get(runId); return found ? (found.extra ? `${found.name} · ${found.extra}` : found.name) : fallback; };

  const bodies: Record<Room, ReactNode> = {
    control: <span>{recipes === null ? "—" : format(copy.recipesCount, { count: recipes })}</span>,
    materials: <span>{list === null ? "—" : format(copy.materialsCount, { count: list.sources.length })}</span>,
    release: <><span>{format(copy.finishedCount, { count: finished.length })}</span>
      {finished[0] && <span className="vua-factory__muted">{format(copy.latestFinished, { name: label(finished[0].run.runId, lineName(finished[0].run.sourceNames)) })}</span>}</>,
    workshop: <>
      <span className="vua-factory__counts">
        {running > 0 && <span data-tone="running">{format(copy.linesRunning, { count: running })}</span>}
        {waiting > 0 && <span data-tone="waiting">{format(copy.linesWaiting, { count: waiting })}</span>}
        {attention > 0 && <span data-tone="attention">{format(copy.linesAttention, { count: attention })}</span>}
        {running + waiting + attention === 0 && <span>{copy.linesIdle}</span>}
      </span>
      {line && stations && <span className="vua-factory__line">
        <span className="vua-factory__line-name">{label(line.runId, lineName(line.sources.map(source => source.itemName)))}</span>
        <span className="vua-factory__station-now">{copy.stations[currentStation(stations)]} · {copy.stationState[stations[currentStation(stations)]]}</span>
      </span>}
    </>,
  };
  const rooms: readonly Room[] = ["materials", "control", "workshop", "release"];

  return <div className="vua-page vua-factory vua-stations" data-focus-scope>
    <header className="vua-factory__command">
      <h1 className="vua-title">{copy.mapTitle}</h1>
      <div className="vua-factory__alerts" role="status">
        {waiting > 0 && <button type="button" className="vua-factory__alert" data-tone="waiting" onClick={() => navigate("workshop")}><Icon name="bell" size={16}/>{format(copy.waitingBanner, { count: waiting })}</button>}
        {attention > 0 && <button type="button" className="vua-factory__alert" data-tone="attention" onClick={() => navigate("workshop")}><Icon name="warning" size={16}/>{format(copy.attentionBanner, { count: attention })}</button>}
        {waiting === 0 && attention === 0 && <span className="vua-factory__alert" data-tone="clear"><Icon name="check" size={16}/>{copy.allClear}</span>}
      </div>
    </header>
    <ul className="vua-stations__list">
      {rooms.map(id => <li key={id} className="vua-stations__item" data-room={id}>
        <span className="vua-factory__plate"><span className="vua-factory__tag">{copy.rooms[id].tag}</span><Icon name={ROOM_ICONS[id]} size={20}/></span>
        <div className="vua-stations__heading">
          <h2 className="vua-stations__name">{copy.rooms[id].name}</h2>
          <span className="vua-factory__hint">{copy.rooms[id].hint}</span>
        </div>
        <span className="vua-factory__body">{bodies[id]}</span>
        <button type="button" className="vua-factory__enter" data-nav-id={`enter-${id}`} onClick={() => navigate(ROOM_PAGES[id])}>
          {copy.enter}<Icon name="arrow-right" size={16}/>
        </button>
      </li>)}
    </ul>
  </div>;
}
