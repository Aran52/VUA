export interface SelectionRect { readonly left: number; readonly top: number; readonly width: number; readonly height: number }
export interface SelectionPoint { readonly x: number; readonly y: number }
/** Anchor belongs to scroll content; the moving endpoint belongs to the screen. */
export function scrollSelectionRect(anchor: SelectionPoint, pointer: SelectionPoint, viewport: SelectionPoint, scroll: SelectionPoint): SelectionRect {
  const x = viewport.x + anchor.x - scroll.x;
  const y = viewport.y + anchor.y - scroll.y;
  return { left: Math.min(x, pointer.x), top: Math.min(y, pointer.y), width: Math.abs(pointer.x - x), height: Math.abs(pointer.y - y) };
}
export function clipSelectionRect(rect: SelectionRect, viewport: SelectionRect): SelectionRect {
  const left = Math.max(rect.left, viewport.left); const top = Math.max(rect.top, viewport.top);
  return { left, top, width: Math.max(0, Math.min(rect.left + rect.width, viewport.left + viewport.width) - left), height: Math.max(0, Math.min(rect.top + rect.height, viewport.top + viewport.height) - top) };
}
