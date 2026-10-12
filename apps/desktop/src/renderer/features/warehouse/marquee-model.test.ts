import { expect, test } from "vitest";
import { clipSelectionRect, scrollSelectionRect } from "./marquee-model.ts";
test("a stationary pointer extends selection when content scrolls, including reversal and clipping", () => {
  const anchor = { x: 50, y: 20 }; const pointer = { x: 250, y: 250 }; const viewport = { x: 100, y: 100 };
  expect(scrollSelectionRect(anchor, pointer, viewport, { x: 0, y: 0 })).toEqual({ left: 150, top: 120, width: 100, height: 130 });
  const down = scrollSelectionRect(anchor, pointer, viewport, { x: 0, y: 200 });
  expect(down).toEqual({ left: 150, top: -80, width: 100, height: 330 });
  expect(clipSelectionRect(down, { left: 100, top: 100, width: 400, height: 200 })).toEqual({ left: 150, top: 100, width: 100, height: 150 });
  expect(scrollSelectionRect(anchor, pointer, viewport, { x: 0, y: 0 }).height).toBe(130);
  expect(scrollSelectionRect({ x: 250, y: 250 }, { x: 110, y: 110 }, viewport, { x: 0, y: 0 })).toEqual({ left: 110, top: 110, width: 240, height: 240 });
});
