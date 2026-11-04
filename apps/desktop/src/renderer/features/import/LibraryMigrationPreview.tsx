import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DesktopLibraryDirectoryPreviewV01 } from "@vua/contracts";
import { Icon } from "@vua/design-system";
import { useGateway } from "../../gateway/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { ContentDialog } from "../../components/primitives/ContentDialog.tsx";
import { strings, format } from "../../i18n/index.ts";
import { FolderPickerDialog } from "./FolderPickerDialog.tsx";
import {
  baseName, canUndo, coverage, decide, emptyMigrationState, groupFiles, migrationGroups,
  migrationManifest, relativeUnder, renameMaterial, separateSiblings, setFileExcluded, undo,
  withPreview, type MigrationDecision, type MigrationState,
} from "./migration-model.ts";
import "./import-menu.css";

const copy = strings.importPage.migration;

type Item = DesktopLibraryDirectoryPreviewV01["items"][number];
const megabytes = (bytes: number | null): string => bytes === null ? copy.unknownSize : `${(bytes / 1024 ** 2).toFixed(1)} MB`;

/** Reads one directory and records it; the review stays read-only. */
async function readDirectory(path: string): Promise<DesktopLibraryDirectoryPreviewV01 | null> {
  const api = window.vua?.library;
  if (typeof api?.previewDirectory !== "function") return null;
  try { return await api.previewDirectory(path); } catch { return null; }
}

/** Loads a directory and (bounded) its subtree, so a wrapper chosen as one material can
 *  account for every file it holds. Limits are the preview's own; an unreadable or truncated
 *  directory stays reported rather than silently smaller. */
async function loadSubtree(state: MigrationState, root: string, budget = { directories: 300 }): Promise<MigrationState> {
  let next = state;
  const queue = [root];
  while (queue.length > 0 && budget.directories > 0) {
    const path = queue.shift()!;
    if (next.previews.has(path)) { const preview = next.previews.get(path)!; if (preview.state === "ready") queue.push(...preview.items.filter(item => item.kind === "directory").map(item => item.path)); continue; }
    budget.directories -= 1;
    const preview = await readDirectory(path);
    next = withPreview(next, preview ?? { path, state: "unavailable", truncated: false, items: [] });
    if (preview?.state === "ready") queue.push(...preview.items.filter(item => item.kind === "directory").map(item => item.path));
  }
  return next;
}

function DecisionPicker({ path, decision, onChange }: { path: string; decision: MigrationDecision | undefined; onChange: (decision: MigrationDecision | null) => void }) {
  const options: readonly { key: MigrationDecision; label: string }[] = [
    { key: "material", label: copy.oneMaterial },
    { key: "collection", label: copy.collection },
    { key: "excluded", label: copy.excluded },
  ];
  return <span className="vua-migration__choices" role="group" aria-label={copy.decide}>
    {options.map(option => <button key={option.key} type="button" data-nav-id={`migration-${option.key}-${baseName(path)}`}
      aria-pressed={decision === option.key} onClick={() => onChange(decision === option.key ? null : option.key)}>{option.label}</button>)}
  </span>;
}

function DirectoryNode({ state, path, depth, busy, actions }: {
  state: MigrationState; path: string; depth: number; busy: boolean;
  actions: {
    decide: (path: string, decision: MigrationDecision | null) => void;
    rename: (path: string, name: string) => void;
    open: (path: string) => void;
    separate: (children: readonly string[]) => void;
    group: (parent: string, files: readonly string[], name: string) => void;
    excludeFile: (path: string, excluded: boolean) => void;
  };
}) {
  const preview = state.previews.get(path);
  const decision = state.decisions.get(path);
  const [checkedDirs, setCheckedDirs] = useState<ReadonlySet<string>>(new Set());
  const [checkedFiles, setCheckedFiles] = useState<ReadonlySet<string>>(new Set());
  const [familyName, setFamilyName] = useState("");
  const toggle = (set: ReadonlySet<string>, value: string) => {
    const next = new Set(set); if (next.has(value)) next.delete(value); else next.add(value); return next;
  };
  if (preview === undefined) return <div className="vua-migration__row"><span>{baseName(path)}</span>
    <Button variant="subtle" disabled={busy} onClick={() => actions.open(path)}>{copy.open}</Button></div>;
  if (preview.state !== "ready") return <p role="status">{copy.failed} <Button variant="subtle" disabled={busy} onClick={() => actions.open(path)}>{strings.folderPicker.retry}</Button></p>;
  const directories = preview.items.filter(item => item.kind === "directory");
  const files = preview.items.filter(item => item.kind === "file");
  const others = preview.items.filter(item => item.kind === "link" || item.kind === "other");
  const grouped = new Map<string, string>();
  for (const group of state.looseGroups) for (const file of group.files) grouped.set(file, group.name);
  return <div className="vua-migration__node" data-depth={depth}>
    {preview.truncated ? <p role="status">{copy.truncated}</p> : null}
    {directories.length > 0 && <div className="vua-migration__bulk">
      <span className="vua-caption vua-text-secondary">{format(copy.bulkHint, { count: checkedDirs.size })}</span>
      <Button variant="subtle" disabled={busy || checkedDirs.size === 0} data-nav-id="migration-bulk-separate"
        onClick={() => { actions.separate([...checkedDirs]); setCheckedDirs(new Set()); }}>{copy.bulkApply}</Button>
    </div>}
    <ul className="vua-migration__list">
      {directories.map(item => <li key={item.path}>
        <div className="vua-migration__row">
          <label className="vua-migration__check"><input type="checkbox" aria-label={format(copy.selectFolder, { name: item.name })} checked={checkedDirs.has(item.path)}
            onChange={() => setCheckedDirs(previous => toggle(previous, item.path))} /></label>
          <Icon name="folder" size={16} />
          <span className="vua-migration__name">{item.name}</span>
          <DecisionPicker path={item.path} decision={state.decisions.get(item.path)} onChange={next => actions.decide(item.path, next)} />
        </div>
        {state.decisions.get(item.path) === "material"
          ? <div className="vua-migration__material">
            <label>{copy.nameLabel}<input className="vua-text-input" value={state.names.get(item.path) ?? item.name} maxLength={80}
              onChange={event => actions.rename(item.path, event.target.value)} /></label>
          </div>
          : null}
        {state.decisions.get(item.path) === "material" || state.decisions.get(item.path) === "collection"
          ? depth < 15
            ? <DirectoryNode state={state} path={item.path} depth={depth + 1} busy={busy} actions={actions} />
            : <p role="status">{copy.depthLimit}</p>
          : null}
      </li>)}
      {files.map(item => <li key={item.path}>
        <div className="vua-migration__row">
          <label className="vua-migration__check"><input type="checkbox" aria-label={format(copy.selectFile, { name: item.name })} disabled={grouped.has(item.path) || state.excludedFiles.has(item.path)}
            checked={checkedFiles.has(item.path)} onChange={() => setCheckedFiles(previous => toggle(previous, item.path))} /></label>
          <Icon name="file" size={16} />
          <span className="vua-migration__name">{item.name}</span>
          <span className="vua-caption vua-text-secondary">{megabytes(item.sizeBytes)}</span>
          {grouped.has(item.path) ? <span className="vua-caption">{format(copy.inGroup, { name: grouped.get(item.path) ?? "" })}</span> : null}
          <Button variant="subtle" data-nav-id={`migration-exclude-file-${item.name}`} disabled={busy || grouped.has(item.path)}
            aria-pressed={state.excludedFiles.has(item.path)}
            onClick={() => actions.excludeFile(item.path, !state.excludedFiles.has(item.path))}>{copy.excluded}</Button>
        </div>
      </li>)}
      {others.map(item => <li key={item.path}>
        <div className="vua-migration__row"><Icon name="file" size={16} /><span className="vua-migration__name">{item.name}</span>
          <span className="vua-caption vua-text-secondary">{item.kind === "link" ? copy.reasons.link : copy.reasons.other}</span></div>
      </li>)}
    </ul>
    {files.length > 0 && <div className="vua-migration__bulk">
      <label>{copy.groupName}<input className="vua-text-input" value={familyName} maxLength={80} placeholder={copy.groupNamePlaceholder} onChange={event => setFamilyName(event.target.value)} /></label>
      <Button variant="subtle" disabled={busy || checkedFiles.size === 0 || familyName.trim() === ""} data-nav-id="migration-group-files"
        onClick={() => { actions.group(path, [...checkedFiles], familyName); setCheckedFiles(new Set()); setFamilyName(""); }}>{copy.groupApply}</Button>
    </div>}
    {files.length === 0 && directories.length === 0 && others.length === 0 ? <p className="vua-caption">{copy.empty}</p> : null}
  </div>;
}

/** Structural review plus the boundary edits of the migration design: per-directory choices,
 *  the sibling bulk command, loose-file grouping, undo, four separate coverage areas and the
 *  manifest. Nothing is copied until the import runs over confirmed complete groups. */
export function LibraryMigrationPreview({ onClose, initialRoots }: { onClose: () => void; initialRoots?: readonly string[] }) {
  const gateway = useGateway();
  const [pickerOpen, setPickerOpen] = useState(initialRoots === undefined);
  const [state, setState] = useState<MigrationState | null>(initialRoots === undefined ? null : emptyMigrationState(initialRoots));
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const chosen = useRef(false);
  const view = useMemo(() => state === null ? null : coverage(state), [state]);

  useEffect(() => {
    if (state === null) return;
    const missing = state.roots.filter(root => !state.previews.has(root));
    if (missing.length === 0) return;
    setBusy(true);
    void (async () => {
      let next = state;
      for (const root of missing) {
        const preview = await readDirectory(root);
        next = withPreview(next, preview ?? { path: root, state: "unavailable", truncated: false, items: [] });
      }
      setState(next); setBusy(false);
    })();
  }, [state]);

  const actions = useMemo(() => ({
    decide: (path: string, decision: MigrationDecision | null) => {
      if (state === null) return;
      const next = decide(state, path, decision);
      setState(next);
      if (decision === "material" || decision === "collection") {
        setBusy(true);
        void loadSubtree(next, path).then(loaded => { setState(current => current === next ? loaded : current); setBusy(false); });
      }
    },
    rename: (path: string, name: string) => { if (state !== null) setState(renameMaterial(state, path, name)); },
    open: (path: string) => {
      if (state === null) return;
      setBusy(true);
      void readDirectory(path).then(preview => {
        setState(current => current === null ? current : withPreview(current, preview ?? { path, state: "unavailable", truncated: false, items: [] }));
        setBusy(false);
      });
    },
    separate: (children: readonly string[]) => { if (state !== null) setState(separateSiblings(state, children)); },
    group: (parent: string, files: readonly string[], name: string) => { if (state !== null) setState(groupFiles(state, parent, files, name)); },
    excludeFile: (path: string, excluded: boolean) => { if (state !== null) setState(setFileExcluded(state, path, excluded)); },
  }), [state]);

  const importMaterials = async () => {
    if (state === null) return;
    const groups = migrationGroups(state);
    if (groups.length === 0) return;
    setBusy(true); setMessage(null);
    const outcome = await gateway.warehouseCommands.importGroups(groups);
    setBusy(false);
    setMessage(outcome.ok ? copy.importAccepted : copy.importFailed);
  };

  const copyManifest = async () => {
    if (state === null) return;
    try {
      await navigator.clipboard.writeText(JSON.stringify(migrationManifest(state), null, 2));
      setMessage(copy.manifestCopied);
    } catch { setMessage(copy.manifestFailed); }
  };

  return <>
    <FolderPickerDialog open={pickerOpen} onClose={() => { setPickerOpen(false); if (!chosen.current) onClose(); }}
      onConfirm={selected => {
        chosen.current = true;
        setState(emptyMigrationState(selected));
        setPickerOpen(false);
      }}
      onUseWindowsPicker={() => window.vua?.dialog.pickWarehouseFolders() ?? Promise.resolve(null)} />
    <ContentDialog open={!pickerOpen} title={strings.importPage.migrationTitle} closeLabel={strings.common.dialogClose} onClose={onClose}>
      <div className="vua-migration">
        <p>{copy.description}</p>
        {state === null || view === null ? <p role="status">{strings.folderPicker.loading}</p> : <>
          <div className="vua-migration__coverage" role="status">
            <span data-tone={view.complete ? "ok" : "open"}>{format(copy.coverageComplete, { count: view.materials.length })}</span>
            <span>{format(copy.coverageUnassigned, { count: view.unassigned.length })}</span>
            <span>{format(copy.coverageExcluded, { count: view.excluded.length })}</span>
            <span>{format(copy.coverageUnreadable, { count: view.unreadable.length })}</span>
          </div>
          <div className="vua-migration__toolbar">
            <Button variant="subtle" disabled={busy || !canUndo(state)} data-nav-id="migration-undo" onClick={() => setState(undo(state))}>{copy.undo}</Button>
            <Button variant="subtle" disabled={busy} data-nav-id="migration-manifest" onClick={() => { void copyManifest(); }}>{copy.copyManifest}</Button>
          </div>
          {state.roots.map(root => <section key={root} className="vua-migration__root">
            <h3 className="vua-migration__root-name">{root}</h3>
            <DirectoryNode state={state} path={root} depth={0} busy={busy} actions={actions} />
          </section>)}
          <section className="vua-migration__sections">
            <div>
              <h4>{copy.confirmed}</h4>
              {view.materials.length === 0 ? <p className="vua-caption vua-text-secondary">{copy.none}</p> : <ul>
                {view.materials.map(material => <li key={material.id}>
                  <strong>{material.name}</strong>
                  <span>{format(copy.files, { count: material.files.length })}</span>
                  {material.problem !== null ? <small>{copy.problems[material.problem]}</small> : null}
                </li>)}
              </ul>}
            </div>
            <div>
              <h4>{copy.unassignedTitle}</h4>
              {view.unassigned.length === 0 ? <p className="vua-caption vua-text-secondary">{copy.none}</p> : <ul>
                {view.unassigned.slice(0, 8).map(item => <li key={item.path}><span>{item.name}</span></li>)}
                {view.unassigned.length > 8 ? <li className="vua-caption">{format(copy.more, { count: view.unassigned.length - 8 })}</li> : null}
              </ul>}
            </div>
            <div>
              <h4>{copy.excludedTitle}</h4>
              {view.excluded.length === 0 ? <p className="vua-caption vua-text-secondary">{copy.none}</p> : <ul>
                {view.excluded.slice(0, 8).map(item => <li key={`${item.root}:${item.path}`}><span>{item.path}</span></li>)}
                {view.excluded.length > 8 ? <li className="vua-caption">{format(copy.more, { count: view.excluded.length - 8 })}</li> : null}
              </ul>}
            </div>
            <div>
              <h4>{copy.unreadableTitle}</h4>
              {view.unreadable.length === 0 ? <p className="vua-caption vua-text-secondary">{copy.none}</p> : <ul>
                {view.unreadable.slice(0, 8).map(item => <li key={`${item.path}:${item.reason}`}><span>{relativeUnder(state.roots[0] ?? "", item.path) || item.path}</span>
                  <small>{copy.reasons[item.reason]}</small></li>)}
                {view.unreadable.length > 8 ? <li className="vua-caption">{format(copy.more, { count: view.unreadable.length - 8 })}</li> : null}
              </ul>}
            </div>
          </section>
          {message !== null ? <p role="status" className="vua-n3__notice">{message}</p> : null}
          <div className="vua-n3__actions">
            <Button variant="subtle" onClick={onClose}>{copy.close}</Button>
            <Button variant="primary" disabled={busy || !view.complete} data-nav-id="migration-import"
              onClick={() => { void importMaterials(); }}>{busy ? copy.importing : format(copy.importCta, { count: migrationGroups(state).length })}</Button>
          </div>
          {!view.complete ? <p className="vua-caption vua-text-secondary">{copy.partialBlocked}</p> : null}
        </>}
      </div>
    </ContentDialog>
  </>;
}
