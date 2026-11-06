import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { PLAY_ROUTES, type PlayRoute } from "@vua/contracts";
import { currentLocale, strings } from "../../i18n/index.ts";
import { playFlowCopy } from "../../i18n/play-flow-copy.ts";
import { structureCopy } from "../../i18n/structure-copy.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { SlideDrawer } from "../../components/SlideDrawer.tsx";
import { useGridColumns } from "../../components/use-grid-columns.ts";
import { RouteGlyph } from "../../components/RouteTile.tsx";
import { useEnvironmentView, useGateway, type EnvironmentView } from "../../gateway/index.ts";
import { CHECK_GROUPS } from "../deployer/deployer-model.ts";
import { EnvironmentCard, type EnvironmentAction } from "./EnvironmentCard.tsx";
import { takeRequestedPlayRoute } from "./play-route-request.ts";
import { usePlaySessions } from "./use-play-sessions.ts";
import { PlayRouteFlow, type PlayFlowStep } from "./PlayRouteFlow.tsx";
import { PlannedRouteFlow } from "./PlannedRouteFlow.tsx";
import { useEscapeClose } from "../../components/use-escape-close.ts";
import { PLANNED_ROUTES, type PlannedRouteId } from "./planned-routes.ts";
import { plannedRoutesCopy } from "../../i18n/planned-routes-copy.ts";
import { ToolGroup } from "../tools/ToolsHub.tsx";
import type { PlayFlowPhase } from "./play-flow-model.ts";
import "./play-flow.css";

const copy = strings.environmentCards;
const structure = structureCopy[currentLocale];
const OTHER_PLATFORMS = { icon: "anim", tools: [{ id: "chilloutvr", title: "ChilloutVR" }, { id: "reborn", title: "Reborn" }] } as const;
type RouteKey = PlayRoute | PlannedRouteId;
const ROUTE_KEYS: readonly RouteKey[] = [...PLAY_ROUTES, ...PLANNED_ROUTES.map(route => route.id)];
const cardId = (route: PlayRoute) => route === "desktop_play" ? "route-desktop" : "route-pico";
const runtimeBrand = (id: string) => id === "pico_runtime" ? "pico-mark" : id === "oculus_runtime" ? "meta" : id === "vive_runtime" ? "htcvive" : id === "steamvr" ? "valve" : undefined;
/** The card's glyph for a step; the other steps name themselves with an icon. */
const stepAction = (phase: PlayFlowPhase | undefined): EnvironmentAction =>
  phase === "launch" ? "start" : phase === "stop" ? "stop" : phase === "unavailable" ? "unknown" : phase === "check" || phase === "confirm" || phase === "execute" || phase === "handoff" || phase === "retry" ? "prepare" : "busy";
/** Steps the player takes next stand out; a running session's Close and work in progress stay quiet. */
const stepIsPrimary = (step: PlayFlowStep | undefined) => step !== undefined && !step.spinning && step.phase !== "stop" && step.phase !== "unavailable";

/** The device scan runs once per app session when Play first opens; the refresh button runs it again. */
let devicesScanned = false;
function useDeviceScan() {
  const gateway = useGateway(), view = useEnvironmentView();
  const [fresh, setFresh] = useState<EnvironmentView | null>(null);
  const [busy, setBusy] = useState(false);
  const run = useCallback(async () => {
    setBusy(true);
    try { setFresh(await gateway.environment.runCheck("play")); } catch { /* The last observed devices stay shown. */ }
    finally { setBusy(false); }
  }, [gateway]);
  useEffect(() => { if (!devicesScanned) { devicesScanned = true; void run(); } }, [run]);
  const phase = (fresh ?? view).deployer.zones.play;
  const evidence = phase.kind === "results" ? phase : phase.kind === "not-run" ? null : phase.last;
  const members: readonly string[] = CHECK_GROUPS[0].memberIds;
  const devices = evidence?.items.filter(item => members.includes(item.id) && item.status === "ok") ?? [];
  return { run, busy: busy || phase.kind === "running", scanned: evidence !== null, devices };
}

export function PlayPage({ onAccounts }: { onAccounts: () => void }) {
  const flowCopy = playFlowCopy[currentLocale];
  const [requestedRoute] = useState(takeRequestedPlayRoute);
  const [selected, setSelected] = useState<RouteKey | null>(requestedRoute);
  // The drawer keeps its place and content while it closes.
  const [shown, setShown] = useState<RouteKey>(requestedRoute ?? "desktop_play");
  const [steps, setSteps] = useState<Partial<Record<PlayRoute, PlayFlowStep>>>({});
  const [platform, setPlatform] = useState<string | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);
  const closeRoute = useCallback(() => setSelected(null), []);
  const closePlatform = useCallback(() => setPlatform(null), []);
  const live = usePlaySessions();
  const scan = useDeviceScan();
  const grid = useRef<HTMLDivElement>(null);
  const columns = useGridColumns(grid);
  const desktopStep = useCallback((step: PlayFlowStep) => setSteps(p => ({ ...p, desktop_play: step })), []);
  const picoStep = useCallback((step: PlayFlowStep) => setSteps(p => ({ ...p, pico_pcvr: step })), []);
  const title = (route: RouteKey) => route === "desktop_play" ? strings.journey.desktop : route === "pico_pcvr" ? "PICO" : PLANNED_ROUTES.find(item => item.id === route)!.title;
  const open = (route: RouteKey) => { setShown(route); setSelected(route); };
  const toggle = (route: RouteKey) => { setShown(route); setSelected(current => current === route ? null : route); };
  const detected = scan.devices.map(item => item.id);
  useEscapeClose(selected !== null, closeRoute);
  useEscapeClose(platform !== null, closePlatform);
  const next = (route: PlayRoute) => {
    const step = steps[route];
    if (!step || (step.needsDetails && selected !== route)) { open(route); return; }
    step.act();
  };
  // A failure or an official page to open is explained in the details, so they open with it.
  const desktopPhase = steps.desktop_play?.phase, picoPhase = steps.pico_pcvr?.phase;
  const desktopIssue = live.sessions.desktop_play?.issue, picoIssue = live.sessions.pico_pcvr?.issue;
  useEffect(() => { if (desktopPhase === "retry" || desktopPhase === "handoff" || desktopIssue) open("desktop_play"); }, [desktopPhase, desktopIssue]);
  useEffect(() => { if (picoPhase === "retry" || picoPhase === "handoff" || picoIssue) open("pico_pcvr"); }, [picoPhase, picoIssue]);
  const refresh = () => { void scan.run(); void live.refresh(); setRefreshToken(value => value + 1); };

  const drawer = <SlideDrawer key="play-drawer" open={selected !== null} className="vua-play-drawer" label={title(shown)}>
    {PLAY_ROUTES.map(route => <div key={route} id={`${cardId(route)}-details`} hidden={shown !== route}>
      <PlayRouteFlow route={route} session={live.sessions[route] ?? null} unavailable={!!live.unavailable[route]} launchPending={!!live.pending[route]}
        refreshToken={refreshToken} onLaunch={stop => { void live.act(route, stop); }} onAccounts={onAccounts} detected={detected}
        onStep={route === "desktop_play" ? desktopStep : picoStep} />
    </div>)}
    {PLANNED_ROUTES.map(route => <div key={route.id} id={`route-${route.id}-details`} hidden={shown !== route.id}>
      <PlannedRouteFlow id={route.id} connections={route.connections} detected={detected} />
    </div>)}
  </SlideDrawer>;
  const cards: ReactNode[] = [
    ...PLAY_ROUTES.map(route => {
      const step = steps[route];
      return <div className="vua-card-entry" data-card-entry={cardId(route)} key={route}>
        <EnvironmentCard title={title(route)} hint={route === "desktop_play" ? structure.routes.desktop : structure.routes.pico} kind={route === "desktop_play" ? "screen" : undefined} brand={route === "pico_pcvr" ? "pico-mark" : undefined} id={cardId(route)}
          action={stepAction(step?.phase)} actionIcon={step?.phase === "launch" || step?.phase === "stop" ? undefined : step?.icon} phase={step?.phase}
          status={step?.label ?? copy.checking} actionHint={step?.phase === "launch" || step?.phase === "stop" ? undefined : step?.label ?? copy.checking} spinning={step === undefined || step.spinning} disabled={step === undefined || step.disabled}
          primary={stepIsPrimary(step)} progress={step?.percent} warning={!!live.sessions[route]?.issue} selected={selected === route} onDetails={() => toggle(route)} onAction={() => next(route)} />
      </div>;
    }),
    ...PLANNED_ROUTES.map(route => <div className="vua-card-entry" data-card-entry={`route-${route.id}`} key={route.id}>
      <EnvironmentCard title={route.title} hint={plannedRoutesCopy[currentLocale].hints[route.id]} brand={route.brand} kind={route.brand ? undefined : "headset"} id={`route-${route.id}`}
        action="unknown" status="" developing actionHint={strings.journey.developingTip} selected={selected === route.id} onDetails={() => toggle(route.id)} />
    </div>),
  ];
  const drawerAfter = Math.min(cards.length - 1, (Math.floor(ROUTE_KEYS.indexOf(shown) / columns) + 1) * columns - 1);
  const items = cards.flatMap((card, index) => index === drawerAfter ? [card, drawer] : [card]);

  return <div className="vua-page vua-route-page vua-play-page vua-compact-environments" data-route-stage={selected ? "prepare" : "pick"}>
    <header className="vua-page__hero vua-environment-heading">
      <h1 className="vua-title">{strings.journey.play}</h1>
      <Button variant="subtle" data-nav-id="play-refresh-devices" disabled={scan.busy} aria-busy={scan.busy} onClick={refresh}>{scan.busy ? flowCopy.detecting : flowCopy.refreshDevices}</Button>
    </header>
    {scan.scanned ? <ul className="vua-play-devices" aria-label={copy.runtime}>
      {scan.devices.length > 0 ? scan.devices.map(item => <li key={item.id}><RouteGlyph brand={runtimeBrand(item.id)} kind="headset" />{item.title}</li>)
        : <li data-empty="true">{flowCopy.devicesNone}</li>}
    </ul> : null}
    <div className="vua-environment-grid" ref={grid} data-tour-anchor="play-environments">{items}</div>
    <div className="vua-other-platforms">
      <ToolGroup title={structure.platforms.title} group={OTHER_PLATFORMS} selected={platform} onSelect={setPlatform} onPrepareSteam={() => { setPlatform(null); open("desktop_play"); }} />
    </div>
  </div>;
}
