// Design audit runner: renders every registered view through the existing
// preview/smoke fixtures in a hidden Electron window, walks computed styles,
// attributes each finding to its CSS rule, and writes screenshots, an
// overview page, summary.md and baseline.json.
//
// Usage: pnpm audit:design [--only=<view prefix>] [--locale=<l>] [--theme=dark|light]
//                          [--out=<dir>] [--check-baseline] [--update-baseline]
//                          [--no-screens] [--max=<n>] [--list]
// Unrelated to component styling: this script never edits UI code.

import { app, BrowserWindow, protocol } from "electron";
import { createServer } from "vite";
import react from "@vitejs/plugin-react";
import { execSync } from "node:child_process";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  FIXTURES, VIEWS, OVERFLOW_LOCALES, DEFAULT_LOCALE, THEMES,
} from "./design-audit/views.mjs";
import { collectAuditSnapshot } from "./design-audit/collector.js";
import { applyAnnotations, removeAnnotations } from "./design-audit/annotate.js";
import { evaluateSnapshot, borderDensityMetric, applyBorderDensity, RULES } from "./design-audit/rules.mjs";
import { Attributor } from "./design-audit/attributor.mjs";
import { groupFindings, writeBaseline, loadBaseline, checkAgainstBaseline } from "./design-audit/baseline.mjs";
import { scanStatic } from "./design-audit/static-scan.mjs";
import { scanCardStyles, countCardImports } from "./design-audit/cards.mjs";
import { buildSummary, buildOverviewHtml, rankRootCauses } from "./design-audit/report.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const desktopRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(desktopRoot, "..", "..");

// ---- CLI ---------------------------------------------------------------------

const args = process.argv.slice(2).filter(a => a.startsWith("--"));
const argValue = (name, fallback = null) => {
  const hit = args.find(a => a.startsWith(`--${name}=`));
  return hit ? hit.slice(name.length + 3) : fallback;
};
const hasFlag = name => args.includes(`--${name}`);

const ONLY = argValue("only", "");
const LOCALE_FILTER = argValue("locale", null);
const THEME_FILTER = argValue("theme", null);
const OUT = argValue("out", path.join(desktopRoot, "_local_design_audit", new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19)));
const MAX_VIEWS = Number(argValue("max", "0"));
const NO_SCREENS = hasFlag("no-screens");
const CHECK_BASELINE = hasFlag("check-baseline");
const UPDATE_BASELINE = hasFlag("update-baseline");
const BASELINE_PATH = path.join(scriptDir, "design-audit", "baseline.json");

if (hasFlag("list")) {
  console.log(VIEWS.map(v => `${v.id}\t${v.area}\t${v.fixture}`).join("\n"));
  process.exit(0);
}

// ---- plan --------------------------------------------------------------------

const ONLY_PREFIXES = ONLY.split(",").map(p => p.trim()).filter(Boolean);
const selectedViews = MAX_VIEWS > 0
  ? VIEWS.slice(0, MAX_VIEWS)
  : VIEWS.filter(v => ONLY_PREFIXES.length === 0 || ONLY_PREFIXES.some(p => v.id.startsWith(p)));
const locales = LOCALE_FILTER ? [LOCALE_FILTER] : [DEFAULT_LOCALE, ...OVERFLOW_LOCALES];
const themes = THEMES.filter(t => !THEME_FILTER || t === THEME_FILTER);

function planGroups() {
  const groups = [];
  for (const view of selectedViews) {
    for (const locale of locales) {
      const isDefaultLocale = locale === DEFAULT_LOCALE;
      const themeList = isDefaultLocale ? themes : themes.filter(t => t === "dark");
      if (!themeList.length) continue;
      groups.push({
        view, locale, themes: themeList,
        states: themeList.map(theme => ({
          theme,
          rules: isDefaultLocale ? null : ["overflow"],
          capture: !NO_SCREENS && (isDefaultLocale || LOCALE_FILTER !== null),
        })),
      });
    }
  }
  return groups;
}

const stamp = () => new Date().toISOString().slice(11, 19).replace(/:/g, "");
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));

let commit = null;
try { commit = execSync("git rev-parse --short HEAD", { cwd: repoRoot }).toString().trim(); } catch { /* detached */ }

const errors = [];
const groups = planGroups();

// vua-img must be privileged before app ready (library-entries thumbnails).
protocol.registerSchemesAsPrivileged([{ scheme: "vua-img", privileges: { standard: true, secure: true } }]);
app.on("window-all-closed", () => {});

const LIMITATIONS = [
  "Aesthetic judgment, motion quality and information architecture are human review topics; the tool only checks the rules listed in docs/design/design-audit.md.",
  "Computed-style analysis sees one state per view. Hover, focus, pressed and disabled interactions, and states that require live Gateway traffic, are not exercised unless a fixture mounts them.",
  "Contrast is computed against alpha-composited backgrounds; gradient and image backgrounds fall back to their background-color layer, so text over busy art may be mis-graded.",
  "Attribution prefers the CDP cascade winner; hover-only rules (:hover) cannot be matched from the static state, so those findings fall back to class+area attribution.",
  "Overflow checks see the zh-CN layout for all views plus four extra locales in dark theme; light-theme text growth in other locales is not measured.",
  "Primary-action detection is heuristic (opaque accent-filled controls); custom-selected states can inflate the one-accent count.",
  "data-audit-skip regions (base-map scene canvas, turntable frames, WebGL scenes, product imagery) are excluded by design and not audited.",
];

async function main() {
  await mkdtemp(path.join(tmpdir(), "vua-design-audit-")).then(dir => app.setPath("userData", dir));
  await app.whenReady();
  protocol.handle("vua-img", request => request.url === `vua-img://local/${"a".repeat(64)}`
    ? new Response(Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=", "base64"), { headers: { "Content-Type": "image/png" } })
    : new Response(null, { status: 404 }));

  const shotsDir = path.join(OUT, "shots");
  const jsonDir = path.join(OUT, "json");
  await mkdir(shotsDir, { recursive: true });
  await mkdir(jsonDir, { recursive: true });

  const server = await createServer({
    configFile: false,
    appType: "custom",
    root: desktopRoot,
    plugins: [react()],
    logLevel: "error",
    server: { host: "127.0.0.1", port: 0 },
    optimizeDeps: { include: ["react", "react-dom/client", "@vua/contracts"] },
  });

  // The middleware must know which locale/theme the runner is about to load.
  let shellParams = { locale: DEFAULT_LOCALE, theme: "dark" };
  const fixturesByPrefix = Object.entries(FIXTURES).sort((a, b) => b[1].prefix.length - a[1].prefix.length);
  server.middlewares.use(async (req, res, next) => {
    const hit = fixturesByPrefix.find(([, fixture]) => req.url?.startsWith(fixture.prefix));
    if (!hit) return next();
    const [, fixture] = hit;
    res.setHeader("Content-Type", "text/html");
    res.end(await server.transformIndexHtml(req.url, fixture.shell(shellParams)));
  });
  await server.listen();
  const origin = `http://127.0.0.1:${server.httpServer.address().port}`;

  let window = null;
  function ensureWindow() {
    if (!window || window.isDestroyed()) {
      // Hidden (not offscreen) like the smoke harnesses: some dialog focus
      // assertions and captures behave closer to the real shell this way.
      window = new BrowserWindow({
        show: false, width: 1280, height: 860,
        webPreferences: { contextIsolation: true, sandbox: true, backgroundThrottling: false },
      });
      window.webContents.on("console-message", event => {
        if (event.level === "error") console.error(`[console] ${event.message?.slice(0, 240)}`);
      });
      // One debugger session per window (attach is synchronous): attribution
      // (CDP) and the media emulation the fixtures assert against share it.
      try { window.webContents.debugger.attach("1.3"); } catch { /* already attached */ }
    }
    return window;
  }

  const viewUrl = (view, theme) => {
    if (view.fixture === "factory") {
      const query = view.id.startsWith("feedback-") ? "?feedback=1" : "";
      return `${origin}/__factory${query}#${view.id}`;
    }
    // Fixtures read their state from the shell itself; the hash is for humans.
    return `${origin}${FIXTURES[view.fixture].prefix}`;
  };

  function pollReady(fixture, win) {
    if (!fixture.readyGlobal) return Promise.resolve(true);
    return (async () => {
      for (let i = 0; i < 200; i += 1) {
        try {
          if (await win.webContents.executeJavaScript(`!!window.${fixture.readyGlobal}`, true)) return true;
        } catch { /* page navigating */ }
        await sleep(50);
      }
      return false;
    })();
  }

  async function settle(win, ms = 400) {
    await win.webContents.executeJavaScript(
      "new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))", true,
    ).catch(() => {});
    await sleep(ms);
  }

  async function runSteps(win, view) {
    for (const step of view.steps ?? []) {
      await win.webContents.executeJavaScript(`(async () => { ${step} })()`, true);
    }
  }

  const collected = []; // per-state results with raw findings

  // One page load per (view, locale): drive the fixture to its state once, then
  // switch themes in place. storage-reload fixtures reload per theme because the
  // component reads the theme at mount.
  async function auditViewLocale(group) {
    const { view, locale } = group;
    const fixture = FIXTURES[view.fixture];
    const win = ensureWindow();
    const size = fixture.viewport(view.id);
    win.setBounds({ width: size.width, height: size.height });

    const load = async theme => {
      shellParams = { locale, theme };
      await win.loadURL(viewUrl(view, theme));
      // Fixtures assert the system appearance resolves to light (the
      // first-run smoke emulates the same media feature).
      if (win.webContents.debugger.isAttached()) {
        await win.webContents.debugger.sendCommand("Emulation.setEmulatedMedia", {
          features: [{ name: "prefers-color-scheme", value: "light" }],
        }).catch(() => {});
      }
      if (!(await pollReady(fixture, win))) throw new Error(`fixture readiness timeout (${fixture.readyGlobal ?? "no signal"})`);
      await runSteps(win, view);
      if (fixture.settleFor) await sleep(fixture.settleFor(view.id));
      else await settle(win, view.settleMs ?? 400);
    };

    for (const [themeIndex, theme] of group.themes.entries()) {
      const state = group.states[themeIndex];
      if (fixture.themeMode === "storage-reload") {
        await load(theme);
      } else if (themeIndex === 0) {
        await load(fixture.themeMode === "attribute" ? "dark" : theme);
        await setTheme(fixture, win, theme);
      } else {
        await setTheme(fixture, win, theme);
      }
      await settle(win, 400);

      const rules = state.rules ?? undefined;
      const collectorOptions = { proposed: !rules, overflowChecks: true };
      const collectSource = `${collectAuditSnapshot.toString()};collectAuditSnapshot(${JSON.stringify(collectorOptions)})`;
      const snapshot = await win.webContents.executeJavaScript(collectSource, true);
      let findings = evaluateSnapshot(snapshot, { rules, exemptRules: view.exemptRules ?? [] });

      // Root-cause attribution against the live page.
      const attributor = new Attributor(win.webContents, () => view.area);
      await attributor.attach();
      const targets = findings.map((finding, findingIndex) => ({
        findingIndex, path: finding.path,
        property: attributionProperty(finding),
        el: finding.index != null ? snapshot.elements[finding.index] : { tag: "view", cls: [] },
      })).filter(t => t.path || t.property == null);
      const attributions = await attributor.attribute(targets, view.area);
      await attributor.detach();
      findings = findings.map((finding, findingIndex) => ({
        ...finding,
        rootCause: finding.path
          ? (attributions.get(findingIndex) ?? "unattributed")
          : `view-level: ${finding.message}`,
      }));

      // Annotate first (needs live rects), then capture annotated + clean shots.
      let shotPath = null;
      let annotPath = null;
      if (state.capture) {
        const annotateSource = `${applyAnnotations.toString()};applyAnnotations(${JSON.stringify(annotateItems(findings, snapshot))}, ${JSON.stringify({ view: view.id, theme, locale })})`;
        await win.webContents.executeJavaScript(annotateSource, true).catch(() => {});
        await sleep(300);
        const name = shotName(view.id, locale, theme);
        annotPath = path.join(shotsDir, `${name}.annot.png`);
        await writeCapture(win, annotPath);
        await win.webContents.executeJavaScript(`${removeAnnotations.toString()};removeAnnotations()`, true).catch(() => {});
        await sleep(150);
        shotPath = path.join(shotsDir, `${name}.png`);
        await writeCapture(win, shotPath);
      }

      collected.push({
        view: view.id, area: view.area, locale, theme,
        findings,
        metrics: borderDensityMetric(snapshot),
        elementCount: snapshot.elementCount,
        viewport: snapshot.viewport,
        shotPath, annotPath,
        rules: rules ?? "all",
      });
      console.log(`[${stamp()}] ${view.id} ${locale}/${theme}: ${findings.length} findings, ${snapshot.elementCount} elements`);
    }
  }

  async function setTheme(fixture, win, theme) {
    if (fixture.themeMode === "attribute") {
      await win.webContents.executeJavaScript(`document.documentElement.dataset.theme=${JSON.stringify(theme)}`, true);
    } else if (fixture.themeMode === "preference") {
      await win.webContents.executeJavaScript(fixture.themeSetter(theme), true);
    } else if (fixture.themeMode === "storage-reload") {
      // Already loaded with the right theme; nothing to switch.
    }
  }

  async function writeCapture(win, file) {
    const image = await win.webContents.capturePage({}, { stayHidden: true, stayAwake: true });
    await writeFile(file, image.toPNG());
  }

  for (const group of groups) {
    try {
      await withTimeout(auditViewLocale(group), 240_000, `${group.view.id}/${group.locale} timed out`);
    } catch (error) {
      for (const state of group.states) {
        errors.push({ view: group.view.id, locale: group.locale, theme: state.theme, error: String(error.message ?? error) });
      }
      console.error(`[fail] ${group.view.id} ${group.locale}: ${error.message ?? error}`);
      try { if (window && !window.isDestroyed()) window.destroy(); } catch { /* gone */ }
    }
  }
  try { if (window && !window.isDestroyed()) window.destroy(); } catch { /* ignore */ }

  // border-density is cross-view; evaluate it over the default-locale dark states.
  const densityMetrics = collected
    .filter(c => c.locale === DEFAULT_LOCALE && c.theme === "dark" && c.rules === "all")
    .map(c => ({ key: `${c.view}|${c.locale}|${c.theme}`, density: c.metrics.density, bordered: c.metrics.bordered }));
  const findingsByKey = new Map(collected.map(c => [`${c.view}|${c.locale}|${c.theme}`, c.findings]));
  applyBorderDensity(densityMetrics, findingsByKey);

  // Group findings per state and aggregate.
  const allGroups = [];
  for (const state of collected) {
    const groups = groupFindings(state.view, state.findings).map(g => ({ ...g, area: state.area, locale: state.locale, theme: state.theme }));
    state.groups = groups;
    state.findingsCount = state.findings.length;
    state.bySeverity = {};
    state.byRule = {};
    for (const finding of state.findings) {
      state.bySeverity[finding.severity] = (state.bySeverity[finding.severity] ?? 0) + 1;
      state.byRule[finding.rule] = (state.byRule[finding.rule] ?? 0) + 1;
    }
    state.groupSummaries = groups.map(g => ({ rule: g.rule, severity: g.severity, rootCause: g.rootCause, count: g.elements.length }));
    for (const group of groups) allGroups.push(group);
    await writeFile(
      path.join(jsonDir, `${jsonName(state.view, state.locale, state.theme)}.json`),
      JSON.stringify({
        view: state.view, area: state.area, locale: state.locale, theme: state.theme,
        elementCount: state.elementCount, findingsCount: state.findingsCount,
        byRule: state.byRule, bySeverity: state.bySeverity,
        groups,
      }, null, 2),
      "utf8",
    );
  }

  // Static scans and the card inventory.
  const rendererDir = path.join(desktopRoot, "src", "renderer");
  const staticScan = await scanStatic(rendererDir);
  const cards = await scanCardStyles(rendererDir, path.join(repoRoot, "packages", "design-system", "src", "tokens.css"));
  const cardImports = await countCardImports(rendererDir);

  const rootCauses = rankRootCauses(allGroups);
  const defaultStates = collected.filter(c => c.locale === DEFAULT_LOCALE);
  const viewResults = defaultStates.map(state => ({
    view: state.view, area: state.area, theme: state.theme,
    shot: state.shotPath, annot: state.annotPath,
    findingsCount: state.findingsCount,
    bySeverity: state.bySeverity, byRule: state.byRule,
    groupSummaries: state.groupSummaries,
  }));

  // Baseline handling: every run writes baseline.json into its output dir;
  // --update-baseline additionally refreshes the committed gate baseline.
  let checkOutcome = { mode: "skip" };
  const outBaseline = await writeBaseline(path.join(OUT, "baseline.json"), allGroups, {
    generatedAt: new Date().toISOString(), commit, views: selectedViews.length,
  });
  if (UPDATE_BASELINE) {
    await writeBaseline(BASELINE_PATH, allGroups, {
      generatedAt: new Date().toISOString(), commit, views: selectedViews.length,
    });
    checkOutcome = { mode: "write", entryCount: outBaseline.entryCount };
    console.log(`baseline written: ${BASELINE_PATH} (${outBaseline.entryCount} entries)`);
  } else if (CHECK_BASELINE) {
    const baseline = await loadBaseline(BASELINE_PATH);
    const { baselineMissing, newFindings, resolvedKeys } = checkAgainstBaseline(allGroups, baseline);
    checkOutcome = {
      mode: "check", baselineMissing,
      newCount: newFindings.length, resolvedCount: resolvedKeys.length,
    };
    if (baselineMissing) {
      console.error(`baseline missing at ${BASELINE_PATH} — run with --update-baseline first`);
    } else {
      for (const group of newFindings) console.error(`NEW: ${group.key}`);
      console.error(`baseline check: ${newFindings.length} new, ${resolvedKeys.length} resolved`);
      if (newFindings.length > 0) process.exitCode = 1;
    }
  }

  const meta = {
    date: new Date().toISOString().slice(0, 10),
    generatedAt: new Date().toISOString(),
    commit,
    viewCount: selectedViews.length,
    viewLocaleStates: collected.length,
    elementTotal: collected.reduce((sum, c) => sum + c.elementCount, 0),
    defaultLocale: DEFAULT_LOCALE,
    overflowLocales: OVERFLOW_LOCALES,
  };

  const overviewHtml = buildOverviewHtml({
    meta, viewResults, rootCauses,
    rulesMeta: Object.fromEntries(Object.entries(RULES).map(([id, r]) => [id, { severity: r.severity, category: r.category }])),
  });
  await writeFile(path.join(OUT, "index.html"), overviewHtml, "utf8");

  const summary = buildSummary({
    meta, viewResults, groups: allGroups, rootCauses, staticScan, cards, cardImports,
    uncovered: UNCOVERED_PAGES, limitations: LIMITATIONS, rulesMeta: RULES_META_FOR_DOCS,
    checkOutcome,
  });
  await writeFile(path.join(OUT, "summary.md"), summary, "utf8");
  await writeFile(path.join(OUT, "run-summary.json"), JSON.stringify({
    meta, errors, states: collected.map(c => ({
      view: c.view, locale: c.locale, theme: c.theme, findings: c.findingsCount, elements: c.elementCount,
    })),
  }, null, 2), "utf8");

  console.log(`\ndone: ${collected.length} states, ${errors.length} failed states`);
  console.log(`output: ${OUT}`);
  await server.close();
  const plannedStates = groups.reduce((sum, g) => sum + g.states.length, 0);
  app.exit(errors.length >= plannedStates && plannedStates > 0 ? 1 : process.exitCode ?? 0);
}

function withTimeout(promise, ms, message) {
  return Promise.race([
    promise,
    new Promise((_, reject) => setTimeout(() => reject(new Error(message)), ms)),
  ]);
}

function attributionProperty(finding) {
  // These rules carry the concrete property in the finding details.
  if (finding.rule === "color-token") return finding.details?.property ?? null;
  if (finding.rule === "spacing-grid") return finding.details?.property ?? null;
  const kind = RULES[finding.rule]?.attribution;
  if (!kind) return null;
  switch (kind) {
    case "font": return "font";
    case "font-weight": return "font-weight";
    case "color": return "color";
    case "radius": return "radius";
    case "box-shadow": return "box-shadow";
    case "backdrop-filter": return "backdrop-filter";
    case "background": return "background-color";
    default: return null;
  }
}

function annotateItems(findings, snapshot) {
  const order = { error: 0, warning: 1, hint: 2 };
  return findings
    .map(f => ({
      rule: f.rule, severity: f.severity, message: f.message, cls: f.cls, tag: f.tag,
      rect: f.index != null ? snapshot.elements[f.index]?.rect ?? null : null,
    }))
    .filter(item => item.rect)
    .sort((a, b) => (order[a.severity] ?? 3) - (order[b.severity] ?? 3));
}

function shotName(view, locale, theme) {
  return locale === DEFAULT_LOCALE ? `${view}--${theme}` : `${view}--${locale}--${theme}`;
}
function jsonName(view, locale, theme) {
  return `${view}--${locale}--${theme}`;
}

const RULES_META_FOR_DOCS = Object.fromEntries(Object.entries(RULES).map(([id, r]) => [id, r]));

const UNCOVERED_PAGES = [
  "Play page environment cards beyond the two preview states (network-check covers the network tile only) — no dedicated fixture; partially covered via env-play/env-create.",
  "Tools page and Settings home/categories pages — no fixture mounts them; smoke:vrcft drives the packaged app and is out of scope for the audit harness. Settings accounts is covered via first-run-settings.",
  "Package manager page (features/packages) outside the AMF control-center variants — no fixture.",
  "First-run wizard steps — the first-run-review fixture only walks them inside run() assertions; the wizard is not reachable as a settled state. Home, Play, Creator and Settings are covered.",
  "Production review page (smoke-production-review) — its fixture is an assert-and-advance interaction harness with no settled visual state; the covered workshop/control/import views show the same components.",
  "Overlay windows (desktop overlay task window is covered; VR overlay, game guide, tutorial companion, reader window) — separate windows not mounted by any fixture.",
  "Command palette and Ctrl+P surfaces — no fixture (first-run-settings opens and closes it mid-run).",
];

void main().catch(error => { console.error(error); app.exit(1); });
