import type { ReactNode } from "react";
import { Icon, type IconName } from "@vua/design-system";
import "./alert.css";

/**
 * Tier 3, alert: a bar at the top of a page or panel that speaks up without stopping work — a
 * failure with a retry, a blocked step, a condition the player should know about. It stays until
 * the condition ends or the player dismisses it. Errors use role="alert"; the rest are polite.
 */
export type AlertTone = "info" | "success" | "warning" | "error";

const ICONS: Record<AlertTone, IconName> = { info: "info", success: "check", warning: "warning", error: "error-circle" };

export function Alert({ tone = "info", title, children, action, onDismiss, dismissLabel, className }: {
  tone?: AlertTone;
  title?: ReactNode;
  children?: ReactNode;
  /** One or two buttons that resolve the condition. */
  action?: ReactNode;
  onDismiss?: () => void;
  dismissLabel?: string;
  className?: string;
}) {
  return <div className={["vua-alert", className].filter(Boolean).join(" ")} data-tone={tone} role={tone === "error" ? "alert" : "status"}>
    <Icon name={ICONS[tone]} size={20} />
    <div className="vua-alert__text">
      {title ? <strong className="vua-alert__title">{title}</strong> : null}
      {children ? <div className="vua-alert__body">{children}</div> : null}
    </div>
    {action ? <div className="vua-alert__actions">{action}</div> : null}
    {onDismiss ? <button type="button" className="vua-alert__dismiss" aria-label={dismissLabel} onClick={onDismiss}><Icon name="close" size={16} /></button> : null}
  </div>;
}
