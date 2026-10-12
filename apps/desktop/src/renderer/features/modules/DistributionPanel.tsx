import { useEffect, useState } from "react";
import type { DistributionSnapshotV01 } from "@vua/contracts";
import { useGateway, createDistributionPort } from "../../gateway/index.ts";
import { format, strings, TERMS } from "../../i18n/index.ts";
import { Card } from "../../components/primitives/Card.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { openExternalUrl } from "../../app/open-external.ts";

export function DistributionPanel({ host = false }: { host?: boolean }) {
  const gateway = useGateway(); const port = gateway.distribution ?? createDistributionPort();
  const copy = strings.distribution;
  const text = (key: keyof typeof copy) => format(copy[key], { amf: TERMS.amf });
  const [view, setView] = useState<DistributionSnapshotV01>({ schemaVersion: "0.1", available: false, state: "idle" });
  const [restore, setRestore] = useState(false);
  useEffect(() => { let active = true; void port.snapshot().then(value => { if (active) setView(value); }); const unsubscribe = port.subscribe(setView); return () => { active = false; unsubscribe(); }; }, [gateway.distribution]);
  const update = async (action: () => Promise<DistributionSnapshotV01>) => { try { setView(await action()); } catch { setView(previous => ({ ...previous, state: "failed" })); } };
  const progress = view.progress; const busy = view.state === "checking" || view.state === "downloading";
  const id = view.state === "interrupted" ? view.pendingManifestId ?? view.target?.manifestId : view.target?.manifestId;
  return <Card><div className="vua-page__stack">
    <h2 className="vua-title">{host ? copy.title : text("amf")}</h2>
    {view.available ? <p>{format(copy.installed, { host: view.hostVersion ?? "", amf: TERMS.amf, module: view.amfVersion ?? "" })}</p> : <p className="vua-text-secondary">{copy.offline}</p>}
    {view.target?.newer && view.target.compatible ? <><p>{format(copy.available, { amf: TERMS.amf, version: view.target.amfVersion })}</p><p className="vua-caption">{format(copy.size, { size: (view.target.downloadBytes / 1_048_576).toFixed(1) })}</p></> : view.target && !view.target.compatible ? <p>{text("incompatible")}</p> : view.target && view.state === "idle" ? <p>{text("current")}</p> : null}
    {view.state === "downloading" && progress ? <div role="status"><progress aria-label={copy.download} max={progress.total || 1} value={progress.completed} /><p>{format(copy.staging, { done: (progress.completed / 1_048_576).toFixed(1), total: (progress.total / 1_048_576).toFixed(1), speed: (progress.bytesPerSecond / 1_048_576).toFixed(1) })}</p>{progress.fullPack ? <p>{text("fallback")}</p> : null}</div> : null}
    {view.state === "staged" ? <p role="status">{copy.staged}</p> : null}
    {view.state === "interrupted" ? <p role="status">{copy.interrupted}</p> : null}
    {view.error === "work_is_running" ? <p role="status">{copy.busy}</p> : view.state === "failed" || (view.error && view.error !== "cancelled") ? <p role="status">{copy.failed}</p> : null}
    <div className="vua-page__actions">
      {view.available ? <Button disabled={busy} onClick={() => void update(() => port.check())}>{view.state === "checking" ? copy.checking : copy.check}</Button> : null}
      {!busy && id && (view.state === "interrupted" || (view.target?.newer && view.target.compatible && view.state !== "staged")) ? <Button variant="primary" onClick={() => void update(() => port.stage(id))}>{view.state === "interrupted" ? copy.resume : copy.download}</Button> : null}
      {view.state === "downloading" ? <Button onClick={() => void update(() => port.cancel())}>{copy.cancel}</Button> : null}
      {view.state === "staged" ? <Button variant="primary" onClick={() => void update(() => port.apply())}>{copy.apply}</Button> : null}
      <Button variant="subtle" onClick={() => openExternalUrl(view.target?.releaseUrl ?? "https://ph-r.com/vua/downloads/")}>{copy.host}</Button>
      {view.rollbackAvailable && !busy && !restore ? <Button variant="subtle" onClick={() => setRestore(true)}>{copy.restore}</Button> : null}
      {restore ? <><Button onClick={() => void update(() => port.restore())}>{copy.restoreConfirm}</Button><Button variant="subtle" onClick={() => setRestore(false)}>{copy.keep}</Button></> : null}
    </div>
    {restore ? <p className="vua-caption">{copy.retained}</p> : null}
  </div></Card>;
}
