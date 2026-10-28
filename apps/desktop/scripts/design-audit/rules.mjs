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
  "grid-shape": { severity: "warning", category: "current", attribution: "radius" },
  "shadow-panel": { severity: "warning", category: "current", attribution: "box-shadow" },
  "blur-scope": { severity: "warning", category: "current", attribution: "backdrop-filter" },
  "one-accent": { severity: "warning", category: "current", attribution: null },
  "target-size": { severity: "error", category: "current", attribution: "size" },
  "overflow": { severity: "error", category: "current", attribution: null },
  "radius-nesting": { severity: "warning", category: "current", attribution: null },
  "surface-nesting": { severity: "warning", category: "current", attribution: null },
  "border-density": { severity: "hint", category: "proposed", attribution: null },
};

const ACTIVE_RULES_DEFAULT = Object.keys(RULES);

// Layout rules cannot differ between themes: the same view's light state would
// duplicate the dark state's findings, so aggregation counts them once (dark).
export const THEME_INVARIANT_RULES = new Set([
  "spacing-grid", "type-scale", "type-weight", "radius-token", "radius-nesting",
  "surface-nesting", "target-size", "overflow", "grid-shape",
]);

export function isThemeInvariant(ruleId) {
  return THEME_INVARIANT_RULES.has(ruleId);
}

/** Why this snapshot is not auditable content, or null when it is usable. */
export function classifyCapture(snapshot, perStateElementFloor, { requireFoundation = false, requestedTheme = null } = {}) {
  if (snapshot.errorOverlay) return "vite error overlay (page did not render)";
  if (requestedTheme && snapshot.theme !== requestedTheme) return `requested theme ${requestedTheme}, collected ${snapshot.theme || "no theme"}`;
  if (requireFoundation && (!snapshot.tokenRaw?.["--vua-font-body"] || !snapshot.palette?.some(token => token.name === "--vua-text-strong"))) {
    return "shared design foundation did not load";
  }
  if ((snapshot.elementCount ?? 0) < perStateElementFloor) {
    return `only ${snapshot.elementCount} elements collected (floor ${perStateElementFloor})`;
  }
  return null;
}

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
    rect: el.paintedRect ?? el.rect,
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

// Zero-blur inset bars and concentric rings draw selection/focus/binding edges.
// Offset cast shadows still count, even when their blur is zero.
export function isLineShadow(value) {
  if (!value || value === "none") return false;
  const shadows = value.replace(/(?:rgba?|hsla?|color)\([^)]*\)|#[\da-f]+/gi, "color").split(",");
  return shadows.every(shadow => {
    const lengths = [...shadow.matchAll(/(-?(?:\d*\.)?\d+)(?:px)?\b/g)].map(match => Number(match[1]));
    return lengths.length >= 2 && (lengths[2] ?? 0) === 0
      && (/\binset\b/.test(shadow) || (lengths[0] === 0 && lengths[1] === 0));
  });
}

/** Evaluates the selected rules over one page snapshot. */
export function evaluateSnapshot(snapshot, { rules = ACTIVE_RULES_DEFAULT, exemptRules = [] } = {}) {
  const active = new Set(rules.filter(id => !exemptRules.includes(id)));
  const findings = [];
  const push = finding => { if (active.has(finding.rule)) findings.push(finding); };
  const tokens = snapshot.palette ?? [];
  const familiesSeen = { purple: [], orange: [] };
  const indicators = (snapshot.elements ?? []).filter(el => el.selfSurface && !el.hasText && el.rect.h <= 8);

  for (const [index, el] of (snapshot.elements ?? []).entries()) {
    const typographyRelevant = el.hasText || el.isTextField || el.tag === "select" || el.tag === "textarea";
    if (active.has("type-scale") && typographyRelevant) {
      if (el.fontSize > 0) {
        const ok = TYPE_SCALE.some(t => t.size === el.fontSize && (t.lineHeight === el.lineHeight || el.nativeLineHeight));
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
    if (active.has("type-weight") && typographyRelevant) {
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
        if (el.autoMargins?.[i]) return;
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
        const exact = [...RADIUS_TOKENS.map(t => t.value), ...Object.values(snapshot.radiusTokens ?? {})].some(value => Math.abs(value - r) < 0.01);
        if (!exact && !offenders.has(r)) offenders.set(r, ["top-left", "top-right", "bottom-right", "bottom-left"][i]);
      });
      for (const [value, corner] of offenders) {
        const nearest = nearestRadiusToken(value);
        push(makeFinding("radius-token", el, index, { corner, value, nearest: nearest?.token.value ?? null, nearestToken: nearest?.token.name ?? null },
          `border-radius ${corner}: ${value}px is not a --vua-radius-* value`, "radius"));
      }
    }
    if (active.has("grid-shape") && el.shapeRole && el.shapeRole !== "none") {
      // Author ruling 2026-10-28: rectangles and rounded rectangles only, one radius per role.
      const pairs = el.cornerPairs ?? [];
      const cut = (el.cornerShapes ?? []).some(shape => shape && !/^(round|superellipse\(1\))$/.test(shape))
        || /polygon\(/.test(el.clipPath ?? "");
      const expected = el.shapeExpectedRadius;
      const uneven = typeof expected === "number" && pairs.some(pair => pair.some(value => Math.abs(value - expected) > 0.1));
      if (cut || uneven) {
        push(makeFinding("grid-shape", el, index, { role: el.shapeRole, expectedRadius: expected, cornerPairs: pairs, cornerShapes: el.cornerShapes, clipPath: el.clipPath },
          cut ? "Controls are rectangles or rounded rectangles; cut corners are not used" : `The ${el.shapeRole} role uses one ${expected}px radius on every corner`, "radius"));
      }
    }
    if (active.has("shadow-panel")) {
      const shadow = el.boxShadow;
      if (shadow && shadow !== "none" && !isLineShadow(shadow)) {
        const glow = Object.entries(snapshot.shadowTokens ?? {}).some(([name, value]) => name.startsWith("--vua-glow-") && shadow === value);
        const elevated = shadow === (snapshot.shadowTokens ?? {})["--vua-shadow-elevated"];
        const actionMaterial = el.tag === "button" && el.cls.includes("vua-button--primary")
          && shadow === (snapshot.shadowTokens ?? {})["--vua-action-shadow"];
        // A theme may declare a card material (Jirai's soft shadow, author ruling 2026-10-28).
        const cardMaterial = shadow === (snapshot.shadowTokens ?? {})["--vua-card-material"];
        const floating = el.overlayContext || el.shellStructure
          || el.position === "fixed" || (el.position === "absolute" && el.zIndex >= 600);
        if (!floating && !glow && !actionMaterial && !cardMaterial) {
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
      if (el.isControl && !el.indicatorRole && !el.disabled && el.family && el.backgroundColor && el.backgroundColor.a >= 0.95) {
        familiesSeen[el.family]?.push(index);
      }
    }
    if (active.has("target-size")) {
      if (el.isControl && !el.disabled && el.display !== "inline") {
        const { w, h } = el.hitRect ?? el.rect;
        if (w < THRESHOLDS.targetFloor || h < THRESHOLDS.targetFloor) {
          push(makeFinding("target-size", el, index, { w, h, floor: THRESHOLDS.targetFloor },
            `interactive target ${w}x${h} is below the 24x24 floor`, "size"));
        } else if (el.primary && !el.isChoice && !el.indicatorRole && (w < THRESHOLDS.primaryFloor || h < THRESHOLDS.primaryFloor)) {
          push(makeFinding("target-size", el, index, { w, h, floor: THRESHOLDS.primaryFloor, primary: true },
            `primary action ${w}x${h} is below the 32x32 interior`, "size"));
        }
      }
    }
    if (active.has("overflow")) {
      const clipped = (el.overflowX === "hidden" || el.overflowY === "hidden")
        && el.textOverflow !== "ellipsis" && el.lineClamp === "none"
        && ((el.overflowX === "hidden" && el.scrollW > el.clientW + 1)
          || (el.overflowY === "hidden" && el.scrollH > el.clientH + 1))
        && (el.hasText || el.textLength > 0);
      if (clipped) {
        push(makeFinding("overflow", el, index, {
          kind: "clipped", scrollW: el.scrollW, clientW: el.clientW, scrollH: el.scrollH, clientH: el.clientH,
        }, "text is clipped without an ellipsis", null));
      }
      if (el.clippedByAncestor && el.hasText && !el.intentionalTruncation && !clipped) {
        push(makeFinding("overflow", el, index, { kind: "ancestor_clip", rect: el.rect, paintedRect: el.paintedRect }, "text is clipped by an ancestor without an ellipsis", null));
      }
      const { x, y, w, h } = el.paintedRect ?? el.rect;
      const tol = THRESHOLDS.overflowTolerance;
      // Horizontal only: vertical growth beyond the fold is normal document
      // scrolling, not layout breakage.
      const beyond = !(el.insideScrollX ?? el.insideScrollContainer)
        && (x < -tol || x + w > snapshot.viewport.w + tol);
      if (beyond && w > 4 && h > 4) {
        push(makeFinding("overflow", el, index, { kind: "viewport", rect: el.rect }, "element extends horizontally beyond the viewport", null));
      }
    }
    if (active.has("radius-nesting")) {
      const own = Math.max(...(el.radius ?? [0]).map(r => (typeof r === "object" ? 999 : r)));
      const ownPct = (el.radius ?? []).some(r => typeof r === "object");
      if (!ownPct && own > 0 && own < 100 && el.ancestorRadius > 0 && !el.ancestorDecorative
        && own > el.ancestorRadius) {
        push(makeFinding("radius-nesting", el, index, { radius: own, ancestorRadius: el.ancestorRadius },
          `radius ${own}px is larger than ancestor ${el.ancestorRadius}px`, null));
      }
    }
    if (active.has("surface-nesting")) {
      // Report only the over-deep surface itself — text and icons inside one
      // would repeat the same finding once per element. SVG frames are not
      // surfaces.
      const indicatorDepth = indicators.filter(marker => marker.path.length <= el.path.length && marker.path.every((part, i) => el.path[i] === part)).length;
      const surfaceDepth = el.surfaceDepth - indicatorDepth;
      if (el.tag !== "svg" && el.selfSurface && !indicators.includes(el) && surfaceDepth >= 3) {
        push(makeFinding("surface-nesting", el, index, { surfaceDepth },
          `${surfaceDepth} nested bordered/filled surfaces`, null));
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
    const commands = new Map();
    for (const [index, el] of (snapshot.elements ?? []).entries()) {
      if (!el.primary || el.isChoice || el.indicatorRole || el.disabled) continue;
      const scope = el.actionScope ?? "page";
      const key = `${scope}|${el.primaryGroup ?? `element-${index}`}`;
      if (!commands.has(key)) commands.set(key, { index, path: el.path, cls: el.cls, text: el.text, scope });
    }
    const primariesByScope = Object.groupBy([...commands.values()], primary => primary.scope);
    for (const primaries of Object.values(primariesByScope).filter(items => items.length > 1)) {
      findings.push({
        rule: "one-accent", severity: severityOf("one-accent"), category: categoryOf("one-accent"),
        index: null, path: null, tag: "view", cls: [], text: `${primaries.length} filled primary buttons`,
        details: { primaries }, message: `${primaries.length} filled primary buttons in one action scope`, property: null,
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
