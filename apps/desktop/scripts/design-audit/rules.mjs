// Design-audit rules. Pure functions over the collector snapshot; no Electron
// and no DOM here so unit tests can drive them with synthetic snapshots.
// Rule ids, severities and categories are fixed contract for baseline.json,
// the overview page and docs/design/design-audit.md.

import {
  THRESHOLDS, TYPE_SCALE, RADIUS_TOKENS, parseColor, rgbDistance, composite,
  contrastRatio, nearestTypeScale, nearestRadiusToken, nearestGridStep,
} from "./tokens.mjs";

export const RULES = {
  "type-scale": { severity: "warning", category: "current", attribution: "font" },
  "type-weight": { severity: "warning", category: "current", attribution: "font-weight" },
  "color-token": { severity: "warning", category: "current", attribution: null },
  "contrast": { severity: "error", category: "current", attribution: "color" },
  "spacing-grid": { severity: "warning", category: "current", attribution: null },
  "radius-token": { severity: "warning", category: "current", attribution: "radius" },
  "shadow-panel": { severity: "warning", category: "current", attribution: "box-shadow" },
  "blur-scope": { severity: "warning", category: "current", attribution: "backdrop-filter" },
  "one-accent": { severity: "warning", category: "current", attribution: null },
  "target-size": { severity: "error", category: "current", attribution: "size" },
  "overflow": { severity: "error", category: "current", attribution: null },
  "radius-nesting": { severity: "hint", category: "proposed", attribution: null },
  "surface-nesting": { severity: "hint", category: "proposed", attribution: null },
  "border-density": { severity: "hint", category: "proposed", attribution: null },
};

const ACTIVE_RULES_DEFAULT = Object.keys(RULES);

export function severityOf(ruleId) { return RULES[ruleId].severity; }
export function categoryOf(ruleId) { return RULES[ruleId].category; }

function makeFinding(ruleId, el, index, details, message, property = null) {
  return {
    rule: ruleId,
    severity: severityOf(ruleId),
    category: categoryOf(ruleId),
    index,
    path: el.path,
    tag: el.tag,
    cls: el.cls,
    text: el.text,
    details,
    message,
    property,
  };
}

function passSpacing(value) {
  const r = Math.abs(Math.round(value)) % 4;
  return r === 0 || r === 1 || r === 3;
}

const MARGIN_SIDES = ["top", "right", "bottom", "left"];

/** Evaluates the selected rules over one page snapshot. */
export function evaluateSnapshot(snapshot, { rules = ACTIVE_RULES_DEFAULT, exemptRules = [] } = {}) {
  const active = new Set(rules.filter(id => !exemptRules.includes(id)));
  const findings = [];
  const push = finding => { if (active.has(finding.rule)) findings.push(finding); };
  const tokens = snapshot.palette ?? [];
  const familiesSeen = { purple: [], orange: [] };

  for (const [index, el] of (snapshot.elements ?? []).entries()) {
    if (active.has("type-scale")) {
      if (el.fontSize > 0) {
        const ok = TYPE_SCALE.some(t => t.size === el.fontSize && t.lineHeight === el.lineHeight);
        if (!ok) {
          const nearest = nearestTypeScale(el.fontSize);
          push(makeFinding("type-scale", el, index, {
            fontSize: el.fontSize, lineHeight: el.lineHeight,
            nearest: nearest ? `${nearest.entry.size}/${nearest.entry.lineHeight}` : null,
            nearestToken: nearest?.entry.name ?? null,
          }, `font ${el.fontSize}px/${el.lineHeight === "normal" ? "normal" : el.lineHeight + "px"} is not a token combination`, "font"));
        }
      }
    }
    if (active.has("type-weight")) {
      if (el.fontSize > 0 && el.fontWeight !== 400 && el.fontWeight !== 600) {
        push(makeFinding("type-weight", el, index, { fontWeight: el.fontWeight },
          `font weight ${el.fontWeight} is not 400 or 600`, "font-weight"));
      }
    }
    if (active.has("color-token")) {
      const surfaces = [];
      if (el.color && el.color.a > 0.05) surfaces.push(["color", el.color, "text"]);
      if (el.backgroundColor && el.backgroundColor.a >= 0.95 && !el.hasBackgroundImage) surfaces.push(["background-color", el.backgroundColor, "background"]);
      for (const border of el.borders ?? []) {
        if (border.width >= 1 && border.style !== "none" && border.color && border.color.a > 0.05) {
          surfaces.push(["border-color", border.color, "border"]);
          break;
        }
      }
      for (const [property, color, kind] of surfaces) {
        let nearest = null;
        for (const token of tokens) {
          const d = rgbDistance(color, token.color);
          if (!nearest || d < nearest.d) nearest = { d, token };
        }
        if (!nearest || nearest.d > THRESHOLDS.colorTokenRgbDistance) {
          push(makeFinding("color-token", el, index, {
            property, kind,
            value: `rgb(${Math.round(color.r)}, ${Math.round(color.g)}, ${Math.round(color.b)})`,
            nearestToken: nearest ? `${nearest.token.name} (distance ${Math.round(nearest.d)})` : "none",
          }, `${kind} color is not close to any --vua-* token`, property));
        }
      }
    }
    if (active.has("contrast")) {
      if ((el.hasText || el.iconOnly) && el.effBg && el.color && el.color.a > 0.05 && !el.disabled) {
        const fg = composite(el.color, el.effBg);
        const large = el.fontSize >= THRESHOLDS.largeTextPx
          || (el.fontSize >= THRESHOLDS.largeTextBoldPx && el.fontWeight >= 600);
        const required = large || el.iconOnly ? THRESHOLDS.contrastLargeRatio : THRESHOLDS.contrastBodyRatio;
        const ratio = contrastRatio(fg, el.effBg);
        if (ratio < required - 0.01) {
          push(makeFinding("contrast", el, index, {
            ratio: Math.round(ratio * 100) / 100, required,
            fontSize: el.fontSize, fontWeight: el.fontWeight, icon: el.iconOnly,
          }, `contrast ${Math.round(ratio * 100) / 100}:1 < ${required}:1`, "color"));
        }
      }
    }
    if (active.has("spacing-grid")) {
      const offenders = [];
      (el.paddings ?? []).forEach((v, i) => { if (v !== 0 && !passSpacing(v)) offenders.push([`padding-${MARGIN_SIDES[i]}`, v]); });
      (el.margins ?? []).forEach((v, i) => {
        if (v === 0) return;
        // Symmetric horizontal auto-centering is a layout idiom, not spacing rhythm.
        if ((i === 1 || i === 3) && el.margins[1] === el.margins[3]) return;
        if (!passSpacing(v)) offenders.push([`margin-${MARGIN_SIDES[i]}`, v]);
      });
      for (const [kind, v] of [["row-gap", el.rowGap], ["column-gap", el.columnGap]]) {
        if (v !== 0 && !passSpacing(v)) offenders.push([kind, v]);
      }
      for (const [property, value] of offenders) {
        const nearest = nearestGridStep(Math.abs(value));
        push(makeFinding("spacing-grid", el, index, { property, value, nearest: nearest.target },
          `${property}: ${value}px is not on the 4 px grid`, property));
      }
    }
    if (active.has("radius-token")) {
      const offenders = new Map(); // value -> corner (one finding per distinct value)
      (el.radius ?? []).forEach((r, i) => {
        if (typeof r === "object" || r >= 100 || r === 0) return;
        const exact = RADIUS_TOKENS.some(t => Math.abs(t.value - r) < 0.01);
        if (!exact && !offenders.has(r)) offenders.set(r, ["top-left", "top-right", "bottom-right", "bottom-left"][i]);
      });
      for (const [value, corner] of offenders) {
        const nearest = nearestRadiusToken(value);
        push(makeFinding("radius-token", el, index, { corner, value, nearest: nearest?.token.value ?? null, nearestToken: nearest?.token.name ?? null },
          `border-radius ${corner}: ${value}px is not a --vua-radius-* value`, "radius"));
      }
    }
    if (active.has("shadow-panel")) {
      const shadow = el.boxShadow;
      if (shadow && shadow !== "none") {
        const glow = Object.values(snapshot.shadowTokens ?? {}).some(v => shadow === v);
        const elevated = shadow === (snapshot.shadowTokens ?? {})["--vua-shadow-elevated"];
        const floating = el.overlayContext || el.shellStructure
          || el.position === "fixed" || (el.position === "absolute" && el.zIndex >= 600);
        if (!floating && !glow) {
          push(makeFinding("shadow-panel", el, index, { shadow, elevatedShadow: elevated },
            elevated ? "elevated surface shadow on an ordinary panel/card" : "shadow on an ordinary panel/card", "box-shadow"));
        }
      }
    }
    if (active.has("blur-scope")) {
      if (el.backdropFilter && el.backdropFilter !== "none"
        && !el.overlayContext && !el.shellStructure && el.position !== "fixed") {
        push(makeFinding("blur-scope", el, index, { backdropFilter: el.backdropFilter },
          "backdrop-filter outside top/side/task/elevated structure", "backdrop-filter"));
      }
    }
    if (active.has("one-accent")) {
      if (el.family && el.backgroundColor && el.backgroundColor.a >= 0.95) {
        familiesSeen[el.family]?.push(index);
      }
    }
    if (active.has("target-size")) {
      if (el.isControl && !el.disabled && el.display !== "inline") {
        const { w, h } = el.rect;
        if (w < THRESHOLDS.targetFloor || h < THRESHOLDS.targetFloor) {
          push(makeFinding("target-size", el, index, { w, h, floor: THRESHOLDS.targetFloor },
            `interactive target ${w}x${h} is below the 24x24 floor`, "size"));
        } else if (el.primary && (w < THRESHOLDS.primaryFloor || h < THRESHOLDS.primaryFloor)) {
          push(makeFinding("target-size", el, index, { w, h, floor: THRESHOLDS.primaryFloor, primary: true },
            `primary action ${w}x${h} is below the 32x32 interior`, "size"));
        }
      }
    }
    if (active.has("overflow")) {
      const clipped = (el.overflowX === "hidden" || el.overflowY === "hidden")
        && !el.insideScrollContainer
        && el.textOverflow !== "ellipsis" && el.lineClamp === "none"
        && ((el.overflowX === "hidden" && el.scrollW > el.clientW + 1)
          || (el.overflowY === "hidden" && el.scrollH > el.clientH + 1))
        && (el.hasText || el.textLength > 0);
      if (clipped) {
        push(makeFinding("overflow", el, index, {
          kind: "clipped", scrollW: el.scrollW, clientW: el.clientW, scrollH: el.scrollH, clientH: el.clientH,
        }, "text is clipped without an ellipsis", null));
      }
      const { x, y, w, h } = el.rect;
      const tol = THRESHOLDS.overflowTolerance;
      // Horizontal only: vertical growth beyond the fold is normal document
      // scrolling, not layout breakage.
      const beyond = !el.insideScrollContainer
        && (x < -tol || x + w > snapshot.viewport.w + tol);
      if (beyond && w > 4 && h > 4) {
        push(makeFinding("overflow", el, index, { kind: "viewport", rect: el.rect }, "element extends horizontally beyond the viewport", null));
      }
    }
    if (active.has("radius-nesting")) {
      const own = Math.max(...(el.radius ?? [0]).map(r => (typeof r === "object" ? 999 : r)));
      const ownPct = (el.radius ?? []).some(r => typeof r === "object");
      if (!ownPct && own > 0 && own < 100 && el.ancestorRadius > 0 && !el.ancestorDecorative
        && own >= el.ancestorRadius) {
        push(makeFinding("radius-nesting", el, index, { radius: own, ancestorRadius: el.ancestorRadius },
          `radius ${own}px does not step down from ancestor ${el.ancestorRadius}px`, null));
      }
    }
    if (active.has("surface-nesting")) {
      if (el.surfaceDepth >= 3) {
        push(makeFinding("surface-nesting", el, index, { surfaceDepth: el.surfaceDepth },
          `${el.surfaceDepth} nested bordered/filled surfaces`, null));
      }
    }
  }

  if (active.has("one-accent")) {
    const families = Object.entries(familiesSeen).filter(([, list]) => list.length > 0).map(([family]) => family);
    if (families.length > 1) {
      const elementRefs = Object.entries(familiesSeen).flatMap(([family, list]) => list.map(index => {
        const el = snapshot.elements[index];
        return { family, path: el.path, cls: el.cls, rect: el.rect, text: el.text };
      }));
      findings.push({
        rule: "one-accent", severity: severityOf("one-accent"), category: categoryOf("one-accent"),
        index: null, path: null, tag: "view", cls: [], text: `families: ${families.join(" + ")}`,
        details: { families, elementRefs },
        message: `two jurisdiction accent families (${families.join(", ")}) in one view`, property: null,
      });
    }
    const primaries = (snapshot.elements ?? [])
      .map((el, i) => ({ el, i }))
      .filter(({ el }) => el.primary)
      .map(({ el, i }) => ({ index: i, path: el.path, cls: el.cls, text: el.text }));
    if (primaries.length > 1) {
      findings.push({
        rule: "one-accent", severity: severityOf("one-accent"), category: categoryOf("one-accent"),
        index: null, path: null, tag: "view", cls: [], text: `${primaries.length} filled primary buttons`,
        details: { primaries }, message: `${primaries.length} filled primary buttons in one view`, property: null,
      });
    }
  }

  return findings;
}

/** border-density is cross-view: evaluateSnapshot only records the metric. */
export function borderDensityMetric(snapshot) {
  const bordered = (snapshot.elements ?? []).filter(el => (el.borders ?? []).some(b => b.width >= 1 && b.style !== "none")).length;
  const area = (snapshot.viewport.w * snapshot.viewport.h) / 10000;
  return { bordered, viewport: snapshot.viewport, density: area > 0 ? bordered / area : 0 };
}

/** Adds border-density findings for the views above the P90 of all views. */
export function applyBorderDensity(viewMetrics, findingsByView) {
  const densities = viewMetrics.map(m => m.density).sort((a, b) => a - b);
  if (densities.length < 3) return;
  const percentile = THRESHOLDS.borderDensityPercentile / 100;
  const idx = Math.min(densities.length - 1, Math.floor(percentile * (densities.length - 1)));
  const p90 = densities[idx];
  if (p90 <= 0) return;
  for (const metric of viewMetrics) {
    if (metric.density > p90) {
      const target = findingsByView instanceof Map ? findingsByView.get(metric.key) : findingsByView[metric.key];
      if (!target) continue;
      target.push({
        rule: "border-density", severity: severityOf("border-density"), category: categoryOf("border-density"),
        index: null, path: null, tag: "view", cls: [],
        text: `density ${metric.density.toFixed(2)} > P90 ${p90.toFixed(2)}`,
        details: { density: Math.round(metric.density * 100) / 100, p90: Math.round(p90 * 100) / 100, bordered: metric.bordered },
        message: `border density ${metric.density.toFixed(2)} per 10k px² exceeds the all-view P90 (${p90.toFixed(2)})`,
        property: null,
      });
    }
  }
}
