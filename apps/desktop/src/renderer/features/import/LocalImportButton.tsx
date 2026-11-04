import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Icon } from "@vua/design-system";
import { Button } from "../../components/primitives/Button.tsx";
import { strings } from "../../i18n/index.ts";
import "./import-menu.css";
export function LocalImportButton({ onSingle, onMigration }: { onSingle: () => void; onMigration: () => void }) {
  const [open, setOpen] = useState(false); const container = useRef<HTMLDivElement>(null); const popup = useRef<HTMLDivElement>(null);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const keepOpen = () => { if (closeTimer.current !== null) clearTimeout(closeTimer.current); closeTimer.current = null; };
  const leave = () => { keepOpen(); closeTimer.current = setTimeout(() => setOpen(false), 120); };
  const inside = (target: EventTarget | null) => target instanceof Node && (container.current?.contains(target) || popup.current?.contains(target));
  useEffect(() => () => keepOpen(), []);
  useLayoutEffect(() => {
    if (!open) return;
    const position = () => {
      if (!container.current || !popup.current) return;
      const anchor = container.current.getBoundingClientRect(); const menu = popup.current.getBoundingClientRect();
      popup.current.style.left = `${Math.max(8, Math.min(anchor.left, window.innerWidth - menu.width - 8))}px`;
      popup.current.style.top = `${Math.max(8, Math.min(anchor.bottom + 2, window.innerHeight - menu.height - 8))}px`;
    };
    position(); window.addEventListener("resize", position); window.addEventListener("scroll", position, true);
    return () => { window.removeEventListener("resize", position); window.removeEventListener("scroll", position, true); };
  }, [open]);
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => { if (!inside(event.target)) setOpen(false); };
    window.addEventListener("pointerdown", outside, true);
    return () => window.removeEventListener("pointerdown", outside, true);
  }, [open]);
  return <div ref={container} className="vua-import-menu" onMouseEnter={keepOpen} onMouseLeave={leave} onBlur={event => { if (!inside(event.relatedTarget)) setOpen(false); }} onKeyDown={event => { if (event.key === "Escape" && open) { event.preventDefault(); event.stopPropagation(); setOpen(false); } }}>
    <div className="vua-import-menu__buttons" data-vua-primary-group="">
      <Button variant="primary" data-nav-id="warehouse-import" onClick={() => { setOpen(false); onSingle(); }}>{strings.importPage.localTitle}</Button>
      <Button variant="primary" aria-label={strings.importPage.importModes} aria-haspopup="menu" aria-expanded={open} onMouseEnter={() => setOpen(true)} onClick={() => setOpen(true)}><Icon name="chevron-down" size={16} /></Button>
    </div>
    {open ? createPortal(<div ref={popup} className="vua-import-menu__choices" role="menu" onMouseEnter={keepOpen} onMouseLeave={leave}><Button role="menuitem" variant="subtle" onClick={() => { setOpen(false); onMigration(); }}>{strings.importPage.migrationTitle}</Button></div>, document.body) : null}
  </div>;
}
