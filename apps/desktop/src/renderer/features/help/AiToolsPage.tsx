import { useEffect, useRef, useState } from "react";
import type { AiActionRequestV01, AiToolsSnapshotV01 } from "@vua/contracts";
import { useGateway } from "../../gateway/index.ts";
import { currentLocale, strings } from "../../i18n/index.ts";
import { Card } from "../../components/primitives/Card.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import "./ai-tools.css";

const copy = strings.aiTools;
const record = (value: unknown): value is Record<string, unknown> => !!value && typeof value === "object" && !Array.isArray(value);
const title = (request: AiActionRequestV01) => copy.tools[request.tool as keyof typeof copy.tools] ?? request.title;
function outcome(request: AiActionRequestV01): string {
  const keys: Record<string, keyof typeof copy.outcomes> = { vua_request_installation: "installation", vua_request_play: "play", vua_request_tool: "tool", vua_request_amf_prepare: "prepare", vua_request_amf_assemble: "assemble", vua_request_amf_reinspect: "reinspect", vua_request_amf_open: "open", vua_request_cancel_task: "cancel" };
  return copy.outcomes[keys[request.tool] ?? "cancel"];
}
/** Facts displayed for consent, not executable text supplied by the Agent. */
function Review({ request }: { request: AiActionRequestV01 }) {
  const params = record(request.details.parameters) ? request.details.parameters : {};
  const reviewed = record(request.details.reviewed) ? request.details.reviewed : {};
  const plan = record(reviewed.deploymentPlan) ? reviewed.deploymentPlan : null;
  const sources = Array.isArray(params.sources) ? params.sources.filter(record) : [];
  const knownSources = Array.isArray(reviewed.sources) ? reviewed.sources.filter(record) : [];
  const inputs = Array.isArray(reviewed.inputs) ? reviewed.inputs.filter(record) : [];
  const selection = record(params.selection) ? params.selection : null;
  const outfits = selection && Array.isArray(selection.outfits) ? selection.outfits.filter(record) : [];
  return <div className="vua-ai-review">
    {plan && Array.isArray(plan.steps) ? <ol>{plan.steps.filter(record).map((step, i) => <li key={i}>
      <strong>{strings.deployment.components[step.component as keyof typeof strings.deployment.components] ?? String(step.component)}</strong>
      {" · "}{strings.deployment.actions[step.action as keyof typeof strings.deployment.actions] ?? String(step.action)}
      {step.version ? ` (${String(step.version)})` : ""}
      {step.location ? <div><code>{String(step.location)}</code></div> : null}
    </li>)}</ol> : null}
    {plan && record(plan.downloadPolicy) ? <p className="vua-caption">
      {strings.deployment.downloadRegion}: {strings.deployment.downloadRegions[plan.downloadPolicy.region as keyof typeof strings.deployment.downloadRegions]}
      <br />{strings.deployment.downloadOrder}: {Array.isArray(plan.downloadPolicy.sources) ? plan.downloadPolicy.sources.map(source => strings.deployment.downloadSources[source as keyof typeof strings.deployment.downloadSources]).join(" → ") : ""}
      {plan.downloadPolicy.editorEditions ? <><br />{strings.deployment.editorEditionOrder}</> : null}
    </p> : null}
    {params.route ? <p>{strings.deployment.purposes[params.route as keyof typeof strings.deployment.purposes] ?? String(params.route)}</p> : null}
    {params.action ? <p>{copy.actions[params.action as keyof typeof copy.actions] ?? String(params.action)}</p> : null}
    {params.editorPath ? <p>{strings.amfProduction.editor}: <code>{String(params.editorPath)}</code></p> : null}
    {sources.length > 0 ? <ul>{sources.map((source, i) => {
      const fact = knownSources.find(s => s.sourceId === source.sourceId);
      return <li key={i}>{fact ? `${String(fact.itemName)} · ${String(fact.name)}` : String(source.sourceId)}</li>;
    })}</ul> : null}
    {inputs.length > 0 ? <ul>{inputs.map((input, i) => <li key={i}>{String(input.name)}</li>)}</ul> : null}
    {params.runId && knownSources.length > 0 ? <ul>{knownSources.map((source, i) => <li key={i}>{String(source.itemName)} · {String(source.name)}</li>)}</ul> : null}
    {reviewed.projectRoot ? <p><code>{String(reviewed.projectRoot)}</code></p> : null}
    {selection ? <p>{strings.amfProduction.avatar}: <code>{String(selection.avatarPrefab)}</code></p> : null}
    {outfits.length > 0 ? <ul>{outfits.map((outfit, i) => <li key={i}><code>{String(outfit.prefabPath)}</code>
      {" · "}{outfit.mode === "attach_to_bone" ? `${strings.amfProduction.attach} (${String(outfit.bone)})` : strings.amfProduction.merge}
      {outfit.toggleName ? ` · ${strings.amfProduction.switchName}: ${String(outfit.toggleName)}` : ""}</li>)}</ul> : null}
    <details><summary>{copy.parameters}</summary><pre>{JSON.stringify(params, null, 2)}</pre></details>
  </div>;
}

export function AiToolsPage() {
  const port = useGateway().aiTools;
  const [snapshot, setSnapshot] = useState<AiToolsSnapshotV01 | null>(null);
  const [prompt, setPrompt] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const [copied, setCopied] = useState(false);
  const approvalSection = useRef<HTMLElement>(null);
  const shownRequests = useRef(new Set<string>());
  useEffect(() => {
    const currentIds = new Set(snapshot?.requests.map(request => request.id) ?? []);
    shownRequests.current = new Set([...shownRequests.current].filter(id => currentIds.has(id)));
    const fresh = snapshot?.requests.filter(request => request.state === "pending" && !shownRequests.current.has(request.id)) ?? [];
    if (fresh.length) {
      fresh.forEach(request => shownRequests.current.add(request.id));
      // Show the actual decision immediately, without changing OS focus or
      // making the player scroll past the connection instructions again.
      approvalSection.current?.scrollIntoView({ block: "start" });
    }
  }, [snapshot?.requests]);
  useEffect(() => {
    if (!port) return;
    let alive = true;
    const update = (value: AiToolsSnapshotV01) => { if (alive) setSnapshot(value); };
    void port.snapshot().then(update, () => { if (alive) setError(true); });
    const unsubscribe = port.subscribe(update);
    return () => { alive = false; unsubscribe(); };
  }, [port]);
  useEffect(() => {
    setPrompt(""); setCopied(false);
    if (!port || !snapshot?.enabled || !snapshot.endpoint) return;
    let alive = true;
    void port.prompt(currentLocale).then(value => { if (alive) setPrompt(value); }, () => { if (alive) setError(true); });
    return () => { alive = false; };
  }, [port, snapshot?.enabled, snapshot?.endpoint]);
  const act = async (action: () => Promise<unknown>) => {
    setBusy(true); setError(false);
    try { await action(); } catch { setError(true); } finally { setBusy(false); }
  };
  if (!port) return <Card><p role="status">{copy.unavailable}</p></Card>;
  return <div className="vua-ai-tools">
    <p>{copy.intro}</p>
    <Card>
      <h2>{copy.first}</h2><p>{copy.firstHint}</p>
      <div className="vua-ai-tools__controls"><Button variant="primary" disabled={busy} onClick={() => { void act(async () => setSnapshot(await port.setEnabled(!snapshot?.enabled))); }}>{snapshot?.enabled ? copy.disable : copy.enable}</Button>
        <span role="status">{snapshot?.enabled ? copy.on : copy.off}</span></div>
      {snapshot?.endpoint ? <p className="vua-caption">{copy.endpoint}: <code>{snapshot.endpoint}</code> · {copy.clients}: {snapshot.connections}</p> : null}
      {error || snapshot?.error ? <p role="alert">{copy.error}</p> : null}
    </Card>
    {snapshot?.enabled ? <Card>
      <h2>{copy.second}</h2><p>{copy.paste}</p>
      <textarea className="vua-ai-tools__prompt" rows={12} readOnly aria-label={copy.second} value={prompt} />
      <Button disabled={busy || !prompt} onClick={() => { void act(async () => { await port.copyPrompt(currentLocale); setCopied(true); }); }}>{copy.copy}</Button>
      {copied ? <p role="status">{copy.copied}</p> : null}<p className="vua-caption">{copy.private}</p>
      <h3>{copy.third}</h3><p>{copy.thirdHint}</p>
    </Card> : null}
    <section ref={approvalSection} className="vua-ai-tools__requests" aria-label={copy.requests}>
      <h2>{copy.requests}</h2><p>{snapshot?.requests.length ? copy.approvalHint : copy.noRequests}</p>
      {[...(snapshot?.requests ?? [])].reverse().map(request => <Card key={request.id}>
        <header><h3>{title(request)}</h3><span role="status">{copy.states[request.state]}</span></header>
        <Review request={request} /><h4>{copy.afterwards}</h4><p>{outcome(request)}</p>
        {request.state === "pending" ? <div className="vua-ai-tools__controls">
          <Button variant="primary" disabled={busy} onClick={() => { void act(async () => setSnapshot(await port.resolve(request.id, "approve"))); }}>{copy.approve}</Button>
          <Button variant="subtle" disabled={busy} onClick={() => { void act(async () => setSnapshot(await port.resolve(request.id, "reject"))); }}>{copy.reject}</Button>
        </div> : null}
        {request.state === "accepted" ? <p role="status">{copy.dispatchedHint}</p> : null}
        {request.state === "expired" ? <p>{copy.expiredHint}</p> : null}
        {request.state === "failed" ? <p role="alert">{copy.error} <code>{request.error}</code></p> : null}
      </Card>)}
    </section>
  </div>;
}
