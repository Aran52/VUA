/** The packages page follows the workshop's permission mode (plan 8 points 3–4):
 * full auto reads conclusions only, limited auto adds the next step and a
 * handle-it action, manual review lists every package, version and feed.
 * Everything here reads existing ports; nothing is fabricated. */
import { useEffect, useState } from "react";
import { useGateway } from "../../gateway/index.ts";
import { format, strings } from "../../i18n/index.ts";
import { openExternalUrl } from "../../app/open-external.ts";
import { Alert } from "../../components/primitives/Alert.tsx";
import { Button } from "../../components/primitives/Button.tsx";
import { SkinSlot } from "./control-slots.tsx";
import type { ControlPackageRow, ControlProject } from "./control-packages-model.ts";
import { packageConclusion, packagePinLines, type GuideNeed, type PackagePin } from "./package-mode-model.ts";
import type { AutomationLevel } from "./factory-model.ts";

const copy = strings.controlCenter.packages;

/** Packages a work project still needs from their authors' own pages (the line's
 * `amf.package.guide` diagnostics): the official page only, never a download. */
export function PackageGuideAlert({ needs }: { readonly needs: readonly GuideNeed[] }) {
  if (needs.length === 0) return null;
  return <Alert tone="info" title={copy.guideLead}>{needs.map(need => {
    const url = need.url;
    return <p key={need.message} className="vua-control__guide-need">
      <span>{need.message}</span>
      {url === null ? null : <Button variant="subtle" onClick={() => { void openExternalUrl(url); }}>{copy.officialPage}</Button>}
    </p>;
  })}</Alert>;
}

/** One project's package conclusion: counts, updates when the backend judged them,
 * and the Recipe pins on installed packages. Full auto stops here; limited auto
 * adds the next step and a "handle dependencies" action. */
export function ProjectPackageConclusion({ level, rows, cached, pins, guideCount, busy, onHandleDependencies }: {
  readonly level: AutomationLevel;
  readonly rows: readonly ControlPackageRow[];
  readonly cached: boolean;
  readonly pins: ReadonlyMap<string, readonly PackagePin[]>;
  readonly guideCount: number;
  readonly busy: boolean;
  readonly onHandleDependencies?: (() => void) | undefined;
}) {
  const summary = packageConclusion(rows);
  const lines = packagePinLines(pins, new Set(rows.map(row => row.packageId)));
  const nameOf = new Map(rows.map(row => [row.packageId, row.displayName] as const));
  const updates = summary.updates ?? 0;
  const next = updates > 0 ? "update" : guideCount > 0 ? "guide" : null;
  return <div className="vua-control__panel vua-control__conclusion">
    <SkinSlot name="control-panel-corner" className="vua-control__corner"/>
    <p><strong>{format(copy.installedCount, { count: summary.installed })}</strong>
      {summary.updates === null ? null : summary.updates === 0 ? <> · {copy.upToDate}</> : <> · {format(copy.updatesCount, { count: summary.updates })}</>}
      {cached ? <small> · {copy.cached}</small> : null}</p>
    {lines.map(line => <p key={`${line.packageId}@${line.version}`} className="vua-caption vua-text-secondary"
      title={line.recipes.join(", ")}>
      {nameOf.get(line.packageId) ?? line.packageId} · {line.recipes.length === 1
        ? format(copy.pinnedBy, { names: line.recipes[0]!, version: line.version })
        : format(copy.pinnedCompact, { count: line.recipes.length, version: line.version })}</p>)}
    {level === "smart" && next !== null ? <p className="vua-control__next-step"><strong>{format(copy.nextStep, { text: next === "update" ? format(copy.nextUpdate, { count: updates }) : copy.nextGuide })}</strong>
      {next === "update" && onHandleDependencies !== undefined
        ? <Button variant="primary" disabled={busy} onClick={onHandleDependencies}>{copy.handleDependencies}</Button> : null}</p> : null}
  </div>;
}

/** The projects list's per-work-project package counts. Silent while loading and
 * on failure — no answer, no numbers. */
export function WorkProjectPackageNote({ project }: { readonly project: ControlProject }) {
  const gateway = useGateway();
  const [summary, setSummary] = useState<{ readonly installed: number; readonly updates: number | null } | null>(null);
  useEffect(() => {
    let alive = true;
    void gateway.packages.listInstalled(project.path).then(outcome => {
      if (alive && outcome.kind === "ok") setSummary(packageConclusion(outcome.result.rows));
    }).catch(() => { /* no note without an answer */ });
    return () => { alive = false; };
  }, [gateway, project.path]);
  if (summary === null) return null;
  return <small className="vua-text-secondary">{format(copy.installedCount, { count: summary.installed })}
    {summary.updates === null || summary.updates === 0 ? null : <> · {format(copy.updatesCount, { count: summary.updates })}</>}</small>;
}
