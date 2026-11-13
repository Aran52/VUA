//! Contract anchor for `schemas/inspection-queries/v0.1` (proposal 016
//! arbitration point 2 — the independent inspection read vocabulary row,
//! get/list after the record.get/list precedent).
//!
//! FROZEN v0.1 (freeze batch 2026-09-13, data role): the freeze
//! hard-preconditions are closed — (1) the production M7 anchor slice
//! (Bridge v3 five-dimension producing operations, c33adb3 → 7d63abe);
//! (2) the core storage+routing implementation batch (merge 7a262b8) plus
//! the data-review revision batch (c914cf2: family-owned
//! INSPECTION_QUERIES_SCHEMA_VERSION unifies the three family replies,
//! avatarRef.ref maxLength dropped to match the evidence body verbatim
//! carry — ratified by the data role); (3) these vectors plus consumer
//! tests green in-tree; (4) the bilingual protocol document and (5) the
//! docs/REGISTRY.md row land with the freeze batch. The vocabulary face is
//! AMF production persistence domain; it never passes through BDL
//! (proposal 016 §5, proposal 011 §5). The ownership chain is the accepted
//! 016 thread plus the wt-4 stance request naming the data role as the
//! taker.

use jsonschema::Validator;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn methods_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/inspection-queries/v0.1/methods")
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/inspection-queries/v0.1/examples")
}

fn evidence_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/inspection-evidence/v0.1")
}

fn read_json_at(dir: &Path, relative: &str) -> Value {
    let bytes = std::fs::read(dir.join(relative)).expect("schema/vector must exist");
    serde_json::from_slice(&bytes).expect("schema/vector must be valid JSON")
}

fn read_json(relative: &str) -> Value {
    read_json_at(&methods_dir(), relative)
}

fn read_example(relative: &str) -> Value {
    read_json_at(&examples_dir(), relative)
}

fn violations(validator: &Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

fn sub_schema_validator(method: &str, def_suffix: &str) -> Validator {
    // Extracting a $defs entry loses the root's $ref context - re-attach
    // the full $defs so nested references (inspectionDocument etc.) resolve.
    // Same compilation shape as the production-use-case v0.2 vector tests
    // (crates/provider-host/tests/production_use_case_vectors.rs).
    let schema = read_json(&format!("{method}.schema.json"));
    let defs = schema
        .get("$defs")
        .expect("method schema has $defs")
        .clone();
    let sub = defs
        .get(format!("{method}{def_suffix}").as_str())
        .expect("sub schema")
        .clone();
    let combined = serde_json::json!({ "allOf": [sub], "$defs": defs });
    jsonschema::validator_for(&combined).expect("sub schema compiles")
}

fn request_validator(method: &str) -> Validator {
    sub_schema_validator(method, "Request")
}

fn result_validator(method: &str) -> Validator {
    sub_schema_validator(method, "Result")
}

#[test]
fn both_methods_have_request_and_result_defs() {
    for method in ["inspection-get", "inspection-list"] {
        let schema = read_json(&format!("{method}.schema.json"));
        let defs = schema.get("$defs").expect("$defs");
        assert!(
            defs.get(format!("{method}Request").as_str()).is_some(),
            "{method} request def missing"
        );
        assert!(
            defs.get(format!("{method}Result").as_str()).is_some(),
            "{method} result def missing"
        );
    }
}

#[test]
fn positive_vectors_validate_against_the_method_faces() {
    let get_request = request_validator("inspection-get");
    let get_result = result_validator("inspection-get");
    let list_request = request_validator("inspection-list");
    let list_result = result_validator("inspection-list");

    let vectors = [
        ("inspection-get.request.json", &get_request),
        ("inspection-get.result.json", &get_result),
        ("inspection-list.request.json", &list_request),
        ("inspection-list.result.json", &list_result),
    ];
    for (name, validator) in vectors {
        let vector = read_example(name);
        let errors = violations(validator, &vector);
        assert!(errors.is_empty(), "{name} must validate: {errors:?}");
    }
}

#[test]
fn negative_vectors_are_rejected() {
    let get_request = request_validator("inspection-get");
    let list_request = request_validator("inspection-list");

    let unknown_param = read_example("invalid-inspection-get-unknown-param.request.json");
    assert!(
        !get_request.is_valid(&unknown_param),
        "an unknown get parameter is a contract error"
    );

    let out_of_range = read_example("invalid-inspection-list-limit-out-of-range.request.json");
    assert!(
        !list_request.is_valid(&out_of_range),
        "limit above the 200 bound is a contract error"
    );

    // Programmed negatives: the closed-set discipline is asserted, not just
    // illustrated by the shipped files.
    let unknown_filter = json!({ "text": "synthetic" });
    assert!(
        !list_request.is_valid(&unknown_filter),
        "no fuzzy text filter is invented on the draft face — unknown filter keys are rejected"
    );
    let out_of_vocabulary = json!({ "overallStatus": "unavailable" });
    assert!(
        !list_request.is_valid(&out_of_vocabulary),
        "overallStatus filter carries the aggregated closed set only (pass|warn|fail)"
    );
}

#[test]
fn inspection_identity_matches_the_evidence_body_convention() {
    // Same identity convention across the vocabulary row and the evidence
    // body schema (uuid v7, W23 production-evidence precedent): the literal
    // patterns must be identical strings, so the two faces cannot drift.
    let get_schema = read_json("inspection-get.schema.json");
    let body_schema = read_json_at(&evidence_dir(), "inspection-evidence.schema.json");
    let query_pattern = get_schema["$defs"]["inspectionId"]["pattern"]
        .as_str()
        .expect("query identity pattern");
    let body_pattern = body_schema["properties"]["inspectionId"]["pattern"]
        .as_str()
        .expect("body identity pattern");
    assert_eq!(query_pattern, body_pattern);
}

#[test]
fn get_result_document_double_validates_against_the_evidence_body_schema() {
    // The cross-vocabulary reference consumed for real: the get result
    // carries the evidence body verbatim; the body must validate in full
    // against the inspection-evidence v0.1 schema (proposal 016 draft, kept
    // in force by the accepted arbitration) while the envelope validates
    // against the method face.
    let get_result = result_validator("inspection-get");
    let body_validator = jsonschema::validator_for(&read_json_at(
        &evidence_dir(),
        "inspection-evidence.schema.json",
    ))
    .expect("evidence body schema compiles");

    let vector = read_example("inspection-get.result.json");
    let errors = violations(&get_result, &vector);
    assert!(errors.is_empty(), "envelope must validate: {errors:?}");
    let document = &vector["inspectionDocument"];
    let body_errors = violations(&body_validator, document);
    assert!(
        body_errors.is_empty(),
        "the carried document must validate against the evidence body schema: {body_errors:?}"
    );
    assert_eq!(
        vector["inspectionId"], document["inspectionId"],
        "the envelope identity and the body identity are the same fact"
    );

    // The sibling positive vector of the draft body (five dimensions with a
    // producing operation per dimension) also rides the read face: the
    // vocabulary row consumes the draft shape as-is.
    let full = read_json_at(
        &evidence_dir(),
        "examples/full-five-dimensions.inspection.json",
    );
    let envelope = json!({
        "inspectionId": full["inspectionId"],
        "inspectionDocument": full,
        "schemaVersion": "0.1"
    });
    let envelope_errors = violations(&get_result, &envelope);
    assert!(
        envelope_errors.is_empty(),
        "the full five-dimension body must ride the get face: {envelope_errors:?}"
    );
}

#[test]
fn list_entry_is_an_identity_row_and_never_inlines_the_body() {
    // Reference, do not copy (012 evidenceIds discipline) as landed on the
    // read face: the entry schema is closed and carries identity/status/
    // ordering facts only — dimensions and checks stay reachable solely via
    // inspection.get.
    let schema = read_json("inspection-list.schema.json");
    let entry = &schema["$defs"]["entry"];
    assert_eq!(entry["additionalProperties"], json!(false));
    let properties = entry["properties"].as_object().expect("entry properties");
    for banned in [
        "dimensions",
        "checks",
        "bridge",
        "notes",
        "inspectionDocument",
    ] {
        assert!(
            !properties.contains_key(banned),
            "the list entry must not inline body member '{banned}'"
        );
    }
    let required = entry["required"].as_array().expect("entry required");
    let required: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(
        required,
        vec!["inspectionId", "avatarRef", "overallStatus", "performedAt"]
    );

    // A row carrying body detail is rejected by the closed entry face.
    let list_result = result_validator("inspection-list");
    let inlining_row = json!({
        "total": 1,
        "entries": [{
            "inspectionId": "01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c21",
            "avatarRef": { "ref": "warehouse:booth-item-1001", "label": null },
            "overallStatus": "warn",
            "performedAt": "2026-09-13T00:20:00Z",
            "dimensions": []
        }],
        "schemaVersion": "0.1"
    });
    assert!(
        !list_result.is_valid(&inlining_row),
        "a list entry inlining dimensions is a contract error"
    );
}
