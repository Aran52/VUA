/** A compact source label is a fallback, never an inferred Avatar identity. */
export function compactSourceLabel(name: string): string {
  return name.replace(/\.(zip|unitypackage)$/iu, "")
    .replace(/[_ -]+v?\d+(?:\.\d+)+(?:[_ -].*)?$/iu, "")
    .replace(/_/gu, " ").trim();
}

const lineNamesKey = "vua-amf-line-names-v01";
const LINE_NAME_LIMIT = 80;

/** Names the player gave their lines. A line's name is theirs to keep on this computer. */
export function storedLineNames(): Readonly<Record<string, string>> {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(lineNamesKey) ?? "{}");
    if (typeof value !== "object" || value === null || Array.isArray(value)) return {};
    return Object.fromEntries(Object.entries(value).filter((entry): entry is [string, string] => typeof entry[1] === "string" && entry[1].length > 0));
  } catch { return {}; }
}

/** Saves a line's name; an empty name goes back to the one taken from its materials. */
export function rememberLineName(runId: string, name: string): Readonly<Record<string, string>> {
  const next = { ...storedLineNames() };
  const trimmed = name.trim().slice(0, LINE_NAME_LIMIT);
  if (trimmed) next[runId] = trimmed; else delete next[runId];
  try { localStorage.setItem(lineNamesKey, JSON.stringify(next)); } catch { /* The name applies to this session. */ }
  return next;
}

/** The player's name for the line, else the name of its first material. Every page shows this one. */
export function lineName(runId: string, firstSource: string, names: Readonly<Record<string, string>> = storedLineNames()): string {
  return names[runId] ?? compactSourceLabel(firstSource);
}
