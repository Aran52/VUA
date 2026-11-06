import { useLayoutEffect, useRef } from "react";
import { canAnimate, milliseconds, motionOff, token } from "./primitives/motion.ts";
import "./segment-bar.css";

const FILL_INSET = 2;

/** The chosen option's fill and light bar: one element that travels between options, so a change of
 * view reads as one movement. Put it first inside a `.vua-segments` group and pass the chosen key. */
export function SegmentBar({ selected }: { selected: string }) {
  const bar = useRef<HTMLSpanElement>(null);
  const last = useRef<{ x: number; width: number } | null>(null);
  useLayoutEffect(() => {
    const element = bar.current;
    const group = element?.parentElement;
    if (!element || !group) return;
    const place = (moving: boolean) => {
      const option = group.querySelector<HTMLElement>('[aria-pressed="true"], [aria-selected="true"]');
      if (!option) { element.style.opacity = "0"; last.current = null; return; }
      // The fill stays clear of the group's edge and of its neighbours, as it does above and below.
      const next = { x: option.offsetLeft + FILL_INSET, width: option.offsetWidth - 2 * FILL_INSET };
      const from = last.current;
      last.current = next;
      element.style.opacity = "1";
      element.style.width = `${next.width}px`;
      element.style.transform = `translateX(${next.x}px)`;
      if (!moving || !from || (from.x === next.x && from.width === next.width) || motionOff() || !canAnimate(element)) return;
      element.animate(
        [{ width: `${from.width}px`, transform: `translateX(${from.x}px)` }, { width: `${next.width}px`, transform: `translateX(${next.x}px)` }],
        { duration: milliseconds(token("--vua-motion-normal", "180ms"), 180), easing: token("--vua-motion-ease-enter", "ease-out") },
      );
    };
    place(true);
    // Fonts and language changes resize the options without changing the choice.
    const observer = typeof ResizeObserver === "function" ? new ResizeObserver(() => place(false)) : null;
    observer?.observe(group);
    return () => observer?.disconnect();
  }, [selected]);
  return <span ref={bar} className="vua-segment-bar" aria-hidden="true" />;
}
