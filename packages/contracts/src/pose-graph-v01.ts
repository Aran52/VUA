/** Candidate pose-graph v0.1 (0.2.0): Host-owned local OpenXR, SteamVR external-driver
 * & Safe Mode block, companion pose-tool and Pharos optical+IMU dual-mode hub
 * observation with live freshness-gated telemetry. */
export const POSE_GRAPH_SCHEMA = "vua.pose-graph/v0.1" as const;

export const POSE_SOURCE_IDS = [
  "headset_6dof",
  "pharos",
  "slimevr",
  "pico_motion",
  "lighthouse",
  "space_calibrator",
  "mocopi",
  "haritorax",
  "rebocap",
  "eye_face_osc",
] as const;
export type PoseSourceIdV01 = typeof POSE_SOURCE_IDS[number];

export const POSE_SOURCE_STATES = [
  "active",
  "ready",
  "installed",
  "not_detected",
] as const;
export type PoseSourceStateV01 = typeof POSE_SOURCE_STATES[number];

export const STEAMVR_POSE_DRIVERS = [
  "space_calibrator",
  "slimevr",
  "alvr",
  "virtual_desktop",
  "lighthouse",
  "pharos",
  "mocopi",
  "haritorax",
  "rebocap",
] as const;
export type SteamVrPoseDriverIdV01 = typeof STEAMVR_POSE_DRIVERS[number];

export const PHAROS_WEARABLE_SLOTS = [
  "left_foot",
  "right_foot",
  "waist",
  "chest",
] as const;
export type PharosWearableSlotV01 = typeof PHAROS_WEARABLE_SLOTS[number];

export const PHAROS_NODE_TRACKING_MODES = [
  "optical_imu",
  "passive_retroreflective",
  "imu_only",
] as const;
export type PharosNodeTrackingModeV01 = typeof PHAROS_NODE_TRACKING_MODES[number];

export const PHAROS_TELEMETRY_STATES = [
  "missing",
  "live",
  "stale",
  "malformed",
] as const;
export type PharosTelemetryStateV01 = typeof PHAROS_TELEMETRY_STATES[number];

export const PHAROS_CALIBRATION_STATES = [
  "unknown",
  "uncalibrated",
  "calibrating",
  "calibrated",
] as const;
export type PharosCalibrationStateV01 = typeof PHAROS_CALIBRATION_STATES[number];

export const PHAROS_MODES = [
  "optical_stereo_2x2x2",
  "imu_fusion",
  "passive_retroreflective",
  "panoramic_spatial_anchor",
] as const;
export type PharosModeIdV01 = typeof PHAROS_MODES[number];

export interface OpenXrRuntimeObservationV01 {
  readonly presence: "found" | "missing" | "unreadable";
  readonly runtimeName: string | null;
}

export interface PoseSourceObservationV01 {
  readonly sourceId: PoseSourceIdV01;
  readonly state: PoseSourceStateV01;
  readonly detailCode: string | null;
}

export interface PharosWearableSlotObservationV01 {
  readonly slot: PharosWearableSlotV01;
  readonly state: "not_connected" | "connected";
  readonly trackingMode: PharosNodeTrackingModeV01 | null;
  readonly batteryPercent: number | null;
  readonly footHeadingAvailable: boolean;
}

export interface PharosHubObservationV01 {
  readonly state: "not_detected" | "driver_registered" | "running";
  readonly telemetryState: PharosTelemetryStateV01;
  readonly calibrationState: PharosCalibrationStateV01;
  readonly spatialAnchorReady: boolean;
  readonly activeModes: readonly PharosModeIdV01[];
  readonly wearableNodes: readonly PharosWearableSlotObservationV01[];
  readonly stereoBaseStationCount: number | null;
  readonly panoramicAnchorCount: number | null;
}

export interface PoseGraphSnapshotV01 {
  readonly schemaVersion: typeof POSE_GRAPH_SCHEMA;
  readonly capturedAt: string;
  readonly openxr: OpenXrRuntimeObservationV01;
  readonly steamvrDrivers: readonly SteamVrPoseDriverIdV01[];
  readonly blockedSteamvrDrivers: readonly SteamVrPoseDriverIdV01[];
  readonly lastHeadsetHint: string | null;
  readonly poseSources: readonly PoseSourceObservationV01[];
  readonly pharosHub: PharosHubObservationV01;
}

export interface ObservePoseGraphParamsV01 {
  readonly schemaVersion: "0.1";
}

export interface PoseGraphResultV01 {
  readonly poseGraph: PoseGraphSnapshotV01;
}

export interface PoseGraphGatewayRequestV1 {
  readonly schemaVersion: 1;
  readonly requestId: string;
  readonly method: "environment.observePoseGraph";
  readonly params: ObservePoseGraphParamsV01;
}

function record(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function exactKeys(v: unknown, expected: readonly string[]): v is Record<string, unknown> {
  return record(v) && Object.keys(v).length === expected.length && expected.every(k => Object.hasOwn(v, k));
}

function safeLabel(v: unknown, max = 80): v is string {
  return typeof v === "string" && v.length >= 1 && v.length <= max && !/[\\/:]/u.test(v);
}

function validDriverList(v: unknown): v is readonly SteamVrPoseDriverIdV01[] {
  if (!Array.isArray(v) || v.length > 16) return false;
  if (new Set(v).size !== v.length) return false;
  return v.every(d => STEAMVR_POSE_DRIVERS.includes(d as SteamVrPoseDriverIdV01));
}

export function isObservePoseGraphParamsV01(v: unknown): v is ObservePoseGraphParamsV01 {
  return exactKeys(v, ["schemaVersion"]) && v.schemaVersion === "0.1";
}

export function isPoseGraphSnapshotV01(v: unknown): v is PoseGraphSnapshotV01 {
  if (!exactKeys(v, [
    "schemaVersion",
    "capturedAt",
    "openxr",
    "steamvrDrivers",
    "blockedSteamvrDrivers",
    "lastHeadsetHint",
    "poseSources",
    "pharosHub",
  ])) return false;
  if (v.schemaVersion !== POSE_GRAPH_SCHEMA) return false;
  if (typeof v.capturedAt !== "string" || v.capturedAt.length < 1 || v.capturedAt.length > 64) return false;

  if (!exactKeys(v.openxr, ["presence", "runtimeName"])) return false;
  if (!["found", "missing", "unreadable"].includes(String(v.openxr.presence))) return false;
  if (v.openxr.runtimeName !== null && !safeLabel(v.openxr.runtimeName, 80)) return false;

  if (!validDriverList(v.steamvrDrivers) || !validDriverList(v.blockedSteamvrDrivers)) return false;

  if (v.lastHeadsetHint !== null && !safeLabel(v.lastHeadsetHint, 80)) return false;

  if (!Array.isArray(v.poseSources) || v.poseSources.length !== POSE_SOURCE_IDS.length) return false;
  for (let i = 0; i < POSE_SOURCE_IDS.length; i += 1) {
    const item = v.poseSources[i];
    if (!exactKeys(item, ["sourceId", "state", "detailCode"])) return false;
    if (item.sourceId !== POSE_SOURCE_IDS[i]) return false;
    if (!POSE_SOURCE_STATES.includes(item.state as PoseSourceStateV01)) return false;
    if (item.detailCode !== null && (typeof item.detailCode !== "string" || !/^[a-z0-9_]{1,64}$/u.test(item.detailCode))) {
      return false;
    }
  }

  if (!exactKeys(v.pharosHub, [
    "state",
    "telemetryState",
    "calibrationState",
    "spatialAnchorReady",
    "activeModes",
    "wearableNodes",
    "stereoBaseStationCount",
    "panoramicAnchorCount",
  ])) return false;
  const hub = v.pharosHub;
  if (!["not_detected", "driver_registered", "running"].includes(String(hub.state))) return false;
  if (!PHAROS_TELEMETRY_STATES.includes(hub.telemetryState as PharosTelemetryStateV01)) return false;
  if (!PHAROS_CALIBRATION_STATES.includes(hub.calibrationState as PharosCalibrationStateV01)) return false;
  if (typeof hub.spatialAnchorReady !== "boolean") return false;
  // Honesty invariant: non-live telemetry cannot claim active modes, connected hardware, or spatial anchor readiness.
  if (hub.telemetryState !== "live") {
    if (hub.spatialAnchorReady || hub.calibrationState !== "unknown") return false;
    if (!Array.isArray(hub.activeModes) || hub.activeModes.length !== 0) return false;
    if (hub.stereoBaseStationCount !== null || hub.panoramicAnchorCount !== null) return false;
  }
  if (!Array.isArray(hub.activeModes) || hub.activeModes.length > PHAROS_MODES.length) return false;
  if (new Set(hub.activeModes).size !== hub.activeModes.length) return false;
  if (!hub.activeModes.every(m => PHAROS_MODES.includes(m as PharosModeIdV01))) return false;

  if (!Array.isArray(hub.wearableNodes) || hub.wearableNodes.length !== PHAROS_WEARABLE_SLOTS.length) return false;
  for (let i = 0; i < PHAROS_WEARABLE_SLOTS.length; i += 1) {
    const node = hub.wearableNodes[i];
    if (!exactKeys(node, ["slot", "state", "trackingMode", "batteryPercent", "footHeadingAvailable"])) return false;
    if (node.slot !== PHAROS_WEARABLE_SLOTS[i]) return false;
    if (!["not_connected", "connected"].includes(String(node.state))) return false;
    if (node.trackingMode !== null && !PHAROS_NODE_TRACKING_MODES.includes(node.trackingMode as PharosNodeTrackingModeV01)) {
      return false;
    }
    if (node.batteryPercent !== null && (typeof node.batteryPercent !== "number" || !Number.isSafeInteger(node.batteryPercent) || node.batteryPercent < 0 || node.batteryPercent > 100)) {
      return false;
    }
    if (typeof node.footHeadingAvailable !== "boolean") return false;
    if (node.state === "not_connected" || hub.telemetryState !== "live") {
      if (node.state === "connected" || node.trackingMode !== null || node.batteryPercent !== null || node.footHeadingAvailable) {
        return false;
      }
    }
  }

  const validBoundedCount = (n: unknown, max: number) =>
    n === null || (typeof n === "number" && Number.isSafeInteger(n) && n >= 0 && n <= max);
  if (!validBoundedCount(hub.stereoBaseStationCount, 16) || !validBoundedCount(hub.panoramicAnchorCount, 8)) {
    return false;
  }
  return true;
}

export function isPoseGraphResultV01(v: unknown): v is PoseGraphResultV01 {
  return exactKeys(v, ["poseGraph"]) && isPoseGraphSnapshotV01(v.poseGraph);
}
