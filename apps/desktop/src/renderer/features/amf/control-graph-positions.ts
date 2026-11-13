/** Where the player's Recipe graph nodes last settled, so the graph opens the way it was left. */
import { storageKeys } from "../../app/storage-keys.ts";

const MAX_NODES = 500;
type Position = { readonly x: number; readonly y: number };

export function readGraphPositions(): readonly (readonly [string, Position])[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(storageKeys.recipeGraphPositions) ?? "null");
    if (typeof value !== "object" || value === null || Array.isArray(value)) return [];
    return Object.entries(value as Record<string, unknown>).flatMap(([id, point]): (readonly [string, Position])[] =>
      Array.isArray(point) && typeof point[0] === "number" && Number.isFinite(point[0]) && typeof point[1] === "number" && Number.isFinite(point[1]) ? [[id, { x: point[0], y: point[1] }]] : []);
  } catch { return []; }
}

export function rememberGraphPositions(positions: ReadonlyMap<string, Position>): void {
  try {
    const entries = [...positions].slice(-MAX_NODES).map(([id, point]) => [id, [Math.round(point.x), Math.round(point.y)]] as const);
    localStorage.setItem(storageKeys.recipeGraphPositions, JSON.stringify(Object.fromEntries(entries)));
  } catch { /* Positions apply to this visit. */ }
}
