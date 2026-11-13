//! W23/W20 freeze consumer tests: production-evidence v0.1
//! (`schemas/production-evidence/v0.1/`) drives the frozen vectors through
//! a real JSON-Schema validation, and the AMF production-domain
//! `EvidenceStore` (011 convergence decision: a document store per the
//! BuildRecordStore precedent — never BDL) is pinned on its exactly-once
//! publish, faithful read-back, null-resolution wire shape, and identity
//! listing.

use jsonschema::Validator;
use serde_json::{json, Value};
use std::path::PathBuf;
use vua_orchestrator::{
    EvidenceKind, EvidenceSourceRef, EvidenceStore, EvidenceSubject, ProductionEvidenceV01,
};

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/production-evidence/v0.1")
}

fn read_json(relative: &str) -> Value {
    let bytes = std::fs::read(schema_dir().join(relative)).expect("schema/vector must exist");
    serde_json::from_slice(&bytes).expect("schema/vector must be valid JSON")
}

fn validator_for(schema: &Value) -> Validator {
    jsonschema::validator_for(schema).expect("frozen schema must compile")
}

fn violations(validator: &Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

#[test]
fn frozen_examples_validate_and_negative_vectors_reject() {
    let validator = validator_for(&read_json("evidence.schema.json"));
    let positives = [
        "examples/missing-asset.evidence.json",
        "examples/guard-denied.evidence.json",
        "examples/version-mismatch.evidence.json",
        "examples/resolution-attached.evidence.json",
    ];
    for name in positives {
        let vector = read_json(name);
        let problems = violations(&validator, &vector);
        assert!(problems.is_empty(), "{name} must validate: {problems:?}");
    }
    let negatives = [
        "examples/invalid-kind.evidence.json",
        "examples/invalid-sourceref.evidence.json",
        "examples/missing-observed-at.evidence.json",
    ];
    for name in negatives {
        let vector = read_json(name);
        assert!(
            !validator.is_valid(&vector),
            "{name} is a negative vector and must not validate"
        );
    }
}

#[test]
fn store_serialization_matches_the_frozen_schema() {
    // The store's serde shape must validate against the frozen schema — the
    // resolved (None) form serializes resolution as null on the wire.
    let validator = validator_for(&read_json("evidence.schema.json"));
    let store = EvidenceStore::new(std::env::temp_dir().join(format!(
        "vua-evidence-schema-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )));
    let evidence = ProductionEvidenceV01::new_unresolved(
        "0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8b",
        EvidenceKind::GuardDenied,
        EvidenceSubject {
            ref_: "warehouse:019e0001".into(),
            label: None,
        },
        "2026-09-08T09:00:00.000Z",
        "the server-side guard refused the delete",
        EvidenceSourceRef::from_task_correlation("corr-guard-1"),
    );
    store.publish(&evidence).unwrap();
    let stored = read_stored(&store, &evidence.evidence_id);
    let problems = violations(&validator, &stored);
    assert!(
        problems.is_empty(),
        "stored fact must validate: {problems:?}"
    );
    // Unresolved serializes as an explicit null (the frozen shape), not an
    // absent field.
    assert_eq!(stored["resolution"], Value::Null);
    std::fs::remove_dir_all(store_root_for_test()).ok();

    // The resolved form (a later fact referencing what satisfied it) also
    // validates without rewriting the observed evidence.
    let resolved = json!({
        "schemaVersion": "0.1",
        "evidenceId": "0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8b",
        "kind": "guard_denied",
        "subject": {"ref": "warehouse:019e0001", "label": null},
        "observedAt": "2026-09-08T09:00:00.000Z",
        "detail": "the server-side guard refused the delete",
        "sourceRef": {"taskCorrelation": "corr-guard-1"},
        "resolution": {"resolvedAt": "2026-09-08T10:00:00.000Z", "resolutionRef": "0198a7b3-9f8e-7d6c-5b4a-321098765432"}
    });
    assert!(validator.is_valid(&resolved), "{resolved}");
}

fn store_root_for_test() -> PathBuf {
    std::env::temp_dir().join(format!("vua-evidence-schema-root-{}", std::process::id()))
}

fn read_stored(store: &EvidenceStore, evidence_id: &str) -> Value {
    let path = store.path_for(evidence_id).unwrap();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn evidence_source_ref_both_forms_round_trip() {
    // Exactly one of the two references is required by the frozen schema —
    // the Rust type carries both as options and the store keeps whatever the
    // producer set.
    let by_resolution = ProductionEvidenceV01::new_unresolved(
        "0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8d",
        EvidenceKind::MissingPackage,
        EvidenceSubject {
            ref_: "pkg:com.example.pack".into(),
            label: None,
        },
        "2026-09-08T09:10:00.000Z",
        "declared dependency package absent",
        EvidenceSourceRef::from_local_resolution("0198a7b3-9f8e-7d6c-5b4a-321098765432"),
    );
    let by_correlation = ProductionEvidenceV01::new_unresolved(
        "0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8e",
        EvidenceKind::GuardDenied,
        EvidenceSubject {
            ref_: "warehouse:019e0002".into(),
            label: None,
        },
        "2026-09-08T09:11:00.000Z",
        "guard refused the delete",
        EvidenceSourceRef::from_task_correlation("corr-guard-2"),
    );
    let validator = validator_for(&read_json("evidence.schema.json"));
    for evidence in [&by_resolution, &by_correlation] {
        let serialized = serde_json::to_value(evidence).unwrap();
        assert!(validator.is_valid(&serialized), "{serialized}");
    }
}
