/** The assembly station's version records: review a recorded state and restore the line to it. */
import { useEffect, useState } from "react";
import type { AmfProductionPort, AmfProductionRevisionPlan, AmfProductionRun } from "../../gateway/index.ts";
import { Badge } from "../../components/primitives/Badge.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { Popconfirm } from "../../components/primitives/Popconfirm.tsx";
import { format, formatDateMinute, strings } from "../../i18n/index.ts";
import { revisionFailure, revisionNames } from "./revisions-model.ts";

const copy = strings.factory.panels.versions;
const fileName = (path: string) => path.split("/").pop()!.replace(/\.mat$/iu, "");
const stepLabel = (key: string): string => key in copy.steps ? copy.steps[key as keyof typeof copy.steps] : key;

export function RevisionsCard({ port, run, disabled, onStarted }: {
  port: AmfProductionPort; run: AmfProductionRun; disabled: boolean;
  onStarted: (taskId: string) => void;
}) {
  const revisions = run.revisions;
  const [selected, setSelected] = useState<string | null>(null);
  const [plan, setPlan] = useState<AmfProductionRevisionPlan | null>(null);
  const [reviewing, setReviewing] = useState(false);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // A changed record list — a restore adds its "before restore" entry on top — asks for a fresh review.
  const recordKey = (revisions ?? []).map(item => item.id).join(" ");
  useEffect(() => { setSelected(null); setPlan(null); setError(null); }, [recordKey]);
  if (revisions === undefined || revisions.length === 0) return null;

  const review = async (revisionId: string) => {
    if (selected === revisionId) { setSelected(null); setPlan(null); setError(null); return; }
    setSelected(revisionId); setPlan(null); setError(null); setReviewing(true);
    try { setPlan(await port.revisionPlan(run.runId, revisionId)); }
    catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setReviewing(false); }
  };
  const start = async () => {
    if (plan === null || starting) return;
    setStarting(true); setError(null);
    try {
      const accepted = await port.restoreRevision(run.runId, plan);
      setSelected(null); setPlan(null); onStarted(accepted.taskId);
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setStarting(false); }
  };
  const failure = error === null ? null : revisionFailure(error);
  return <Card className="vua-revisions">
    <h2>{copy.title}</h2>
    <ul className="vua-revisions__list">
      {revisions.map((revision, index) => <li key={revision.id}>
        <button type="button" className="vua-revisions__row" aria-pressed={selected === revision.id}
          disabled={disabled || reviewing || starting} onClick={() => { void review(revision.id); }}
          data-nav-id={`workshop-revision-${revision.id}`}>
          <span className="vua-revisions__line">
            <strong>{copy.before[revision.before]}</strong>
            {revision.editedInUnity && <Badge tone="warning">{copy.editedInUnity}</Badge>}
            <time dateTime={revision.takenAt}>{formatDateMinute(revision.takenAt)}</time>
          </span>
          <small>{revisionNames(revision)}</small>
        </button>
        {index === 0 && revision.before === "restore" && <p className="vua-caption vua-text-secondary">{copy.undoHint}</p>}
      </li>)}
    </ul>
    {selected !== null && <div className="vua-revisions__plan">
      {reviewing && <p role="status" className="vua-text-secondary">{copy.reviewing}</p>}
      {error !== null && !reviewing && <div role="alert">
        <p>{error}</p>
        {failure !== null && <p className="vua-caption vua-text-secondary">{copy.hint[failure]}</p>}
      </div>}
      {plan !== null && !reviewing && <>
        <ul className="vua-revisions__steps">
          {plan.steps.map(step => <li key={step.key} data-skipped={step.skipped || undefined}>
            <span>{stepLabel(step.key)}
              {step.rebuild === true && !step.skipped && <small>{copy.rebuild}</small>}
              {!step.skipped && step.materials !== undefined && step.materials.length > 0 &&
                step.materials.map(entry => <small key={entry.material}>{format(copy.lightingEntry, { name: fileName(entry.material), min: entry.min, max: entry.max })}</small>)}
              {!step.skipped && step.fixes !== undefined && step.fixes.length > 0 && step.fixes.map(fix => <small key={fix}>{fix}</small>)}
            </span>
            {step.skipped && <span>{copy.skipped}</span>}
          </li>)}
        </ul>
        {plan.warning !== null && <p className="vua-n3__notice">{copy.unityEdited}</p>}
        <div className="vua-n3__actions">
          <Button variant="subtle" onClick={() => { setSelected(null); setPlan(null); setError(null); }}>{copy.back}</Button>
          <Popconfirm title={copy.confirmTitle} description={copy.confirmBody} confirmLabel={copy.confirm} cancelLabel={copy.cancel}
            onConfirm={() => { void start(); }}>
            <Button variant="primary" disabled={disabled || starting} data-nav-id="workshop-revision-restore">{starting ? copy.starting : copy.restore}</Button>
          </Popconfirm>
        </div>
      </>}
    </div>}
  </Card>;
}
