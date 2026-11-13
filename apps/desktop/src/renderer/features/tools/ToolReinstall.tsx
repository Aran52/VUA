/** Reinstalling a Steam application that stopped working: Steam asks to uninstall, then VUA installs it again. */
import { format, strings } from "../../i18n/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { useToolConnection } from "./use-tool-connection.ts";
import type { ExternalToolId } from "@vua/contracts";

type Live = ReturnType<typeof useToolConnection>;

export function ToolReinstall({ id, title, live }: { id: string; title: string; live: Live }) {
  const copy = strings.toolConnection;
  const s = live.snapshot;
  if (!s || !s.steamReady || s.presence === "missing") return null;
  if (s.activity === "reinstall_requested") return <div className="vua-tool-reinstall" data-tool-reinstall={id} data-state="waiting">
    <p role="status">{format(copy.reinstallWaiting, { tool: title })}</p>
    <Button variant="subtle" disabled={live.pending} onClick={() => { void live.act("cancel"); }}>{copy.cancelWait}</Button>
  </div>;
  if (s.activity !== "idle") return null;
  return <div className="vua-tool-reinstall" data-tool-reinstall={id}>
    <p>{format(copy.reinstallHint, { tool: title })}</p>
    <Button data-nav-id={`tool-reinstall-${id}`} disabled={live.pending || s.running} onClick={() => { void live.act("reinstall"); }}>{copy.reinstall}</Button>
  </div>;
}

/** The same action for an application that has no card of its own, such as VRChat under its play route. */
export function ToolReinstallRow({ toolId, title }: { toolId: ExternalToolId; title: string }) {
  const live = useToolConnection(toolId);
  const copy = strings.toolConnection;
  const issue = live.snapshot?.issue;
  return <div className="vua-tool-reinstall-row">
    <strong>{title}</strong>
    <ToolReinstall id={toolId} title={title} live={live} />
    {live.snapshot?.presence === "missing" ? <p>{format(copy.presence.missing, { tool: title })}</p> : null}
    {issue === "close_first" || issue === "reinstall_timeout" || issue === "handoff_failed" ? <p role="alert">{format(copy.issues[issue], { tool: title })}</p> : null}
  </div>;
}
