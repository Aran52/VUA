//! Contract anchor for `schemas/bdl-commands/v0.3` (M5 batch import —
//! proposal 010, W19): the `warehouse.import` vector drives the real
//! batch-import task, the acceptance payload validates back against the
//! frozen result schema, and the negative vectors stay rejected.
//!
//! The four pre-existing commands travel v0.3 unchanged (their semantics are
//! anchored by `bdl_commands_contract.rs` on the v0.1 face and by the
//! provider-host v0.2 vectors); this file anchors the new import face and the
//! v0.3 closed set. The transport envelope (requestId, commandId) belongs to
//! the application contract.

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

use vua_acquisition::warehouse_import::{submit_warehouse_import, WarehouseImportTaskSpec};
use vua_bdl_store::BdlStore;
use vua_orchestrator::{FixedIdGenerator, MemoryJournal, SystemClock, TaskRuntime};

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/bdl-commands/v0.3")
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

const IMPORT_REQUEST: &str = "examples/warehouse-import.request.json";
const IMPORT_RESULT: &str = "examples/warehouse-import.result.json";
const IMPORT_NEGATIVES: &[&str] = &[
    "examples/invalid-import-empty-folders.json",
    "examples/invalid-import-folders-type.json",
    "examples/invalid-operation.json",
];

fn unique_dir(tag: &str) -> PathBuf {
    // Board #7 hardening: a process-unique serial (plus pid) keeps parallel
    // tests from sharing a same-tick timestamp directory whose teardown
    // would delete another test's files.
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-import-v03-{tag}-pid{}-t{nanos:016x}-{serial}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Two source folders, each holding one material package file — the minimal
/// honest fixture for a two-folder batch.
fn source_folders(tag: &str) -> Vec<PathBuf> {
    let parent = unique_dir(tag);
    ["uniform-pack", "hair-pack"]
        .iter()
        .map(|name| {
            let folder = parent.join(name);
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join("pack.unitypackage"), b"PK unity fixture").unwrap();
            folder
        })
        .collect()
}

#[test]
fn import_vector_validates_against_the_command_schema() {
    let vector = read_json(IMPORT_REQUEST);
    let errors: Vec<String> = command_validator()
        .iter_errors(&vector)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "the import vector must validate: {errors:?}"
    );
}

#[test]
fn import_result_vector_validates_against_the_result_schema() {
    let vector = read_json(IMPORT_RESULT);
    let errors: Vec<String> = result_validator()
        .iter_errors(&vector)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "the import result must validate: {errors:?}"
    );
}

#[test]
fn import_negative_vectors_are_rejected_by_the_command_schema() {
    let validator = command_validator();
    for name in IMPORT_NEGATIVES {
        let vector = read_json(name);
        assert!(
            !validator.is_valid(&vector),
            "{name} is a negative vector and must not validate"
        );
    }
}

#[test]
fn operation_closed_set_is_exactly_the_five_commands() {
    let command_schema = read_json("command.schema.json");
    let operations: Vec<String> = command_schema["properties"]["operation"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        operations,
        vec![
            "warehouse.setArtifactMode",
            "warehouse.generateVpm",
            "warehouse.deleteOriginals",
            "warehouse.setGlobalDefaultMode",
            "warehouse.import",
        ]
    );
}

/// The v0.3 generateVpm params accept the optional audit-chain field
/// (proposal 010 commitment 6) and stay valid without it — a manually
/// initiated generation never carries it.
#[test]
fn generate_vpm_import_correlation_id_is_optional_but_frozen() {
    let validator = command_validator();
    let mut orchestrated = read_json("examples/warehouse-generate-vpm.request.json");
    orchestrated["schemaVersion"] = json!("0.3");
    orchestrated["params"]["importCorrelationId"] = json!("corr-import-1");
    assert!(
        validator.is_valid(&orchestrated),
        "the import-orchestrated generation must carry importCorrelationId"
    );
    let mut manual = read_json("examples/warehouse-generate-vpm.request.json");
    manual["schemaVersion"] = json!("0.3");
    assert!(
        validator.is_valid(&manual),
        "a manually initiated generation must not need the field"
    );
    // The field is frozen, not free-form: an unknown key stays a contract error.
    manual["params"]["unknownKey"] = json!("x");
    assert!(!validator.is_valid(&manual));
}

/// The import vector drives the real batch-import task: retargeting the
/// vector's folders at honest fixtures yields a real task acceptance that
/// validates against the frozen result schema, and the Done payload reports
/// both folders with created entries in the store.
#[test]
fn import_vector_drives_the_real_batch_import_and_validates() {
    let request = read_json(IMPORT_REQUEST);
    let folders = source_folders("batch");

    // Retarget the vector at the fixture folders; the shape (a two-folder
    // batch) stays the vector's.
    let retargeted = json!({
        "schemaVersion": "0.3",
        "operation": "warehouse.import",
        "params": {
            "sourceFolders": folders
                .iter()
                .map(|folder| folder.to_string_lossy().into_owned())
                .collect::<Vec<String>>(),
        }
    });
    let errors: Vec<String> = command_validator()
        .iter_errors(&retargeted)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "the retargeted vector must validate: {errors:?}"
    );

    let store = Arc::new(BdlStore::open_in_memory().unwrap());
    let runtime = TaskRuntime::new(
        Arc::new(MemoryJournal::default()),
        Arc::new(SystemClock),
        Arc::new(FixedIdGenerator::default()),
    );

    let correlation_id = request["params"]
        .get("sourceFolders")
        .map(|_| "corr-import-1".to_owned())
        .unwrap();
    let accepted = submit_warehouse_import(
        &runtime,
        store.clone(),
        Arc::new(vua_orchestrator::SystemClock),
        WarehouseImportTaskSpec {
            correlation_id: correlation_id.clone(),
            source_folders: folders.clone(),
            warehouse_root: unique_dir("wh"),
            auto_generate: None,
        },
        None,
    )
    .expect("submission is accepted");

    // The acceptance assembles into the frozen v0.3 import result shape.
    let acceptance = json!({
        "schemaVersion": "0.3",
        "operation": "warehouse.import",
        "taskId": accepted.task_id,
        "correlationId": correlation_id,
    });
    let errors: Vec<String> = result_validator()
        .iter_errors(&acceptance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "the real acceptance must validate: {errors:?}"
    );
    let expected = read_json(IMPORT_RESULT);
    assert_eq!(acceptance["operation"], expected["operation"]);

    // The batch finishes terminal with both folders imported; the durable
    // effect is asserted through the store (stronger than the payload): one
    // entry per folder, exactly as the vector's batch promised.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let snapshot = loop {
        let snapshot = runtime
            .snapshot(&accepted.task_id)
            .expect("task must exist");
        if snapshot.state.is_terminal() {
            break snapshot;
        }
        assert!(std::time::Instant::now() < deadline, "task did not finish");
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    assert_eq!(snapshot.correlation_id, correlation_id);
    let cards = store
        .warehouse_entry_cards(vua_bdl_store::ArtifactMode::GenerateVpm)
        .unwrap();
    assert_eq!(cards.len(), 2, "the batch created one entry per folder");
}
