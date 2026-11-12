//! `release.openForHandoff` + `release.openForInspection` wire tests. The
//! consumer consumes the frozen method schemas from
//! `schemas/release-handoff/v0.2`; changing the handoff face without this
//! consumer fails here first. The vectors drive JSON-Schema validation
//! directly, and the frame loop pins the honest absence of an unwired
//! route. The v0.2 batch (U19 user ruling 2026-09-21, BOARD row =
//! normative source) adds the backend-authoritative record-state gate on
//! the handoff entry (typed rejections with the record's original state as
//! the `state` param) and the independent state-gate-free inspection
//! entry — every build-record status enum value is exercised THROUGH the
//! real host loop (bypassing any UI), plus the missing and unparseable
//! states. Real-machine evidence stays out (ruling-15 local-first; W25
//! owns the real-editor walkthrough); zero end-to-end claims.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use jsonschema::Validator;
use serde_json::{json, Value};
use vua_provider_host::run_provider_host_full;

/// The LIVE face under test (v0.2: the U19 gate + the inspection entry).
fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/release-handoff/v0.2")
}

/// The frozen v0.1 family documents (single-method generation, 2026-09-16)
/// stay byte-frozen; this pin keeps them compiling as history.
fn schemas_dir_v01() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/release-handoff/v0.1")
}

fn read_json_at(dir: &Path, relative: &str) -> Value {
    let bytes = fs::read(dir.join(relative)).expect("schema file must exist");
    serde_json::from_slice(&bytes).expect("schema file must be valid JSON")
}

/// Compiles one $defs entry of a method schema with the full $defs
/// re-attached, so nested $ref targets resolve (the editor-verify wire-test
/// compilation shape).
fn def_validator(method_file: &str, def_name: &str) -> Validator {
    let schema = read_json_at(&schemas_dir().join("methods"), method_file);
    let defs = schema
        .get("$defs")
        .expect("method schema has $defs")
        .clone();
    let sub = defs.get(def_name).expect("sub schema exists").clone();
    let combined = json!({ "allOf": [sub], "$defs": defs });
    jsonschema::validator_for(&combined).expect("sub schema compiles")
}

fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-release-handoff-wire-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// Sends one command frame through the real host loop and returns the
/// response payload.
fn run_frame(database: &Path, method: &str, frame_label: &str, params: Value) -> Value {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": format!("frame-{frame_label}"),
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": format!("req-{frame_label}"),
            "correlationId": format!("corr-{frame_label}"),
            "kind": "command",
            "commandId": format!("command-{frame_label}"),
            "method": method,
            "params": params,
        },
    });
    let mut output = Vec::new();
    run_provider_host_full(
        Cursor::new(format!("{frame}\n")),
        &mut output,
        database,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("frame loop runs");
    let frames: Vec<Value> = String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect();
    frames[0]["payload"].clone()
}

// ---- Frozen vocabulary pins (file level) ----

#[test]
fn frozen_v02_method_schemas_compile() {
    for method_file in [
        "release-open-for-handoff.schema.json",
        "release-open-for-inspection.schema.json",
    ] {
        let schema = read_json_at(&schemas_dir().join("methods"), method_file);
        jsonschema::validator_for(&schema)
            .unwrap_or_else(|e| panic!("{method_file} must compile: {e}"));
    }
}

#[test]
fn frozen_v01_family_documents_still_compile_as_history() {
    let schema = read_json_at(
        &schemas_dir_v01().join("methods"),
        "release-open-for-handoff.schema.json",
    );
    jsonschema::validator_for(&schema).expect("frozen v0.1 method schema must compile");
}

#[test]
fn v02_valid_vectors_pass_their_frozen_defs() {
    // 正例 1:params 闭集 {buildId}(两方法同形)
    let params = read_json_at(
        &schemas_dir().join("examples"),
        "release-open-for-handoff.request.json",
    );
    let params_validator = def_validator(
        "release-open-for-handoff.schema.json",
        "release-openForHandoffParams",
    );
    assert!(
        params_validator.is_valid(&params),
        "params vector must validate"
    );
    let inspection_params = read_json_at(
        &schemas_dir().join("examples"),
        "release-open-for-inspection.request.json",
    );
    let inspection_params_validator = def_validator(
        "release-open-for-inspection.schema.json",
        "release-openForInspectionParams",
    );
    assert!(
        inspection_params_validator.is_valid(&inspection_params),
        "inspection params vector must validate"
    );

    // 正例 2:tasked 受理回执(inspection.requestRun 形状先例)
    let accepted = read_json_at(
        &schemas_dir().join("examples"),
        "release-open-for-handoff.accepted.json",
    );
    let accepted_validator = def_validator(
        "release-open-for-handoff.schema.json",
        "release-openForHandoffAccepted",
    );
    assert!(
        accepted_validator.is_valid(&accepted),
        "accepted vector must validate"
    );
    let inspection_accepted = read_json_at(
        &schemas_dir().join("examples"),
        "release-open-for-inspection.accepted.json",
    );
    let inspection_accepted_validator = def_validator(
        "release-open-for-inspection.schema.json",
        "release-openForInspectionAccepted",
    );
    assert!(
        inspection_accepted_validator.is_valid(&inspection_accepted),
        "inspection accepted vector must validate"
    );

    // 正例 3:交接事实文档(无上传状态字段——形状即诚实纪律)
    let fact = read_json_at(
        &schemas_dir().join("examples"),
        "release-open-for-handoff.fact.json",
    );
    let fact_validator = def_validator(
        "release-open-for-handoff.schema.json",
        "release-openForHandoffFact",
    );
    assert!(fact_validator.is_valid(&fact), "fact vector must validate");

    // 正例 4:检视事实文档(显式 operation 词面——绝不误读为交接完成)
    let inspection_fact = read_json_at(
        &schemas_dir().join("examples"),
        "release-open-for-inspection.fact.json",
    );
    let inspection_fact_validator = def_validator(
        "release-open-for-inspection.schema.json",
        "release-openForInspectionFact",
    );
    assert!(
        inspection_fact_validator.is_valid(&inspection_fact),
        "inspection fact vector must validate"
    );

    // 正例 5:record_state_blocked 的 params(state=记录状态原值)
    let blocked_params = read_json_at(
        &schemas_dir().join("examples"),
        "release-open-for-handoff.blocked-error.params.json",
    );
    let blocked_validator = def_validator(
        "release-open-for-handoff.schema.json",
        "recordStateBlockedParams",
    );
    assert!(
        blocked_validator.is_valid(&blocked_params),
        "blocked params vector must validate"
    );
}

#[test]
fn v02_invalid_vectors_are_refused_by_their_frozen_defs() {
    // 负例 1:params 闭集外键(工程身份字段不进 params——单一事实源)
    let extra_param = read_json_at(
        &schemas_dir().join("examples"),
        "invalid-release-open-for-handoff-unknown-param.request.json",
    );
    let params_validator = def_validator(
        "release-open-for-handoff.schema.json",
        "release-openForHandoffParams",
    );
    assert!(
        !params_validator.is_valid(&extra_param),
        "closed-set violation must be refused"
    );

    // 负例 2:交接事实携带上传状态字段——诚实纪律 1/2 由形状钉死
    // (上传在官方 SDK 中完成,绝非 VUA 可猜事实)
    let upload_state = read_json_at(
        &schemas_dir().join("examples"),
        "invalid-release-open-for-handoff-upload-state.fact.json",
    );
    let fact_validator = def_validator(
        "release-open-for-handoff.schema.json",
        "release-openForHandoffFact",
    );
    assert!(
        !fact_validator.is_valid(&upload_state),
        "an upload-status field must never enter the handoff fact"
    );

    // 负例 3:错误码闭集外(上传类错误码永不进入本词表——任务运行期失败
    // 走任务面九态)
    let unknown_code = read_json_at(
        &schemas_dir().join("examples"),
        "invalid-release-open-for-handoff-unknown-error-code.json",
    );
    let code_validator = def_validator("release-open-for-handoff.schema.json", "errorCode");
    assert!(
        !code_validator.is_valid(&unknown_code),
        "out-of-set error codes must be refused"
    );

    // 负例 4:blocked params 缺 state——状态原值是词面依据,不可缺席
    let missing_state = read_json_at(
        &schemas_dir().join("examples"),
        "invalid-release-open-for-handoff-blocked-params-missing-state.json",
    );
    let blocked_validator = def_validator(
        "release-open-for-handoff.schema.json",
        "recordStateBlockedParams",
    );
    assert!(
        !blocked_validator.is_valid(&missing_state),
        "blocked params without the state value must be refused"
    );

    // 负例 5:检视事实携带上传状态字段
    let inspection_upload = read_json_at(
        &schemas_dir().join("examples"),
        "invalid-release-open-for-inspection-upload-state.fact.json",
    );
    let inspection_fact_validator = def_validator(
        "release-open-for-inspection.schema.json",
        "release-openForInspectionFact",
    );
    assert!(
        !inspection_fact_validator.is_valid(&inspection_upload),
        "an upload-status field must never enter the inspection fact"
    );

    // 负例 6:检视事实宣称交接操作——词面纪律由形状钉死(U19:打开检查
    // 绝不宣称交接完成)
    let handoff_wording = read_json_at(
        &schemas_dir().join("examples"),
        "invalid-release-open-for-inspection-handoff-wording.fact.json",
    );
    assert!(
        !inspection_fact_validator.is_valid(&handoff_wording),
        "an inspection fact must never carry the handoff operation wording"
    );
}

// ---- Unwired-route honest absence ----

#[test]
fn unwired_route_answers_the_honest_absence_never_a_fabricated_acceptance() {
    // 实现域(产线进程/窗口 port＋核心 use case)未接线:诚实缺席——
    // 绝不伪造受理回执/任务快照/交接事实
    let root = unique_root("absence");
    let payload = run_frame(
        &root.join("host.sqlite"),
        "release.openForHandoff",
        "release-handoff",
        json!({ "buildId": "019513e7-7a2b-7cd1-9f3a-4d8e21b90c99" }),
    );
    fs::remove_dir_all(&root).ok();

    assert_eq!(payload["ok"], json!(false));
    let error = &payload["error"];
    assert_eq!(error["code"], json!("vua.release_handoff.unavailable"));
    assert_eq!(error["category"], json!("unavailable"));
    assert_eq!(
        error["messageKey"],
        json!("errors.releaseHandoff.unavailable")
    );
    // 缺席响应绝不携带任务面事实(taskId/task 字段缺位——伪造受理即撒谎)
    assert!(
        payload.get("task").is_none(),
        "absence must not fabricate a task snapshot"
    );
    assert!(
        payload.get("schemaVersion").is_none(),
        "absence must not fabricate an acceptance"
    );
}

#[test]
fn params_closed_set_violations_answer_validation_not_absence() {
    let root = unique_root("params");
    let database = root.join("host.sqlite");

    // 词表外键(projectPath):validation,不冒充缺席
    let payload = run_frame(
        &database,
        "release.openForHandoff",
        "release-handoff",
        json!({ "buildId": "019513e7-7a2b-7cd1-9f3a-4d8e21b90c99", "projectPath": "C:/Projects/X" }),
    );
    assert_eq!(
        payload["error"]["code"],
        json!("vua.release_handoff.invalid_params")
    );
    assert_eq!(payload["error"]["category"], json!("validation"));

    // 缺 buildId
    let payload = run_frame(
        &database,
        "release.openForHandoff",
        "release-handoff",
        json!({}),
    );
    assert_eq!(
        payload["error"]["code"],
        json!("vua.release_handoff.invalid_params")
    );

    // 空 buildId(minLength 1)
    let payload = run_frame(
        &database,
        "release.openForHandoff",
        "release-handoff",
        json!({ "buildId": "" }),
    );
    assert_eq!(
        payload["error"]["code"],
        json!("vua.release_handoff.invalid_params")
    );

    // buildId 非字符串
    let payload = run_frame(
        &database,
        "release.openForHandoff",
        "release-handoff",
        json!({ "buildId": 17 }),
    );
    assert_eq!(
        payload["error"]["code"],
        json!("vua.release_handoff.invalid_params")
    );

    fs::remove_dir_all(&root).ok();
}

// ---- Implementation-domain wiring (023 follow-up slice 2: the core use
// case + the frozen `ReleaseHandoffPort` trait, driven here through a fake
// port — ruling-15 local-first; the real process/window adapter belongs to
// the production-domain slice and real-machine evidence belongs to W25) ----

use std::sync::Arc;
use std::time::{Duration, Instant};

use vua_bdl_store::{ArtifactMode, BdlStore};
use vua_orchestrator::{
    EditorSelection, HandoffLaunch, HandoffOutcome, HandoffPortError, ReleaseHandoffPort,
    UnityBridge, UnityCommand, UnityResult,
};
use vua_project_manager::{EditorPathIdentity, EditorPathVerdict};
use vua_provider_host::{ProductionUseCaseConfig, WarehouseConfig};

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

/// The fake production-domain port: it returns the preprogrammed outcome
/// and records the launch it received, so the tests can pin that the route
/// hands the RESOLVED identity (ruling 5) to the port and that the port
/// outcome alone decides the task's terminal state (ruling 3).
struct FakePort {
    outcome: Result<HandoffOutcome, HandoffPortError>,
    seen: std::sync::Mutex<Option<HandoffLaunch>>,
}

impl FakePort {
    fn arriving() -> Self {
        Self {
            outcome: Ok(HandoffOutcome::HandshakeArrived),
            seen: std::sync::Mutex::new(None),
        }
    }
    fn timing_out() -> Self {
        Self {
            outcome: Ok(HandoffOutcome::HandshakeTimeout),
            seen: std::sync::Mutex::new(None),
        }
    }
    fn launch_failed() -> Self {
        Self {
            outcome: Err(HandoffPortError {
                detail: "editor executable vanished".to_owned(),
            }),
            seen: std::sync::Mutex::new(None),
        }
    }
    fn observed_launch(&self) -> HandoffLaunch {
        self.seen
            .lock()
            .expect("lock")
            .clone()
            .expect("the port must have been called")
    }
    fn was_never_called(&self) -> bool {
        self.seen.lock().expect("lock").is_none()
    }
}

impl ReleaseHandoffPort for FakePort {
    fn open_for_handoff(&self, launch: &HandoffLaunch) -> Result<HandoffOutcome, HandoffPortError> {
        *self.seen.lock().expect("lock") = Some(launch.clone());
        self.outcome.clone()
    }
}

const BUILD_ID: &str = "019513e7-7a2b-7cd1-9f3a-4d8e21b90c99";
const PROJECT_ID: &str = "proj-synthetic-avatar-a";

/// Seeds a build record with an explicit `status` value (any JSON value —
/// including a missing status, a wrong type, or an out-of-enum word: the
/// record store publishes documents verbatim, which is exactly how a
/// drifted/legacy record looks).
fn seed_build_record_with_status(config: &ProductionUseCaseConfig, status: Value) {
    let mut record = json!({
        "schemaVersion": "0.3",
        "buildId": BUILD_ID,
        "projectId": PROJECT_ID,
        "unityEditorVersion": "2022.3.22f1",
    });
    if !status.is_null() {
        record["status"] = status;
    }
    config
        .records
        .publish(BUILD_ID, &record)
        .expect("record seeds");
}

fn seed_build_record(config: &ProductionUseCaseConfig, editor_version: &str) {
    let mut record = json!({
        "schemaVersion": "0.3",
        "buildId": BUILD_ID,
        "projectId": PROJECT_ID,
        "unityEditorVersion": editor_version,
        "status": "succeeded",
    });
    if editor_version.is_empty() {
        record
            .as_object_mut()
            .expect("object")
            .remove("unityEditorVersion");
    }
    config
        .records
        .publish(BUILD_ID, &record)
        .expect("record seeds");
}

fn handoff_config(
    root: &std::path::Path,
    port: Option<Arc<dyn ReleaseHandoffPort>>,
    selection: EditorSelection,
) -> ProductionUseCaseConfig {
    let production_root = root.join("production");
    ProductionUseCaseConfig {
        recipes: Arc::new(
            vua_orchestrator::RecipeDocumentStore::new_with_system_clock(
                production_root.join("recipes"),
            ),
        ),
        plans: Arc::new(vua_orchestrator::PlanDocumentStore::new(
            production_root.join("plans"),
        )),
        evidence: Arc::new(vua_orchestrator::EvidenceStore::new(
            production_root.join("evidence"),
        )),
        records: Arc::new(vua_orchestrator::RecipeRecordStore::new(
            production_root.join("records"),
        )),
        inspections: Arc::new(vua_orchestrator::InspectionEvidenceStore::new(
            production_root.join("inspections"),
        )),
        editor_version: "2022.3.22f1".to_owned(),
        bridge: Arc::new(NoBridge),
        project_root: root.join("project"),
        editor_selection: selection,
        handoff: port,
        draft_exporter: None,
    }
}

fn auto_selected(version: &str) -> EditorSelection {
    EditorSelection::AutoSelected {
        editor: vua_orchestrator::InstalledUnityEditor {
            parsed: vua_orchestrator::parse_editor_version(version)
                .expect("fixture version parses"),
            path: std::path::PathBuf::from(format!("C:/Unity/{version}/Editor/Unity.exe")),
        },
    }
}

/// Runs one command frame through the real host loop with the use-case
/// face wired (the given config), returning the response payload.
fn run_frame_wired(
    database: &std::path::Path,
    config: &ProductionUseCaseConfig,
    method: &str,
    label: &str,
    params: Value,
) -> Value {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": format!("frame-{label}"),
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": format!("req-{label}"),
            "correlationId": format!("corr-{label}"),
            "kind": "command",
            "commandId": format!("command-{label}"),
            "method": method,
            "params": params,
        },
    });
    // The task authority rides the warehouse wiring (the same shared
    // SQLite runtime the inspection tasked face consumes).
    let bdl_root = database.parent().unwrap_or(database).join("bdl");
    let warehouse = WarehouseConfig {
        bdl: Arc::new(BdlStore::open(bdl_root.join("bdl.db")).expect("BDL opens")),
        warehouse_root: database.parent().unwrap_or(database).join("warehouse"),
        global_default: ArtifactMode::UseOriginalUnitypackage,
        executor: None,
        dependencies_queries: None,
    };
    let mut output = Vec::new();
    run_provider_host_full(
        Cursor::new(format!("{frame}\n")),
        &mut output,
        database,
        None,
        None,
        Some(warehouse),
        Some(config.clone()),
        None,
        None,
        None,
        None,
    )
    .expect("frame loop runs");
    let frames: Vec<Value> = String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect();
    frames[0]["payload"].clone()
}

/// Polls the store until the task reaches a terminal state (the task
/// snapshot wire face projects this single stored authority).
fn wait_terminal(database: &std::path::Path, task_id: &str) -> vua_orchestrator::StoredTask {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let store = vua_orchestrator::SqliteTaskStore::open(database).expect("store opens");
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
fn wired_route_accepts_the_task_and_the_succeeded_snapshot_carries_the_fact() {
    let root = unique_root("wired");
    let database = root.join("host.sqlite");
    let port = Arc::new(FakePort::arriving());
    let selection = auto_selected("2022.3.22f1");
    let config = handoff_config(&root, Some(port.clone()), selection);
    seed_build_record(&config, "2022.3.22f1");

    let payload = run_frame_wired(
        &database,
        &config,
        "release.openForHandoff",
        "release-handoff-wired",
        json!({ "buildId": BUILD_ID }),
    );
    // 受理回执照 inspection.requestRun 形状（冻结批文本）
    assert_eq!(payload["ok"], json!(true));
    let value = &payload["value"];
    assert_eq!(value["schemaVersion"], json!("0.2"));
    assert_eq!(value["operation"], json!("release.openForHandoff"));
    let task_id = value["taskId"]
        .as_str()
        .expect("acceptance carries a taskId")
        .to_owned();
    assert_eq!(value["correlationId"], json!("corr-release-handoff-wired"));

    // The task reaches Succeeded ONLY via the handshake (ruling 3); the
    // succeeded snapshot's result is the frozen five-key fact.
    let task = wait_terminal(&database, &task_id);
    assert!(matches!(task.state, vua_orchestrator::TaskState::Succeeded));
    let result = task.result.expect("a succeeded handoff refluxes its fact");
    let fact_validator = def_validator(
        "release-open-for-handoff.schema.json",
        "release-openForHandoffFact",
    );
    assert!(
        fact_validator.is_valid(&result),
        "the refluxed fact must satisfy the frozen fact def: {result}"
    );
    assert_eq!(result["schemaVersion"], json!("0.2"));
    assert_eq!(result["buildId"], json!(BUILD_ID));
    assert_eq!(result["projectId"], json!(PROJECT_ID));
    assert_eq!(
        result["editor"]["exePath"],
        json!("C:/Unity/2022.3.22f1/Editor/Unity.exe")
    );
    assert_eq!(result["editor"]["version"], json!("2022.3.22f1"));
    assert!(
        result["occurredAt"].as_str().is_some(),
        "occurredAt is a fact key"
    );
    let fact_object = result.as_object().expect("fact is an object");
    assert_eq!(fact_object.len(), 5, "five-key closed set");
    assert!(fact_object.get("uploadStatus").is_none());

    // The port received the RESOLVED identity and the trusted-side project
    // root (never a wire fact) — ruling 5 tier 2 + boundary 6.
    let launch = port.observed_launch();
    assert_eq!(launch.build_id, BUILD_ID);
    assert_eq!(launch.project_id, PROJECT_ID);
    assert_eq!(launch.editor_version, "2022.3.22f1");
    assert_eq!(
        launch.editor_exe,
        std::path::PathBuf::from("C:/Unity/2022.3.22f1/Editor/Unity.exe")
    );

    fs::remove_dir_all(&root).ok();
}

#[test]
fn handshake_timeout_fails_the_task_honestly_with_no_result() {
    let root = unique_root("timeout");
    let database = root.join("host.sqlite");
    let port = Arc::new(FakePort::timing_out());
    let config = handoff_config(&root, Some(port.clone()), auto_selected("2022.3.22f1"));
    seed_build_record(&config, "2022.3.22f1");

    let payload = run_frame_wired(
        &database,
        &config,
        "release.openForHandoff",
        "release-handoff-timeout",
        json!({ "buildId": BUILD_ID }),
    );
    let task_id = payload["value"]["taskId"]
        .as_str()
        .expect("accepted")
        .to_owned();
    let task = wait_terminal(&database, &task_id);
    // 裁决③:超时如实失败,绝不伪装成功;快照绝不携带交接事实
    assert!(matches!(task.state, vua_orchestrator::TaskState::Failed));
    let error = task.error.expect("honest failure carries its error");
    assert_eq!(error.code, "vua.task.timeout");
    assert!(task.result.is_none(), "a failed task never refluxes a fact");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn port_launch_failure_fails_the_task_as_an_external_failure() {
    let root = unique_root("launchfailed");
    let database = root.join("host.sqlite");
    let port = Arc::new(FakePort::launch_failed());
    let config = handoff_config(&root, Some(port.clone()), auto_selected("2022.3.22f1"));
    seed_build_record(&config, "2022.3.22f1");

    let payload = run_frame_wired(
        &database,
        &config,
        "release.openForHandoff",
        "release-handoff-launchfailed",
        json!({ "buildId": BUILD_ID }),
    );
    let task_id = payload["value"]["taskId"]
        .as_str()
        .expect("accepted")
        .to_owned();
    let task = wait_terminal(&database, &task_id);
    assert!(matches!(task.state, vua_orchestrator::TaskState::Failed));
    let error = task.error.expect("honest failure carries its error");
    // 任务运行期失败走任务面/执行族码,不进 vua.release_handoff.* 闭集
    assert_eq!(error.code, "vua.job.handoff_launch_failed");
    assert!(task.result.is_none());

    fs::remove_dir_all(&root).ok();
}

#[test]
fn unknown_build_id_answers_build_unknown_without_accepting_a_task() {
    let root = unique_root("buildunknown");
    let database = root.join("host.sqlite");
    let port = Arc::new(FakePort::arriving());
    let config = handoff_config(&root, Some(port.clone()), auto_selected("2022.3.22f1"));

    let payload = run_frame_wired(
        &database,
        &config,
        "release.openForHandoff",
        "release-handoff-buildunknown",
        json!({ "buildId": BUILD_ID }),
    );
    assert_eq!(payload["ok"], json!(false));
    assert_eq!(
        payload["error"]["code"],
        json!("vua.release_handoff.build_unknown")
    );
    assert_eq!(payload["error"]["category"], json!("validation"));
    // 受理期校验失败不受理任务——库里零任务
    let store = vua_orchestrator::SqliteTaskStore::open(&database).expect("store opens");
    assert!(store.tasks().expect("tasks readable").is_empty());
    assert!(port.was_never_called(), "the port is never reached");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn record_version_mismatch_answers_editor_unresolved() {
    let root = unique_root("unresolved");
    let database = root.join("host.sqlite");
    let port = Arc::new(FakePort::arriving());
    let config = handoff_config(&root, Some(port.clone()), auto_selected("2021.3.5f1"));
    seed_build_record(&config, "2022.3.22f1");

    let payload = run_frame_wired(
        &database,
        &config,
        "release.openForHandoff",
        "release-handoff-unresolved",
        json!({ "buildId": BUILD_ID }),
    );
    assert_eq!(payload["ok"], json!(false));
    assert_eq!(
        payload["error"]["code"],
        json!("vua.release_handoff.editor_unresolved")
    );
    // 依赖类:身份解析依赖环境事实(协议本冻结错误码表 category=dependency)
    assert_eq!(payload["error"]["category"], json!("dependency"));
    let store = vua_orchestrator::SqliteTaskStore::open(&database).expect("store opens");
    assert!(store.tasks().expect("tasks readable").is_empty());
    assert!(port.was_never_called());

    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_store_without_a_port_keeps_the_honest_absence() {
    // use-case 面已装配但产线 port 缺省(生产装配现状):诚实缺席语义
    // 维持——模拟 Provider 缺席三元与真实缺省装配同形(023 冻结批声明)
    let root = unique_root("noport");
    let database = root.join("host.sqlite");
    let config = handoff_config(&root, None, auto_selected("2022.3.22f1"));
    seed_build_record(&config, "2022.3.22f1");

    let payload = run_frame_wired(
        &database,
        &config,
        "release.openForHandoff",
        "release-handoff-noport",
        json!({ "buildId": BUILD_ID }),
    );
    assert_eq!(payload["ok"], json!(false));
    let error = &payload["error"];
    assert_eq!(error["code"], json!("vua.release_handoff.unavailable"));
    assert_eq!(error["category"], json!("unavailable"));
    assert_eq!(
        error["messageKey"],
        json!("errors.releaseHandoff.unavailable")
    );

    fs::remove_dir_all(&root).ok();
}

#[test]
fn explicit_injection_short_circuits_and_reaches_the_port() {
    // 裁决⑤第一级:显式注入(021 权威)优先于构建记录身份——版本与记录
    // 不同仍按显式注入执行(021 手选权威,不再对账)
    let root = unique_root("explicit");
    let database = root.join("host.sqlite");
    let port = Arc::new(FakePort::arriving());
    let selection = EditorSelection::Explicit {
        path: std::path::PathBuf::from("C:/Unity/2019.4.31f1/Editor/Unity.exe"),
    };
    let config = handoff_config(&root, Some(port.clone()), selection);
    seed_build_record(&config, "2022.3.22f1");

    // The route resolves the explicit injection through the editor-verify
    // face; this fixture injects the verifier so identity comes from the
    // "executable" without a real PE version resource.
    let verifier: vua_provider_host::EditorPathVerifier = Arc::new(|path: &std::path::Path| {
        EditorPathVerdict::Verified(EditorPathIdentity {
            editor_root: "C:/Unity/2019.4.31f1".to_owned(),
            exe_path: path.to_string_lossy().to_string(),
            version: "2019.4.31f1".to_owned(),
            classification: vua_orchestrator::EditorClass::MigrationSource,
            guidance_code: "vua.env_managers.editor_migration_source",
            china_distribution: false,
        })
    });

    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-release-handoff-explicit",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-release-handoff-explicit",
            "correlationId": "corr-release-handoff-explicit",
            "kind": "command",
            "commandId": "command-release-handoff-explicit",
            "method": "release.openForHandoff",
            "params": { "buildId": BUILD_ID },
        },
    });
    let bdl_root = database.parent().unwrap_or(&database).join("bdl");
    let warehouse = WarehouseConfig {
        bdl: Arc::new(BdlStore::open(bdl_root.join("bdl.db")).expect("BDL opens")),
        warehouse_root: database.parent().unwrap_or(&database).join("warehouse"),
        global_default: ArtifactMode::UseOriginalUnitypackage,
        executor: None,
        dependencies_queries: None,
    };
    let mut output = Vec::new();
    run_provider_host_full(
        Cursor::new(format!("{frame}\n")),
        &mut output,
        &database,
        None,
        None,
        Some(warehouse),
        Some(config.clone()),
        None,
        None,
        Some(verifier),
        None,
    )
    .expect("frame loop runs");
    let frames: Vec<Value> = String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect();
    let payload = frames[0]["payload"].clone();
    assert_eq!(
        payload["ok"],
        json!(true),
        "explicit injection accepts: {payload}"
    );
    let task_id = payload["value"]["taskId"]
        .as_str()
        .expect("accepted")
        .to_owned();
    let task = wait_terminal(&database, &task_id);
    assert!(matches!(task.state, vua_orchestrator::TaskState::Succeeded));

    // The port got the EXPLICIT identity (tier 1 wins over the record's
    // 2022.3.22f1), and the fact carries it verbatim.
    let launch = port.observed_launch();
    assert_eq!(launch.editor_version, "2019.4.31f1");
    let result = task.result.expect("succeeded handoff refluxes its fact");
    assert_eq!(result["editor"]["version"], json!("2019.4.31f1"));
    assert_eq!(result["projectId"], json!(PROJECT_ID));

    fs::remove_dir_all(&root).ok();
}

// ---- U19 record-state gate (v0.2): FULL state coverage through the real
// host loop, bypassing any UI. The ruling's normative table, one entry per
// build-record v0.3 status enum value, plus the missing and unparseable
// states. Backend-authoritative: a refused handoff accepts NO task and
// never reaches the port; an admitted one proceeds exactly as before. ----

#[test]
fn the_state_gate_admits_succeeded_and_succeeded_with_warnings_only() {
    for state in ["succeeded", "succeeded_with_warnings"] {
        let root = unique_root("gate-allowed");
        let database = root.join("host.sqlite");
        let port = Arc::new(FakePort::arriving());
        let config = handoff_config(&root, Some(port.clone()), auto_selected("2022.3.22f1"));
        seed_build_record_with_status(&config, json!(state));

        let payload = run_frame_wired(
            &database,
            &config,
            "release.openForHandoff",
            "release-handoff-gate-allowed",
            json!({ "buildId": BUILD_ID }),
        );
        assert_eq!(
            payload["ok"],
            json!(true),
            "{state} must pass the whitelist: {payload}"
        );
        assert_eq!(payload["value"]["schemaVersion"], json!("0.2"));
        assert_eq!(
            payload["value"]["operation"],
            json!("release.openForHandoff")
        );
        let task_id = payload["value"]["taskId"]
            .as_str()
            .expect("accepted")
            .to_owned();
        let task = wait_terminal(&database, &task_id);
        assert!(
            matches!(task.state, vua_orchestrator::TaskState::Succeeded),
            "{state} admits a task that completes via the handshake"
        );
        assert!(!port.was_never_called(), "{state} reaches the port");

        fs::remove_dir_all(&root).ok();
    }
}

#[test]
fn the_state_gate_blocks_failed_cancelled_rolled_back_and_recovered_with_verbatim_state() {
    for state in ["failed", "cancelled", "rolled_back", "recovered"] {
        let root = unique_root("gate-blocked");
        let database = root.join("host.sqlite");
        let port = Arc::new(FakePort::arriving());
        let config = handoff_config(&root, Some(port.clone()), auto_selected("2022.3.22f1"));
        seed_build_record_with_status(&config, json!(state));

        let payload = run_frame_wired(
            &database,
            &config,
            "release.openForHandoff",
            "release-handoff-gate-blocked",
            json!({ "buildId": BUILD_ID }),
        );
        assert_eq!(payload["ok"], json!(false), "{state} must be blocked");
        let error = &payload["error"];
        assert_eq!(
            error["code"],
            json!("vua.release_handoff.record_state_blocked"),
            "{state}: the typed policy rejection"
        );
        // 政策拒绝类(permission)——状态是事实,「哪些状态可交接」是政策
        assert_eq!(error["category"], json!("permission"));
        assert_eq!(
            error["messageKey"],
            json!("errors.releaseHandoff.stateBlocked")
        );
        // state param 携带记录原值,逐字不归一
        assert_eq!(
            error["params"]["state"],
            json!(state),
            "{state}: the state param carries the record's original value verbatim"
        );
        // 受理期政策拒绝不受理任务——库里零任务,port 未达
        let store = vua_orchestrator::SqliteTaskStore::open(&database).expect("store opens");
        assert!(
            store.tasks().expect("tasks readable").is_empty(),
            "{state}: no task may be accepted for a blocked record"
        );
        assert!(
            port.was_never_called(),
            "{state}: the port is never reached"
        );

        fs::remove_dir_all(&root).ok();
    }
}

#[test]
fn missing_and_unparseable_status_answer_record_state_unknown() {
    for (label, status) in [
        ("missing", None),
        ("wrong-type", Some(json!(17))),
        ("out-of-enum", Some(json!("completed"))),
        ("empty", Some(json!(""))),
    ] {
        let root = unique_root("gate-unknown");
        let database = root.join("host.sqlite");
        let port = Arc::new(FakePort::arriving());
        let config = handoff_config(&root, Some(port.clone()), auto_selected("2022.3.22f1"));
        seed_build_record_with_status(&config, status.unwrap_or(Value::Null));

        let payload = run_frame_wired(
            &database,
            &config,
            "release.openForHandoff",
            "release-handoff-gate-unknown",
            json!({ "buildId": BUILD_ID }),
        );
        assert_eq!(payload["ok"], json!(false), "{label} must be refused");
        let error = &payload["error"];
        assert_eq!(
            error["code"],
            json!("vua.release_handoff.record_state_unknown"),
            "{label}: the record cannot be confirmed"
        );
        assert_eq!(error["category"], json!("validation"));
        assert_eq!(
            error["messageKey"],
            json!("errors.releaseHandoff.stateUnknown")
        );
        assert!(
            error.get("params").is_none(),
            "{label}: the unknown rejection carries no state param"
        );
        let store = vua_orchestrator::SqliteTaskStore::open(&database).expect("store opens");
        assert!(store.tasks().expect("tasks readable").is_empty());
        assert!(port.was_never_called());

        fs::remove_dir_all(&root).ok();
    }
}

#[test]
fn the_gate_sits_after_record_existence_and_before_editor_resolution() {
    // 准入序钉:不存在记录先答 build_unknown(不是 record_state_*);
    // 被拦状态先答 record_state_blocked(不是 editor_unresolved)——
    // 政策闸在身份解析之前,被拦记录不泄漏解析细节。
    let root = unique_root("gate-order");
    let database = root.join("host.sqlite");
    let port = Arc::new(FakePort::arriving());
    // 编辑器解析必败(版本错位):blocked 记录仍必须答 blocked
    let config = handoff_config(&root, Some(port.clone()), auto_selected("2021.3.5f1"));
    seed_build_record_with_status(&config, json!("failed"));

    let payload = run_frame_wired(
        &database,
        &config,
        "release.openForHandoff",
        "release-handoff-gate-order",
        json!({ "buildId": BUILD_ID }),
    );
    assert_eq!(
        payload["error"]["code"],
        json!("vua.release_handoff.record_state_blocked")
    );

    fs::remove_dir_all(&root).ok();
}

// ---- U19 independent inspection entry (v0.2): NOT gated by the record's
// state — opening the editor is neither recovery-execution nor upload
// permission. The completion fact carries the explicit operation wording
// and never reads as a handoff completion. ----

#[test]
fn inspection_entry_opens_regardless_of_the_record_state() {
    // 被拦态/缺失态记录照样可打开检查——检视路径不按记录状态闸
    for status in [
        Some(json!("failed")),
        Some(json!("recovered")),
        Some(json!("rolled_back")),
        None,
        Some(json!("completed")), // 枚举外词面同样不闸
    ] {
        let root = unique_root("inspection-ungated");
        let database = root.join("host.sqlite");
        let port = Arc::new(FakePort::arriving());
        let config = handoff_config(&root, Some(port.clone()), auto_selected("2022.3.22f1"));
        seed_build_record_with_status(&config, status.unwrap_or(Value::Null));

        let payload = run_frame_wired(
            &database,
            &config,
            "release.openForInspection",
            "release-inspection-ungated",
            json!({ "buildId": BUILD_ID }),
        );
        assert_eq!(
            payload["ok"],
            json!(true),
            "the inspection entry must not be state-gated: {payload}"
        );
        assert_eq!(
            payload["value"]["operation"],
            json!("release.openForInspection")
        );
        assert_eq!(payload["value"]["schemaVersion"], json!("0.2"));
        let task_id = payload["value"]["taskId"]
            .as_str()
            .expect("accepted")
            .to_owned();
        let task = wait_terminal(&database, &task_id);
        assert!(matches!(task.state, vua_orchestrator::TaskState::Succeeded));

        fs::remove_dir_all(&root).ok();
    }
}

#[test]
fn inspection_completion_fact_carries_the_operation_wording_never_a_handoff_claim() {
    let root = unique_root("inspection-fact");
    let database = root.join("host.sqlite");
    let port = Arc::new(FakePort::arriving());
    let config = handoff_config(&root, Some(port.clone()), auto_selected("2022.3.22f1"));
    seed_build_record_with_status(&config, json!("failed"));

    let payload = run_frame_wired(
        &database,
        &config,
        "release.openForInspection",
        "release-inspection-fact",
        json!({ "buildId": BUILD_ID }),
    );
    let task_id = payload["value"]["taskId"]
        .as_str()
        .expect("accepted")
        .to_owned();
    let task = wait_terminal(&database, &task_id);
    assert!(matches!(task.state, vua_orchestrator::TaskState::Succeeded));
    let result = task
        .result
        .expect("a succeeded inspection refluxes its fact");
    let inspection_fact_validator = def_validator(
        "release-open-for-inspection.schema.json",
        "release-openForInspectionFact",
    );
    assert!(
        inspection_fact_validator.is_valid(&result),
        "the inspection fact must satisfy its frozen def: {result}"
    );
    // 词面:operation 恰为检视操作,绝不是交接——检视完成不宣称交接完成
    assert_eq!(result["operation"], json!("release.openForInspection"));
    assert_ne!(result["operation"], json!("release.openForHandoff"));
    assert_eq!(result["schemaVersion"], json!("0.2"));
    assert_eq!(result["buildId"], json!(BUILD_ID));
    assert_eq!(result["projectId"], json!(PROJECT_ID));
    let fact_object = result.as_object().expect("fact is an object");
    assert_eq!(
        fact_object.len(),
        6,
        "six-key closed set (five identity keys + operation)"
    );
    assert!(fact_object.get("uploadStatus").is_none());
    assert!(fact_object.get("upload").is_none());

    // The port received the resolved identity — same mechanics, different
    // wording (the operation semantics live in the route, not the port).
    let launch = port.observed_launch();
    assert_eq!(launch.build_id, BUILD_ID);
    assert_eq!(launch.editor_version, "2022.3.22f1");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn inspection_entry_still_validates_record_existence_and_editor_identity() {
    // 检视路径不闸状态,但工程(记录)存在与编辑器解析两验照旧
    let root = unique_root("inspection-admission");
    let database = root.join("host.sqlite");
    let port = Arc::new(FakePort::arriving());
    // 编辑器解析必败(版本错位)——检视同样拒绝(编辑器解析照验)
    let config = handoff_config(&root, Some(port.clone()), auto_selected("2021.3.5f1"));
    seed_build_record_with_status(&config, json!("failed"));

    let payload = run_frame_wired(
        &database,
        &config,
        "release.openForInspection",
        "release-inspection-unresolved",
        json!({ "buildId": BUILD_ID }),
    );
    assert_eq!(
        payload["error"]["code"],
        json!("vua.release_handoff.editor_unresolved"),
        "editor resolution is still validated on the inspection path"
    );

    // 记录不存在——build_unknown 照答(独立 config,零播种)
    let empty_root = unique_root("inspection-admission-empty");
    let empty_database = empty_root.join("host.sqlite");
    let empty_config = handoff_config(
        &empty_root,
        Some(port.clone()),
        auto_selected("2022.3.22f1"),
    );
    let payload = run_frame_wired(
        &empty_database,
        &empty_config,
        "release.openForInspection",
        "release-inspection-unknown",
        json!({ "buildId": BUILD_ID }),
    );
    assert_eq!(
        payload["error"]["code"],
        json!("vua.release_handoff.build_unknown"),
        "record existence is still validated on the inspection path"
    );
    fs::remove_dir_all(&empty_root).ok();

    // params 闭集照验
    let payload = run_frame_wired(
        &database,
        &config,
        "release.openForInspection",
        "release-inspection-params",
        json!({}),
    );
    assert_eq!(
        payload["error"]["code"],
        json!("vua.release_handoff.invalid_params")
    );

    fs::remove_dir_all(&root).ok();
}
