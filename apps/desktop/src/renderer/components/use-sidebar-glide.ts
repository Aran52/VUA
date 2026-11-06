import { useLayoutEffect, useRef, type RefObject } from "react";
import { canAnimate, enterEasing, motionOff, normalMs } from "./primitives/motion.ts";

const CURRENT = '.vua-shell__sidebar-item[aria-current="page"]';

/** The sidebar's highlight travels from the item the player left to the one they chose. A short-lived
 * copy of the highlight makes the trip, so it looks the same whatever the theme draws for it. */
export function useSidebarGlide(sidebar: RefObject<HTMLElement | null>, current: string, scope: string): void {
  const last = useRef<{ top: number; scope: string } | null>(null);
  useLayoutEffect(() => {
    const root = sidebar.current;
    const item = root?.querySelector<HTMLElement>(CURRENT) ?? null;
    if (!root || !item) { last.current = null; return; }
    const top = item.offsetTop;
    const from = last.current;
    last.current = { top, scope };
    if (!from || from.scope !== scope || from.top === top || motionOff() || !canAnimate(item)) return;
    // The item's own colour transition has only just begun, so read its final look with transitions off.
    item.style.transition = "none";
    const style = getComputedStyle(item);
    const glide = document.createElement("span");
    glide.className = "vua-shell__sidebar-glide";
    glide.setAttribute("aria-hidden", "true");
    Object.assign(glide.style, {
      left: `${item.offsetLeft}px`, top: `${top}px`, width: `${item.offsetWidth}px`, height: `${item.offsetHeight}px`,
      background: style.background, border: style.border, borderRadius: style.borderRadius, boxShadow: style.boxShadow, clipPath: style.clipPath,
    });
    root.appendChild(glide);
    item.dataset.arriving = "";
    void item.offsetHeight;
    item.style.transition = "";
    const animation = glide.animate(
      [{ transform: `translateY(${from.top - top}px)` }, { transform: "none" }],
      { duration: normalMs(), easing: enterEasing() },
    );
    // The item takes over the highlight in the same frame, with no fade between the two.
    const finish = () => {
      item.style.transition = "none";
      delete item.dataset.arriving;
      void item.offsetHeight;
      item.style.transition = "";
      glide.remove();
    };
    animation.finished.then(finish, finish);
    return () => animation.cancel();
  }, [sidebar, current, scope]);
}
