// Report generation: per-run summary.md, the overview index.html, and the
// root-cause ranking. Pure string assembly — no Electron here.

import path from "node:path";
import { THRESHOLDS } from "./tokens.mjs";

/** Aggregates grouped findings across views into ranked root causes. */
export function rankRootCauses(groups) {
  const map = new Map();
  for (const group of groups) {
    const key = `${group.rule}|${group.rootCause}`;
    let entry = map.get(key);
    if (!entry) {
      entry = {
        rule: group.rule,
        severity: group.severity,
        rootCause: group.rootCause,
        views: new Set(),
        instances: 0,
        exampleView: group.view,
        exampleElement: group.examples[0] ?? null,
      };
      map.set(key, entry);
    }
    entry.views.add(group.view);
    entry.instances += group.elements.length;
  }
  return [...map.values()]
    .map(entry => ({ ...entry, viewCount: entry.views.size, views: [...entry.views].sort() }))
    .sort((a, b) => b.instances - a.instances || b.viewCount - a.viewCount);
}

export function tallyGroups(groups) {
  const byRule = {};
  const bySeverity = {};
  const byArea = {};
  const byCategory = {};
  for (const group of groups) {
    const n = group.elements.length;
    byRule[group.rule] = (byRule[group.rule] ?? 0) + n;
    bySeverity[group.severity] = (bySeverity[group.severity] ?? 0) + n;
    byArea[group.area] = (byArea[group.area] ?? 0) + n;
    byCategory[group.category] = (byCategory[group.category] ?? 0) + n;
  }
  return { byRule, bySeverity, byArea, byCategory };
}

function mdTable(headers, rows) {
  const head = `| ${headers.join(" | ")} |`;
  const sep = `| ${headers.map(() => "---").join(" | ")} |`;
  const body = rows.map(row => `| ${row.map(cell => String(cell).replaceAll("|", "\\|")).join(" | ")} |`);
  return [head, sep, ...body].join("\n");
}

/** Data-driven fix suggestion for a root-cause entry. */
export function suggestForRootCause(rc) {
  const details = rc.exampleElement?.details ?? {};
  switch (rc.rule) {
    case "type-scale":
      return details.nearestToken
        ? `use ${details.nearestToken} (${details.nearest})`
        : "map to --vua-font-* tokens";
    case "type-weight": return "use weight 400 or 600";
    case "color-token":
      return details.nearestToken ? `use ${details.nearestToken}` : "map to a --vua-* color token";
    case "contrast":
      return `raise to ${details.required}:1 (switch to a text token or stronger surface)`;
    case "spacing-grid":
      return details.nearest ? `move to ${details.nearest}px (4 px grid)` : "use --vua-space-* steps";
    case "radius-token":
      return details.nearestToken ? `use ${details.nearestToken} (${details.nearest}px)` : "use --vua-radius-* tokens";
    case "shadow-panel": return "remove the shadow (§4: no shadow on ordinary panels/cards)";
    case "blur-scope": return "limit backdrop-filter to top/side/task/elevated structure";
    case "one-accent": return "keep one jurisdiction accent per view (§4)";
    case "target-size": return `grow the control to at least ${details.floor}px`;
    case "overflow": return "let the container size itself, scroll, or use an intentional ellipsis";
    case "radius-nesting": return "keep the inner corner no larger than its container";
    case "surface-nesting": return "flatten nested bordered/filled containers";
    case "border-density": return "reduce bordered elements; separate with spacing/hierarchy (proposed rule)";
    default: return "review against design-standard §4";
  }
}

const RULE_SEVERITY = {
  "type-scale": "warning", "type-weight": "warning", "color-token": "warning",
  contrast: "error", "spacing-grid": "warning", "radius-token": "warning",
  "shadow-panel": "warning", "blur-scope": "warning", "one-accent": "warning",
  "target-size": "error", overflow: "error", "radius-nesting": "warning",
  "surface-nesting": "warning", "border-density": "hint",
};

/** Builds summary.md content from the run data. */
export function buildSummary(data) {
  const {
    meta, viewResults, groups, rootCauses, staticScan, cards, cardImports,
    uncovered, rulesMeta, checkOutcome, collectionFailures = [], coverage, errors = [],
  } = data;
  const lines = [];
  const push = (...xs) => lines.push(...xs);

  push(`# VUA design audit — ${meta.date}`, "");
  push(`Tool: \`pnpm audit:design\` at commit \`${meta.commit ?? "(local)"}\`. Machine output stays in the ignored run directory (screenshots, annotated screenshots, snapshots, per-view JSON, \`baseline.json\`). A reviewed, sanitized comparison may be committed under \`docs/research/\`.`, "");
  push(`## Scope`, "");
  push(`- Views: ${meta.viewCount} (${meta.viewLocaleStates} view/locale/theme states; ${meta.elementTotal.toLocaleString("en-US")} elements inspected).`);
  if (coverage) {
    push(`- Collection: **${meta.usable ? "COMPLETE" : "FAILED"}**; ${coverage.audited}/${coverage.planned} planned states audited; ${coverage.executionFailures} execution failures, ${coverage.captureFailures} capture failures.`);
    if (coverage.missing.length || coverage.duplicates.length || coverage.unexpected.length) {
      push(`- Accounting errors: ${coverage.missing.length} missing, ${coverage.duplicates.length} duplicate, ${coverage.unexpected.length} unexpected states.`);
    }
  }
  if (meta.requestedScope) {
    push(`- Scope: ${meta.filtered ? "filtered run; completion applies to the selected plan" : "full registered plan"}.`);
    for (const pass of meta.requestedScope) push(`- Requested pass: ${pass.locale} x ${pass.themes.join(" + ")}, ${pass.rules === "all" ? "all rules" : "overflow rule only"}; ${pass.states} planned states.`);
  } else {
    push(`- Default pass: every view x ${meta.defaultLocale} x dark + light, all rules.`);
    push(`- Overflow pass: every view x ${meta.overflowLocales.join(", ")} x dark, overflow rule only.`);
  }
  push(`- Theme-invariant rules (${meta.themeInvariantRules}) are counted once per view (dark state); color rules are counted per theme.`);
  if (meta.emulatedMedia) push(`- Emulated media: ${Object.entries(meta.emulatedMedia).map(([name, value]) => `${name}: ${value}`).join(", ")}.`);
  push(`- Uncovered pages and reasons:`);
  if (uncovered.length) {
    for (const item of uncovered) push(`  - ${item}`);
  } else {
    push("  - none recorded");
  }
  if (errors.length) {
    push("", "## Execution failures", "", "These fixture or page operations failed. Their states were not audited and the run fails:", "");
    push(mdTable(["View", "Locale", "Theme", "Error"], errors.map(error => [error.view, error.locale, error.theme, error.error])));
  }
  if (collectionFailures.length) {
    push("", `## Capture failures`, "");
    push(`These states produced unusable evidence (rendering, shared foundation, locale or capture validation). They are excluded from the counts below and fail the run:`, "");
    push(mdTable(["View", "Locale", "Theme", "Reason"], collectionFailures.map(f => [f.view, f.locale, f.theme, f.reason])));
  } else {
    push("", `Capture failures: none among the collected states.${errors.length ? " Execution failures are listed above; coverage is incomplete." : ""}`);
  }
  push("", `## Totals`, "");
  push(`Findings are grouped per view by rule + root cause; counts below are element instances.`);
  const t = tallyGroups(groups);
  push("", `By rule:`, "");
  push(mdTable(["Rule", "Category", "Severity", "Instances", "Views"], Object.entries(t.byRule)
    .sort((a, b) => b[1] - a[1])
    .map(([rule, n]) => [rule, rulesMeta[rule]?.category ?? "?", rulesMeta[rule]?.severity ?? RULE_SEVERITY[rule] ?? "?", n,
      new Set(groups.filter(g => g.rule === rule).map(g => g.view)).size])));
  push("", `By severity:`, "", mdTable(["Severity", "Instances"], Object.entries(t.bySeverity).sort((a, b) => b[1] - a[1])));
  push("", `By functional area:`, "", mdTable(["Area", "Instances"], Object.entries(t.byArea).sort((a, b) => b[1] - a[1])));
  push("", `By category (current standard vs proposed rules):`, "", mdTable(["Category", "Instances"], Object.entries(t.byCategory).sort((a, b) => b[1] - a[1])));

  const categoryOfRule = rule => rulesMeta[rule]?.category ?? "current";
  const rankTable = (list, label) => {
    push("", `## Top ${Math.min(20, list.length)} root causes — ${label}`, "");
    push(`Ranked by how many element instances one style source explains; fixing one line often clears many findings. Suggested fixes are mechanical mappings (nearest token); the author decides the intended design.`, "");
    push(mdTable(["#", "Rule", "Root cause (selector @ file)", "Views", "Instances", "Typical view", "Suggestion"],
      list.slice(0, 20).map((rc, i) => [
        i + 1, rc.rule, `\`${rc.rootCause}\``, rc.viewCount, rc.instances, rc.exampleView,
        suggestForRootCause(rc),
      ])));
  };
  const causesCurrent = rootCauses.filter(rc => categoryOfRule(rc.rule) === "current");
  const causesProposed = rootCauses.filter(rc => categoryOfRule(rc.rule) === "proposed");
  rankTable(causesCurrent, "current standard");
  rankTable(causesProposed, "proposed rules (not yet VUA standard)");

  push("", `## Per-page card styles vs the shared Card`, "");
  push(`The shared \`Card\` primitive (\`components/primitives/card.css\`) is imported by ${cardImports.length} files. The CSS scan found ${cards.cards.length} other card-style components:`, "");
  push(mdTable(["Card component", "Defined in", "Radius", "Padding", "Border", "Background", "Shadow", "Verdict"],
    cards.cards.map(c => [
      `\`.${c.name}\` (${c.selectorCount} rules)`,
      c.file,
      c.radius ?? "—", c.padding ?? "—", c.border ?? "—", c.background ?? "—", c.shadow ?? "—",
      c.suggestion,
    ])));

  push("", `## Static checks (renderer CSS + TSX inline styles)`, "");
  const s = staticScan;
  const countOf = list => list.length;
  push(`- Hardcoded font sizes: ${countOf(s.css.filter(f => f.kind === "font-size"))} in CSS, ${countOf(s.tsx.filter(f => f.kind === "font-size"))} in TSX.`);
  push(`- Hardcoded border radii: ${countOf(s.css.filter(f => f.kind === "border-radius"))} in CSS, ${countOf(s.tsx.filter(f => f.kind === "border-radius"))} in TSX.`);
  push(`- Hardcoded hex colors: ${countOf(s.css.filter(f => f.kind === "hex-color"))} in CSS, ${countOf(s.tsx.filter(f => f.kind === "hex-color"))} in TSX.`);
  for (const [label, list] of [["CSS", s.css], ["TSX inline styles", s.tsx]]) {
    for (const kind of ["font-size", "border-radius", "hex-color"]) {
      const rows = list.filter(f => f.kind === kind);
      if (!rows.length) continue;
      push("", `### ${label} — hardcoded ${kind} (${rows.length})`, "");
      push(mdTable(["File:line", kind === "hex-color" ? "Value(s)" : "Value", "Selector / context", "Snippet"],
        rows.map(f => [
          `\`${path.relative(process.cwd(), f.file).replaceAll("\\", "/")}:${f.line}\``,
          f.value, f.selector ?? "", `\`${(f.snippet ?? "").slice(0, 90)}\``,
        ])));
    }
  }

  push("", `## Proposed rules (ZCode-inspired, not yet VUA standard)`, "");
  const proposed = Object.entries(t.byRule).filter(([rule]) => rulesMeta[rule]?.category === "proposed");
  for (const [rule, n] of proposed.sort((a, b) => b[1] - a[1])) {
    const views = new Set(groups.filter(g => g.rule === rule).map(g => g.view));
    push(`- \`${rule}\`: ${n} instances across ${views.size} views.`);
  }
  push(`Adopting them is a standard change and needs the author's ruling; the counts above estimate the blast radius.`);

  push("", `## Baseline`, "");
  if (checkOutcome?.mode === "write") {
    push(`- Refreshed the committed gate baseline with ${checkOutcome.entryCount} entries (one per rule + view + root cause).`);
    push(`- Future runs use \`--check-baseline\` to fail on findings outside this baseline; update it only when a fix lowers the violation count (\`pnpm audit:design --update-baseline\`).`);
  } else if (checkOutcome?.mode === "check") {
    push(`- Baseline check: ${checkOutcome.newCount} new or worsened finding groups, ${checkOutcome.resolvedCount} resolved groups. ${checkOutcome.failed ? "FAILED" : "Passed"}.`);
  } else if (checkOutcome?.mode === "refused") {
    push(`- Baseline update refused: ${checkOutcome.reason}`);
  } else {
    push(`- Wrote \`baseline.json\` into the run output only. The committed gate baseline was left untouched; refresh it with \`pnpm audit:design --update-baseline\` when a fix lowers the count.`);
  }

  push("", `## Limitations`, "");
  push(...data.limitations.map(l => `- ${l}`));
  push("", `## Machine-readable artifacts`, "");
  push(`- \`shots/<view>--<theme>.png\` and \`<view>--<theme>.annot.png\`: clean and annotated screenshots (default locale).`);
  push(`- \`json/<view>--<locale>--<theme>.json\`: actual findings and groups per state.`);
  push(`- \`snapshots/<view>--<theme>.json\`: collected style and geometry for rule re-evaluation (locale suffix for overflow passes).`);
  push(`- \`index.html\`: filterable overview (by rule, area, severity) linking screenshots and finding lists.`);
  push(`- \`baseline.json\`: committed gate input; keys are \`rule|view|root cause\`.`);
  return lines.join("\n") + "\n";
}

const OVERVIEW_CSS = `
:root { color-scheme: dark; font: 14px/20px "Segoe UI", sans-serif; background: #0b0a12; color: #f5f3ff; }
body { margin: 0; padding: 16px; }
h1 { font-size: 20px; }
.controls { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; }
select, input { background: #151322; color: #f5f3ff; border: 1px solid #2e2a45; border-radius: 6px; padding: 4px 8px; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 12px; }
.card { border: 1px solid #2e2a45; border-radius: 10px; overflow: hidden; cursor: pointer; background: #151322; }
.card img { width: 100%; display: block; }
.card .meta { padding: 8px; font-size: 12px; line-height: 16px; }
.sev-error { color: #fb7185; } .sev-warning { color: #fde047; } .sev-hint { color: #94a3ff; }
dialog { background: #151322; color: #f5f3ff; border: 1px solid #2e2a45; border-radius: 12px; max-width: 92vw; max-height: 92vh; }
dialog::backdrop { background: rgb(0 0 0 / 0.6); }
dialog img { max-width: 100%; }
.finding { font-size: 12px; margin: 4px 0; border-top: 1px solid #2e2a45; padding-top: 4px; }
`;

/** Builds the self-contained index.html overview. */
export function buildOverviewHtml(data) {
  const views = data.viewResults
    .filter(v => v.shot)
    .map(v => ({
      id: `${v.view}--${v.theme}`,
      view: v.view, area: v.area, theme: v.theme,
      png: path.basename(v.shot), annot: v.annot ? path.basename(v.annot) : null,
      total: v.findingsCount,
      bySeverity: v.bySeverity,
      byRule: v.byRule,
      findings: v.groupSummaries,
    }));
  const payload = {
    generatedAt: data.meta.generatedAt,
    commit: data.meta.commit,
    rules: data.rulesMeta,
    rootCauses: data.rootCauses.slice(0, 50).map(rc => ({
      rule: rc.rule, rootCause: rc.rootCause, instances: rc.instances, viewCount: rc.viewCount,
      views: rc.views, exampleView: rc.exampleView, severity: rc.severity,
      example: rc.exampleElement,
    })),
    views,
  };
  return `<!doctype html>
<html><head><meta charset="utf-8"><title>VUA design audit overview</title>
<style>${OVERVIEW_CSS}</style></head>
<body>
<h1>VUA design audit — ${data.meta.date} <small style="font-weight:400;font-size:12px">${data.meta.commit ?? ""}</small></h1>
${data.coverage ? `<p role="status"><strong>${data.meta.usable ? "Collection complete" : "Collection FAILED"}</strong>: ${data.coverage.audited}/${data.coverage.planned} states; ${data.coverage.executionFailures} execution failures; ${data.coverage.captureFailures} capture failures. <a href="summary.md">Full status and failures</a></p>` : ""}
<div class="controls">
  <label>Rule <select id="f-rule"><option value="">all</option></select></label>
  <label>Area <select id="f-area"><option value="">all</option></select></label>
  <label>Severity <select id="f-sev"><option value="">all</option><option>error</option><option>warning</option><option>hint</option></select></label>
  <label>Search <input id="f-text" placeholder="view / selector"></label>
  <label>Sort <select id="f-sort"><option value="findings">findings</option><option value="name">name</option></select></label>
  <span id="f-count"></span>
</div>
<div class="grid" id="grid"></div>
<dialog id="detail"><div id="detail-body"></div><p><button onclick="document.getElementById('detail').close()">Close</button></p></dialog>
<script>
const DATA = ${JSON.stringify(payload)};
const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const grid = document.getElementById("grid");
const ruleSel = document.getElementById("f-rule"), areaSel = document.getElementById("f-area"), sevSel = document.getElementById("f-sev");
for (const rule of Object.keys(DATA.rules).sort()) ruleSel.insertAdjacentHTML("beforeend", \`<option>\${esc(rule)}</option>\`);
for (const area of [...new Set(DATA.views.map(v => v.area))].sort()) areaSel.insertAdjacentHTML("beforeend", \`<option>\${esc(area)}</option>\`);
function render() {
  const rule = ruleSel.value, area = areaSel.value, sev = sevSel.value, text = document.getElementById("f-text").value.toLowerCase();
  const sort = document.getElementById("f-sort").value;
  let list = DATA.views.filter(v =>
    (!rule || (v.byRule[rule] ?? 0) > 0) && (!area || v.area === area)
    && (!sev || (v.bySeverity[sev] ?? 0) > 0)
    && (!text || v.id.toLowerCase().includes(text) || v.findings.some(f => (f.rootCause || "").toLowerCase().includes(text))));
  list.sort((a, b) => sort === "name" ? a.id.localeCompare(b.id) : b.total - a.total || a.id.localeCompare(b.id));
  document.getElementById("f-count").textContent = list.length + " / " + DATA.views.length + " view states";
  grid.innerHTML = list.map(v => {
    const sevHtml = ["error", "warning", "hint"].filter(s => v.bySeverity[s]).map(s => \`<span class="sev-\${s}">\${s} \${v.bySeverity[s]}</span>\`).join(" · ");
    return \`<div class="card" data-id="\${esc(v.id)}"><img loading="lazy" src="shots/\${esc(v.png)}" alt="\${esc(v.id)}">
      <div class="meta"><b>\${esc(v.view)}</b> (\${esc(v.theme)})<br>\${esc(v.area)} · \${v.total} findings<br>\${sevHtml}</div></div>\`;
  }).join("");
}
grid.addEventListener("click", event => {
  const card = event.target.closest(".card"); if (!card) return;
  const v = DATA.views.find(x => x.id === card.dataset.id);
  const body = document.getElementById("detail-body");
  body.innerHTML = \`<h2>\${esc(v.view)} (\${esc(v.theme)}) — \${v.total} findings</h2>
    <img src="shots/\${esc(v.annot || v.png)}" alt="annotated">
    <h3>Top root causes in this view</h3>\`
    + v.findings.slice(0, 30).map(f => \`<div class="finding"><span class="sev-\${f.severity}">\${esc(f.rule)}</span> — \${esc(f.rootCause)} (\${f.count} elements)</div>\`).join("");
  document.getElementById("detail").showModal();
});
for (const el of ["f-rule", "f-area", "f-sev", "f-text", "f-sort"]) document.getElementById(el).addEventListener("input", render);
render();
</script></body></html>`;
}
