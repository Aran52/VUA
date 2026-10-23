import { useEffect, useState } from "react";
import type { OutfitRemovalPlan } from "@vua/contracts";
import type { AmfProductionPort, AmfProductionRun } from "../../gateway/index.ts";
import { Card } from "../../components/primitives/Card.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { strings } from "../../i18n/index.ts";

export function OutfitRemovalCard({ port, run, disabled, onStarted, onError }: {
  port: AmfProductionPort; run: AmfProductionRun; disabled: boolean;
  onStarted: (task: string) => void; onError: (message: string) => void;
}) {
  const copy = strings.outfitRemoval;
  const [source, setSource] = useState("");
  const [plan, setPlan] = useState<OutfitRemovalPlan | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [working, setWorking] = useState(false);
  const facts = run.result?.facts;
  const key = JSON.stringify([run.selection, facts?.outfitRemoval]);
  useEffect(() => { setPlan(null); setConfirmed(false); }, [run.runId, key]);
  const outfits = run.selection?.outfits ?? [];
  const manual = plan?.removal.objects.filter(o => o.manual) ?? [];
  const blocked = disabled || working || outfits.length < 2 || facts?.faceTracking?.manualReplayRequired === true || facts?.outfitRemoval === undefined;
  const review = async () => {
    setWorking(true); setPlan(null); setConfirmed(false);
    try { setPlan(await port.planOutfitRemoval(run.runId, source)); }
    catch (e) { onError(e instanceof Error ? e.message : copy.unknown); }
    finally { setWorking(false); }
  };
  const apply = async () => {
    if (!plan || !confirmed) return;
    setWorking(true);
    try { onStarted((await port.removeOutfit(run.runId, plan, true)).taskId); }
    catch (e) { onError(e instanceof Error ? e.message : copy.unknown); }
    finally { setWorking(false); }
  };
  return <Card>
    <h2>{copy.title}</h2>
    <p>{copy.warning}</p>
    {outfits.length < 2 && <p>{copy.last}</p>}
    {facts?.faceTracking?.manualReplayRequired && <p>{copy.face}</p>}
    {facts?.outfitRemoval === undefined && <p>{copy.unknown}</p>}
    <label className="vua-n3__field">{copy.outfit}<select value={source} disabled={blocked} onChange={e => { setSource(e.target.value); setPlan(null); setConfirmed(false); }}>
      <option value="">{copy.choose}</option>{outfits.map(o => <option key={o.sourceId} value={o.sourceId}>{run.sources.find(s => s.sourceId === o.sourceId)?.itemName ?? o.sourceId}</option>)}</select></label>
    <Button disabled={blocked || !source} onClick={() => { void review(); }}>{copy.review}</Button>
    {plan && <>
      {manual.length > 0 && <div role="alert"><h3>{copy.manual}</h3><ul>{manual.map(o => <li key={o.globalObjectId}>{o.path} · {o.typeName}</li>)}</ul></div>}
      <details><summary>{copy.objects} ({plan.removal.objects.length})</summary><ul>{plan.removal.objects.map(o => <li key={o.globalObjectId}>
        {o.path} · {o.typeName} <code>{o.globalObjectId}</code></li>)}</ul></details>
      <label><input type="checkbox" checked={confirmed} disabled={blocked} onChange={e => setConfirmed(e.target.checked)}/>{copy.confirm}</label>
      <Button variant="primary" disabled={blocked || !confirmed} onClick={() => { void apply(); }}>{working ? copy.working : copy.apply}</Button>
    </>}
  </Card>;
}
