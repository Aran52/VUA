import { useEffect, useRef, useState } from "react";
import { Icon } from "@vua/design-system";
import { Button } from "../../components/primitives/Button.tsx";
import { strings } from "../../i18n/index.ts";
import "./import-menu.css";
export function LocalImportButton({ onSingle, onMigration }: { onSingle: () => void; onMigration: () => void }) {
  const [open, setOpen] = useState(false); const container = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => { if (event.target instanceof Node && !container.current?.contains(event.target)) setOpen(false); };
    window.addEventListener("pointerdown", outside, true);
    return () => window.removeEventListener("pointerdown", outside, true);
  }, [open]);
  return <div ref={container} className="vua-import-menu" onMouseLeave={() => setOpen(false)} onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false); }} onKeyDown={event => { if (event.key === "Escape" && open) { event.preventDefault(); event.stopPropagation(); setOpen(false); } }}>
    <div className="vua-import-menu__buttons">
      <Button variant="primary" data-nav-id="warehouse-import" onClick={() => { setOpen(false); onSingle(); }}>{strings.importPage.localTitle}</Button>
      <Button variant="primary" aria-label={strings.importPage.importModes} aria-haspopup="menu" aria-expanded={open} onMouseEnter={() => setOpen(true)} onClick={() => setOpen(true)}><Icon name="chevron-down" size={16} /></Button>
    </div>
    {open ? <div className="vua-import-menu__choices" role="menu"><Button role="menuitem" variant="subtle" onClick={() => { setOpen(false); onMigration(); }}>{strings.importPage.migrationTitle}</Button></div> : null}
  </div>;
}
