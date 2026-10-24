import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { auditCoverage, runPassed, stateResult } from "./design-audit/results.mjs";
import { groupFindings, writeBaseline, checkAgainstBaseline } from "./design-audit/baseline.mjs";
import { buildSummary, buildOverviewHtml } from "./design-audit/report.mjs";

const plan = [{ view: { id: "library" }, locale: "en", states: [{ theme: "dark" }, { theme: "light" }] }];
const dark = { view: "library", locale: "en", theme: "dark" };
const light = { ...dark, theme: "light" };

test("one failed fixture fails an otherwise large, nonempty audit", () => {
  const coverage = auditCoverage(plan, [dark], [{ ...light, error: "fixture assertion" }]);
  assert.equal(coverage.planned, 2);
  assert.equal(coverage.executionFailures, 1);
  assert.equal(runPassed(coverage, 80000, 30000), false);
  assert.equal(runPassed(auditCoverage(plan, [dark, light]), 80000, 30000), true);
});

test("missing, duplicate, unplanned and empty coverage cannot pass", () => {
  for (const coverage of [auditCoverage(plan, [dark]), auditCoverage(plan, [dark, dark]),
    auditCoverage(plan, [dark, light, { ...dark, view: "other" }]), auditCoverage([], [])]) {
    assert.equal(runPassed(coverage, 80000, 30000), false);
  }
  const captured = auditCoverage(plan, [dark], [], [{ ...light, reason: "error overlay" }]);
  assert.equal(captured.captureFailures, 1);
  assert.equal(captured.complete, false);
});

test("saved per-page JSON contains findings and their groups, never the run plan", () => {
  const findings = [{ rule: "overflow", severity: "error", rootCause: ".wide", details: { kind: "viewport" } }];
  const groups = groupFindings("library", findings);
  const saved = JSON.parse(JSON.stringify(stateResult({ ...dark, findings, groups, findingsCount: 1 })));
  assert.equal(saved.groups[0].rule, "overflow");
  assert.equal(saved.groups[0].elements[0].details.kind, "viewport");
  assert.deepEqual(saved.findings, findings);
  assert.equal(saved.groups[0].states, undefined);
});

test("the readable report and overview expose partial execution failures", () => {
  const errors = [{ ...light, error: "fixture assertion" }];
  const coverage = auditCoverage(plan, [dark], errors);
  const data = {
    meta: { date: "2026-10-25", viewCount: 1, viewLocaleStates: 1, elementTotal: 100, defaultLocale: "en", overflowLocales: [], usable: false },
    coverage, errors, collectionFailures: [], viewResults: [], groups: [], rootCauses: [],
    staticScan: { css: [], tsx: [] }, cards: { cards: [] }, cardImports: [], uncovered: [],
    rulesMeta: {}, checkOutcome: { mode: "skip" }, limitations: [],
  };
  const summary = buildSummary(data);
  assert.match(summary, /1\/2 planned states audited/);
  assert.match(summary, /Execution failures/);
  assert.match(summary, /fixture assertion/);
  assert.doesNotMatch(summary, /every planned state rendered/);
  assert.match(buildOverviewHtml(data), /Collection FAILED/);
});

test("the baseline retains the worst locale and rejects count or severity regressions", async () => {
  const make = (count, severity = "warning") => groupFindings("library", Array.from({ length: count }, () => ({
    rule: "spacing-grid", severity, category: "current", rootCause: ".row", details: {},
  })))[0];
  const directory = await mkdtemp(path.join(os.tmpdir(), "vua-audit-baseline-"));
  try {
    const file = path.join(directory, "baseline.json");
    await writeBaseline(file, [make(4), make(1)], {});
    const baseline = JSON.parse(await readFile(file, "utf8"));
    assert.equal(baseline.entries[make(1).key].instances, 4);
    assert.equal(checkAgainstBaseline([make(5)], baseline).newFindings[0].reason, "increased_instances");
    assert.equal(checkAgainstBaseline([make(4, "error")], baseline).newFindings[0].reason, "increased_severity");
    assert.equal(checkAgainstBaseline([make(3)], baseline).newFindings.length, 0);
    assert.deepEqual(checkAgainstBaseline([], baseline, ["other"]).resolvedKeys, []);
  } finally {
    assert.equal(path.dirname(path.resolve(directory)), path.resolve(os.tmpdir()));
    assert.match(path.basename(directory), /^vua-audit-baseline-/);
    await rm(directory, { recursive: true, force: true });
  }
});
