/** The workshop's production lines: open one, start a new one, or rename, hide or delete a line from its menu. */
import { useEffect, useState } from "react";
import { Icon } from "@vua/design-system";
import { useGateway } from "../../gateway/index.ts";
import { Button } from "../../components/primitives/Button.tsx";
import { ConfirmDialog } from "../../components/primitives/ConfirmDialog.tsx";
import { format, strings } from "../../i18n/index.ts";
import { rememberHiddenLines as rememberHidden, storedHiddenLines as storedHidden, type LineStatus } from "./factory-model.ts";

const factory = strings.factory;
const menu = factory.lineMenu;
const ENDED = ["completed", "completedWithWarnings", "failed", "cancelled"];

export function StatusChip({ status }: { status: LineStatus }) {
  return <span className="vua-line__status" data-status={status}>{factory.status[status]}</span>;
}

export interface RailLine {
  readonly runId: string;
  readonly name: string;
  readonly extra: string | null;
  readonly status: LineStatus;
}

/** How many Unity jobs may run at once, and whether it can change now (only while nothing runs or waits). */
export interface RailCapacity { readonly value: 1 | 2 | 3; readonly changeable: boolean }

export function WorkshopRail({ lines, current, capacity, onSelect, onNew, onRename, onDeleting, onDeleted, onError, refresh }: {
  lines: readonly RailLine[]; current: string; capacity?: RailCapacity | undefined;
  onSelect: (runId: string) => void; onNew: () => void;
  onRename: (runId: string, name: string) => void;
  /** A delete was accepted; the line leaves the page now. */
  onDeleting: (runId: string) => void;
  onDeleted: (runId: string) => void;
  onError: (message: string) => void;
  refresh: () => void;
}) {
  const gateway = useGateway();
  const port = gateway.amfProduction;
  const [hidden, setHidden] = useState(storedHidden);
  const [showHidden, setShowHidden] = useState(false);
  const [open, setOpen] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [confirmLine, setConfirmLine] = useState<RailLine | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [deleting, setDeleting] = useState<ReadonlyMap<string, string>>(new Map());

  // The menu closes on Escape or a press elsewhere.
  useEffect(() => {
    if (open === null) return;
    const outside = (event: PointerEvent) => { if (!(event.target instanceof Element) || !event.target.closest(`[data-line-menu="${open}"]`)) setOpen(null); };
    const escape = (event: KeyboardEvent) => { if (event.key === "Escape") setOpen(null); };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape);
    return () => { document.removeEventListener("pointerdown", outside); document.removeEventListener("keydown", escape); };
  }, [open]);

  const toggleHidden = (runId: string) => {
    const next = new Set(hidden);
    if (next.has(runId)) next.delete(runId); else next.add(runId);
    setHidden(next); rememberHidden(next); setOpen(null);
  };

  const remove = async (line: RailLine) => {
    setConfirming(false); setOpen(null);
    if (!port) return;
    try {
      const accepted = await port.delete(line.runId);
      setDeleting(previous => new Map(previous).set(line.runId, accepted.taskId));
      onDeleting(line.runId);
    } catch (e) { onError(e instanceof Error ? e.message : menu.delete); }
  };

  // A deleted line leaves the list once its task ends; one that could not go says why.
  useEffect(() => {
    if (deleting.size === 0) return;
    const timer = setInterval(() => {
      void gateway.task.snapshot().then(view => {
        for (const [runId, taskId] of deleting) {
          const task = view.tasks.find(item => item.id === taskId);
          if (task !== undefined && !ENDED.includes(task.status)) continue;
          setDeleting(previous => { const next = new Map(previous); next.delete(runId); return next; });
          if (task?.status === "failed") { onError(task.errorText ?? menu.delete); continue; }
          setHidden(previous => { if (!previous.has(runId)) return previous; const next = new Set(previous); next.delete(runId); rememberHidden(next); return next; });
          onDeleted(runId);
        }
        refresh();
      }, () => {});
    }, 1500);
    return () => clearInterval(timer);
  }, [gateway, deleting, onDeleted, onError, refresh]);

  const hiddenCount = lines.filter(line => hidden.has(line.runId)).length;
  const [settingCapacity, setSettingCapacity] = useState(false);
  const chooseCapacity = async (value: 1 | 2 | 3) => {
    if (!port || !capacity || value === capacity.value || settingCapacity) return;
    setSettingCapacity(true);
    try { await port.setCapacity(value); refresh(); } catch (e) { onError(e instanceof Error ? e.message : factory.capacity.label); }
    finally { setSettingCapacity(false); }
  };
  // The confirmation sits outside the sticky rail, so it layers above the whole page.
  return <><aside className="vua-workshop__rail" aria-label={factory.lineList}>
    <Button variant={lines.length === 0 ? "primary" : "default"} onClick={onNew}>{factory.newLine}</Button>
    <ul className="vua-workshop__lines">
      {lines.filter(line => showHidden || !hidden.has(line.runId)).map(line => {
        const removing = deleting.has(line.runId);
        const isHidden = hidden.has(line.runId);
        return <li key={line.runId} className="vua-workshop__line-item" data-line-menu={line.runId} data-hidden={isHidden || undefined} data-selected={line.runId === current || undefined}>
          {renaming === line.runId ? <input className="vua-workshop__line-rename" autoFocus defaultValue={line.name} maxLength={80} aria-label={menu.rename} data-nav-id="workshop-line-name"
            onFocus={event => event.currentTarget.select()}
            onBlur={event => { onRename(line.runId, event.currentTarget.value); setRenaming(null); }}
            onKeyDown={event => {
              if (event.key === "Enter") event.currentTarget.blur();
              if (event.key === "Escape") { event.stopPropagation(); event.currentTarget.value = line.name; event.currentTarget.blur(); }
            }} />
          : <button type="button" className="vua-workshop__line" aria-current={line.runId === current || undefined} disabled={removing} onClick={() => onSelect(line.runId)}>
            <span>{line.name}</span>
            {line.extra && <small className="vua-workshop__line-extra">{line.extra}</small>}
            {removing ? <span className="vua-line__status" data-status="running">{menu.deleting}</span> : <StatusChip status={line.status}/>}
          </button>}
          {!removing && renaming !== line.runId && <button type="button" className="vua-workshop__line-menu" aria-haspopup="true" aria-expanded={open === line.runId}
            aria-label={format(menu.open, { name: line.name })} title={format(menu.open, { name: line.name })}
            onClick={() => setOpen(current => current === line.runId ? null : line.runId)} data-nav-id="workshop-line-menu"><Icon name="caret-down" size={16}/></button>}
          {!removing && open === line.runId && <div className="vua-workshop__line-choices">
            <button type="button" className="vua-workshop__line-hide" onClick={() => { setOpen(null); setRenaming(line.runId); }} data-nav-id="workshop-line-rename">{menu.rename}</button>
            <button type="button" className="vua-workshop__line-hide" onClick={() => toggleHidden(line.runId)} data-nav-id="workshop-line-hide">{isHidden ? menu.unhide : menu.hide}</button>
            <button type="button" className="vua-workshop__line-delete" disabled={line.status === "running"} onClick={() => { setConfirmLine(line); setConfirming(true); }} data-nav-id="workshop-line-delete">{menu.delete}</button>
          </div>}
        </li>;
      })}
    </ul>
    {hiddenCount > 0 && <button type="button" className="vua-workshop__hidden-toggle" onClick={() => setShowHidden(value => !value)}>
      {showHidden ? menu.hideHidden : format(menu.showHidden, { count: hiddenCount })}</button>}
    {capacity && <details className="vua-workshop__capacity" aria-label={factory.capacity.label}>
      <summary>{factory.capacity.label} · {capacity.value}</summary>
      <div className="vua-workshop__capacity-choices">{([1, 2, 3] as const).map(value => <button key={value} type="button" aria-pressed={value === capacity.value}
        disabled={!capacity.changeable || settingCapacity} onClick={() => { void chooseCapacity(value); }} data-nav-id={`workshop-capacity-${value}`}>{value}</button>)}</div>
      <small className="vua-text-secondary">{capacity.changeable ? factory.capacity.hint : factory.capacity.busy}</small>
    </details>}
  </aside>
    <ConfirmDialog open={confirming} danger title={menu.deleteTitle} cancelLabel={menu.cancel} confirmLabel={menu.deleteConfirm}
      onCancel={() => setConfirming(false)} onConfirm={() => { if (confirmLine) void remove(confirmLine); }}>
      <p className="vua-confirm-dialog__text">{format(menu.deleteBody, { name: confirmLine?.name ?? "" })}</p>
    </ConfirmDialog>
  </>;
}
