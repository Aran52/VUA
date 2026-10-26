/** The materials room's "Start production" panel: one Avatar, the outfits that
 * fit it first, and one button that opens a prepared line in the workshop. */
import { useEffect, useMemo, useState } from "react";
import { Alert } from "../../components/primitives/Alert.tsx";
import type { PageId } from "../../app/nav-model.ts";
import { libraryBrowser } from "../../app/library-browser-instance.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { ContentDialog } from "../../components/primitives/ContentDialog.tsx";
import { ConfirmDialog } from "../../components/primitives/ConfirmDialog.tsx";
import { Skeleton } from "../../components/primitives/Skeleton.tsx";
import { useGateway, type AmfProductionList, type AmfProductionSource, type LibraryBrowserPort, type LibraryCardFacts } from "../../gateway/index.ts";
import { strings } from "../../i18n/index.ts";
import { buildStartChoices, groupOutfitsByFit } from "./materials-fit-model.ts";
import { rememberAutomationLevel, storedAutomationLevel, type AutomationLevel } from "./factory-model.ts";
import { PermissionModePicker } from "./PermissionModePicker.tsx";
import { requestProductionRun } from "./production-recipe-navigation.ts";
import "./factory.css";

const copy = strings.factory.materialsStart;
const production = strings.amfProduction;

/** One library item the panel can dress the Avatar with: its materials as the
 * workshop knows them, plus the product title when the library matched one. */
interface StartItem {
  readonly itemId: string;
  readonly itemName: string;
  readonly productName: string | null;
  readonly files: readonly string[];
  readonly sources: readonly AmfProductionSource[];
}

type LoadState =
  | { kind: "loading" }
  | { kind: "failed" }
  | { kind: "ready"; list: AmfProductionList; avatarEntryIds: readonly string[]; avatarMaterialNames: readonly string[]; items: readonly StartItem[] };

/** Library rows carry the product → entry mapping the panel needs; read the
 * pages once (libraries are small) and keep only the links. */
async function readLibrary(library: LibraryBrowserPort, avatarProductId: string): Promise<{ avatarEntryIds: string[]; avatarMaterialNames: string[]; titles: Map<string, string | null> }> {
  const avatarEntryIds: string[] = []; const avatarMaterialNames: string[] = []; const titles = new Map<string, string | null>();
  let offset = 0; let total = Number.POSITIVE_INFINITY;
  while (offset < total && offset < 1000) {
    const page = await library.list({ schemaVersion: "0.1", limit: 200, offset });
    total = page.total; offset += page.limit > 0 ? page.limit : 200;
    for (const product of page.catalog.kind === "results" ? page.catalog.items : []) {
      const facts: LibraryCardFacts | undefined = page.facts.get(product.productId);
      const entries = facts?.localEntries ?? [];
      for (const entry of entries) if (!titles.has(entry.entryId)) titles.set(entry.entryId, product.title);
      if (product.productId === avatarProductId) { avatarEntryIds.push(...entries.map(entry => entry.entryId)); avatarMaterialNames.push(...entries.map(entry => entry.displayName)); }
    }
    for (const entry of page.localEntries) {
      const match = page.facts.get(entry.warehouseItemId)?.sourceMatch;
      if (match?.product.productId === avatarProductId) { avatarEntryIds.push(entry.warehouseItemId); avatarMaterialNames.push(entry.displayName); }
      if (!titles.has(entry.warehouseItemId)) titles.set(entry.warehouseItemId, match?.product.title ?? null);
    }
  }
  return { avatarEntryIds: [...new Set(avatarEntryIds)], avatarMaterialNames: [...new Set(avatarMaterialNames)], titles };
}

export function MaterialsStartDialog({ avatar, onNavigate, onClose }: {
  avatar: { readonly productId: string; readonly title: string | null; readonly imageUrl: string | null };
  onNavigate: (page: PageId) => void;
  onClose: () => void;
}) {
  const gateway = useGateway();
  const port = gateway.amfProduction;
  const library = gateway.libraryBrowser ?? libraryBrowser;
  const [load, setLoad] = useState<LoadState>({ kind: "loading" });
  const [retry, setRetry] = useState(0);
  const [checked, setChecked] = useState<ReadonlySet<string>>(new Set());
  const [starting, setStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [level, setLevel] = useState<AutomationLevel>(storedAutomationLevel);
  const [confirmFull, setConfirmFull] = useState(false);
  const applyLevel = (next: AutomationLevel) => { setLevel(next); rememberAutomationLevel(next); };
  const chooseLevel = (next: AutomationLevel) => {
    if (next === "full" && level !== "full") setConfirmFull(true);
    else applyLevel(next);
  };

  useEffect(() => {
    if (port === undefined) { setLoad({ kind: "failed" }); return; }
    let alive = true; setLoad({ kind: "loading" }); setChecked(new Set()); setStartError(null);
    void Promise.all([port.list(), readLibrary(library, avatar.productId)])
      .then(([list, library]) => {
        if (!alive) return;
        const avatarIds = new Set(library.avatarEntryIds);
        const groups = new Map<string, AmfProductionSource[]>();
        for (const source of list.sources) {
          if (avatarIds.has(source.itemId)) continue;
          const group = groups.get(source.itemId) ?? []; group.push(source); groups.set(source.itemId, group);
        }
        const items: StartItem[] = [...groups.entries()].map(([itemId, sources]) => ({
          itemId, itemName: sources[0]?.itemName ?? itemId, productName: library.titles.get(itemId) ?? null,
          files: sources.map(source => source.name), sources,
        }));
        setLoad({ kind: "ready", list, avatarEntryIds: library.avatarEntryIds, avatarMaterialNames: library.avatarMaterialNames, items });
      })
      .catch(() => { if (alive) setLoad({ kind: "failed" }); });
    return () => { alive = false; };
  }, [port, library, avatar.productId, retry]);

  const ready = load.kind === "ready" ? load : null;
  const avatarSources = useMemo(() => ready ? ready.list.sources.filter(source => ready.avatarEntryIds.includes(source.itemId)) : [], [ready]);
  const { fitted, others } = useMemo(() => ready
    ? groupOutfitsByFit(
      { productName: avatar.title, packageNames: [...ready.avatarMaterialNames, ...avatarSources.flatMap(source => [source.itemName, source.name])] },
      ready.items.map(item => ({ item, productName: item.productName, materialName: [item.itemName, ...item.files].join(" ") })))
    : { fitted: [], others: [] }, [ready, avatar.title, avatarSources]);
  const checkedSources = useMemo(() => ready ? ready.items.filter(item => checked.has(item.itemId)).flatMap(item => item.sources.map(source => source.sourceId)) : [], [ready, checked]);
  const canStart = ready !== null && !starting && avatarSources.length > 0 && ready.list.editors.length > 0 && (checked.size > 0 || ready.items.length === 0);

  const start = async () => {
    if (port === undefined || ready === null || !canStart) return;
    setStarting(true); setStartError(null);
    try {
      // The same preparation the workshop's own button sends, with the roles set here.
      const accepted = await port.prepare(buildStartChoices(avatarSources.map(source => source.sourceId), checkedSources), ready.list.editors[0]?.path ?? "");
      requestProductionRun(accepted.runId);
      onNavigate("workshop");
      onClose();
    } catch (error) {
      setStartError(error instanceof Error ? error.message : copy.startFailed);
      setStarting(false);
    }
  };

  const toggle = (itemId: string, on: boolean) => setChecked(current => {
    const next = new Set(current); if (on) next.add(itemId); else next.delete(itemId); return next;
  });
  const pick = (items: readonly { item: StartItem }[], marked: boolean) => items.length === 0 ? null : <ul className="vua-start__picks">
    {items.map(({ item }) => <li key={item.itemId}>
      <label className="vua-start__pick" data-selected={checked.has(item.itemId) || undefined}>
        <input type="checkbox" checked={checked.has(item.itemId)} disabled={starting} onChange={event => toggle(item.itemId, event.target.checked)} />
        <span className="vua-start__pick-text">
          <strong title={item.productName ?? item.itemName}>{item.productName ?? item.itemName}</strong>
          <small title={item.files.join(" · ")}>{item.files.join(" · ")}</small>
        </span>
        {marked && <span className="vua-start__fits">{copy.fitsBadge}</span>}
      </label>
    </li>)}
  </ul>;

  return <ContentDialog open onClose={onClose} title={copy.panelTitle} closeLabel={strings.common.dialogClose}>
    <div className="vua-start">
      <div className="vua-start__avatar">
        {avatar.imageUrl !== null ? <img className="vua-start__thumb" src={avatar.imageUrl} alt="" />
          : <span className="vua-start__thumb vua-start__thumb--empty" aria-hidden="true" />}
        <div>
          <span className="vua-start__avatar-kind">{copy.avatarLabel}</span>
          <strong className="vua-start__avatar-name" title={avatar.title ?? avatar.productId}>{avatar.title ?? avatar.productId}</strong>
        </div>
      </div>
      {load.kind === "loading" ? <div className="vua-start__loading" role="status"><Skeleton width="40%" height={16} /><Skeleton width="100%" height={48} /><Skeleton width="100%" height={48} /></div> :
        load.kind === "failed" ? <Alert tone="error" action={<Button onClick={() => setRetry(value => value + 1)}>{production.retry}</Button>}>{copy.loadFailed}</Alert> : ready !== null && <>
          {avatarSources.length === 0 ? <div className="vua-start__notice" role="status">
            <strong>{copy.notSourceTitle}</strong>
            <p>{copy.notSourceHint}</p>
            <Button onClick={() => { onNavigate("asset-browser"); onClose(); }}>{copy.goToBrowse}</Button>
          </div> : <>
            {ready.items.length === 0 ? <p className="vua-caption vua-text-secondary">{copy.outfitEmpty}</p> : <>
              <h3 className="vua-start__group">{copy.outfits}</h3>
              {pick(fitted, true)}
              {others.length > 0 && <details className="vua-start__others"><summary>{copy.otherOutfits}</summary>{pick(others, false)}</details>}
            </>}
            {ready.list.editors.length === 0 && <Alert tone="warning">{production.editorMissing}</Alert>}
          </>}
          {startError !== null && <Alert tone="error">{copy.startFailed} {startError}</Alert>}
          <fieldset className="vua-start__automation" disabled={starting}>
            <PermissionModePicker value={level} onChoose={chooseLevel} />
            <p className="vua-start__auto-note">{level === "full" ? copy.fullNote : strings.factory.levels[level].hint}</p>
          </fieldset>
          <div className="vua-start__actions">
            <Button variant="subtle" disabled={starting} onClick={onClose}>{production.back}</Button>
            <Button variant="primary" disabled={!canStart} onClick={() => { void start(); }}>{starting ? copy.starting : level === "full" ? copy.startFull : copy.start}</Button>
          </div>
        </>}
      <ConfirmDialog open={confirmFull} danger title={strings.factory.fullAccess.title} cancelLabel={strings.factory.fullAccess.cancel} confirmLabel={strings.factory.fullAccess.confirm}
        onCancel={() => setConfirmFull(false)} onConfirm={() => { setConfirmFull(false); applyLevel("full"); }}>
        <p className="vua-confirm-dialog__text">{strings.factory.fullAccess.body}</p>
      </ConfirmDialog>
    </div>
  </ContentDialog>;
}
