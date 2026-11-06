import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react";

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
export const normalMs = () => milliseconds(token("--vua-motion-normal", "180ms"), 180);
export const slowMs = () => milliseconds(token("--vua-motion-slow", "360ms"), 360);
export const enterEasing = () => token("--vua-motion-ease-enter", "ease-out");
export const exitEasing = () => token("--vua-motion-ease-exit", "ease-in");

/** A menu or popover: a short slide from its anchor as it appears, a fade as it leaves. */
export const POPOVER_ENTER: Keyframe[] = [{ opacity: 0, transform: "translateY(-4px)" }, { opacity: 1, transform: "none" }];
export const POPOVER_EXIT: Keyframe[] = [{ opacity: 1 }, { opacity: 0 }];

/** Plays once each time `key` changes to a non-null value, for things that only need an entrance. */
export function useEntranceMotion(key: unknown, ref: RefObject<HTMLElement | null>, keyframes: Keyframe[], duration: () => number = normalMs): void {
  useLayoutEffect(() => {
    if (key === null || key === undefined || key === false || motionOff() || !canAnimate(ref.current)) return;
    const animation = ref.current.animate(keyframes, { duration: duration(), easing: enterEasing() });
    return () => animation.cancel();
    // Keyframes and duration are constants at the call sites.
  }, [key, ref]);
}

/** Content that changes in place fades in, without remounting it and losing its state. */
export function useSwapFade(key: unknown, ref: RefObject<HTMLElement | null>): void {
  const first = useRef(true);
  useLayoutEffect(() => {
    if (first.current) { first.current = false; return; }
    if (motionOff() || !canAnimate(ref.current)) return;
    const animation = ref.current.animate([{ opacity: 0 }, { opacity: 1 }], { duration: normalMs(), easing: enterEasing() });
    return () => animation.cancel();
  }, [key, ref]);
}

/** A block that opens and closes in the flow, moving its neighbours with it. Returns whether to render it. */
export function useCollapseMotion(open: boolean, ref: RefObject<HTMLElement | null>): boolean {
  const [shown, setShown] = useState(open);
  if (open && !shown) setShown(true);
  useLayoutEffect(() => {
    if (!open || motionOff() || !canAnimate(ref.current)) return;
    const height = ref.current.offsetHeight;
    const animation = ref.current.animate([{ height: "0px", opacity: 0 }, { height: `${height}px`, opacity: 1 }], { duration: normalMs(), easing: enterEasing() });
    return () => animation.cancel();
  }, [open, ref]);
  useEffect(() => {
    if (open || !shown) return;
    if (motionOff() || !canAnimate(ref.current)) { setShown(false); return; }
    const height = ref.current.offsetHeight;
    const animation = ref.current.animate([{ height: `${height}px`, opacity: 1 }, { height: "0px", opacity: 0 }], { duration: normalMs() * 2 / 3, easing: exitEasing(), fill: "forwards" });
    let active = true;
    const done = () => { if (active) setShown(false); };
    animation.finished.then(done, done);
    return () => { active = false; animation.cancel(); };
  }, [open, shown, ref]);
  return shown;
}

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
