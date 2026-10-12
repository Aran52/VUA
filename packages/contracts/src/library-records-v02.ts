export interface LibraryCardTargetV02 { readonly kind: "entry" | "product"; readonly id: string }
export interface LibraryRemoveCardsV02 { readonly schemaVersion: "0.2"; readonly targets: readonly LibraryCardTargetV02[] }
export function isLibraryRemoveCardsV02(v: unknown): v is LibraryRemoveCardsV02 {
  if (typeof v !== "object" || v === null || Array.isArray(v)) return false;
  const p = v as Record<string, unknown>;
  if (Object.keys(p).length !== 2 || p.schemaVersion !== "0.2" || !Array.isArray(p.targets) || p.targets.length === 0 || p.targets.length > 200) return false;
  const keys = new Set<string>();
  return p.targets.every((target: unknown) => {
    if (typeof target !== "object" || target === null || Array.isArray(target)) return false;
    const t = target as Record<string, unknown>;
    if (Object.keys(t).length !== 2 || typeof t.kind !== "string" || !["entry", "product"].includes(t.kind) || typeof t.id !== "string" || !/^[A-Za-z0-9_.:-]{1,128}$/.test(t.id)
      || t.kind === "product" && !/^booth:[0-9]+$/.test(t.id)) return false;
    const key = `${t.kind}:${t.id}`; if (keys.has(key)) return false; keys.add(key); return true;
  });
}
