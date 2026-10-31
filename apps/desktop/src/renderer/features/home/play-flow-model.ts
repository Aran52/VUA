import type { DeploymentProgress, PlaySession, ScenarioEnvironmentCheck, ScenarioSop, TaskSnapshotV01 } from "@vua/contracts";

/** The one next action a play route offers. Each phase is derived from observed facts, never stored. */
export type PlayFlowPhase = "unavailable" | "check" | "checking" | "confirm" | "execute" | "running" | "handoff" | "retry" | "launch" | "starting" | "stop";

export interface PlayFlowFacts {
  readonly unavailable: boolean;
  readonly session: PlaySession | null;
  /** A start or stop request is in flight. */
  readonly launchPending: boolean;
  readonly checking: boolean;
  readonly check: ScenarioEnvironmentCheck | null;
  readonly checkFailed: boolean;
  readonly confirmed: boolean;
  /** A plan or execute request is in flight. */
  readonly preparing: boolean;
  readonly prepareFailed: boolean;
  readonly task: TaskSnapshotV01 | null;
  readonly taskActive: boolean;
}

export function playFlowPhase(f: PlayFlowFacts): PlayFlowPhase {
  if (f.unavailable) return "unavailable";
  if (f.session?.canStop) return f.session.state === "starting" || f.session.state === "stopping" ? "starting" : "stop";
  if (f.launchPending) return "starting";
  if (f.taskActive || f.preparing) return "running";
  if (f.checking) return "checking";
  if (f.task?.result?.outcome === "manual_required") return "handoff";
  if (f.checkFailed || f.prepareFailed || f.task?.state === "failed" || f.task?.recoveryDisposition === "inspect_required") return "retry";
  if (f.check === null) return "check";
  if (!f.confirmed) return "confirm";
  return f.check.softwareReady ? "launch" : "execute";
}

export interface PlayChecklistItem { readonly id: string; readonly automatic: boolean; readonly done: boolean }

/** The route's SOP as a checklist. Automatic steps reflect observed facts; the player's own steps
 * count as done once the checklist is confirmed. The recovery note is not a step to tick. */
export function playChecklist(sop: ScenarioSop | null, f: Pick<PlayFlowFacts, "check" | "confirmed" | "session">): PlayChecklistItem[] {
  return (sop?.steps ?? []).filter(step => step.id !== "recovery").map(step => ({
    id: step.id,
    automatic: step.automatic,
    done: step.id === "check" ? f.check !== null
      : step.id === "install" ? f.check?.softwareReady === true
      : step.id === "launch" ? f.session?.state === "running"
      : f.confirmed,
  }));
}

/** Identifies a confirmed checklist: a changed SOP or connection asks for confirmation again. */
export function checklistSignature(sop: ScenarioSop | null, connection: string): string {
  return `${connection}|${(sop?.steps ?? []).map(step => step.id).join(",")}`;
}

export function readConfirmed(raw: string | null, route: string, signature: string): boolean {
  try {
    const value: unknown = JSON.parse(raw ?? "{}");
    return typeof value === "object" && value !== null && !Array.isArray(value) && (value as Record<string, unknown>)[route] === signature;
  } catch { return false; }
}

export function writeConfirmed(raw: string | null, route: string, signature: string): string {
  let value: Record<string, unknown> = {};
  try { const parsed: unknown = JSON.parse(raw ?? "{}"); if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) value = parsed as Record<string, unknown>; }
  catch { /* A damaged record is replaced. */ }
  return JSON.stringify({ ...value, [route]: signature });
}

/** Whole-percent progress when the installer reports byte counts; null while it does not. */
export function progressPercent(step: DeploymentProgress | null): number | null {
  if (!step || step.completedBytes === undefined || step.totalBytes === undefined || step.totalBytes <= 0) return null;
  return Math.max(0, Math.min(100, Math.floor(step.completedBytes / step.totalBytes * 100)));
}
