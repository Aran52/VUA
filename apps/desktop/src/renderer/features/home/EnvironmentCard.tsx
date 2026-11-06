import type { CSSProperties, ReactNode } from "react";
import { Icon, type IconName } from "@vua/design-system";
import { strings, format } from "../../i18n/index.ts";
import { RouteGlyph } from "../../components/RouteTile.tsx";
import type { VrBrand } from "../../components/vr-brand-glyphs.ts";
export type EnvironmentAction = "prepare" | "start" | "stop" | "busy" | "attention" | "unknown";
export function ActionGlyph({ action, spinning = false }: { action: EnvironmentAction; spinning?: boolean }) {
  return <svg className={spinning ? "vua-environment-card__spin" : undefined} viewBox="0 0 32 32" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {spinning ? <path d="M27 16a11 11 0 1 1-8-10.6" /> : action === "start" ? <path d="m12 7 14 9-14 9Z" fill="currentColor" stroke="none" /> : action === "stop" ? <path d="m9 9 14 14M23 9 9 23" /> : action === "prepare" ? <path d="M16 6v20M6 16h20" /> : action === "attention" || action === "unknown" ? <><path d="m16 4 13 23H3Z" /><path d="M16 12v7M16 23v.1" /></> : <path d="M27 16a11 11 0 1 1-8-10.6" />}
  </svg>;
}
/** Two sibling buttons; neither half is nested inside the other control. */
export function EnvironmentCard({ title, titleContent, hint, kind, brand, icon, logo, id, action, status, actionHint, selected, onDetails, detailsOpen = true, onAction, disabled, spinning, warning, developing, actionText, primary, actionIcon, progress, phase, children }: {
  title: string; titleContent?: ReactNode;
  /** One line on what this is, for a player who has not met it yet. */
  hint?: string | undefined;
  kind?: "screen" | "headset" | "unity" | undefined; brand?: VrBrand | undefined; icon?: IconName;
  /** The item's own logo, shown in place of the glyph. */
  logo?: ReactNode; id: string; action: EnvironmentAction; status: string;
  actionHint?: string | undefined;
  selected?: boolean; onDetails?: () => void;
  /** The left half opens details here, so it shows a caret; false when it leads to another page. */
  detailsOpen?: boolean;
  onAction?: () => void; disabled?: boolean; spinning?: boolean; warning?: boolean; developing?: boolean;
  /** Words in place of a glyph and status, for an action that only opens this card's details. */
  actionText?: string | undefined;
  /** This half is the card's next step; it reads as the page's main action. */
  primary?: boolean | undefined;
  /** An icon that names the next step better than the action glyph. */
  actionIcon?: IconName | undefined;
  /** Whole-percent progress of the running step, drawn behind the action. */
  progress?: number | null | undefined;
  /** Which step the action takes, for theme and test hooks. */
  phase?: string | undefined;
  children?: ReactNode;
}) {
  const copy = strings.environmentCards;
  const label = action === "prepare" ? copy.prepare : action === "start" ? copy.start : action === "stop" ? copy.stop : copy.details;
  const actionLabel = actionHint ? actionHint.includes(title) ? actionHint : `${title} · ${actionHint}` : format(label, { name: title });
  const actionButton = <button type="button" className="vua-environment-card__action" data-nav-id={`${id}-action`} data-primary={primary || undefined} data-phase={phase} disabled={disabled || developing} aria-label={actionLabel} aria-busy={spinning} onClick={onAction}
    style={progress === null || progress === undefined ? undefined : { "--vua-card-progress": `${progress}%` } as CSSProperties}>
    {developing || actionText ? <span className="vua-environment-card__developing">{actionText ?? strings.journey.developing}</span> : <>{actionIcon && !spinning ? <Icon name={actionIcon} size={20} /> : <ActionGlyph action={action} spinning={!!spinning} />}{warning && action === "stop" ? <span className="vua-environment-card__warning"><ActionGlyph action="attention" /></span> : null}<span role="status">{status}</span></>}
  </button>;
  return <article className="vua-environment-card" data-card={id} data-action={action} data-selected={selected}>
    <button type="button" className="vua-environment-card__details" data-nav-id={id} disabled={!onDetails} aria-label={format(copy.details, { name: title })} aria-expanded={onDetails ? !!selected : undefined} aria-controls={onDetails ? `${id}-details` : undefined} onClick={onDetails}>
      {logo ?? <RouteGlyph kind={kind} brand={brand} icon={icon} />}<span className="vua-environment-card__name"><strong>{titleContent ?? title}</strong>{hint ? <small title={hint}>{hint}</small> : null}</span>{onDetails && detailsOpen ? <span className="vua-environment-card__caret" aria-hidden="true"><Icon name="caret-down" size={16} /></span> : null}{children}
    </button>
    {actionButton}
  </article>;
}
