//! B3 executor as a TaskRuntime task type: submitted through the frozen
//! v0.4.2 runtime (nine states, commandId idempotency). All ports are
//! fakes; the snapshot store is real.

use flate2::write::GzEncoder;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tar::{Builder, Header};

use vua_orchestrator::{
    BridgeError, BuildRecordStatus, BuildRecordStore, FileSystemSnapshotStore, FixedClock,
    MaterialEntryMode, ProjectRef, ResultStatus, RiskDecisionChoice, TaskEventKind, TaskRuntime,
    TaskState, UnityBridge, UnityCommand, UnityOperation, UnityResult, VpmBackend, VpmCapabilities,
};
use vua_unity_bridge::{
    LocalPackageIdentityStore, MaterialCancelToken, MaterialExecutor,
    MaterialIntakeConfirmationV01, MaterialIntakeEngine, MaterialIntakePlanV01,
    MaterialIntakeStepKind, MaterialIntakeTaskSpec, RiskDecisionV01,
};

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-mtask-{label}-{nanos}"))
}

fn append(b: &mut Builder<GzEncoder<fs::File>>, path: &str, bytes: &[u8]) {
    let mut h = Header::new_gnu();
    h.set_size(bytes.len() as u64);
    h.set_cksum();
    b.append_data(&mut h, path, bytes).unwrap();
}

fn unitypackage(path: &Path, assets: &[&str]) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    let f = fs::File::create(path).unwrap();
    let mut b = Builder::new(GzEncoder::new(f, flate2::Compression::default()));
    for a in assets {
        append(&mut b, a, b"fixture");
    }
    b.finish().unwrap();
}

fn make_world(label: &str) -> (PathBuf, ProjectRef, PathBuf) {
    let base = temp_dir(label);
    let source = base.join("source");
    unitypackage(&source.join("pack.unitypackage"), &["Assets/Asset.prefab"]);
    let project_root = base.join("target");
    fs::create_dir_all(project_root.join("Assets")).unwrap();
    fs::create_dir_all(project_root.join("Packages")).unwrap();
    fs::create_dir_all(project_root.join("ProjectSettings")).unwrap();
    // Already-provisioned target: the standing fixtures plan the UNCHANGED
    // v0.1 step set (zero-change law for provisioned projects).
    fs::write(
        project_root
            .join("ProjectSettings")
            .join("ProjectVersion.txt"),
        "2022.3.22f1",
    )
    .unwrap();
    fs::write(project_root.join("vpm-manifest.json"), "{}").unwrap();
    let project = ProjectRef {
        id: "project".into(),
        root: project_root,
    };
    (base, project, source)
}

fn confirmation(plan: &MaterialIntakePlanV01) -> MaterialIntakeConfirmationV01 {
    MaterialIntakeConfirmationV01 {
        plan: plan.clone(),
        risk_decision: RiskDecisionV01 {
            choice: RiskDecisionChoice::Continue,
            source_fingerprint: plan.source.source_fingerprint.clone(),
            risk_fingerprint: plan.source.risk_fingerprint.clone(),
            remember_for_session: false,
        },
        confirmed_at: "2026-09-05T00:00:00Z".into(),
        correlation_id: "corr".into(),
    }
}

struct NoVpm;
impl VpmBackend for NoVpm {
    fn name(&self) -> &'static str {
        "none"
    }
    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: false,
            preview_install: false,
            list_packages: false,
            remove_packages: false,
            project_registry: false,
            resolve_project: false,
        }
    }
    fn preview_install(
        &self,
        _: &ProjectRef,
        _: &[vua_orchestrator::PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, vua_orchestrator::AppErrorV1> {
        panic!("direct mode never previews")
    }
    fn apply_install(
        &self,
        _: &ProjectRef,
        _: &[vua_orchestrator::PackageRequestV1],
        _: &str,
    ) -> Result<serde_json::Value, vua_orchestrator::AppErrorV1> {
        panic!("direct mode never installs")
    }
    fn create_project(
        &self,
        _: &Path,
        _: &str,
        _: Option<&str>,
    ) -> Result<ProjectRef, vua_orchestrator::AppErrorV1> {
        panic!("direct mode never creates projects")
    }
}

/// Always succeeds and reports a fresh project fingerprint per command, so
/// the executor's fingerprint chaining is exercised for real.
struct FakeBridge {
    commands: Mutex<Vec<UnityCommand>>,
}

impl FakeBridge {
    fn new() -> Self {
        Self {
            commands: Mutex::new(Vec::new()),
        }
    }

    fn command_count(&self) -> usize {
        self.commands.lock().unwrap().len()
    }
}

impl UnityBridge for FakeBridge {
    fn execute(&self, _: &ProjectRef, command: &UnityCommand) -> Result<UnityResult, BridgeError> {
        let mut commands = self.commands.lock().unwrap();
        commands.push(command.clone());
        // 第 158 批（BOARD #45(2)）：the executor parses a Succeeded validate
        // receipt's loadedAssetPaths strictly (missing field = honest
        // bridge_failed), so the fake mirrors the real C# handler — the
        // Ordinal-sorted expected list IS the loaded list on success.
        let mut data = serde_json::json!({
            "projectFingerprint": format!("fp-{}", commands.len())
        });
        if command.operation == UnityOperation::ValidateAssetPaths {
            let mut loaded = command.payload.expected_asset_paths.clone();
            loaded.sort();
            data["loadedAssetPaths"] = loaded.into_iter().map(serde_json::Value::String).collect();
        }
        Ok(UnityResult {
            schema_version: 1,
            command_id: command.command_id.clone(),
            status: ResultStatus::Succeeded,
            changed_paths: vec![],
            diagnostics: vec![],
            steps: Vec::new(),
            replayed: None,
            snapshot_id: None,
            restored_from: None,
            project_fingerprint_before: None,
            data,
        })
    }
}

fn build_executor(base: &Path, bridge: FakeBridge) -> (Arc<MaterialExecutor>, Arc<FakeBridge>) {
    let bridge = Arc::new(bridge);
    let executor = Arc::new(MaterialExecutor::new(
        bridge.clone(),
        FileSystemSnapshotStore,
        Arc::new(NoVpm),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-05T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    ));
    (executor, bridge)
}

fn runtime() -> TaskRuntime {
    TaskRuntime::new(
        Arc::new(vua_orchestrator::MemoryJournal::default()),
        Arc::new(vua_orchestrator::SystemClock),
        Arc::new(vua_orchestrator::FixedIdGenerator::default()),
    )
}

fn wait_for_terminal(rt: &TaskRuntime, task_id: &str) -> vua_orchestrator::TaskSnapshot {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let s = rt.snapshot(task_id).expect("task must exist");
        if s.state.is_terminal() {
            return s;
        }
        assert!(Instant::now() < deadline, "task did not finish");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn b3_task_001_happy_path_runs_and_replays() {
    let (base, project, source) = make_world("task-happy");
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, "corr")
        .unwrap();
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            "project-fingerprint",
            inspection,
            &project.root,
            "corr",
        )
        .unwrap();
    let confirmation = confirmation(&plan);

    let (executor, bridge) = build_executor(&base, FakeBridge::new());
    let token = MaterialCancelToken::new();
    let rt = runtime();
    let accepted = vua_unity_bridge::submit_material_intake(
        &rt,
        executor.clone(),
        MaterialIntakeTaskSpec {
            confirmation: confirmation.clone(),
            source_folder: source.clone(),
            project: project.clone(),
            artifact_output_root: base.join("artifacts"),
            token: token.clone(),
        },
        Some(Duration::from_secs(120)),
    )
    .unwrap();

    let snapshot = wait_for_terminal(&rt, &accepted.task_id);
    assert_eq!(snapshot.state, TaskState::Succeeded);
    let record = BuildRecordStore::new(base.join("records"))
        .read(&format!("material-{}", plan.plan_id))
        .expect("receipt published");
    assert_eq!(record.status, BuildRecordStatus::Succeeded);
    assert_eq!(bridge.command_count(), 2, "one import + one validation");

    // The published receipt makes a direct re-run a replay: no Unity traffic.
    let second = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );
    assert!(second.replayed, "replay must not touch Unity");
    assert_eq!(bridge.command_count(), 2);
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

// --- 第 150 批：失败词面按类别分流（provisionFailed / executionFailed） ---

/// An unprovisioned target: no ProjectSettings/, so the plan carries the
/// conditional ProvisionProject step (plan v0.2).
fn make_unprovisioned_world(label: &str) -> (PathBuf, ProjectRef, PathBuf) {
    let base = temp_dir(label);
    let source = base.join("source");
    unitypackage(&source.join("pack.unitypackage"), &["Assets/Asset.prefab"]);
    let project_root = base.join("empty-target");
    fs::create_dir_all(&project_root).unwrap();
    let project = ProjectRef {
        id: "project".into(),
        root: project_root,
    };
    (base, project, source)
}

/// A backend whose creation leg fails — the provision arm's honest failure.
struct FailingCreateVpm;
impl VpmBackend for FailingCreateVpm {
    fn name(&self) -> &'static str {
        "failing-create"
    }
    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: true,
            preview_install: false,
            list_packages: false,
            remove_packages: false,
            project_registry: false,
            resolve_project: true,
        }
    }
    fn preview_install(
        &self,
        _: &ProjectRef,
        _: &[vua_orchestrator::PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, vua_orchestrator::AppErrorV1> {
        panic!("never reached: creation fails first")
    }
    fn apply_install(
        &self,
        _: &ProjectRef,
        _: &[vua_orchestrator::PackageRequestV1],
        _: &str,
    ) -> Result<serde_json::Value, vua_orchestrator::AppErrorV1> {
        panic!("never reached: creation fails first")
    }
    fn create_project(
        &self,
        _: &Path,
        _: &str,
        _: Option<&str>,
    ) -> Result<ProjectRef, vua_orchestrator::AppErrorV1> {
        Err(vua_orchestrator::AppErrorV1::new(
            "vua.vpm.template_missing",
            vua_orchestrator::ErrorCategory::ExternalFailure,
            "errors.vpm.templateMissing",
            "corr",
        ))
    }
}

/// Drains the subscribed events and returns the Completed payload — the
/// serialized AppErrorV1 the task exit carried.
fn completed_payload(
    receiver: &mut std::sync::mpsc::Receiver<vua_orchestrator::TaskEventV1>,
) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline, "no Completed event arrived");
        match receiver.recv_timeout(Duration::from_millis(200)) {
            Ok(event) if event.kind == TaskEventKind::Completed => return event.payload,
            Ok(_) => continue,
            Err(_) => continue,
        }
    }
}

#[test]
fn b3_task_003_provision_failure_presents_the_reserved_provision_face() {
    let (base, project, source) = make_unprovisioned_world("task-provision-face");
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, "corr")
        .unwrap();
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            "project-fingerprint",
            inspection,
            &project.root,
            "corr",
        )
        .unwrap();
    assert!(
        plan.steps
            .iter()
            .any(|step| step.kind == MaterialIntakeStepKind::ProvisionProject),
        "the fixture must plan the provision step"
    );

    let executor = Arc::new(MaterialExecutor::new(
        Arc::new(FakeBridge::new()),
        FileSystemSnapshotStore,
        Arc::new(FailingCreateVpm),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-05T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    ));
    let rt = runtime();
    let mut events = rt.subscribe();
    let accepted = vua_unity_bridge::submit_material_intake(
        &rt,
        executor,
        MaterialIntakeTaskSpec {
            confirmation: confirmation(&plan),
            source_folder: source.clone(),
            project: project.clone(),
            artifact_output_root: base.join("artifacts"),
            token: MaterialCancelToken::new(),
        },
        None,
    )
    .unwrap();

    let snapshot = wait_for_terminal(&rt, &accepted.task_id);
    assert_eq!(snapshot.state, TaskState::Failed);
    let payload = completed_payload(&mut events);
    assert_eq!(
        payload["messageKey"], "errors.material.provisionFailed",
        "the provision segment hits the long reserved row: {payload}"
    );
    assert!(
        payload["code"]
            .as_str()
            .unwrap_or("")
            .starts_with("vua.material.provision_failed"),
        "the family code still carries the detail: {payload}"
    );
    // The compensation is unchanged: the empty-state restore + failed receipt.
    let receipt = BuildRecordStore::new(base.join("records"))
        .read(&format!("material-{}", plan.plan_id))
        .expect("the failed provision still publishes its receipt");
    assert_eq!(receipt.status, BuildRecordStatus::Failed);
    assert!(!project.root.join("ProjectSettings").exists());
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

/// A bridge that rejects the first command — the bridge-segment failure.
struct RejectingBridge;
impl UnityBridge for RejectingBridge {
    fn execute(&self, _: &ProjectRef, command: &UnityCommand) -> Result<UnityResult, BridgeError> {
        Ok(UnityResult {
            schema_version: 1,
            command_id: command.command_id.clone(),
            status: ResultStatus::Rejected,
            changed_paths: vec![],
            diagnostics: vec![],
            steps: Vec::new(),
            replayed: None,
            snapshot_id: None,
            restored_from: None,
            project_fingerprint_before: None,
            data: serde_json::json!({}),
        })
    }
}

#[test]
fn b3_task_004_bridge_failure_keeps_the_standing_execution_face() {
    let (base, project, source) = make_world("task-bridge-face");
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, "corr")
        .unwrap();
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            "project-fingerprint",
            inspection,
            &project.root,
            "corr",
        )
        .unwrap();

    let executor = Arc::new(MaterialExecutor::new(
        Arc::new(RejectingBridge),
        FileSystemSnapshotStore,
        Arc::new(NoVpm),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-05T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    ));
    let rt = runtime();
    let mut events = rt.subscribe();
    let accepted = vua_unity_bridge::submit_material_intake(
        &rt,
        executor,
        MaterialIntakeTaskSpec {
            confirmation: confirmation(&plan),
            source_folder: source.clone(),
            project: project.clone(),
            artifact_output_root: base.join("artifacts"),
            token: MaterialCancelToken::new(),
        },
        None,
    )
    .unwrap();

    let snapshot = wait_for_terminal(&rt, &accepted.task_id);
    assert_eq!(snapshot.state, TaskState::Failed);
    let payload = completed_payload(&mut events);
    assert_eq!(
        payload["messageKey"], "errors.material.executionFailed",
        "a bridge-segment failure keeps the standing face — only the provision segment moved: {payload}"
    );
    assert_eq!(payload["code"], "vua.material.bridge_rejected");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_task_002_cancelled_run_exits_cancelled_without_a_receipt() {
    let (base, project, source) = make_world("task-cancel");
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, "corr")
        .unwrap();
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            "project-fingerprint",
            inspection,
            &project.root,
            "corr",
        )
        .unwrap();
    let confirmation = confirmation(&plan);

    // Cancellation lands before the first step: nothing mutated, so the
    // executor publishes no receipt (same semantics as b3_exec_003) — a
    // later re-run simply runs fresh.
    let (executor, bridge) = build_executor(&base, FakeBridge::new());
    let token = MaterialCancelToken::new();
    token.cancel();
    let rt = runtime();
    let accepted = vua_unity_bridge::submit_material_intake(
        &rt,
        executor,
        MaterialIntakeTaskSpec {
            confirmation: confirmation.clone(),
            source_folder: source.clone(),
            project: project.clone(),
            artifact_output_root: base.join("artifacts"),
            token: token.clone(),
        },
        None,
    )
    .unwrap();

    let snapshot = wait_for_terminal(&rt, &accepted.task_id);
    assert_eq!(snapshot.state, TaskState::Cancelled);
    assert_eq!(
        bridge.command_count(),
        0,
        "cancellation precedes every step"
    );
    assert!(
        BuildRecordStore::new(base.join("records"))
            .read(&format!("material-{}", plan.plan_id))
            .is_err(),
        "a pre-mutation cancel leaves no receipt"
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}
