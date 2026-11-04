// Baseline generation and comparison for the design audit.
// One entry per (rule | view | root-cause key); the CI gate uses --check-baseline.

import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

export function findingKey(view, finding) {
  return `${finding.rule}|${view}|${finding.rootCause ?? "unattributed"}`;
}

/** Groups raw findings of one view by (rule, rootCause) with element details. */
export function groupFindings(view, findings) {
  const groups = new Map();
  for (const finding of findings) {
    const key = findingKey(view, finding);
    let group = groups.get(key);
    if (!group) {
      group = {
        key,
        rule: finding.rule,
        severity: finding.severity,
        category: finding.category,
        view,
        rootCause: finding.rootCause ?? "unattributed",
        elements: [],
        examples: [],
      };
      groups.set(key, group);
    }
    group.elements.push({
      path: finding.path, tag: finding.tag, cls: finding.cls,
      rect: finding.rect ?? null, text: finding.text ?? "",
      details: finding.details,
    });
    if (group.examples.length < 3) group.examples.push({ tag: finding.tag, cls: finding.cls, text: finding.text, details: finding.details });
  }
  return [...groups.values()];
}

export function entryCounts(group) {
  return { instances: group.elements.length };
}

function baselineEntries(groups) {
  const entries = {};
  const rank = { hint: 0, warning: 1, error: 2 };
  for (const group of groups) {
    const previous = entries[group.key];
    entries[group.key] = {
      instances: Math.max(previous?.instances ?? 0, group.elements.length),
      severity: previous && rank[previous.severity] > rank[group.severity] ? previous.severity : group.severity,
    };
  }
  return entries;
}

export async function writeBaseline(file, allGroups, meta) {
  const entries = baselineEntries(allGroups);
  const payload = {
    toolVersion: 2,
    generatedAt: meta.generatedAt,
    commit: meta.commit ?? null,
    views: meta.views,
    entryCount: Object.keys(entries).length,
    entries,
  };
  await mkdir(path.dirname(file), { recursive: true });
  await writeFile(file, JSON.stringify(payload, null, 2) + "\n", "utf8");
  return payload;
}

export async function loadBaseline(file) {
  try {
    return JSON.parse(await readFile(file, "utf8"));
  } catch {
    return null;
  }
}

/**
 * Compares fresh groups against the baseline.
 * Returns { newFindings, resolvedKeys, baselineMissing }.
 */
export function checkAgainstBaseline(groups, baseline, scopeViews) {
  if (!baseline?.entries) return { baselineMissing: true, newFindings: groups, resolvedKeys: [] };
  const scope = scopeViews && new Set(scopeViews);
  const known = new Set(Object.keys(baseline.entries).filter(key => !scope || scope.has(key.split("|")[1])));
  const current = new Set(groups.map(g => g.key));
  const counts = baselineEntries(groups);
  const rank = { hint: 0, warning: 1, error: 2 };
  const newFindings = [...new Set(groups.map(group => group.key))].flatMap(key => {
    const before = baseline.entries[key];
    const after = counts[key];
    const reason = !before ? "new_group" : after.instances > before.instances ? "increased_instances"
      : rank[after.severity] > rank[before.severity] ? "increased_severity" : null;
    return reason ? [{ ...groups.find(group => group.key === key), reason, previousInstances: before?.instances ?? 0, instances: after.instances }] : [];
  });
  const resolvedKeys = [...known].filter(k => !current.has(k));
  return { baselineMissing: false, newFindings, resolvedKeys };
}
