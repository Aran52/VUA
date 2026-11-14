//! Candidate `pose-graph v0.1` (`0.2.0`): Host-owned read-only observation of local OpenXR
//! runtime, SteamVR external pose drivers (`openvrpaths.vrpath` + `driver.vrdrivermanifest` +
//! `steamvr.vrsettings` Safe Mode blocks), companion pose applications, and the Pharos
//! optical+IMU dual-mode tracking hub contract with live `hub-status.json` freshness gating.
//!
//! Safety & honesty invariants:
//! - Read-only local observation; never opens foreign process memory or binds network sockets.
//! - OpenXR runtime names and headset hints are sanitized display labels (`<= 80` UTF-16 code
//!   units) stripped of filesystem path separators (`\`, `/`, `:`).
//! - `steamvrDrivers` and `blockedSteamvrDrivers` emit only closed vocabulary identifiers.
//! - Pharos wearable slots (`left_foot`, `right_foot`, `waist`, `chest`) and station counts
//!   never fabricate connected hardware or synthetic counts when live telemetry is absent or
//!   stale (`> 15 s` old or Pharos process not running).

use crate::environment::EnvironmentRoots;
use crate::time::{rfc3339_offset_millis, Clock};
use crate::win_registry::RegistryHive;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const POSE_GRAPH_SCHEMA: &str = "vua.pose-graph/v0.1";

const OPENXR_REGISTRY_SUBKEY: &str = "SOFTWARE\\Khronos\\OpenXR\\1";
const STEAM_REGISTRY_SUBKEY: &str = "SOFTWARE\\WOW6432Node\\Valve\\Steam";
const MAX_HUB_STATUS_BYTES: u64 = 65_536;
const MAX_DRIVER_MANIFEST_BYTES: u64 = 65_536;
const MAX_OPENVR_JSON_BYTES: u64 = 2 * 1024 * 1024;

fn read_bounded_regular_utf8(path: &Path, max_bytes: u64) -> Option<String> {
    let meta = fs::symlink_metadata(path).ok()?;
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > max_bytes {
        return None;
    }
    let file = File::open(path).ok()?;
    let mut buf = Vec::with_capacity((meta.len() as usize).min(65_536));
    file.take(max_bytes + 1).read_to_end(&mut buf).ok()?;
    if (buf.len() as u64) > max_bytes {
        return None;
    }
    String::from_utf8(buf).ok()
}
const TELEMETRY_MAX_AGE_MS: i64 = 15_000;
const TELEMETRY_MAX_FUTURE_SKEW_MS: i64 = -2_000;

pub const POSE_SOURCE_IDS: [&str; 10] = [
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
];

pub const STEAMVR_POSE_DRIVER_IDS: [&str; 9] = [
    "space_calibrator",
    "slimevr",
    "alvr",
    "virtual_desktop",
    "lighthouse",
    "pharos",
    "mocopi",
    "haritorax",
    "rebocap",
];

const FBT_OPENVR_DRIVER_IDS: [&str; 5] = ["pharos", "slimevr", "mocopi", "haritorax", "rebocap"];

pub const PHAROS_WEARABLE_SLOTS: [&str; 4] = ["left_foot", "right_foot", "waist", "chest"];

const CANDIDATE_EXE_NAMES: [&str; 16] = [
    "vrserver.exe",
    "vrcompositor.exe",
    "pharos.exe",
    "pharos-hub.exe",
    "slimevr.exe",
    "slimevr-server.exe",
    "picoconnect.exe",
    "pico connect.exe",
    "spacecalibrator.exe",
    "mocopi.exe",
    "mocopi receiver.exe",
    "mocopireceiver.exe",
    "haritoraconfigurator.exe",
    "shiftallvrbridge.exe",
    "haritorax-interpreter.exe",
    "rebocap.exe",
];

const EXTRA_CANDIDATE_EXES: [&str; 1] = ["vrcfacetracking.exe"];

/// Returned when OS process liveness enumeration is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoseProbeUnavailable;

impl std::fmt::Display for PoseProbeUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("pose process probe unavailable")
    }
}

impl std::error::Error for PoseProbeUnavailable {}

/// OS process liveness port so unit and route tests run deterministically without reading
/// live host processes.
pub trait PoseProcessProbe: Send + Sync {
    fn is_process_running(&self, exe_name: &str) -> Result<bool, PoseProbeUnavailable>;

    /// Batch query for a closed candidate executable list so OS implementations take a single
    /// kernel snapshot per observation instead of N separate passes.
    fn running_from_candidates(
        &self,
        candidates: &[&str],
    ) -> Result<BTreeSet<String>, PoseProbeUnavailable> {
        let mut out = BTreeSet::new();
        for exe in candidates {
            let lower = exe.to_ascii_lowercase();
            if self.is_process_running(&lower)? {
                out.insert(lower);
            }
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenXrRuntimeObservation {
    pub presence: &'static str,
    pub runtime_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoseSourceObservation {
    pub source_id: &'static str,
    pub state: &'static str,
    pub detail_code: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PharosWearableSlotObservation {
    pub slot: &'static str,
    pub state: &'static str,
    pub tracking_mode: Option<&'static str>,
    pub battery_percent: Option<u8>,
    pub foot_heading_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PharosHubObservation {
    pub state: &'static str,
    pub telemetry_state: &'static str,
    pub calibration_state: &'static str,
    pub spatial_anchor_ready: bool,
    pub active_modes: Vec<&'static str>,
    pub wearable_nodes: Vec<PharosWearableSlotObservation>,
    pub stereo_base_station_count: Option<u64>,
    pub panoramic_anchor_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoseGraphSnapshotV01 {
    pub schema_version: &'static str,
    pub captured_at: String,
    pub openxr: OpenXrRuntimeObservation,
    pub steamvr_drivers: Vec<&'static str>,
    pub blocked_steamvr_drivers: Vec<&'static str>,
    pub fbt_driver_conflict_risk: &'static str,
    pub last_headset_hint: Option<String>,
    pub pose_sources: Vec<PoseSourceObservation>,
    pub pharos_hub: PharosHubObservation,
}

pub struct PoseGraphService {
    roots: EnvironmentRoots,
    probe: Arc<dyn PoseProcessProbe>,
    clock: Arc<dyn Clock>,
}

impl PoseGraphService {
    pub fn new(
        roots: EnvironmentRoots,
        probe: Arc<dyn PoseProcessProbe>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            roots,
            probe,
            clock,
        }
    }

    pub fn observe(&self) -> PoseGraphSnapshotV01 {
        let captured_at = self.clock.now_rfc3339();
        let openxr = inspect_openxr(&self.roots);
        let steamvr_drivers = inspect_openvr_drivers(&self.roots);
        let (last_headset_hint, blocked_steamvr_drivers) = inspect_steamvr_settings(&self.roots);

        let steam_commons = effective_steam_common_roots(&self.roots);
        let steamvr_installed = steam_commons.iter().any(|r| r.join("SteamVR").is_dir());
        let space_calibrator_installed = steam_commons.iter().any(|r| {
            r.join("01spacecalibrator")
                .join("SpaceCalibrator.exe")
                .is_file()
        });
        let vrcft_installed = steam_commons.iter().any(|r| {
            r.join("VRCFaceTracking")
                .join("VRCFaceTracking.exe")
                .is_file()
        });
        let pico_installed = self.roots.vr_runtime_roots.pico.iter().any(|r| r.exists());
        let slimevr_installed = is_slimevr_installed(&self.roots);

        let mut all_candidates =
            Vec::with_capacity(CANDIDATE_EXE_NAMES.len() + EXTRA_CANDIDATE_EXES.len());
        all_candidates.extend_from_slice(&CANDIDATE_EXE_NAMES);
        all_candidates.extend_from_slice(&EXTRA_CANDIDATE_EXES);
        let running_set = self
            .probe
            .running_from_candidates(&all_candidates)
            .unwrap_or_default();
        let running = |exe: &str| running_set.contains(exe);

        let vrserver_running = running("vrserver.exe") || running("vrcompositor.exe");
        let pharos_running = running("pharos.exe") || running("pharos-hub.exe");
        let slimevr_running = running("slimevr.exe") || running("slimevr-server.exe");
        let pico_running = running("picoconnect.exe") || running("pico connect.exe");
        let calibrator_running = running("spacecalibrator.exe");
        let mocopi_running = running("mocopi.exe")
            || running("mocopi receiver.exe")
            || running("mocopireceiver.exe");
        let haritorax_running = running("haritoraconfigurator.exe")
            || running("shiftallvrbridge.exe")
            || running("haritorax-interpreter.exe");
        let rebocap_running = running("rebocap.exe");
        let vrcft_running = running("vrcfacetracking.exe");

        let has_driver = |id: &str| steamvr_drivers.contains(&id);
        let is_blocked = |id: &str| blocked_steamvr_drivers.contains(&id);

        let pharos_hub = inspect_pharos_hub(
            &self.roots,
            pharos_running,
            has_driver("pharos"),
            &captured_at,
        );

        let headset_source = if vrserver_running || pico_running {
            PoseSourceObservation {
                source_id: "headset_6dof",
                state: "active",
                detail_code: Some("runtime_running"),
            }
        } else if openxr.presence == "found" {
            PoseSourceObservation {
                source_id: "headset_6dof",
                state: "ready",
                detail_code: Some("openxr_ready"),
            }
        } else if steamvr_installed {
            PoseSourceObservation {
                source_id: "headset_6dof",
                state: "installed",
                detail_code: Some("steamvr_installed"),
            }
        } else {
            PoseSourceObservation {
                source_id: "headset_6dof",
                state: "not_detected",
                detail_code: None,
            }
        };

        let pharos_source = if pharos_running {
            PoseSourceObservation {
                source_id: "pharos",
                state: "active",
                detail_code: Some(if pharos_hub.telemetry_state == "live" {
                    "telemetry_live"
                } else {
                    "hub_running"
                }),
            }
        } else if is_blocked("pharos") {
            PoseSourceObservation {
                source_id: "pharos",
                state: "installed",
                detail_code: Some("blocked_by_safe_mode"),
            }
        } else if has_driver("pharos") {
            PoseSourceObservation {
                source_id: "pharos",
                state: "ready",
                detail_code: Some("driver_registered"),
            }
        } else {
            PoseSourceObservation {
                source_id: "pharos",
                state: "not_detected",
                detail_code: None,
            }
        };

        let slimevr_source = if slimevr_running {
            PoseSourceObservation {
                source_id: "slimevr",
                state: "active",
                detail_code: Some("process_running"),
            }
        } else if is_blocked("slimevr") {
            PoseSourceObservation {
                source_id: "slimevr",
                state: "installed",
                detail_code: Some("blocked_by_safe_mode"),
            }
        } else if has_driver("slimevr") {
            PoseSourceObservation {
                source_id: "slimevr",
                state: "ready",
                detail_code: Some("driver_registered"),
            }
        } else if slimevr_installed {
            PoseSourceObservation {
                source_id: "slimevr",
                state: "installed",
                detail_code: Some("app_installed"),
            }
        } else {
            PoseSourceObservation {
                source_id: "slimevr",
                state: "not_detected",
                detail_code: None,
            }
        };

        let pico_source = if pico_running {
            PoseSourceObservation {
                source_id: "pico_motion",
                state: "active",
                detail_code: Some("process_running"),
            }
        } else if pico_installed {
            PoseSourceObservation {
                source_id: "pico_motion",
                state: "installed",
                detail_code: Some("app_installed"),
            }
        } else {
            PoseSourceObservation {
                source_id: "pico_motion",
                state: "not_detected",
                detail_code: None,
            }
        };

        let lighthouse_source = if calibrator_running {
            PoseSourceObservation {
                source_id: "lighthouse",
                state: "active",
                detail_code: Some("calibrator_running"),
            }
        } else if has_driver("space_calibrator") {
            PoseSourceObservation {
                source_id: "lighthouse",
                state: "ready",
                detail_code: Some("calibrator_registered"),
            }
        } else if has_driver("lighthouse") {
            PoseSourceObservation {
                source_id: "lighthouse",
                state: "ready",
                detail_code: Some("driver_registered"),
            }
        } else if space_calibrator_installed {
            PoseSourceObservation {
                source_id: "lighthouse",
                state: "installed",
                detail_code: Some("calibrator_installed"),
            }
        } else {
            PoseSourceObservation {
                source_id: "lighthouse",
                state: "not_detected",
                detail_code: None,
            }
        };

        let space_calibrator_source = if calibrator_running {
            PoseSourceObservation {
                source_id: "space_calibrator",
                state: "active",
                detail_code: Some("process_running"),
            }
        } else if is_blocked("space_calibrator") {
            PoseSourceObservation {
                source_id: "space_calibrator",
                state: "installed",
                detail_code: Some("blocked_by_safe_mode"),
            }
        } else if has_driver("space_calibrator") {
            PoseSourceObservation {
                source_id: "space_calibrator",
                state: "ready",
                detail_code: Some("driver_registered"),
            }
        } else if space_calibrator_installed {
            PoseSourceObservation {
                source_id: "space_calibrator",
                state: "installed",
                detail_code: Some("app_installed"),
            }
        } else {
            PoseSourceObservation {
                source_id: "space_calibrator",
                state: "not_detected",
                detail_code: None,
            }
        };

        let mocopi_source = companion_driver_source(
            "mocopi",
            mocopi_running,
            has_driver("mocopi"),
            is_blocked("mocopi"),
        );
        let haritorax_source = companion_driver_source(
            "haritorax",
            haritorax_running,
            has_driver("haritorax"),
            is_blocked("haritorax"),
        );
        let rebocap_source = companion_driver_source(
            "rebocap",
            rebocap_running,
            has_driver("rebocap"),
            is_blocked("rebocap"),
        );

        let eye_face_source = if vrcft_running {
            PoseSourceObservation {
                source_id: "eye_face_osc",
                state: "active",
                detail_code: Some("process_running"),
            }
        } else if vrcft_installed {
            PoseSourceObservation {
                source_id: "eye_face_osc",
                state: "installed",
                detail_code: Some("app_installed"),
            }
        } else {
            PoseSourceObservation {
                source_id: "eye_face_osc",
                state: "not_detected",
                detail_code: None,
            }
        };

        let unblocked_fbt_driver_count = FBT_OPENVR_DRIVER_IDS
            .iter()
            .filter(|id| has_driver(id) && !is_blocked(id))
            .count();
        let fbt_driver_conflict_risk = if unblocked_fbt_driver_count >= 2 {
            "multiple_openvr_fbt_drivers"
        } else {
            "none"
        };

        PoseGraphSnapshotV01 {
            schema_version: POSE_GRAPH_SCHEMA,
            captured_at,
            openxr,
            steamvr_drivers,
            blocked_steamvr_drivers,
            fbt_driver_conflict_risk,
            last_headset_hint,
            pose_sources: vec![
                headset_source,
                pharos_source,
                slimevr_source,
                pico_source,
                lighthouse_source,
                space_calibrator_source,
                mocopi_source,
                haritorax_source,
                rebocap_source,
                eye_face_source,
            ],
            pharos_hub,
        }
    }
}

fn companion_driver_source(
    source_id: &'static str,
    is_running: bool,
    driver_registered: bool,
    driver_blocked: bool,
) -> PoseSourceObservation {
    if is_running {
        PoseSourceObservation {
            source_id,
            state: "active",
            detail_code: Some("process_running"),
        }
    } else if driver_blocked {
        PoseSourceObservation {
            source_id,
            state: "installed",
            detail_code: Some("blocked_by_safe_mode"),
        }
    } else if driver_registered {
        PoseSourceObservation {
            source_id,
            state: "ready",
            detail_code: Some("driver_registered"),
        }
    } else {
        PoseSourceObservation {
            source_id,
            state: "not_detected",
            detail_code: None,
        }
    }
}

fn disconnected_wearable_nodes() -> Vec<PharosWearableSlotObservation> {
    PHAROS_WEARABLE_SLOTS
        .into_iter()
        .map(|slot| PharosWearableSlotObservation {
            slot,
            state: "not_connected",
            tracking_mode: None,
            battery_percent: None,
            foot_heading_available: false,
        })
        .collect()
}

fn idle_pharos_hub(state: &'static str, telemetry_state: &'static str) -> PharosHubObservation {
    PharosHubObservation {
        state,
        telemetry_state,
        calibration_state: "unknown",
        spatial_anchor_ready: false,
        active_modes: Vec::new(),
        wearable_nodes: disconnected_wearable_nodes(),
        stereo_base_station_count: None,
        panoramic_anchor_count: None,
    }
}

fn inspect_pharos_hub(
    roots: &EnvironmentRoots,
    pharos_running: bool,
    driver_registered: bool,
    captured_at_rfc3339: &str,
) -> PharosHubObservation {
    let hub_state = if pharos_running {
        "running"
    } else if driver_registered {
        "driver_registered"
    } else {
        "not_detected"
    };

    let mut candidate_file = None;
    for candidate in &roots.pharos_status_candidates {
        let Ok(meta) = fs::symlink_metadata(candidate) else {
            continue;
        };
        if meta.file_type().is_symlink() || !meta.is_file() {
            continue;
        }
        candidate_file = Some((candidate, meta.len()));
        break;
    }

    let Some((status_path, file_len)) = candidate_file else {
        return idle_pharos_hub(hub_state, "missing");
    };
    if file_len == 0 || file_len > MAX_HUB_STATUS_BYTES {
        return idle_pharos_hub(hub_state, "malformed");
    }
    let Ok(raw_text) = fs::read_to_string(status_path) else {
        return idle_pharos_hub(hub_state, "malformed");
    };
    let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(&raw_text) else {
        return idle_pharos_hub(hub_state, "malformed");
    };

    let Some(updated_str) = doc.get("updatedAt").and_then(Value::as_str) else {
        return idle_pharos_hub(hub_state, "malformed");
    };
    let (Some(now_ms), Some(updated_ms)) = (
        rfc3339_offset_millis(captured_at_rfc3339),
        rfc3339_offset_millis(updated_str),
    ) else {
        return idle_pharos_hub(hub_state, "malformed");
    };
    let Some(age_ms) = now_ms.checked_sub(updated_ms) else {
        return idle_pharos_hub(hub_state, "malformed");
    };

    // Honesty gate: never report connected hardware unless Pharos is live right now
    // AND the telemetry heartbeat is within [-2s, +15s].
    if !pharos_running || !(TELEMETRY_MAX_FUTURE_SKEW_MS..=TELEMETRY_MAX_AGE_MS).contains(&age_ms) {
        return idle_pharos_hub(hub_state, "stale");
    }

    let mut active_modes = Vec::new();
    if let Some(modes) = doc.get("activeModes").and_then(Value::as_array) {
        for raw_mode in modes.iter().filter_map(Value::as_str) {
            let mapped = match raw_mode {
                "optical_stereo_2x2x2" => Some("optical_stereo_2x2x2"),
                "imu_fusion" => Some("imu_fusion"),
                "passive_retroreflective" => Some("passive_retroreflective"),
                "panoramic_spatial_anchor" => Some("panoramic_spatial_anchor"),
                _ => None,
            };
            if let Some(mode) = mapped {
                if !active_modes.contains(&mode) {
                    active_modes.push(mode);
                }
            }
        }
    }

    let calibration_state = match doc.get("calibrationState").and_then(Value::as_str) {
        Some("uncalibrated") => "uncalibrated",
        Some("calibrating") => "calibrating",
        Some("calibrated") => "calibrated",
        _ => "unknown",
    };

    let nodes_obj = doc.get("wearableNodes").and_then(Value::as_object);
    let wearable_nodes = PHAROS_WEARABLE_SLOTS
        .into_iter()
        .map(|slot| {
            let Some(node) = nodes_obj
                .and_then(|m| m.get(slot))
                .and_then(Value::as_object)
            else {
                return PharosWearableSlotObservation {
                    slot,
                    state: "not_connected",
                    tracking_mode: None,
                    battery_percent: None,
                    foot_heading_available: false,
                };
            };
            let connected = node.get("connected").and_then(Value::as_bool) == Some(true)
                || node.get("state").and_then(Value::as_str) == Some("connected");
            if !connected {
                return PharosWearableSlotObservation {
                    slot,
                    state: "not_connected",
                    tracking_mode: None,
                    battery_percent: None,
                    foot_heading_available: false,
                };
            }
            let tracking_mode = match node.get("trackingMode").and_then(Value::as_str) {
                Some("optical_imu") => Some("optical_imu"),
                Some("passive_retroreflective") => Some("passive_retroreflective"),
                Some("imu_only") => Some("imu_only"),
                _ => None,
            };
            let battery_percent = node
                .get("batteryPercent")
                .and_then(Value::as_u64)
                .filter(|v| *v <= 100)
                .map(|v| v as u8);
            let foot_heading_available = matches!(slot, "left_foot" | "right_foot")
                && node
                    .get("footHeadingAvailable")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            PharosWearableSlotObservation {
                slot,
                state: "connected",
                tracking_mode,
                battery_percent,
                foot_heading_available,
            }
        })
        .collect::<Vec<_>>();

    let stereo_base_station_count = doc
        .get("stereoBaseStationCount")
        .and_then(Value::as_u64)
        .filter(|v| *v <= 16);
    let panoramic_anchor_count = doc
        .get("panoramicAnchorCount")
        .and_then(Value::as_u64)
        .filter(|v| *v <= 8);

    let spatial_anchor_ready = doc
        .get("spatialAnchorReady")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || (calibration_state == "calibrated"
            && active_modes.contains(&"panoramic_spatial_anchor")
            && panoramic_anchor_count.is_some_and(|n| n >= 1));

    PharosHubObservation {
        state: "running",
        telemetry_state: "live",
        calibration_state,
        spatial_anchor_ready,
        active_modes,
        wearable_nodes,
        stereo_base_station_count,
        panoramic_anchor_count,
    }
}

/// Sanitizes a human-readable display label so filesystem paths (`\`, `/`, `:`) and control
/// characters never leak across the wire, bounded to `max_utf16` UTF-16 code units.
pub fn sanitize_pose_label(raw: &str, max_utf16: usize) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let looks_like_drive_path = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');
    if looks_like_drive_path || trimmed.starts_with("\\\\") || trimmed.starts_with('/') {
        return None;
    }

    let cleaned = trimmed
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '\\' | '/' | ':') {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>();

    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }

    let mut out = String::new();
    let mut utf16_used = 0usize;
    for ch in collapsed.chars() {
        let len = ch.len_utf16();
        if utf16_used + len > max_utf16 {
            break;
        }
        out.push(ch);
        utf16_used += len;
    }
    let out = out.trim().to_owned();
    (!out.is_empty()).then_some(out)
}

fn inspect_openxr(roots: &EnvironmentRoots) -> OpenXrRuntimeObservation {
    let active = roots
        .registry
        .get_string(
            RegistryHive::LocalMachine,
            OPENXR_REGISTRY_SUBKEY,
            "ActiveRuntime",
        )
        .or_else(|| {
            roots.registry.get_string(
                RegistryHive::CurrentUser,
                OPENXR_REGISTRY_SUBKEY,
                "ActiveRuntime",
            )
        });
    let Some(manifest_path) = active else {
        return OpenXrRuntimeObservation {
            presence: "missing",
            runtime_name: None,
        };
    };
    let manifest_path = manifest_path.trim().trim_matches('"');
    if manifest_path.is_empty() {
        return OpenXrRuntimeObservation {
            presence: "missing",
            runtime_name: None,
        };
    }
    let label = read_bounded_regular_utf8(Path::new(manifest_path), MAX_DRIVER_MANIFEST_BYTES)
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|val| {
            val.pointer("/runtime/name")
                .and_then(Value::as_str)
                .and_then(|s| sanitize_pose_label(s, 80))
        });
    match label {
        Some(name) => OpenXrRuntimeObservation {
            presence: "found",
            runtime_name: Some(name),
        },
        None => OpenXrRuntimeObservation {
            presence: "unreadable",
            runtime_name: None,
        },
    }
}

fn classify_driver_token(token_lower: &str) -> Option<&'static str> {
    let rules: [(&str, &[&str]); 9] = [
        (
            "space_calibrator",
            &["01spacecalibrator", "spacecalibrator", "space_calibrator"],
        ),
        ("slimevr", &["slimevr"]),
        ("alvr", &["alvr"]),
        (
            "virtual_desktop",
            &["virtualdesktop", "virtual desktop", "vdstreamer"],
        ),
        ("lighthouse", &["lighthouse"]),
        ("pharos", &["pharos"]),
        ("mocopi", &["mocopi"]),
        ("haritorax", &["haritorax", "haritora", "shiftall"]),
        ("rebocap", &["rebocap"]),
    ];
    for (driver_id, needles) in rules {
        if needles.iter().any(|needle| token_lower.contains(needle)) {
            return Some(driver_id);
        }
    }
    None
}

fn inspect_openvr_drivers(roots: &EnvironmentRoots) -> Vec<&'static str> {
    let mut raw_paths = Vec::new();
    for vrpath in &roots.openvrpaths {
        let Some(text) = read_bounded_regular_utf8(vrpath, MAX_OPENVR_JSON_BYTES) else {
            continue;
        };
        if let Ok(val) = serde_json::from_str::<Value>(&text) {
            if let Some(arr) = val.get("external_drivers").and_then(Value::as_array) {
                for item in arr.iter().filter_map(Value::as_str).take(64) {
                    raw_paths.push(item.to_owned());
                }
            }
        }
        break;
    }

    let mut discovered_tokens = Vec::new();
    for raw_dir in &raw_paths {
        discovered_tokens.push(raw_dir.to_ascii_lowercase());
        let manifest = Path::new(raw_dir).join("driver.vrdrivermanifest");
        if let Some(text) = read_bounded_regular_utf8(&manifest, MAX_DRIVER_MANIFEST_BYTES) {
            if let Ok(val) = serde_json::from_str::<Value>(&text) {
                if let Some(name) = val.get("name").and_then(Value::as_str) {
                    discovered_tokens.push(name.to_ascii_lowercase());
                }
            }
        }
    }

    let mut matched = Vec::new();
    for canonical in STEAMVR_POSE_DRIVER_IDS {
        if discovered_tokens
            .iter()
            .any(|tok| classify_driver_token(tok) == Some(canonical))
        {
            matched.push(canonical);
        }
    }
    matched
}

fn inspect_steamvr_settings(roots: &EnvironmentRoots) -> (Option<String>, Vec<&'static str>) {
    let mut hint: Option<String> = None;
    let mut blocked = Vec::new();

    for candidate in steamvr_settings_candidates(roots) {
        let Some(raw) = read_bounded_regular_utf8(&candidate, MAX_OPENVR_JSON_BYTES) else {
            continue;
        };
        let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };

        if hint.is_none() {
            let last_known = obj.get("LastKnown").and_then(Value::as_object).or_else(|| {
                obj.get("steamvr")
                    .and_then(Value::as_object)
                    .and_then(|s| s.get("LastKnown"))
                    .and_then(Value::as_object)
            });
            if let Some(last_known) = last_known {
                let mfg = last_known
                    .get("HMDManufacturer")
                    .and_then(Value::as_str)
                    .and_then(|s| sanitize_pose_label(s, 40));
                let model = last_known
                    .get("HMDModel")
                    .and_then(Value::as_str)
                    .and_then(|s| sanitize_pose_label(s, 60));
                hint = match (mfg, model) {
                    (Some(m), Some(md)) => {
                        if md.to_ascii_lowercase().starts_with(&m.to_ascii_lowercase()) {
                            Some(md)
                        } else {
                            sanitize_pose_label(&format!("{m} {md}"), 80)
                        }
                    }
                    (None, Some(md)) => Some(md),
                    (Some(m), None) => Some(m),
                    (None, None) => None,
                };
            }
        }

        for (key, val) in &obj {
            let Some(driver_suffix) = key.strip_prefix("driver_") else {
                continue;
            };
            let is_blocked = val
                .as_object()
                .and_then(|d| d.get("blocked_by_safe_mode"))
                .and_then(Value::as_bool)
                == Some(true);
            if !is_blocked {
                continue;
            }
            if let Some(canonical) = classify_driver_token(&driver_suffix.to_ascii_lowercase()) {
                if !blocked.contains(&canonical) {
                    blocked.push(canonical);
                }
            }
        }
        break;
    }

    let ordered_blocked = STEAMVR_POSE_DRIVER_IDS
        .into_iter()
        .filter(|id| blocked.contains(id))
        .collect();
    (hint, ordered_blocked)
}

fn steamvr_settings_candidates(roots: &EnvironmentRoots) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for vrpath in &roots.openvrpaths {
        if let Some(raw) = read_bounded_regular_utf8(vrpath, MAX_OPENVR_JSON_BYTES) {
            if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(&raw) {
                if let Some(configs) = obj.get("config").and_then(Value::as_array) {
                    for entry in configs.iter().filter_map(Value::as_str) {
                        let candidate = PathBuf::from(entry).join("steamvr.vrsettings");
                        if !out.contains(&candidate) {
                            out.push(candidate);
                        }
                    }
                }
            }
        }
    }
    for (hive, subkey, value_name) in [
        (
            RegistryHive::CurrentUser,
            "Software\\Valve\\Steam",
            "SteamPath",
        ),
        (
            RegistryHive::LocalMachine,
            STEAM_REGISTRY_SUBKEY,
            "InstallPath",
        ),
        (
            RegistryHive::LocalMachine,
            "SOFTWARE\\Valve\\Steam",
            "InstallPath",
        ),
    ] {
        if let Some(steam_dir) = roots.registry.get_string(hive, subkey, value_name) {
            let trimmed = steam_dir.trim();
            if !trimmed.is_empty() {
                let candidate = PathBuf::from(trimmed)
                    .join("config")
                    .join("steamvr.vrsettings");
                if !out.contains(&candidate) {
                    out.push(candidate);
                }
            }
        }
    }
    for steam_root in &roots.steam_install_candidates {
        let candidate = steam_root.join("config").join("steamvr.vrsettings");
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    }
    out
}

fn effective_steam_common_roots(roots: &EnvironmentRoots) -> Vec<PathBuf> {
    let mut install = None;
    for (hive, subkey, value_name) in [
        (
            RegistryHive::LocalMachine,
            STEAM_REGISTRY_SUBKEY,
            "InstallPath",
        ),
        (
            RegistryHive::LocalMachine,
            "SOFTWARE\\Valve\\Steam",
            "InstallPath",
        ),
        (
            RegistryHive::CurrentUser,
            "Software\\Valve\\Steam",
            "SteamPath",
        ),
    ] {
        if let Some(found) = roots
            .registry
            .get_string(hive, subkey, value_name)
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
        {
            install = Some(found);
            break;
        }
    }
    if install.is_none() {
        install = roots
            .steam_install_candidates
            .iter()
            .find(|c| c.is_dir())
            .cloned();
    }
    let Some(install) = install else {
        return roots.steam_common.clone();
    };
    let mut out = vec![install.join("steamapps").join("common")];
    let vdf = install.join("steamapps").join("libraryfolders.vdf");
    if let Some(text) = read_bounded_regular_utf8(&vdf, MAX_OPENVR_JSON_BYTES) {
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("\"path\"") {
                let quoted = rest.trim().trim_matches('"').replace("\\\\", "\\");
                if !quoted.is_empty() {
                    let common = PathBuf::from(quoted).join("steamapps").join("common");
                    if !out.contains(&common) {
                        out.push(common);
                    }
                }
            }
        }
    }
    out
}

fn is_slimevr_installed(roots: &EnvironmentRoots) -> bool {
    for (hive, subkey) in [
        (
            RegistryHive::LocalMachine,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SlimeVR",
        ),
        (
            RegistryHive::LocalMachine,
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SlimeVR",
        ),
    ] {
        for value_name in ["InstallLocation", "DisplayIcon"] {
            if let Some(raw) = roots.registry.get_string(hive, subkey, value_name) {
                let cleaned = raw.split(',').next().unwrap_or("").trim().trim_matches('"');
                if !cleaned.is_empty() && Path::new(cleaned).exists() {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FakeRegistrySource, FixedClock, VrRuntimeRoots};
    use serde_json::json;
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let n = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "vua-pose-graph-test-{}-{}",
                std::process::id(),
                n
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct StubPoseProbe {
        running: HashSet<String>,
    }
    impl PoseProcessProbe for StubPoseProbe {
        fn is_process_running(&self, exe_name: &str) -> Result<bool, PoseProbeUnavailable> {
            Ok(self.running.contains(&exe_name.to_ascii_lowercase()))
        }
    }

    fn empty_roots(base: &Path, registry: FakeRegistrySource) -> EnvironmentRoots {
        EnvironmentRoots {
            steam_common: vec![base.join("steam_common")],
            local_low: base.join("LocalLow"),
            unity_hub_exe_candidates: Vec::new(),
            unity_hub_registry_display_icon_keys: Vec::new(),
            unity_editors_root: base.join("Editors"),
            vrc_get_executable: "vrc-get".to_owned(),
            disk_target: base.to_path_buf(),
            network_probes: Vec::new(),
            registry: Arc::new(registry),
            steam_install_candidates: Vec::new(),
            vr_runtime_roots: VrRuntimeRoots {
                oculus: Vec::new(),
                pico: Vec::new(),
                vive: Vec::new(),
                virtual_desktop: Vec::new(),
                alvr: Vec::new(),
                pimax: Vec::new(),
                varjo: Vec::new(),
                hp_omnicept: Vec::new(),
            },
            openvrpaths: Vec::new(),
            vcc_settings_candidates: Vec::new(),
            pico_connect_settings_candidates: Vec::new(),
            pharos_status_candidates: Vec::new(),
            vcc_exe_candidates: Vec::new(),
            alcom_exe_candidates: Vec::new(),
        }
    }

    #[test]
    fn empty_environment_reports_honest_not_detected_and_zero_fabricated_hardware() {
        let dir = TestDir::new();
        let roots = empty_roots(dir.path(), FakeRegistrySource::new());
        let svc = PoseGraphService::new(
            roots,
            Arc::new(StubPoseProbe {
                running: HashSet::new(),
            }),
            Arc::new(FixedClock::new(&["2026-11-14T00:00:00Z"])),
        );

        let snap = svc.observe();
        assert_eq!(snap.schema_version, POSE_GRAPH_SCHEMA);
        assert_eq!(snap.openxr.presence, "missing");
        assert_eq!(snap.openxr.runtime_name, None);
        assert!(snap.steamvr_drivers.is_empty());
        assert!(snap.blocked_steamvr_drivers.is_empty());
        assert_eq!(snap.fbt_driver_conflict_risk, "none");
        assert_eq!(snap.last_headset_hint, None);
        assert_eq!(snap.pose_sources.len(), 10);
        for (idx, id) in POSE_SOURCE_IDS.iter().enumerate() {
            assert_eq!(snap.pose_sources[idx].source_id, *id);
            assert_eq!(snap.pose_sources[idx].state, "not_detected");
            assert_eq!(snap.pose_sources[idx].detail_code, None);
        }
        assert_eq!(snap.pharos_hub.state, "not_detected");
        assert_eq!(snap.pharos_hub.telemetry_state, "missing");
        assert_eq!(snap.pharos_hub.calibration_state, "unknown");
        assert!(!snap.pharos_hub.spatial_anchor_ready);
        assert!(snap.pharos_hub.active_modes.is_empty());
        assert_eq!(snap.pharos_hub.wearable_nodes.len(), 4);
        for (idx, slot) in PHAROS_WEARABLE_SLOTS.iter().enumerate() {
            assert_eq!(snap.pharos_hub.wearable_nodes[idx].slot, *slot);
            assert_eq!(snap.pharos_hub.wearable_nodes[idx].state, "not_connected");
            assert_eq!(snap.pharos_hub.wearable_nodes[idx].tracking_mode, None);
            assert_eq!(snap.pharos_hub.wearable_nodes[idx].battery_percent, None);
            assert!(!snap.pharos_hub.wearable_nodes[idx].foot_heading_available);
        }
        assert_eq!(snap.pharos_hub.stereo_base_station_count, None);
        assert_eq!(snap.pharos_hub.panoramic_anchor_count, None);
    }

    #[test]
    fn aggregates_openxr_manifest_drivers_safe_mode_blocks_and_redacts_paths() {
        let dir = TestDir::new();
        let openxr_json = dir.path().join("steamxr_win64.json");
        fs::write(&openxr_json, r#"{"runtime":{"name":"SteamVR"}}"#).unwrap();

        let custom_mocopi_dir = dir.path().join("CustomVendorDir");
        fs::create_dir_all(&custom_mocopi_dir).unwrap();
        fs::write(
            custom_mocopi_dir.join("driver.vrdrivermanifest"),
            json!({ "name": "mocopi" }).to_string(),
        )
        .unwrap();

        let steam_cfg = dir.path().join("steam_config");
        fs::create_dir_all(&steam_cfg).unwrap();
        fs::write(
            steam_cfg.join("steamvr.vrsettings"),
            json!({
                "LastKnown": {
                    "HMDManufacturer": "PICO",
                    "HMDModel": "PICO 4 Ultra"
                },
                "driver_01spacecalibrator": {
                    "blocked_by_safe_mode": true
                }
            })
            .to_string(),
        )
        .unwrap();

        let vrpath = dir.path().join("openvrpaths.vrpath");
        fs::write(
            &vrpath,
            json!({
                "config": [steam_cfg.to_string_lossy()],
                "external_drivers": [
                    "C:\\Users\\SecretUser\\AppData\\Local\\01spacecalibrator",
                    "C:\\Program Files\\SlimeVR Server\\steamvr",
                    "D:\\Pharos\\driver_pharos",
                    custom_mocopi_dir.to_string_lossy(),
                    "C:\\Users\\SecretUser\\CustomPrivateDriver"
                ]
            })
            .to_string(),
        )
        .unwrap();

        let pico_dir = dir.path().join("PICO Connect");
        fs::create_dir_all(&pico_dir).unwrap();

        let registry = FakeRegistrySource::new().with(
            RegistryHive::LocalMachine,
            OPENXR_REGISTRY_SUBKEY,
            "ActiveRuntime",
            &openxr_json.to_string_lossy(),
        );
        let mut roots = empty_roots(dir.path(), registry);
        roots.openvrpaths = vec![vrpath];
        roots.vr_runtime_roots.pico = vec![pico_dir];

        let mut running = HashSet::new();
        running.insert("slimevr.exe".to_owned());
        let svc = PoseGraphService::new(
            roots,
            Arc::new(StubPoseProbe { running }),
            Arc::new(FixedClock::new(&["2026-11-14T00:00:00Z"])),
        );

        let snap = svc.observe();
        assert_eq!(snap.openxr.presence, "found");
        assert_eq!(snap.openxr.runtime_name.as_deref(), Some("SteamVR"));
        assert_eq!(
            snap.steamvr_drivers,
            vec!["space_calibrator", "slimevr", "pharos", "mocopi"]
        );
        assert_eq!(snap.fbt_driver_conflict_risk, "multiple_openvr_fbt_drivers");
        assert_eq!(snap.blocked_steamvr_drivers, vec!["space_calibrator"]);
        assert_eq!(snap.last_headset_hint.as_deref(), Some("PICO 4 Ultra"));
        assert_eq!(snap.pharos_hub.state, "driver_registered");
        assert_eq!(snap.pharos_hub.telemetry_state, "missing");

        let by_id = |id: &str| {
            snap.pose_sources
                .iter()
                .find(|s| s.source_id == id)
                .unwrap()
        };
        assert_eq!(by_id("headset_6dof").state, "ready");
        assert_eq!(by_id("pharos").state, "ready");
        assert_eq!(by_id("slimevr").state, "active");
        assert_eq!(by_id("pico_motion").state, "installed");
        assert_eq!(by_id("lighthouse").state, "ready");
        assert_eq!(by_id("space_calibrator").state, "installed");
        assert_eq!(
            by_id("space_calibrator").detail_code,
            Some("blocked_by_safe_mode")
        );
        assert_eq!(by_id("mocopi").state, "ready");
        assert_eq!(by_id("haritorax").state, "not_detected");
        assert_eq!(by_id("rebocap").state, "not_detected");
        assert_eq!(by_id("eye_face_osc").state, "not_detected");

        let serialized = serde_json::to_string(&snap).unwrap();
        assert!(!serialized.contains("SecretUser"));
        assert!(!serialized.contains("CustomPrivateDriver"));
        assert!(!serialized.contains("CustomVendorDir"));
    }

    #[test]
    fn reads_live_pharos_dual_mode_telemetry_and_rejects_stale_or_stopped_status_files() {
        let dir = TestDir::new();
        let status_file = dir.path().join("hub-status.json");
        fs::write(
            &status_file,
            json!({
                "updatedAt": "2026-11-14T10:00:05.000Z",
                "calibrationState": "calibrated",
                "activeModes": [
                    "optical_stereo_2x2x2",
                    "imu_fusion",
                    "passive_retroreflective",
                    "panoramic_spatial_anchor"
                ],
                "stereoBaseStationCount": 2,
                "panoramicAnchorCount": 1,
                "wearableNodes": {
                    "left_foot": {
                        "connected": true,
                        "trackingMode": "optical_imu",
                        "batteryPercent": 82,
                        "footHeadingAvailable": true
                    },
                    "right_foot": {
                        "connected": true,
                        "trackingMode": "passive_retroreflective",
                        "batteryPercent": 0,
                        "footHeadingAvailable": true
                    },
                    "waist": {
                        "connected": true,
                        "trackingMode": "optical_imu",
                        "batteryPercent": 64
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

        let mut roots = empty_roots(dir.path(), FakeRegistrySource::new());
        roots.pharos_status_candidates = vec![status_file];

        // 1. Live running Pharos within 15s -> full dual-mode telemetry + passive fallback on depleted right_foot.
        let mut running = HashSet::new();
        running.insert("pharos-hub.exe".to_owned());
        let live_svc = PoseGraphService::new(
            roots,
            Arc::new(StubPoseProbe {
                running: running.clone(),
            }),
            Arc::new(FixedClock::new(&["2026-11-14T10:00:12.000Z"])),
        );
        let live = live_svc.observe();
        assert_eq!(live.pharos_hub.state, "running");
        assert_eq!(live.pharos_hub.telemetry_state, "live");
        assert_eq!(live.pharos_hub.calibration_state, "calibrated");
        assert!(live.pharos_hub.spatial_anchor_ready);
        assert_eq!(live.pharos_hub.stereo_base_station_count, Some(2));
        assert_eq!(live.pharos_hub.panoramic_anchor_count, Some(1));
        assert_eq!(
            live.pharos_hub.wearable_nodes[0],
            PharosWearableSlotObservation {
                slot: "left_foot",
                state: "connected",
                tracking_mode: Some("optical_imu"),
                battery_percent: Some(82),
                foot_heading_available: true,
            }
        );
        assert_eq!(
            live.pharos_hub.wearable_nodes[1],
            PharosWearableSlotObservation {
                slot: "right_foot",
                state: "connected",
                tracking_mode: Some("passive_retroreflective"),
                battery_percent: Some(0),
                foot_heading_available: true,
            }
        );
        assert_eq!(live.pharos_hub.wearable_nodes[3].state, "not_connected");

        // 2. Stale (> 15s old) -> refuses to claim connected hardware.
        let mut roots_stale = empty_roots(dir.path(), FakeRegistrySource::new());
        roots_stale.pharos_status_candidates = vec![dir.path().join("hub-status.json")];
        let stale_svc = PoseGraphService::new(
            roots_stale,
            Arc::new(StubPoseProbe { running }),
            Arc::new(FixedClock::new(&["2026-11-14T10:00:30.000Z"])),
        );
        let stale = stale_svc.observe();
        assert_eq!(stale.pharos_hub.telemetry_state, "stale");
        assert!(!stale.pharos_hub.spatial_anchor_ready);
        assert!(stale.pharos_hub.active_modes.is_empty());
        assert_eq!(stale.pharos_hub.stereo_base_station_count, None);
        assert!(stale
            .pharos_hub
            .wearable_nodes
            .iter()
            .all(|n| n.state == "not_connected" && n.battery_percent.is_none()));
    }

    #[test]
    fn sanitize_pose_label_rejects_paths_and_strips_separators() {
        assert_eq!(
            sanitize_pose_label("C:\\Program Files\\Steam\\steamxr_win64.json", 80),
            None
        );
        assert_eq!(sanitize_pose_label("\\\\server\\share\\runtime", 80), None);
        assert_eq!(
            sanitize_pose_label("Meta/Quest: Link  Runtime", 80).as_deref(),
            Some("Meta Quest Link Runtime")
        );
    }
}
