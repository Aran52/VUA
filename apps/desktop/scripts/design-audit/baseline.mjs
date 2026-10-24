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

export async function writeBaseline(file, allGroups, meta) {
  const entries = {};
  for (const group of allGroups) {
    entries[group.key] = { instances: group.elements.length, severity: group.severity };
  }
  const payload = {
    toolVersion: 1,
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
export function checkAgainstBaseline(groups, baseline) {
  if (!baseline?.entries) return { baselineMissing: true, newFindings: groups, resolvedKeys: [] };
  const known = new Set(Object.keys(baseline.entries));
  const current = new Set(groups.map(g => g.key));
  const newFindings = groups.filter(g => !known.has(g.key));
  const resolvedKeys = [...known].filter(k => !current.has(k));
  return { baselineMissing: false, newFindings, resolvedKeys };
}
