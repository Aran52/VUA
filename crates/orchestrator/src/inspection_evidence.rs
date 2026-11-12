//! M7 inspection slice (proposal 016, hard precondition 2): the AMF
//! production-domain store for inspection-evidence documents
//! (`schemas/inspection-evidence/v0.1/`, DRAFT — the freeze lands with this
//! implementation batch's acceptance) plus the pure transcription face that
//! turns Bridge receipts into evidence dimensions.
//!
//! Store discipline anchors the RecipeRecordStore / EvidenceStore precedent
//! (016 arbitration point 1): documents are immutable observation facts —
//! publish is exactly-once per inspectionId (hard link, fails rather than
//! replacing), `{inspectionId}.json` identity addressing, reads are
//! faithful, an absent root is the honest empty state. Inspection evidence
//! never passes through BDL (proposal 016 §5, proposal 011 §5).
//!
//! Transcription discipline (proposal 016 §2, "转抄不解释"): Bridge receipt
//! diagnostics are transcribed verbatim into dimension checks (code,
//! severity, message, optional metrics). The only derived values are the
//! dimension status (the aggregation rule declared in proposal 016 §4:
//! error → fail, else warning → warn, else pass) and the overallStatus
//! (fail > warn [including unavailable] > pass — an incomplete inspection
//! never reads as a clean pass). No check message is ever rewritten and no
//! official rating is ever inferred (`official_sdk_rating` stays reserved).

use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use crate::model::{ResultStatus, UnityCommand, UnityOperation, UnityPayload};

pub const INSPECTION_EVIDENCE_SCHEMA_VERSION: &str = "0.1";

/// The five M7 dimensions (closed vocabulary, proposal 016 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectionDimension {
    Functional,
    Performance,
    Dependencies,
    Lighting,
    UploadReadiness,
}

impl InspectionDimension {
    pub fn kind(self) -> &'static str {
        match self {
            Self::Functional => "functional",
            Self::Performance => "performance",
            Self::Dependencies => "dependencies",
            Self::Lighting => "lighting",
            Self::UploadReadiness => "upload_readiness",
        }
    }
}

/// The producing operation of each dimension and the wire schema version it
/// speaks (v1 typed checks for functional/performance, v3 read-only
/// inspection operations for the rest — v1/v2 Bridges reject the new
/// operations per the v1-rejects-v2 precedent, so the version is not a
/// free choice).
pub fn producing_operations() -> [(InspectionDimension, UnityOperation, u8, &'static str); 5] {
    [
        (
            InspectionDimension::Functional,
            UnityOperation::ValidateAvatar,
            1,
            "bridge_typed_checks",
        ),
        (
            InspectionDimension::Performance,
            UnityOperation::AnalyzePerformance,
            1,
            "bridge_local_estimate",
        ),
        (
            InspectionDimension::Dependencies,
            UnityOperation::InspectAvatarReferences,
            3,
            "bridge_typed_checks",
        ),
        (
            InspectionDimension::Lighting,
            UnityOperation::InspectLighting,
            3,
            "bridge_typed_checks",
        ),
        (
            InspectionDimension::UploadReadiness,
            UnityOperation::InspectUploadReadiness,
            3,
            "bridge_typed_checks",
        ),
    ]
}

/// Builds the Bridge command for one producing operation. Read-only by
/// definition: every inspection operation runs dry-run (the three v3
/// operations are schema-forced `dryRun: true`; the two v1 check operations
/// are non-mutating). Payload carries only the avatar target — the single
/// closed payload shape of the 016 operation-shape proposal.
pub fn build_inspection_command(
    command_id: &str,
    project_id: &str,
    operation: UnityOperation,
    avatar_global_object_id: &str,
) -> UnityCommand {
    let schema_version = if matches!(
        operation,
        UnityOperation::InspectAvatarReferences
            | UnityOperation::InspectLighting
            | UnityOperation::InspectUploadReadiness
    ) {
        3
    } else {
        1
    };
    UnityCommand {
        schema_version,
        command_id: command_id.to_owned(),
        operation,
        project_id: project_id.to_owned(),
        dry_run: true,
        expected_project_fingerprint: None,
        payload: UnityPayload {
            avatar_global_object_id: avatar_global_object_id.to_owned(),
            ..UnityPayload::default()
        },
    }
}

/// Transcribes one receipt into a dimension object (evidence schema shape).
/// `receipt` = `Some(serde_json::Value)` of the UnityResult document when
/// the Bridge produced one; `None` = the operation could not be observed in
/// this run → the dimension is declared unavailable with basis=none and an
/// empty checks array (honest absence, never a fabricated check).
pub fn transcribe_dimension(
    dimension: InspectionDimension,
    basis: &str,
    receipt: Option<&Value>,
) -> Value {
    let Some(receipt) = receipt else {
        return serde_json::json!({
            "kind": dimension.kind(),
            "status": "unavailable",
            "basis": "none",
            "checks": [],
        });
    };
    let mut checks: Vec<Value> = receipt
        .get("diagnostics")
        .and_then(Value::as_array)
        .map(|diagnostics| {
            diagnostics
                .iter()
                .map(|diagnostic| {
                    let mut check = serde_json::json!({
                        "code": diagnostic.get("code").cloned().unwrap_or(Value::Null),
                        "severity": diagnostic.get("severity").cloned().unwrap_or(Value::Null),
                        "message": diagnostic.get("message").cloned().unwrap_or(Value::Null),
                    });
                    if let Some(metrics) = diagnostic.get("metrics") {
                        if metrics.is_object() {
                            check["metrics"] = metrics.clone();
                        }
                    }
                    check
                })
                .collect()
        })
        .unwrap_or_default();
    // The local-estimate metrics are declared wire facts of the v1 result
    // `data` object (triangles/materialSlots/skinnedMeshRenderers/bones);
    // they transcribe onto the estimate finding — verbatim numbers, never
    // derived. When the receipt carried no check, no synthetic check is
    // invented to host them (the operation entry still declares the run).
    if matches!(dimension, InspectionDimension::Performance) {
        let metrics = estimate_metric_scalars(receipt.get("data"));
        if !metrics.as_object().is_some_and(|map| map.is_empty()) {
            if let Some(first) = checks.get_mut(0) {
                first["metrics"] = metrics;
            }
        }
    }
    let status = dimension_status_from_checks(receipt);
    serde_json::json!({
        "kind": dimension.kind(),
        "status": status,
        "basis": basis,
        "checks": checks,
    })
}

/// The declared mechanical mapping from a receipt's transcribed checks to
/// the dimension status (proposal 016 §2 status semantics): any error
/// finding → fail; else any warning → warn; else pass (checks ran).
fn dimension_status_from_checks(receipt: &Value) -> &'static str {
    let diagnostics = receipt
        .get("diagnostics")
        .and_then(Value::as_array)
        .map(|items| items.as_slice())
        .unwrap_or(&[]);
    let mut status = "pass";
    for diagnostic in diagnostics {
        match diagnostic.get("severity").and_then(Value::as_str) {
            Some("error") => return "fail",
            Some("warning") => status = "warn",
            _ => {}
        }
    }
    status
}

/// Transcribes the declared local-estimate metric scalars out of a v1
/// result `data` object. Unknown keys and non-number values are not
/// metrics and stay out — transcription of declared facts, not a dump.
fn estimate_metric_scalars(data: Option<&Value>) -> Value {
    const METRIC_KEYS: [&str; 4] = [
        "triangles",
        "materialSlots",
        "skinnedMeshRenderers",
        "bones",
    ];
    let mut metrics = serde_json::Map::new();
    if let Some(object) = data.and_then(Value::as_object) {
        for key in METRIC_KEYS {
            if let Some(value) = object.get(key).filter(|value| value.is_number()) {
                metrics.insert(key.to_owned(), value.clone());
            }
        }
    }
    Value::Object(metrics)
}

/// Maps a receipt status into the evidence `bridge.operations[].status`
/// vocabulary (succeeded | failed | rejected).
pub fn operation_status(status: ResultStatus) -> &'static str {
    match status {
        ResultStatus::Succeeded => "succeeded",
        ResultStatus::Failed => "failed",
        ResultStatus::Rejected => "rejected",
    }
}

/// The declared aggregation rule (proposal 016 §4, asserted by the draft
/// vector tests): any dimension fail → fail; else any warn or unavailable
/// → warn; else pass. Unavailable degrades to warn — an incomplete
/// inspection must not read as a clean pass.
pub fn aggregate_overall_status(dimensions: &[Value]) -> &'static str {
    let mut overall = "pass";
    for dimension in dimensions {
        match dimension.get("status").and_then(Value::as_str) {
            Some("fail") => return "fail",
            Some("warn") | Some("unavailable") => overall = "warn",
            _ => {}
        }
    }
    overall
}

/// The AMF production-domain document store for inspection-evidence v0.1
/// documents. Same document-store discipline as the build-record store.
pub struct InspectionEvidenceStore {
    root: PathBuf,
}

impl InspectionEvidenceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path_for(&self, inspection_id: &str) -> io::Result<PathBuf> {
        validate_id(inspection_id)?;
        Ok(self.root.join(format!("{inspection_id}.json")))
    }

    /// Publishes exactly once (hard link: fails rather than replacing an
    /// immutable prior observation). The document must declare
    /// `schemaVersion: "0.1"` and a matching `inspectionId`.
    pub fn publish(&self, inspection_id: &str, document: &Value) -> io::Result<PathBuf> {
        if document.get("schemaVersion").and_then(Value::as_str)
            != Some(INSPECTION_EVIDENCE_SCHEMA_VERSION)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsupported inspection evidence schema version",
            ));
        }
        if document.get("inspectionId").and_then(Value::as_str) != Some(inspection_id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "inspectionId mismatch between identity and document",
            ));
        }
        if self.get(inspection_id)?.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "inspection evidence already exists; observations are immutable",
            ));
        }
        fs::create_dir_all(&self.root)?;
        let destination = self.path_for(inspection_id)?;
        let temporary = self.root.join(format!(".{inspection_id}.tmp"));
        let bytes = serde_json::to_vec_pretty(document)
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

    /// Reads one evidence document (None when absent).
    pub fn get(&self, inspection_id: &str) -> io::Result<Option<Value>> {
        let path = self.path_for(inspection_id)?;
        match fs::read(&path) {
            Ok(bytes) => {
                Ok(Some(serde_json::from_slice(&bytes).map_err(|error| {
                    io::Error::new(io::ErrorKind::InvalidData, error)
                })?))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Identity listing (sorted by inspectionId).
    pub fn list_ids(&self) -> io::Result<Vec<String>> {
        let mut ids = Vec::new();
        if !self.root.exists() {
            return Ok(ids);
        }
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(id) = name.strip_suffix(".json") {
                if !id.starts_with('.') {
                    ids.push(id.to_owned());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Full listing (documents), sorted by inspectionId. An absent root is
    /// the honest empty state (no inspection runs yet). Consumers derive
    /// the performedAt-descending read order and the identity summary rows
    /// (application face, never the store's concern).
    pub fn list_documents(&self) -> io::Result<Vec<Value>> {
        let mut documents = Vec::new();
        for id in self.list_ids()? {
            if let Some(document) = self.get(&id)? {
                documents.push(document);
            }
        }
        Ok(documents)
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
            "invalid inspection id",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn unique_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "vua-inspection-evidence-{tag}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn evidence_document(inspection_id: &str) -> Value {
        json!({
            "schemaVersion": "0.1",
            "inspectionId": inspection_id,
            "avatarRef": {"ref": "warehouse:booth-item-1001", "label": "Synthetic Avatar A"},
            "performedAt": "2026-09-13T00:20:00Z",
            "bridge": {
                "editorVersion": "2022.3.22f1",
                "bridgeSchemaVersion": 3,
                "operations": [
                    {"operation": "validate_avatar", "commandId": "insp-01", "status": "succeeded"}
                ]
            },
            "dimensions": [
                {"kind": "functional", "status": "pass", "basis": "bridge_typed_checks", "checks": [
                    {"code": "validation.passed", "severity": "info", "message": "检查通过。"}
                ]},
                {"kind": "performance", "status": "unavailable", "basis": "none", "checks": []},
                {"kind": "dependencies", "status": "unavailable", "basis": "none", "checks": []},
                {"kind": "lighting", "status": "unavailable", "basis": "none", "checks": []},
                {"kind": "upload_readiness", "status": "unavailable", "basis": "none", "checks": []}
            ],
            "overallStatus": "warn"
        })
    }

    #[test]
    fn new_operations_are_read_only() {
        assert!(!UnityOperation::InspectAvatarReferences.is_mutating());
        assert!(!UnityOperation::InspectLighting.is_mutating());
        assert!(!UnityOperation::InspectUploadReadiness.is_mutating());
    }

    #[test]
    fn serialized_operation_names_match_the_v3_schema_enum() {
        // The v3 command schema and the evidence body schema pin the
        // operation names; the serde rename must never drift from them.
        let names: Vec<String> = [
            UnityOperation::InspectAvatarReferences,
            UnityOperation::InspectLighting,
            UnityOperation::InspectUploadReadiness,
        ]
        .iter()
        .map(|operation| {
            serde_json::to_value(operation)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
        assert_eq!(
            names,
            vec![
                "inspect_avatar_references".to_owned(),
                "inspect_lighting".to_owned(),
                "inspect_upload_readiness".to_owned(),
            ]
        );
    }

    #[test]
    fn inspection_commands_speak_the_right_wire_versions_and_force_dry_run() {
        let command = build_inspection_command(
            "insp-vec-01",
            "proj-1",
            UnityOperation::InspectAvatarReferences,
            "scene:0123",
        );
        assert_eq!(command.schema_version, 3);
        assert!(command.dry_run);
        assert_eq!(command.payload.avatar_global_object_id, "scene:0123");
        let v1_command = build_inspection_command(
            "insp-vec-02",
            "proj-1",
            UnityOperation::ValidateAvatar,
            "scene:0123",
        );
        assert_eq!(v1_command.schema_version, 1);
        assert!(v1_command.dry_run);
        let performance = build_inspection_command(
            "insp-vec-03",
            "proj-1",
            UnityOperation::AnalyzePerformance,
            "scene:0123",
        );
        assert_eq!(performance.schema_version, 1);
    }

    #[test]
    fn transcription_is_verbatim_and_status_mapping_is_declared() {
        let receipt = json!({
            "schemaVersion": 3,
            "commandId": "insp-vec-01",
            "status": "succeeded",
            "changedPaths": [],
            "diagnostics": [
                {"code": "references.missing_material", "severity": "error", "message": "丢失材质槽。"},
                {"code": "references.clean", "severity": "info", "message": "其余引用完整。"}
            ],
            "data": {}
        });
        let dimension = transcribe_dimension(
            InspectionDimension::Dependencies,
            "bridge_typed_checks",
            Some(&receipt),
        );
        assert_eq!(dimension["kind"], "dependencies");
        assert_eq!(dimension["status"], "fail");
        assert_eq!(dimension["basis"], "bridge_typed_checks");
        let checks = dimension["checks"].as_array().unwrap();
        assert_eq!(checks.len(), 2);
        assert_eq!(checks[0]["code"], "references.missing_material");
        assert_eq!(checks[0]["severity"], "error");
        // Transcription never rewrites messages.
        assert_eq!(checks[0]["message"], "丢失材质槽。");

        let warn_receipt = json!({
            "status": "succeeded",
            "diagnostics": [
                {"code": "lighting.realtime_lights_present", "severity": "warning", "message": "存在未烘焙实时光源。"}
            ]
        });
        let lighting = transcribe_dimension(
            InspectionDimension::Lighting,
            "bridge_typed_checks",
            Some(&warn_receipt),
        );
        assert_eq!(lighting["status"], "warn");

        let pass_receipt = json!({"status": "succeeded", "diagnostics": [
            {"code": "validation.passed", "severity": "info", "message": "通过。"}
        ]});
        let functional = transcribe_dimension(
            InspectionDimension::Functional,
            "bridge_typed_checks",
            Some(&pass_receipt),
        );
        assert_eq!(functional["status"], "pass");
    }

    #[test]
    fn estimate_metrics_transcribe_onto_the_finding_without_being_invented() {
        let receipt = json!({
            "schemaVersion": 1,
            "commandId": "insp-02",
            "status": "succeeded",
            "changedPaths": [],
            "diagnostics": [
                {"code": "performance.triangle_estimate_high", "severity": "warning",
                 "message": "三角面较多。"}
            ],
            "data": {
                "basis": "local_estimate",
                "triangles": 92410,
                "materialSlots": 12,
                "skinnedMeshRenderers": 6,
                "bones": 248,
                "recommendations": ["考虑 LOD"]
            }
        });
        let dimension = transcribe_dimension(
            InspectionDimension::Performance,
            "bridge_local_estimate",
            Some(&receipt),
        );
        assert_eq!(dimension["status"], "warn");
        let check = &dimension["checks"][0];
        assert_eq!(check["metrics"]["triangles"], 92410);
        assert_eq!(check["metrics"]["bones"], 248);
        // Non-metric keys stay out of the transcription.
        assert!(check["metrics"].get("basis").is_none());
        assert!(check["metrics"].get("recommendations").is_none());

        // No diagnostics → no synthetic check is invented to host metrics.
        let bare = json!({
            "schemaVersion": 1, "commandId": "insp-03", "status": "succeeded",
            "changedPaths": [],
            "diagnostics": [],
            "data": {"triangles": 100}
        });
        let empty = transcribe_dimension(
            InspectionDimension::Performance,
            "bridge_local_estimate",
            Some(&bare),
        );
        assert_eq!(empty["checks"].as_array().unwrap().len(), 0);
        assert_eq!(empty["status"], "pass");
    }

    #[test]
    fn unobservable_operations_surface_as_unavailable_with_empty_checks() {
        let dimension = transcribe_dimension(
            InspectionDimension::Performance,
            "bridge_local_estimate",
            None,
        );
        assert_eq!(dimension["status"], "unavailable");
        assert_eq!(dimension["basis"], "none");
        assert_eq!(dimension["checks"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn aggregation_rule_fail_gt_warn_including_unavailable_gt_pass() {
        let unavailable = json!({"status": "unavailable", "basis": "none", "checks": []});
        let warn = json!({"status": "warn", "basis": "bridge_typed_checks", "checks": []});
        let pass = json!({"status": "pass", "basis": "bridge_typed_checks", "checks": []});
        let fail = json!({"status": "fail", "basis": "bridge_typed_checks", "checks": []});
        assert_eq!(
            aggregate_overall_status(&[pass.clone(), pass.clone()]),
            "pass"
        );
        assert_eq!(
            aggregate_overall_status(&[pass.clone(), unavailable.clone()]),
            "warn"
        );
        assert_eq!(
            aggregate_overall_status(&[pass.clone(), warn.clone()]),
            "warn"
        );
        assert_eq!(aggregate_overall_status(&[warn, fail.clone()]), "fail");
        assert_eq!(aggregate_overall_status(&[unavailable, fail]), "fail");
    }

    #[test]
    fn publish_is_exactly_once_and_reads_back_faithfully() {
        let store = InspectionEvidenceStore::new(unique_root("once"));
        let inspection_id = "01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c21";
        let document = evidence_document(inspection_id);
        store.publish(inspection_id, &document).unwrap();
        assert!(store.publish(inspection_id, &document).is_err());
        let read = store.get(inspection_id).unwrap().unwrap();
        assert_eq!(read, document);
    }

    #[test]
    fn mismatched_identity_and_bad_versions_are_typed_errors() {
        let store = InspectionEvidenceStore::new(unique_root("guards"));
        let inspection_id = "01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c22";
        let mut document = evidence_document(inspection_id);
        document["schemaVersion"] = Value::String("0.2".into());
        assert!(store.publish(inspection_id, &document).is_err());
        let document = evidence_document("01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c23");
        assert!(store.publish(inspection_id, &document).is_err());
    }

    #[test]
    fn absent_root_is_the_honest_empty_state() {
        let store = InspectionEvidenceStore::new(unique_root("empty"));
        assert_eq!(store.list_documents().unwrap().len(), 0);
        assert_eq!(store.list_ids().unwrap().len(), 0);
        assert!(store
            .get("01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c24")
            .unwrap()
            .is_none());
    }

    #[test]
    fn listing_is_identity_sorted() {
        let store = InspectionEvidenceStore::new(unique_root("list"));
        let later = "01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c31";
        let earlier = "01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c30";
        store.publish(later, &evidence_document(later)).unwrap();
        store.publish(earlier, &evidence_document(earlier)).unwrap();
        let ids = store.list_ids().unwrap();
        assert_eq!(ids, vec![earlier.to_owned(), later.to_owned()]);
        assert_eq!(store.list_documents().unwrap().len(), 2);
    }
}
