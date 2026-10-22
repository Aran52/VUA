import { useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Icon } from "@vua/design-system";
import { usePresenceMotion } from "./motion.ts";
import "./drawer.css";

/**
 * Tier 5, drawer: a panel that slides in from the right under the title bar for longer content
 * read next to the page — details of a row, terms, the pinned notification list. There is no
 * backdrop: the page behind stays visible and usable. Escape inside the drawer closes it.
 */
export function Drawer({ open, title, closeLabel, onClose, children, width = 420, actions }: {
  open: boolean;
  title: ReactNode;
  closeLabel: string;
  onClose: () => void;
  children: ReactNode;
  width?: number;
  /** Extra controls in the header, before the close button. */
  actions?: ReactNode;
}) {
  const id = useId();
  const panel = useRef<HTMLElement>(null);
  const [top, setTop] = useState(0);
  // Rendered in a portal, so it takes the module color of the page that opened it.
  const [module, setModule] = useState<string | undefined>(undefined);
  const shown = usePresenceMotion(open, panel,
    [{ opacity: 0, transform: "translateX(24px)" }, { opacity: 1, transform: "none" }],
    [{ opacity: 1, transform: "none" }, { opacity: 0, transform: "translateX(16px)" }], true);

  useLayoutEffect(() => {
    if (!open) return;
    setModule((document.activeElement?.closest<HTMLElement>("[data-module]") ?? document.querySelector<HTMLElement>(".vua-shell"))?.dataset.module);
    const measure = () => setTop(document.querySelector(".vua-shell__header")?.getBoundingClientRect().bottom ?? 0);
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, [open]);

  if (!shown) return null;
  return createPortal(<aside ref={panel} className="vua-drawer" role="dialog" aria-modal="false" aria-labelledby={`${id}-title`}
    data-closing={open ? undefined : true} data-module={module} style={{ top, width: `min(${width}px, calc(100vw - 32px))` }}
    onKeyDown={event => { if (event.key === "Escape") { event.stopPropagation(); onClose(); } }}>
    <header className="vua-drawer__header">
      <h2 id={`${id}-title`} className="vua-drawer__title">{title}</h2>
      {actions}
      <button type="button" className="vua-drawer__close" aria-label={closeLabel} onClick={onClose}><Icon name="close" size={16} /></button>
    </header>
    <div className="vua-drawer__body">{children}</div>
  </aside>, document.body);
}
