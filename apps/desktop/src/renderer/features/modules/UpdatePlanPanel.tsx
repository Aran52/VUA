import { useEffect, useState } from "react";
import type { UpdatePlanSnapshotV01 } from "@vua/contracts";
import { useGateway } from "../../gateway/index.ts";
import { amfDataMigrationCopy, currentLocale, format, strings, updatePlanCopy } from "../../i18n/index.ts";
import { openExternalUrl } from "../../app/open-external.ts";
import { Card } from "../../components/primitives/Card.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { Popconfirm } from "../../components/primitives/Popconfirm.tsx";

export function UpdatePlanPanel() {
  const port = useGateway().updatePlans;
  const [view, setView] = useState<UpdatePlanSnapshotV01>({ schemaVersion: "0.1", available: false, state: "idle", rollbackAvailable: false });
  useEffect(() => {
    let mounted = true;
    const unsubscribe = port?.subscribe(value => { if (mounted) setView(value); });
    void port?.snapshot().then(value => { if (mounted) setView(value); }).catch(() => {});
    return () => { mounted = false; unsubscribe?.(); };
  }, [port]);
  if (!view.available) return null;
  const copy = updatePlanCopy[currentLocale]; const common = strings.distribution;
  const plan = view.plan; const busy = ["checking", "downloading", "activating"].includes(view.state);
  const progress = view.progress;
  return <Card><section className="vua-page__stack" data-update-plan aria-label={copy.title}>
    <h2 className="vua-title">{copy.title}</h2><p className="vua-caption vua-text-secondary">{copy.explanation}</p>
    {plan ? <><p>{format(copy.versions, { hostFrom: plan.installed.host, hostTo: plan.targets[0].target.version, amfFrom: plan.installed.amf, amfTo: plan.targets[1].target.version })}</p>
      <p className="vua-caption">{format(common.size, { size: ((plan.targets[0].target.downloadBytes + plan.targets[1].target.downloadBytes) / 1_048_576).toFixed(1) })}</p></> : null}
    <div role="status" aria-live="polite">{view.error === "data_migration_requires_installer" ? amfDataMigrationCopy[currentLocale] : view.error === "no_compatible_update_plan" ? copy.none : view.error === "work_is_running" ? common.busy
      : view.state === "checking" ? common.checking : view.state === "staged" ? common.staged : view.state === "interrupted" ? common.interrupted : view.state === "failed" ? common.failed : null}</div>
    {view.state === "downloading" && progress ? <div role="status"><p>{format(copy.component, { component: progress.component === "host" ? "VUA" : "AMF" })}</p>
      <progress aria-label={copy.download} value={progress.completed} max={progress.total || 1} /><p className="vua-caption">{format(common.staging, { done: (progress.completed / 1_048_576).toFixed(1), total: (progress.total / 1_048_576).toFixed(1), speed: (progress.bytesPerSecond / 1_048_576).toFixed(1) })}</p></div> : null}
    <div className="vua-page__actions">
      <Button disabled={busy} onClick={() => void port?.propose()}>{copy.find}</Button>
      {view.error === "data_migration_requires_installer" ? <Button onClick={() => openExternalUrl("https://ph-r.com/vua/downloads/")}>{common.host}</Button> : null}
      {busy && view.state !== "activating" ? <Button variant="subtle" onClick={() => void port?.cancel()}>{view.state === "checking" ? strings.common.cancel : common.cancel}</Button> : null}
      {plan && !busy && ["proposed", "interrupted"].includes(view.state) ? <Button variant="primary" onClick={() => void port?.stage(plan.id)}>{copy.download}</Button> : null}
      {plan && ["staged", "waiting-for-idle"].includes(view.state) ? <Button variant="primary" onClick={() => void port?.apply(plan.id)}>{copy.apply}</Button> : null}
      {view.rollbackAvailable && !busy ? <Popconfirm title={copy.restoreTitle} description={common.retained} confirmLabel={common.restoreConfirm} cancelLabel={common.keep} onConfirm={() => void port?.restore()}><Button variant="subtle">{copy.restore}</Button></Popconfirm> : null}
    </div>
  </section></Card>;
}
