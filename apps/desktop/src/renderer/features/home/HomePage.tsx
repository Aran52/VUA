/** Home answers "what now": the work that is waiting, then the next step of each goal.
 * The sidebar stays the directory; nothing here repeats it. */
import { Icon, type IconName } from "@vua/design-system";
import type { PageId } from "../../app/nav-model.ts";
import { currentLocale, format, presentationCopy, strings, TERMS } from "../../i18n/index.ts";
import { playFlowCopy } from "../../i18n/play-flow-copy.ts";
import { structureCopy } from "../../i18n/structure-copy.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { useTaskCenter } from "../../gateway/index.ts";
import { requestNewProductionLine, requestProductionRun } from "../amf/production-recipe-navigation.ts";
import { directoryGroupLabel } from "../shell/directory.ts";
import type { LineSummary, SummaryLine } from "../shell/use-line-summary.ts";
import { isActiveStatus, taskRowOpenTarget } from "../task-center/notification-model.ts";
import { EnvironmentCard } from "./EnvironmentCard.tsx";
import { playCardDecision } from "./play-card-model.ts";
import { requestPlayRoute } from "./play-route-request.ts";
import { PC_SETUP, playDevice, readPlaySetup, setupEngine } from "./play-setup-model.ts";
import { setupTitle } from "./play-setup-text.ts";
import { usePlaySessions } from "./use-play-sessions.ts";

const journey = strings.journey;
const factory = strings.factory;
const structure = structureCopy[currentLocale];
const copy = structure.home;
const flow = playFlowCopy[currentLocale];
/** Rows the activity list shows before pointing at the workshop for the rest. */
const MAX_LINE_ROWS = 3;

/** The combination chosen on Launch, ready to start from here. */
function PlayRoutes({ navigate }: { navigate: (page: PageId) => void }) {
  const live = usePlaySessions(5000);
  const setup = readPlaySetup() ?? PC_SETUP;
  const engine = setupEngine(setup);
  const device = playDevice(setup.device);
  const kind = device.brand ? undefined : device.kind ?? "headset";
  if (engine.kind !== "route") {
    const open = () => navigate("env-play");
    return <div className="vua-environment-grid"><div className="vua-card-entry" data-card-entry="home-route-current">
      <EnvironmentCard title={setupTitle(setup)} detailsOpen={false} kind={kind} brand={device.brand} id="home-route-current" action="prepare" status={journey.launchPage} primary onDetails={open} onAction={open} />
    </div></div>;
  }
  const route = engine.route;
  const id = route === "desktop_play" ? "home-route-desktop" : "home-route-pico";
  const openRoute = () => { requestPlayRoute(route); navigate("env-play"); };
  const decision = playCardDecision(live.sessions[route] ?? null, !!live.unavailable[route], false, false);
  const canAct = decision.action === "stop" || decision.action === "start";
  const label = decision.action === "stop" ? flow.phases.stop
    : canAct ? flow.phases.launch
    : decision.spinning ? strings.environmentCards[decision.status]
    : copy.prepareRoute;
  return <div className="vua-environment-grid"><div className="vua-card-entry" data-card-entry={id}>
    <EnvironmentCard title={setupTitle(setup)} hint={route === "desktop_play" ? structure.routes.desktop : undefined} detailsOpen={false} kind={kind} brand={device.brand} id={id}
      action={canAct ? decision.action : decision.spinning ? "busy" : "prepare"} status={label} spinning={decision.spinning || !!live.pending[route]}
      disabled={decision.disabled || !!live.pending[route]} primary={!decision.spinning && !decision.disabled}
      onDetails={openRoute} onAction={() => { if (canAct) void live.act(route, decision.action === "stop"); else openRoute(); }} />
  </div></div>;
}

function StationTile({ id, icon, title, fact, onClick }: { id: string; icon: IconName; title: string; fact: string; onClick: () => void }) {
  return <button type="button" className="vua-home__station" data-nav-id={id} onClick={onClick}>
    <Icon name={icon} size={20} />
    <strong>{title}</strong>
    <span>{fact}</span>
  </button>;
}

export function HomePage({ navigate, amfInstalled = false, lines }: {
  navigate: (page: PageId) => void; amfInstalled?: boolean; lines: LineSummary;
}) {
  const tasks = useTaskCenter().tasks.filter(item => isActiveStatus(item.status));
  const needsYou = [...lines.lines.filter(line => line.status === "attention"), ...lines.lines.filter(line => line.status === "waiting")];
  const openLine = (line: SummaryLine) => { requestProductionRun(line.runId); navigate("workshop"); };
  const workshopFact = [
    lines.running > 0 ? format(factory.linesRunning, { count: lines.running }) : null,
    lines.waiting > 0 ? format(factory.linesWaiting, { count: lines.waiting }) : null,
    lines.attention > 0 ? format(factory.linesAttention, { count: lines.attention }) : null,
  ].filter(Boolean).join(" · ") || factory.linesIdle;
  const startLine = () => {
    if (lines.loaded && lines.materials === 0) { navigate("warehouse"); return; }
    requestNewProductionLine(); navigate("workshop");
  };
  return <div className="vua-page vua-home" data-focus-scope>
    <header className="vua-page__hero"><h1 className="vua-display">VUA</h1></header>
    {tasks.length + needsYou.length > 0 ? <section className="vua-home__activity" aria-label={presentationCopy.home.continue}>
      <h2>{presentationCopy.home.continue}</h2>
      {tasks.slice(0, MAX_LINE_ROWS).map(task => <button type="button" key={task.id} onClick={() => navigate(taskRowOpenTarget(task))}>
        <span className="vua-home__spinner" aria-hidden="true" /><strong>{task.title}</strong><span>{strings.taskStatus[task.status]}</span>
      </button>)}
      {needsYou.slice(0, MAX_LINE_ROWS).map(line => <button type="button" key={line.runId} data-line-status={line.status} data-nav-id={`home-line-${line.status}`} onClick={() => openLine(line)}>
        <Icon name={line.status === "attention" ? "warning" : "bell"} size={20} /><strong>{line.name}</strong>
        <span>{line.status === "attention" ? copy.attentionLine : copy.waitingLine} <em>{line.status === "attention" ? copy.fixLine : copy.confirmLine}</em></span>
      </button>)}
      {needsYou.length > MAX_LINE_ROWS ? <button type="button" onClick={() => navigate("workshop")}>
        <Icon name="arrow-right" size={20} /><strong>{format(copy.moreLines, { count: needsYou.length - MAX_LINE_ROWS })}</strong>
      </button> : null}
    </section> : null}
    <section className="vua-home__section vua-compact-environments">
      <h2>{directoryGroupLabel("env")}</h2>
      <PlayRoutes navigate={navigate} />
    </section>
    <section className="vua-home__section">
      <div className="vua-environment-heading">
        <h2>{directoryGroupLabel("production")}</h2>
        {amfInstalled ? <Button variant="primary" data-nav-id="home-new-line" onClick={startLine}>{lines.loaded && lines.materials === 0 ? copy.importMaterials : factory.newLine}</Button> : null}
      </div>
      {amfInstalled ? <div className="vua-home__stations">
        <StationTile id="home-warehouse" icon="folder" title={factory.rooms.materials.name} fact={lines.loaded ? format(factory.materialsCount, { count: lines.materials }) : copy.startHint} onClick={() => navigate("warehouse")} />
        <StationTile id="home-workshop" icon="avatar" title={factory.rooms.workshop.name} fact={lines.loaded ? workshopFact : "—"} onClick={() => navigate("workshop")} />
        <StationTile id="home-release" icon="anim" title={strings.nav.pages.release} fact={lines.loaded ? format(factory.finishedCount, { count: lines.finished }) : "—"} onClick={() => navigate("release")} />
        {lines.loaded && !lines.hasEditor ? <StationTile id="home-unity" icon="flask" title={structure.unitySetup} fact={structure.workshop.needUnity} onClick={() => navigate("env-create")} /> : null}
      </div> : <div className="vua-home__amf-off">
        <p>{copy.amfOff}</p>
        <Button variant="primary" data-nav-id="home-enable-amf" onClick={() => {
          navigate("env-create");
          window.requestAnimationFrame(() => document.querySelector(".vua-creator-page [data-amf-setup]")?.scrollIntoView({ block: "start" }));
        }}>{format(strings.amfModule.enable, { amf: TERMS.amf })}</Button>
      </div>}
    </section>
  </div>;
}
