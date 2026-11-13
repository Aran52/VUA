/** The steps of a combination VUA guides: Steam and the platform through Steam, the connection through its
 * official pages, and each step's owner. The headset's own version needs no computer at all. */
import { useEffect, useRef, useState } from "react";
import type { ExternalToolId, PlayScenarioSopV02, PlaySetupCatalogV02 } from "@vua/contracts";
import { Icon } from "@vua/design-system";
import { openExternalUrl } from "../../app/open-external.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { useGateway } from "../../gateway/index.ts";
import { currentLocale, format, strings } from "../../i18n/index.ts";
import { playFlowCopy } from "../../i18n/play-flow-copy.ts";
import { playSetupCopy } from "../../i18n/play-setup-copy.ts";
import { ToolReinstall } from "../tools/ToolReinstall.tsx";
import { deriveToolCardState } from "../tools/tool-connection-state.ts";
import { useToolConnection } from "../tools/use-tool-connection.ts";
import type { PlayFlowStep } from "./PlayRouteFlow.tsx";
import { PLATFORM_NAMES, PLAY_CONNECTIONS, playDevice, setupSteps, type PlaySetup, type SetupStep } from "./play-setup-model.ts";
import { connectionWords } from "./play-setup-text.ts";

const STEAM_PAGE = "https://store.steampowered.com/about/";

let cachedCatalog: PlaySetupCatalogV02 | null = null;
let catalogRequested = false;
const sopCache = new Map<string, PlayScenarioSopV02>();

const setupKey = (setup: PlaySetup): string => `${setup.platform}:${setup.device}:${setup.connection ?? ""}`;

export function resolveSetupSteps(setup: PlaySetup, sop: PlayScenarioSopV02 | null): readonly SetupStep[] {
  return sop ? sop.steps.map(step => ({ id: step.id, actor: step.actor })) : setupSteps(setup);
}

function usePlayScenarioSop(setup: PlaySetup): { sop: PlayScenarioSopV02 | null; catalog: PlaySetupCatalogV02 | null } {
  const scenarios = useGateway().environment.scenarios;
  const key = setupKey(setup);
  const [catalog, setCatalog] = useState<PlaySetupCatalogV02 | null>(() => cachedCatalog);
  const [sop, setSop] = useState<PlayScenarioSopV02 | null>(() => sopCache.get(key) ?? null);

  useEffect(() => {
    if (!scenarios) return;
    let alive = true;
    if (!catalogRequested) {
      catalogRequested = true;
      void scenarios.playSetups().then(
        next => { cachedCatalog = next; if (alive) setCatalog(next); },
        () => { catalogRequested = false; },
      );
    } else if (cachedCatalog && catalog !== cachedCatalog) {
      setCatalog(cachedCatalog);
    }
    return () => { alive = false; };
  }, [scenarios, catalog]);

  useEffect(() => {
    const cached = sopCache.get(key) ?? null;
    setSop(cached);
    if (!scenarios) return;
    let alive = true;
    void scenarios.playSop({ platform: setup.platform, device: setup.device, connection: setup.connection }).then(
      next => { sopCache.set(key, next); if (alive) setSop(next); },
      () => { /* Synchronous fallback steps remain shown when provider SOP is unavailable. */ },
    );
    return () => { alive = false; };
  }, [scenarios, key, setup.platform, setup.device, setup.connection]);

  return { sop, catalog };
}

export function StepRows({ rows }: { rows: readonly { readonly id: string; readonly text: string; readonly actor: SetupStep["actor"]; readonly title?: string | undefined; readonly done?: boolean; readonly action?: { readonly label: string; readonly id: string; readonly onClick: () => void } | undefined }[] }) {
  const copy = playSetupCopy[currentLocale];
  return <ol className="vua-setup-steps">
    {rows.map((row, index) => <li key={row.id} data-step={row.id} title={row.title} data-done={row.done || undefined}>
      <span className="vua-setup-steps__mark" aria-hidden="true">{row.done ? <Icon name="check" size={16} /> : index + 1}</span>
      <span className="vua-setup-steps__text">{row.text}</span>
      {row.action ? <Button variant="subtle" data-nav-id={row.action.id} onClick={row.action.onClick}>{row.action.label}</Button> : null}
      <span className="vua-setup-steps__actor" data-actor={row.actor}>{copy.actors[row.actor]}</span>
    </li>)}
  </ol>;
}

/** Through Steam: the next step comes from the platform's state on this computer. */
export function GuidedFlow({ setup, toolId, detected, refreshToken = 0, onStep }: { setup: PlaySetup; toolId: ExternalToolId; detected: readonly string[]; refreshToken?: number; onStep: (step: PlayFlowStep) => void }) {
  const copy = playSetupCopy[currentLocale];
  const flow = playFlowCopy[currentLocale];
  const tools = strings.toolConnection;
  const name = PLATFORM_NAMES[setup.platform];
  const { sop, catalog } = usePlayScenarioSop(setup);
  const live = useToolConnection(toolId);
  const liveRefresh = live.refresh;
  useEffect(() => {
    if (refreshToken > 0) void liveRefresh();
  }, [refreshToken, liveRefresh]);
  const s = live.snapshot;
  const state = deriveToolCardState(live);
  const act = () => {
    if (!s || live.failed) { void live.refresh(); return; }
    if (s.canStop) void live.act("stop");
    else if (!s.steamReady) void openExternalUrl(STEAM_PAGE);
    else if (s.presence === "installed") void live.act("start");
    else if (s.presence === "unknown") void live.refresh();
    else void live.act("install");
  };
  const actRef = useRef(act); actRef.current = act;
  const phase = state.action === "start" ? "launch" as const : state.action === "stop" ? "stop" as const : state.action === "busy" ? "starting" as const : state.action === "unknown" ? "retry" as const : s && !s.steamReady ? "handoff" as const : "execute" as const;
  const label = phase === "execute" ? format(copy.steps.platform, { platform: name }) : phase === "handoff" ? tools.prepareSteam : phase === "retry" ? tools.recheck : phase === "starting" ? format(tools.short[state.status], { tool: name }) : flow.phases[phase];
  useEffect(() => {
    onStep({ phase, label, icon: phase === "execute" ? "download" : phase === "handoff" ? "arrow-right" : phase === "retry" ? "refresh" : undefined, spinning: phase === "starting", percent: null, needsDetails: false, disabled: phase === "starting", act: () => actRef.current() });
  }, [phase, label, onStep]);
  const connection = setup.connection === null ? null : PLAY_CONNECTIONS[setup.connection];
  const catalogConnection = setup.connection === null ? undefined : catalog?.connections.find(item => item.id === setup.connection);
  const connectionUrl = catalogConnection?.url ?? connection?.url;
  const text = setup.connection === null ? null : connectionWords(setup.connection);
  const open = connection === null || !connectionUrl ? undefined : { label: copy.official, id: `play-official-${connection.id}`, onClick: () => { void openExternalUrl(connectionUrl); } };
  const rows = resolveSetupSteps(setup, sop).map(step => {
    switch (step.id) {
      case "steam": return { ...step, text: copy.steps.steam, done: s?.steamReady === true, action: s && !s.steamReady ? { label: copy.official, id: "play-official-steam", onClick: () => { void openExternalUrl(STEAM_PAGE); } } : undefined };
      case "platform": return { ...step, text: format(copy.steps.platform, { platform: name }), done: s?.presence === "installed" };
      case "launch": return { ...step, text: format(copy.steps.launch, { platform: name }), done: false };
      case "connection-pc": return { ...step, text: text?.pc ?? connection?.name ?? "", done: connection?.component != null && detected.includes(connection.component), action: open };
      case "connection-headset": return { ...step, text: text?.headset ?? "" };
      default: return { ...step, text: copy.steps[step.id as "steam-sign-in"] };
    }
  });
  const sourceNote = sop ? format(copy.verifiedSource, { date: sop.lastVerification.date }) : copy.officialSource;
  return <div className="vua-play-flow" data-guided={toolId}>
    <StepRows rows={rows} />
    {s?.issue ? <p className="vua-play-flow__notice" role="alert" data-tone="warning">{format(tools.issues[s.issue], { tool: name })}</p> : null}
    <div className="vua-setup-foot">
      <small className="vua-text-secondary" data-sop-verified={sop?.lastVerification.date}>{sourceNote}</small>
      {s && s.presence !== "missing" ? <details className="vua-play-flow__repair"><summary>{flow.repair}</summary><ToolReinstall id={toolId} title={name} live={live} /></details> : null}
    </div>
  </div>;
}

/** The headset's own VRChat: nothing runs on this computer. */
export function HeadsetFlow({ setup }: { setup: PlaySetup }) {
  const copy = playSetupCopy[currentLocale];
  const { sop, catalog } = usePlayScenarioSop(setup);
  const store = playDevice(setup.device).store;
  const rows = resolveSetupSteps(setup, sop).map(step => ({ ...step, text: step.id === "store-install" ? format(copy.steps["store-install"], { store: store ? copy.stores[store] : "" }) : copy.steps["store-launch"] }));
  const standaloneUrl = sop?.sources[0]?.url ?? catalog?.connections.find(item => item.id === "standalone")?.url ?? PLAY_CONNECTIONS.standalone.url;
  const sourceNote = sop ? format(copy.verifiedSource, { date: sop.lastVerification.date }) : copy.officialSource;
  return <div className="vua-play-flow" data-guided="headset">
    <StepRows rows={rows} />
    <p className="vua-play-flow__notice">{copy.standaloneNote}</p>
    <div className="vua-setup-foot">
      <small className="vua-text-secondary" data-sop-verified={sop?.lastVerification.date}>{sourceNote}</small>
      <Button variant="subtle" data-nav-id="play-official-standalone" onClick={() => { void openExternalUrl(standaloneUrl); }}>{copy.official}</Button>
    </div>
  </div>;
}
