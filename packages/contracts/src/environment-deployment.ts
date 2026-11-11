/** Deployment v0.1: user intent and backend facts. No shell text or executable supplied by UI.
 * This family extends the existing Gateway envelope; frozen environment inspection is unchanged. */
import { INSTALLER_TOOL_IDS, type InstallerToolId } from "./installer-tool.js";

export const DEPLOYMENT_SCHEMA = "vua.environment-deployment/v0.1" as const;
export const UNITY_HUB_INSTALL_LINK = "unityhub://2022.3.22f1/887be4894c44" as const;
export const DEPLOYMENT_PURPOSES = ["desktop_play", "pico_pcvr", "pc_avatar", "quest_avatar"] as const;
export type DeploymentPurpose = typeof DEPLOYMENT_PURPOSES[number];
export type PicoInstallRegion = "china_mainland" | "other";
export interface DeploymentIntent {
  readonly purposes: readonly DeploymentPurpose[];
  readonly editorRoot: string;
  readonly useMirrors?: boolean;
  readonly picoRegion?: PicoInstallRegion;
}
export interface ToolDeploymentIntent {
  readonly purposes?: readonly DeploymentPurpose[];
  readonly tools: readonly InstallerToolId[];
  readonly reinstallTools?: readonly InstallerToolId[];
  readonly editorRoot: string;
  readonly useMirrors?: boolean;
  readonly picoRegion?: PicoInstallRegion;
}
export interface EditorDownloadPolicy {
  readonly region: "china_mainland" | "other" | "unknown";
  readonly mirrorsEnabled: boolean;
  readonly sources: readonly ("official" | "nounitycn")[];
  /** Absent only in older Candidate plans. New plans bind this order to consent. */
  readonly editorEditions?: readonly ["global", "china"];
  readonly hubFallbackUrl: typeof UNITY_HUB_INSTALL_LINK;
}
export type DeploymentPresence = "verified" | "missing" | "unsuitable" | "detection_failed";
export const DEPLOYMENT_ACTIONS = ["retain", "manual_install", "inspect", "install_editor", "add_android_modules", "install_unity_cli", "install_steam", "install_pico_runtime"] as const;
export type DeploymentAction = typeof DEPLOYMENT_ACTIONS[number];
export const TOOL_DEPLOYMENT_ACTIONS = [...DEPLOYMENT_ACTIONS, "install_official_tool"] as const;
export type ToolDeploymentAction = typeof TOOL_DEPLOYMENT_ACTIONS[number];
export const OFFICIAL_TOOL_URLS: Readonly<Record<InstallerToolId, string>> = {
  slimevr: "https://github.com/SlimeVR/SlimeVR-Installer",
  vrct: "https://github.com/misyaguziya/VRCT",
  vrcx: "https://github.com/vrcx-team/VRCX",
  vrcs: "https://github.com/Dreaminko/VRCS",
};
export interface DeploymentStep {
  readonly component: string;
  readonly action: DeploymentAction;
  readonly reason: DeploymentPresence;
  readonly location: string | null;
  readonly version: string | null;
  readonly officialUrl: string | null;
}
export interface ToolDeploymentStep {
  readonly component: string;
  readonly action: ToolDeploymentAction;
  readonly reason: DeploymentPresence;
  readonly location: string | null;
  readonly version: string | null;
  readonly officialUrl: string | null;
}
export interface DeploymentPlan {
  readonly schemaVersion: typeof DEPLOYMENT_SCHEMA;
  readonly intent: DeploymentIntent;
  readonly steps: readonly DeploymentStep[];
  readonly digest: string;
  readonly prerequisitesReady: boolean;
  readonly installer: DeploymentInstaller | null;
  readonly downloadPolicy?: EditorDownloadPolicy;
}
export interface ToolDeploymentPlan {
  readonly schemaVersion: typeof DEPLOYMENT_SCHEMA;
  readonly intent: ToolDeploymentIntent;
  readonly steps: readonly ToolDeploymentStep[];
  readonly digest: string;
  readonly prerequisitesReady: boolean;
  readonly installer: DeploymentInstaller | null;
  readonly downloadPolicy?: EditorDownloadPolicy;
}
export interface DeploymentInstaller {
  readonly kind: "unity_cli" | "hub_cli" | "unity_cli_bootstrap";
  readonly location: string;
  readonly version: string;
  readonly fileSha256: string;
  readonly editorRoot: string;
}
export interface DeploymentPlanResult {
  readonly deploymentPlan: DeploymentPlan;
}
export interface ToolDeploymentPlanResult {
  readonly deploymentPlan: ToolDeploymentPlan;
}
export interface DeploymentAccepted {
  readonly schemaVersion: typeof DEPLOYMENT_SCHEMA;
  readonly operation: "environment.executeDeployment";
  readonly taskId: string;
  readonly correlationId: string;
}
export interface DeploymentPlanParams {
  readonly intent: DeploymentIntent | ToolDeploymentIntent;
}
export interface DeploymentExecuteParams extends DeploymentPlanParams {
  readonly confirmedDigest: string;
}

export const DEPLOYMENT_PHASES = ["started", "resolving_source", "downloading", "verifying", "installing", "inspecting", "registering", "source_failed", "installation_failed", "cache_rejected", "verified"] as const;
export interface DeploymentProgress {
  readonly component: string;
  readonly action: ToolDeploymentAction;
  readonly phase: typeof DEPLOYMENT_PHASES[number];
  readonly source?: "official" | "nounitycn";
  readonly editorVersion?: "2022.3.22f1" | "2022.3.22f1c1";
  readonly completedBytes?: number;
  readonly totalBytes?: number;
  readonly cause?: string;
}

const KNOWN_COMPONENTS = ["steam", "vrchat", "steamvr", "pico_runtime", "unity_hub", "unity_cli", "unity_editor", "android_modules", ...INSTALLER_TOOL_IDS] as const;

/** Decode only bounded adapter facts carried inside the existing task-progress params.
 * Step counts remain separate from bytes; unknown-length transfers have no percentage. */
export function readDeploymentProgress(v: unknown): DeploymentProgress | null {
  if (!record(v) || v.operation !== "environment.executeDeployment"
    || typeof v.component !== "string" || !(KNOWN_COMPONENTS as readonly string[]).includes(v.component)
    || !TOOL_DEPLOYMENT_ACTIONS.some(action => action === v.action)
    || !DEPLOYMENT_PHASES.some(p => p === v.phase)
    || (v.source !== undefined && v.source !== "official" && v.source !== "nounitycn")
    || (v.editorVersion !== undefined && v.editorVersion !== "2022.3.22f1" && v.editorVersion !== "2022.3.22f1c1")
    || (v.cause !== undefined && (typeof v.cause !== "string" || !/^vua\.deployment\.[a-z_]{1,64}$/.test(v.cause)))) return null;
  for (const key of ["completedBytes", "totalBytes"] as const) {
    if (v[key] !== undefined && (typeof v[key] !== "number" || !Number.isSafeInteger(v[key]) || v[key] < 0)) return null;
  }
  if (typeof v.totalBytes === "number" && (v.totalBytes === 0 || typeof v.completedBytes !== "number" || v.completedBytes > v.totalBytes)) return null;
  const vendorComponent = vendorInstallComponent(v.action);
  const officialTool = v.action === "install_official_tool";
  if (vendorComponent !== null && (v.component !== vendorComponent || v.editorVersion !== undefined
    || (v.source !== undefined && v.source !== "official"))) return null;
  if (officialTool && (!INSTALLER_TOOL_IDS.includes(v.component as InstallerToolId) || v.editorVersion !== undefined
    || (v.source !== undefined && v.source !== "official"))) return null;
  if (v.phase === "source_failed" && (v.source === undefined || v.cause === undefined)) return null;
  if (v.phase === "installation_failed" && (v.cause === undefined
    || (vendorComponent === null && !officialTool && v.editorVersion === undefined))) return null;
  return { component: v.component, action: v.action as ToolDeploymentAction, phase: v.phase as DeploymentProgress["phase"],
    ...(v.source === undefined ? {} : { source: v.source }),
    ...(v.editorVersion === undefined ? {} : { editorVersion: v.editorVersion }),
    ...(v.completedBytes === undefined ? {} : { completedBytes: v.completedBytes as number }),
    ...(v.totalBytes === undefined ? {} : { totalBytes: v.totalBytes as number }),
    ...(v.cause === undefined ? {} : { cause: v.cause as string }) };
}

function record(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}
function keys(v: Record<string, unknown>, expected: readonly string[]): boolean {
  return Object.keys(v).length === expected.length && expected.every(k => Object.hasOwn(v, k));
}
function vendorInstallComponent(action: unknown): "steam" | "pico_runtime" | null {
  return action === "install_steam" ? "steam" : action === "install_pico_runtime" ? "pico_runtime" : null;
}
function validEditorRoot(editorRoot: unknown): editorRoot is string {
  return typeof editorRoot === "string" && new TextEncoder().encode(editorRoot).length <= 240
    && /^[a-z]:[\\/].+/i.test(editorRoot) && !/[\p{Cc}"*?<>|]/u.test(editorRoot)
    && !editorRoot.slice(2).includes(":")
    && !editorRoot.slice(3).split(/[\\/]/).some(p => p.length === 0 || /[. ]$/.test(p)
      || /^(CON|PRN|AUX|NUL|CONIN\$|CONOUT\$|COM[1-9¹²³]|LPT[1-9¹²³])$/i.test(p.split(".")[0]!));
}
/** Closed user intent. Windows aliases are refused before a request crosses the Gateway;
 * the adapter additionally checks actual filesystem ancestors before an automatic write. */
export function isDeploymentIntent(v: unknown): v is DeploymentIntent {
  return record(v) && keys(v, ["purposes", "editorRoot", ...(Object.hasOwn(v, "useMirrors") ? ["useMirrors"] : []), ...(Object.hasOwn(v, "picoRegion") ? ["picoRegion"] : [])])
    && (!Object.hasOwn(v, "picoRegion") || v.picoRegion === "china_mainland" || v.picoRegion === "other")
    && (!Object.hasOwn(v, "useMirrors") || typeof v.useMirrors === "boolean") && Array.isArray(v.purposes)
    && v.purposes.length >= 1 && v.purposes.length <= 4 && new Set(v.purposes).size === v.purposes.length
    && v.purposes.every(p => DEPLOYMENT_PURPOSES.some(allowed => p === allowed))
    && validEditorRoot(v.editorRoot);
}
export function isToolDeploymentIntent(v: unknown): v is ToolDeploymentIntent {
  if (!record(v)) return false;
  const expected = [
    "tools", "editorRoot",
    ...(Object.hasOwn(v, "purposes") ? ["purposes"] : []),
    ...(Object.hasOwn(v, "reinstallTools") ? ["reinstallTools"] : []),
    ...(Object.hasOwn(v, "useMirrors") ? ["useMirrors"] : []),
    ...(Object.hasOwn(v, "picoRegion") ? ["picoRegion"] : []),
  ];
  if (!keys(v, expected)) return false;
  if (Object.hasOwn(v, "picoRegion") && v.picoRegion !== "china_mainland" && v.picoRegion !== "other") return false;
  if (Object.hasOwn(v, "useMirrors") && typeof v.useMirrors !== "boolean") return false;
  if (Object.hasOwn(v, "purposes")) {
    if (!Array.isArray(v.purposes) || v.purposes.length > 4 || new Set(v.purposes).size !== v.purposes.length
      || !v.purposes.every(p => DEPLOYMENT_PURPOSES.some(allowed => p === allowed))) return false;
  }
  if (!Array.isArray(v.tools) || v.tools.length < 1 || v.tools.length > 4
    || new Set(v.tools).size !== v.tools.length
    || !v.tools.every(t => INSTALLER_TOOL_IDS.includes(t as InstallerToolId))) return false;
  if (Object.hasOwn(v, "reinstallTools")) {
    if (!Array.isArray(v.reinstallTools) || v.reinstallTools.length > 4
      || new Set(v.reinstallTools).size !== v.reinstallTools.length
      || !v.reinstallTools.every(t => (v.tools as unknown[]).includes(t))) return false;
  }
  return validEditorRoot(v.editorRoot);
}
/** Keep command identity bounds identical to Rust; do not trim or normalize a replay key. */
export function isDeploymentCommandId(v: unknown): v is string {
  return typeof v === "string" && v.length > 0 && new TextEncoder().encode(v).length <= 128 && !/\p{Cc}/u.test(v);
}
/** Params exclude application-envelope commandId and all renderer-supplied command details. */
export function isDeploymentParams(v: unknown, execute: boolean): boolean {
  return record(v) && keys(v, execute ? ["intent", "confirmedDigest"] : ["intent"])
    && (isDeploymentIntent(v.intent) || isToolDeploymentIntent(v.intent))
    && (!execute || (typeof v.confirmedDigest === "string" && /^[0-9a-f]{64}$/.test(v.confirmedDigest)));
}
/** A receipt acknowledges a task, never successful installation. */
export function isDeploymentAccepted(v: unknown): v is DeploymentAccepted {
  return record(v) && keys(v, ["schemaVersion", "operation", "taskId", "correlationId"])
    && v.schemaVersion === DEPLOYMENT_SCHEMA && v.operation === "environment.executeDeployment"
    && typeof v.taskId === "string" && v.taskId.length > 0
    && typeof v.correlationId === "string" && v.correlationId.length > 0;
}
/** Validate returned facts and selected destinations; the UI never manufactures a ready plan. */
export function isDeploymentPlanResult(v: unknown): v is DeploymentPlanResult {
  if (!record(v) || !keys(v, ["deploymentPlan"]) || !record(v.deploymentPlan)) return false;
  const p = v.deploymentPlan;
  const expectedKeys = ["schemaVersion", "intent", "steps", "digest", "prerequisitesReady", "installer"];
  if (Object.hasOwn(p, "downloadPolicy")) expectedKeys.push("downloadPolicy");
  return keys(p, expectedKeys)
    && p.schemaVersion === DEPLOYMENT_SCHEMA && isDeploymentIntent(p.intent)
    && (!Object.hasOwn(p, "downloadPolicy") || validDownloadPolicy(p.downloadPolicy, p.intent))
    && typeof p.digest === "string" && /^[0-9a-f]{64}$/.test(p.digest)
    && (p.installer === null || (record(p.installer) && keys(p.installer, ["kind", "location", "version", "fileSha256", "editorRoot"])
      && ["unity_cli", "hub_cli", "unity_cli_bootstrap"].includes(String(p.installer.kind)) && typeof p.installer.location === "string" && p.installer.location.length > 0
      && typeof p.installer.version === "string" && p.installer.version.length > 0
      && typeof p.installer.fileSha256 === "string" && /^[0-9a-f]{64}$/.test(p.installer.fileSha256)
      && record(p.intent) && p.installer.editorRoot === p.intent.editorRoot))
    && typeof p.prerequisitesReady === "boolean" && Array.isArray(p.steps) && p.steps.length >= 2 && p.steps.length <= 7
    && new Set(p.steps.map(s => record(s) ? s.component : null)).size === p.steps.length
    && p.prerequisitesReady === p.steps.every(s => record(s) && s.action === "retain")
    && p.steps.every(s => record(s) && keys(s, ["component", "action", "reason", "location", "version", "officialUrl"])
      && typeof s.component === "string" && ["steam", "vrchat", "steamvr", "pico_runtime", "unity_hub", "unity_cli", "unity_editor", "android_modules"].includes(s.component)
      && typeof s.action === "string" && DEPLOYMENT_ACTIONS.some(action => action === s.action)
      && typeof s.reason === "string" && ["verified", "missing", "unsuitable", "detection_failed"].includes(s.reason)
      && ((s.action === "retain") === (s.reason === "verified"))
      && (vendorInstallComponent(s.action) === null || (s.component === vendorInstallComponent(s.action) && s.reason === "missing"))
      && (s.action !== "install_pico_runtime" || (record(p.intent) && (p.intent.picoRegion === "china_mainland" || p.intent.picoRegion === "other")
        && s.officialUrl === (p.intent.picoRegion === "china_mainland" ? "https://www.picoxr.com/cn/software/pico-link" : "https://www.picoxr.com/global/software/pico-link")))
      && (s.action !== "install_unity_cli" || (s.component === "unity_cli" && s.reason === "missing"
        && record(p.installer) && p.installer.kind === "unity_cli_bootstrap"))
      && (!["install_editor", "add_android_modules"].includes(s.action) || (s.reason === "missing"
        && record(p.installer) && ["unity_cli", "hub_cli"].includes(String(p.installer.kind))
        && s.component === (s.action === "install_editor" ? "unity_editor" : "android_modules")))
      && (s.location === null || typeof s.location === "string") && (s.version === null || typeof s.version === "string")
      && (s.component !== "unity_editor" || !record(p.downloadPolicy)
        || s.officialUrl === OFFICIAL_EDITOR_ENTRY)
      && (s.officialUrl === null || (typeof s.officialUrl === "string" && ALLOWED_DESTINATIONS.includes(s.officialUrl))));
}
export function isToolDeploymentPlanResult(v: unknown): v is ToolDeploymentPlanResult {
  if (!record(v) || !keys(v, ["deploymentPlan"]) || !record(v.deploymentPlan)) return false;
  const p = v.deploymentPlan;
  const expectedKeys = ["schemaVersion", "intent", "steps", "digest", "prerequisitesReady", "installer"];
  if (Object.hasOwn(p, "downloadPolicy")) expectedKeys.push("downloadPolicy");
  if (!keys(p, expectedKeys) || p.schemaVersion !== DEPLOYMENT_SCHEMA || !isToolDeploymentIntent(p.intent)) return false;
  const reinstall = new Set(p.intent.reinstallTools ?? []);
  return (!Object.hasOwn(p, "downloadPolicy") || validDownloadPolicy(p.downloadPolicy, p.intent))
    && typeof p.digest === "string" && /^[0-9a-f]{64}$/.test(p.digest)
    && typeof p.prerequisitesReady === "boolean" && Array.isArray(p.steps) && p.steps.length >= 1 && p.steps.length <= 11
    && new Set(p.steps.map(s => record(s) ? s.component : null)).size === p.steps.length
    && p.prerequisitesReady === p.steps.every(s => record(s) && s.action === "retain")
    && p.steps.every(s => record(s) && keys(s, ["component", "action", "reason", "location", "version", "officialUrl"])
      && typeof s.component === "string" && (KNOWN_COMPONENTS as readonly string[]).includes(s.component)
      && typeof s.action === "string" && TOOL_DEPLOYMENT_ACTIONS.some(action => action === s.action)
      && typeof s.reason === "string" && ["verified", "missing", "unsuitable", "detection_failed"].includes(s.reason)
      && (s.action !== "retain" || s.reason === "verified")
      && (s.action !== "install_official_tool" || (INSTALLER_TOOL_IDS.includes(s.component as InstallerToolId)
        && (s.reason === "missing" || (s.reason === "verified" && reinstall.has(s.component as InstallerToolId)))
        && s.officialUrl === OFFICIAL_TOOL_URLS[s.component as InstallerToolId]))
      && (s.location === null || typeof s.location === "string") && (s.version === null || typeof s.version === "string")
      && (s.officialUrl === null || (typeof s.officialUrl === "string" && ALLOWED_DESTINATIONS.includes(s.officialUrl))));
}

function validDownloadPolicy(v: unknown, intent: { readonly useMirrors?: boolean }): v is EditorDownloadPolicy {
  if (!record(v) || !keys(v, ["region", "mirrorsEnabled", "sources", "hubFallbackUrl", ...(Object.hasOwn(v, "editorEditions") ? ["editorEditions"] : [])])) return false;
  if (v.editorEditions !== undefined && (!Array.isArray(v.editorEditions) || v.editorEditions.length !== 2 || v.editorEditions[0] !== "global" || v.editorEditions[1] !== "china")) return false;
  if (!["china_mainland", "other", "unknown"].includes(String(v.region))
    || typeof v.mirrorsEnabled !== "boolean" || v.mirrorsEnabled !== (intent.useMirrors !== false)
    || v.hubFallbackUrl !== UNITY_HUB_INSTALL_LINK || !Array.isArray(v.sources)) return false;
  const expected = !v.mirrorsEnabled ? ["official"] : ["official", "nounitycn"];
  const sources = v.sources;
  return sources.length === expected.length && expected.every((source, index) => sources[index] === source);
}

// The consumer rejects arbitrary Provider URLs. Destinations belong to this closed family.
const MIRROR_EDITOR_ENTRY = "https://www.nounitycn.top/download?v=unityhub%3A%2F%2F2022.3.22f1%2F887be4894c44";
const OFFICIAL_EDITOR_ENTRY = "https://unity.com/releases/editor/whats-new/2022.3.22f1";
const ALLOWED_DESTINATIONS = [
  MIRROR_EDITOR_ENTRY, OFFICIAL_EDITOR_ENTRY,
  "https://store.steampowered.com/about/", "https://store.steampowered.com/app/438100/",
  "https://store.steampowered.com/app/250820/", "https://www.picoxr.com/software/pico-connect",
  "https://www.picoxr.com/cn/software/pico-link", "https://www.picoxr.com/global/software/pico-link",
  "https://docs.unity.com/en-us/unity-cli/use-unity-cli", "https://unity.com/download",
  "https://docs.unity.com/en-us/hub/add-modules",
  ...Object.values(OFFICIAL_TOOL_URLS),
];
