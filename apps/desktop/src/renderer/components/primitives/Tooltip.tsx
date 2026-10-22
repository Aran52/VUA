import { cloneElement, useId, useLayoutEffect, useRef, useState, type ReactElement, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { useModalOwner } from "./modal-layer.tsx";
import { canAnimate, easing, fastMs, motionOff } from "./motion.ts";
import "./tooltip.css";

/**
 * Tier 1, tooltip: a short line about an icon or a control, shown on hover or keyboard focus and
 * gone when the pointer or focus leaves. It never holds actions and never carries the only copy of
 * something the player must know.
 */
const SHOW_DELAY_MS = 350;
const GAP = 8;
const EDGE = 8;

export function Tooltip({ content, placement = "top", block = false, children }: {
  content: ReactNode;
  placement?: "top" | "bottom";
  /** The anchor fills its container, for a whole card or tile. */
  block?: boolean;
  /** One element; it receives `aria-describedby`. A disabled control still shows its tooltip. */
  children: ReactElement<{ "aria-describedby"?: string | undefined }>;
}) {
  const id = useId();
  const owner = useModalOwner();
  const anchor = useRef<HTMLSpanElement>(null);
  const bubble = useRef<HTMLDivElement>(null);
  const timer = useRef<number | null>(null);
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<{ left: number; top: number; side: "top" | "bottom" } | null>(null);

  const clear = () => { if (timer.current !== null) { window.clearTimeout(timer.current); timer.current = null; } };
  const show = (delay: number) => { clear(); timer.current = window.setTimeout(() => setOpen(true), delay); };
  const hide = () => { clear(); setOpen(false); setPosition(null); };

  useLayoutEffect(() => {
    if (!open || !anchor.current || !bubble.current) return;
    const a = anchor.current.getBoundingClientRect();
    const b = bubble.current.getBoundingClientRect();
    const roomAbove = a.top - b.height - GAP >= EDGE;
    const side = placement === "top" ? (roomAbove ? "top" : "bottom") : (a.bottom + b.height + GAP <= window.innerHeight - EDGE ? "bottom" : "top");
    const left = Math.min(Math.max(a.left + a.width / 2 - b.width / 2, EDGE), window.innerWidth - b.width - EDGE);
    const top = side === "top" ? a.top - b.height - GAP : a.bottom + GAP;
    setPosition({ left, top, side });
  }, [open, placement, content]);

  useLayoutEffect(() => {
    if (!position || motionOff() || !canAnimate(bubble.current)) return;
    const rise = position.side === "top" ? 4 : -4;
    const animation = bubble.current.animate([{ opacity: 0, transform: `translateY(${rise}px)` }, { opacity: 1, transform: "none" }], { duration: fastMs(), easing: easing() });
    return () => animation.cancel();
  }, [position?.side, position === null]);

  useLayoutEffect(() => {
    if (!open) return;
    const close = () => hide();
    const onKey = (event: KeyboardEvent) => { if (event.key === "Escape") hide(); };
    window.addEventListener("scroll", close, true);
    window.addEventListener("keydown", onKey, true);
    return () => { window.removeEventListener("scroll", close, true); window.removeEventListener("keydown", onKey, true); };
  }, [open]);

  useLayoutEffect(() => clear, []);

  if (content === null || content === undefined || content === "") return children;
  const describedBy = [children.props["aria-describedby"], open ? id : null].filter(Boolean).join(" ") || undefined;
  return <>
    <span ref={anchor} className="vua-tooltip-anchor" data-block={block || undefined}
      onPointerEnter={() => show(SHOW_DELAY_MS)} onPointerLeave={hide}
      onFocus={event => { if ((event.target as HTMLElement).matches(":focus-visible")) show(0); }} onBlur={hide}
      onPointerDown={hide}>
      {cloneElement(children, { "aria-describedby": describedBy })}
    </span>
    {open ? createPortal(<div ref={bubble} id={id} role="tooltip" className="vua-tooltip" data-side={position?.side ?? placement}
      data-vua-modal-owner={owner}
      style={position ? { left: position.left, top: position.top } : { left: -9999, top: -9999 }}>{content}</div>, document.body) : null}
  </>;
}
