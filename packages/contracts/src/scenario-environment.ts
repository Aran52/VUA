import { isDeploymentParams, type DeploymentIntent, type DeploymentPurpose } from "./environment-deployment.js";
import type { PlayRoute } from "./play-session.js";

export const SCENARIO_ENVIRONMENT_SCHEMA = "vua.scenario-environment/v0.1" as const;
export const SCENARIO_ENVIRONMENT_SCHEMA_V02 = "vua.scenario-environment/v0.2" as const;

export type ScenarioConnection = "usb" | "wifi";
export interface ScenarioSelection { readonly scenario: DeploymentPurpose; readonly connection?: ScenarioConnection | null }
export interface ScenarioSop { readonly schemaVersion: typeof SCENARIO_ENVIRONMENT_SCHEMA; readonly scenario: DeploymentPurpose;
  readonly connection: ScenarioConnection | null; readonly requirements: readonly string[];
  readonly steps: readonly { readonly id: string; readonly title: string; readonly explanation: string; readonly expectedResult: string; readonly automatic: boolean }[];
  readonly performed: "sop_only"; readonly nextAction: "check_environment"; readonly licenseRequiredFor: "editor_execution" | "none" }
export interface ScenarioEnvironmentCheck { readonly schemaVersion: typeof SCENARIO_ENVIRONMENT_SCHEMA; readonly intent: DeploymentIntent;
  readonly observations: readonly { readonly component: string; readonly presence: "verified" | "missing" | "unsuitable" | "detection_failed"; readonly location: string | null; readonly version: string | null }[];
  readonly softwareReady: boolean; readonly licenseState: "not_checked"; readonly performed: "software_check_only";
  readonly nextAction: "follow_sop" | "plan_installation"; readonly installationStarted: false }

export const PLAY_PLATFORMS = ["vrchat", "chilloutvr", "reborn"] as const;
export type PlayPlatformId = (typeof PLAY_PLATFORMS)[number];

export const PLAY_DEVICE_IDS = [
  "pc", "pico", "quest", "steam-frame", "vive-standalone", "vive-wired", "index",
  "psvr2", "bigscreen", "pimax", "varjo", "meganex", "rift-s", "wmr", "other-headset",
] as const;
export type PlayDeviceId = (typeof PLAY_DEVICE_IDS)[number];

export const PLAY_CONNECTION_IDS = [
  "pico-connect", "standalone", "quest-link", "steam-link", "virtual-desktop", "alvr",
  "steam-frame", "vive-hub", "steamvr-direct", "psvr2-app", "bigscreen-driver", "pimax-play",
  "varjo-base", "meganex-configurator", "rift-app", "wmr-steamvr", "steamvr-other",
] as const;
export type PlayConnectionId = (typeof PLAY_CONNECTION_IDS)[number];

export const PLAY_SETUP_STEP_IDS = [
  "steam", "steam-sign-in", "platform", "connection-pc", "connection-headset", "launch", "store-install", "store-launch",
] as const;
export type PlaySetupStepId = (typeof PLAY_SETUP_STEP_IDS)[number];
export type PlayStepActor = "vua" | "open" | "player";

export interface PlaySetupSelectionV02 {
  readonly platform: PlayPlatformId;
  readonly device: PlayDeviceId;
  readonly connection: PlayConnectionId | null;
}

export interface PlayPlatformSpecV02 {
  readonly id: PlayPlatformId;
  readonly name: string;
}

export interface PlayDeviceSpecV02 {
  readonly id: PlayDeviceId;
  readonly name: string;
  readonly connections: readonly PlayConnectionId[];
  readonly store?: "pico" | "meta" | "viveport";
  readonly other?: true;
}

export interface PlayConnectionSpecV02 {
  readonly id: PlayConnectionId;
  readonly name: string;
  readonly component: string | null;
  readonly url: string;
}

export interface PlaySetupCatalogV02 {
  readonly schemaVersion: typeof SCENARIO_ENVIRONMENT_SCHEMA_V02;
  readonly platforms: readonly PlayPlatformSpecV02[];
  readonly devices: readonly PlayDeviceSpecV02[];
  readonly connections: readonly PlayConnectionSpecV02[];
}

export type PlayEngineV02 =
  | { readonly kind: "route"; readonly route: PlayRoute }
  | { readonly kind: "tool"; readonly toolId: PlayPlatformId }
  | { readonly kind: "headset" };

export interface PlayScenarioSopStepV02 {
  readonly id: PlaySetupStepId;
  readonly actor: PlayStepActor;
  readonly title: string;
  readonly explanation: string;
  readonly expectedResult: string;
  readonly automatic: boolean;
  readonly verification: string;
  readonly onFailure: string;
}

export interface PlayScenarioSopV02 {
  readonly schemaVersion: typeof SCENARIO_ENVIRONMENT_SCHEMA_V02;
  readonly selection: PlaySetupSelectionV02;
  readonly engine: PlayEngineV02;
  readonly requirements: readonly string[];
  readonly steps: readonly PlayScenarioSopStepV02[];
  readonly performed: "sop_only";
  readonly nextAction: "check_environment" | "follow_sop";
  readonly sopVersions: { readonly platform: string; readonly device: string; readonly connection: string | null };
  readonly lastVerification: { readonly date: string; readonly environment: string; readonly softwareVersions: readonly string[] };
  readonly sources: readonly { readonly label: string; readonly url: string }[];
}

export type ScenarioEnvironmentRequestV1 = { readonly schemaVersion: 1; readonly requestId: string } & (
  | { readonly method: "environment.listPlaySetups"; readonly params: { readonly schemaVersion: "0.2" } }
  | { readonly method: "environment.getScenarioSop"; readonly params: { readonly schemaVersion: "0.1"; readonly scenario: ScenarioSelection } | { readonly schemaVersion: "0.2"; readonly selection: PlaySetupSelectionV02 } }
  | { readonly method: "environment.checkScenarioEnvironment"; readonly params: { readonly schemaVersion: "0.1"; readonly intent: DeploymentIntent } }
);

const DEVICE_CONNECTIONS: Readonly<Record<PlayDeviceId, readonly PlayConnectionId[]>> = {
  pc: [],
  pico: ["pico-connect", "steam-link", "virtual-desktop", "alvr", "standalone"],
  quest: ["quest-link", "steam-link", "virtual-desktop", "alvr", "standalone"],
  "steam-frame": ["steam-frame"],
  "vive-standalone": ["vive-hub", "steam-link", "virtual-desktop", "alvr", "standalone"],
  "vive-wired": ["steamvr-direct"],
  index: ["steamvr-direct"],
  psvr2: ["psvr2-app"],
  bigscreen: ["bigscreen-driver"],
  pimax: ["pimax-play"],
  varjo: ["varjo-base"],
  meganex: ["meganex-configurator"],
  "rift-s": ["rift-app"],
  wmr: ["wmr-steamvr"],
  "other-headset": ["steamvr-other"],
};

const CONNECTION_COMPONENTS: Readonly<Record<PlayConnectionId, string | null>> = {
  "pico-connect": "pico_runtime",
  standalone: null,
  "quest-link": "oculus_runtime",
  "steam-link": null,
  "virtual-desktop": "virtual_desktop",
  alvr: "alvr",
  "steam-frame": "steamvr",
  "vive-hub": "vive_runtime",
  "steamvr-direct": "steamvr",
  "psvr2-app": "psvr2",
  "bigscreen-driver": "bigscreen_beyond",
  "pimax-play": "pimax_runtime",
  "varjo-base": "varjo_runtime",
  "meganex-configurator": null,
  "rift-app": "oculus_runtime",
  "wmr-steamvr": null,
  "steamvr-other": "steamvr",
};

const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const exact = (v: Record<string, unknown>, keys: readonly string[]) => Object.keys(v).length === keys.length && keys.every(k => Object.hasOwn(v, k));
const nonEmpty = (v: unknown): v is string => typeof v === "string" && v.length > 0;
const httpsUrl = (v: unknown): v is string => typeof v === "string" && /^https:\/\/\S+$/u.test(v);

export function scenarioRequiredComponents(purposes: readonly DeploymentPurpose[]): readonly string[] {
  const components: Record<DeploymentPurpose, readonly string[]> = { desktop_play: ["steam", "vrchat"], pico_pcvr: ["steam", "vrchat", "steamvr", "pico_runtime"], pc_avatar: ["unity_editor"], quest_avatar: ["unity_editor", "android_modules"] };
  return [...new Set(purposes.flatMap(p => components[p]))].sort();
}

export function playSetupRequiredComponents(selection: PlaySetupSelectionV02): readonly string[] {
  if (selection.connection === "standalone") return [];
  const items = ["steam", selection.platform, ...(selection.device === "pc" ? [] : ["steamvr"])];
  const comp = selection.connection ? CONNECTION_COMPONENTS[selection.connection] : null;
  if (comp) items.push(comp);
  return [...new Set(items)].sort();
}

export function expectedPlayEngineV02(selection: PlaySetupSelectionV02): PlayEngineV02 {
  if (selection.connection === "standalone") return { kind: "headset" };
  if (selection.platform === "vrchat" && selection.device === "pc") return { kind: "route", route: "desktop_play" };
  if (selection.platform === "vrchat" && selection.connection === "pico-connect") return { kind: "route", route: "pico_pcvr" };
  return { kind: "tool", toolId: selection.platform };
}

export function isScenarioSelection(v: unknown): v is ScenarioSelection {
  return record(v) && ["desktop_play", "pico_pcvr", "pc_avatar", "quest_avatar"].includes(String(v.scenario))
    && Object.keys(v).every(k => k === "scenario" || k === "connection") && (v.connection === undefined || v.connection === null || v.connection === "usb" || v.connection === "wifi")
    && (v.scenario === "pico_pcvr" || v.connection === undefined || v.connection === null);
}

export function isPlaySetupSelectionV02(v: unknown): v is PlaySetupSelectionV02 {
  if (!record(v) || !exact(v, ["platform", "device", "connection"])) return false;
  if (!PLAY_PLATFORMS.includes(v.platform as PlayPlatformId) || !PLAY_DEVICE_IDS.includes(v.device as PlayDeviceId)) return false;
  if (v.platform === "reborn" && v.device === "pc") return false;
  const allowed = DEVICE_CONNECTIONS[v.device as PlayDeviceId].filter(id => id !== "standalone" || v.platform === "vrchat");
  if (allowed.length === 0) return v.connection === null;
  return typeof v.connection === "string" && allowed.includes(v.connection as PlayConnectionId);
}

export function isScenarioEnvironmentParams(method: unknown, v: unknown): boolean {
  if (!record(v)) return false;
  if (method === "environment.listPlaySetups") {
    return exact(v, ["schemaVersion"]) && v.schemaVersion === "0.2";
  }
  if (method === "environment.getScenarioSop") {
    if (v.schemaVersion === "0.1") return exact(v, ["schemaVersion", "scenario"]) && isScenarioSelection(v.scenario);
    if (v.schemaVersion === "0.2") return exact(v, ["schemaVersion", "selection"]) && isPlaySetupSelectionV02(v.selection);
    return false;
  }
  if (method === "environment.checkScenarioEnvironment") {
    return exact(v, ["schemaVersion", "intent"]) && v.schemaVersion === "0.1" && isDeploymentParams({ intent: v.intent }, false);
  }
  return false;
}

export function isScenarioSop(v: unknown): v is ScenarioSop {
  if (!record(v) || !exact(v, ["schemaVersion", "scenario", "connection", "requirements", "steps", "performed", "nextAction", "licenseRequiredFor"])
    || v.schemaVersion !== SCENARIO_ENVIRONMENT_SCHEMA || !isScenarioSelection({ scenario: v.scenario, connection: v.connection })) return false;
  const required = scenarioRequiredComponents([v.scenario as DeploymentPurpose]);
  return Array.isArray(v.requirements) && JSON.stringify(v.requirements) === JSON.stringify(required) && Array.isArray(v.steps) && v.steps.length >= 3 && v.steps.length <= 32
    && v.steps.every(x => record(x) && exact(x, ["id", "title", "explanation", "expectedResult", "automatic"]) && [x.id, x.title, x.explanation, x.expectedResult].every(nonEmpty) && typeof x.automatic === "boolean")
    && new Set(v.steps.map(x => (x as Record<string, unknown>).id)).size === v.steps.length && v.performed === "sop_only" && v.nextAction === "check_environment"
    && v.licenseRequiredFor === (["pc_avatar", "quest_avatar"].includes(String(v.scenario)) ? "editor_execution" : "none");
}

export function isPlaySetupCatalogV02(v: unknown): v is PlaySetupCatalogV02 {
  if (!record(v) || !exact(v, ["schemaVersion", "platforms", "devices", "connections"]) || v.schemaVersion !== SCENARIO_ENVIRONMENT_SCHEMA_V02) return false;
  if (!Array.isArray(v.platforms) || v.platforms.length !== PLAY_PLATFORMS.length || !Array.isArray(v.devices) || v.devices.length !== PLAY_DEVICE_IDS.length || !Array.isArray(v.connections) || v.connections.length !== PLAY_CONNECTION_IDS.length) return false;
  const platformsOk = v.platforms.every((p, i) => record(p) && exact(p, ["id", "name"]) && p.id === PLAY_PLATFORMS[i] && nonEmpty(p.name));
  const devicesOk = v.devices.every((d, i) => record(d) && Object.keys(d).every(k => ["id", "name", "connections", "store", "other"].includes(k))
    && d.id === PLAY_DEVICE_IDS[i] && typeof d.name === "string" && Array.isArray(d.connections)
    && JSON.stringify(d.connections) === JSON.stringify(DEVICE_CONNECTIONS[PLAY_DEVICE_IDS[i]!])
    && (d.store === undefined || ["pico", "meta", "viveport"].includes(String(d.store)))
    && (d.other === undefined || d.other === true));
  const connectionsOk = v.connections.every((c, i) => record(c) && exact(c, ["id", "name", "component", "url"])
    && c.id === PLAY_CONNECTION_IDS[i] && typeof c.name === "string"
    && (c.component === null || nonEmpty(c.component)) && httpsUrl(c.url));
  return platformsOk && devicesOk && connectionsOk;
}

export function isPlayScenarioSopV02(v: unknown): v is PlayScenarioSopV02 {
  if (!record(v) || !exact(v, ["schemaVersion", "selection", "engine", "requirements", "steps", "performed", "nextAction", "sopVersions", "lastVerification", "sources"])) return false;
  if (v.schemaVersion !== SCENARIO_ENVIRONMENT_SCHEMA_V02 || !isPlaySetupSelectionV02(v.selection)) return false;
  const expectedEngine = expectedPlayEngineV02(v.selection);
  const expectedReq = playSetupRequiredComponents(v.selection);
  if (!record(v.engine) || JSON.stringify(v.engine) !== JSON.stringify(expectedEngine)) return false;
  if (!Array.isArray(v.requirements) || JSON.stringify(v.requirements) !== JSON.stringify(expectedReq)) return false;
  if (!Array.isArray(v.steps) || v.steps.length < 2 || v.steps.length > 16) return false;
  const stepsOk = v.steps.every(s => record(s) && exact(s, ["id", "actor", "title", "explanation", "expectedResult", "automatic", "verification", "onFailure"])
    && PLAY_SETUP_STEP_IDS.includes(s.id as PlaySetupStepId)
    && ["vua", "open", "player"].includes(String(s.actor))
    && [s.title, s.explanation, s.expectedResult, s.verification, s.onFailure].every(nonEmpty)
    && typeof s.automatic === "boolean" && s.automatic === (s.actor === "vua"))
    && new Set(v.steps.map(s => (s as Record<string, unknown>).id)).size === v.steps.length;
  if (!stepsOk || v.performed !== "sop_only") return false;
  if (v.nextAction !== (expectedEngine.kind === "headset" ? "follow_sop" : "check_environment")) return false;
  if (!record(v.sopVersions) || !exact(v.sopVersions, ["platform", "device", "connection"])
    || !nonEmpty(v.sopVersions.platform) || !nonEmpty(v.sopVersions.device)
    || (v.selection.connection === null ? v.sopVersions.connection !== null : !nonEmpty(v.sopVersions.connection))) return false;
  if (!record(v.lastVerification) || !exact(v.lastVerification, ["date", "environment", "softwareVersions"])
    || !/^\d{4}-\d{2}-\d{2}$/u.test(String(v.lastVerification.date)) || !nonEmpty(v.lastVerification.environment)
    || !Array.isArray(v.lastVerification.softwareVersions) || !v.lastVerification.softwareVersions.every(nonEmpty)) return false;
  if (!Array.isArray(v.sources) || v.sources.length < 1 || v.sources.length > 8
    || !v.sources.every(src => record(src) && exact(src, ["label", "url"]) && nonEmpty(src.label) && httpsUrl(src.url))) return false;
  return true;
}

export function isScenarioEnvironmentCheck(v: unknown): v is ScenarioEnvironmentCheck {
  if (!record(v) || !exact(v, ["schemaVersion", "intent", "observations", "softwareReady", "licenseState", "performed", "nextAction", "installationStarted"])
    || v.schemaVersion !== SCENARIO_ENVIRONMENT_SCHEMA || !isDeploymentParams({ intent: v.intent }, false) || !Array.isArray(v.observations)) return false;
  const required = scenarioRequiredComponents((v.intent as DeploymentIntent).purposes);
  return v.observations.length === required.length && v.observations.every((x, i) => record(x) && exact(x, ["component", "presence", "location", "version"]) && x.component === required[i]
    && ["verified", "missing", "unsuitable", "detection_failed"].includes(String(x.presence)) && (x.location === null || typeof x.location === "string") && (x.version === null || typeof x.version === "string"))
    && v.softwareReady === v.observations.every(x => (x as Record<string, unknown>).presence === "verified") && v.licenseState === "not_checked" && v.performed === "software_check_only" && v.installationStarted === false
    && v.nextAction === (v.softwareReady ? "follow_sop" : "plan_installation");
}
