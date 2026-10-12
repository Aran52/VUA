import type { VuaDesktopApiV2 } from "./desktop-gateway.js";
/** First-party installation/update face; independent of frozen provider methods. */
export interface DistributionTargetV01 {
  readonly manifestId: string; readonly amfVersion: string; readonly hostVersion: string;
  readonly compatible: boolean; readonly newer: boolean; readonly downloadBytes: number; readonly releaseUrl: string;
}
export interface DistributionProgressV01 {
  readonly phase: string; readonly completed: number; readonly total: number;
  readonly bytesPerSecond: number; readonly reusedBytes: number; readonly fullPack: boolean;
}
export interface DistributionSnapshotV01 {
  readonly schemaVersion: "0.1"; readonly available: boolean;
  readonly hostVersion?: string | undefined; readonly amfVersion?: string | undefined; readonly runtimeVersion?: string | undefined;
  readonly state: "idle" | "checking" | "downloading" | "staged" | "interrupted" | "failed";
  readonly target?: DistributionTargetV01 | undefined; readonly progress?: DistributionProgressV01 | undefined;
  readonly error?: string | undefined; readonly rollbackAvailable?: boolean | undefined;
  readonly pendingManifestId?: string | undefined; readonly targetAmfVersion?: string | undefined;
}
export interface DesktopDistributionApiV01 {
  snapshot(): Promise<DistributionSnapshotV01>;
  check(): Promise<DistributionSnapshotV01>;
  stage(manifestId: string): Promise<DistributionSnapshotV01>;
  cancel(): Promise<DistributionSnapshotV01>;
  apply(): Promise<DistributionSnapshotV01>;
  restore(): Promise<DistributionSnapshotV01>;
  subscribe(listener: (value: DistributionSnapshotV01) => void): () => void;
}
export interface VuaDesktopApiV3 extends VuaDesktopApiV2 {
  readonly distribution?: DesktopDistributionApiV01;
}
