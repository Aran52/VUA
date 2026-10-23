import { cloneElement, useEffect, useId, useLayoutEffect, useRef, useState, type MouseEvent, type ReactElement, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Button } from "./Button.tsx";
import { useModalOwner } from "./modal-layer.tsx";
import { usePresenceMotion } from "./motion.ts";
import "./popconfirm.css";

/**
 * Tier 4, popconfirm: a small bubble beside the button that asks once before an everyday action
 * the player can undo or redo (remove a repository, stop a task, disable a module). The page stays
 * in place; Escape or a click elsewhere cancels. Irreversible or high-stakes decisions use a dialog.
 */
const GAP = 8;
const EDGE = 8;

type TriggerProps = { onClick?: (event: MouseEvent<HTMLElement>) => void; "aria-expanded"?: boolean; "aria-haspopup"?: "dialog"; disabled?: boolean };

export function Popconfirm({ title, description, confirmLabel, cancelLabel, danger = false, onConfirm, children }: {
  title: ReactNode;
  description?: ReactNode;
  confirmLabel: string;
  cancelLabel: string;
  danger?: boolean;
  onConfirm: () => void;
  /** The button that opens the bubble. */
  children: ReactElement<TriggerProps>;
}) {
  const id = useId();
  const owner = useModalOwner();
  const anchor = useRef<HTMLSpanElement>(null);
  const bubble = useRef<HTMLDivElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<{ left: number; top: number; side: "top" | "bottom" } | null>(null);
  // Rendered in a portal, so it takes the module color of the page around its button.
  const module = anchor.current?.closest<HTMLElement>("[data-module]")?.dataset.module;
  const shown = usePresenceMotion(open, bubble,
    [{ opacity: 0, transform: "scale(0.96)" }, { opacity: 1, transform: "none" }],
    [{ opacity: 1, transform: "none" }, { opacity: 0, transform: "scale(0.98)" }]);

  const close = (refocus: boolean) => {
    setOpen(false);
    if (refocus) anchor.current?.querySelector<HTMLElement>("button, [href], [tabindex]")?.focus();
  };

  useLayoutEffect(() => {
    if (!open || !anchor.current || !bubble.current) return;
    const a = anchor.current.getBoundingClientRect();
    const b = bubble.current.getBoundingClientRect();
    const side = a.bottom + GAP + b.height <= window.innerHeight - EDGE ? "bottom" : "top";
    const left = Math.min(Math.max(a.left + a.width / 2 - b.width / 2, EDGE), window.innerWidth - b.width - EDGE);
    setPosition({ left, top: side === "bottom" ? a.bottom + GAP : a.top - GAP - b.height, side });
    cancel.current?.focus();
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!bubble.current?.contains(target) && !anchor.current?.contains(target)) close(false);
    };
    const reposition = () => close(false);
    document.addEventListener("pointerdown", outside, true);
    window.addEventListener("resize", reposition);
    return () => { document.removeEventListener("pointerdown", outside, true); window.removeEventListener("resize", reposition); };
  }, [open]);

  const trigger = cloneElement(children, {
    "aria-haspopup": "dialog",
    "aria-expanded": open,
    onClick: (event: MouseEvent<HTMLElement>) => { event.stopPropagation(); setOpen(value => !value); },
  });

  return <>
    <span ref={anchor} className="vua-popconfirm-anchor">{trigger}</span>
    {shown ? createPortal(<div ref={bubble} role="alertdialog" aria-modal="false" aria-labelledby={`${id}-title`}
      aria-describedby={description ? `${id}-body` : undefined} className="vua-popconfirm" data-side={position?.side ?? "bottom"}
      data-vua-modal-owner={owner} data-module={module} data-vua-esc-local="" data-closing={open ? undefined : true}
      style={position ? { left: position.left, top: position.top } : { left: -9999, top: -9999 }}
      onKeyDown={event => { if (event.key === "Escape") { event.stopPropagation(); close(true); } }}>
      <strong id={`${id}-title`} className="vua-popconfirm__title" data-danger={danger || undefined}>{title}</strong>
      {description ? <div id={`${id}-body`} className="vua-popconfirm__body">{description}</div> : null}
      <div className="vua-popconfirm__actions">
        <button ref={cancel} type="button" className="vua-button vua-button--subtle" onClick={() => close(true)}>{cancelLabel}</button>
        <Button variant={danger ? "danger" : "primary"} onClick={() => { close(false); onConfirm(); }}>{confirmLabel}</Button>
      </div>
    </div>, document.body) : null}
  </>;
}
