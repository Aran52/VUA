//! Proposal 029 B-face freeze consumer tests: the recipe-export v0.1
//! word-row (`schemas/recipe-export/v0.1/`) — the project→draft export face
//! `recipe.exportProjectDraft` (core freeze batch 2026-09-22).
//!
//! The frozen vectors drive a real JSON-Schema validation on this end:
//! positive vectors validate, negative vectors are rejected, and the draft
//! semantics pinned here are the ones the desktop confirmation flow
//! consumes (029 inline thread, wt-3 shape verdict sections ①②③) — the
//! draft is NEVER a Recipe (no recipeId, no title, no relation face), the
//! missing list is the honesty core, and the environmentUnityVersion
//! marker is iff-bound to a null constraint. The zero-bridge ruling
//! (029 open item 4, option B) is pinned by shape: a draft carrying the
//! relation face is invalid by construction.

use jsonschema::Validator;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/recipe-export/v0.1")
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

// --- typed face of the draft document (serde-level closed set) ---

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ProjectDraftV01 {
    schema_version: String,
    draft_id: String,
    exported_at: String,
    origin: DraftOriginV01,
    environment: DraftEnvironmentV01,
    dependencies: Vec<DraftDependencyV01>,
    missing: Vec<MissingDimensionV01>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DraftOriginV01 {
    project_path: String,
    project_name: Option<String>,
    vua_identity_status: VuaIdentityStatusV01,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum VuaIdentityStatusV01 {
    Absent,
    Present,
    Unreadable,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DraftEnvironmentV01 {
    unity_version_constraint: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DraftDependencyV01 {
    package_id: String,
    version_constraint: String,
    locked_version: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum MissingDimensionV01 {
    Assets,
    Instances,
    Relations,
    WardrobeGroups,
    TargetAvatar,
    AssetRoles,
    AssetLabels,
    SourceRefs,
    TitleSemantics,
    EnvironmentUnityVersion,
}

fn result_of(vector: &Value) -> &Value {
    vector
        .get("result")
        .expect("result envelope carries result")
}

#[test]
fn positive_vectors_validate_against_the_frozen_schemas() {
    let command_validator = validator_for(&read_json("command.schema.json"));
    let result_validator = validator_for(&read_json("result.schema.json"));

    let request = read_json("examples/recipe-export-project-draft.request.json");
    let problems = violations(&command_validator, &request);
    assert!(problems.is_empty(), "request must validate: {problems:?}");

    for name in [
        "recipe-export-project-draft.result.json",
        "recipe-export-project-draft.non-vua.result.json",
        "recipe-export-project-draft.unity-unreadable.result.json",
        "recipe-export-project-draft.empty-dependencies.result.json",
    ] {
        let vector = read_json(&format!("examples/{name}"));
        let problems = violations(&result_validator, &vector);
        assert!(problems.is_empty(), "{name} must validate: {problems:?}");
    }
}

#[test]
fn negative_vectors_are_rejected() {
    let command_validator = validator_for(&read_json("command.schema.json"));
    let result_validator = validator_for(&read_json("result.schema.json"));

    let extra_param = read_json("examples/invalid-request-extra-param.json");
    assert!(
        !command_validator.is_valid(&extra_param),
        "params closed set: extra key must not validate"
    );
    let empty_path = read_json("examples/invalid-request-empty-project-path.json");
    assert!(
        !command_validator.is_valid(&empty_path),
        "empty projectPath must not validate"
    );

    for name in [
        "invalid-result-invented-dependency-field.result.json",
        "invalid-result-missing-constant-dimension.result.json",
        "invalid-result-unknown-missing-dimension.result.json",
        "invalid-result-unity-null-without-marker.result.json",
        "invalid-result-marker-without-unity-null.result.json",
        "invalid-result-relation-face-present.result.json",
    ] {
        let vector = read_json(&format!("examples/{name}"));
        assert!(
            !result_validator.is_valid(&vector),
            "{name} must be rejected by the frozen schema"
        );
    }
}

#[test]
fn the_word_row_pins_its_identity_and_closed_sets() {
    let command = read_json("command.schema.json");
    assert_eq!(command["properties"]["schemaVersion"]["const"], "0.1");
    assert_eq!(
        command["properties"]["operation"]["enum"][0],
        "recipe.exportProjectDraft"
    );
    assert_eq!(
        command["properties"]["params"]["additionalProperties"],
        Value::Bool(false)
    );
    assert_eq!(
        command["properties"]["params"]["required"][0],
        "projectPath"
    );

    let result = read_json("result.schema.json");
    assert_eq!(result["properties"]["schemaVersion"]["const"], "0.1");
    assert_eq!(
        result["properties"]["operation"]["enum"][0],
        "recipe.exportProjectDraft"
    );

    let draft = &result["$defs"]["projectDraft"];
    // Family const of its own (c914cf2 standing rule).
    assert_eq!(
        draft["properties"]["schemaVersion"]["const"],
        "vua.recipe-export/v0.1"
    );
    // A draft never carries the Recipe face: no recipeId, no title, no
    // relation face, no locked block (zero-bridge ruling + draft ruling).
    for forbidden in [
        "recipeId",
        "title",
        "assets",
        "instances",
        "relations",
        "wardrobeGroups",
        "locked",
    ] {
        assert!(
            draft["properties"].get(forbidden).is_none(),
            "draft must not carry {forbidden}"
        );
    }
    // The missing list: closed ten-value enum, nine constant contains pins,
    // and exactly two iff implications binding environmentUnityVersion to a
    // null constraint.
    let missing = &draft["properties"]["missing"];
    let enum_values = result["$defs"]["missingDimension"]["enum"]
        .as_array()
        .expect("closed enum");
    assert_eq!(enum_values.len(), 10);
    assert_eq!(missing["allOf"].as_array().expect("contains pins").len(), 9);
    let implications = draft["allOf"].as_array().expect("iff implications");
    assert_eq!(implications.len(), 2);
    for implication in implications {
        assert!(implication.get("if").is_some() && implication.get("then").is_some());
    }
}

#[test]
fn typed_draft_consumes_every_positive_result() {
    let full = read_json("examples/recipe-export-project-draft.result.json");
    let draft: ProjectDraftV01 =
        serde_json::from_value(result_of(&full).clone()).expect("full draft must deserialize");
    assert_eq!(draft.schema_version, "vua.recipe-export/v0.1");
    assert_eq!(
        draft.origin.vua_identity_status,
        VuaIdentityStatusV01::Present
    );
    assert_eq!(
        draft.environment.unity_version_constraint.as_deref(),
        Some("2022.3.22f1")
    );
    assert_eq!(draft.dependencies.len(), 3);
    assert_eq!(
        draft.dependencies[0].locked_version.as_deref(),
        Some("3.7.12")
    );
    assert_eq!(draft.dependencies[1].locked_version, Option::<String>::None);
    assert_eq!(draft.missing.len(), 9);
    assert!(!draft
        .missing
        .contains(&MissingDimensionV01::EnvironmentUnityVersion));

    let non_vua = read_json("examples/recipe-export-project-draft.non-vua.result.json");
    let draft: ProjectDraftV01 = serde_json::from_value(result_of(&non_vua).clone())
        .expect("non-VUA draft must deserialize");
    assert_eq!(
        draft.origin.vua_identity_status,
        VuaIdentityStatusV01::Absent
    );
    assert_eq!(draft.origin.project_name, None);

    let unreadable = read_json("examples/recipe-export-project-draft.unity-unreadable.result.json");
    let draft: ProjectDraftV01 = serde_json::from_value(result_of(&unreadable).clone())
        .expect("unreadable-unity draft must deserialize");
    assert_eq!(draft.environment.unity_version_constraint, None);
    assert!(draft
        .missing
        .contains(&MissingDimensionV01::EnvironmentUnityVersion));
    assert_eq!(draft.missing.len(), 10);

    let empty = read_json("examples/recipe-export-project-draft.empty-dependencies.result.json");
    let draft: ProjectDraftV01 =
        serde_json::from_value(result_of(&empty).clone()).expect("empty draft must deserialize");
    assert!(draft.dependencies.is_empty(), "honest empty dependencies");
}

#[test]
fn serde_face_rejects_the_invented_shape_negatives() {
    // The invented `optional` flag on a dependency row: schema-invalid AND
    // serde-invalid (deny_unknown_fields) — two carriers, one verdict.
    let invented = read_json("examples/invalid-result-invented-dependency-field.result.json");
    assert!(serde_json::from_value::<ProjectDraftV01>(result_of(&invented).clone()).is_err());
    // A draft claiming the relation face (recipeId/title/assets/instances):
    // invalid on both carriers — the draft/Recipe boundary is a type fact.
    let relation_face = read_json("examples/invalid-result-relation-face-present.result.json");
    assert!(serde_json::from_value::<ProjectDraftV01>(result_of(&relation_face).clone()).is_err());
}

#[test]
fn dependency_order_is_a_frozen_presentation_fact() {
    let full = read_json("examples/recipe-export-project-draft.result.json");
    let draft: ProjectDraftV01 =
        serde_json::from_value(result_of(&full).clone()).expect("full draft must deserialize");
    let ids: Vec<&str> = draft
        .dependencies
        .iter()
        .map(|row| row.package_id.as_str())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "packageId ascending is frozen presentation");
}
