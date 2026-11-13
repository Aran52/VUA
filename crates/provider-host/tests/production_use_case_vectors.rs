//! W20 freeze consumer tests: the production-use-case v0.2 vectors
//! (`schemas/production-use-case/v0.2/examples/`) drive real JSON-Schema
//! validation over the frozen ten-method schemas. Positive request/result
//! vectors validate; negative vectors (out-of-range limits, missing
//! required params, unknown params) are rejected.

use jsonschema::Validator;
use serde_json::Value;
use std::path::PathBuf;

fn methods_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/production-use-case/v0.2/methods")
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/production-use-case/v0.2/examples")
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

fn sub_schema_validator(method: &str, def_suffix: &str) -> Validator {
    // Extracting a $defs entry loses the root's $ref context - re-attach
    // the full $defs so nested references (recipeDocument etc.) resolve.
    let schema = read_json(&methods_dir().join(format!("{method}.schema.json")));
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

fn method_validator(method: &str) -> Validator {
    sub_schema_validator(method, "Request")
}

fn result_validator(method: &str) -> Validator {
    sub_schema_validator(method, "Result")
}

#[test]
fn all_ten_methods_have_frozen_request_and_result_schemas() {
    let methods = [
        "recipe-save",
        "recipe-get",
        "recipe-list",
        "recipe-resolve",
        "plan-approve",
        "plan-get",
        "plan-list",
        "job-execute",
        "record-get",
        "record-list",
    ];
    for method in methods {
        let request = read_json(&methods_dir().join(format!("{method}.schema.json")));
        let defs = request.get("$defs").expect("$defs");
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
fn positive_request_and_result_vectors_validate() {
    let positive_pairs = [
        (
            "recipe-save",
            "recipe-save.request.json",
            "recipe-save.result.json",
        ),
        (
            "recipe-get",
            "recipe-get.request.json",
            "recipe-get.result.json",
        ),
        (
            "recipe-list",
            "recipe-list.request.json",
            "recipe-list.result.json",
        ),
        (
            "recipe-resolve",
            "recipe-resolve.request.json",
            "recipe-resolve.result.json",
        ),
        (
            "plan-approve",
            "plan-approve.request.json",
            "plan-approve.result.json",
        ),
        ("plan-get", "plan-get.request.json", "plan-get.result.json"),
        (
            "plan-list",
            "plan-list.request.json",
            "plan-list.result.json",
        ),
        (
            "job-execute",
            "job-execute.request.json",
            "job-execute.result.json",
        ),
        (
            "record-get",
            "record-get.request.json",
            "record-get.result.json",
        ),
        (
            "record-list",
            "record-list.request.json",
            "record-list.result.json",
        ),
    ];
    for (method, request_vector, result_vector) in positive_pairs {
        // Request vectors carry the transport envelope; the params inside
        // are what the frozen method schema governs.
        let request = read_json(&examples_dir().join(request_vector));
        let params = request
            .get("params")
            .cloned()
            .expect("request vector carries params");
        let request_problems = violations(&method_validator(method), &params);
        assert!(
            request_problems.is_empty(),
            "{request_vector} must validate: {request_problems:?}"
        );
        let result = read_json(&examples_dir().join(result_vector));
        let result_problems = violations(&result_validator(method), &result);
        assert!(
            result_problems.is_empty(),
            "{result_vector} must validate: {result_problems:?}"
        );
    }
}

#[test]
fn negative_request_vectors_are_rejected() {
    let negatives = [
        (
            "recipe-save",
            "invalid-recipe-save-no-base-revision.request.json",
        ),
        (
            "recipe-list",
            "invalid-recipe-list-limit-out-of-range.request.json",
        ),
        (
            "plan-approve",
            "invalid-plan-approve-missing-plan-id.request.json",
        ),
        (
            "job-execute",
            "invalid-job-execute-unknown-param.request.json",
        ),
    ];
    for (method, vector_name) in negatives {
        let vector = read_json(&examples_dir().join(vector_name));
        let problems = violations(&method_validator(method), &vector);
        assert!(
            !problems.is_empty(),
            "{vector_name} is a negative vector and must not validate"
        );
    }
}
