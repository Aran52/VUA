/** Shared motion helpers (slice 22's rule): animations follow the resource-saving
 * mode flag on the document root, and timing tokens stay overridable by skins. */
import { useEffect, useState } from "react";

/** Reads a token from the document root, so skins set scene timing and colors in one place. */
export function token(name: string, fallback: number): number {
  const value = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name));
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

/** True while resource-saving mode is off (it sets data-effects="off" on the document). */
export function useEffectsOn(): boolean {
  const read = () => document.documentElement.dataset.effects !== "off";
  const [on, setOn] = useState(read);
  useEffect(() => {
    const observer = new MutationObserver(() => setOn(read()));
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-effects"] });
    return () => observer.disconnect();
  }, []);
  return on;
}
