import { useId, useState } from "react";
import { createPortal } from "react-dom";
import { Icon, type IconName } from "@vua/design-system";
import { strings } from "../../i18n/index.ts";
import type { LibraryCornerState } from "./library-corner-model.ts";

const icons: Record<LibraryCornerState, IconName> = { cloud: "cloud", local: "computer", linked: "computer", downloading: "refresh", paused: "pause", searching: "refresh", uncertain: "question", error: "close" };
export function LibraryCorner({ state, onAction, busy = false }: { state: LibraryCornerState; onAction?: (() => void) | undefined; busy?: boolean }) {
  const [anchor, setAnchor] = useState<{ left: number; top: number } | null>(null);
  const id = useId(); const copy = strings.warehouse.corner;
  const action = onAction !== undefined && state !== "searching" && !busy;
  const hoveredIcon = state === "cloud" || state === "error" ? "download" : state === "local" ? "refresh" : state === "downloading" ? "pause" : state === "paused" ? "anim" : icons[state];
  return <>
    <button type="button" className="vua-library-corner" data-vua-control-role="indicator" data-corner-state={state} data-action={action || undefined}
      aria-label={copy[state]} aria-describedby={anchor === null ? undefined : id} aria-disabled={!action}
      onMouseEnter={event => { const rect = event.currentTarget.getBoundingClientRect(); setAnchor({ left: rect.left + 12, top: rect.top + 58 }); }}
      onMouseLeave={() => setAnchor(null)} onFocus={event => { const rect = event.currentTarget.getBoundingClientRect(); setAnchor({ left: rect.left + 12, top: rect.top + 58 }); }} onBlur={() => setAnchor(null)}
      onClick={event => { event.stopPropagation(); if (action) onAction?.(); }} onDoubleClick={event => event.stopPropagation()}>
      <span className="vua-library-corner__rest"><Icon name={icons[state]} size={16} /></span>
      <span className="vua-library-corner__hover"><Icon name={action ? hoveredIcon : icons[state]} size={16} /></span>
      <svg className="vua-library-corner__focus" viewBox="0 0 40 40" aria-hidden="true"><path d="M2 2H38L2 38Z" fill="none" stroke="currentColor" strokeWidth="2" /></svg>
    </button>
    {anchor === null ? null : createPortal(<div id={id} role="tooltip" className="vua-library-corner__tooltip" style={{ left: Math.max(8, Math.min(anchor.left, window.innerWidth - 280)), top: Math.max(8, Math.min(anchor.top, window.innerHeight - 80)) }}>{copy[state]}</div>, document.body)}
  </>;
}
