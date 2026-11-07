import { useLayoutEffect, useRef, type RefObject } from "react";
import { canAnimate, enterEasing, fastMs, motionOff, normalMs } from "./primitives/motion.ts";

const CURRENT = '.vua-shell__sidebar-item[aria-current="page"]';

interface Stop { readonly item: HTMLElement; readonly top: number; readonly scope: string; readonly background: string; readonly shadow: string }

/** The sidebar's highlight travels from the item the player left to the one they chose, and changes colour on the
 * way when the two groups differ. A short-lived copy of the highlight makes the trip, so it looks the same whatever
 * the theme draws for it; while it travels, neither item draws its own. Items sit in positioned groups, so the trip
 * is measured against the sidebar itself. */
export function useSidebarGlide(sidebar: RefObject<HTMLElement | null>, current: string, scope: string): void {
  const last = useRef<Stop | null>(null);
  useLayoutEffect(() => {
    const root = sidebar.current;
    const item = root?.querySelector<HTMLElement>(CURRENT) ?? null;
    if (!root || !item) { last.current = null; return; }
    const from = last.current;
    const left = from?.item !== item && from?.scope === scope ? from.item : null;
    // Before anything reads layout, or the two items' colour transitions begin: the highlight moves, it does not fade.
    if (left) left.style.transition = `color ${fastMs()}ms ease`;
    item.style.transition = "none";
    const frame = root.getBoundingClientRect(), box = item.getBoundingClientRect();
    const top = box.top - frame.top - root.clientTop + root.scrollTop;
    const style = getComputedStyle(item);
    const look = { background: style.backgroundColor, shadow: style.boxShadow };
    last.current = { item, top, scope, ...look };
    const settle = () => {
      item.style.transition = "none";
      delete item.dataset.arriving;
      void item.offsetHeight;
      item.style.transition = "";
      if (left) left.style.transition = "";
    };
    if (!from || from.scope !== scope || from.top === top || motionOff() || !canAnimate(item)) { void item.offsetHeight; settle(); return; }
    const glide = document.createElement("span");
    glide.className = "vua-shell__sidebar-glide";
    glide.setAttribute("aria-hidden", "true");
    Object.assign(glide.style, {
      left: `${box.left - frame.left - root.clientLeft + root.scrollLeft}px`, top: `${top}px`, width: `${box.width}px`, height: `${box.height}px`,
      background: style.background, border: style.border, borderRadius: style.borderRadius, boxShadow: style.boxShadow, clipPath: style.clipPath,
    });
    root.appendChild(glide);
    item.dataset.arriving = "";
    void item.offsetHeight;
    item.style.transition = "";
    const animation = glide.animate(
      [{ transform: `translateY(${from.top - top}px)`, backgroundColor: from.background, boxShadow: from.shadow }, { transform: "none", backgroundColor: look.background, boxShadow: look.shadow }],
      { duration: normalMs(), easing: enterEasing() },
    );
    // The item takes over the highlight in the same frame, with no fade between the two.
    const finish = () => { settle(); glide.remove(); };
    animation.finished.then(finish, finish);
    return () => animation.cancel();
  }, [sidebar, current, scope]);
}
