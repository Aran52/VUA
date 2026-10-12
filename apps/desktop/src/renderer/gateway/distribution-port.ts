import type { DesktopDistributionApiV01 } from "@vua/contracts";
export function createDistributionPort(native?: DesktopDistributionApiV01): DesktopDistributionApiV01 {
  if (native) return native;
  const snapshot = async () => ({ schemaVersion: "0.1" as const, available: false, state: "idle" as const });
  return { snapshot, check: snapshot, stage: snapshot, cancel: snapshot, apply: snapshot, restore: snapshot, subscribe: () => () => {} };
}
