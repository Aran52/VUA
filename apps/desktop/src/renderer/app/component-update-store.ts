import { useEffect, useState } from "react";
import type { ComponentUpdateSnapshotV02, DesktopComponentUpdatesApiV02, UpdateComponentV02 } from "@vua/contracts";
import { useGateway } from "../gateway/index.ts";
import { storageKeys } from "./storage-keys.ts";

const eventName = "vua-component-update-preference";
const key = (component: UpdateComponentV02) => `vua-component-update-${component}-enabled`;
export function componentCheckEnabled(component: UpdateComponentV02): boolean {
  try {
    const saved = localStorage.getItem(key(component));
    return saved === null && component === "host" ? localStorage.getItem(storageKeys.updateCheckEnabled) !== "off" : saved !== "off";
  } catch { return true; }
}
export function setComponentCheckEnabled(component: UpdateComponentV02, value: boolean): void {
  try { localStorage.setItem(key(component), value ? "on" : "off"); } catch { /* The current session remains usable. */ }
  window.dispatchEvent(new Event(eventName));
}
export async function checkComponents(port: DesktopComponentUpdatesApiV02 | undefined, manual = false): Promise<void> {
  if (!port) return;
  await Promise.allSettled((["host", "amf"] as const).filter(component => manual || componentCheckEnabled(component)).map(component => port.check(component)));
}
export function useComponentUpdate(component: UpdateComponentV02) {
  const port = useGateway().componentUpdates;
  const [view, setView] = useState<ComponentUpdateSnapshotV02>({ schemaVersion: "0.2", component, available: false, state: "idle", rollbackAvailable: false });
  const [automatic, setAutomatic] = useState(() => componentCheckEnabled(component));
  useEffect(() => {
    const sync = () => setAutomatic(componentCheckEnabled(component));
    window.addEventListener(eventName, sync); window.addEventListener("storage", sync);
    return () => { window.removeEventListener(eventName, sync); window.removeEventListener("storage", sync); };
  }, [component]);
  useEffect(() => {
    let mounted = true;
    const unsubscribe = port?.subscribe(value => { if (mounted && value.component === component) setView(value); });
    void port?.snapshot(component).then(value => { if (mounted) setView(value); }).catch(() => {
      if (mounted) setView(previous => ({ ...previous, state: "failed", error: "update_unavailable" }));
    });
    return () => { mounted = false; unsubscribe?.(); };
  }, [port, component]);
  const check = () => port?.check(component);
  return { view, port, automatic, check, setAutomatic: (on: boolean) => setComponentCheckEnabled(component, on) };
}
