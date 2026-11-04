import { useLayoutEffect, useState, type RefObject } from "react";

/** Keep a full-width details drawer below the row containing its card. */
export function useGridColumns(ref: RefObject<HTMLElement | null>): number {
  const [columns, setColumns] = useState(1);
  useLayoutEffect(() => {
    const grid = ref.current;
    if (!grid) return;
    const measure = () => setColumns(Math.max(1, getComputedStyle(grid).gridTemplateColumns.split(" ").filter(Boolean).length));
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(grid);
    return () => observer.disconnect();
  }, [ref]);
  return columns;
}
