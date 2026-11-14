//! Candidate `pose-graph v0.1`: Host-owned read-only observation of local OpenXR runtime,
//! SteamVR external pose drivers, companion pose applications, and the Pharos optical+IMU
//! dual-mode tracking hub contract.
//!
//! Safety & honesty invariants:
//! - Read-only local observation; never opens foreign process memory or binds network sockets.
//! - OpenXR runtime names and headset hints are sanitized display labels (`<= 80` UTF-16 code
//!   units) stripped of filesystem path separators (`\`, `/`, `:`).
//! - `steamvrDrivers` emits only closed vocabulary identifiers.
//! - Pharos wearable slots (`left_foot`, `right_foot`, `waist`, `chest`) and station counts
//!   never fabricate connected hardware or synthetic counts when live telemetry is absent.

use crate::environment::EnvironmentRoots;
use crate::time::Clock;
use crate::win_registry::RegistryHive;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const POSE_GRAPH_SCHEMA: &str = "vua.pose-graph/v0.1";

const OPENXR_REGISTRY_SUBKEY: &str = "SOFTWARE\\Khronos\\OpenXR\\1";
const STEAM_REGISTRY_SUBKEY: &str = "SOFTWARE\\WOW6432Node\\Valve\\Steam";

pub const POSE_SOURCE_IDS: [&str; 6] = [
    "headset_6dof",
    "pharos",
    "slimevr",
    "pico_motion",
    "lighthouse",
    "eye_face_osc",
];

pub const PHAROS_WEARABLE_SLOTS: [&str; 4] = ["left_foot", "right_foot", "waist", "chest"];

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
    pub foot_heading_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PharosHubObservation {
    pub state: &'static str,
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
        let openxr = inspect_openxr(&self.roots);
        let steamvr_drivers = inspect_openvr_drivers(&self.roots);
        let last_headset_hint = inspect_last_headset_hint(&self.roots);

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

        let running = |exe: &str| matches!(self.probe.is_process_running(exe), Ok(true));
        let vrserver_running = running("vrserver.exe") || running("vrcompositor.exe");
        let pharos_running = running("pharos.exe") || running("pharos-hub.exe");
        let slimevr_running = running("slimevr.exe") || running("slimevr-server.exe");
        let pico_running = running("picoconnect.exe") || running("pico connect.exe");
        let calibrator_running = running("spacecalibrator.exe");
        let vrcft_running = running("vrcfacetracking.exe");

        let has_driver = |id: &str| steamvr_drivers.contains(&id);

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
                detail_code: Some("hub_running"),
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

        let pharos_state = if pharos_running {
            "running"
        } else if has_driver("pharos") {
            "driver_registered"
        } else {
            "not_detected"
        };

        let pharos_hub = PharosHubObservation {
            state: pharos_state,
            active_modes: Vec::new(),
            wearable_nodes: PHAROS_WEARABLE_SLOTS
                .into_iter()
                .map(|slot| PharosWearableSlotObservation {
                    slot,
                    state: "not_connected",
                    foot_heading_available: false,
                })
                .collect(),
            stereo_base_station_count: None,
            panoramic_anchor_count: None,
        };

        PoseGraphSnapshotV01 {
            schema_version: POSE_GRAPH_SCHEMA,
            captured_at: self.clock.now_rfc3339(),
            openxr,
            steamvr_drivers,
            last_headset_hint,
            pose_sources: vec![
                headset_source,
                pharos_source,
                slimevr_source,
                pico_source,
                lighthouse_source,
                eye_face_source,
            ],
            pharos_hub,
        }
    }
}

/// Sanitizes a human-readable display label so filesystem paths (`\`, `/`, `:`) and control
/// characters never leak across the wire, bounded to `max_utf16` UTF-16 code units.
pub fn sanitize_pose_label(raw: &str, max_utf16: usize) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Reject obvious filesystem paths outright rather than leaking path segments.
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
    let label = fs::read_to_string(Path::new(manifest_path))
        .ok()
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

fn inspect_openvr_drivers(roots: &EnvironmentRoots) -> Vec<&'static str> {
    let mut entries_lower = Vec::new();
    for vrpath in &roots.openvrpaths {
        if !vrpath.is_file() {
            continue;
        }
        if let Ok(text) = fs::read_to_string(vrpath) {
            if let Ok(val) = serde_json::from_str::<Value>(&text) {
                if let Some(arr) = val.get("external_drivers").and_then(Value::as_array) {
                    for item in arr.iter().filter_map(Value::as_str) {
                        entries_lower.push(item.to_ascii_lowercase());
                    }
                }
            }
        }
        break;
    }

    let mut matched = Vec::new();
    let rules: [(&str, &[&str]); 6] = [
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
    ];
    for (driver_id, needles) in rules {
        if entries_lower
            .iter()
            .any(|entry| needles.iter().any(|needle| entry.contains(needle)))
        {
            matched.push(driver_id);
        }
    }
    matched
}

fn inspect_last_headset_hint(roots: &EnvironmentRoots) -> Option<String> {
    for candidate in steamvr_settings_candidates(roots) {
        let Ok(raw) = fs::read_to_string(&candidate) else {
            continue;
        };
        let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let last_known = obj.get("LastKnown").and_then(Value::as_object).or_else(|| {
            obj.get("steamvr")
                .and_then(Value::as_object)
                .and_then(|s| s.get("LastKnown"))
                .and_then(Value::as_object)
        });
        let Some(last_known) = last_known else {
            continue;
        };
        let mfg = last_known
            .get("HMDManufacturer")
            .and_then(Value::as_str)
            .and_then(|s| sanitize_pose_label(s, 40));
        let model = last_known
            .get("HMDModel")
            .and_then(Value::as_str)
            .and_then(|s| sanitize_pose_label(s, 60));
        let combined = match (mfg, model) {
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
        if combined.is_some() {
            return combined;
        }
    }
    None
}

fn steamvr_settings_candidates(roots: &EnvironmentRoots) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for vrpath in &roots.openvrpaths {
        if let Ok(raw) = fs::read_to_string(vrpath) {
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
    let mut install = roots
        .registry
        .get_string(
            RegistryHive::LocalMachine,
            STEAM_REGISTRY_SUBKEY,
            "InstallPath",
        )
        .map(PathBuf::from)
        .filter(|p| p.is_dir());
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
    if let Ok(text) = fs::read_to_string(vdf) {
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
        assert_eq!(snap.last_headset_hint, None);
        assert_eq!(snap.pose_sources.len(), 6);
        for (idx, id) in POSE_SOURCE_IDS.iter().enumerate() {
            assert_eq!(snap.pose_sources[idx].source_id, *id);
            assert_eq!(snap.pose_sources[idx].state, "not_detected");
            assert_eq!(snap.pose_sources[idx].detail_code, None);
        }
        assert_eq!(snap.pharos_hub.state, "not_detected");
        assert!(snap.pharos_hub.active_modes.is_empty());
        assert_eq!(snap.pharos_hub.wearable_nodes.len(), 4);
        for (idx, slot) in PHAROS_WEARABLE_SLOTS.iter().enumerate() {
            assert_eq!(snap.pharos_hub.wearable_nodes[idx].slot, *slot);
            assert_eq!(snap.pharos_hub.wearable_nodes[idx].state, "not_connected");
            assert!(!snap.pharos_hub.wearable_nodes[idx].foot_heading_available);
        }
        assert_eq!(snap.pharos_hub.stereo_base_station_count, None);
        assert_eq!(snap.pharos_hub.panoramic_anchor_count, None);
    }

    #[test]
    fn aggregates_openxr_openvr_drivers_headset_hint_and_pharos_without_path_leaks() {
        let dir = TestDir::new();
        let openxr_json = dir.path().join("steamxr_win64.json");
        fs::write(&openxr_json, r#"{"runtime":{"name":"SteamVR"}}"#).unwrap();

        let steam_cfg = dir.path().join("steam_config");
        fs::create_dir_all(&steam_cfg).unwrap();
        fs::write(
            steam_cfg.join("steamvr.vrsettings"),
            json!({
                "LastKnown": {
                    "HMDManufacturer": "PICO",
                    "HMDModel": "PICO 4 Ultra"
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
            vec!["space_calibrator", "slimevr", "pharos"]
        );
        assert_eq!(snap.last_headset_hint.as_deref(), Some("PICO 4 Ultra"));
        assert_eq!(snap.pharos_hub.state, "driver_registered");
        // Even with driver registered, hardware nodes stay honestly not_connected without live telemetry.
        assert!(snap
            .pharos_hub
            .wearable_nodes
            .iter()
            .all(|n| n.state == "not_connected" && !n.foot_heading_available));

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
        assert_eq!(by_id("eye_face_osc").state, "not_detected");

        let serialized = serde_json::to_string(&snap).unwrap();
        assert!(!serialized.contains("SecretUser"));
        assert!(!serialized.contains("CustomPrivateDriver"));
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
