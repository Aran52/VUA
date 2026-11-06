import { useEffect, useState, type ReactNode } from "react";

/** A loading line that stays away for a moment, so a quick load shows no flash of text. */
export function DelayedStatus({ children, delayMs = 400 }: { children: ReactNode; delayMs?: number }) {
  const [shown, setShown] = useState(false);
  useEffect(() => {
    const timer = setTimeout(() => setShown(true), delayMs);
    return () => clearTimeout(timer);
  }, [delayMs]);
  return shown ? <p role="status">{children}</p> : null;
}
