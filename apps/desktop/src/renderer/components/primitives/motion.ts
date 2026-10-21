import { useEffect, useLayoutEffect, useState, type RefObject } from "react";

/**
 * Scripted motion for the feedback tiers (tooltip, toast, popconfirm, drawer, dialog).
 * Windows' animation setting flattens CSS motion on the development machine, so motion runs through
 * WAAPI; VUA's resource-saving mode (`data-effects="off"`) turns it off. Durations and easing are
 * design tokens, so a skin sets them.
 */
export const motionOff = () => document.documentElement.dataset.effects === "off";

export const token = (name: string, fallback: string) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;

export const milliseconds = (value: string, fallback: number) => {
  const number = Number.parseFloat(value);
  if (!Number.isFinite(number) || number < 0) return fallback;
  return value.endsWith("ms") ? number : value.endsWith("s") ? number * 1000 : number;
};

export const canAnimate = (element: HTMLElement | null): element is HTMLElement =>
  element !== null && typeof element.animate === "function";

export const fastMs = () => milliseconds(token("--vua-motion-fast", "140ms"), 140);
export const baseMs = () => milliseconds(token("--vua-motion-base", "300ms"), 300);
export const easing = () => token("--vua-motion-ease", "ease-out");

/** Whether to render an element that moves in while `open` and moves out before it unmounts. */
export function usePresenceMotion(open: boolean, ref: RefObject<HTMLElement | null>, enter: Keyframe[], exit: Keyframe[], slow = false): boolean {
  const [shown, setShown] = useState(open);
  if (open && !shown) setShown(true);

  useLayoutEffect(() => {
    if (!open || motionOff() || !canAnimate(ref.current)) return;
    const animation = ref.current.animate(enter, { duration: slow ? baseMs() : fastMs() * 1.4, easing: easing() });
    return () => animation.cancel();
    // Keyframes are constants at the call sites.
  }, [open, ref]);

  useEffect(() => {
    if (open || !shown) return;
    if (motionOff() || !canAnimate(ref.current)) { setShown(false); return; }
    const animation = ref.current.animate(exit, { duration: fastMs(), easing: "ease-in", fill: "forwards" });
    let active = true;
    const done = () => { if (active) setShown(false); };
    animation.finished.then(done, done);
    return () => { active = false; animation.cancel(); };
  }, [open, shown, ref]);

  return shown;
}
