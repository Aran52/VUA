/** The steps of a combination VUA guides: Steam and the platform through Steam, the connection through its
 * official pages, and each step's owner. The headset's own version needs no computer at all. */
import { useEffect, useRef } from "react";
import type { ExternalToolId } from "@vua/contracts";
import { Icon } from "@vua/design-system";
import { openExternalUrl } from "../../app/open-external.ts";
import { Button } from "../../components/primitives/Button.tsx";
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
export function GuidedFlow({ setup, toolId, detected, onStep }: { setup: PlaySetup; toolId: ExternalToolId; detected: readonly string[]; onStep: (step: PlayFlowStep) => void }) {
  const copy = playSetupCopy[currentLocale];
  const flow = playFlowCopy[currentLocale];
  const tools = strings.toolConnection;
  const name = PLATFORM_NAMES[setup.platform];
  const live = useToolConnection(toolId);
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
  const label = phase === "handoff" ? tools.prepareSteam : phase === "retry" ? tools.recheck : phase === "starting" ? format(tools.short[state.status], { tool: name }) : flow.phases[phase];
  useEffect(() => {
    onStep({ phase, label, icon: phase === "execute" ? "download" : phase === "handoff" ? "arrow-right" : phase === "retry" ? "refresh" : undefined, spinning: phase === "starting", percent: null, needsDetails: false, disabled: phase === "starting", act: () => actRef.current() });
  }, [phase, label, onStep]);
  const connection = setup.connection === null ? null : PLAY_CONNECTIONS[setup.connection];
  const text = setup.connection === null ? null : connectionWords(setup.connection);
  const open = connection === null ? undefined : { label: copy.official, id: `play-official-${connection.id}`, onClick: () => { void openExternalUrl(connection.url); } };
  const rows = setupSteps(setup).map(step => {
    switch (step.id) {
      case "steam": return { ...step, text: copy.steps.steam, done: s?.steamReady === true, action: s && !s.steamReady ? { label: copy.official, id: "play-official-steam", onClick: () => { void openExternalUrl(STEAM_PAGE); } } : undefined };
      case "platform": return { ...step, text: format(copy.steps.platform, { platform: name }), done: s?.presence === "installed" };
      case "launch": return { ...step, text: format(copy.steps.launch, { platform: name }), done: false };
      case "connection-pc": return { ...step, text: text?.pc ?? connection?.name ?? "", done: connection?.component != null && detected.includes(connection.component), action: open };
      case "connection-headset": return { ...step, text: text?.headset ?? "" };
      default: return { ...step, text: copy.steps[step.id as "steam-sign-in"] };
    }
  });
  return <div className="vua-play-flow" data-guided={toolId}>
    <StepRows rows={rows} />
    {s?.issue ? <p className="vua-play-flow__notice" role="alert" data-tone="warning">{format(tools.issues[s.issue], { tool: name })}</p> : null}
    <div className="vua-setup-foot">
      <small className="vua-text-secondary">{copy.officialSource}</small>
      {s && s.presence !== "missing" ? <details className="vua-play-flow__repair"><summary>{flow.repair}</summary><ToolReinstall id={toolId} title={name} live={live} /></details> : null}
    </div>
  </div>;
}

/** The headset's own VRChat: nothing runs on this computer. */
export function HeadsetFlow({ setup }: { setup: PlaySetup }) {
  const copy = playSetupCopy[currentLocale];
  const store = playDevice(setup.device).store;
  const rows = setupSteps(setup).map(step => ({ ...step, text: step.id === "store-install" ? format(copy.steps["store-install"], { store: store ? copy.stores[store] : "" }) : copy.steps["store-launch"] }));
  return <div className="vua-play-flow" data-guided="headset">
    <StepRows rows={rows} />
    <p className="vua-play-flow__notice">{copy.standaloneNote}</p>
    <div className="vua-setup-foot">
      <small className="vua-text-secondary">{copy.officialSource}</small>
      <Button variant="subtle" data-nav-id="play-official-standalone" onClick={() => { void openExternalUrl(PLAY_CONNECTIONS.standalone.url); }}>{copy.official}</Button>
    </div>
  </div>;
}
