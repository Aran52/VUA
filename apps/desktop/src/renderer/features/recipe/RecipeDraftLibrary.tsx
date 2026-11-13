import { useEffect, useState } from "react";
import type {
  RecipeDraftLegacyCensusV01,
  RecipeDraftMaterialsForLineV01,
  RecipeDraftSelectionStatusV01,
} from "@vua/contracts";
import type { PageId } from "../../app/nav-model.ts";
import { recipeDrafts } from "../../app/recipe-draft-instance.ts";
import { recipePersisted, useRecipeLibraryRevision } from "../../app/recipe-library-revision.ts";
import { Alert } from "../../components/primitives/Alert.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { Card } from "../../components/primitives/Card.tsx";
import { ContentDialog } from "../../components/primitives/ContentDialog.tsx";
import { Skeleton } from "../../components/primitives/Skeleton.tsx";
import { RecipeDraftError, type RecipeAssetRef, type RecipeDraftRead, type RecipeDraftSummary } from "../../gateway/index.ts";
import { format, formatDateTime, strings } from "../../i18n/index.ts";
import { requestNewProductionLine } from "../amf/production-recipe-navigation.ts";

const copy = strings.warehouse.recipeDialog;
const flowCopy = strings.recipeDraftFlow;

function DraftLineDialog({
  draftTitle,
  selections,
  result,
  onClose,
  onStartWorkshop,
}: {
  draftTitle: string;
  selections: readonly RecipeAssetRef[];
  result: RecipeDraftMaterialsForLineV01;
  onClose: () => void;
  onStartWorkshop?: (sourceIds: readonly string[]) => void;
}) {
  const sourceIds = result.sources.map(item => item.sourceId);
  return (
    <ContentDialog
      open
      title={format(flowCopy.dialogTitle, { title: draftTitle })}
      closeLabel={strings.common.dialogClose}
      onClose={onClose}
    >
      <div className="vua-page__stack">
        {result.sources.length > 0 ? (
          <p>{format(flowCopy.readyLead, { count: result.sources.length })}</p>
        ) : (
          <p>{flowCopy.noSources}</p>
        )}
        {result.missing.length > 0 ? (
          <>
            <p className="vua-caption">{format(flowCopy.missingLead, { count: result.missing.length })}</p>
            <ul className="vua-add-to-recipe__list">
              {result.missing.map(entry => {
                const ref = selections[entry.index];
                const label = ref
                  ? `${ref.displayName}${ref.variantName ? ` · ${ref.variantName}` : ""}`
                  : `#${entry.index + 1}`;
                return (
                  <li key={entry.index} className="vua-page__actions">
                    <span>
                      {label}
                      <small className="vua-caption vua-text-secondary"> · {flowCopy.reasons[entry.reason]}</small>
                    </span>
                  </li>
                );
              })}
            </ul>
          </>
        ) : null}
        <div className="vua-page__actions">
          <Button variant="subtle" onClick={onClose}>
            {flowCopy.close}
          </Button>
          {sourceIds.length > 0 && onStartWorkshop ? (
            <Button
              variant="primary"
              data-nav-id="draft-confirm-start-line"
              onClick={() => onStartWorkshop(sourceIds)}
            >
              {format(flowCopy.startWithReady, { count: sourceIds.length })}
            </Button>
          ) : null}
        </div>
      </div>
    </ContentDialog>
  );
}

function DraftEditor({
  draftId,
  onClose,
  onStartLine,
  startingLine,
}: {
  draftId: string;
  onClose: () => void;
  onStartLine?: () => void;
  startingLine: boolean;
}) {
  const [current, setCurrent] = useState<RecipeDraftRead | null>(null);
  const [title, setTitle] = useState("");
  const [selections, setSelections] = useState<readonly RecipeAssetRef[]>([]);
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(true);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [refresh, setRefresh] = useState(0);
  const [fileStatus, setFileStatus] = useState<RecipeDraftSelectionStatusV01 | null>(null);
  const [statusFailed, setStatusFailed] = useState(false);
  const [statusRefresh, setStatusRefresh] = useState(0);
  useEffect(() => {
    let active = true;
    setBusy(true); setFeedback(null); setCurrent(null);
    void recipeDrafts.get(draftId).then((read) => {
      if (!active) return;
      setCurrent(read); setTitle(read.document.title); setSelections(read.document.selections); setDirty(false);
    }).catch(() => { if (active) setFeedback(copy.loadFailed); }).finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, [draftId, refresh]);
  useEffect(() => {
    if (current === null) return;
    let active = true; setFileStatus(null); setStatusFailed(false);
    void recipeDrafts.selectionStatus(draftId).then((status) => {
      if (!active) return;
      if (status.revision !== current.revision || status.items.length !== current.document.selections.length) { setStatusFailed(true); return; }
      setFileStatus(status);
    }).catch(() => { if (active) setStatusFailed(true); });
    return () => { active = false; };
  }, [draftId, current?.revision, refresh, statusRefresh]);
  useEffect(() => {
    const observe = () => setStatusRefresh((value) => value + 1);
    window.addEventListener("focus", observe);
    return () => window.removeEventListener("focus", observe);
  }, []);
  const selectionKey = (s: RecipeAssetRef) => JSON.stringify([s.source, s.identity, s.variantName ?? null, s.warehouseItemId ?? null]);
  const presenceText = (selection: RecipeAssetRef) => {
    if (statusFailed) return strings.warehouse.removeFiles.draftStatusFailed;
    const index = current?.document.selections.findIndex((saved) => selectionKey(saved) === selectionKey(selection));
    const fact = index === undefined || index < 0 ? undefined : fileStatus?.items[index];
    if (fact === undefined) return null;
    return fact.state === "present" ? strings.warehouse.removeFiles.draftPresent : fact.state === "missing" ? strings.warehouse.removeFiles.draftMissing : strings.warehouse.removeFiles.draftNotStored;
  };
  const save = async () => {
    if (current === null || busy || !dirty || title.trim() === "") return;
    setBusy(true); setFeedback(null);
    try {
      const saved = await recipeDrafts.save(draftId, title.trim(), selections, current.revision);
      setCurrent(saved); setTitle(saved.document.title); setSelections(saved.document.selections); setDirty(false);
      recipePersisted(); setFeedback(copy.savedDraft);
    } catch (error) {
      setFeedback(error instanceof RecipeDraftError && error.code === "revision_conflict" ? copy.conflict : copy.loadFailed);
    } finally { setBusy(false); }
  };
  return <ContentDialog open title={copy.libraryTitle} closeLabel={strings.common.dialogClose} onClose={() => { if (!busy) onClose(); }}>
    <div className="vua-page__stack">
      {busy && current === null ? <Skeleton width="100%" height={72} /> : null}
      {current !== null ? <>
        <label>{copy.nameLabel}<input type="text" className="vua-warehouse__search" maxLength={120} disabled={busy} value={title} onChange={(event) => { setTitle(event.target.value); setDirty(true); }} /></label>
        <p className="vua-caption">{format(copy.selectionCount, { count: selections.length })}</p>
        {selections.length === 0 ? <p className="vua-caption vua-text-secondary">{copy.emptyDraft}</p> : <ul className="vua-add-to-recipe__list">
          {selections.map((selection, index) => <li key={JSON.stringify([selection.source, selection.identity, selection.variantName, selection.warehouseItemId])} className="vua-page__actions">
            <span>{selection.displayName}{selection.variantName ? ` · ${selection.variantName}` : ""}<small className="vua-caption vua-text-secondary">{presenceText(selection)}</small></span>
            <Button variant="default" disabled={busy} onClick={() => { setSelections((refs) => refs.filter((_, position) => position !== index)); setDirty(true); }}>{copy.removeSelection}</Button>
          </li>)}
        </ul>}
        <div className="vua-page__actions">
          <Button variant="primary" disabled={busy || !dirty || title.trim() === ""} onClick={() => void save()}>{copy.saveDraft}</Button>
          {onStartLine && selections.length > 0 && !dirty ? (
            <Button variant="default" disabled={busy || startingLine} onClick={onStartLine}>
              {startingLine ? flowCopy.checkingMaterials : flowCopy.useInLine}
            </Button>
          ) : null}
        </div>
      </> : null}
      {feedback !== null ? <p role="status">{feedback}</p> : null}
      <Button variant="default" disabled={busy} onClick={() => setRefresh((value) => value + 1)}>{copy.refreshDraft}</Button>
    </div>
  </ContentDialog>;
}

/** Selection drafts have their own read face, legacy import banner, and Workshop material prefill. */
export function RecipeDraftLibrary({ onNavigate }: { onNavigate?: (page: PageId) => void } = {}) {
  const revision = useRecipeLibraryRevision();
  const [drafts, setDrafts] = useState<readonly RecipeDraftSummary[] | null>(null);
  const [failed, setFailed] = useState(false);
  const [refresh, setRefresh] = useState(0);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [census, setCensus] = useState<RecipeDraftLegacyCensusV01 | null>(null);
  const [migrating, setMigrating] = useState(false);
  const [migrationNote, setMigrationNote] = useState<string | null>(null);
  const [checkingDraftId, setCheckingDraftId] = useState<string | null>(null);
  const [lineReview, setLineReview] = useState<{
    readonly title: string;
    readonly selections: readonly RecipeAssetRef[];
    readonly result: RecipeDraftMaterialsForLineV01;
  } | null>(null);

  useEffect(() => {
    let active = true;
    setFailed(false); setDrafts(null);
    void recipeDrafts.list().then((entries) => { if (active) setDrafts(entries); }).catch(() => { if (active) setFailed(true); });
    void recipeDrafts.legacyCensus().then((observed) => { if (active) setCensus(observed); }).catch(() => { if (active) setCensus(null); });
    return () => { active = false; };
  }, [revision, refresh]);

  const unmigratedCount = census
    ? census.documents.filter(doc => doc.parseable && doc.existingDraftRevision === null && doc.verifiableSelections > 0).length
    : 0;

  const runMigrateLegacy = async () => {
    if (migrating) return;
    setMigrating(true); setMigrationNote(null);
    try {
      const outcome = await recipeDrafts.migrateLegacy();
      const skipped = outcome.unresolvedOnlyCount + outcome.failedCount;
      setMigrationNote(
        skipped > 0
          ? format(flowCopy.legacyDoneWithSkipped, { migrated: outcome.migratedCount, skipped })
          : format(flowCopy.legacyDone, { migrated: outcome.migratedCount }),
      );
      recipePersisted();
      setRefresh(value => value + 1);
    } catch {
      setMigrationNote(flowCopy.error);
    } finally {
      setMigrating(false);
    }
  };

  const startFromDraft = async (draftId: string, fallbackTitle: string) => {
    if (checkingDraftId !== null) return;
    setCheckingDraftId(draftId);
    setMigrationNote(null);
    try {
      const [read, result] = await Promise.all([
        recipeDrafts.get(draftId),
        recipeDrafts.materialsForLine(draftId),
      ]);
      if (result.missing.length === 0 && result.sources.length > 0 && onNavigate) {
        setSelectedId(null);
        requestNewProductionLine(result.sources.map(item => item.sourceId));
        onNavigate("workshop");
        return;
      }
      setLineReview({
        title: read.document.title || fallbackTitle,
        selections: read.document.selections,
        result,
      });
    } catch {
      setMigrationNote(flowCopy.error);
    } finally {
      setCheckingDraftId(null);
    }
  };

  return <Card><div className="vua-page__stack">
    <h2 className="vua-subtitle">{copy.libraryTitle}</h2>
    {unmigratedCount > 0 ? (
      <Alert
        tone="info"
        title={format(flowCopy.legacyBannerTitle, { count: unmigratedCount })}
        action={
          <Button
            variant="primary"
            disabled={migrating}
            data-nav-id="recipe-draft-migrate-legacy"
            onClick={() => { void runMigrateLegacy(); }}
          >
            {migrating ? flowCopy.legacyMigrating : flowCopy.legacyMigrate}
          </Button>
        }
      >
        {flowCopy.legacyBannerBody}
      </Alert>
    ) : null}
    {migrationNote !== null ? <p role="status" className="vua-caption">{migrationNote}</p> : null}
    {failed ? <div role="alert"><p>{copy.loadFailed}</p><Button variant="default" onClick={() => setRefresh((value) => value + 1)}>{strings.warehouse.detail.retry}</Button></div>
      : drafts === null ? <Skeleton width="100%" height={40} />
      : drafts.length === 0 ? <p className="vua-caption vua-text-secondary">{copy.noRecipes}</p>
      : <ul className="vua-add-to-recipe__list">{drafts.map((draft) => <li key={draft.draftId} className="vua-page__actions">
        <span>{draft.title} · {format(copy.selectionCount, { count: draft.selectionCount })} · {formatDateTime(draft.updatedAt)}</span>
        <div className="vua-page__actions">
          {draft.selectionCount > 0 ? (
            <Button
              variant="primary"
              disabled={checkingDraftId !== null}
              data-nav-id={`draft-start-line-${draft.draftId}`}
              onClick={() => { void startFromDraft(draft.draftId, draft.title); }}
            >
              {checkingDraftId === draft.draftId ? flowCopy.checkingMaterials : flowCopy.useInLine}
            </Button>
          ) : null}
          <Button variant="default" onClick={() => setSelectedId(draft.draftId)}>{copy.openDraft}</Button>
        </div>
      </li>)}</ul>}
    {selectedId !== null ? (
      <DraftEditor
        key={selectedId}
        draftId={selectedId}
        onClose={() => setSelectedId(null)}
        onStartLine={() => {
          const found = drafts?.find(item => item.draftId === selectedId);
          void startFromDraft(selectedId, found?.title ?? selectedId);
        }}
        startingLine={checkingDraftId === selectedId}
      />
    ) : null}
    {lineReview !== null ? (
      <DraftLineDialog
        draftTitle={lineReview.title}
        selections={lineReview.selections}
        result={lineReview.result}
        onClose={() => setLineReview(null)}
        {...(onNavigate
          ? {
              onStartWorkshop: (sourceIds: readonly string[]) => {
                setLineReview(null);
                setSelectedId(null);
                requestNewProductionLine(sourceIds);
                onNavigate("workshop");
              },
            }
          : {})}
      />
    ) : null}
  </div></Card>;
}
