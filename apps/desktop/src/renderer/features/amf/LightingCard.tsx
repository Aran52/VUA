/** The quality check's lilToon brightness alignment: outfit materials take the body's lower and upper limits. */
import { useEffect, useState } from "react";
import { Icon } from "@vua/design-system";
import type { AmfLightingPlan, AmfProductionPort, AmfProductionRun } from "../../gateway/index.ts";
import { Card } from "../../components/primitives/Card.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { format, strings } from "../../i18n/index.ts";
import type { AutomationLevel } from "./factory-model.ts";

const copy = strings.factory.panels.lighting;
const fileName = (path: string) => path.split("/").pop()!.replace(/\.mat$/iu, "");

export function LightingCard({ port, run, level, disabled, onStarted, onError }: {
  port: AmfProductionPort; run: AmfProductionRun; level: AutomationLevel; disabled: boolean;
  onStarted: (taskId: string) => void; onError: (message: string) => void;
}) {
  const [plan, setPlan] = useState<AmfLightingPlan | null>(null);
  const [state, setState] = useState<"loading" | "ready" | "needs_check" | "unavailable">("loading");
  const [chosen, setChosen] = useState<ReadonlySet<string>>(new Set());
  const [starting, setStarting] = useState(false);
  // The plan follows the line's facts: a new check or a finished change reads it again.
  const revision = `${run.runId}:${run.steps.length}:${run.result?.facts?.avatarName ?? ""}`;
  useEffect(() => {
    let alive = true;
    setState("loading");
    void port.planLighting(run.runId).then(next => {
      if (!alive) return;
      setPlan(next); setChosen(new Set(next.entries.map(entry => entry.material))); setState("ready");
    }, error => {
      if (!alive) return;
      setPlan(null); setState(error instanceof Error && error.message.includes("inspect_required") ? "needs_check" : "unavailable");
    });
    return () => { alive = false; };
  }, [port, revision]);
  if (state === "loading" || state === "unavailable") return null;
  const apply = async () => {
    if (!plan || starting) return;
    setStarting(true);
    try { onStarted((await port.applyLighting(run.runId, plan, [...chosen])).taskId); }
    catch (error) { onError(error instanceof Error ? error.message : copy.title); }
    finally { setStarting(false); }
  };
  const perMaterial = level === "ask";
  return <Card className="vua-lighting">
    <h2>{copy.title}</h2>
    {state === "needs_check" || plan === null ? <p className="vua-text-secondary">{copy.needsCheck}</p>
      : plan.reference === null ? <p className="vua-text-secondary">{plan.referenceReason === "tie" ? copy.noReference.tie : copy.noReference.no_avatar_liltoon}</p>
      : plan.entries.length === 0 ? <p><Icon name="check" size={16}/> {format(copy.matched, { ...plan.reference })}</p>
      : <>
        <p>{format(copy.intro, { ...plan.reference })}</p>
        <ul className="vua-lighting__entries">{plan.entries.map(entry => <li key={entry.material}>
          {perMaterial ? <label><input type="checkbox" checked={chosen.has(entry.material)} onChange={event => setChosen(previous => {
            const next = new Set(previous); if (event.target.checked) next.add(entry.material); else next.delete(entry.material); return next;
          })}/>{fileName(entry.material)}</label> : <span>{fileName(entry.material)}</span>}
          <span className="vua-caption vua-text-secondary">{format(copy.entry, { from: `${entry.current.min} – ${entry.current.max}`, to: `${entry.proposed.min} – ${entry.proposed.max}` })}</span>
        </li>)}</ul>
        {plan.skipped.length > 0 && <p className="vua-caption vua-text-secondary">{format(copy.shared, { count: String(plan.skipped.length) })}</p>}
        <p className="vua-caption vua-text-secondary">{copy.restoreNote}</p>
        <div className="vua-n3__actions"><Button variant="primary" disabled={disabled || starting || chosen.size === 0} onClick={() => { void apply(); }} data-nav-id="workshop-lighting-apply">
          {starting ? copy.applying : format(copy.apply, { count: String(chosen.size) })}</Button></div>
      </>}
  </Card>;
}
