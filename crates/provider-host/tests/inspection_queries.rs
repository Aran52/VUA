//! M7 inspection slice wire tests (proposal 016 hard precondition 2): the
//! `inspection.get` / `inspection.list` read face over the inspection-evidence
//! store (data-role v0.1 draft shapes) and the tasked
//! `inspection.requestRun` write face that drives the Bridge producing
//! operations and publishes the evidence bundle exactly once. Honest
//! absence: unwired faces answer typed `vua.inspection.unavailable`, and a
//! run that observed no receipt at all publishes nothing.

use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use vua_orchestrator::{
    EvidenceStore, InspectionEvidenceStore, PlanDocumentStore, RecipeDocumentStore,
    RecipeRecordStore, UnityBridge, UnityCommand, UnityResult,
};
use vua_provider_host::{
    run_provider_host_with_services, ProductionUseCaseConfig, WarehouseConfig,
};

/// A unique per-test root (temp dir + label + pid + nanos).
fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-inspection-queries-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// One observed Bridge command (identity, wire version, dry-run flag, target).
#[derive(Debug, Clone)]
struct ExecutedCommand {
    operation: String,
    schema_version: u8,
    dry_run: bool,
    avatar_global_object_id: String,
    project_id: String,
}

/// Returns a canned succeeded receipt per producing operation, matching the
/// v1 typed-check / v3 read-only inspection receipt shapes. The v1
/// performance receipt carries the declared estimate metrics in `data`.
fn canned_receipt(command: &UnityCommand) -> Value {
    let base = json!({
        "schemaVersion": command.schema_version,
        "commandId": command.command_id,
        "status": "succeeded",
        "changedPaths": [],
    });
    match command.operation {
        vua_orchestrator::UnityOperation::ValidateAvatar => json!({
            "schemaVersion": base["schemaVersion"],
            "commandId": base["commandId"],
            "status": "succeeded",
            "changedPaths": [],
            "diagnostics": [
                {"code": "validation.passed", "severity": "info", "message": "检查通过。"}
            ],
            "data": {}
        }),
        vua_orchestrator::UnityOperation::AnalyzePerformance => json!({
            "schemaVersion": base["schemaVersion"],
            "commandId": base["commandId"],
            "status": "succeeded",
            "changedPaths": [],
            "diagnostics": [
                {"code": "performance.triangle_estimate_high", "severity": "warning",
                 "message": "三角面较多，建议检查衣装遮挡区域。"}
            ],
            "data": {
                "basis": "local_estimate",
                "triangles": 92410,
                "materialSlots": 12,
                "skinnedMeshRenderers": 6,
                "bones": 248
            }
        }),
        vua_orchestrator::UnityOperation::InspectAvatarReferences => json!({
            "schemaVersion": base["schemaVersion"],
            "commandId": base["commandId"],
            "status": "succeeded",
            "changedPaths": [],
            "diagnostics": [
                {"code": "references.missing_material", "severity": "error",
                 "message": "检测到丢失的材质槽引用。"}
            ],
            "data": {}
        }),
        vua_orchestrator::UnityOperation::InspectLighting => json!({
            "schemaVersion": base["schemaVersion"],
            "commandId": base["commandId"],
            "status": "succeeded",
            "changedPaths": [],
            "diagnostics": [
                {"code": "lighting.realtime_lights_present", "severity": "warning",
                 "message": "存在未烘焙实时光源。"}
            ],
            "data": {}
        }),
        vua_orchestrator::UnityOperation::InspectUploadReadiness => json!({
            "schemaVersion": base["schemaVersion"],
            "commandId": base["commandId"],
            "status": "succeeded",
            "changedPaths": [],
            "diagnostics": [
                {"code": "upload_readiness.clean", "severity": "info", "message": "无前置缺失。"}
            ],
            "data": {}
        }),
        _ => base,
    }
}

struct CannedInspectionBridge {
    executed: Mutex<Vec<ExecutedCommand>>,
}

impl UnityBridge for CannedInspectionBridge {
    fn execute(
        &self,
        _project: &vua_orchestrator::ProjectRef,
        command: &UnityCommand,
    ) -> Result<UnityResult, vua_orchestrator::BridgeError> {
        self.executed
            .lock()
            .expect("executed lock")
            .push(ExecutedCommand {
                operation: serde_json::to_value(command.operation)
                    .unwrap_or(Value::Null)
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                schema_version: command.schema_version,
                dry_run: command.dry_run,
                avatar_global_object_id: command.payload.avatar_global_object_id.clone(),
                project_id: command.project_id.clone(),
            });
        let receipt = canned_receipt(command);
        serde_json::from_value(receipt).map_err(|error| {
            vua_orchestrator::BridgeError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                error,
            ))
        })
    }
}

fn use_case_config(
    root: &std::path::Path,
    bridge: std::sync::Arc<dyn UnityBridge>,
) -> ProductionUseCaseConfig {
    let production_root = root.join("production");
    ProductionUseCaseConfig {
        recipes: std::sync::Arc::new(RecipeDocumentStore::new_with_system_clock(
            production_root.join("recipes"),
        )),
        plans: std::sync::Arc::new(PlanDocumentStore::new(production_root.join("plans"))),
        evidence: std::sync::Arc::new(EvidenceStore::new(production_root.join("evidence"))),
        records: std::sync::Arc::new(RecipeRecordStore::new(production_root.join("records"))),
        inspections: std::sync::Arc::new(InspectionEvidenceStore::new(
            production_root.join("inspections"),
        )),
        editor_version: "2022.3.22f1".to_owned(),
        bridge,
        project_root: root.join("project"),
        editor_selection: vua_orchestrator::EditorSelection::Unavailable {
            reason: vua_orchestrator::EditorSelectionGap::NotDetected,
        },
        handoff: None,
        draft_exporter: None,
    }
}

/// The overlay-face placeholder must never be reached by the read face.
struct NoBridge;

impl UnityBridge for NoBridge {
    fn execute(
        &self,
        _project: &vua_orchestrator::ProjectRef,
        command: &UnityCommand,
    ) -> Result<UnityResult, vua_orchestrator::BridgeError> {
        Err(vua_orchestrator::BridgeError::Io(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!("this fixture bridge must not run ({})", command.command_id),
        )))
    }
}

/// Runs request frames through the real host loop with the given wiring.
fn run_frames(
    database_path: &std::path::Path,
    use_cases: Option<ProductionUseCaseConfig>,
    warehouse: Option<WarehouseConfig>,
    requests: &[Value],
) -> Vec<Value> {
    let mut input = String::new();
    for (index, request) in requests.iter().enumerate() {
        let frame = json!({
            "frameVersion": "0.1",
            "frameId": format!("frame-inspection-{index}"),
            "kind": "request",
            "payload": {
                "contractVersion": "0.1",
                "requestId": format!("req-inspection-{index}"),
                "correlationId": format!("corr-inspection-{index}"),
                "kind": request["kind"],
                "method": request["method"],
                "params": request.get("params").cloned().unwrap_or(json!({})),
            },
        });
        input.push_str(&frame.to_string());
        input.push('\n');
    }
    let mut output = Vec::new();
    run_provider_host_with_services(
        Cursor::new(input),
        &mut output,
        database_path,
        None,
        None,
        warehouse,
        use_cases,
    )
    .expect("frame loop runs");
    String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect()
}

fn query(method: &str, params: Value) -> Value {
    json!({"kind": "query", "method": method, "params": params})
}

fn command(method: &str, params: Value) -> Value {
    json!({"kind": "command", "method": method, "params": params})
}

fn store_for(root: &std::path::Path) -> InspectionEvidenceStore {
    InspectionEvidenceStore::new(root.join("production").join("inspections"))
}

/// Seeds one evidence document directly into the store.
fn seed_evidence(
    root: &std::path::Path,
    inspection_id: &str,
    performed_at: &str,
    overall: &str,
    avatar_ref: &str,
) {
    let document = json!({
        "schemaVersion": "0.1",
        "inspectionId": inspection_id,
        "avatarRef": {"ref": avatar_ref, "label": "Synthetic Avatar"},
        "performedAt": performed_at,
        "bridge": {
            "editorVersion": "2022.3.22f1",
            "bridgeSchemaVersion": 3,
            "operations": [
                {"operation": "validate_avatar", "commandId": "seed-01", "status": "succeeded"}
            ]
        },
        "dimensions": [
            {"kind": "functional", "status": "pass", "basis": "bridge_typed_checks", "checks": []},
            {"kind": "performance", "status": "unavailable", "basis": "none", "checks": []},
            {"kind": "dependencies", "status": "unavailable", "basis": "none", "checks": []},
            {"kind": "lighting", "status": "unavailable", "basis": "none", "checks": []},
            {"kind": "upload_readiness", "status": "unavailable", "basis": "none", "checks": []}
        ],
        "overallStatus": overall,
    });
    store_for(root)
        .publish(inspection_id, &document)
        .expect("seed publishes");
}

fn warehouse_config(root: &std::path::Path) -> WarehouseConfig {
    let bdl = std::sync::Arc::new(
        vua_bdl_store::BdlStore::open(root.join("bdl").join("bdl.db")).expect("BDL opens"),
    );
    WarehouseConfig {
        bdl,
        warehouse_root: root.join("warehouse"),
        global_default: vua_bdl_store::ArtifactMode::UseOriginalUnitypackage,
        executor: None,
        dependencies_queries: None,
    }
}

/// Polls the store until the inspection run's evidence appears.
fn wait_for_documents(root: &std::path::Path, minimum: usize) -> Vec<Value> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let documents = store_for(root).list_documents().expect("store reads");
        if documents.len() >= minimum {
            return documents;
        }
        if Instant::now() > deadline {
            panic!(
                "the inspection run never published; store holds {}",
                documents.len()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn get_answers_typed_unavailable_without_the_use_case_wiring() {
    let root = unique_root("get-unwired");
    let database = root.join("tasks.sqlite");
    let frames = run_frames(
        &database,
        None,
        None,
        &[query(
            "inspection.get",
            json!({"inspectionId": "01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c21"}),
        )],
    );
    let payload = &frames[0]["payload"];
    assert_eq!(
        payload["error"]["code"], "vua.inspection.unavailable",
        "{payload}"
    );
    assert_eq!(payload["error"]["category"], "unavailable");
}

#[test]
fn get_returns_the_stored_document_and_honest_not_found() {
    let root = unique_root("get-read");
    let database = root.join("tasks.sqlite");
    let inspection_id = "01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c21";
    seed_evidence(
        &root,
        inspection_id,
        "2026-09-13T00:20:00Z",
        "warn",
        "warehouse:booth-item-1001",
    );

    let frames = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None,
        &[query(
            "inspection.get",
            json!({"inspectionId": inspection_id}),
        )],
    );
    let payload = &frames[0]["payload"];
    assert!(payload["ok"].as_bool().expect("ok flag"), "{payload}");
    let value = &payload["value"];
    assert_eq!(value["inspectionId"], inspection_id);
    assert_eq!(value["schemaVersion"], "0.1");
    assert_eq!(
        value["inspectionDocument"]["performedAt"],
        "2026-09-13T00:20:00Z"
    );
    assert_eq!(value["inspectionDocument"]["overallStatus"], "warn");
    assert_eq!(
        value["inspectionDocument"]["avatarRef"]["ref"],
        "warehouse:booth-item-1001"
    );

    // An unknown identity is an honest not_found, never a synthesized row.
    let missing = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None,
        &[query(
            "inspection.get",
            json!({"inspectionId": "01982b5a-3f10-7c4e-9d2a-4b8e1f6a7cff"}),
        )],
    );
    assert_eq!(
        missing[0]["payload"]["error"]["code"],
        "vua.inspection.not_found"
    );
}

#[test]
fn get_rejects_unknown_and_missing_params() {
    let root = unique_root("get-params");
    let database = root.join("tasks.sqlite");
    let unknown = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None,
        &[query(
            "inspection.get",
            json!({"inspectionId": "x", "text": "y"}),
        )],
    );
    assert_eq!(
        unknown[0]["payload"]["error"]["code"],
        "vua.inspection.invalid_params"
    );
    let missing = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None,
        &[query("inspection.get", json!({}))],
    );
    assert_eq!(
        missing[0]["payload"]["error"]["code"],
        "vua.inspection.invalid_params"
    );
}

#[test]
fn list_serves_the_honest_empty_state() {
    let root = unique_root("list-empty");
    let database = root.join("tasks.sqlite");
    let frames = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None,
        &[query("inspection.list", json!({}))],
    );
    let value = &frames[0]["payload"]["value"];
    assert!(frames[0]["payload"]["ok"].as_bool().expect("ok flag"));
    assert_eq!(value["total"], 0);
    assert_eq!(value["entries"], json!([]));
    assert_eq!(value["schemaVersion"], "0.1");
}

#[test]
fn list_orders_newest_first_and_applies_the_closed_filters() {
    let root = unique_root("list-filters");
    let database = root.join("tasks.sqlite");
    seed_evidence(
        &root,
        "01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c21",
        "2026-09-13T01:00:00Z",
        "fail",
        "warehouse:item-a",
    );
    seed_evidence(
        &root,
        "01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c22",
        "2026-09-13T03:00:00Z",
        "warn",
        "warehouse:item-b",
    );
    seed_evidence(
        &root,
        "01982b5a-3f10-7c4e-9d2a-4b8e1f6a7c23",
        "2026-09-13T02:00:00Z",
        "pass",
        "warehouse:item-a",
    );

    let frames = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None,
        &[query("inspection.list", json!({}))],
    );
    let value = &frames[0]["payload"]["value"];
    assert_eq!(value["total"], 3);
    let entries = value["entries"].as_array().expect("entries array");
    // Newest first: 03:00, then 02:00, then 01:00 — storage order is
    // irrelevant, the read face owns the declared ordering.
    assert_eq!(entries[0]["performedAt"], "2026-09-13T03:00:00Z");
    assert_eq!(entries[1]["performedAt"], "2026-09-13T02:00:00Z");
    assert_eq!(entries[2]["performedAt"], "2026-09-13T01:00:00Z");
    // Identity summary rows: no dimensions, no checks leak into the list.
    assert!(entries[0]["avatarRef"].is_object());
    assert!(entries[0].get("dimensions").is_none());
    assert_eq!(entries[0]["overallStatus"], "warn");

    // avatarRef exact-match filter.
    let filtered = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None,
        &[query(
            "inspection.list",
            json!({"avatarRef": "warehouse:item-a"}),
        )],
    );
    let value = &filtered[0]["payload"]["value"];
    assert_eq!(value["total"], 2);
    let entries = value["entries"].as_array().unwrap();
    assert_eq!(entries[0]["avatarRef"]["ref"], "warehouse:item-a");

    // overallStatus closed-set filter.
    let failed = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None,
        &[query("inspection.list", json!({"overallStatus": "fail"}))],
    );
    let value = &failed[0]["payload"]["value"];
    assert_eq!(value["total"], 1);
    assert_eq!(value["entries"][0]["overallStatus"], "fail");

    // Bounded paging (limit + offset).
    let paged = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None,
        &[query("inspection.list", json!({"limit": 1, "offset": 1}))],
    );
    let value = &paged[0]["payload"]["value"];
    assert_eq!(value["total"], 3, "total counts every match, not the page");
    assert_eq!(value["entries"].as_array().unwrap().len(), 1);
    assert_eq!(value["entries"][0]["performedAt"], "2026-09-13T02:00:00Z");
}

#[test]
fn list_rejects_unknown_param_and_out_of_vocabulary_status() {
    let root = unique_root("list-params");
    let database = root.join("tasks.sqlite");
    let unknown = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None,
        &[query("inspection.list", json!({"text": "fuzzy"}))],
    );
    assert_eq!(
        unknown[0]["payload"]["error"]["code"],
        "vua.inspection.invalid_params"
    );
    let bad_status = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None,
        &[query(
            "inspection.list",
            json!({"overallStatus": "unavailable"}),
        )],
    );
    assert_eq!(
        bad_status[0]["payload"]["error"]["code"],
        "vua.inspection.invalid_params"
    );
}

#[test]
fn request_run_drives_the_five_producing_operations_and_publishes_once() {
    let root = unique_root("run-happy");
    let database = root.join("tasks.sqlite");
    let bridge: std::sync::Arc<CannedInspectionBridge> =
        std::sync::Arc::new(CannedInspectionBridge {
            executed: Mutex::new(Vec::new()),
        });
    let executed_handle = std::sync::Arc::clone(&bridge);
    let config = use_case_config(&root, bridge);

    let frames = run_frames(
        &database,
        Some(config),
        Some(warehouse_config(&root)),
        &[command(
            "inspection.requestRun",
            json!({
                "avatarGlobalObjectId": "scene:0x1",
                "avatarRef": {"ref": "warehouse:booth-item-1001", "label": "Synthetic Avatar A"}
            }),
        )],
    );
    let payload = &frames[0]["payload"];
    assert!(payload["ok"].as_bool().expect("acceptance ok"), "{payload}");
    let accepted = &payload["value"];
    assert_eq!(accepted["operation"], "inspection.requestRun");
    assert!(
        accepted["taskId"].is_string(),
        "the acceptance carries the task identity"
    );
    assert_eq!(accepted["schemaVersion"], "0.1");

    // The worker runs detached: poll until the evidence bundle publishes.
    let documents = wait_for_documents(&root, 1);
    assert_eq!(
        documents.len(),
        1,
        "publish is exactly-once per run identity"
    );
    let document = &documents[0];

    // The evidence body: five dimensions, transcription and aggregation.
    assert_eq!(document["schemaVersion"], "0.1");
    assert_eq!(document["avatarRef"]["ref"], "warehouse:booth-item-1001");
    assert_eq!(document["bridge"]["editorVersion"], "2022.3.22f1");
    assert_eq!(document["bridge"]["bridgeSchemaVersion"], 3);
    let operations = document["bridge"]["operations"]
        .as_array()
        .expect("operations");
    assert_eq!(operations.len(), 5);
    let names: Vec<&str> = operations
        .iter()
        .map(|o| o["operation"].as_str().expect("name"))
        .collect();
    assert_eq!(
        names,
        vec![
            "validate_avatar",
            "analyze_performance",
            "inspect_avatar_references",
            "inspect_lighting",
            "inspect_upload_readiness"
        ]
    );
    assert!(operations.iter().all(|o| o["status"] == "succeeded"));

    let dimensions = document["dimensions"].as_array().expect("dimensions");
    assert_eq!(dimensions.len(), 5);
    let by_kind = |kind: &str| {
        dimensions
            .iter()
            .find(|d| d["kind"] == kind)
            .unwrap_or_else(|| panic!("dimension {kind} present"))
    };
    // functional: v1 typed checks transcribed verbatim.
    assert_eq!(by_kind("functional")["status"], "pass");
    assert_eq!(by_kind("functional")["basis"], "bridge_typed_checks");
    // performance: local estimate basis, warn from the transcribed warning,
    // metrics transcribed from the declared data scalars.
    assert_eq!(by_kind("performance")["status"], "warn");
    assert_eq!(by_kind("performance")["basis"], "bridge_local_estimate");
    assert_eq!(
        by_kind("performance")["checks"][0]["metrics"]["triangles"],
        92410
    );
    // dependencies: the missing material error fails the dimension.
    assert_eq!(by_kind("dependencies")["status"], "fail");
    assert_eq!(
        by_kind("dependencies")["checks"][0]["code"],
        "references.missing_material"
    );
    // lighting: warning → warn.
    assert_eq!(by_kind("lighting")["status"], "warn");
    // upload_readiness: clean info → pass.
    assert_eq!(by_kind("upload_readiness")["status"], "pass");
    // Aggregation: fail > warn (incl. unavailable) > pass.
    assert_eq!(document["overallStatus"], "fail");
    // The honest note names the estimate discipline.
    let notes = document["notes"].as_str().expect("notes");
    assert!(notes.contains("本地结构估算"), "{notes}");

    // The driven commands: five, read-only, correct wire versions.
    let executed = executed_handle.executed.lock().expect("executed lock");
    assert_eq!(executed.len(), 5);
    let by_operation = |name: &str| {
        executed
            .iter()
            .find(|c| c.operation == name)
            .unwrap_or_else(|| panic!("command {name} executed"))
    };
    assert_eq!(by_operation("validate_avatar").schema_version, 1);
    assert_eq!(by_operation("analyze_performance").schema_version, 1);
    assert_eq!(by_operation("inspect_avatar_references").schema_version, 3);
    assert_eq!(by_operation("inspect_lighting").schema_version, 3);
    assert_eq!(by_operation("inspect_upload_readiness").schema_version, 3);
    assert!(
        executed.iter().all(|c| c.dry_run),
        "every inspection command is dry-run"
    );
    assert!(executed
        .iter()
        .all(|c| c.avatar_global_object_id == "scene:0x1"));
    // The command project id is the run-scope correlation identity.
    assert!(executed
        .iter()
        .all(|c| c.project_id.starts_with("inspection-")));
}

#[test]
fn request_run_without_any_receipt_publishes_nothing() {
    let root = unique_root("run-bridge-down");
    let database = root.join("tasks.sqlite");
    let config = use_case_config(&root, std::sync::Arc::new(NoBridge));

    let frames = run_frames(
        &database,
        Some(config),
        Some(warehouse_config(&root)),
        &[command(
            "inspection.requestRun",
            json!({
                "avatarGlobalObjectId": "scene:0x1",
                "avatarRef": {"ref": "warehouse:booth-item-1001"}
            }),
        )],
    );
    assert!(frames[0]["payload"]["ok"].as_bool().expect("acceptance ok"));

    // The Bridge produced no receipt at all: the run fails honestly and the
    // store stays empty — a zero-operation evidence bundle would be a
    // fabricated document. Poll a bounded window to observe the absence.
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(
        store_for(&root)
            .list_documents()
            .expect("store reads")
            .len(),
        0,
        "no receipts → no evidence document"
    );
}

#[test]
fn request_run_without_the_task_authority_answers_unavailable() {
    let root = unique_root("run-unwired");
    let database = root.join("tasks.sqlite");
    let frames = run_frames(
        &database,
        Some(use_case_config(&root, std::sync::Arc::new(NoBridge))),
        None, // no warehouse wiring → no shared task authority
        &[command(
            "inspection.requestRun",
            json!({
                "avatarGlobalObjectId": "scene:0x1",
                "avatarRef": {"ref": "warehouse:booth-item-1001"}
            }),
        )],
    );
    let payload = &frames[0]["payload"];
    assert_eq!(
        payload["error"]["code"], "vua.inspection.unavailable",
        "{payload}"
    );
    assert_eq!(payload["error"]["category"], "unavailable");
}

#[test]
fn request_run_rejects_invalid_params() {
    let root = unique_root("run-params");
    let database = root.join("tasks.sqlite");
    let config = use_case_config(&root, std::sync::Arc::new(NoBridge));

    // Unknown param.
    let unknown = run_frames(
        &database,
        Some(config),
        Some(warehouse_config(&root)),
        &[command(
            "inspection.requestRun",
            json!({
                "avatarGlobalObjectId": "scene:0x1",
                "avatarRef": {"ref": "warehouse:booth-item-1001"},
                "dimensions": ["functional"]
            }),
        )],
    );
    assert_eq!(
        unknown[0]["payload"]["error"]["code"],
        "vua.inspection.invalid_params"
    );

    // Missing avatarRef.
    let config = use_case_config(&root, std::sync::Arc::new(NoBridge));
    let missing = run_frames(
        &database,
        Some(config),
        Some(warehouse_config(&root)),
        &[command(
            "inspection.requestRun",
            json!({"avatarGlobalObjectId": "scene:0x1"}),
        )],
    );
    assert_eq!(
        missing[0]["payload"]["error"]["code"],
        "vua.inspection.invalid_params"
    );

    // avatarRef without ref.
    let config = use_case_config(&root, std::sync::Arc::new(NoBridge));
    let empty_ref = run_frames(
        &database,
        Some(config),
        Some(warehouse_config(&root)),
        &[command(
            "inspection.requestRun",
            json!({"avatarGlobalObjectId": "scene:0x1", "avatarRef": {"label": "no ref"}}),
        )],
    );
    assert_eq!(
        empty_ref[0]["payload"]["error"]["code"],
        "vua.inspection.invalid_params"
    );
}
