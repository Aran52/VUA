/** The quality check's fit prompt: a garment adjusted in Unity can be kept as a named fit. */
import { useState } from "react";
import type { AmfProductionFitChanges, AmfProductionPort, AmfProductionRun } from "../../gateway/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { toast } from "../../components/primitives/toast-store.ts";
import { format, strings } from "../../i18n/index.ts";
import { defaultFitName, fitFailure, validFitName } from "./fits-model.ts";

const copy = strings.factory.panels.fits;

/** The Avatar's item name on this line, for the default fit name; empty when nothing is selected. */
function avatarItemName(run: AmfProductionRun): string {
  const avatarPath = run.selection?.avatarPrefab ?? "";
  const prefab = run.prefabs.find(candidate => candidate.path === avatarPath);
  return run.sources.find(source => source.sourceId === prefab?.sourceId)?.itemName ?? "";
}

export function FitSaveCard({ port, run, disabled }: { port: AmfProductionPort; run: AmfProductionRun; disabled: boolean }) {
  const [names, setNames] = useState<Readonly<Record<string, string>>>({});
  const [saving, setSaving] = useState<string | null>(null);
  const [saved, setSaved] = useState<readonly string[]>([]);
  const [failure, setFailure] = useState<{ readonly sourceId: string; readonly message: string } | null>(null);
  // Only moved objects make a prompt; player-deleted paths alone never ask for anything.
  const pending = (run.fitChanges ?? []).filter(entry => entry.moved.length > 0 && !saved.includes(entry.sourceId));
  if (pending.length === 0) return null;
  const garmentName = (sourceId: string) => run.sources.find(source => source.sourceId === sourceId)?.itemName ?? sourceId;
  const nameOf = (entry: AmfProductionFitChanges) => names[entry.sourceId] ?? defaultFitName(garmentName(entry.sourceId), avatarItemName(run));
  const save = async (entry: AmfProductionFitChanges) => {
    const name = nameOf(entry);
    if (saving !== null || !validFitName(name)) return;
    setSaving(entry.sourceId); setFailure(null);
    try {
      await port.saveFit(run.runId, entry.sourceId, name, entry.changesDigest);
      setSaved(previous => [...previous, entry.sourceId]);
      toast(copy.saved, "success");
    } catch (reason) {
      setFailure({ sourceId: entry.sourceId, message: reason instanceof Error ? reason.message : String(reason) });
    } finally { setSaving(null); }
  };
  return <Card className="vua-fitsave">
    <h2>{copy.title}</h2>
    {pending.map(entry => {
      const name = nameOf(entry);
      const hint = failure?.sourceId === entry.sourceId ? fitFailure(failure.message) : null;
      return <div className="vua-fitsave__entry" key={entry.sourceId}>
        <p>{format(copy.prompt, { garment: garmentName(entry.sourceId), count: entry.moved.length })}</p>
        {entry.missing.length > 0 && <p className="vua-caption vua-text-secondary">{format(copy.missingNote, { count: entry.missing.length })}</p>}
        <div className="vua-fitsave__row">
          <input value={name} maxLength={80} aria-label={copy.nameLabel} disabled={disabled || saving !== null}
            onChange={event => setNames(previous => ({ ...previous, [entry.sourceId]: event.target.value }))}
            data-nav-id={`workshop-fit-name-${entry.sourceId}`}/>
          <Button disabled={disabled || saving !== null || !validFitName(name)} onClick={() => { void save(entry); }}
            data-nav-id={`workshop-fit-save-${entry.sourceId}`}>{saving === entry.sourceId ? copy.saving : copy.save}</Button>
        </div>
        {failure?.sourceId === entry.sourceId && <div role="alert">
          <p>{failure.message}</p>
          {hint !== null && <p className="vua-caption vua-text-secondary">{copy.hint[hint]}</p>}
        </div>}
      </div>;
    })}
  </Card>;
}
