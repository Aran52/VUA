/**
 * Where each caption of the Recipe graph sits. Captions keep one size at every zoom and land on whole
 * pixels, so text stays crisp. Each tries below, right, left and above its node and takes the first spot
 * clear of nodes and of captions already placed; one with no spot waits for the pointer or keyboard.
 */
export interface LabelBox { readonly x: number; readonly y: number; readonly width: number; readonly height: number }
export interface LabelInput {
  /** Node centre and outline radius, in view pixels. */
  readonly x: number; readonly y: number; readonly radius: number;
  /** Measured caption size. */
  readonly width: number; readonly height: number;
  /** Lower goes first. */
  readonly rank: number;
  /** Shown even without a clear spot, because the pointer, the keyboard or the selection is on it. */
  readonly required: boolean;
  /** Takes a spot only when one is clear of everything; off means it waits for the pointer. */
  readonly optional: boolean;
}
export type LabelSpot = "below" | "right" | "left" | "above";
export interface PlacedLabel { readonly x: number; readonly y: number; readonly shown: boolean; readonly spot: LabelSpot }

const GAP = 4;
const SPOTS: readonly LabelSpot[] = ["below", "right", "left", "above"];

function boxFor(label: LabelInput, spot: LabelSpot): LabelBox {
  const { x, y, radius: r, width, height } = label;
  const box = spot === "below" ? { x: x - width / 2, y: y + r + GAP }
    : spot === "right" ? { x: x + r + GAP + 2, y: y - height / 2 }
    : spot === "left" ? { x: x - r - GAP - 2 - width, y: y - height / 2 }
    : { x: x - width / 2, y: y - r - GAP - height };
  return { x: Math.round(box.x), y: Math.round(box.y), width, height };
}

const overlaps = (a: LabelBox, b: LabelBox): boolean =>
  a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;

export function placeLabels(labels: readonly LabelInput[], view: { readonly width: number; readonly height: number }): readonly PlacedLabel[] {
  const nodes = labels.map(label => ({ x: label.x - label.radius - 2, y: label.y - label.radius - 2, width: label.radius * 2 + 4, height: label.radius * 2 + 4 }));
  const placed: LabelBox[] = [];
  const result: PlacedLabel[] = labels.map(() => ({ x: 0, y: 0, shown: false, spot: "below" }));
  const order = labels.map((_, index) => index).sort((a, b) => Number(labels[b]!.required) - Number(labels[a]!.required) || labels[a]!.rank - labels[b]!.rank || a - b);
  for (const index of order) {
    const label = labels[index]!;
    if (!label.required && !label.optional) continue;
    const clear = (box: LabelBox) => box.x >= 0 && box.y >= 0 && box.x + box.width <= view.width && box.y + box.height <= view.height
      && !nodes.some((node, other) => other !== index && overlaps(box, node)) && !placed.some(other => overlaps(box, other));
    const spot = SPOTS.find(candidate => clear(boxFor(label, candidate)));
    if (spot === undefined && !label.required) continue;
    const chosen = spot ?? "below";
    const box = boxFor(label, chosen);
    placed.push(box);
    result[index] = { x: box.x, y: box.y, shown: true, spot: chosen };
  }
  return result;
}
