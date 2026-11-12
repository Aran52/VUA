/** Candidate N2 installer-tool v0.1: fixed official-installer launch and observation.
 * Acquisition and silent NSIS installation run through environment-deployment v0.1. */
export const INSTALLER_TOOL_SCHEMA = "vua.installer-tool/v0.1" as const;
export const INSTALLER_TOOL_IDS = ["slimevr", "vrct", "vrcx", "vrcs"] as const;
export type InstallerToolId = typeof INSTALLER_TOOL_IDS[number];
export const INSTALLER_TOOL_NAMES: Readonly<Record<InstallerToolId, string>> = {
  slimevr: "SlimeVR Server",
  vrct: "VRCT",
  vrcx: "VRCX",
  vrcs: "VRCS",
};
export const INSTALLER_TOOL_ACTIONS = ["start", "stop", "cancel"] as const;
export type InstallerToolAction = typeof INSTALLER_TOOL_ACTIONS[number];
export interface InstallerToolSnapshot {
  readonly schemaVersion: typeof INSTALLER_TOOL_SCHEMA;
  readonly toolId: InstallerToolId;
  readonly capturedAt: string;
  readonly presence: "installed" | "missing" | "incomplete" | "unknown";
  readonly location: string | null;
  readonly version: string | null;
  readonly running: boolean;
  readonly canStop: boolean;
  readonly activity: "idle" | "starting" | "stopping";
  readonly issue: "not_installed" | "handoff_failed" | "start_timeout" | "close_failed" | "close_timeout" | null;
}
export interface InstallerToolResult {
  readonly installerTool: InstallerToolSnapshot;
}
function keys(v: unknown, names: readonly string[]): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v)
    && Object.keys(v).length === names.length && names.every(key => Object.hasOwn(v, key));
}
export function isInstallerToolParams(value: unknown, command: boolean): boolean {
  return keys(value, command ? ["toolId", "action"] : ["toolId"])
    && INSTALLER_TOOL_IDS.includes(value.toolId as InstallerToolId)
    && (!command || INSTALLER_TOOL_ACTIONS.includes(value.action as InstallerToolAction));
}
export function isInstallerToolResult(value: unknown): value is InstallerToolResult {
  if (!keys(value, ["installerTool"]) || !keys(value.installerTool, [
    "schemaVersion", "toolId", "capturedAt", "presence", "location", "version",
    "running", "canStop", "activity", "issue",
  ])) return false;
  const v = value.installerTool;
  return v.schemaVersion === INSTALLER_TOOL_SCHEMA
    && INSTALLER_TOOL_IDS.includes(v.toolId as InstallerToolId)
    && typeof v.capturedAt === "string" && v.capturedAt.length > 0
    && ["installed", "missing", "incomplete", "unknown"].includes(v.presence as string)
    && (v.location === null || (typeof v.location === "string" && v.location.length > 0))
    && (v.version === null || (typeof v.version === "string" && v.version.length > 0))
    && typeof v.running === "boolean" && typeof v.canStop === "boolean" && (!v.canStop || v.running)
    && ["idle", "starting", "stopping"].includes(v.activity as string)
    && (v.issue === null || ["not_installed", "handoff_failed", "start_timeout", "close_failed", "close_timeout"].includes(v.issue as string))
    && (v.activity !== "stopping" || v.canStop) && (v.activity !== "starting" || !v.running);
}
