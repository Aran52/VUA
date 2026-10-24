// Shared token tables, color math and CSS helpers for the design audit.
// Pure functions only — unit tests import this module without Electron.

export const TYPE_SCALE = [
  { size: 12, lineHeight: 16, name: "--vua-font-caption", label: "Caption 400 12/16" },
  { size: 14, lineHeight: 20, name: "--vua-font-body", label: "Body 400 14/20" },
  { size: 20, lineHeight: 28, name: "--vua-font-title", label: "Title 600 20/28" },
  { size: 28, lineHeight: 36, name: "--vua-font-display", label: "Display 600 28/36" },
];

// --vua-radius-* values that are ordinary lengths (pill/badge are 999px capsules).
export const RADIUS_TOKENS = [
  { value: 4, name: "--vua-radius-sm" },
  { value: 6, name: "--vua-radius-control" },
  { value: 10, name: "--vua-radius-card" },
  { value: 12, name: "--vua-radius-overlay" },
];

export const SPACING_GRID = [4, 8, 12, 16, 24, 32];

// Thresholds, stated here so tests and docs quote the same numbers.
export const THRESHOLDS = {
  // A computed solid color counts as "matching a token" when its RGB Euclidean
  // distance to some resolved token color is at most this. Token pairs differ by
  // >= 14 (amber #ffd34d vs warning #fde047) so 12 keeps tokens distinguishable.
  colorTokenRgbDistance: 12,
  // Effective background search stops at the first opaque background; translucent
  // layers in between are alpha-composited in order.
  contrastBodyRatio: 4.5,
  contrastLargeRatio: 3.0,
  largeTextPx: 24,
  largeTextBoldPx: 18.66,
  // Spacing passes on multiples of 4 plus +/-1 px border compensation.
  spacingTolerance: 1,
  // Target-size floors from the standard (24 floor, 32 primary interior).
  targetFloor: 24,
  primaryFloor: 32,
  // Viewport overflow tolerance in px.
  overflowTolerance: 2,
  // Percentile for the proposed border-density rule.
  borderDensityPercentile: 90,
  // Max annotated elements per view screenshot.
  maxAnnotations: 40,
};

const INTERACTIVE_SELECTOR = [
  "button", "a[href]", "input", "select", "textarea", "summary",
  '[role="button"]', '[role="tab"]', '[role="menuitem"]', '[role="option"]',
  '[role="checkbox"]', '[role="radio"]', '[role="switch"]', '[role="link"]',
  '[tabindex]:not([tabindex="-1"])',
].join(",");

export function isInteractiveLike(el) {
  return Boolean(el.closest?.(INTERACTIVE_SELECTOR));
}

// ---- Color parsing / math ----------------------------------------------------

/** Parses computed rgb/rgba, sRGB/linear-sRGB color() and CSS hex colours. */
export function parseColor(text) {
  if (typeof text !== "string") return null;
  const value = text.trim();
  const modern = value.match(/^color\(srgb(-linear)?\s+([^)]*)\)$/i);
  if (modern) {
    const values = modern[2].split(/[\s/]+/).filter(Boolean).map(part => parseFloat(part) / (part.endsWith("%") ? 100 : 1));
    if (values.length < 3 || !values.every(Number.isFinite)) return null;
    const channel = value => {
      const clamped = Math.max(0, Math.min(1, value));
      return 255 * (modern[1] ? clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * clamped ** (1 / 2.4) - 0.055 : clamped);
    };
    return { r: channel(values[0]), g: channel(values[1]), b: channel(values[2]), a: values[3] ?? 1 };
  }
  const rgb = value.match(/^rgba?\(([^)]+)\)$/i);
  if (rgb) {
    const parts = rgb[1].split(/[\s,/]+/).filter(Boolean).map(Number);
    if (parts.length >= 3 && parts.every(n => Number.isFinite(n))) {
      return { r: parts[0], g: parts[1], b: parts[2], a: parts[3] ?? 1 };
    }
    return null;
  }
  const hex = value.match(/^#([0-9a-f]+)$/i);
  if (hex) {
    const digits = hex[1];
    const expand = d => parseInt(d.length === 1 ? d + d : d, 16);
    if (digits.length === 3 || digits.length === 4) {
      return { r: expand(digits[0]), g: expand(digits[1]), b: expand(digits[2]), a: digits.length === 4 ? expand(digits[3]) / 255 : 1 };
    }
    if (digits.length === 6 || digits.length === 8) {
      return {
        r: parseInt(digits.slice(0, 2), 16), g: parseInt(digits.slice(2, 4), 16), b: parseInt(digits.slice(4, 6), 16),
        a: digits.length === 8 ? parseInt(digits.slice(6, 8), 16) / 255 : 1,
      };
    }
  }
  return null;
}

export function rgbDistance(a, b) {
  return Math.sqrt((a.r - b.r) ** 2 + (a.g - b.g) ** 2 + (a.b - b.b) ** 2);
}

/** Composites `top` over `bottom`; both {r,g,b,a}. Returns an opaque color. */
export function composite(top, bottom) {
  const a = top.a;
  return {
    r: top.r * a + bottom.r * (1 - a),
    g: top.g * a + bottom.g * (1 - a),
    b: top.b * a + bottom.b * (1 - a),
    a: 1,
  };
}

function channelLuminance(c) {
  const v = c / 255;
  return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}

export function relativeLuminance({ r, g, b }) {
  return 0.2126 * channelLuminance(r) + 0.7152 * channelLuminance(g) + 0.0722 * channelLuminance(b);
}

export function contrastRatio(fg, bg) {
  const l1 = relativeLuminance(fg);
  const l2 = relativeLuminance(bg);
  const [hi, lo] = l1 >= l2 ? [l1, l2] : [l2, l1];
  return (hi + 0.05) / (lo + 0.05);
}

// ---- CSS specificity (for the CSSOM attribution fallback) --------------------

/** Rough specificity [a, b, c] of a complex selector; enough for cascade ordering. */
export function selectorSpecificity(selector) {
  let a = 0, b = 0, c = 0;
  let text = selector.replace(/\/\*[\s\S]*?\*\//g, "").trim();
  // Strip :not/:is/:where arguments into their inner contribution (:where = 0).
  text = text.replace(/:where\([^)]*\)/gi, "");
  const notBodies = [];
  text = text.replace(/:(not|is)\(([^()]*(?:\([^()]*\)[^()]*)*)\)/gi, (_, name, body) => {
    notBodies.push({ name: name.toLowerCase(), body });
    return "";
  });
  for (const { name, body } of notBodies) {
    const inner = selectorSpecificity(body);
    if (name === "not") { a += inner[0]; b += inner[1]; c += inner[2]; }
    else { a += inner[0]; b += inner[1]; c += inner[2]; }
  }
  const compound = text.split(/[\s>+~]+/).filter(Boolean);
  for (const part of compound) {
    let rest = part;
    // pseudo-elements count as type
    rest = rest.replace(/::[a-z-]+/gi, () => { c += 1; return ""; });
    // pseudo-classes and attribute selectors count as class
    rest = rest.replace(/:[a-z-]+(\([^)]*\))?/gi, () => { b += 1; return ""; });
    rest = rest.replace(/\[[^\]]*\]/g, () => { b += 1; return ""; });
    a += (rest.match(/#[\w-]+/g) ?? []).length;
    rest = rest.replace(/#[\w-]+/g, "");
    b += (rest.match(/\.[\w-]+/g) ?? []).length;
    rest = rest.replace(/\.[\w-]+/g, "");
    c += (rest.match(/^[a-z][\w-]*|\b[a-z][\w-]*/gi) ?? []).length;
  }
  return [a, b, c];
}

export function compareSpecificity(x, y) {
  for (const i of [0, 1, 2]) if (x[i] !== y[i]) return x[i] - y[i];
  return 0;
}

/** Nearest type-scale entry for a font size; returns the entry and the gap. */
export function nearestTypeScale(size) {
  let best = null;
  for (const entry of TYPE_SCALE) {
    const gap = Math.abs(entry.size - size);
    if (!best || gap < best.gap) best = { entry, gap };
  }
  return best;
}

export function nearestRadiusToken(value) {
  let best = null;
  for (const token of RADIUS_TOKENS) {
    const gap = Math.abs(token.value - value);
    if (!best || gap < best.gap) best = { token, gap };
  }
  return best;
}

export function nearestGridStep(value) {
  const lower = Math.floor(value / 4) * 4;
  return { value: lower, gap: value - lower, target: lower === 0 ? 4 : lower };
}
