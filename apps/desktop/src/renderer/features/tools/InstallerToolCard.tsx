import type { IconName } from "@vua/design-system";
import { OFFICIAL_TOOL_URLS, type InstallerToolId } from "@vua/contracts";
import { openExternalUrl } from "../../app/open-external.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { format, strings } from "../../i18n/index.ts";
import { EnvironmentCard } from "../home/EnvironmentCard.tsx";
import { ToolDetails } from "./ToolDetails.tsx";
import { ToolLogo } from "./ToolLogo.tsx";
import {
  deriveInstallerCardState,
  describeInstallerError,
  describeInstallerStage,
  useInstallerTool,
} from "./use-installer-tool.ts";
import "./tools.css";

/** Official-installer tool card (SlimeVR, VRCT, VRCX, VRCS): launch/observe via installer-tool v0.1,
 * silent official package acquisition and NSIS setup via environment-deployment v0.1. */
export function InstallerToolCard({ toolId, title, hint, icon, selected, onDetails, onReveal }: {
  toolId: InstallerToolId;
  title: string;
  hint?: string | undefined;
  icon: IconName;
  selected: boolean;
  onDetails(): void;
  onReveal(): void;
}) {
  const copy = strings.installerTool;
  const tc = strings.toolConnection;
  const live = useInstallerTool(toolId);
  const s = live.snapshot;
  const waiting = !!s && s.activity !== "idle";
  const state = deriveInstallerCardState(live, copy, tc);
  const text = (template: string) => format(template, { tool: title });
  const stageText = describeInstallerStage(live.step, copy);

  const actionHint = live.installing
    ? stageText
    : state.busy
      ? s?.activity === "stopping" ? tc.waitClose : s?.activity === "starting" ? tc.waitOpen : tc.checking
      : !s || live.failed || !live.available
        ? tc.recheck
        : s.canStop
          ? text(tc.closeApp)
          : s.presence === "installed"
            ? text(tc.openApp)
            : s.presence === "unknown"
              ? tc.recheck
              : copy.install;

  const act = () => {
    onReveal();
    if (live.installing) return;
    if (!s || live.failed || !live.available) { void live.refresh(); return; }
    if (s.canStop) void live.act("stop");
    else if (s.presence === "installed") void live.act("start");
    else if (s.presence === "unknown") void live.refresh();
    else void live.install(false);
  };

  const officialUrl = OFFICIAL_TOOL_URLS[toolId];

  return <>
    <EnvironmentCard
      title={title}
      icon={icon}
      logo={<ToolLogo id={toolId} icon={icon} />}
      id={`tool-${toolId}`}
      hint={hint}
      selected={selected}
      action={state.action}
      status={state.status}
      actionHint={actionHint}
      {...(state.actionIcon !== undefined ? { actionIcon: state.actionIcon } : {})}
      progress={state.percent}
      onDetails={onDetails}
      onAction={act}
      disabled={state.busy}
      spinning={state.busy}
      warning={!!s?.issue && s.canStop}
    />
    <ToolDetails id={toolId} title={title} selected={selected}>
      <header className="vua-environment-heading">
        <h2>{title}</h2>
        <Button variant="subtle" onClick={onDetails}>{strings.environmentCards.closeDetails}</Button>
      </header>
      <p>{strings.helpUi[toolId]}</p>
      <p className="vua-tool-source">{copy.badge}</p>
      <div className="vua-tool-connection-actions">
        <Button variant="subtle" disabled={live.installing || live.pending} onClick={() => { void live.refresh(); }}>{tc.recheck}</Button>
        <Button variant="subtle" data-nav-id={`tool-official-${toolId}`} onClick={() => { void openExternalUrl(officialUrl); }}>{copy.officialDownload}</Button>
      </div>
      {live.failed || !live.available ? <p role="status">{text(tc.unknown)}</p> : s ? <>
        <p role="status" data-tool-presence={s.presence}>
          {s.presence === "installed"
            ? (s.version ? format(copy.versionLine, { version: s.version }) : copy.badge)
            : s.presence === "unknown"
              ? tc.unknown
              : copy.notInstalled}
          {s.running ? " · " + tc.running : ""}
        </p>
        {s.issue && s.issue !== "not_installed" ? <p role="alert">{text(tc.issues[s.issue])}</p> : null}
        {live.installing ? <div className="vua-tool-connection-actions">
          <p role="status">{stageText}{state.percent !== null ? ` (${state.percent}%)` : ""}</p>
          {live.canCancelInstall ? <Button variant="subtle" disabled={live.cancelPending} onClick={() => { void live.cancelInstall(); }}>{live.cancelling ? copy.cancelling : copy.cancel}</Button> : null}
        </div> : null}
        {live.installError && !live.installing ? <p role="alert">{describeInstallerError(live.installError, copy)}</p> : null}
        {(s.presence === "missing" || s.presence === "incomplete") && !live.installing && !waiting ? (
          <Button variant="primary" data-nav-id={`tool-install-${toolId}`} disabled={live.pending} onClick={() => { void live.install(false); }}>{copy.install}</Button>
        ) : null}
        {s.presence === "installed" && !live.installing ? (
          <Button disabled={live.pending || waiting} onClick={() => { void live.act(s.canStop ? "stop" : "start"); }}>{s.canStop ? text(tc.closeApp) : text(tc.openApp)}</Button>
        ) : null}
        {waiting && s.activity === "starting" ? <div>
          <p>{tc.waitOpen}</p>
          <Button variant="subtle" disabled={live.pending} onClick={() => { void live.act("cancel"); }}>{tc.cancelWait}</Button>
        </div> : null}
        {s.running && !s.canStop ? <p className="vua-text-secondary">{text(tc.existing)}</p> : null}
        {s.presence === "installed" && !live.installing && s.activity === "idle" && live.canDeploy ? (
          <div className="vua-tool-reinstall" data-tool-reinstall={toolId}>
            <p>{s.version ? format(copy.versionLine, { version: s.version }) : copy.badge}</p>
            <Button data-nav-id={`tool-reinstall-${toolId}`} disabled={live.pending || s.running} onClick={() => { void live.install(true); }}>{copy.reinstall}</Button>
          </div>
        ) : null}
      </> : <p role="status">{text(tc.checking)}</p>}
    </ToolDetails>
  </>;
}
