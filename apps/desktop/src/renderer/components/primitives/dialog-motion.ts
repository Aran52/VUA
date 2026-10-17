import { useEffect, useLayoutEffect, useState, type RefObject } from "react";

/**
 * Dialogs open and close with scripted motion, so Windows' animation setting, which flattens CSS
 * motion, leaves them moving; VUA's resource-saving mode (`data-effects="off"`) turns it off.
 * Durations and easing are design tokens, so a skin sets them.
 */
const motionOff = () => document.documentElement.dataset.effects === "off";
const token = (name: string, fallback: string) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
const milliseconds = (value: string, fallback: number) => {
  const number = Number.parseFloat(value);
  if (!Number.isFinite(number) || number < 0) return fallback;
  return value.endsWith("ms") ? number : value.endsWith("s") ? number * 1000 : number;
};
const canAnimate = (element: HTMLElement | null): element is HTMLElement =>
  element !== null && typeof element.animate === "function";

/** Whether to render the dialog: it stays on screen while it moves out after `open` turns false. */
export function useDialogMotion(open: boolean, overlay: RefObject<HTMLElement | null>, panel: RefObject<HTMLElement | null>): boolean {
  const [shown, setShown] = useState(open);
  if (open && !shown) setShown(true);

  useLayoutEffect(() => {
    if (!open || motionOff() || !canAnimate(panel.current)) return;
    const duration = milliseconds(token("--vua-motion-base", "300ms"), 300);
    const easing = token("--vua-motion-ease", "ease-out");
    const fade = overlay.current?.animate([{ opacity: 0 }, { opacity: 1 }], { duration, easing });
    const rise = panel.current.animate(
      [{ opacity: 0, transform: "translateY(14px) scale(0.97)" }, { opacity: 1, transform: "none" }],
      { duration, easing },
    );
    return () => { fade?.cancel(); rise.cancel(); };
  }, [open, overlay, panel]);

  useEffect(() => {
    if (open || !shown) return;
    if (motionOff() || !canAnimate(panel.current)) { setShown(false); return; }
    const duration = milliseconds(token("--vua-motion-fast", "140ms"), 140);
    const options = { duration, easing: "ease-in", fill: "forwards" as const };
    const fade = overlay.current?.animate([{ opacity: 1 }, { opacity: 0 }], options);
    const sink = panel.current.animate(
      [{ opacity: 1, transform: "none" }, { opacity: 0, transform: "translateY(8px) scale(0.98)" }],
      options,
    );
    let active = true;
    const done = () => { if (active) setShown(false); };
    sink.finished.then(done, done);
    // Opening again while moving out keeps the dialog and drops the exit.
    return () => { active = false; fade?.cancel(); sink.cancel(); };
  }, [open, shown, overlay, panel]);

  return shown;
}
