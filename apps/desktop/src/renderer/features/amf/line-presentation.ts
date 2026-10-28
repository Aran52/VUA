/** A compact source label is a fallback, never an inferred Avatar identity. */
export function compactSourceLabel(name: string): string {
  return name.replace(/\.(zip|unitypackage)$/iu, "")
    .replace(/[_ -]+v?\d+(?:\.\d+)+(?:[_ -].*)?$/iu, "")
    .replace(/_/gu, " ").trim();
}
