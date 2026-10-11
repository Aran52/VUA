import type { LibraryCardFacts } from "../../gateway/index.ts";
export type LibraryCornerState = "cloud" | "local" | "linked" | "downloading" | "paused" | "searching" | "uncertain" | "error";
export type LocalSourceSearchState = "searching" | "uncertain" | "matched";
export function libraryCornerState(facts: LibraryCardFacts | undefined, kind: "product" | "local", options: { paused?: boolean; active?: boolean; sourceSearch?: LocalSourceSearchState | undefined } = {}): LibraryCornerState {
  if (options.sourceSearch === "searching") return "searching";
  if (options.active || facts?.operation?.state === "running" && !facts.operation.inspectRequired) return options.paused ? "paused" : "downloading";
  if (facts?.storage.state === "missing" || facts?.storage.state === "partial" || facts?.storage.state === "unreadable" || facts?.storage.state === "changed"
    || facts?.operation?.state === "failed" || facts?.operation?.inspectRequired) return "error";
  if (options.sourceSearch === "uncertain") return "uncertain";
  if (facts?.storage.presentCopies || kind === "local") return kind === "product" || options.sourceSearch === "matched" || facts?.sourceMatch?.basis === "mapping" ? "linked" : "local";
  return "cloud";
}
