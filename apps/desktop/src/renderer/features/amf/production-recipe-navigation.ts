// A pending selection survives page remounts, without persisting an execution request.
let requestedRecipe: string | null = null;
export function requestProductionRecipe(recipeId: string): void { requestedRecipe = recipeId; }
export function takeRequestedProductionRecipe(): string | null {
  const recipeId = requestedRecipe; requestedRecipe = null; return recipeId;
}

// Home's "new line": the workshop opens on the start page instead of the last line it showed.
// Host and independently bundled AMF can have separate copies of this module, so the hint is stored.
const newLineKey = "vua-amf-production-new-line";
export function requestNewProductionLine(): void {
  try { sessionStorage.setItem(newLineKey, "1"); } catch { /* The workshop opens on its last line instead. */ }
}
export function takeRequestedNewProductionLine(): boolean {
  try { const asked = sessionStorage.getItem(newLineKey) !== null; sessionStorage.removeItem(newLineKey); return asked; } catch { return false; }
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
