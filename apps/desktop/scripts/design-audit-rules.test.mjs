// Unit tests for the design-audit rule engine, grouping/ranking and baseline
// gate. Pure node:test — no Electron, no network. Synthetic snapshots mirror
// the collector output shape.

import { test } from "node:test";
import assert from "node:assert/strict";
import { evaluateSnapshot, borderDensityMetric, applyBorderDensity, RULES, isThemeInvariant, THEME_INVARIANT_RULES, classifyCapture } from "./design-audit/rules.mjs";
import { groupFindings, checkAgainstBaseline, findingKey } from "./design-audit/baseline.mjs";
import { rankRootCauses, suggestForRootCause } from "./design-audit/report.mjs";
import { parseColor, composite, contrastRatio, rgbDistance, selectorSpecificity, compareSpecificity, nearestTypeScale } from "./design-audit/tokens.mjs";
import { candidatesFor, normalizeFile } from "./design-audit/attributor.mjs";
import { parseCssBlocks } from "./design-audit/static-scan.mjs";

const PALETTE = [
  { name: "--vua-text-strong", color: { r: 245, g: 243, b: 255, a: 1 } },
  { name: "--vua-bg-panel", color: { r: 21, g: 19, b: 34, a: 1 } },
  { name: "--vua-border", color: { r: 46, g: 42, b: 69, a: 1 } },
  { name: "--vua-accent", color: { r: 167, g: 139, b: 250, a: 1 } },
];

function el(overrides = {}) {
  return {
    path: [1, 0], tag: "div", id: "", cls: ["test-el"], rect: { x: 10, y: 10, w: 100, h: 40 },
    text: "hello", textLength: 5,
    fontSize: 14, lineHeight: 20, fontWeight: 400,
    color: { r: 245, g: 243, b: 255, a: 1 },
    backgroundColor: { r: 21, g: 19, b: 34, a: 1 },
    hasBackgroundImage: false,
    boxShadow: "none", backdropFilter: "none",
    borders: [{ width: 0, style: "none", color: null }, { width: 0, style: "none", color: null }, { width: 0, style: "none", color: null }, { width: 0, style: "none", color: null }],
    radius: [0, 0, 0, 0],
    paddings: [16, 16, 16, 16], margins: [0, 0, 0, 0], rowGap: 8, columnGap: 8,
    overflowX: "visible", overflowY: "visible", textOverflow: "clip", lineClamp: "none",
    scrollW: 100, clientW: 100, scrollH: 40, clientH: 40,
    display: "block", position: "static", zIndex: 0, opacity: 1,
    inlineStyle: false,
    interactive: false, isControl: false, disabled: false, primary: false,
    family: null,
    overlayContext: false, shellStructure: false, insideScrollContainer: false,
    effBg: { r: 21, g: 19, b: 34, a: 1 }, effBgFromCanvas: false,
    iconOnly: false, hasText: true,
    ancestorRadius: 0, ancestorDecorative: false, surfaceDepth: 1,
    ...overrides,
  };
}

function snapshot(elements = [], extra = {}) {
  return {
    viewport: { w: 1280, h: 860 },
    palette: PALETTE,
    shadowTokens: {
      "--vua-shadow-elevated": "0 8px 28px rgba(0,0,0,0.5)",
      "--vua-glow-accent": "0 0 18px rgba(167,139,250,0.45)",
      "--vua-glow-accent-soft": "0 0 28px rgba(167,139,250,0.22)",
    },
    elements,
    ...extra,
  };
}

// ---- rule verdicts -----------------------------------------------------------

test("type-scale accepts token combinations and flags off-scale sizes", () => {
  const ok = evaluateSnapshot(snapshot([el()]));
  assert.equal(ok.filter(f => f.rule === "type-scale").length, 0);
  const off = evaluateSnapshot(snapshot([el({ fontSize: 13, lineHeight: 18 }), el({ fontSize: 14, lineHeight: 20 })]));
  const flagged = off.filter(f => f.rule === "type-scale");
  assert.equal(flagged.length, 1);
  assert.equal(flagged[0].details.fontSize, 13);
});

test("type-scale flags 'normal' line height", () => {
  const findings = evaluateSnapshot(snapshot([el({ lineHeight: "normal" })]));
  assert.equal(findings.filter(f => f.rule === "type-scale").length, 1);
});

test("type-weight flags weights outside 400/600", () => {
  const findings = evaluateSnapshot(snapshot([el({ fontWeight: 500 }), el({ fontWeight: 600 }), el({ fontWeight: 400 })]));
  assert.equal(findings.filter(f => f.rule === "type-weight").length, 1);
});

test("color-token flags hardcoded colors and accepts token colors", () => {
  const findings = evaluateSnapshot(snapshot([
    el({ backgroundColor: { r: 255, g: 255, b: 255, a: 1 } }), // white is far from every dark token
    el({ color: { r: 245, g: 243, b: 255, a: 1 } }),
  ]));
  const flagged = findings.filter(f => f.rule === "color-token");
  assert.equal(flagged.length, 1);
  assert.equal(flagged[0].details.kind, "background");
});

test("color-token ignores translucent backgrounds (glass surfaces)", () => {
  const findings = evaluateSnapshot(snapshot([el({ backgroundColor: { r: 255, g: 0, b: 0, a: 0.5 } })]));
  assert.equal(findings.filter(f => f.rule === "color-token").length, 0);
});

test("contrast fails below 4.5:1 for body text and passes large text at 3:1", () => {
  const low = evaluateSnapshot(snapshot([el({ color: { r: 80, g: 80, b: 90, a: 1 } })]));
  const contrastFinding = low.find(f => f.rule === "contrast");
  assert.ok(contrastFinding);
  assert.equal(contrastFinding.details.required, 4.5);
  const big = evaluateSnapshot(snapshot([el({ color: { r: 80, g: 80, b: 90, a: 1 }, fontSize: 28, lineHeight: 36 })]));
  const bigFinding = big.find(f => f.rule === "contrast");
  if (bigFinding) assert.equal(bigFinding.details.required, 3);
});

test("contrast skips disabled controls", () => {
  const findings = evaluateSnapshot(snapshot([el({ color: { r: 80, g: 80, b: 90, a: 1 }, disabled: true })]));
  assert.equal(findings.filter(f => f.rule === "contrast").length, 0);
});

test("spacing-grid passes multiples of 4 and border compensation, flags 6/10/14", () => {
  const findings = evaluateSnapshot(snapshot([
    el({ paddings: [12, 13, 16, 11], margins: [0, 0, 0, 0], rowGap: 8, columnGap: 8 }), // 13/11 = +1 border compensation
    el({ paddings: [16, 16, 16, 16], margins: [0, 6, 0, 0], rowGap: 10, columnGap: 8 }),
  ]));
  const flagged = findings.filter(f => f.rule === "spacing-grid");
  assert.equal(flagged.length, 2); // margin-right 6 and row-gap 10
  assert.deepEqual(flagged.map(f => f.details.property).sort(), ["margin-right", "row-gap"]);
});

test("spacing-grid exempts symmetric horizontal centering margins", () => {
  const findings = evaluateSnapshot(snapshot([el({ margins: [0, 33, 0, 33] })]));
  assert.equal(findings.filter(f => f.rule === "spacing-grid").length, 0);
});

test("radius-token accepts 0, tokens, pill and circles", () => {
  const findings = evaluateSnapshot(snapshot([
    el({ radius: [0, 6, 10, 12] }),
    el({ radius: [999, 0, 0, 0] }),
    el({ radius: [{ pct: 50 }, 0, 0, 0] }),
  ]));
  assert.equal(findings.filter(f => f.rule === "radius-token").length, 0);
  const off = evaluateSnapshot(snapshot([el({ radius: [8, 8, 8, 8] })]));
  const flagged = off.filter(f => f.rule === "radius-token");
  assert.equal(flagged.length, 1);
  assert.ok(flagged[0].details.nearest === 6 || flagged[0].details.nearest === 10);
});

test("shadow-panel flags ordinary panels but not glow, elevated tokens or floating layers", () => {
  const glow = evaluateSnapshot(snapshot([el({ boxShadow: "0 0 18px rgba(167,139,250,0.45)" })]));
  assert.equal(glow.filter(f => f.rule === "shadow-panel").length, 0);
  const shadow = evaluateSnapshot(snapshot([el({ boxShadow: "0 4px 12px rgba(0,0,0,0.4)" })]));
  assert.equal(shadow.filter(f => f.rule === "shadow-panel").length, 1);
  const floating = evaluateSnapshot(snapshot([el({ boxShadow: "0 4px 12px rgba(0,0,0,0.4)", overlayContext: true })]));
  assert.equal(floating.filter(f => f.rule === "shadow-panel").length, 0);
});

test("blur-scope flags card walls, exempts shell structure and overlays", () => {
  const wall = evaluateSnapshot(snapshot([el({ backdropFilter: "blur(10px)" })]));
  assert.equal(wall.filter(f => f.rule === "blur-scope").length, 1);
  const shell = evaluateSnapshot(snapshot([el({ backdropFilter: "blur(10px)", shellStructure: true })]));
  assert.equal(shell.filter(f => f.rule === "blur-scope").length, 0);
});

test("one-accent flags mixed jurisdiction families and multiple filled primaries", () => {
  const mixed = evaluateSnapshot(snapshot([
    el({ backgroundColor: { r: 167, g: 139, b: 250, a: 1 }, family: "purple", interactive: true, isControl: true }),
    el({ backgroundColor: { r: 255, g: 122, b: 69, a: 1 }, family: "orange", interactive: true, isControl: true }),
  ]));
  const oneAccent = mixed.filter(f => f.rule === "one-accent");
  assert.equal(oneAccent.length, 1);
  assert.deepEqual(oneAccent[0].details.families.sort(), ["orange", "purple"]);
  const primaries = evaluateSnapshot(snapshot([
    el({ backgroundColor: { r: 167, g: 139, b: 250, a: 1 }, primary: true, interactive: true, isControl: true }),
    el({ backgroundColor: { r: 167, g: 139, b: 250, a: 1 }, primary: true, interactive: true, isControl: true }),
  ]));
  assert.equal(primaries.filter(f => f.rule === "one-accent").length, 1);
});

test("one-accent is exemptable per view (factory map)", () => {
  const mixed = snapshot([
    el({ backgroundColor: { r: 167, g: 139, b: 250, a: 1 }, family: "purple" }),
    el({ backgroundColor: { r: 255, g: 122, b: 69, a: 1 }, family: "orange" }),
  ]);
  assert.equal(evaluateSnapshot(mixed, { exemptRules: ["one-accent"] }).filter(f => f.rule === "one-accent").length, 0);
});

test("target-size flags sub-24 controls as errors and small primaries as warnings", () => {
  const tiny = evaluateSnapshot(snapshot([el({ isControl: true, rect: { x: 0, y: 0, w: 18, h: 18 } })]));
  const tinyFinding = tiny.find(f => f.rule === "target-size");
  assert.equal(tinyFinding.severity, "error");
  const primary = evaluateSnapshot(snapshot([el({ isControl: true, primary: true, rect: { x: 0, y: 0, w: 28, h: 28 } })]));
  const primaryFinding = primary.find(f => f.rule === "target-size");
  assert.equal(primaryFinding.severity, "error");
  assert.equal(primaryFinding.details.primary, true);
  assert.equal(primaryFinding.details.floor, 32);
});

test("overflow flags hidden clipping without ellipsis, exempts ellipsis and line-clamp", () => {
  const clipped = evaluateSnapshot(snapshot([el({ overflowX: "hidden", scrollW: 220, clientW: 100 })]));
  assert.equal(clipped.filter(f => f.rule === "overflow").length, 1);
  const ellipsis = evaluateSnapshot(snapshot([el({ overflowX: "hidden", scrollW: 220, clientW: 100, textOverflow: "ellipsis" })]));
  assert.equal(ellipsis.filter(f => f.rule === "overflow").length, 0);
  const clamp = evaluateSnapshot(snapshot([el({ overflowY: "hidden", scrollH: 220, clientH: 40, lineClamp: "2" })]));
  assert.equal(clamp.filter(f => f.rule === "overflow").length, 0);
  const scrolled = evaluateSnapshot(snapshot([el({ overflowX: "hidden", scrollW: 220, clientW: 100, insideScrollContainer: true })]));
  assert.equal(scrolled.filter(f => f.rule === "overflow").length, 0);
});

test("overflow flags elements beyond the viewport", () => {
  const beyond = evaluateSnapshot(snapshot([el({ rect: { x: 1200, y: 10, w: 200, h: 40 } })]));
  const flagged = beyond.filter(f => f.rule === "overflow");
  assert.equal(flagged.length, 1);
  assert.equal(flagged[0].details.kind, "viewport");
});

test("radius-nesting flags equal-or-larger child radius, exempts pills/circles", () => {
  const nested = evaluateSnapshot(snapshot([el({ radius: [10, 10, 10, 10], ancestorRadius: 10 })]));
  assert.equal(nested.filter(f => f.rule === "radius-nesting").length, 1);
  const pill = evaluateSnapshot(snapshot([el({ radius: [999, 999, 999, 999], ancestorRadius: 10 })]));
  assert.equal(pill.filter(f => f.rule === "radius-nesting").length, 0);
  const stepped = evaluateSnapshot(snapshot([el({ radius: [6, 6, 6, 6], ancestorRadius: 10 })]));
  assert.equal(stepped.filter(f => f.rule === "radius-nesting").length, 0);
});

test("surface-nesting reports only self-surfaces, once each; svg and inner text never", () => {
  // A deep surface itself: flagged once.
  const deep = evaluateSnapshot(snapshot([el({ selfSurface: true, surfaceDepth: 3 })]));
  const deepFindings = deep.filter(f => f.rule === "surface-nesting");
  assert.equal(deepFindings.length, 1);
  // Text and icons inside that surface: depth repeats but they are not surfaces.
  const inner = evaluateSnapshot(snapshot([
    el({ cls: ["inner-span"], selfSurface: false, surfaceDepth: 4 }),
    el({ cls: ["inner-strong"], selfSurface: false, surfaceDepth: 4 }),
    el({ cls: ["inner-svg"], tag: "svg", selfSurface: false, surfaceDepth: 4 }),
    el({ cls: ["svg-with-bg"], tag: "svg", selfSurface: true, surfaceDepth: 4 }),
  ]));
  assert.equal(inner.filter(f => f.rule === "surface-nesting").length, 0);
  // A shallow self-surface passes.
  const shallow = evaluateSnapshot(snapshot([el({ selfSurface: true, surfaceDepth: 2 })]));
  assert.equal(shallow.filter(f => f.rule === "surface-nesting").length, 0);
});

test("theme-invariant classification covers the layout rules only", () => {
  for (const rule of ["spacing-grid", "type-scale", "type-weight", "radius-token", "radius-nesting", "surface-nesting", "target-size", "overflow"]) {
    assert.ok(isThemeInvariant(rule), `${rule} is theme-invariant`);
  }
  for (const rule of ["color-token", "contrast", "shadow-panel", "one-accent", "border-density"]) {
    assert.ok(!isThemeInvariant(rule), `${rule} is theme-dependent`);
  }
  assert.equal(THEME_INVARIANT_RULES.size, 8);
});

test("capture failures: vite error overlay and near-empty pages are unusable", () => {
  assert.match(classifyCapture({ errorOverlay: true, elementCount: 3 }, 6), /vite error overlay/);
  assert.match(classifyCapture({ errorOverlay: false, elementCount: 3 }, 6), /floor 6/);
  assert.equal(classifyCapture({ errorOverlay: false, elementCount: 10 }, 6), null, "sparsest real view passes");
  assert.equal(classifyCapture({ errorOverlay: false, elementCount: 28 }, 6), null);
});

test("theme-invariant findings aggregate once: the light state drops layout rules", () => {
  // Mirrors the runner's aggregation: dark keeps everything, light drops
  // theme-invariant rules so per-view totals count layout findings once.
  const darkFindings = [finding("spacing-grid", ".a @ f.css"), finding("color-token", ".c @ g.css", "warning")];
  const lightFindings = [finding("spacing-grid", ".a @ f.css"), finding("color-token", ".c @ g.css", "warning"), finding("contrast", ".d @ h.css", "error")];
  const aggregated = [
    ...darkFindings,
    ...lightFindings.filter(f => !isThemeInvariant(f.rule)),
  ];
  const spacing = aggregated.filter(f => f.rule === "spacing-grid");
  assert.equal(spacing.length, 1, "layout finding counted once across themes");
  const color = aggregated.filter(f => f.rule === "color-token");
  assert.equal(color.length, 2, "color findings counted per theme");
  assert.equal(aggregated.filter(f => f.rule === "contrast").length, 1, "light-only contrast kept");
});

test("border-density fires above the cross-view P90", () => {
  const metrics = [
    { key: "a|zh-CN|dark", density: 1, bordered: 8 },
    { key: "b|zh-CN|dark", density: 2, bordered: 16 },
    { key: "c|zh-CN|dark", density: 3, bordered: 24 },
    { key: "d|zh-CN|dark", density: 9, bordered: 72 },
  ];
  const findingsByView = new Map([
    ["a|zh-CN|dark", []], ["b|zh-CN|dark", []], ["c|zh-CN|dark", []], ["d|zh-CN|dark", []],
  ]);
  applyBorderDensity(metrics, findingsByView);
  assert.equal(findingsByView.get("d|zh-CN|dark").filter(f => f.rule === "border-density").length, 1);
  assert.equal(findingsByView.get("a|zh-CN|dark").filter(f => f.rule === "border-density").length, 0);
});

test("borderDensityMetric counts bordered elements per viewport area", () => {
  const metric = borderDensityMetric(snapshot([el({ borders: [{ width: 1, style: "solid", color: PALETTE[2].color }, { width: 0, style: "none", color: null }, { width: 0, style: "none", color: null }, { width: 0, style: "none", color: null }] })]));
  assert.equal(metric.bordered, 1);
  assert.ok(metric.density > 0);
});

// ---- grouping, ranking, baseline ---------------------------------------------

function finding(rule, rootCause, severity = "warning") {
  return { rule, severity, category: RULES[rule]?.category ?? "current", path: [1], tag: "div", cls: ["x"], text: "", details: {}, message: rule, rootCause };
}

test("groupFindings merges by rule + root cause and keeps examples", () => {
  const groups = groupFindings("map", [finding("type-scale", ".a @ f.css"), finding("type-scale", ".a @ f.css"), finding("type-scale", ".b @ f.css")]);
  assert.equal(groups.length, 2);
  const first = groups.find(g => g.rootCause === ".a @ f.css");
  assert.equal(first.elements.length, 2);
  assert.equal(first.examples.length, 2);
});

test("rankRootCauses ranks by instances and counts views", () => {
  const groups = [
    { ...groupFindings("map", [finding("type-scale", ".a @ f.css"), finding("type-scale", ".a @ f.css")])[0], area: "workshop", locale: "zh-CN", theme: "dark" },
    { ...groupFindings("home", [finding("type-scale", ".a @ f.css")])[0], area: "first-run", locale: "zh-CN", theme: "dark" },
    { ...groupFindings("home", [finding("color-token", ".c @ g.css")])[0], area: "first-run", locale: "zh-CN", theme: "dark" },
  ];
  const ranked = rankRootCauses(groups);
  assert.equal(ranked[0].rootCause, ".a @ f.css");
  assert.equal(ranked[0].instances, 3);
  assert.equal(ranked[0].viewCount, 2);
});

test("suggestForRootCause maps example details to tokens", () => {
  assert.equal(suggestForRootCause({ rule: "type-scale", exampleElement: { details: { nearestToken: "--vua-font-body", nearest: "14/20" } } }), "use --vua-font-body (14/20)");
  assert.equal(suggestForRootCause({ rule: "shadow-panel", exampleElement: { details: {} } }), "remove the shadow (§4: no shadow on ordinary panels/cards)");
});

test("baseline check flags new groups and reports resolved keys", () => {
  const groups = [groupFindings("map", [finding("type-scale", ".a @ f.css")])[0]];
  const baseline = { entries: { [findingKey("map", { rule: "type-scale", rootCause: ".old @ gone.css" })]: { instances: 4, severity: "warning" } } };
  const outcome = checkAgainstBaseline(groups, baseline);
  assert.equal(outcome.baselineMissing, false);
  assert.equal(outcome.newFindings.length, 1);
  assert.equal(outcome.resolvedKeys.length, 1);
  const missing = checkAgainstBaseline(groups, null);
  assert.equal(missing.baselineMissing, true);
});

// ---- color math ---------------------------------------------------------------

test("color math: parse, distance, composite and WCAG ratio", () => {
  assert.deepEqual(parseColor("#a78bfa"), { r: 167, g: 139, b: 250, a: 1 });
  assert.deepEqual(parseColor("rgb(255 0 0 / 0.5)"), { r: 255, g: 0, b: 0, a: 0.5 });
  assert.equal(parseColor("not-a-color"), null);
  const mixed = composite({ r: 255, g: 255, b: 255, a: 0.5 }, { r: 0, g: 0, b: 0, a: 1 });
  assert.equal(Math.round(mixed.r), 128);
  assert.equal(contrastRatio({ r: 255, g: 255, b: 255 }, { r: 0, g: 0, b: 0 }), 21);
  assert.ok(rgbDistance({ r: 0, g: 0, b: 0 }, { r: 3, g: 4, b: 0 }) === 5);
});

test("selector specificity orders id > class > type", () => {
  assert.deepEqual(selectorSpecificity("#a .b c"), [1, 1, 1]);
  assert.deepEqual(selectorSpecificity(".a:hover"), [0, 2, 0]);
  assert.deepEqual(selectorSpecificity("::before"), [0, 0, 1]);
  assert.ok(compareSpecificity([1, 0, 0], [0, 9, 9]) > 0);
  assert.equal(compareSpecificity([0, 1, 0], [0, 1, 0]), 0);
});

test("nearestTypeScale picks the closest step", () => {
  assert.equal(nearestTypeScale(13).entry.size, 12);
  assert.equal(nearestTypeScale(26).entry.size, 28);
});

// ---- attribution helpers -------------------------------------------------------

test("candidatesFor maps physical properties to shorthand and logical forms", () => {
  assert.deepEqual(candidatesFor("padding-bottom"), ["padding-bottom", "padding", "padding-block", "padding-inline", "padding-block-end"]);
  assert.ok(candidatesFor("color").includes("color"));
  assert.ok(candidatesFor("radius").includes("border-radius"));
});

test("normalizeFile reduces absolute paths to repo-relative form", () => {
  assert.equal(normalizeFile("C:/repo/apps/desktop/src/renderer/app-shell.css"), "apps/desktop/src/renderer/app-shell.css");
  assert.equal(normalizeFile("C:/repo/packages/design-system/src/base.css"), "packages/design-system/src/base.css");
  assert.equal(normalizeFile("unknown"), "unknown");
});

// ---- static CSS parser ---------------------------------------------------------

test("parseCssBlocks reads nested rules with at-rule context and line numbers", () => {
  const css = [
    ".a { color: red; }",
    "@media (min-width: 100px) {",
    "  .b { border-radius: 8px; }",
    "}",
  ].join("\n");
  const blocks = parseCssBlocks(css);
  assert.equal(blocks.length, 2);
  assert.equal(blocks[0].selector, ".a");
  assert.equal(blocks[1].selector, ".b");
  assert.equal(blocks[1].atRules.length, 1);
  assert.equal(blocks[1].line, 3);
});
