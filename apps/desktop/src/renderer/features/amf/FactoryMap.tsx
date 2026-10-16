/** AMF base map: the factory as a building cross-section; each room shows its live state. */
import { useEffect, useMemo, useState } from "react";
import { Icon } from "@vua/design-system";
import { useGateway, type AmfProductionList, type AmfProductionRun } from "../../gateway/index.ts";
import type { PageId } from "../../app/nav-model.ts";
import { strings, format } from "../../i18n/index.ts";
import { currentStation, lineStations, lineStatus, phaseStatus, STATIONS, type LineStatus } from "./factory-model.ts";
import "./factory.css";

const copy = strings.factory;
type Room = "materials" | "workshop" | "control" | "release";
export const ROOM_PAGES: Record<Room, PageId> = { materials: "warehouse", workshop: "workshop", control: "recipe", release: "release" };
const ROOM_ICONS = { materials: "folder", workshop: "avatar", control: "edit", release: "anim" } as const;
const PRIORITY: readonly LineStatus[] = ["running", "waiting", "attention", "finished", "stopped", "idle"];

export function FactoryMap({ navigate }: { navigate: (page: PageId) => void }) {
  const port = useGateway().amfProduction;
  const [list, setList] = useState<AmfProductionList | null>(null);
  const [recipes, setRecipes] = useState<number | null>(null);
  const [focus, setFocus] = useState<AmfProductionRun | null>(null);
  const statuses = useMemo(() => (list?.runs ?? []).map(run => ({ run, status: phaseStatus(run.phase) })), [list]);
  const focusId = useMemo(() => [...statuses].sort((a, b) => PRIORITY.indexOf(a.status) - PRIORITY.indexOf(b.status))[0]?.run.runId ?? null, [statuses]);

  useEffect(() => {
    if (!port) return;
    let alive = true; let timer: ReturnType<typeof setTimeout> | undefined;
    const load = async () => {
      try {
        const [observed, saved] = await Promise.all([port.list(), port.listRecipes().catch(() => null)]);
        if (!alive) return;
        setList(observed); if (saved) setRecipes(saved.length);
      } catch { /* The rooms keep their last facts. */ }
      if (alive) timer = setTimeout(() => { void load(); }, 4000);
    };
    void load();
    return () => { alive = false; if (timer !== undefined) clearTimeout(timer); };
  }, [port]);
  useEffect(() => {
    if (!port || !focusId) { setFocus(null); return; }
    let alive = true;
    void port.get(focusId).then(run => { if (alive) setFocus(run); }).catch(() => { if (alive) setFocus(null); });
    return () => { alive = false; };
  }, [port, focusId, list]);

  // A reinspection keeps the run's previous phase; the focused line's steps say what it is doing now.
  const count = (status: LineStatus) => statuses.filter(item => (item.run.runId === focus?.runId ? lineStatus(focus) : item.status) === status).length;
  const running = count("running"); const waiting = count("waiting"); const attention = count("attention");
  const finished = statuses.filter(item => item.status === "finished");
  const stations = focus ? lineStations(focus) : null;
  const room = (id: Room, body: React.ReactNode, wide = false) => <button type="button" className="vua-factory__room" data-room={id} data-wide={wide || undefined}
    data-nav-id={`room-${id}`} onClick={() => navigate(ROOM_PAGES[id])} aria-label={`${copy.rooms[id].name} · ${copy.enter}`}>
    <span className="vua-factory__plate"><span className="vua-factory__tag">{copy.rooms[id].tag}</span><Icon name={ROOM_ICONS[id]} size={20}/></span>
    <span className="vua-factory__name">{copy.rooms[id].name}</span>
    <span className="vua-factory__hint">{copy.rooms[id].hint}</span>
    <span className="vua-factory__body">{body}</span>
    <span className="vua-factory__enter">{copy.enter}<Icon name="arrow-right" size={16}/></span>
  </button>;

  return <div className="vua-page vua-factory" data-focus-scope>
    <header className="vua-factory__command">
      <div><h1 className="vua-title">{copy.mapTitle}</h1><p className="vua-text-secondary">{copy.mapIntro}</p></div>
      <div className="vua-factory__alerts" role="status">
        {waiting > 0 && <button type="button" className="vua-factory__alert" data-tone="waiting" onClick={() => navigate("workshop")}><Icon name="bell" size={16}/>{format(copy.waitingBanner, { count: waiting })}</button>}
        {attention > 0 && <button type="button" className="vua-factory__alert" data-tone="attention" onClick={() => navigate("workshop")}><Icon name="warning" size={16}/>{format(copy.attentionBanner, { count: attention })}</button>}
        {waiting === 0 && attention === 0 && <span className="vua-factory__alert" data-tone="clear"><Icon name="check" size={16}/>{copy.allClear}</span>}
      </div>
    </header>
    <div className="vua-factory__building">
      <div className="vua-factory__floor" data-floor="2">
        {room("control", <>
          <span>{recipes === null ? "—" : format(copy.recipesCount, { count: recipes })}</span>
        </>)}
        {room("materials", <span>{list === null ? "—" : format(copy.materialsCount, { count: list.sources.length })}</span>, true)}
      </div>
      <div className="vua-factory__floor" data-floor="1">
        {room("release", <>
          <span>{format(copy.finishedCount, { count: finished.length })}</span>
          {finished[0] && <span className="vua-factory__muted">{format(copy.latestFinished, { name: [...new Set(finished[0].run.sourceNames.map(name => name.replace(/\.zip$/iu, "")))].join(" + ") || finished[0].run.runId })}</span>}
        </>)}
        {room("workshop", <>
          <span className="vua-factory__counts">
            {running > 0 && <span data-tone="running">{format(copy.linesRunning, { count: running })}</span>}
            {waiting > 0 && <span data-tone="waiting">{format(copy.linesWaiting, { count: waiting })}</span>}
            {attention > 0 && <span data-tone="attention">{format(copy.linesAttention, { count: attention })}</span>}
            {running + waiting + attention === 0 && <span className="vua-factory__muted">{copy.linesIdle}</span>}
          </span>
          {focus && stations && <span className="vua-factory__line">
            <span className="vua-factory__line-name">{[...new Set(focus.sources.map(source => source.itemName.replace(/\.zip$/iu, "")))].join(" + ")}</span>
            <span className="vua-factory__stations" aria-label={copy.stations[currentStation(stations)]}>
              {STATIONS.map(key => <span key={key} data-state={stations[key]} data-current={currentStation(stations) === key || undefined}>{copy.stations[key]}</span>)}
            </span>
          </span>}
        </>, true)}
      </div>
    </div>
  </div>;
}
