import type {
  EacProbeSnapshotV02,
  EacRecoveryStepIdV02,
  GuideTargetV1,
} from "@vua/contracts";
import { format, type Strings } from "../../i18n/index.ts";

export const EAC_UNLOADING_LONG_MS = 5 * 60_000;
export const EAC_DIALOG_POLL_MS = 5_000;

export type EacGuideSectionId =
  | "eac-30005-occupied"
  | "eac-not-installed"
  | "eac-1275-driver-blocked"
  | "eac-untrusted-system-file"
  | "eac-connection-lost"
  | "eac-sdk-multi-instance";

export function unloadingElapsedMs(snapshot: EacProbeSnapshotV02, nowMs: number): number {
  let earliest: number | null = null;
  for (const service of snapshot.services) {
    if (service.firstObservedAt === null) continue;
    const parsed = Date.parse(service.firstObservedAt);
    if (!Number.isFinite(parsed)) continue;
    if (earliest === null || parsed < earliest) {
      earliest = parsed;
    }
  }
  if (earliest === null) return 0;
  return Math.max(0, nowMs - earliest);
}

export function isUnloadingLong(snapshot: EacProbeSnapshotV02, nowMs: number): boolean {
  return snapshot.conclusion === "unloading" && unloadingElapsedMs(snapshot, nowMs) >= EAC_UNLOADING_LONG_MS;
}

export function unloadingDisplayMinutes(snapshot: EacProbeSnapshotV02, nowMs: number): number {
  return Math.max(1, Math.floor(unloadingElapsedMs(snapshot, nowMs) / 60_000));
}

export function shouldShowInlineEacRow(snapshot: EacProbeSnapshotV02 | null): boolean {
  return (
    snapshot !== null
    && snapshot.conclusion !== "ready"
    && snapshot.conclusion !== "session_active"
  );
}

export function eacConclusionHeadline(
  snapshot: EacProbeSnapshotV02,
  copy: Strings["eac"],
  nowMs: number,
  readyAfterWait = false,
): string {
  if (readyAfterWait && snapshot.conclusion === "ready") {
    return copy.conclusions.ready_after_wait;
  }
  if (snapshot.conclusion === "unloading") {
    if (isUnloadingLong(snapshot, nowMs)) {
      return copy.conclusions.unloading_long;
    }
    return format(copy.conclusions.unloading, {
      minutes: unloadingDisplayMinutes(snapshot, nowMs),
    });
  }
  return copy.conclusions[snapshot.conclusion];
}

export function eacEncyclopediaSection(snapshot: EacProbeSnapshotV02): EacGuideSectionId {
  for (const finding of snapshot.logFindings) {
    if (finding.code === "start_service_1275") return "eac-1275-driver-blocked";
    if (finding.code === "untrusted_system_file") return "eac-untrusted-system-file";
    if (finding.code === "connection_lost") return "eac-connection-lost";
  }
  switch (snapshot.conclusion) {
    case "not_installed":
    case "install_damaged":
      return "eac-not-installed";
    case "session_active":
      return "eac-sdk-multi-instance";
    case "unloading":
    case "residual_process":
    case "steam_stale":
      return "eac-30005-occupied";
    case "ready":
    case "unknown":
      return snapshot.logFindings.some((item) => item.code === "not_installed")
        ? "eac-not-installed"
        : "eac-30005-occupied";
  }
}

export function eacGuideTarget(snapshot: EacProbeSnapshotV02): GuideTargetV1 {
  return {
    topic: "guide-start",
    section: eacEncyclopediaSection(snapshot),
  };
}

export interface SelectedLadderStep {
  readonly current: EacRecoveryStepIdV02 | null;
  readonly index: number;
  readonly hasNext: boolean;
}

export function selectLadderStep(
  snapshot: EacProbeSnapshotV02,
  stepOffset: number,
  nowMs: number,
): SelectedLadderStep {
  const { plan } = snapshot;
  if (plan.length === 0) {
    return { current: null, index: 0, hasNext: false };
  }
  const baseIndex =
    stepOffset === 0 && isUnloadingLong(snapshot, nowMs)
      ? Math.max(0, plan.indexOf("restart_pc"))
      : stepOffset;
  const clamped = Math.min(Math.max(0, baseIndex), plan.length - 1);
  return {
    current: plan[clamped] ?? null,
    index: clamped,
    hasNext: clamped + 1 < plan.length,
  };
}

export function formatEvidenceTime(iso: string): string {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return iso;
  return parsed.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}
