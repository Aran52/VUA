//! W14 consumer tests: bdl-commands v0.2 — the U8-ruled global-default
//! write (`warehouse.setGlobalDefaultMode`) over the frozen v0.2 vocabulary.
//!
//! One end consumes the frozen vectors for real: the positive request vector
//! drives the persisted global default, the read-back validates inside the
//! frozen result envelope, and the two-level resolution (entry override ??
//! global default) is pinned end to end. The persistence-location decision
//! is exercised as shipped: bdl_meta in the store's own database — a
//! reopened store keeps the persisted default, and the provider's
//! environment-injected initial default only rules before the first write.

use serde_json::{json, Value};
use std::path::PathBuf;
use vua_bdl_store::{
    ArtifactMode, BdlStore, BdlStoreError, CatalogListParams, CatalogListResult,
    CatalogProductDetail, CatalogRevision, CatalogStatusResult,
};

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/bdl-commands/v0.2")
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
fn w14_positive_vectors_validate_against_both_frozen_schemas() {
    let command_validator = command_validator();
    let result_validator = result_validator();
    for name in [
        "examples/warehouse-set-artifact-mode.request.json",
        "examples/warehouse-set-artifact-mode-clear.request.json",
        "examples/warehouse-set-global-default.request.json",
        "examples/warehouse-generate-vpm.request.json",
        "examples/warehouse-delete-originals.request.json",
    ] {
        let vector = read_json(name);
        assert!(command_validator.is_valid(&vector), "{name} must validate");
    }
    for name in [
        "examples/warehouse-set-artifact-mode.result.json",
        "examples/warehouse-set-artifact-mode-clear.result.json",
        "examples/warehouse-set-global-default.result.json",
        "examples/warehouse-generate-vpm.result.json",
        "examples/warehouse-delete-originals.result.json",
    ] {
        let vector = read_json(name);
        assert!(result_validator.is_valid(&vector), "{name} must validate");
    }
}

#[test]
fn w14_negative_vectors_are_rejected() {
    let command_validator = command_validator();
    for name in [
        "examples/invalid-operation.json",
        "examples/invalid-mode.json",
        "examples/invalid-global-default-mode.json",
        "examples/missing-warehouse-item-id.json",
    ] {
        let vector = read_json(name);
        assert!(
            !command_validator.is_valid(&vector),
            "{name} is a negative vector and must not validate"
        );
    }
}

#[test]
fn w14_global_default_vector_drives_the_persisted_two_level_resolution() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let database_path =
        std::env::temp_dir().join(format!("vua-w14-{}-{nanos}", std::process::id()));
    let store = BdlStore::open(&database_path).unwrap();

    // Before any write: not persisted — the provider's environment-injected
    // initial default rules (represented here by the query argument).
    assert_eq!(store.global_default_mode().unwrap(), None);
    let entry_id = store
        .create_warehouse_item(
            "Example Pack",
            "imported_material",
            "2026-09-07T07:00:00.000Z",
        )
        .unwrap()
        .warehouse_item_id;
    // The provider composes the effective global default: the persisted
    // value when one exists, otherwise its environment-injected initial
    // default. Entry resolution stays `override ?? composed global`.
    let env_initial = ArtifactMode::UseOriginalUnitypackage;
    let effective = |env_initial: ArtifactMode| {
        let global = store.global_default_mode().unwrap().unwrap_or(env_initial);
        store
            .warehouse_entry_detail(&entry_id, global)
            .unwrap()
            .unwrap()
            .effective_artifact_mode
    };
    assert_eq!(
        effective(env_initial),
        ArtifactMode::UseOriginalUnitypackage
    );

    // The frozen positive vector drives the real write; the read-back is the
    // stored fact, and the two-level resolution follows it for entries
    // without an override.
    let request = read_json("examples/warehouse-set-global-default.request.json");
    let written = store
        .set_global_default_mode(
            ArtifactMode::parse(request["params"]["mode"].as_str().unwrap()).unwrap(),
        )
        .unwrap();
    assert_eq!(written, ArtifactMode::GenerateVpm);
    assert_eq!(
        store.global_default_mode().unwrap(),
        Some(ArtifactMode::GenerateVpm)
    );
    assert_eq!(
        effective(env_initial),
        ArtifactMode::GenerateVpm,
        "entries without an override follow the new global default"
    );

    // The entry level still wins: the two options are independent levels.
    store
        .set_artifact_mode(&entry_id, Some(ArtifactMode::UseOriginalUnitypackage))
        .unwrap();
    assert_eq!(
        effective(ArtifactMode::GenerateVpm),
        ArtifactMode::UseOriginalUnitypackage,
        "the entry override rules over the persisted global default"
    );

    // The persisted default survives a reopened store.
    drop(store);
    let reopened = BdlStore::open(&database_path).unwrap();
    assert_eq!(
        reopened.global_default_mode().unwrap(),
        Some(ArtifactMode::GenerateVpm)
    );
    let _ = std::fs::remove_file(&database_path);
}

#[test]
fn w14_acceptance_shape_validates_inside_the_frozen_envelope() {
    // The setGlobalDefaultMode answer, as the provider face would deliver it:
    // the frozen result payload inside the { schemaVersion, operation }
    // envelope, with the value read back from the store — never echoed.
    let store = BdlStore::open_in_memory().unwrap();
    let written = store
        .set_global_default_mode(ArtifactMode::GenerateVpm)
        .unwrap();
    let read_back = store.global_default_mode().unwrap().unwrap();
    let acceptance = json!({
        "schemaVersion": BDL_QUERIES_SCHEMA_VERSION_BDL_COMMANDS_V02,
        "operation": "warehouse.setGlobalDefaultMode",
        "globalDefaultMode": written.name(),
    });
    assert!(result_validator().is_valid(&acceptance), "{acceptance}");
    assert_eq!(acceptance["globalDefaultMode"], read_back.name());
}

/// The v0.2 envelope constant the provider face composes (mirrored here so
/// a silent change breaks loudly).
const BDL_QUERIES_SCHEMA_VERSION_BDL_COMMANDS_V02: &str = "0.2";

#[test]
fn w14_type_anchors() {
    // The catalog/warehouse anchors live in the same crate; this test pins
    // that the v0.2 work did not disturb them.
    let _: Option<CatalogListResult> = None;
    let _: Option<CatalogProductDetail> = None;
    let _: Option<CatalogRevision> = None;
    let _: Option<CatalogStatusResult> = None;
    let _: Option<CatalogListParams> = None;
    let _: Option<BdlStoreError> = None;
}
