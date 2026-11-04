/** Disposers are keyed by a Main-issued UI mount, including failed imports. */
const cleanups = new Map<string, Set<() => void>>();
const disposed = new Set<string>();
export function registerAmfGenerationCleanup(instanceId: string, cleanup: () => void): () => void {
  if (!/^[a-f0-9]{32}$/.test(instanceId)) throw Error("invalid_amf_generation");
  if (disposed.has(instanceId)) { cleanup(); return () => {}; }
  let scope = cleanups.get(instanceId); if (!scope) { scope = new Set(); cleanups.set(instanceId, scope); }
  scope.add(cleanup);
  return () => { scope.delete(cleanup); };
}
export function disposeAmfGeneration(instanceId: string): void {
  disposed.add(instanceId);
  const scope = cleanups.get(instanceId); cleanups.delete(instanceId);
  for (const cleanup of scope ?? []) { try { cleanup(); } catch { /* One optional effect cannot retain the others. */ } }
}
