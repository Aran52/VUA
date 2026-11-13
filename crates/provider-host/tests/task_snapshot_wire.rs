//! Task-snapshot wire tests (BOARD #22 result-reflux increment,
//! core proposal 020): the task face carries the Done payload on the
//! snapshot channel — `task.get` / `task.list` snapshots project the single
//! stored task result with the same value the `task.completed` event
//! publishes, closing the cross-batch seam where the renderer could see the
//! acceptance receipt but never the result document. The consumer consumes
//! the frozen snapshot schema from `schemas/application-contract/v0.1`;
//! changing the snapshot face without this consumer fails here first. The
//! vectors (3 valid + 3 invalid) drive JSON-Schema validation directly.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use vua_orchestrator::{AppErrorV1, ErrorCategory, NewTask, SqliteTaskStore, TaskMutation};
use vua_project_manager::ManagerRoots;
use vua_provider_host::{run_provider_host_full, ProjectOpsConfig};

fn snapshot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/application-contract/v0.1")
}

fn snapshot_validator() -> jsonschema::Validator {
    let bytes =
        fs::read(snapshot_dir().join("task-snapshot.schema.json")).expect("snapshot schema exists");
    let schema: Value = serde_json::from_slice(&bytes).expect("snapshot schema is valid JSON");
    jsonschema::validator_for(&schema).expect("frozen snapshot schema must compile")
}

/// A unique per-test root (temp dir + label + pid + nanos).
fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-task-snapshot-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// Seeds a minimal real Unity project (Assets / Packages + manifest /
/// ProjectSettings) plus a VCC settings.json that registers it. Returns
/// (source path, target parent, vcc settings path).
fn seed_registered_source(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let source = root.join("source-project");
    fs::create_dir_all(source.join("Assets")).expect("Assets dir");
    fs::create_dir_all(source.join("ProjectSettings")).expect("ProjectSettings dir");
    fs::create_dir_all(source.join("Packages")).expect("Packages dir");
    fs::write(source.join("Assets").join("avatar.fbx"), b"fb fixture").expect("asset file");
    fs::write(
        source.join("ProjectSettings").join("ProjectSettings.asset"),
        "productName: Source Project",
    )
    .expect("project settings");
    fs::write(
        source.join("ProjectSettings").join("ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1",
    )
    .expect("project version");
    fs::write(
        source.join("Packages").join("manifest.json"),
        r#"{"dependencies": {"com.unity.render-pipelines.universal": "14.0.0"}}"#,
    )
    .expect("manifest");
    fs::write(
        source.join("Packages").join("vpm-manifest.json"),
        r#"{"dependencies": {}}"#,
    )
    .expect("vpm manifest");

    let vcc_settings = root.join("vcc-settings.json");
    fs::write(
        &vcc_settings,
        json!({"userProjects": [source.to_string_lossy()]}).to_string(),
    )
    .expect("vcc settings");

    let target_parent = root.join("targets");
    fs::create_dir_all(&target_parent).expect("target parent");
    (source, target_parent, vcc_settings)
}

fn project_ops_config(vcc_settings: &Path) -> ProjectOpsConfig {
    ProjectOpsConfig {
        vcc_settings_candidates: vec![vcc_settings.to_path_buf()],
        manager_roots: ManagerRoots {
            alcom_settings_candidates: Vec::new(),
        },
        editor_roots: Vec::new(),
    }
}

/// Runs one frame through the real host loop. `kind` selects the wire kind
/// ("query" for task.get, "command" for tasked submissions and
/// task.requestCancellation, which also carries a commandId).
fn run_frame(
    database_path: &Path,
    config: Option<&ProjectOpsConfig>,
    kind: &str,
    method: &str,
    extra: Value,
) -> Vec<Value> {
    let mut payload = json!({
        "contractVersion": "0.1",
        "requestId": "req-0",
        "correlationId": "corr-0",
        "kind": kind,
        "method": method,
    });
    let object = payload.as_object_mut().expect("payload is an object");
    if kind == "command" {
        object.insert("commandId".into(), json!(format!("cmd-{method}")));
    }
    if let Some(map) = extra.as_object() {
        for (key, value) in map {
            object.insert(key.clone(), value.clone());
        }
    }
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-0",
        "kind": "request",
        "payload": payload,
    });
    let mut input = String::new();
    input.push_str(&frame.to_string());
    input.push('\n');
    let mut output = Vec::new();
    run_provider_host_full(
        Cursor::new(input),
        &mut output,
        database_path,
        None,
        None,
        None,
        None,
        config.cloned(),
        None,
        None,
        None,
    )
    .expect("frame loop runs");
    String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect()
}

/// Polls the tasked command to its terminal state and returns the stored
/// task (the single source both the completed-event payload and the
/// snapshot's result project).
fn wait_terminal(database_path: &Path, task_id: &str) -> vua_orchestrator::StoredTask {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let store = SqliteTaskStore::open(database_path).expect("store opens");
        let task = store
            .task(task_id)
            .expect("store readable")
            .expect("task exists");
        if task.state.is_terminal() {
            return task;
        }
        assert!(Instant::now() < deadline, "the task did not finish");
        drop(store);
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn vectors_drive_the_frozen_task_snapshot_schema() {
    let validator = snapshot_validator();
    let dir = snapshot_dir().join("examples");
    let mut seen = 0;
    for entry in fs::read_dir(&dir).expect("examples dir") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_string();
        // The examples dir hosts sibling vector families too (e.g. the
        // 017 overlay-snapshot vectors with their own schema + driver);
        // this driver owns only the task-snapshot family.
        if !name.starts_with("task-snapshot.") || !name.ends_with(".json") {
            continue;
        }
        let bytes = fs::read(&path).expect("vector readable");
        let vector: Value = serde_json::from_slice(&bytes).expect("vector is valid JSON");
        if name.contains(".valid.") {
            assert!(
                validator.is_valid(&vector),
                "valid vector must pass: {name}\n{vector}"
            );
        } else {
            assert!(
                name.contains(".invalid."),
                "vector files are named *.valid.* / *.invalid.*: {name}"
            );
            assert!(
                !validator.is_valid(&vector),
                "invalid vector must be refused: {name}\n{vector}"
            );
        }
        seen += 1;
    }
    assert_eq!(seen, 6, "the frozen vector set is exactly six files");
}

#[test]
fn completed_task_snapshot_refluxes_the_result_document() {
    let validator = snapshot_validator();
    let root = unique_root("result-reflux");
    let (source, target_parent, vcc_settings) = seed_registered_source(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    // Submit the tasked plan phase; the acceptance is the receipt the
    // renderer sees today (the seam this increment closes).
    let frames = run_frame(
        &database,
        Some(&config),
        "command",
        "project.import-copy",
        json!({
            "params": {
                "phase": "plan",
                "sourcePath": source.to_string_lossy(),
                "targetParentDirectory": target_parent.to_string_lossy(),
                "targetProjectName": "Copied Project",
            }
        }),
    );
    let acceptance = &frames[0]["payload"]["value"];
    assert_eq!(
        acceptance["operation"], "project.import-copy",
        "the live acceptance is the tasked receipt, not a result document"
    );
    let task_id = acceptance["taskId"].as_str().expect("taskId").to_owned();

    // The single stored result is what both channels must project.
    let stored = wait_terminal(&database, &task_id);
    assert!(stored.state.is_terminal());
    let done_payload = stored
        .result
        .as_ref()
        .expect("the plan task carries a result");
    assert_eq!(done_payload["operation"], "project.import-copy");
    assert_eq!(done_payload["result"]["kind"], "plan");

    // The snapshot channel now refluxes it verbatim.
    let frames = run_frame(
        &database,
        None,
        "query",
        "task.get",
        json!({
            "params": { "taskId": task_id }
        }),
    );
    let snapshot = &frames[0]["payload"]["value"];
    assert_eq!(snapshot["state"], "succeeded");
    assert_eq!(
        snapshot["result"], *done_payload,
        "the snapshot result is the stored Done payload, verbatim"
    );
    assert!(
        validator.is_valid(snapshot),
        "the snapshot matches the frozen face: {snapshot}"
    );

    // task.list projects the same shape.
    let frames = run_frame(
        &database,
        None,
        "query",
        "task.list",
        json!({ "params": {} }),
    );
    let tasks = &frames[0]["payload"]["value"]["tasks"];
    let listed = tasks
        .as_array()
        .expect("tasks array")
        .iter()
        .find(|task| task["taskId"] == task_id.as_str())
        .expect("the task is listed")
        .clone();
    assert_eq!(listed["result"], *done_payload);
    assert!(
        validator.is_valid(&listed),
        "the listed snapshot matches the frozen face"
    );
}

#[test]
fn cancelled_task_snapshot_never_carries_a_result() {
    let validator = snapshot_validator();
    let root = unique_root("cancelled-no-result");
    let database = root.join("tasks.sqlite");

    // The demo command creates a real queued task over the task face.
    let frames = run_frame(
        &database,
        None,
        "command",
        "task.startDemo",
        json!({
            "commandId": "demo-cancel-1"
        }),
    );
    let task_id = frames[0]["payload"]["value"]["task"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();

    // Cancel it; the terminal face is the failure/cancellation face.
    let frames = run_frame(
        &database,
        None,
        "command",
        "task.requestCancellation",
        json!({
            "params": { "taskId": task_id }
        }),
    );
    assert_eq!(frames[0]["payload"]["value"]["state"], "cancelled");

    let frames = run_frame(
        &database,
        None,
        "query",
        "task.get",
        json!({
            "params": { "taskId": task_id }
        }),
    );
    let snapshot = &frames[0]["payload"]["value"];
    assert_eq!(snapshot["state"], "cancelled");
    assert!(
        snapshot.get("result").is_none(),
        "a cancelled snapshot never carries a result: {snapshot}"
    );
    assert!(
        validator.is_valid(snapshot),
        "the snapshot matches the frozen face: {snapshot}"
    );
}

#[test]
fn failed_snapshot_never_refluxes_a_storage_face_result() {
    // The job.execute recovery flow persists rollback-observation payloads on
    // FAILED tasks (storage-face facts the recovery flow reads directly).
    // The frozen invariant (proposal 020) keeps those off the contract face:
    // a failed snapshot surfaces the failure through `error` only, so the
    // projection is state-scoped, not merely null-scoped.
    let validator = snapshot_validator();
    let root = unique_root("failed-storage-result");
    let database = root.join("tasks.sqlite");
    let occurred_at = "2026-09-12T06:00:00.000Z";

    let store = SqliteTaskStore::open(&database).expect("store opens");
    let acceptance = store
        .accept_idempotent_task(
            "project.import-copy",
            "cmd-failed-with-result",
            "fingerprint-failed-with-result",
            &NewTask {
                task_id: "task-failed-with-result".into(),
                correlation_id: "corr-failed-with-result".into(),
                occurred_at: occurred_at.into(),
            },
            &json!({ "accepted": true }),
        )
        .expect("task accepted");
    let task_id = match acceptance {
        vua_orchestrator::IdempotentTaskAcceptance::Accepted { task, .. } => task.task_id,
        other => panic!("expected a fresh acceptance, got {other:?}"),
    };
    // Walk the state machine to Running before the terminal commit (the
    // runtime only completes non-terminal tasks from Running).
    store
        .mutate_task(
            &task_id,
            1,
            occurred_at,
            TaskMutation::Transition {
                state: vua_orchestrator::TaskState::Preparing,
                payload: json!({}),
            },
        )
        .expect("transition to preparing lands");
    store
        .mutate_task(
            &task_id,
            2,
            occurred_at,
            TaskMutation::Transition {
                state: vua_orchestrator::TaskState::Running,
                payload: json!({}),
            },
        )
        .expect("transition to running lands");
    store
        .mutate_task(
            &task_id,
            3,
            occurred_at,
            TaskMutation::Complete {
                state: vua_orchestrator::TaskState::Failed,
                error: Some(
                    AppErrorV1::new(
                        "vua.material.rollback_failed",
                        ErrorCategory::ExternalFailure,
                        "errors.material.executionFailed",
                        "corr-failed-with-result",
                    )
                    .with_recoverable(true),
                ),
                // The recovery-observation payload stays a storage fact.
                result: Some(json!({
                    "recovered": "rollback",
                    "restored": false,
                    "detail": "restore failed"
                })),
            },
        )
        .expect("terminal mutation lands");
    drop(store);

    let frames = run_frame(
        &database,
        None,
        "query",
        "task.get",
        json!({
            "params": { "taskId": task_id }
        }),
    );
    let snapshot = &frames[0]["payload"]["value"];
    assert_eq!(snapshot["state"], "failed");
    assert_eq!(
        snapshot["error"]["code"], "vua.material.rollback_failed",
        "the failure fact travels the error field"
    );
    assert!(
        snapshot.get("result").is_none(),
        "a failed snapshot never refluxes a storage-face result: {snapshot}"
    );
    assert!(
        validator.is_valid(snapshot),
        "the snapshot matches the frozen face: {snapshot}"
    );

    // task.list projects the same state-scoped face.
    let frames = run_frame(
        &database,
        None,
        "query",
        "task.list",
        json!({ "params": {} }),
    );
    let listed = frames[0]["payload"]["value"]["tasks"]
        .as_array()
        .expect("tasks array")
        .iter()
        .find(|task| task["taskId"] == task_id.as_str())
        .expect("the task is listed")
        .clone();
    assert_eq!(listed["state"], "failed");
    assert!(
        listed.get("result").is_none(),
        "the listed snapshot never carries a result"
    );
    assert!(
        validator.is_valid(&listed),
        "the listed snapshot matches the frozen face"
    );
}
