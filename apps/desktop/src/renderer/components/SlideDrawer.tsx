import { useEffect, useRef, type ReactNode } from "react";
import { motionOff, normalMs } from "./primitives/motion.ts";
import "./slide-drawer.css";

/** A panel that slides open across the full row below its trigger. Its children stay mounted while
 * it is closed, so work they watch keeps running; a closed panel is inert and hidden from assistive
 * technology. The motion follows the theme and stops when effects are off or motion is reduced. */
export function SlideDrawer({ open, id, label, className, children }: {
  open: boolean; id?: string; label?: string; className?: string; children: ReactNode;
}) {
  const panel = useRef<HTMLElement>(null);
  const wasOpen = useRef(open);
  useEffect(() => {
    const opening = open && !wasOpen.current;
    wasOpen.current = open;
    if (!opening) return;
    const timer = window.setTimeout(() => { panel.current?.scrollIntoView?.({ block: "nearest", behavior: motionOff() ? "auto" : "smooth" }); }, normalMs());
    return () => window.clearTimeout(timer);
  }, [open]);
  return <section ref={panel} id={id} className={className ? `vua-slide-drawer ${className}` : "vua-slide-drawer"} data-open={open}
    aria-label={label} aria-hidden={!open} inert={!open}>
    <div className="vua-slide-drawer__inner">{children}</div>
  </section>;
}
