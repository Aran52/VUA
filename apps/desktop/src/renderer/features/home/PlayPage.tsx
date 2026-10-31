import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { PLAY_ROUTES, type PlayRoute } from "@vua/contracts";
import { currentLocale, strings } from "../../i18n/index.ts";
import { playFlowCopy } from "../../i18n/play-flow-copy.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { Tooltip } from "../../components/primitives/Tooltip.tsx";
import { SlideDrawer } from "../../components/SlideDrawer.tsx";
import { useGridColumns } from "../../components/use-grid-columns.ts";
import { RouteGlyph } from "../../components/RouteTile.tsx";
import { useEnvironmentView, useGateway, type EnvironmentView } from "../../gateway/index.ts";
import { CHECK_GROUPS } from "../deployer/deployer-model.ts";
import type { DeploymentCardState } from "../deployer/DeploymentPanel.tsx";
import { NetworkPanel } from "../deployer/NetworkPanel.tsx";
import { EnvironmentCard } from "./EnvironmentCard.tsx";
import { playCardDecision } from "./play-card-model.ts";
import { usePlaySessions } from "./use-play-sessions.ts";
import { PlayRouteFlow } from "./PlayRouteFlow.tsx";
import "./play-flow.css";

const copy = strings.environmentCards;
const developingRoutes = [
  { id: "quest", title: "Meta Quest", brand: "meta" },
  { id: "vive", title: "HTC VIVE", brand: "htcvive" },
  { id: "index", title: "Valve Index", brand: "valve" },
] as const;
const cardId = (route: PlayRoute) => route === "desktop_play" ? "route-desktop" : "route-pico";
const runtimeBrand = (id: string) => id === "pico_runtime" ? "pico" : id === "oculus_runtime" ? "meta" : id === "vive_runtime" ? "htcvive" : id === "steamvr" ? "valve" : undefined;

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
  const [selected, setSelected] = useState<PlayRoute | null>(null);
  // The drawer keeps its place and content while it closes.
  const [shown, setShown] = useState<PlayRoute>("desktop_play");
  const [deployment, setDeployment] = useState<Partial<Record<PlayRoute, DeploymentCardState>>>({});
  const [refreshToken, setRefreshToken] = useState(0);
  const live = usePlaySessions();
  const scan = useDeviceScan();
  const grid = useRef<HTMLDivElement>(null);
  const columns = useGridColumns(grid);
  const desktopState = useCallback((s: DeploymentCardState) => setDeployment(p => ({ ...p, desktop_play: s })), []);
  const picoState = useCallback((s: DeploymentCardState) => setDeployment(p => ({ ...p, pico_pcvr: s })), []);
  const title = (route: PlayRoute) => route === "desktop_play" ? strings.journey.desktop : "PICO";
  const open = (route: PlayRoute) => { setShown(route); setSelected(route); };
  const toggle = (route: PlayRoute) => { setShown(route); setSelected(current => current === route ? null : route); };
  const decide = (route: PlayRoute) => playCardDecision(live.sessions[route] ?? null, !!live.unavailable[route], !!deployment[route]?.busy, !!deployment[route]?.error);
  const right = (route: PlayRoute) => {
    const decision = decide(route);
    if (decision.action === "start" || decision.action === "stop") void live.act(route, decision.action === "stop");
    else open(route);
  };
  const refresh = () => { void scan.run(); void live.refresh(); setRefreshToken(value => value + 1); };

  const drawer = <SlideDrawer key="play-drawer" open={selected !== null} className="vua-play-drawer" label={title(shown)}>
    {PLAY_ROUTES.map(route => <div key={route} id={`${cardId(route)}-details`} hidden={shown !== route}>
      <PlayRouteFlow route={route} session={live.sessions[route] ?? null} unavailable={!!live.unavailable[route]} launchPending={!!live.pending[route]}
        refreshToken={refreshToken} onLaunch={stop => { void live.act(route, stop); }} onAccounts={onAccounts}
        onStateChange={route === "desktop_play" ? desktopState : picoState} />
    </div>)}
  </SlideDrawer>;
  const cards: ReactNode[] = [
    ...PLAY_ROUTES.map(route => {
      const decision = decide(route);
      return <div className="vua-card-entry" data-card-entry={cardId(route)} key={route}>
        <EnvironmentCard title={title(route)} kind={route === "desktop_play" ? "screen" : undefined} brand={route === "pico_pcvr" ? "pico" : undefined} id={cardId(route)}
          {...decision} status={copy[decision.status]} spinning={decision.spinning || !!live.pending[route]} disabled={decision.disabled || !!live.pending[route]}
          selected={selected === route} onDetails={() => toggle(route)} onAction={() => right(route)} />
      </div>;
    }),
    ...developingRoutes.map(route => <div className="vua-card-entry" data-card-entry={`route-${route.id}`} key={route.id}>
      <Tooltip block content={strings.journey.developingTip}><EnvironmentCard title={route.title} brand={route.brand} id={`route-${route.id}`} action="unknown" status="" developing /></Tooltip>
    </div>),
  ];
  const drawerAfter = Math.min(cards.length - 1, (Math.floor(PLAY_ROUTES.indexOf(shown) / columns) + 1) * columns - 1);
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
    <NetworkPanel compact />
    <div className="vua-environment-grid" ref={grid} data-tour-anchor="play-environments">{items}</div>
  </div>;
}
