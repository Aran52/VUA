import type { UpdateComponentV02 } from "@vua/contracts";
import { useComponentUpdate } from "../../app/component-update-store.ts";
import { componentUpdateCopy, currentLocale, format, formatDateTime, strings, updateRecoveryCopy } from "../../i18n/index.ts";
import { Card } from "../../components/primitives/Card.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { openExternalUrl } from "../../app/open-external.ts";
import { Popconfirm } from "../../components/primitives/Popconfirm.tsx";

export function ComponentUpdatePanel({ component }: { component: UpdateComponentV02 }) {
  const { view, port, check, automatic, setAutomatic } = useComponentUpdate(component);
  const copy = componentUpdateCopy[currentLocale]; const common = strings.distribution;
  const busy = ["checking", "downloading", "activating", "verifying"].includes(view.state);
  const progress = view.progress;
  const hostAction = component === "host";
  return <Card><section className="vua-page__stack" aria-label={copy[component]} data-component-update={component}>
    <h2 className="vua-title">{copy[component]}</h2>
    {view.installedVersion ? <p>{format(copy.installed, { version: view.installedVersion })}</p> : null}
    {view.available ? <>
      <label className="vua-settings-toggle"><input type="checkbox" checked={automatic} onChange={event => setAutomatic(event.target.checked)} />{copy.automatic}</label>
      <p className="vua-caption vua-text-secondary">{copy.source}</p>
      {view.error === "update_recovered" ? <p role="status">{updateRecoveryCopy[currentLocale]}</p> : null}
      <div role="status" aria-live="polite">
        {view.state === "checking" ? common.checking : view.state === "failed" || (view.error && !["cancelled", "work_is_running", "update_recovered"].includes(view.error)) ? common.failed : view.state === "staged" ? common.staged
          : view.error === "work_is_running" ? common.busy : view.state === "interrupted" ? common.interrupted : view.error === "cancelled" ? copy.cancelled
          : view.target ? format(copy.available, { version: view.target.version }) : view.requiresHost ? format(copy.requiresHost, { range: view.requiresHost })
          : !view.checkedAt ? copy.notChecked : !view.latestVersion ? copy.empty : copy.current}
      </div>
      {view.target ? <p className="vua-caption">{format(common.size, { size: (view.target.downloadBytes / 1_048_576).toFixed(1) })}</p> : null}
      {view.checkedAt ? <p className="vua-caption vua-text-secondary">{format(strings.settings.version.updateCheckedAt, { at: formatDateTime(view.checkedAt) })}</p> : null}
      {view.state === "downloading" && progress ? <div role="status"><progress aria-label={common.download} max={progress.total || 1} value={progress.completed} /><p>{format(common.staging, { done: (progress.completed / 1_048_576).toFixed(1), total: (progress.total / 1_048_576).toFixed(1), speed: (progress.bytesPerSecond / 1_048_576).toFixed(1) })}</p></div> : null}
      <div className="vua-page__actions">
        <Button disabled={busy} data-nav-id={`${component}-check-updates`} onClick={() => void check()}>{common.check}</Button>
        {view.state === "checking" || view.state === "downloading" ? <Button variant="subtle" onClick={() => void port?.cancel(component)}>{view.state === "checking" ? strings.common.cancel : common.cancel}</Button> : null}
        {hostAction && view.target && !busy && ["idle", "interrupted"].includes(view.state) ? <Button variant="primary" onClick={() => void port?.stage(component, view.target!.manifestId)}>{view.state === "interrupted" ? common.resume : common.download}</Button> : null}
        {hostAction && ["staged", "waiting-for-idle"].includes(view.state) ? <Button variant="primary" onClick={() => void port?.apply(component)}>{common.apply}</Button> : null}
        {hostAction && view.rollbackAvailable && !busy ? <Popconfirm title={common.restoreTitle} description={common.retained} confirmLabel={common.restoreConfirm} cancelLabel={common.keep} onConfirm={() => void port?.restore(component)}><Button variant="subtle">{common.restore}</Button></Popconfirm> : null}
        {view.target ? <Button variant="subtle" onClick={() => openExternalUrl(view.target!.releaseUrl)}>{copy.release}</Button> : null}
      </div>
      {!hostAction && view.target ? <p className="vua-caption vua-text-secondary">{copy.downloadLater}</p> : null}
    </> : <><p>{copy.unavailable}</p><div><Button onClick={() => openExternalUrl("https://ph-r.com/vua/downloads/")}>{common.host}</Button></div></>}
  </section></Card>;
}
