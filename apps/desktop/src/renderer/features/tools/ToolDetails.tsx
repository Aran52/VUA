import { createContext, useContext, type ReactNode } from "react";
import { createPortal } from "react-dom";

/** Cards keep their live connection and device state while their details occupy the shared drawer. */
export const ToolDetailsContext = createContext<{ host: HTMLDivElement | null; shown: string } | null>(null);

export function ToolDetails({ id, title, selected, className = "", children }: {
  id: string; title: string; selected: boolean; className?: string; children: ReactNode;
}) {
  const placement = useContext(ToolDetailsContext);
  const panel = <section id={`tool-${id}-details`} className={`vua-environment-detail ${className}`}
    hidden={placement ? placement.shown !== id : !selected} aria-label={title}>{children}</section>;
  return placement ? placement.host ? createPortal(panel, placement.host) : null : panel;
}
