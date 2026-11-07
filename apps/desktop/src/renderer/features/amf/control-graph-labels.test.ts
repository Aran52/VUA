import { describe, expect, it } from "vitest";
import { placeLabels, type LabelInput } from "./control-graph-labels.ts";

const view = { width: 800, height: 600 };
const label = (x: number, y: number, extra: Partial<LabelInput> = {}): LabelInput => ({ x, y, radius: 12, width: 100, height: 18, rank: 1, required: false, optional: true, ...extra });

describe("graph captions", () => {
  it("sits below its node on whole pixels when there is room", () => {
    const [placed] = placeLabels([label(200.4, 100.6)], view);
    expect(placed).toMatchObject({ shown: true, spot: "below" });
    expect(Number.isInteger(placed!.x) && Number.isInteger(placed!.y)).toBe(true);
    expect(placed!.y).toBeGreaterThan(100.6 + 12);
  });

  it("moves to a free side when a neighbour takes the space below", () => {
    const [top, bottom] = placeLabels([label(200, 100), label(200, 140, { rank: 2 })], view);
    expect(top!.spot).not.toBe("below");
    expect(bottom!.shown).toBe(true);
  });

  it("never covers another node or caption", () => {
    const labels = Array.from({ length: 40 }, (_, i) => label(60 + (i % 8) * 90, 60 + Math.floor(i / 8) * 60, { rank: i % 3 }));
    const placed = placeLabels(labels, view);
    const boxes = placed.flatMap((spot, i) => spot.shown ? [{ ...spot, width: labels[i]!.width, height: labels[i]!.height, i }] : []);
    for (const a of boxes) {
      for (const b of boxes) if (a !== b) expect(a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height).toBe(false);
      for (const [other, node] of labels.entries()) {
        if (other === a.i) continue;
        const hit = a.x < node.x + node.radius && node.x - node.radius < a.x + a.width && a.y < node.y + node.radius && node.y - node.radius < a.y + a.height;
        expect(hit).toBe(false);
      }
    }
  });

  it("gives the more important caption the first choice", () => {
    const [outfit, recipe] = placeLabels([label(200, 100, { rank: 3 }), label(200, 118, { rank: 0, radius: 2 })], view);
    expect(recipe!.shown).toBe(true);
    expect(outfit!.shown).toBe(true);
    expect(outfit!.spot).not.toBe("below");
  });

  it("hides a caption with no clear spot until the pointer reaches it", () => {
    const crowded = [label(100, 100, { rank: 0, width: 780 }), label(100, 101, { rank: 1, width: 780 })];
    expect(placeLabels(crowded, { width: 800, height: 140 })[1]!.shown).toBe(false);
    expect(placeLabels(crowded.map((item, i) => i === 1 ? { ...item, required: true } : item), { width: 800, height: 140 })[1]!.shown).toBe(true);
  });

  it("leaves out captions that only show on demand", () => {
    expect(placeLabels([label(100, 100, { optional: false })], view)[0]!.shown).toBe(false);
  });

  it("stays inside the view", () => {
    const [edge] = placeLabels([label(10, 590)], view);
    expect(edge!.x).toBeGreaterThanOrEqual(0);
    expect(edge!.y + 18).toBeLessThanOrEqual(600);
  });
});
