// Card-style inventory: finds per-page card components in renderer CSS, reads
// their visual declarations, and diffs them against the shared Card primitive
// (.vua-card) so the report can say which are directly replaceable.

import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { parseCssBlocks } from "./static-scan.mjs";

async function walkFiles(dir, out = []) {
  let entries;
  try { entries = await readdir(dir, { withFileTypes: true }); } catch { return out; }
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) await walkFiles(full, out);
    else out.push(full);
  }
  return out;
}

/** Reads --vua-* values from the design system tokens (dark theme defaults). */
export async function tokenValueMap(tokensCssPath) {
  const css = strip(await readFile(tokensCssPath, "utf8"));
  const map = {};
  const rootBlock = css.match(/:root\s*\{([\s\S]*?)\n\}/);
  for (const decl of (rootBlock?.[1] ?? "").split(";")) {
    const m = decl.match(/(--vua-[\w-]+)\s*:\s*([^;]+)/);
    if (m) map[m[1]] = m[2].trim();
  }
  return map;
}

function strip(css) {
  return css.replace(/\/\*[\s\S]*?\*\//g, "");
}

function resolveTokenValue(value, tokens) {
  return value.replace(/var\((--vua-[\w-]+)\)/g, (whole, name) => tokens[name] ?? whole);
}

function parsePx(value) {
  const m = value.match(/(-?\d+(?:\.\d+)?)px/);
  return m ? parseFloat(m[1]) : null;
}

const CARD_CLASS = /\.([\w-]*card[\w-]*)/gi;

/** Scans renderer CSS for card-like components and their key declarations. */
export async function scanCardStyles(rendererDir, tokensPath, repoRoot) {
  const tokens = await tokenValueMap(tokensPath);
  const primitives = new Map(); // base class -> declarations (from card.css)
  const cards = new Map();

  const propertyOf = (body, names) => {
    for (const decl of body.split(";")) {
      const colon = decl.indexOf(":");
      if (colon < 0) continue;
      const name = decl.slice(0, colon).trim().toLowerCase();
      if (names.includes(name)) return decl.slice(colon + 1).trim();
    }
    return null;
  };

  for (const file of await walkFiles(rendererDir)) {
    if (!file.endsWith(".css")) continue;
    const css = strip(await readFile(file, "utf8"));
    for (const block of parseCssBlocks(css)) {
      const matches = [...block.selector.matchAll(/\.([\w-]+)/g)].map(m => m[1]);
      const cardClass = matches.find(name => /card/i.test(name));
      if (!cardClass) continue;
      const isPrimitive = file.endsWith(`${path.sep}card.css`);
      const base = cardClass.split("__")[0].split("--")[0];
      const record = {
        name: base,
        fullClass: cardClass,
        file,
        radius: propertyOf(block.body, ["border-radius"]),
        padding: propertyOf(block.body, ["padding"]),
        borderWidth: propertyOf(block.body, ["border-width", "border"]),
        borderColor: propertyOf(block.body, ["border-color"]),
        border: propertyOf(block.body, ["border"]),
        background: propertyOf(block.body, ["background", "background-color"]),
        shadow: propertyOf(block.body, ["box-shadow"]),
        backdrop: propertyOf(block.body, ["backdrop-filter"]),
        selector: block.selector,
        atRules: block.atRules,
      };
      const target = isPrimitive ? primitives : cards;
      const key = `${base} @ ${file}`;
      if (!target.has(key)) target.set(key, { name: base, file, selectors: [], props: [] });
      const entry = target.get(key);
      entry.selectors.push(block.selector);
      entry.props.push(record);
    }
  }

  const primitive = [...primitives.values()][0] ?? null;
  const primitiveProps = primitive?.props?.find(p => p.name && !p.fullClass.includes("--")) ?? null;
  const cardBaseline = {
    radius: primitiveProps?.radius ?? "var(--vua-radius-card)",
    padding: primitiveProps?.padding ?? "var(--vua-space-4)",
    border: primitiveProps?.border ?? null,
    background: primitiveProps?.background ?? null,
    shadow: primitiveProps?.shadow ?? null,
    backdrop: primitiveProps?.backdrop ?? null,
  };

  const results = [];
  for (const entry of cards.values()) {
    // Only the base rule block (selector is exactly the class) speaks for the component.
    const baseBlock = entry.props.find(p => p.selector.trim() === `.${p.name}`)
      ?? entry.props.find(p => p.radius || p.padding || p.background || p.border || p.shadow);
    const pick = value => (value ? resolveTokenValue(value, tokens) : null);
    const radius = pick(baseBlock?.radius);
    const padding = pick(baseBlock?.padding);
    const border = pick(baseBlock?.border ?? baseBlock?.borderWidth);
    const background = pick(baseBlock?.background);
    const shadow = pick(baseBlock?.shadow);
    const backdrop = pick(baseBlock?.backdrop);
    const radiusPx = parsePx(radius ?? "") ?? (radius?.includes("%") ? 50 : radius?.includes("999") ? 999 : null);
    const shadowNone = !shadow || shadow === "none";
    const matchesCard = radiusPx === 10 && shadowNone && !backdrop;
    const replaceable = matchesCard && (!border || border.includes("var(--vua-border)"));
    results.push({
      name: entry.name,
      file: path.relative(repoRoot ?? rendererDir, entry.file).replace(/\\/g, "/"),
      selectorCount: entry.selectors.length,
      selectors: entry.selectors.slice(0, 4),
      radius, padding, border, background, shadow, backdrop,
      radiusPx,
      suggestion: replaceable
        ? "directly replaceable with Card"
        : radiusPx === 10 && shadowNone
          ? "close to Card; align padding/border, then replace"
          : "keep as a variant; record the token it should use",
    });
  }
  results.sort((a, b) => a.name.localeCompare(b.name));
  // The same class name can appear in several files (a defining file plus
  // selectors that reuse it); keep the most-defined entry and note the rest.
  const byName = new Map();
  for (const card of results) {
    const defined = [card.radius, card.padding, card.border, card.background, card.shadow, card.backdrop].filter(Boolean).length;
    const existing = byName.get(card.name);
    if (!existing) { byName.set(card.name, card); continue; }
    const existingDefined = [existing.radius, existing.padding, existing.border, existing.background, existing.shadow, existing.backdrop].filter(Boolean).length;
    if (defined > existingDefined) {
      card.alsoIn = [...(existing.alsoIn ?? []), existing.file];
      byName.set(card.name, card);
    } else {
      existing.alsoIn = [...(existing.alsoIn ?? []), card.file];
    }
  }
  return { cardBaseline, cards: [...byName.values()].sort((a, b) => a.name.localeCompare(b.name)), primitiveFile: primitive?.file ?? null };
}

/** Counts files importing the shared Card primitive. */
export async function countCardImports(rendererDir) {
  const files = await walkFiles(rendererDir, []);
  const importing = [];
  for (const file of files) {
    if (!file.endsWith(".tsx") && !file.endsWith(".ts")) continue;
    const text = await readFile(file, "utf8");
    if (text.includes("primitives/Card") || text.includes("primitives\\Card")) {
      importing.push(file);
    }
  }
  return importing;
}
