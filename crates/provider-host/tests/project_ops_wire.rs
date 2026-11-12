//! Project-ops wire tests (proposal 014, the project-ops v0.2 write face):
//! the tasked two-phase import and the setNote note task ride the real
//! frame loop over the frozen project-ops word list — plan/apply against a
//! real VCC-registered source project, typed guard refusals as Done-payload
//! result documents, and the closed-set/unavailable faces. The consumer
//! consumes the environment-side frozen schemas from
//! `schemas/project-ops/v0.2`; changing that word list without this
//! consumer fails here first. The v0.2 vectors (import-copy carried plus
//! the setNote set) drive JSON-Schema validation directly.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use std::time::{Duration, Instant};

use serde_json::{json, Value};
use vua_orchestrator::SqliteTaskStore;
use vua_project_manager::ManagerRoots;
use vua_provider_host::{run_provider_host_full, ProjectOpsConfig};

fn command_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/project-ops/v0.2")
}

fn result_validator() -> jsonschema::Validator {
    let bytes = fs::read(command_dir().join("result.schema.json")).expect("result schema exists");
    let schema: Value = serde_json::from_slice(&bytes).expect("result schema is valid JSON");
    jsonschema::validator_for(&schema).expect("frozen result schema must compile")
}

fn command_validator() -> jsonschema::Validator {
    let bytes = fs::read(command_dir().join("command.schema.json")).expect("command schema exists");
    let schema: Value = serde_json::from_slice(&bytes).expect("command schema is valid JSON");
    jsonschema::validator_for(&schema).expect("frozen command schema must compile")
}

/// A unique per-test root (temp dir + label + pid + nanos).
fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-project-ops-{label}-{}-{nanos}",
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
    project_ops_config_with_editors(vcc_settings, &[])
}

fn project_ops_config_with_editors(
    vcc_settings: &Path,
    editor_roots: &[PathBuf],
) -> ProjectOpsConfig {
    ProjectOpsConfig {
        vcc_settings_candidates: vec![vcc_settings.to_path_buf()],
        manager_roots: ManagerRoots {
            alcom_settings_candidates: Vec::new(),
        },
        editor_roots: editor_roots.to_vec(),
    }
}

/// Runs one frame through the real host loop with the project-ops wiring.
fn run_frames(
    database_path: &Path,
    config: Option<&ProjectOpsConfig>,
    requests: &[Value],
) -> Vec<Value> {
    let mut input = String::new();
    for (index, request) in requests.iter().enumerate() {
        let frame = json!({
            "frameVersion": "0.1",
            "frameId": format!("frame-{index}"),
            "kind": "request",
            "payload": {
                "contractVersion": "0.1",
                "requestId": format!("req-{index}"),
                "correlationId": format!("corr-{index}"),
                "kind": "command",
                "method": request["method"],
                "params": request["params"],
            },
        });
        input.push_str(&frame.to_string());
        input.push('\n');
    }
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

/// Polls the tasked command to its terminal state and returns the Done
/// payload (the frozen result document travels inside it).
fn wait_done(database_path: &Path, task_id: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let store = SqliteTaskStore::open(database_path).expect("store opens");
        let task = store
            .task(task_id)
            .expect("store readable")
            .expect("task exists");
        if task.state.is_terminal() {
            assert_eq!(
                serde_json::to_value(task.state).unwrap(),
                "succeeded",
                "a typed guard refusal still completes the task: {:?}",
                task.error
            );
            return task.result.expect("done payload");
        }
        assert!(
            Instant::now() < deadline,
            "the import-copy task did not finish"
        );
        drop(store);
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn import_copy_plan_and_apply_run_the_full_two_phase_chain() {
    let validator = result_validator();
    let command_validator = command_validator();
    let root = unique_root("full-chain");
    let (source, target_parent, vcc_settings) = seed_registered_source(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    // The command we send must match the frozen command schema.
    let plan_command = json!({
        "schemaVersion": "0.2",
        "operation": "project.import-copy",
        "params": {
            "phase": "plan",
            "sourcePath": source.to_string_lossy(),
            "targetParentDirectory": target_parent.to_string_lossy(),
            "targetProjectName": "Copied Project",
        }
    });
    assert!(
        command_validator.is_valid(&plan_command),
        "the request must match the frozen command schema: {plan_command}"
    );
    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.import-copy",
            "params": plan_command["params"].clone(),
        })],
    );
    let acceptance = &frames[0]["payload"]["value"];
    assert_eq!(acceptance["operation"], "project.import-copy");
    let task_id = acceptance["taskId"].as_str().expect("taskId").to_owned();

    // The plan face: measured bytes, exclusions, transparency list, digest.
    let payload = wait_done(&database, &task_id);
    assert_eq!(payload["schemaVersion"], "0.2");
    // The frozen result schema describes the whole envelope
    // (schemaVersion/operation/result) — exactly the Done payload.
    assert!(
        validator.is_valid(&payload),
        "plan must match the frozen result schema: {payload}"
    );
    let plan = &payload["result"];
    assert_eq!(plan["kind"], "plan");
    assert!(plan["estimatedBytes"].as_u64().expect("bytes") > 0);
    let excluded = plan["excludedEntries"].as_array().expect("exclusions");
    assert!(
        excluded.contains(&json!("Library")),
        "Unity regenerables are excluded: {excluded:?}"
    );
    let top_levels = plan["sourceTopLevels"].as_array().expect("top levels");
    assert!(top_levels.contains(&json!("Assets")) && top_levels.contains(&json!("Packages")));
    let plan_digest = plan["planDigest"].as_str().expect("planDigest").to_owned();

    // The apply face: drift-checked copy + receipt + the recorded source link.
    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.import-copy",
            "params": {
                "phase": "apply",
                "sourcePath": source.to_string_lossy(),
                "targetParentDirectory": target_parent.to_string_lossy(),
                "targetProjectName": "Copied Project",
                "confirmedPlanDigest": plan_digest,
            }
        })],
    );
    let task_id = frames[0]["payload"]["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();
    let payload = wait_done(&database, &task_id);
    assert!(
        validator.is_valid(&payload),
        "receipt must match the frozen result schema: {payload}"
    );
    let receipt = &payload["result"];
    assert_eq!(receipt["kind"], "receipt");
    assert!(receipt["copiedTopLevels"]
        .as_array()
        .expect("copied")
        .contains(&json!("Assets")));
    assert_eq!(
        receipt["sourceLink"]["sourceAssociations"],
        json!(["vcc_registered"])
    );
    assert!(
        !receipt["sourceLink"]["taskCorrelation"]
            .as_str()
            .unwrap_or_default()
            .is_empty(),
        "the audit chain carries the originating correlation: {receipt}"
    );

    // The copy really landed: Assets exists, Library does not, and the
    // source link is recorded inside the new project's .vua.
    let target = target_parent.join("Copied Project");
    assert!(target.join("Assets").exists(), "Assets copied");
    assert!(
        !target.join("Library").exists(),
        "regenerables are not copied"
    );
    assert!(
        target.join(".vua").join("source.json").exists(),
        "source link recorded"
    );
}

#[test]
fn import_copy_apply_refuses_on_plan_drift() {
    let validator = result_validator();
    let root = unique_root("drift");
    let (source, target_parent, vcc_settings) = seed_registered_source(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.import-copy",
            "params": {
                "phase": "apply",
                "sourcePath": source.to_string_lossy(),
                "targetParentDirectory": target_parent.to_string_lossy(),
                "targetProjectName": "Copied Project",
                "confirmedPlanDigest": "stale-digest",
            }
        })],
    );
    let task_id = frames[0]["payload"]["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();
    let payload = wait_done(&database, &task_id);
    assert!(
        validator.is_valid(&payload),
        "rejection must match the frozen schema: {payload}"
    );
    let rejected = &payload["result"];
    assert_eq!(rejected["kind"], "rejected");
    assert_eq!(rejected["guard"], "plan_drift");
    assert_eq!(rejected["code"], "vua.project.plan_drift");
    // Nothing was copied behind a refused confirmation.
    assert!(!target_parent.join("Copied Project").exists());
}

#[test]
fn import_copy_refuses_sources_no_manager_registers() {
    let validator = result_validator();
    let root = unique_root("unregistered");
    let (source, target_parent, vcc_settings) = seed_registered_source(&root);
    // A second project that no manager registers.
    let stranger = root.join("stranger-project");
    fs::create_dir_all(stranger.join("Assets")).expect("stranger Assets");
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.import-copy",
            "params": {
                "phase": "plan",
                "sourcePath": stranger.to_string_lossy(),
                "targetParentDirectory": target_parent.to_string_lossy(),
                "targetProjectName": "Stranger Copy",
            }
        })],
    );
    let task_id = frames[0]["payload"]["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();
    let payload = wait_done(&database, &task_id);
    assert!(
        validator.is_valid(&payload),
        "rejection must match the frozen schema: {payload}"
    );
    let rejected = &payload["result"];
    assert_eq!(rejected["kind"], "rejected");
    assert_eq!(rejected["guard"], "source_not_registered");
    assert_eq!(rejected["code"], "vua.project.source_not_registered");
    let _ = source;
}

#[test]
fn import_copy_wire_guards_and_honest_absence() {
    let root = unique_root("guards");
    let (source, target_parent, vcc_settings) = seed_registered_source(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    // Unknown params key: a contract error (closed set), never ignored.
    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.import-copy",
            "params": {
                "phase": "plan",
                "sourcePath": source.to_string_lossy(),
                "targetParentDirectory": target_parent.to_string_lossy(),
                "targetProjectName": "Copied Project",
                "extra": true,
            }
        })],
    );
    assert_eq!(
        frames[0]["payload"]["error"]["code"],
        "vua.project.invalid_params"
    );

    // apply without the confirmed digest: a params violation.
    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.import-copy",
            "params": {
                "phase": "apply",
                "sourcePath": source.to_string_lossy(),
                "targetParentDirectory": target_parent.to_string_lossy(),
                "targetProjectName": "Copied Project",
            }
        })],
    );
    assert_eq!(
        frames[0]["payload"]["error"]["code"],
        "vua.project.invalid_params"
    );

    // An unknown project.* method is a contract error (the write word list
    // is project.import-copy plus project.setNote — nothing else).
    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({"method": "project.delete", "params": {}})],
    );
    assert_eq!(
        frames[0]["payload"]["error"]["code"],
        "vua.provider.unknown_method"
    );

    // Without the project-ops wiring the face is honestly unavailable.
    let frames = run_frames(
        &database,
        None,
        &[json!({
            "method": "project.import-copy",
            "params": {
                "phase": "plan",
                "sourcePath": source.to_string_lossy(),
                "targetParentDirectory": target_parent.to_string_lossy(),
                "targetProjectName": "Copied Project",
            }
        })],
    );
    assert_eq!(
        frames[0]["payload"]["error"]["code"],
        "vua.project.unavailable"
    );
}

// --- proposal 013 read face: the environment-managers snapshot ---

/// Creates a fake Unity Hub editor install (`<root>/<version>/Editor`).
fn install_fake_editor(editors_root: &Path, version: &str) {
    fs::create_dir_all(editors_root.join(version).join("Editor")).expect("fake editor layout");
}

#[test]
fn environment_managers_snapshot_serves_over_the_wire() {
    let validator = jsonschema::validator_for(&{
        let bytes = fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../schemas/project-inspection/v0.2/result.schema.json"),
        )
        .expect("result schema exists");
        serde_json::from_slice::<Value>(&bytes).expect("result schema is valid JSON")
    })
    .expect("frozen result schema must compile");
    let root = unique_root("env-managers");
    let (source, _target_parent, vcc_settings) = seed_registered_source(&root);
    let database = root.join("tasks.sqlite");
    let editors_root = root.join("editors");
    install_fake_editor(&editors_root, "2022.3.22f1");
    let mut config = project_ops_config(&vcc_settings);
    config.editor_roots = vec![editors_root];

    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({"method": "project.environmentManagers", "params": {}})],
    );
    let value = &frames[0]["payload"]["value"];
    assert_eq!(value["schemaVersion"], "0.1");
    assert_eq!(value["operation"], "project.environmentManagers");
    assert!(
        validator.is_valid(value),
        "the snapshot must match the frozen result schema: {value}"
    );
    let result = &value["result"];
    assert_eq!(result["vcc"]["presence"], "found");
    assert_eq!(result["vcc"]["projectsSource"], "userProjects");
    let projects = result["projects"].as_array().expect("projects");
    assert!(
        projects
            .iter()
            .any(|project| project["path"].as_str() == Some(source.to_string_lossy().as_ref())),
        "the registered source project must appear: {projects:?}"
    );
    let editors = result["editors"].as_array().expect("editors");
    assert!(
        editors
            .iter()
            .any(|editor| editor["version"].as_str() == Some("2022.3.22f1")),
        "the fake editor install must be observed: {editors:?}"
    );
}

#[test]
fn project_read_face_closed_set_and_absence_are_typed() {
    let root = unique_root("read-face-guards");
    let (_source, _target_parent, vcc_settings) = seed_registered_source(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    // Params content on the param-free query is a contract error.
    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.environmentManagers",
            "params": {"unexpected": true},
        })],
    );
    assert_eq!(
        frames[0]["payload"]["error"]["code"],
        "vua.project.invalid_params"
    );

    // An unknown project.* method is a contract error.
    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({"method": "project.frobnicate", "params": {}})],
    );
    assert_eq!(
        frames[0]["payload"]["error"]["code"],
        "vua.provider.unknown_method"
    );
}

// --- proposal 013 read face: the three project queries ---

fn v02_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&{
        let bytes = fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../schemas/project-inspection/v0.2/result.schema.json"),
        )
        .expect("result schema exists");
        serde_json::from_slice::<Value>(&bytes).expect("result schema is valid JSON")
    })
    .expect("frozen result schema must compile")
}

#[test]
fn project_list_projects_serves_the_registered_inspection_aggregate() {
    let validator = v02_result_validator();
    let root = unique_root("list-projects");
    let (source, _target_parent, vcc_settings) = seed_registered_source(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({"method": "project.listProjects", "params": {}})],
    );
    let value = &frames[0]["payload"]["value"];
    assert_eq!(value["schemaVersion"], "0.1");
    assert_eq!(value["operation"], "project.listProjects");
    assert!(
        validator.is_valid(value),
        "aggregate must match the frozen schema: {value}"
    );
    let projects = value["result"]["projects"].as_array().expect("projects");
    assert_eq!(projects.len(), 1, "exactly the registered project lists");
    let project = &projects[0];
    assert_eq!(project["path"], json!(source.to_string_lossy()));
    assert_eq!(project["name"], "source-project");
    assert_eq!(project["associations"], json!(["vcc_registered"]));
    assert_eq!(project["mutationStatus"], "none");
    // The v0.2 tri-state: the fresh copy has no VUA identity yet.
    assert_eq!(project["vuaIdentity"]["status"], "absent");
}

#[test]
fn project_inspect_project_serves_one_and_refuses_unregistered() {
    let validator = v02_result_validator();
    let root = unique_root("inspect-one");
    let (source, _target_parent, vcc_settings) = seed_registered_source(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.inspectProject",
            "params": {"projectPath": source.to_string_lossy()},
        })],
    );
    let value = &frames[0]["payload"]["value"];
    assert_eq!(value["operation"], "project.inspectProject");
    assert!(
        validator.is_valid(value),
        "single inspection must match the frozen schema: {value}"
    );
    assert_eq!(value["result"]["path"], json!(source.to_string_lossy()));
    assert_eq!(value["result"]["manifestPresent"], true);

    // A path no manager registers is the typed not-found (the detection
    // face's registry is its world).
    let stranger = root.join("stranger-project");
    fs::create_dir_all(&stranger).expect("stranger dir");
    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.inspectProject",
            "params": {"projectPath": stranger.to_string_lossy()},
        })],
    );
    assert_eq!(
        frames[0]["payload"]["error"]["code"],
        "vua.project.project_not_found"
    );

    // Missing projectPath is a params violation (the frozen negative vector).
    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({"method": "project.inspectProject", "params": {}})],
    );
    assert_eq!(
        frames[0]["payload"]["error"]["code"],
        "vua.project.invalid_params"
    );
}

#[test]
fn project_lock_status_reports_the_marker_observation() {
    let validator = v02_result_validator();
    let root = unique_root("lock-status");
    let (_source, target_parent, vcc_settings) = seed_registered_source(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    // A project directory with no marker observes "none".
    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.lockStatus",
            "params": {"projectPath": target_parent.to_string_lossy()},
        })],
    );
    let value = &frames[0]["payload"]["value"];
    assert!(
        validator.is_valid(value),
        "lock status must match the frozen schema: {value}"
    );
    assert_eq!(value["result"]["mutationStatus"], "none");
    assert_eq!(
        value["result"]["projectPath"],
        json!(target_parent.to_string_lossy())
    );
}

// --- project-ops v0.2: the setNote note task (D-6 confirmation) ---

/// The frozen v0.2 vectors drive schema validation: every positive
/// request/result validates, every negative request vector is rejected,
/// and the guard-refusal result vector is a legal result shape.
#[test]
fn set_note_vectors_validate_against_the_frozen_schemas() {
    let command_validator = command_validator();
    let result_validator = result_validator();

    let positive_requests = [
        "project-set-note.request.json",
        "project-set-note-clear.request.json",
    ];
    for name in positive_requests {
        let vector: Value = serde_json::from_slice(
            &fs::read(command_dir().join("examples").join(name)).expect("vector exists"),
        )
        .expect("valid JSON");
        assert!(
            command_validator.is_valid(&vector),
            "positive request vector {name} must validate: {vector}"
        );
    }

    let positive_results = [
        "project-import-copy-plan.result.json",
        "project-import-copy-apply.result.json",
        "project-set-note.result.json",
        "project-set-note-clear.result.json",
    ];
    for name in positive_results {
        let vector: Value = serde_json::from_slice(
            &fs::read(command_dir().join("examples").join(name)).expect("vector exists"),
        )
        .expect("valid JSON");
        assert!(
            result_validator.is_valid(&vector),
            "positive result vector {name} must validate: {vector}"
        );
    }

    let negative_requests = [
        "invalid-operation.json",
        "invalid-phase.json",
        "invalid-set-note-missing-project-path.request.json",
        "invalid-set-note-empty-note.request.json",
        "invalid-set-note-multiline.request.json",
    ];
    for name in negative_requests {
        let vector: Value = serde_json::from_slice(
            &fs::read(command_dir().join("examples").join(name)).expect("vector exists"),
        )
        .expect("valid JSON");
        assert!(
            !command_validator.is_valid(&vector),
            "negative request vector {name} must be rejected: {vector}"
        );
    }

    let refusal: Value = serde_json::from_slice(
        &fs::read(
            command_dir()
                .join("examples")
                .join("set-note-rejected-not-vua-native.result.json"),
        )
        .expect("vector exists"),
    )
    .expect("valid JSON");
    assert!(
        result_validator.is_valid(&refusal),
        "the guard-refusal vector must be a legal result: {refusal}"
    );
}

/// Marks a registered project VUA-native with the environment-side
/// first-marking primitive (import-copy completion does the same).
fn mark_source_vua_native(source: &Path) {
    vua_project_manager::mark_vua_native(source, "2026-09-12T04:10:00.000Z")
        .expect("identity marked");
}

#[test]
fn set_note_stores_and_clears_over_the_wire() {
    let validator = result_validator();
    let command_validator = command_validator();
    let root = unique_root("set-note");
    let (source, _target_parent, vcc_settings) = seed_registered_source(&root);
    mark_source_vua_native(&source);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    // The command we send must match the frozen command schema.
    let note_command = json!({
        "schemaVersion": "0.2",
        "operation": "project.setNote",
        "params": {
            "projectPath": source.to_string_lossy(),
            "note": "fixture note",
        }
    });
    assert!(
        command_validator.is_valid(&note_command),
        "the request must match the frozen command schema: {note_command}"
    );

    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.setNote",
            "params": note_command["params"].clone(),
        })],
    );
    let acceptance = &frames[0]["payload"]["value"];
    assert_eq!(acceptance["schemaVersion"], "0.2");
    assert_eq!(acceptance["operation"], "project.setNote");
    let task_id = acceptance["taskId"].as_str().expect("taskId").to_owned();

    // The Done payload carries the frozen note result: the stored state,
    // projected with the vuaIdentity present-face field names.
    let payload = wait_done(&database, &task_id);
    assert!(
        validator.is_valid(&payload),
        "note result must match the frozen schema: {payload}"
    );
    let note = &payload["result"];
    assert_eq!(note["kind"], "note");
    assert_eq!(note["note"], json!("fixture note"));
    assert_eq!(
        note["markedAt"],
        json!("2026-09-12T04:10:00.000Z"),
        "setting a note never re-marks the project"
    );
    // The note really landed in the identity file.
    let identity = vua_project_manager::read_identity(&source);
    assert_eq!(identity.note(), Some("fixture note"));

    // null clears; markedAt stays.
    let frames = run_frames(
        &database,
        Some(&config),
        &[json!({
            "method": "project.setNote",
            "params": {"projectPath": source.to_string_lossy(), "note": null},
        })],
    );
    let task_id = frames[0]["payload"]["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();
    let payload = wait_done(&database, &task_id);
    assert!(
        validator.is_valid(&payload),
        "cleared note must match the frozen schema: {payload}"
    );
    assert_eq!(payload["result"]["kind"], "note");
    assert_eq!(payload["result"]["note"], json!(null));
    assert_eq!(
        vua_project_manager::read_identity(&source).note(),
        None,
        "the cleared note is really gone"
    );
}

#[test]
fn set_note_guards_refuse_typecally_inside_the_task() {
    let validator = result_validator();
    let root = unique_root("set-note-guards");
    let (source, _target_parent, vcc_settings) = seed_registered_source(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let submit_note =
        |database_path: &Path, config: Option<&ProjectOpsConfig>, project_path: String| {
            run_frames(
                database_path,
                config,
                &[json!({
                    "method": "project.setNote",
                    "params": {"projectPath": project_path, "note": "fixture note"},
                })],
            )
        };

    // A registered project with no identity file: not_vua_native — the
    // note presupposes the VUA-native declaration.
    let frames = submit_note(
        &database,
        Some(&config),
        source.to_string_lossy().into_owned(),
    );
    let task_id = frames[0]["payload"]["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();
    let payload = wait_done(&database, &task_id);
    assert!(
        validator.is_valid(&payload),
        "refusal must match the frozen schema: {payload}"
    );
    assert_eq!(payload["result"]["kind"], "rejected");
    assert_eq!(payload["result"]["guard"], "not_vua_native");
    assert_eq!(payload["result"]["code"], "vua.project.not_vua_native");
    // The refusal is honest on disk too: no identity file was invented.
    assert!(!source.join(".vua").join("project.json").exists());

    // A path no manager registers: project_not_found (the detection
    // face's registry is the writable world).
    let stranger = root.join("stranger-project");
    fs::create_dir_all(&stranger).expect("stranger dir");
    let frames = submit_note(
        &database,
        Some(&config),
        stranger.to_string_lossy().into_owned(),
    );
    let task_id = frames[0]["payload"]["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();
    let payload = wait_done(&database, &task_id);
    assert!(
        validator.is_valid(&payload),
        "refusal must match the frozen schema: {payload}"
    );
    assert_eq!(payload["result"]["guard"], "project_not_found");
    assert_eq!(payload["result"]["code"], "vua.project.project_not_found");

    // An unreadable identity is evidence: refused, never overwritten by
    // a blind rewrite.
    fs::create_dir_all(source.join(".vua")).expect(".vua dir");
    fs::write(source.join(".vua").join("project.json"), b"{ not json").expect("garbage identity");
    let frames = submit_note(
        &database,
        Some(&config),
        source.to_string_lossy().into_owned(),
    );
    let task_id = frames[0]["payload"]["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();
    let payload = wait_done(&database, &task_id);
    assert!(
        validator.is_valid(&payload),
        "refusal must match the frozen schema: {payload}"
    );
    assert_eq!(payload["result"]["guard"], "identity_unreadable");
    assert_eq!(payload["result"]["code"], "vua.project.identity_unreadable");
    // The unreadable evidence is untouched.
    assert_eq!(
        fs::read(source.join(".vua").join("project.json")).expect("identity intact"),
        b"{ not json"
    );
}

#[test]
fn set_note_params_are_a_frozen_closed_set() {
    let root = unique_root("set-note-params");
    let (source, _target_parent, vcc_settings) = seed_registered_source(&root);
    mark_source_vua_native(&source);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let expect_invalid_params = |params: Value| {
        let frames = run_frames(
            &database,
            Some(&config),
            &[json!({"method": "project.setNote", "params": params})],
        );
        assert_eq!(
            frames[0]["payload"]["error"]["code"], "vua.project.invalid_params",
            "params must be refused: {params}"
        );
    };

    // Missing note.
    expect_invalid_params(json!({"projectPath": source.to_string_lossy()}));
    // Missing projectPath.
    expect_invalid_params(json!({"note": "fixture note"}));
    // An empty note is not a note: clearing uses null.
    expect_invalid_params(json!({"projectPath": source.to_string_lossy(), "note": ""}));
    // Single-line plain text only.
    expect_invalid_params(json!({"projectPath": source.to_string_lossy(), "note": "a\nb"}));
    // Unknown key: a contract error, never ignored.
    expect_invalid_params(
        json!({"projectPath": source.to_string_lossy(), "note": "n", "extra": 1}),
    );

    // Without the project-ops wiring the face is honestly unavailable.
    let frames = run_frames(
        &database,
        None,
        &[json!({
            "method": "project.setNote",
            "params": {"projectPath": source.to_string_lossy(), "note": "fixture note"},
        })],
    );
    assert_eq!(
        frames[0]["payload"]["error"]["code"],
        "vua.project.unavailable"
    );
}
