//! Contract anchor for `schemas/editor-verify/v0.1` (proposal 021 vocabulary
//! row `environment.verifyEditor`, core ruling 2026-09-13 seven points).
//!
//! DRAFT drift protection (proposal 016 precedent): the draft state does not
//! imply freeze — the freeze batch (bilingual protocol document, REGISTRY
//! row, SCHEMA_EXEMPT removal) lands later. This test pins the wire shapes to
//! the primitive of record (`verify_editor_path`, 3eef4e4) and to the core
//! classification authority (`classify_editor`), so the schema and vectors
//! cannot drift silently before the core routing batch consumes them.
//!
//! The three nails from ruling point 4 are asserted here at the schema face:
//! (1) a refusal is a VALID result state, never an application error
//! envelope; (2) refusal details are carried verbatim (no re-interpretation
//! is even possible through the closed result shape); (3) the envelope
//! schemaVersion is the const "0.1".

use jsonschema::Validator;
use serde_json::Value;
use vua_orchestrator::{classify_editor, parse_editor_version};

fn read_json_at(dir: impl AsRef<std::path::Path>, relative: &str) -> Value {
    let bytes = std::fs::read(dir.as_ref().join(relative)).expect("schema/vector must exist");
    serde_json::from_slice(&bytes).expect("schema/vector must be valid JSON")
}

fn schemas_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/editor-verify/v0.1")
}

fn violations(validator: &Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

fn sub_schema_validator(def_suffix: &str) -> Validator {
    // Extracting a $defs entry loses the root's $ref context - re-attach
    // the full $defs so nested references resolve. Same compilation shape as
    // the inspection-queries v0.1 vector tests
    // (crates/acquisition/tests/inspection_queries_contract.rs).
    let schema = read_json_at(
        schemas_dir().join("methods"),
        "environment-verify-editor.schema.json",
    );
    let defs = schema
        .get("$defs")
        .expect("method schema has $defs")
        .clone();
    let sub = defs
        .get(format!("environment-verifyEditor{def_suffix}").as_str())
        .expect("sub schema")
        .clone();
    let combined = serde_json::json!({ "allOf": [sub], "$defs": defs });
    jsonschema::validator_for(&combined).expect("sub schema compiles")
}

fn request_validator() -> Validator {
    sub_schema_validator("Request")
}

fn result_validator() -> Validator {
    sub_schema_validator("Result")
}

const POSITIVE_VECTORS: [(&str, &str, &str, &str); 3] = [
    (
        "environment-verify-editor.exe-direct",
        "2022.3.22f1",
        "production_target",
        "vua.env_managers.editor_production_target",
    ),
    (
        "environment-verify-editor.versioned-root",
        "2022.3.22f1c1",
        "other_unity_version",
        "vua.env_managers.editor_china_distribution",
    ),
    (
        "environment-verify-editor.editor-directory",
        "2019.4.31f1",
        "migration_source",
        "vua.env_managers.editor_migration_source",
    ),
];

const REFUSAL_VECTORS: [(&str, &str, bool); 3] = [
    // (stem, refusal code, exePath located?)
    (
        "invalid-environment-verify-editor.target-missing",
        "vua.editor_verify.target_missing",
        false,
    ),
    // Gate 1 counter-example: the directory name claims the production
    // target, but the executable's own identity contradicts it — the path
    // name is never trusted.
    (
        "invalid-environment-verify-editor.not-an-editor",
        "vua.editor_verify.not_an_editor",
        true,
    ),
    (
        "invalid-environment-verify-editor.exe-missing",
        "vua.editor_verify.exe_missing",
        false,
    ),
];

#[test]
fn method_schema_has_request_and_result_defs() {
    let schema = read_json_at(
        schemas_dir().join("methods"),
        "environment-verify-editor.schema.json",
    );
    let defs = schema.get("$defs").expect("$defs");
    assert!(
        defs.get("environment-verifyEditorRequest").is_some(),
        "request def missing"
    );
    assert!(
        defs.get("environment-verifyEditorResult").is_some(),
        "result def missing"
    );
}

#[test]
fn positive_vectors_validate_against_the_method_faces() {
    let request = request_validator();
    let result = result_validator();
    for (stem, _, _, _) in POSITIVE_VECTORS {
        let request_vector = read_json_at(
            schemas_dir().join("examples"),
            &format!("{stem}.request.json"),
        );
        let errors = violations(&request, &request_vector);
        assert!(
            errors.is_empty(),
            "{stem} request must validate: {errors:?}"
        );

        let result_vector = read_json_at(
            schemas_dir().join("examples"),
            &format!("{stem}.result.json"),
        );
        let errors = violations(&result, &result_vector);
        assert!(errors.is_empty(), "{stem} result must validate: {errors:?}");
        assert_eq!(
            result_vector["verdict"], "verified",
            "{stem} must be a verified vector"
        );
        assert_eq!(
            result_vector["schemaVersion"], "0.1",
            "{stem} envelope const"
        );
    }
}

#[test]
fn positive_vectors_match_the_core_classification_authority() {
    // Zero-drift check: every vector's classification/guidanceCode pair is
    // re-derived from the core classifier over the vector's own version
    // string, so the vectors cannot silently diverge from the authority.
    for (stem, version, classification, guidance) in POSITIVE_VECTORS {
        let vector = read_json_at(
            schemas_dir().join("examples"),
            &format!("{stem}.result.json"),
        );
        let parsed = parse_editor_version(version).expect("vector version parses");
        let (class, code) = classify_editor(&parsed);
        let class_face: Value = serde_json::to_value(class).unwrap();
        assert_eq!(
            vector["classification"], class_face,
            "{stem} classification must match the core authority"
        );
        assert_eq!(class_face, classification, "{stem} classification literal");
        assert_eq!(vector["guidanceCode"], guidance, "{stem} guidance literal");
        assert_eq!(code, guidance, "{stem} authority guidance literal");
        assert_eq!(
            vector["chinaDistribution"],
            parsed.china_suffix.is_some(),
            "{stem} china flag must match the parsed identity"
        );
    }
}

#[test]
fn positive_vectors_pin_all_three_normalize_branches() {
    // Ruling point 6: the three input layouts are three independent
    // normalize branches, so each positive vector pins one branch's
    // editorRoot/exePath relationship to the picked path.
    let exe_direct = read_json_at(
        schemas_dir().join("examples"),
        "environment-verify-editor.exe-direct.result.json",
    );
    let root_picked = read_json_at(
        schemas_dir().join("examples"),
        "environment-verify-editor.versioned-root.result.json",
    );
    let editor_dir_picked = read_json_at(
        schemas_dir().join("examples"),
        "environment-verify-editor.editor-directory.result.json",
    );

    // Branch 1: the executable itself — exePath equals the picked path.
    assert_eq!(
        exe_direct["exePath"],
        "C:\\Editors\\2022.3.22f1\\Editor\\Unity.exe"
    );
    assert_eq!(exe_direct["editorRoot"], "C:\\Editors\\2022.3.22f1");

    // Branch 2: the versioned Hub root — exePath is <root>/Editor/Unity.exe.
    assert_eq!(root_picked["editorRoot"], "C:\\Editors\\2022.3.22f1c1");
    assert_eq!(
        root_picked["exePath"],
        "C:\\Editors\\2022.3.22f1c1\\Editor\\Unity.exe"
    );

    // Branch 3: the Editor directory — the root is the versioned parent.
    assert_eq!(editor_dir_picked["editorRoot"], "C:\\Editors\\2019.4.31f1");
    assert_eq!(
        editor_dir_picked["exePath"],
        "C:\\Editors\\2019.4.31f1\\Editor\\Unity.exe"
    );
}

#[test]
fn refusal_vectors_are_valid_results_with_closed_codes() {
    // Nail 1 at the schema face: a refusal validates as a normal result —
    // it must never surface as an application error envelope, so the closed
    // result shape carrying it is itself the contract expression.
    let request = request_validator();
    let result = result_validator();
    for (stem, code, exe_located) in REFUSAL_VECTORS {
        let request_vector = read_json_at(
            schemas_dir().join("examples"),
            &format!("{stem}.request.json"),
        );
        let errors = violations(&request, &request_vector);
        assert!(
            errors.is_empty(),
            "{stem} request must validate: {errors:?}"
        );

        let result_vector = read_json_at(
            schemas_dir().join("examples"),
            &format!("{stem}.result.json"),
        );
        let errors = violations(&result, &result_vector);
        assert!(
            errors.is_empty(),
            "{stem} refusal must validate as a normal result (nail 1): {errors:?}"
        );
        assert_eq!(result_vector["verdict"], "refused", "{stem} verdict");
        assert_eq!(
            result_vector["code"], code,
            "{stem} closed-set code literal"
        );
        assert_eq!(
            result_vector["schemaVersion"], "0.1",
            "{stem} envelope const"
        );
        assert!(
            result_vector["detail"]
                .as_str()
                .map(|d| !d.is_empty())
                .unwrap_or(false),
            "{stem} detail carries the primitive diagnosis verbatim"
        );
        assert_eq!(
            result_vector["exePath"].is_null(),
            !exe_located,
            "{stem} exePath presence semantics"
        );
    }
}

#[test]
fn request_face_is_a_closed_single_key_set() {
    let request = request_validator();

    // minLength 1 (ruling point 3), deliberately without maxLength.
    let empty = serde_json::json!({ "path": "" });
    assert!(
        !request.is_valid(&empty),
        "an empty path is a contract error"
    );

    let missing = serde_json::json!({});
    assert!(
        !request.is_valid(&missing),
        "a missing path is a contract error"
    );

    let extra = serde_json::json!({ "path": "C:\\x", "includeDimensions": true });
    assert!(
        !request.is_valid(&extra),
        "speculative fields are a contract error (closed set)"
    );

    let wrong_type = serde_json::json!({ "path": 42 });
    assert!(!request.is_valid(&wrong_type), "path is a string, verbatim");
}

#[test]
fn verified_face_rejects_foreign_guidance_family_and_unknown_class() {
    let result = result_validator();

    let mut foreign = read_json_at(
        schemas_dir().join("examples"),
        "environment-verify-editor.exe-direct.result.json",
    );
    foreign["guidanceCode"] = serde_json::json!("vua.editor_verify.target_missing");
    assert!(
        !result.is_valid(&foreign),
        "guidanceCode is pinned to the vua.env_managers.* family"
    );

    let mut unknown_class = read_json_at(
        schemas_dir().join("examples"),
        "environment-verify-editor.exe-direct.result.json",
    );
    unknown_class["classification"] = serde_json::json!("beta_channel");
    assert!(
        !result.is_valid(&unknown_class),
        "classification is the four-value closed set shared with environment-managers v0.1"
    );
}

#[test]
fn refusal_face_rejects_missing_absence_code_overload() {
    // Ruling point 5: vua.environment.verify_unavailable is reserved for the
    // unwired route / unreachable primitive only — it is not part of the
    // refusal closed set and must never be reusable as a verification
    // refusal.
    let result = result_validator();
    let mut overloaded = read_json_at(
        schemas_dir().join("examples"),
        "invalid-environment-verify-editor.target-missing.result.json",
    );
    overloaded["code"] = serde_json::json!("vua.environment.verify_unavailable");
    assert!(
        !result.is_valid(&overloaded),
        "the honest absence code must not overload the refusal closed set"
    );
}
