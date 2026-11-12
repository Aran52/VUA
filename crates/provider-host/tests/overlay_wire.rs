//! Overlay read-face wire tests (proposal 017 batch 1, `overlay.getSnapshot`):
//! the overlay's one-glance surface — task cards plus the production-status
//! card — served as one polling query over the ordinary provider connection
//! (desktop stances 1–2: on-demand polling, zero overlay session identity).
//! The consumer consumes the frozen snapshot schema from
//! `schemas/application-contract/v0.1`; changing the overlay face without
//! this consumer fails here first. The vectors (3 valid + 3 invalid) drive
//! JSON-Schema validation directly.

use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use serde_json::{json, Value};
use vua_orchestrator::{
    EvidenceStore, NewTask, PlanDocumentStore, RecipeDocumentStore, RecipeRecordStore,
    SqliteTaskStore, UnityBridge, UnityCommand, UnityResult,
};
use vua_provider_host::{run_provider_host_with_services, ProductionUseCaseConfig};

fn snapshot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/application-contract/v0.1")
}

fn overlay_validator() -> jsonschema::Validator {
    let bytes = fs::read(snapshot_dir().join("overlay-snapshot.schema.json"))
        .expect("overlay schema exists");
    let schema: Value = serde_json::from_slice(&bytes).expect("overlay schema is valid JSON");
    jsonschema::validator_for(&schema).expect("frozen overlay schema must compile")
}

/// A unique per-test root (temp dir + label + pid + nanos).
fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-overlay-wire-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// The overlay face never executes jobs; the bridge in these fixtures is a
/// placeholder that must never be reached by the read face.
struct NoBridge;

impl UnityBridge for NoBridge {
    fn execute(
        &self,
        _project: &vua_orchestrator::ProjectRef,
        command: &UnityCommand,
    ) -> Result<UnityResult, vua_orchestrator::BridgeError> {
        Err(vua_orchestrator::BridgeError::Io(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!(
                "the overlay read face must not execute bridge commands ({})",
                command.command_id
            ),
        )))
    }
}

fn use_case_config(root: &std::path::Path) -> ProductionUseCaseConfig {
    let production_root = root.join("production");
    ProductionUseCaseConfig {
        recipes: std::sync::Arc::new(RecipeDocumentStore::new_with_system_clock(
            production_root.join("recipes"),
        )),
        plans: std::sync::Arc::new(PlanDocumentStore::new(production_root.join("plans"))),
        evidence: std::sync::Arc::new(EvidenceStore::new(production_root.join("evidence"))),
        records: std::sync::Arc::new(RecipeRecordStore::new(production_root.join("records"))),
        inspections: std::sync::Arc::new(vua_orchestrator::InspectionEvidenceStore::new(
            production_root.join("inspections"),
        )),
        editor_version: "2022.3.22f1".to_owned(),
        bridge: std::sync::Arc::new(NoBridge),
        project_root: root.join("project"),
        editor_selection: vua_orchestrator::EditorSelection::Unavailable {
            reason: vua_orchestrator::EditorSelectionGap::NotDetected,
        },
        handoff: None,
        draft_exporter: None,
    }
}

/// Runs one `overlay.getSnapshot` query frame through the real host loop.
fn run_overlay_frame(
    database_path: &std::path::Path,
    use_cases: Option<ProductionUseCaseConfig>,
    params: Value,
) -> Vec<Value> {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-overlay-0",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-overlay-0",
            "correlationId": "corr-overlay-0",
            "kind": "query",
            "method": "overlay.getSnapshot",
            "params": params,
        },
    });
    let mut output = Vec::new();
    run_provider_host_with_services(
        Cursor::new(format!("{frame}\n")),
        &mut output,
        database_path,
        None,
        None,
        None,
        use_cases,
    )
    .expect("frame loop runs");
    String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect()
}

/// Seeds one durable task directly into the store (the task card projects
/// whatever the authority holds, running or terminal).
fn seed_task(database_path: &std::path::Path, suffix: &str) {
    let store = SqliteTaskStore::open(database_path).expect("store opens");
    store
        .accept_idempotent_task(
            "task.startDemo",
            &format!("command-{suffix}"),
            &format!("fingerprint-{suffix}"),
            &NewTask {
                task_id: format!("task-{suffix}"),
                correlation_id: format!("corr-{suffix}"),
                occurred_at: "2026-09-12T06:00:00.000Z".into(),
            },
            &json!({ "seeded": true }),
        )
        .expect("task accepted");
}

#[test]
fn vectors_drive_the_frozen_overlay_snapshot_schema() {
    let validator = overlay_validator();
    let dir = snapshot_dir().join("examples");
    let mut seen = 0;
    for entry in fs::read_dir(&dir).expect("examples dir") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_string();
        if !name.starts_with("overlay-snapshot.") || !name.ends_with(".json") {
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
    assert_eq!(
        seen, 8,
        "the frozen overlay vector set is exactly eight files"
    );
}

#[test]
fn overlay_snapshot_serves_the_honest_empty_state() {
    let validator = overlay_validator();
    let root = unique_root("empty");
    let database = root.join("tasks.sqlite");

    // 空态即终态: no tasks, no plans, no records — an empty tasks list and
    // a card with two null halves, never a synthesized row.
    let frames = run_overlay_frame(&database, Some(use_case_config(&root)), json!({}));
    let value = &frames[0]["payload"]["value"];
    assert!(
        frames[0]["payload"]["ok"].as_bool().expect("ok flag"),
        "the empty query succeeds: {frames:?}"
    );
    assert_eq!(value["contractVersion"], "0.1");
    assert_eq!(value["tasks"], json!([]));
    assert_eq!(
        value["productionCard"],
        json!({ "currentPlan": null, "latestRecord": null })
    );
    // Batch 2: nothing in flight is the honest empty download card — never
    // an error, never a synthesized row.
    assert_eq!(value["downloadCard"], json!({ "activeDownloads": [] }));
    assert!(
        validator.is_valid(value),
        "the empty snapshot matches the frozen face: {value}"
    );
}

#[test]
fn overlay_snapshot_projects_tasks_and_the_production_card() {
    let validator = overlay_validator();
    let root = unique_root("populated");
    let database = root.join("tasks.sqlite");
    let config = use_case_config(&root);

    // Seed the authorities: one durable task, three plans (out-of-order
    // storage: the newest createdAt must win), two records.
    seed_task(&database, "alpha");
    {
        let plans = PlanDocumentStore::new(root.join("production").join("plans"));
        let document = |plan_id: &str, created_at: &str| {
            json!({
                "schemaVersion": "0.3",
                "planId": plan_id,
                "recipeId": "019e0000-0000-7000-8000-000000000001",
                "recipeRevision": 1,
                "createdAt": created_at,
                "planHash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "jobs": [],
                "status": "draft",
            })
        };
        plans
            .publish_draft(
                "plan-b-older",
                &document("plan-b-older", "2026-09-12T01:00:00.000Z"),
            )
            .unwrap();
        plans
            .publish_draft(
                "plan-c-newest",
                &document("plan-c-newest", "2026-09-12T03:00:00.000Z"),
            )
            .unwrap();
        plans.approve("plan-c-newest").unwrap();
        plans
            .publish_draft(
                "plan-a-oldest",
                &document("plan-a-oldest", "2026-09-12T00:00:00.000Z"),
            )
            .unwrap();

        let records = RecipeRecordStore::new(root.join("production").join("records"));
        let record = |build_id: &str, finished_at: &str, status: &str| {
            json!({
                "schemaVersion": "0.3",
                "buildId": build_id,
                "recipeId": "019e0000-0000-7000-8000-000000000001",
                "planId": "plan-c-newest",
                "startedAt": "2026-09-12T01:30:00.000Z",
                "finishedAt": finished_at,
                "status": status,
                "jobs": [],
            })
        };
        records
            .publish(
                "build-z-mid",
                &record("build-z-mid", "2026-09-12T02:00:00.000Z", "succeeded"),
            )
            .unwrap();
        records
            .publish(
                "build-a-newest",
                &record("build-a-newest", "2026-09-12T04:00:00.000Z", "failed"),
            )
            .unwrap();
    }

    let frames = run_overlay_frame(&database, Some(config), json!({}));
    let value = &frames[0]["payload"]["value"];
    assert!(
        validator.is_valid(value),
        "the snapshot matches the frozen face: {value}"
    );

    // Task card: the seeded durable task projects with its identity facts.
    let tasks = value["tasks"].as_array().expect("tasks array");
    assert_eq!(tasks.len(), 1, "exactly the seeded task is listed");
    assert_eq!(tasks[0]["taskId"], "task-alpha");
    assert_eq!(tasks[0]["correlationId"], "corr-alpha");
    assert_eq!(tasks[0]["state"], "queued");

    // Production card: newest createdAt / newest finishedAt win.
    let card = &value["productionCard"];
    assert_eq!(card["currentPlan"]["planId"], "plan-c-newest");
    assert_eq!(card["currentPlan"]["planStatus"], "approved");
    assert_eq!(card["currentPlan"]["createdAt"], "2026-09-12T03:00:00.000Z");
    assert_eq!(card["latestRecord"]["buildId"], "build-a-newest");
    assert_eq!(card["latestRecord"]["status"], "failed");
    assert_eq!(
        card["latestRecord"]["finishedAt"],
        "2026-09-12T04:00:00.000Z"
    );

    // Pure-function discipline: a second unchanged query observes the same
    // payload (no query instant, no aggregate revision is invented).
    let again = run_overlay_frame(&database, Some(use_case_config(&root)), json!({}));
    assert_eq!(
        again[0]["payload"]["value"], *value,
        "polling never changes what it observes"
    );
}

#[test]
fn overlay_snapshot_carries_the_download_card_for_in_flight_attempts() {
    let validator = overlay_validator();
    let root = unique_root("downloads");
    let database = root.join("tasks.sqlite");
    let store = SqliteTaskStore::open(&database).expect("store opens");

    // Seed the authorities directly: two in-flight download attempts plus
    // one terminal attempt and one foreign (demo) task. Only the in-flight
    // `dl-` rows may enter the card — the stance is "while items are in
    // flight" (017 stance 3), a finished attempt leaves the card.
    let attempts = [
        (
            "dl-019e0000-0000-7000-8000-000000000601-a1",
            "019e0000-0000-7000-8000-000000000601",
        ),
        (
            "dl-019e0000-0000-7000-8000-000000000602-a1",
            "019e0000-0000-7000-8000-000000000602",
        ),
        (
            "dl-019e0000-0000-7000-8000-000000000603-a1",
            "019e0000-0000-7000-8000-000000000603",
        ),
    ];
    for (index, (task_id, download_id)) in attempts.iter().enumerate() {
        store
            .accept_task(&NewTask {
                task_id: (*task_id).to_owned(),
                correlation_id: (*download_id).to_owned(),
                occurred_at: format!("2026-09-12T0{}:00:00.000Z", index + 1),
            })
            .expect("download attempt accepted");
    }
    store
        .accept_task(&NewTask {
            task_id: "demo-019e0000-0000-7000-8000-000000000604".to_owned(),
            correlation_id: "corr-demo".to_owned(),
            occurred_at: "2026-09-12T04:00:00.000Z".to_owned(),
        })
        .expect("demo task accepted");
    // Walk one attempt through the real nine-state path to `succeeded`.
    for state in [
        vua_orchestrator::TaskState::Preparing,
        vua_orchestrator::TaskState::Running,
    ] {
        let current = store
            .task("dl-019e0000-0000-7000-8000-000000000603-a1")
            .expect("task readable")
            .expect("attempt exists");
        store
            .mutate_task(
                &current.task_id,
                current.revision,
                "2026-09-12T03:10:00.000Z",
                vua_orchestrator::TaskMutation::Transition {
                    state,
                    payload: json!({ "receivedBytes": 1, "expectedBytes": 2 }),
                },
            )
            .expect("transition lands");
    }
    let finished = store
        .task("dl-019e0000-0000-7000-8000-000000000603-a1")
        .expect("task readable")
        .expect("attempt exists");
    store
        .mutate_task(
            &finished.task_id,
            finished.revision,
            "2026-09-12T03:30:00.000Z",
            vua_orchestrator::TaskMutation::Complete {
                state: vua_orchestrator::TaskState::Succeeded,
                error: None,
                result: Some(json!({ "downloadId": "019e0000-0000-7000-8000-000000000603" })),
            },
        )
        .expect("completion lands");

    let frames = run_overlay_frame(&database, Some(use_case_config(&root)), json!({}));
    let value = &frames[0]["payload"]["value"];
    assert!(
        frames[0]["payload"]["ok"].as_bool().expect("ok flag"),
        "{frames:?}"
    );
    assert!(
        validator.is_valid(value),
        "the snapshot matches the frozen face: {value}"
    );

    // The download card holds exactly the in-flight attempts, trimmed to
    // their own identity facts — no byte progress is invented (progress
    // travels the task-event channel).
    let card = &value["downloadCard"];
    let rows = card["activeDownloads"].as_array().expect("rows array");
    assert_eq!(
        rows.len(),
        2,
        "terminal attempt and foreign task stay out: {card}"
    );
    assert_eq!(
        rows[0]["downloadId"],
        "019e0000-0000-7000-8000-000000000601"
    );
    assert_eq!(rows[0]["state"], "queued");
    assert_eq!(
        rows[1]["downloadId"],
        "019e0000-0000-7000-8000-000000000602"
    );
    assert!(
        rows[0].get("receivedBytes").is_none(),
        "no invented progress on the snapshot face"
    );
}

#[test]
fn overlay_snapshot_answers_typed_unavailable_without_the_production_wiring() {
    let root = unique_root("unavailable");
    let database = root.join("tasks.sqlite");

    // The production use-case wiring is absent: honest absence — a typed
    // unavailable error, never a silent empty snapshot.
    let frames = run_overlay_frame(&database, None, json!({}));
    let payload = &frames[0]["payload"];
    assert_eq!(
        payload["error"]["code"], "vua.overlay.unavailable",
        "honest absence: {payload}"
    );
    assert_eq!(payload["error"]["category"], "unavailable");
}

#[test]
fn overlay_snapshot_rejects_unknown_params() {
    let root = unique_root("params");
    let database = root.join("tasks.sqlite");

    // Params are a closed empty set: an unknown key is a contract error,
    // never silently ignored.
    let frames = run_overlay_frame(
        &database,
        Some(use_case_config(&root)),
        json!({ "watch": "everything" }),
    );
    let payload = &frames[0]["payload"];
    assert_eq!(
        payload["error"]["code"], "vua.overlay.invalid_params",
        "{payload}"
    );
    assert_eq!(payload["error"]["category"], "validation");
}
