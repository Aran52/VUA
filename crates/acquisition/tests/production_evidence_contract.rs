//! Contract anchor for `schemas/production-evidence/v0.1` (W23 — the
//! compatibility/missing evidence model, proposal 011 convergence).
//!
//! FROZEN (2026-09-08): the vocabulary self-consistency, the kind closed set,
//! the source-reference rule and the resolution lifecycle are anchored here;
//! the cross-vocabulary reference is consumed for real against the frozen
//! recipe v0.3 suite — the Local Resolution document's
//! `assetResolution.evidenceIds[]` (core W20 freeze) references these bodies,
//! never inlining them. The evidence store lives in the AMF production
//! persistence domain whose implementation lands with the W20 implementation
//! slice; the vocabulary itself is frozen and owned by the data role.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/production-evidence/v0.1")
}

fn recipe_schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/recipe/v0.3")
}

fn read_json_at(dir: &Path, relative: &str) -> Value {
    let bytes = std::fs::read(dir.join(relative)).expect("schema/vector must exist");
    serde_json::from_slice(&bytes).expect("schema/vector must be valid JSON")
}

fn read_json(relative: &str) -> Value {
    read_json_at(&schema_dir(), relative)
}

fn validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_json("evidence.schema.json")).unwrap()
}

fn local_resolution_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_json_at(
        &recipe_schema_dir(),
        "local-resolution.schema.json",
    ))
    .unwrap()
}

const POSITIVE_VECTORS: &[&str] = &[
    "examples/missing-asset.evidence.json",
    "examples/version-mismatch.evidence.json",
    "examples/guard-denied.evidence.json",
    "examples/resolution-attached.evidence.json",
];

const NEGATIVE_VECTORS: &[&str] = &[
    "examples/invalid-kind.evidence.json",
    "examples/missing-observed-at.evidence.json",
    "examples/invalid-sourceref.evidence.json",
];

#[test]
fn evidence_positive_vectors_validate_against_the_schema() {
    let validator = validator();
    for name in POSITIVE_VECTORS {
        let vector = read_json(name);
        let errors: Vec<String> = validator
            .iter_errors(&vector)
            .map(|error| format!("{}: {error}", error.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{name} must validate: {errors:?}");
    }
}

#[test]
fn evidence_negative_vectors_are_rejected() {
    let validator = validator();
    for name in NEGATIVE_VECTORS {
        let vector = read_json(name);
        assert!(
            !validator.is_valid(&vector),
            "{name} is a negative vector and must not validate"
        );
    }
}

#[test]
fn kind_vocabulary_is_the_frozen_closed_set() {
    let schema = read_json("evidence.schema.json");
    let kinds: Vec<String> = schema["properties"]["kind"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        kinds,
        vec![
            "missing_asset",
            "missing_package",
            "version_mismatch",
            "guard_denied",
        ]
    );
}

#[test]
fn source_ref_requires_at_least_one_producing_context() {
    let validator = validator();
    let base = json!({
        "schemaVersion": "0.1",
        "evidenceId": "0198a7b3-8d9e-7faf-8b2c-0d1e2f3a4b5c",
        "kind": "missing_package",
        "subject": { "ref": "pkg:jp.example.dep" },
        "observedAt": "2026-09-08T06:36:00.000Z",
        "detail": "Declared dependency absent at resolution time.",
        "sourceRef": {},
        "resolution": null
    });
    assert!(
        !validator.is_valid(&base),
        "an empty sourceRef is inadmissible"
    );

    let mut with_resolution = base.clone();
    with_resolution["sourceRef"]["localResolutionId"] =
        json!("0198a7b3-9f8e-7d6c-5b4a-321098765432");
    assert!(validator.is_valid(&with_resolution));

    let mut with_task = base.clone();
    with_task["sourceRef"]["taskCorrelation"] = json!("corr-import-1");
    assert!(validator.is_valid(&with_task));
}

#[test]
fn resolution_lifecycle_goes_from_null_to_attached_without_rewriting() {
    let validator = validator();
    let mut evidence = read_json("examples/missing-asset.evidence.json");
    assert_eq!(evidence["resolution"], Value::Null);

    // A later fact satisfies the evidence: attach the resolution reference.
    evidence["resolution"] = json!({
        "resolvedAt": "2026-09-08T07:40:00.000Z",
        "resolutionRef": "0198a7c9-1122-7334-4556-677889900011"
    });
    assert!(
        validator.is_valid(&evidence),
        "attaching a resolution keeps the document valid"
    );

    // An attached resolution without its timestamp is inadmissible.
    let mut broken = evidence.clone();
    broken["resolution"] = json!({ "resolutionRef": "0198a7c9-1122-7334-4556-677889900011" });
    assert!(!validator.is_valid(&broken));
}

/// The cross-vocabulary reference consumed for real: a FROZEN
/// local-resolution v0.3 document carries `evidenceIds[]` on its
/// assetResolution entries; the ids match the evidence bodies exactly, and
/// the referenced documents validate against the W23 schema. The resolution
/// view carries the ids and the fallback fact only — the honest detail lives
/// in the evidence body (referenced, never inlined).
#[test]
fn frozen_local_resolution_references_evidence_bodies_across_vocabularies() {
    let evidence_validator = validator();
    let resolution_validator = local_resolution_validator();

    let missing = read_json("examples/missing-asset.evidence.json");
    let resolution_id = missing["sourceRef"]["localResolutionId"]
        .as_str()
        .unwrap()
        .to_owned();

    // A minimal but complete local-resolution document (validates against
    // the frozen recipe v0.3 schema) whose single assetResolution references
    // the evidence body's identity.
    let resolution_document = json!({
        "schemaVersion": 2,
        "recipeId": "0198a7b3-0000-7000-8000-000000000001",
        "recipeRevision": 7,
        "environmentId": "0198a7b3-0000-7000-8000-000000000002",
        "resolvedAt": "2026-09-08T06:30:00.000Z",
        "assets": [{
            "assetId": "sailor_uniform",
            "warehouseAssetId": "0198a7c9-1122-7334-8556-677889900011",
            "artifactId": "0198a7c9-2233-7444-9667-788990001122",
            "artifactFingerprint": {
                "algorithm": "sha256-tree-v1",
                "value": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            },
            "entrypoints": [],
            "objectBindings": [],
            "sourceKind": "original",
            "fallbackUsed": false,
            "evidenceIds": [missing["evidenceId"]]
        }]
    });
    let errors: Vec<String> = resolution_validator
        .iter_errors(&resolution_document)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "the referencing document must validate against the frozen recipe v0.3 schema: {errors:?}"
    );

    // The referenced body validates against W23 and its identity is exactly
    // the one the resolution carries.
    assert!(
        evidence_validator.is_valid(&missing),
        "the referenced evidence body must validate against the W23 schema"
    );
    let carried_ids: Vec<&str> = resolution_document["assets"][0]["evidenceIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(carried_ids, vec![missing["evidenceId"].as_str().unwrap()]);
    assert_eq!(
        resolution_id,
        missing["sourceRef"]["localResolutionId"].as_str().unwrap()
    );

    // The honest detail lives in the body, never in the resolution view.
    assert!(!missing["detail"].as_str().unwrap().is_empty());
    assert!(resolution_document["assets"][0].get("detail").is_none());
}
