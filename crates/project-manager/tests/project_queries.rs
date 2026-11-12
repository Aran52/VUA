//! M6 T-A (proposal 013) tests: the read-face command word list frozen in
//! `schemas/project-inspection/v0.2/command.schema.json` + result envelope.
//! The frozen example vectors must validate, the negatives must be refused,
//! and the envelope must not leak into the payload documents (whose shapes
//! are pinned by their own schemas and tested in project_inspection.rs).

use serde_json::Value;
use std::fs;
use std::path::Path;

fn read_repo_json(relative: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/project-inspection/v0.2/command.schema.json",
    ))
    .unwrap()
}

fn result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/project-inspection/v0.2/result.schema.json",
    ))
    .unwrap()
}

fn violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

#[test]
fn all_positive_request_vectors_pass_the_command_schema() {
    for name in [
        "project-list-projects.request.json",
        "project-inspect-project.request.json",
        "project-environment-managers.request.json",
        "project-lock-status.request.json",
    ] {
        let vector = read_repo_json(&format!("schemas/project-inspection/v0.2/examples/{name}"));
        let problems = violations(&command_validator(), &vector);
        assert!(problems.is_empty(), "{name}: {problems:#?}");
    }
}

#[test]
fn all_positive_result_vectors_pass_the_result_envelope() {
    for name in [
        "project-list-projects.result.json",
        "project-inspect-project.result.json",
        "project-environment-managers.result.json",
        "project-lock-status.result.json",
    ] {
        let vector = read_repo_json(&format!("schemas/project-inspection/v0.2/examples/{name}"));
        let problems = violations(&result_validator(), &vector);
        assert!(problems.is_empty(), "{name}: {problems:#?}");
    }
}

#[test]
fn negative_vectors_are_refused_by_the_command_schema() {
    for name in [
        "invalid-inspect-missing-path.json",
        "invalid-read-face-operation.json",
    ] {
        let vector = read_repo_json(&format!("schemas/project-inspection/v0.2/examples/{name}"));
        let problems = violations(&command_validator(), &vector);
        assert!(
            !problems.is_empty(),
            "{name} must be refused by the frozen word list"
        );
    }
}

#[test]
fn the_read_word_list_stays_separate_from_the_write_word_list() {
    // The write command must NOT be part of the read-face operation closed
    // set (read/write lines are separate by the 014 arbitration).
    let write_command =
        read_repo_json("schemas/project-ops/v0.2/examples/project-import-copy-plan.request.json");
    let problems = violations(&command_validator(), &write_command);
    assert!(
        !problems.is_empty(),
        "project.import-copy must be refused by the read-face word list"
    );

    // …and the read-face result payloads are not valid write-command
    // requests either.
    let read_request =
        read_repo_json("schemas/project-inspection/v0.2/examples/project-lock-status.request.json");
    let ops_validator = jsonschema::validator_for(&read_repo_json(
        "schemas/project-ops/v0.2/command.schema.json",
    ))
    .unwrap();
    assert!(!violations(&ops_validator, &read_request).is_empty());
}
