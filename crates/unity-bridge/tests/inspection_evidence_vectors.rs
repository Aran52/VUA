//! inspection-evidence v0.1 vector tests (FROZEN): the frozen vectors
//! (`schemas/inspection-evidence/v0.1/examples/`) drive JSON-Schema
//! validation over the frozen v0.1 schema. Positive vectors validate;
//! negative vectors (unknown dimension kind, unknown severity,
//! unavailable-with-basis contradiction, missing inspectionId, unknown
//! overallStatus) are rejected. Frozen 2026-09-13 with the proposal 016 §7
//! freeze batch (shape unchanged from the reviewed draft): producing Bridge
//! operations accepted (7d63abe), core store/read routes/task-driven
//! requestRun accepted (7a262b8), bilingual protocol + REGISTRY row landed
//! with the freeze batch. These tests keep pinning vector/schema coherence
//! and the closed-vocabulary consumer convention (BOARD ticket BG-4).

use jsonschema::Validator;
use serde_json::Value;
use std::path::PathBuf;

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/inspection-evidence/v0.1/inspection-evidence.schema.json")
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/inspection-evidence/v0.1/examples")
}

fn read_json(path: &PathBuf) -> Value {
    let bytes = std::fs::read(path).expect("file must exist");
    serde_json::from_slice(&bytes).expect("valid JSON")
}

fn violations(validator: &Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

fn frozen_validator() -> Validator {
    let schema = read_json(&schema_path());
    jsonschema::validator_for(&schema).expect("frozen schema compiles")
}

const POSITIVE: [&str; 2] = [
    "full-five-dimensions.inspection.json",
    "minimal-with-unavailable.inspection.json",
];

const NEGATIVE: [&str; 5] = [
    "invalid-unknown-kind.inspection.json",
    "invalid-severity.inspection.json",
    "invalid-unavailable-with-basis.inspection.json",
    "missing-inspection-id.inspection.json",
    "invalid-unknown-overall.inspection.json",
];

#[test]
fn positive_vectors_validate() {
    let validator = frozen_validator();
    for name in POSITIVE {
        let instance = read_json(&examples_dir().join(name));
        let errors = violations(&validator, &instance);
        assert!(
            errors.is_empty(),
            "positive vector {name} must validate against the frozen v0.1 schema, got: {errors:?}"
        );
    }
}

#[test]
fn negative_vectors_are_rejected() {
    let validator = frozen_validator();
    for name in NEGATIVE {
        let instance = read_json(&examples_dir().join(name));
        let errors = violations(&validator, &instance);
        assert!(
            !errors.is_empty(),
            "negative vector {name} must be rejected by the frozen v0.1 schema"
        );
    }
}

#[test]
fn vector_dimensions_use_the_closed_vocabulary_without_duplicates() {
    // JSON Schema cannot enforce uniqueness by key across the dimensions
    // array; the vector suite carries that consumer convention (the
    // consuming route landed with 7a262b8 and relies on the same closed
    // vocabulary).
    let kinds = [
        "functional",
        "performance",
        "dependencies",
        "lighting",
        "upload_readiness",
    ];
    let validator = frozen_validator();
    for name in POSITIVE {
        let instance = read_json(&examples_dir().join(name));
        assert!(
            violations(&validator, &instance).is_empty(),
            "vector {name} must be schema-clean before the convention check"
        );
        let mut seen: Vec<&str> = Vec::new();
        for dimension in instance["dimensions"].as_array().expect("dimensions array") {
            let kind = dimension["kind"].as_str().expect("kind string");
            assert!(
                kinds.contains(&kind),
                "vector {name}: kind {kind} is outside the closed vocabulary"
            );
            assert!(
                !seen.contains(&kind),
                "vector {name}: dimension kind {kind} appears more than once"
            );
            seen.push(kind);
        }
    }
}

#[test]
fn aggregation_rule_holds_across_positive_vectors() {
    // Declared aggregation (proposal 016): fail if any dimension is fail;
    // else warn if any dimension is warn or unavailable; else pass.
    for name in POSITIVE {
        let instance = read_json(&examples_dir().join(name));
        let mut overall = "pass";
        for dimension in instance["dimensions"].as_array().expect("dimensions array") {
            match dimension["status"].as_str().expect("status string") {
                "fail" => overall = "fail",
                "warn" | "unavailable" if overall != "fail" => overall = "warn",
                _ => {}
            }
        }
        assert_eq!(
            instance["overallStatus"].as_str().expect("overall string"),
            overall,
            "vector {name}: overallStatus must follow the declared aggregation rule"
        );
    }
}
