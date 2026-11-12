//! M6 T-A/T-B: the read-only project-inspection aggregate behind the
//! general vrc-get project path and the ALCOM/VCC compatibility matrix.
//!
//! One pass over exactly the paths the managers registered — never a scan,
//! never a write (product-boundary 1.2.0 allow-list: discovery/identification,
//! version/package/SDK/compatibility/environment-state reads; every write is
//! forbidden). For each registered path the inspection reports:
//!
//! - manager associations (VCC and/or ALCOM registration, unioned);
//! - Unity version + classification (same policy as the editor matrix);
//! - the VPM manifest's declared `dependencies` and `locked` maps, read as
//!   documents: a parse failure is an honest warning finding, never an
//!   invented package list;
//! - VRChat SDK packages spotted among those maps (`com.vrchat.*` prefix —
//!   reported as seen, without inventing product semantics);
//! - the pending-mutation marker state (read-only observation of the
//!   `project_lock` artifacts; acquiring the lock would be a write).
//!
//! The result is the versioned [`ProjectInspectionSnapshotV01`] payload,
//! whose shape is pinned by
//! `schemas/project-inspection/v0.1/snapshot.schema.json`.
//!
//! Deliberate scope boundary: this is the *inspection* face (documents the
//! declared on-disk state). The *operation* face — full package resolution,
//! previews, and applies — remains the [`crate::vpm_backend`] `VpmBackend`
//! port; the wire vocabulary that will expose either face is proposed in
//! `collab/proposals/013` and must not be guessed here.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use vua_orchestrator::{
    classify_version_string, env_managers_codes, Clock, EditorClass, FindingSeverity,
    ManagerDiagnostic, PRODUCTION_TARGET,
};

use crate::environment_managers::{read_alcom_settings, read_vcc_settings, ManagerRoots};

pub const PROJECT_INSPECTION_SCHEMA_VERSION: &str = "vua.project-inspection/v0.2";

/// Inspection-specific diagnostics. Shared path-level findings reuse the
/// core `env_managers_codes`; these codes are owned by this crate because
/// they describe the manifest/lock reading this module performs.
pub mod codes {
    pub const MANIFEST_UNREADABLE: &str = "vua.project_inspection.manifest_unreadable";
    pub const MANIFEST_SCHEMA_UNEXPECTED: &str =
        "vua.project_inspection.manifest_schema_unexpected";
}

/// Read-only observation of the pending-mutation marker for one project
/// (see [`crate::project_lock`]): `none` is clean, `leftover` is a parseable
/// unfinished-mutation marker, `unreadable` is a marker that exists but
/// cannot be parsed — itself evidence of a half-written mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    None,
    Leftover,
    Unreadable,
}

/// Read-only projection of the VUA-native identity finding for one
/// project ([`crate::vua_identity`]): `absent` = no identity file (not
/// VUA-native), `present` carries the marking time and the user note
/// (ruling 12: the note is shown only in the project list), `unreadable`
/// is an identity file that exists but cannot be parsed — evidence, never
/// silently reported as absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum VuaIdentityFinding {
    Absent,
    Present {
        /// RFC 3339 at the moment the project was marked VUA-native.
        marked_at: String,
        /// User note; `None` = no note set.
        note: Option<String>,
    },
    Unreadable,
}

impl From<crate::vua_identity::VuaIdentity> for VuaIdentityFinding {
    fn from(identity: crate::vua_identity::VuaIdentity) -> Self {
        match identity {
            crate::vua_identity::VuaIdentity::Absent => VuaIdentityFinding::Absent,
            crate::vua_identity::VuaIdentity::Present(found) => VuaIdentityFinding::Present {
                marked_at: found.marked_at,
                note: found.note,
            },
            crate::vua_identity::VuaIdentity::Unreadable => VuaIdentityFinding::Unreadable,
        }
    }
}

/// One entry of the manifest's `dependencies` or `locked` map: the package
/// id and the version string exactly as declared in the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestPackage {
    pub package_id: String,
    pub version: String,
}

/// The VRChat SDK packages spotted among the manifest maps, reported as
/// seen (`com.vrchat.*` prefix) without inventing product semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VrchatSdkFinding {
    pub package_id: String,
    /// Where the version string came from: `dependencies` (the requirement)
    /// or `locked` (the pinned resolution). When both maps carry the
    /// package, `locked` wins — it is the actually-installed pin.
    pub source: &'static str,
    pub version: String,
}

/// The deep read-only inspection of one registered project path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInspectionV01 {
    pub path: String,
    /// Directory name as registered — the closest thing to a display name
    /// the filesystem honestly offers.
    pub name: String,
    /// True when the registered path exists and is a directory; false
    /// keeps the entry visible as a stale registration (with a warning
    /// diagnostic) instead of silently dropping it.
    pub path_present: bool,
    pub associations: Vec<crate::environment_managers::ProjectAssociation>,
    pub unity_version: Option<String>,
    pub unity_classification: Option<EditorClass>,
    /// `Packages/vpm-manifest.json` exists.
    pub manifest_present: bool,
    /// The manifest parsed as a JSON object. A manifest that exists but
    /// does not parse is an honest warning, never an invented package list.
    pub manifest_schema_ok: bool,
    pub dependencies: Vec<ManifestPackage>,
    pub locked: Vec<ManifestPackage>,
    pub vrchat_sdks: Vec<VrchatSdkFinding>,
    pub mutation_status: MutationStatus,
    /// VUA-native identity finding (`.vua/project.json`): absent /
    /// present(markedAt, note) / unreadable.
    pub vua_identity: VuaIdentityFinding,
    pub diagnostics: Vec<ManagerDiagnostic>,
}

/// Versioned aggregate pinned by
/// `schemas/project-inspection/v0.1/snapshot.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInspectionSnapshotV01 {
    pub schema_version: &'static str,
    pub captured_at: String,
    pub production_target: &'static str,
    pub migration_sources: [&'static str; 2],
    pub projects: Vec<ProjectInspectionV01>,
    pub diagnostics: Vec<ManagerDiagnostic>,
}

/// Assembles the read-only inspection aggregate over exactly the paths the
/// managers registered (VCC `userProjects`/`localProjectFolders`, ALCOM
/// `userProjects`). Deterministic for a given tree: every map is sorted,
/// projects are sorted by path, and only a failed *observation* produces a
/// diagnostic.
pub fn collect_project_inspections(
    vcc_settings_candidates: &[std::path::PathBuf],
    roots: &ManagerRoots,
    clock: &dyn Clock,
) -> ProjectInspectionSnapshotV01 {
    let mut diagnostics = Vec::new();
    let vcc = read_vcc_settings(vcc_settings_candidates, &mut diagnostics);
    let alcom = read_alcom_settings(&roots.alcom_settings_candidates, &mut diagnostics);

    // Same union discipline as `collect_projects`: one entry per path,
    // associations from every manager that registered it, sorted.
    let mut by_path: BTreeMap<String, Vec<crate::environment_managers::ProjectAssociation>> =
        BTreeMap::new();
    let register = |path: &str,
                    association: crate::environment_managers::ProjectAssociation,
                    by_path: &mut BTreeMap<
        String,
        Vec<crate::environment_managers::ProjectAssociation>,
    >| {
        let entry = by_path.entry(path.to_owned()).or_default();
        if !entry.contains(&association) {
            entry.push(association);
            entry.sort();
        }
    };
    match vcc.projects_source {
        Some("userProjects") => {
            for path in &vcc.user_projects {
                register(
                    path,
                    crate::environment_managers::ProjectAssociation::VccRegistered,
                    &mut by_path,
                );
            }
        }
        Some("localProjectFolders") => {
            for folder in &vcc.local_project_folders {
                if let Ok(read_dir) = std::fs::read_dir(folder) {
                    for entry in read_dir.flatten() {
                        if entry.path().is_dir() {
                            register(
                                &entry.path().to_string_lossy(),
                                crate::environment_managers::ProjectAssociation::VccRegistered,
                                &mut by_path,
                            );
                        }
                    }
                } else {
                    diagnostics.push(ManagerDiagnostic {
                        code: env_managers_codes::PROJECT_PATH_MISSING,
                        severity: FindingSeverity::Warning,
                        detail: format!("{folder}: registered project folder cannot be read"),
                    });
                }
            }
        }
        _ => {}
    }
    for path in &alcom.user_projects {
        register(
            path,
            crate::environment_managers::ProjectAssociation::AlcomRegistered,
            &mut by_path,
        );
    }

    let projects = by_path
        .into_iter()
        .map(|(path, associations)| inspect_one(&path, associations))
        .collect();

    ProjectInspectionSnapshotV01 {
        schema_version: PROJECT_INSPECTION_SCHEMA_VERSION,
        captured_at: clock.now_rfc3339(),
        production_target: PRODUCTION_TARGET,
        migration_sources: vua_orchestrator::MIGRATION_SOURCES,
        projects,
        diagnostics,
    }
}

/// Deep read-only inspection of one path with explicit associations — the
/// single-project face of [`collect_project_inspections`], used by the
/// import-copy re-inspection (proposal 014) so the receipt's reInspection
/// block and the read face can never drift apart.
pub fn inspect_project_deep(
    path: &str,
    associations: Vec<crate::environment_managers::ProjectAssociation>,
) -> ProjectInspectionV01 {
    inspect_one(path, associations)
}

/// Deep read-only inspection of one registered path. Project-level
/// findings ride on the per-project `diagnostics`; the aggregate-level
/// `diagnostics` only carries discovery findings.
fn inspect_one(
    path: &str,
    associations: Vec<crate::environment_managers::ProjectAssociation>,
) -> ProjectInspectionV01 {
    let dir = Path::new(path);
    let name = dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned());

    let mut inspection = ProjectInspectionV01 {
        path: path.to_owned(),
        name,
        path_present: false,
        associations,
        unity_version: None,
        unity_classification: None,
        manifest_present: false,
        manifest_schema_ok: false,
        dependencies: Vec::new(),
        locked: Vec::new(),
        vrchat_sdks: Vec::new(),
        mutation_status: MutationStatus::None,
        vua_identity: VuaIdentityFinding::Absent,
        diagnostics: Vec::new(),
    };

    let metadata = match std::fs::metadata(dir) {
        Ok(metadata) => metadata,
        Err(_) => {
            // Stale registration: keep the entry visible with a warning —
            // dropping it would hide exactly the fact the user needs.
            inspection.diagnostics.push(ManagerDiagnostic {
                code: env_managers_codes::PROJECT_PATH_MISSING,
                severity: FindingSeverity::Warning,
                detail: format!("{path}: registered project path does not exist"),
            });
            return inspection;
        }
    };
    if !metadata.is_dir() {
        inspection.diagnostics.push(ManagerDiagnostic {
            code: env_managers_codes::PROJECT_MARKERS_INCOMPLETE,
            severity: FindingSeverity::Warning,
            detail: format!("{path}: registered project path is not a directory"),
        });
        return inspection;
    }
    inspection.path_present = true;

    // Unity version + classification (same policy as the editor matrix).
    // A version file that exists but cannot be read is an observation
    // failure and announces itself (BG-12 family) — only a genuinely
    // absent file is the designed silent absence.
    let version_file = dir.join("ProjectSettings").join("ProjectVersion.txt");
    match std::fs::read_to_string(&version_file) {
        Ok(text) => {
            let raw = text
                .lines()
                .find_map(|line| line.strip_prefix("m_EditorVersion:"))
                .map(str::trim)
                .unwrap_or_default();
            if !raw.is_empty() {
                if let Some((classification, _)) = classify_version_string(raw) {
                    inspection.unity_version = Some(raw.to_owned());
                    inspection.unity_classification = Some(classification);
                } else {
                    inspection.diagnostics.push(ManagerDiagnostic {
                        code: env_managers_codes::PROJECT_VERSION_UNPARSEABLE,
                        severity: FindingSeverity::Warning,
                        detail: format!("{path}: ProjectVersion.txt does not contain a complete version string: {raw}"),
                    });
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // Absent version file: a registered path without the Unity
            // marker is a normal finding (unityVersion stays None).
        }
        Err(error) => {
            inspection.diagnostics.push(ManagerDiagnostic {
                code: env_managers_codes::PROJECT_MARKERS_INCOMPLETE,
                severity: FindingSeverity::Warning,
                detail: format!("{path}: ProjectVersion.txt exists but cannot be read: {error}"),
            });
        }
    }

    // VPM manifest: declared dependencies + locked pins, read as documents.
    let manifest_path = dir.join("Packages").join("vpm-manifest.json");
    if manifest_path.is_file() {
        inspection.manifest_present = true;
        match std::fs::read_to_string(&manifest_path) {
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(value) if value.is_object() => {
                    inspection.manifest_schema_ok = true;
                    let dependencies =
                        manifest_map(&value, "dependencies", path, &mut inspection.diagnostics);
                    let locked = manifest_map(&value, "locked", path, &mut inspection.diagnostics);
                    inspection.dependencies = dependencies;
                    inspection.locked = locked;
                    inspection.vrchat_sdks =
                        vrchat_sdks(&inspection.dependencies, &inspection.locked);
                }
                Ok(_) => inspection.diagnostics.push(ManagerDiagnostic {
                    code: codes::MANIFEST_SCHEMA_UNEXPECTED,
                    severity: FindingSeverity::Warning,
                    detail: format!(
                        "{}: manifest is valid JSON but not an object",
                        manifest_path.display()
                    ),
                }),
                Err(error) => inspection.diagnostics.push(ManagerDiagnostic {
                    code: codes::MANIFEST_SCHEMA_UNEXPECTED,
                    severity: FindingSeverity::Warning,
                    detail: format!(
                        "{}: manifest is not valid JSON: {error}",
                        manifest_path.display()
                    ),
                }),
            },
            Err(error) => inspection.diagnostics.push(ManagerDiagnostic {
                code: codes::MANIFEST_UNREADABLE,
                severity: FindingSeverity::Warning,
                detail: format!("{}: {error}", manifest_path.display()),
            }),
        }
    }

    // Pending-mutation marker: a read-only observation; acquiring the lock
    // would be a write, so the inspection never does.
    inspection.mutation_status = match crate::project_lock::read_pending_mutation(dir) {
        crate::project_lock::PendingMutation::None => MutationStatus::None,
        crate::project_lock::PendingMutation::Leftover(_) => MutationStatus::Leftover,
        crate::project_lock::PendingMutation::Unreadable => MutationStatus::Unreadable,
    };

    // VUA-native identity (`.vua/project.json`): same read-only discipline.
    inspection.vua_identity = crate::vua_identity::read_identity(dir).into();

    inspection
}

/// Extracts a sorted `string -> string` map field from the manifest. A
/// field with the wrong shape yields an empty vector plus a warning —
/// never a guessed entry. Entries whose version value is not a JSON
/// string are skipped, and the skip is announced with the same warning
/// discipline (BG-12 family: a partially readable map is a partial
/// finding, never a silently truncated one).
fn manifest_map(
    manifest: &Value,
    field: &str,
    path: &str,
    diagnostics: &mut Vec<ManagerDiagnostic>,
) -> Vec<ManifestPackage> {
    let mut packages = Vec::new();
    match manifest.get(field) {
        Some(Value::Object(map)) => {
            let mut skipped: Vec<&str> = Vec::new();
            for (package_id, version) in map {
                if let Some(version) = version.as_str() {
                    packages.push(ManifestPackage {
                        package_id: package_id.clone(),
                        version: version.to_owned(),
                    });
                } else {
                    skipped.push(package_id.as_str());
                }
            }
            if !skipped.is_empty() {
                skipped.sort_unstable();
                diagnostics.push(ManagerDiagnostic {
                    code: codes::MANIFEST_SCHEMA_UNEXPECTED,
                    severity: FindingSeverity::Warning,
                    detail: format!(
                        "{path}: manifest {field} carries non-string version values; skipped: {}",
                        skipped.join(", ")
                    ),
                });
            }
        }
        Some(Value::Null) => {}
        Some(_) => {
            diagnostics.push(ManagerDiagnostic {
                code: codes::MANIFEST_SCHEMA_UNEXPECTED,
                severity: FindingSeverity::Warning,
                detail: format!(
                    "{path}: manifest field {field} is present but not an object; reported as empty"
                ),
            });
        }
        None => {}
    }
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    packages
}

/// VRChat SDK packages among the manifest maps: `locked` wins over
/// `dependencies` (the pin is the installed truth), and the union is
/// reported as seen — `com.vrchat.*` is the only claimed pattern.
fn vrchat_sdks(
    dependencies: &[ManifestPackage],
    locked: &[ManifestPackage],
) -> Vec<VrchatSdkFinding> {
    fn is_vrchat_sdk(package_id: &str) -> bool {
        package_id.starts_with("com.vrchat.")
    }
    let mut findings: Vec<VrchatSdkFinding> = Vec::new();
    for package in locked {
        if is_vrchat_sdk(&package.package_id) {
            findings.push(VrchatSdkFinding {
                package_id: package.package_id.clone(),
                source: "locked",
                version: package.version.clone(),
            });
        }
    }
    for package in dependencies {
        if is_vrchat_sdk(&package.package_id)
            && !findings
                .iter()
                .any(|finding| finding.package_id == package.package_id)
        {
            findings.push(VrchatSdkFinding {
                package_id: package.package_id.clone(),
                source: "dependencies",
                version: package.version.clone(),
            });
        }
    }
    findings.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    findings
}
