/** Independent first-party update channels; windows-distribution v0.2. */
import type { VuaDesktopApiV4 } from "./desktop-ai-tools.js";
export type UpdateComponentV02 = "host" | "amf";
export type UpdateActivationV02 = "restart" | "reload";
export interface ComponentUpdateTargetV02 {
  readonly manifestId: string;
  readonly version: string;
  readonly build: string;
  readonly releaseUrl: string;
  readonly activation: UpdateActivationV02;
  readonly downloadBytes: number;
  readonly reusedBytes: number;
  readonly runtimeBytes: number;
}
export interface ComponentUpdateSnapshotV02 {
  readonly schemaVersion: "0.2";
  readonly component: UpdateComponentV02;
  readonly available: boolean;
  readonly state: "idle" | "checking" | "downloading" | "staged" | "waiting-for-idle" | "activating" | "verifying" | "interrupted" | "failed";
  readonly installedVersion?: string | undefined;
  readonly installedBuild?: string | undefined;
  readonly checkedAt?: string | undefined;
  readonly latestVersion?: string | undefined;
  readonly requiresHost?: string | undefined;
  readonly target?: ComponentUpdateTargetV02 | undefined;
  readonly progress?: { readonly completed: number; readonly total: number; readonly bytesPerSecond: number; readonly reusedBytes: number; readonly fullPack: boolean } | undefined;
  readonly rollbackAvailable: boolean;
  readonly error?: string | undefined;
}
export interface DesktopComponentUpdatesApiV02 {
  snapshot(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02>;
  check(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02>;
  stage(component: UpdateComponentV02, manifestId: string): Promise<ComponentUpdateSnapshotV02>;
  cancel(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02>;
  apply(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02>;
  restore(component: UpdateComponentV02): Promise<ComponentUpdateSnapshotV02>;
  subscribe(listener: (value: ComponentUpdateSnapshotV02) => void): () => void;
}
/** Additive desktop face; earlier host and AMF clients keep their existing APIs. */
export interface VuaDesktopApiV5 extends VuaDesktopApiV4 {
  readonly componentUpdates?: DesktopComponentUpdatesApiV02;
}
