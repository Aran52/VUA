import { useEffect, useRef, useState } from "react";
import type { DesktopLibraryDirectoryPreviewV01 } from "@vua/contracts";
import { Icon } from "@vua/design-system";
import { Button } from "../../components/primitives/Button.tsx";
import { ContentDialog } from "../../components/primitives/ContentDialog.tsx";
import { strings, format } from "../../i18n/index.ts";
import { FolderPickerDialog } from "./FolderPickerDialog.tsx";
import "./import-menu.css";
const copy = strings.importPage.migrationPreview;

function DirectoryPreview({ path, depth = 0 }: { path: string; depth?: number }) {
  const [preview, setPreview] = useState<DesktopLibraryDirectoryPreviewV01 | null>(null);
  const [failed, setFailed] = useState(false); const [opened, setOpened] = useState<ReadonlySet<string>>(new Set());
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    let active = true; setFailed(false); setPreview(null);
    const api = window.vua?.library;
    if (typeof api?.previewDirectory !== "function") { setFailed(true); return; }
    void api.previewDirectory(path).then(value => { if (active) { setPreview(value); setFailed(value.state !== "ready"); } }, () => { if (active) setFailed(true); });
    return () => { active = false; };
  }, [path, retry]);
  if (failed) return <div role="status">{copy.failed} <Button variant="subtle" onClick={() => setRetry(value => value + 1)}>{strings.folderPicker.retry}</Button></div>;
  if (preview === null) return <p role="status">{strings.folderPicker.loading}</p>;
  return <>
    <p className="vua-caption vua-text-secondary">{format(copy.children, { folders: preview.items.filter(item => item.kind === "directory").length, files: preview.items.filter(item => item.kind === "file").length })}</p>
    {preview.truncated ? <p role="status">{copy.truncated}</p> : null}
    {preview.items.length === 0 ? <p className="vua-caption">{copy.empty}</p> : null}
    <ul className="vua-migration-preview__tree">{preview.items.map(item => <li key={item.path}>
      {item.kind === "directory" ? <details onToggle={event => { if (event.currentTarget.open) setOpened(previous => new Set([...previous, item.path])); }}>
        <summary>{item.name} <span className="vua-caption vua-text-secondary">{copy.unassigned}</span></summary>
        {opened.has(item.path) ? depth < 15 ? <DirectoryPreview path={item.path} depth={depth + 1} /> : <p role="status">{copy.depthLimit}</p> : null}
      </details> : <div className="vua-migration-preview__row"><Icon name="file" size={16} /><span>{item.name}</span>{item.kind === "link" || item.kind === "other" ? <span className="vua-caption vua-text-secondary">{copy.skipped}</span> : null}</div>}
    </li>)}</ul>
  </>;
}

/** Structural review is read-only while material-boundary rules are being designed. */
export function LibraryMigrationPreview({ onClose }: { onClose: () => void }) {
  const [pickerOpen, setPickerOpen] = useState(true); const [paths, setPaths] = useState<readonly string[]>([]);
  const [openedRoots, setOpenedRoots] = useState<ReadonlySet<string>>(new Set());
  const chosen = useRef(false);
  return <>
    <FolderPickerDialog open={pickerOpen} onClose={() => { setPickerOpen(false); if (!chosen.current) onClose(); }} onConfirm={selected => { chosen.current = true; setPaths(selected); setOpenedRoots(new Set(selected.length === 1 ? selected : [])); setPickerOpen(false); }} onUseWindowsPicker={() => window.vua?.dialog.pickWarehouseFolders() ?? Promise.resolve(null)} />
    <ContentDialog open={!pickerOpen} title={strings.importPage.migrationTitle} closeLabel={strings.common.dialogClose} onClose={onClose}>
      <div className="vua-migration-preview"><p>{copy.description}</p><p className="vua-caption vua-text-secondary">{copy.planning}</p>
        {paths.map(path => <details key={path} open={openedRoots.has(path)} onToggle={event => { const open = event.currentTarget.open; setOpenedRoots(previous => { const next = new Set(previous); if (open) next.add(path); else next.delete(path); return next; }); }}>
          <summary className="vua-migration-preview__path">{path} <span className="vua-caption vua-text-secondary">{copy.unassigned}</span></summary>
          {openedRoots.has(path) ? <DirectoryPreview path={path} /> : null}
        </details>)}
        <Button onClick={onClose}>{strings.common.dialogClose}</Button>
      </div>
    </ContentDialog>
  </>;
}
