//! Immutable local Build Record for the B3 production slice.

use crate::{Diagnostic, MaterialEntryMode, RiskDecisionChoice, SourceFolderInspectionV01};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

pub const BUILD_RECORD_SCHEMA_VERSION: &str = "0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildRecordStatus {
    Succeeded,
    SucceededWithWarnings,
    Failed,
    Cancelled,
    Recovered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildSnapshotEvidenceV01 {
    pub snapshot_id: String,
    pub verified: bool,
    pub restore_attempted: bool,
    pub restore_succeeded: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeJobEvidenceV01 {
    pub command_id: String,
    pub operation: String,
    pub status: String,
    pub changed_paths: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalVpmEvidenceV01 {
    pub package_id: String,
    pub display_name: String,
    pub version: String,
    pub manifest_sha256: String,
    pub tree_sha256: String,
    pub archive_sha256: String,
    pub installed_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildValidationEvidenceV01 {
    /// B3 deliberately claims only minimum structure, never Avatar semantics.
    pub level: String,
    pub unity_validated: bool,
    pub expected_assets_loaded: Vec<String>,
    pub diagnostic_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildRecordV01 {
    pub schema_version: String,
    pub record_id: String,
    pub task_id: String,
    pub correlation_id: String,
    pub plan_id: String,
    pub plan_hash: String,
    pub mode: MaterialEntryMode,
    pub status: BuildRecordStatus,
    pub started_at: String,
    pub completed_at: String,
    pub source: SourceFolderInspectionV01,
    pub risk_choice: RiskDecisionChoice,
    pub project_id: String,
    /// Normalized digest of the project root the record belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_identity: Option<String>,
    /// Set on recovery receipts: the failed run's record this one
    /// supersedes, and the user decision that authorized the recovery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovered_from_record_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_decision_id: Option<String>,
    pub initial_project_fingerprint: String,
    pub final_project_fingerprint: Option<String>,
    pub unity_editor_version: String,
    pub snapshot: Option<BuildSnapshotEvidenceV01>,
    pub bridge_jobs: Vec<BridgeJobEvidenceV01>,
    pub local_vpm: Option<LocalVpmEvidenceV01>,
    pub validation: Option<BuildValidationEvidenceV01>,
    pub result_code: String,
}

/// The versioned, presentation-safe wire projection (M3/T1, cosign §5):
/// the renderer consumes THIS shape; the raw evidence sections stay in the
/// stored record at the Orchestrator/diagnostics boundary. Null anchors are
/// pinned: when a section was not attempted, its remaining fields are null.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildRecordWireV02 {
    pub record_id: String,
    pub task_id: String,
    pub plan_id: String,
    pub mode: MaterialEntryMode,
    pub status: BuildRecordStatus,
    pub stages: Vec<&'static str>,
    pub evidence_summary: EvidenceSummary,
    pub restore_attempted: bool,
    pub restore_succeeded: Option<bool>,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceSummary {
    pub snapshot: SnapshotSummary,
    pub bridge: BridgeSummary,
    pub local_vpm: LocalVpmSummary,
    pub validation: ValidationSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSummary {
    pub attempted: bool,
    pub succeeded: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeSummary {
    pub jobs_run: u64,
    pub all_succeeded: Option<bool>,
    pub last_operation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalVpmSummary {
    pub attempted: bool,
    pub published: Option<bool>,
    pub package_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationSummary {
    /// `passed` / `failed` / `skipped` (skipped = not attempted).
    pub status: &'static str,
}

/// The workflow stage chain of the first production vertical use case: both
/// material intake modes traverse the same lifecycle (the dual-channel work
/// happens inside `execute`).
pub const PRODUCTION_STAGES: [&str; 5] =
    ["inspect", "snapshot", "execute", "validate", "completed"];

pub fn evidence_summary(record: &BuildRecordV01) -> EvidenceSummary {
    let snapshot = match &record.snapshot {
        Some(snapshot) => SnapshotSummary {
            attempted: true,
            succeeded: Some(snapshot.verified),
        },
        None => SnapshotSummary {
            attempted: false,
            succeeded: None,
        },
    };
    let jobs_run = record.bridge_jobs.len() as u64;
    let bridge = if record.bridge_jobs.is_empty() {
        BridgeSummary {
            jobs_run: 0,
            all_succeeded: None,
            last_operation: None,
        }
    } else {
        let all_succeeded = record
            .bridge_jobs
            .iter()
            .all(|job| job.status.eq_ignore_ascii_case("succeeded"));
        BridgeSummary {
            jobs_run,
            all_succeeded: Some(all_succeeded),
            last_operation: record.bridge_jobs.last().map(|job| job.operation.clone()),
        }
    };
    let local_vpm = match &record.local_vpm {
        Some(local_vpm) => LocalVpmSummary {
            attempted: true,
            published: Some(!local_vpm.archive_sha256.is_empty()),
            package_id: Some(local_vpm.package_id.clone()),
        },
        None => LocalVpmSummary {
            attempted: false,
            published: None,
            package_id: None,
        },
    };
    let validation = match &record.validation {
        Some(validation) => ValidationSummary {
            status: if validation.unity_validated {
                "passed"
            } else {
                "failed"
            },
        },
        None => ValidationSummary { status: "skipped" },
    };
    EvidenceSummary {
        snapshot,
        bridge,
        local_vpm,
        validation,
    }
}

/// The v0.2 wire projection of a stored record.
pub fn wire_v02(record: &BuildRecordV01) -> BuildRecordWireV02 {
    let (restore_attempted, restore_succeeded) = record
        .snapshot
        .as_ref()
        .map(|snapshot| (snapshot.restore_attempted, snapshot.restore_succeeded))
        .unwrap_or((false, None));
    BuildRecordWireV02 {
        record_id: record.record_id.clone(),
        task_id: record.task_id.clone(),
        plan_id: record.plan_id.clone(),
        mode: record.mode,
        status: record.status,
        stages: PRODUCTION_STAGES.to_vec(),
        evidence_summary: evidence_summary(record),
        restore_attempted,
        restore_succeeded,
        started_at: record.started_at.clone(),
        finished_at: record.completed_at.clone(),
    }
}

/// The build-record document store (records/*.json), identity-addressed by
/// the record id.
///
/// Registry ruling (batch 156, observation B): records of EVERY terminal
/// status — `failed`, `cancelled`, `rolled_back` and `recovered` included —
/// are published here and nowhere else. They are deliberately NOT rows in
/// the `production_domain_records` SQLite registry: that registry's kind
/// closed set is frozen at `('inspection', 'plan')` (schema
/// orchestrator-task-store v0.1 002) and exists solely as the
/// reference-resolution chain (requestPlan resolves an inspectionId,
/// confirmPlan resolves a planId + revision, build receipts resolve the
/// engine plan-id alias). Every consumer of a build record — `record.get`,
/// the release-handoff admission gate, evidence listing — addresses it
/// directly by record id through THIS store, so a build row in the registry
/// (successful or failed) would be dead weight. The observed shape "a
/// failed run has a records/*.json file but no production_domain_records
/// row" is therefore design, not lost bookkeeping; do not "fix" it by
/// registering builds there.
#[derive(Debug, Clone)]
pub struct BuildRecordStore {
    root: PathBuf,
}

impl BuildRecordStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn path_for(&self, record_id: &str) -> io::Result<PathBuf> {
        validate_id(record_id)?;
        Ok(self.root.join(format!("{record_id}.json")))
    }

    /// Publishes exactly once. A repeated production command must read and
    /// return the existing record; it cannot overwrite history.
    pub fn publish(&self, record: &BuildRecordV01) -> io::Result<PathBuf> {
        if record.schema_version != BUILD_RECORD_SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsupported build record version",
            ));
        }
        let destination = self.path_for(&record.record_id)?;
        fs::create_dir_all(&self.root)?;
        let temporary = self.root.join(format!(".{}.tmp", record.record_id));
        let bytes = serde_json::to_vec_pretty(record)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        {
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            output.write_all(&bytes)?;
            output.write_all(b"\n")?;
            output.sync_all()?;
        }
        // A hard link publishes the fully synced inode without an empty-file
        // window and fails rather than replacing an immutable prior record.
        match fs::hard_link(&temporary, &destination) {
            Ok(()) => {
                fs::remove_file(&temporary)?;
                Ok(destination)
            }
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                Err(error)
            }
        }
    }

    pub fn read(&self, record_id: &str) -> io::Result<BuildRecordV01> {
        let bytes = fs::read(self.path_for(record_id)?)?;
        serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}

fn validate_id(value: &str) -> io::Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid build record id",
        ));
    }
    Ok(())
}
