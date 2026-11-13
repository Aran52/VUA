// A pending selection survives page remounts and split host/AMF bundles, without persisting an execution request.
let requestedRecipe: string | null = null;
let recipePersistedHint = false;
const recipeRequestKey = "vua-amf-production-recipe";
export function requestProductionRecipe(recipeId: string): void {
  requestedRecipe = recipeId; recipePersistedHint = false;
  try { sessionStorage.setItem(recipeRequestKey, recipeId); recipePersistedHint = true; } catch { /* Keep the in-memory hint. */ }
}
export function takeRequestedProductionRecipe(): string | null {
  const recipeId = requestedRecipe, persisted = recipePersistedHint; requestedRecipe = null; recipePersistedHint = false;
  try {
    const stored = sessionStorage.getItem(recipeRequestKey); sessionStorage.removeItem(recipeRequestKey);
    if (stored !== null) return stored.length > 0 && stored.length <= 160 ? stored : null;
    return persisted ? null : recipeId;
  } catch { return recipeId; }
}

// Home's "new line": the workshop opens on the start page instead of the last line it showed.
// Host and independently bundled AMF can have separate copies of this module, so the hint is stored.
const newLineKey = "vua-amf-production-new-line";
export function requestNewProductionLine(sourceIds?: readonly string[]): void {
  try { sessionStorage.setItem(newLineKey, JSON.stringify(sourceIds ?? [])); } catch { /* The workshop opens on its last line instead. */ }
}
/** The asked-for materials, or null when no new line was asked for. */
export function takeRequestedNewProductionLine(): readonly string[] | null {
  try {
    const asked = sessionStorage.getItem(newLineKey);
    sessionStorage.removeItem(newLineKey);
    if (asked === null) return null;
    const value: unknown = JSON.parse(asked);
    return Array.isArray(value) && value.every(id => typeof id === "string") ? value as string[] : [];
  } catch { return null; }
}

// Full auto continues into the 360° preview when the player follows a finished line to Finished Avatars.
const turntableKey = "vua-amf-production-turntable";
export function requestTurntable(runId: string): void {
  try { sessionStorage.setItem(turntableKey, runId); } catch { /* The button under the stage still offers it. */ }
}
export function takeRequestedTurntable(): string | null {
  try { const runId = sessionStorage.getItem(turntableKey); sessionStorage.removeItem(turntableKey); return runId; } catch { return null; }
}

// The run an AI request acts on: taken when the production page mounts, or delivered to it while open.
let requestedRun: string | null = null;
let runPersisted = false;
const runRequestKey = "vua-amf-production-run-request";
const runListeners = new Set<(runId: string) => void>();
export function requestProductionRun(runId: string): void {
  requestedRun = runId; runPersisted = false;
  // Host and independently bundled AMF can have separate copies of this module.
  // The session hint bridges them; it selects a view and never retries production.
  try { sessionStorage.setItem(runRequestKey, runId); runPersisted = true; } catch { /* Keep the in-memory hint. */ }
  for (const listener of runListeners) listener(runId);
}
export function takeRequestedProductionRun(): string | null {
  const runId = requestedRun, persisted = runPersisted; requestedRun = null; runPersisted = false;
  try {
    const stored = sessionStorage.getItem(runRequestKey); sessionStorage.removeItem(runRequestKey);
    if (stored !== null) return /^[a-zA-Z0-9-]{1,100}$/u.test(stored) ? stored : null;
    // Another bundle may already have consumed a persisted hint.
    return persisted ? null : runId;
  } catch { return runId; }
}
export function onProductionRunRequested(listener: (runId: string) => void): () => void {
  runListeners.add(listener); return () => { runListeners.delete(listener); };
}
