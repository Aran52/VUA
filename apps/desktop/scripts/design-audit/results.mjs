// Coverage and saved state results are shared by the runner and its regressions.
export const stateKey = state => `${typeof state.view === "string" ? state.view : state.view.id}|${state.locale}|${state.theme}`;

export function auditCoverage(plan, collected, errors = [], collectionFailures = []) {
  const planned = plan.flatMap(group => group.states.map(state => ({ view: group.view.id, locale: group.locale, theme: state.theme })));
  const expected = new Map(planned.map(state => [stateKey(state), state]));
  const seen = new Set();
  const duplicates = [];
  const unexpected = [];
  for (const state of [...collected, ...errors, ...collectionFailures]) {
    const key = stateKey(state);
    if (seen.has(key)) duplicates.push(key);
    if (!expected.has(key)) unexpected.push(key);
    seen.add(key);
  }
  const missing = [...expected].filter(([key]) => !seen.has(key)).map(([, state]) => state);
  return {
    planned: planned.length,
    audited: collected.length,
    executionFailures: errors.length,
    captureFailures: collectionFailures.length,
    missing, duplicates, unexpected,
    complete: planned.length > 0 && expected.size === planned.length && collected.length === planned.length
      && errors.length === 0 && collectionFailures.length === 0 && missing.length === 0
      && duplicates.length === 0 && unexpected.length === 0,
  };
}

export function stateResult(state) {
  return {
    view: state.view, area: state.area, locale: state.locale, theme: state.theme,
    elementCount: state.elementCount, findingsCount: state.findingsCount,
    byRule: state.byRule, bySeverity: state.bySeverity,
    findings: state.findings, groups: state.groups,
  };
}

export function runPassed(coverage, elementTotal, elementFloor) {
  return coverage.complete && elementTotal >= elementFloor;
}
