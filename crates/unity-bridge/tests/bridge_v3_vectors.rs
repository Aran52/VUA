//! M7 inspection-slice consumer tests: unity-bridge v3 vectors (proposal 016
//! hard precondition 1, operation-shape proposal 2026-09-12).
//!
//! v3 is a same-face superset of v2: every v2 vector still validates under v3
//! with `schemaVersion` bumped, and the three new read-only inspection
//! operations (`inspect_avatar_references`, `inspect_lighting`,
//! `inspect_upload_readiness`) pin the proposal-016 operation-shape rulings —
//! read-only means `dryRun` forced true, the payload requires only
//! `avatarGlobalObjectId`, and findings travel as typed diagnostics codes.
//! A succeeded inspection receipt carries at least one diagnostic (a
//! completed check always has a conclusion) and an empty `changedPaths`
//! (read-only operations change nothing). v3 also legalizes the proposal-011
//! `instanceGlobalObjectId` receipt field (reserved in C# since aa2a9da,
//! previously missing from the frozen v2 schema).

use serde_json::Value;
use std::path::PathBuf;

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/unity-bridge/v3")
}

fn read_json(relative: &str) -> Value {
    let bytes = std::fs::read(schema_dir().join(relative)).expect("schema/vector must exist");
    serde_json::from_slice(&bytes).expect("schema/vector must be valid JSON")
}

fn command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_json("command.schema.json")).unwrap()
}

fn result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_json("result.schema.json")).unwrap()
}

#[test]
fn w21_positive_vectors_validate_against_both_draft_schemas() {
    let command_validator = command_validator();
    let result_validator = result_validator();
    for name in [
        "examples/inspect-references.request.json",
        "examples/inspect-lighting.request.json",
        "examples/inspect-upload-readiness.request.json",
    ] {
        let vector = read_json(name);
        assert!(
            command_validator.is_valid(&vector),
            "{name} must validate against the v3 command schema"
        );
    }
    for name in [
        "examples/inspect-references-findings.result.json",
        "examples/inspect-lighting-clean.result.json",
        "examples/inspect-upload-readiness.result.json",
    ] {
        let vector = read_json(name);
        assert!(
            result_validator.is_valid(&vector),
            "{name} must validate against the v3 result schema"
        );
    }
    // Proposal-011 success-criterion field legalized in v3 (drift declared
    // in proposal 016): an install_modular_asset receipt may carry the
    // instance-root GlobalObjectId.
    let install_receipt = read_json("examples/production-job-run-instance.result.json");
    assert!(
        result_validator.is_valid(&install_receipt),
        "instanceGlobalObjectId receipt must validate under v3"
    );
    assert!(install_receipt["data"]["instanceGlobalObjectId"]
        .as_str()
        .is_some());
}

#[test]
fn w21v2_operations_remain_valid_as_v3_superset() {
    // Same-face upgrade: a v2-shaped command (operation + payload) with
    // schemaVersion 3 must still validate — v3 adds inspection operations,
    // it does not remove or re-shape v2 ones.
    let command_validator = command_validator();
    let v2_job: Value = serde_json::json!({
        "schemaVersion": 3,
        "commandId": "v2-compat-01",
        "operation": "execute_production_job",
        "projectId": "proj",
        "dryRun": true,
        "payload": {
            "planHash": "sha256:1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5d6e7f809",
            "planSchemaVersion": "0.3",
            "planRef": "recipe/plans/plan-0001.json"
        }
    });
    assert!(
        command_validator.is_valid(&v2_job),
        "v2 execute_production_job must remain valid under v3"
    );
}

#[test]
fn w21v2_receipts_remain_valid_as_v3_superset() {
    // A v2 production-job receipt with schemaVersion 3 stays valid: v3 does
    // not reshape v2 receipt semantics.
    let result_validator = result_validator();
    let v2_receipt: Value = serde_json::json!({
        "schemaVersion": 3,
        "commandId": "job-run-02",
        "operation": "execute_production_job",
        "status": "rejected",
        "changedPaths": [],
        "diagnostics": [
            { "code": "plan_schema_version_unsupported", "severity": "error", "message": "outside the supported set" }
        ],
        "data": {
            "dryRun": false,
            "replayed": false,
            "planHash": "sha256:9a8b7c6d5e4f302918a7b6c5d4e3f2091a8b7c6d5e4f302918a7b6c5d4e3f209",
            "steps": []
        }
    });
    assert!(
        result_validator.is_valid(&v2_receipt),
        "v2 receipt shape must remain valid under v3"
    );
}

#[test]
fn w21v3_negative_command_vectors_are_rejected() {
    let command_validator = command_validator();
    for name in [
        "examples/invalid-inspection-operation.json",
        "examples/inspection-realrun-forbidden.json",
        "examples/inspection-missing-avatar.json",
    ] {
        let vector = read_json(name);
        assert!(
            !command_validator.is_valid(&vector),
            "{name} must be rejected by the v3 command schema"
        );
    }
}

#[test]
fn w21v3_negative_result_vectors_are_rejected() {
    let result_validator = result_validator();
    let vector = read_json("examples/inspection-empty-diagnostics.result.json");
    assert!(
        !result_validator.is_valid(&vector),
        "a succeeded inspection receipt with no diagnostic must be rejected — a completed check always has a conclusion"
    );
}

#[test]
fn w21v3_inspections_are_read_only_and_findings_stay_in_diagnostics() {
    // The three v3 operations are forced dry-run (read-only) and their
    // receipts carry findings in diagnostics only: no inspection-specific
    // data fields were added in v3 (the single added data field is the
    // proposal-011 instanceGlobalObjectId legalization), and succeeded
    // inspection receipts change nothing.
    let command_validator = command_validator();
    let result_validator = result_validator();
    for name in [
        "examples/inspect-references.request.json",
        "examples/inspect-lighting.request.json",
        "examples/inspect-upload-readiness.request.json",
    ] {
        let request = read_json(name);
        assert_eq!(
            request["dryRun"],
            serde_json::json!(true),
            "{name}: inspection operations are read-only"
        );
        assert!(
            command_validator.is_valid(&request),
            "{name} must be a valid v3 command"
        );
    }
    for name in [
        "examples/inspect-references-findings.result.json",
        "examples/inspect-lighting-clean.result.json",
        "examples/inspect-upload-readiness.result.json",
    ] {
        let receipt = read_json(name);
        assert!(
            result_validator.is_valid(&receipt),
            "{name} must be a valid v3 receipt"
        );
        assert!(
            receipt["changedPaths"].as_array().unwrap().is_empty(),
            "{name}: read-only inspections change nothing"
        );
        for (field, _) in receipt["data"].as_object().unwrap() {
            let field: &str = field;
            assert!(
                [
                    "projectFingerprint",
                    "projectFingerprintBefore",
                    "avatarName",
                    "outfitName",
                    "basis",
                    "triangles",
                    "materialSlots",
                    "skinnedMeshRenderers",
                    "bones",
                    "recommendations",
                    "importedAssetPaths",
                    "packageRoot",
                    "loadedAssetPaths",
                    "commandFingerprint",
                    "replayed",
                    "dryRun",
                    "planHash",
                    "steps",
                    "snapshotId",
                    "restoredFrom",
                    "instanceGlobalObjectId"
                ]
                .contains(&field),
                "{name}: unexpected v3 data field {field}"
            );
        }
    }
}
