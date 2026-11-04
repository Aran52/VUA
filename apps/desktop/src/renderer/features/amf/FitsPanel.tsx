/** The control center's saved fits: rename and delete the adjustments kept across lines. */
import { useEffect, useState } from "react";
import type { AmfFitSummary, AmfProductionPort } from "../../gateway/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { Popconfirm } from "../../components/primitives/Popconfirm.tsx";
import { toast } from "../../components/primitives/toast-store.ts";
import { format, formatDateTime, strings } from "../../i18n/index.ts";
import { fitFailure, validFitName } from "./fits-model.ts";

const copy = strings.controlCenter.fits;
const hints = strings.factory.panels.fits.hint;

export function FitsPanel({ port }: { port: AmfProductionPort }) {
  const [fits, setFits] = useState<readonly AmfFitSummary[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [working, setWorking] = useState(false);
  const [failure, setFailure] = useState<{ readonly fitId: string; readonly message: string } | null>(null);
  const [reload, setReload] = useState(0);
  useEffect(() => {
    let alive = true;
    setLoadError(null);
    void port.listFits().then(value => { if (alive) setFits(value); },
      (reason: unknown) => { if (alive) { setFits(null); setLoadError(reason instanceof Error ? reason.message : String(reason)); } });
    return () => { alive = false; };
  }, [port, reload]);
  // The management list appears once a first fit exists; an empty store renders nothing, a load failure says so.
  if (loadError === null && (fits === null || fits.length === 0)) return null;
  const refresh = async () => { setFits(await port.listFits()); };
  const rename = async (fit: AmfFitSummary) => {
    if (working || !validFitName(name)) return;
    setWorking(true); setFailure(null);
    try { await port.renameFit(fit.fitId, name); setRenaming(null); await refresh(); }
    catch (reason) { setFailure({ fitId: fit.fitId, message: reason instanceof Error ? reason.message : String(reason) }); }
    finally { setWorking(false); }
  };
  const remove = async (fit: AmfFitSummary) => {
    if (working) return;
    setWorking(true); setFailure(null);
    try { await port.deleteFit(fit.fitId); toast(copy.deleted, "success"); await refresh(); }
    catch (reason) { setFailure({ fitId: fit.fitId, message: reason instanceof Error ? reason.message : String(reason) }); }
    finally { setWorking(false); }
  };
  return <section className="vua-fits" aria-label={copy.title}>
    <h2>{copy.title}</h2>
    {loadError !== null ? <p role="alert">{loadError} <Button variant="subtle" onClick={() => setReload(value => value + 1)}>{strings.amfProduction.retry}</Button></p>
      : <>
    <p className="vua-caption vua-text-secondary">{copy.condition}</p>
    <ul className="vua-fits__list">
      {(fits ?? []).map(fit => {
        const hint = failure?.fitId === fit.fitId ? fitFailure(failure.message) : null;
        return <li key={fit.fitId}>
          {renaming === fit.fitId ? <>
            <input value={name} maxLength={80} aria-label={copy.rename} disabled={working} autoFocus
              onChange={event => setName(event.target.value)}
              onKeyDown={event => { if (event.key === "Enter") void rename(fit); if (event.key === "Escape") setRenaming(null); }}
              data-nav-id={`control-fit-name-${fit.fitId}`}/>
            <Button disabled={working || !validFitName(name)} onClick={() => { void rename(fit); }}>{copy.renameSave}</Button>
            <Button variant="subtle" disabled={working} onClick={() => setRenaming(null)}>{copy.cancel}</Button>
          </> : <>
            <span className="vua-fits__facts"><strong>{fit.name}</strong>
              <small>{fit.garment.name} · {fit.avatar.name} · {format(copy.updated, { time: formatDateTime(fit.updatedAt) })}</small></span>
            <span className="vua-fits__actions">
              <Button variant="subtle" disabled={working} onClick={() => { setRenaming(fit.fitId); setName(fit.name); setFailure(null); }}
                data-nav-id={`control-fit-rename-${fit.fitId}`}>{copy.rename}</Button>
              <Popconfirm danger title={copy.deleteTitle} description={format(copy.deleteBody, { name: fit.name })}
                confirmLabel={copy.deleteConfirm} cancelLabel={copy.cancel} onConfirm={() => { void remove(fit); }}>
                <Button variant="subtle" disabled={working} data-nav-id={`control-fit-delete-${fit.fitId}`}>{copy.deleteAction}</Button>
              </Popconfirm>
            </span>
          </>}
          {failure?.fitId === fit.fitId && <div role="alert" className="vua-fits__error">
            <p>{failure.message}</p>
            {hint !== null && <p className="vua-caption vua-text-secondary">{hints[hint]}</p>}
          </div>}
        </li>;
      })}
    </ul>
    </>}
  </section>;
}
