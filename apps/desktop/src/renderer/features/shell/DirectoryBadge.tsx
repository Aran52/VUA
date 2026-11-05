/** The directory says when a line needs the player, from any page: a count for lines to review or
 * fix, a small mark while lines only run. It sits in the item's own padding, so the item keeps its size. */
import { currentLocale, format } from "../../i18n/index.ts";
import { structureCopy } from "../../i18n/structure-copy.ts";
import type { LineSummary } from "./use-line-summary.ts";

const copy = structureCopy[currentLocale].badge;

export function DirectoryBadge({ lines }: { lines: LineSummary }) {
  const needs = lines.waiting + lines.attention;
  if (needs === 0 && lines.running === 0) return null;
  const label = [
    lines.attention > 0 ? format(copy.attention, { count: lines.attention }) : null,
    lines.waiting > 0 ? format(copy.waiting, { count: lines.waiting }) : null,
    lines.running > 0 ? format(copy.running, { count: lines.running }) : null,
  ].filter(Boolean).join(" · ");
  const tone = lines.attention > 0 ? "attention" : lines.waiting > 0 ? "waiting" : "running";
  return <span className="vua-directory-badge" data-tone={tone} title={label}>
    <span className="vua-directory-badge__label">{label}</span>
    {needs > 0 ? <span aria-hidden="true">{needs}</span> : null}
  </span>;
}
