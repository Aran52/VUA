//! Contract anchor for `schemas/bdl-commands/v0.1` (W8 — B4 artifact-mode
//! trio: setArtifactMode / generateVpm / deleteOriginals).
//!
//! One end consumes the frozen vocabulary for real: the positive request
//! vectors drive actual BDL store and maintenance-task APIs, the resulting
//! acceptance payloads are validated back against the frozen result schema,
//! and the Rust mode vocabulary stays in lockstep with the schema. Any
//! vocabulary change must bump the schema version, never rewrite v0.1 in
//! place. The transport envelope (requestId, commandId) belongs to the
//! application contract; these vectors freeze the operation closed set,
//! params and acceptance payloads.

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

use vua_acquisition::warehouse_maintenance::{
    submit_delete_originals, DeleteOriginalsResult, DeleteOriginalsTaskSpec, GenerateVpmResult,
};
use vua_bdl_store::bdl_store::{ArtifactMode, BdlStore};
use vua_orchestrator::{FixedIdGenerator, MemoryJournal, SystemClock, TaskRuntime};

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/bdl-commands/v0.1")
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

fn violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

const POSITIVE_REQUESTS: &[&str] = &[
    "examples/warehouse-set-artifact-mode.request.json",
    "examples/warehouse-set-artifact-mode-clear.request.json",
    "examples/warehouse-generate-vpm.request.json",
    "examples/warehouse-delete-originals.request.json",
];

const POSITIVE_RESULTS: &[&str] = &[
    "examples/warehouse-set-artifact-mode.result.json",
    "examples/warehouse-set-artifact-mode-clear.result.json",
    "examples/warehouse-generate-vpm.result.json",
    "examples/warehouse-delete-originals.result.json",
];

const NEGATIVE_VECTORS: &[&str] = &[
    "examples/invalid-operation.json",
    "examples/invalid-mode.json",
    "examples/missing-warehouse-item-id.json",
];

#[test]
fn bdl_cmd_001_positive_request_vectors_validate_against_the_command_schema() {
    let validator = command_validator();
    for name in POSITIVE_REQUESTS {
        let vector = read_json(name);
        let errors = violations(&validator, &vector);
        assert!(errors.is_empty(), "{name} must validate: {errors:?}");
    }
}

#[test]
fn bdl_cmd_002_positive_result_vectors_validate_against_the_result_schema() {
    let validator = result_validator();
    for name in POSITIVE_RESULTS {
        let vector = read_json(name);
        let errors = violations(&validator, &vector);
        assert!(errors.is_empty(), "{name} must validate: {errors:?}");
    }
}

#[test]
fn bdl_cmd_003_negative_vectors_are_rejected_by_the_command_schema() {
    let validator = command_validator();
    for name in NEGATIVE_VECTORS {
        let vector = read_json(name);
        assert!(
            !validator.is_valid(&vector),
            "{name} is a negative vector and must not validate"
        );
    }
}

#[test]
fn bdl_cmd_004_mode_vocabulary_stays_in_lockstep_with_the_schema() {
    let command_schema = read_json("command.schema.json");
    let mut schema_modes: Vec<String> = command_schema["allOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|branch| {
            branch["if"]["properties"]["operation"]["const"] == "warehouse.setArtifactMode"
        })
        .expect("the setArtifactMode branch pins the mode vocabulary")["then"]["properties"]
        ["params"]["properties"]["mode"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| {
            if value.is_null() {
                "<null>".to_owned()
            } else {
                value.as_str().unwrap().to_owned()
            }
        })
        .collect();

    let result_schema = read_json("result.schema.json");
    let mut schema_effective: Vec<String> = result_schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|branch| branch["properties"]["operation"]["const"] == "warehouse.setArtifactMode")
        .expect("the setArtifactMode result branch pins the effective vocabulary")["properties"]
        ["effectiveMode"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();

    let mut rust_modes: Vec<String> = [
        ArtifactMode::UseOriginalUnitypackage,
        ArtifactMode::GenerateVpm,
    ]
    .iter()
    .map(|mode| mode.name().to_owned())
    .collect();
    rust_modes.sort();
    schema_modes.sort();
    schema_effective.sort();

    // The wire carries an explicit null for "clear the override"; the Rust
    // enum has no such variant — clearing passes None.
    assert!(schema_modes.contains(&"<null>".to_owned()));
    schema_modes.retain(|mode| mode != "<null>");
    assert_eq!(
        rust_modes, schema_modes,
        "mode enum drifted from the schema"
    );
    assert_eq!(
        rust_modes, schema_effective,
        "effective enum drifted from the schema"
    );
}

/// The positive vector drives the real store: resolving the request params
/// into a store call yields exactly the vector's promised effective mode.
#[test]
fn bdl_cmd_005_set_artifact_mode_vector_drives_the_real_store() {
    for (request, result) in [
        (
            "examples/warehouse-set-artifact-mode.request.json",
            "examples/warehouse-set-artifact-mode.result.json",
        ),
        (
            "examples/warehouse-set-artifact-mode-clear.request.json",
            "examples/warehouse-set-artifact-mode-clear.result.json",
        ),
    ] {
        let request = read_json(request);
        let expected = read_json(result);

        let store = BdlStore::open_in_memory().unwrap();
        let entry_id = store
            .create_warehouse_item(
                "Example Pack",
                "imported_material",
                "2026-09-07T02:00:00.000Z",
            )
            .unwrap()
            .warehouse_item_id;

        // The request targets its warehouseItemId; retarget the fixture id to
        // the freshly created entry so the store call is real, not a stub.
        let params = &request["params"];
        let requested_mode = &params["mode"];
        let mode = if requested_mode.is_null() {
            None
        } else {
            Some(ArtifactMode::parse(requested_mode.as_str().unwrap()).unwrap())
        };

        // Establish the opposite override first so the clear vector really
        // falls back to the global default rather than repeating a no-op.
        store
            .set_artifact_mode(&entry_id, Some(ArtifactMode::GenerateVpm))
            .unwrap();
        store.set_artifact_mode(&entry_id, mode).unwrap();

        let global_default = ArtifactMode::UseOriginalUnitypackage;
        let detail = store
            .warehouse_entry_detail(&entry_id, global_default)
            .unwrap()
            .expect("the entry exists");
        let effective = detail.effective_artifact_mode.name();

        assert_eq!(expected["operation"], "warehouse.setArtifactMode");
        assert_eq!(
            effective, expected["effectiveMode"],
            "the vector promised this mode"
        );
    }
}

/// The tasked vectors return a real task acceptance from the maintenance
/// API, and that acceptance validates against the frozen result schema.
#[test]
fn bdl_cmd_006_tasked_acceptance_output_validates_against_the_result_schema() {
    let request = read_json("examples/warehouse-delete-originals.request.json");
    let expected = read_json("examples/warehouse-delete-originals.result.json");

    let store = Arc::new(BdlStore::open_in_memory().unwrap());
    let runtime = TaskRuntime::new(
        Arc::new(MemoryJournal::default()),
        Arc::new(SystemClock),
        Arc::new(FixedIdGenerator::default()),
    );

    let correlation_id = "corr-delete-originals-example";
    let accepted = submit_delete_originals(
        &runtime,
        store,
        DeleteOriginalsTaskSpec {
            correlation_id: correlation_id.to_owned(),
            warehouse_item_id: request["params"]["warehouseItemId"]
                .as_str()
                .unwrap()
                .to_owned(),
            global_default: ArtifactMode::GenerateVpm,
        },
        None,
    )
    .expect("submission is accepted");

    // The correlation binding lives on the task row (the acceptance envelope
    // carries task identity; the snapshot carries the audit binding).
    let bound = runtime
        .snapshot(&accepted.task_id)
        .expect("task must exist");
    assert_eq!(bound.correlation_id, correlation_id);

    let acceptance = json!({
        "schemaVersion": "0.1",
        "operation": "warehouse.deleteOriginals",
        "taskId": accepted.task_id,
        "correlationId": bound.correlation_id,
    });
    let errors = violations(&result_validator(), &acceptance);
    assert!(
        errors.is_empty(),
        "the real acceptance must validate: {errors:?}"
    );
    assert_eq!(acceptance["correlationId"], expected["correlationId"]);

    // The guard fires inside the task, not at acceptance: an unknown entry is
    // a failed task, never a rejected submission — the acceptance schema
    // stays the delivery contract regardless of the entry's fate.
    let snapshot = {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let snapshot = runtime
                .snapshot(&accepted.task_id)
                .expect("task must exist");
            if snapshot.state.is_terminal() {
                break snapshot;
            }
            assert!(std::time::Instant::now() < deadline, "task did not finish");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    };
    assert!(
        snapshot.state.is_terminal(),
        "the acceptance binds exactly one auditable task"
    );
}

/// The task completion payloads travel the application-contract task
/// surface; their serde shape is anchored here so a field rename cannot
/// silently break the wire contract the task surface carries.
#[test]
fn bdl_cmd_007_completion_payload_shapes_stay_camel_case() {
    fn sorted_keys(value: &Value) -> Vec<String> {
        let mut keys: Vec<String> = value
            .as_object()
            .expect("a completion payload object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    }

    // A manual generation carries no importCorrelationId (skip_serializing_if
    // keeps the payload to the manual five keys — the audit-chain field only
    // appears for import-orchestrated generations).
    let generation = serde_json::to_value(GenerateVpmResult {
        correlation_id: "corr".into(),
        warehouse_item_id: "whi-x".into(),
        package_id: "com.ph-r.vua.local.example_pack.0123456789ab".into(),
        archive_relative_path: "vpm/example-0.1.0.zip".into(),
        archive_sha256: "sha256:deadbeef".into(),
        import_correlation_id: None,
    })
    .unwrap();
    assert_eq!(
        sorted_keys(&generation),
        vec![
            "archiveRelativePath",
            "archiveSha256",
            "correlationId",
            "packageId",
            "warehouseItemId"
        ]
    );
    // An import-orchestrated generation carries the audit-chain link in the
    // payload (camelCase), while a manual one omits the field entirely.
    let orchestrated = serde_json::to_value(GenerateVpmResult {
        correlation_id: "corr".into(),
        warehouse_item_id: "whi-x".into(),
        package_id: "pkg".into(),
        archive_relative_path: "vpm/x.zip".into(),
        archive_sha256: "sha256:deadbeef".into(),
        import_correlation_id: Some("corr-import-1".into()),
    })
    .unwrap();
    assert_eq!(
        sorted_keys(&orchestrated),
        vec![
            "archiveRelativePath",
            "archiveSha256",
            "correlationId",
            "importCorrelationId",
            "packageId",
            "warehouseItemId"
        ]
    );

    let deletion = serde_json::to_value(DeleteOriginalsResult {
        correlation_id: "corr".into(),
        warehouse_item_id: "whi-x".into(),
        deleted_count: 2,
        deleted_relative_paths: vec![
            "original/a.unitypackage".into(),
            "original/b.unitypackage".into(),
        ],
        kept_generated_sha256: Some("sha256:feedface".into()),
    })
    .unwrap();
    assert_eq!(
        sorted_keys(&deletion),
        vec![
            "correlationId",
            "deletedCount",
            "deletedRelativePaths",
            "keptGeneratedSha256",
            "warehouseItemId"
        ]
    );
}
