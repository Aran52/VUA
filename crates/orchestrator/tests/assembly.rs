//! E-ASSEMB integration tests: the M4 vertical smoke over a fake Unity
//! Bridge and a scripted vrc-get backend. Each test cites its ORC requirement
//! (ORC-TST-006). The real-Unity fixture run stays a manual acceptance
//! (agile plan), exactly like the E-PKG real-vrc-get smoke.

#![allow(clippy::result_large_err)]

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use vua_orchestrator::{
    derive_project_spec, document_digest, AssemblyConfirmation, AssemblyEngine, BridgeError,
    ChangeItemV1, ChangeKindV1, ChangePreviewV1, FixedClock, FixedIdGenerator, MemoryJournal,
    PackageRequestV1, ProjectRef, RecipeV02, SubmitRequest, TaskEventKind, TaskExit, TaskRuntime,
    TaskState, UnityBridge, UnityCommand, UnityResult, VpmBackend, VpmCapabilities,
};

// --- fixtures ---

fn fixture_recipe() -> RecipeV02 {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/recipe/v0.2/example.recipe.json");
    let bytes = std::fs::read(path).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-assembly-{label}-{nanos}"))
}

/// Records every command and answers success by default; scripts and a
/// per-operation timeout switch support failure injection.
struct FakeBridge {
    calls: Mutex<Vec<UnityCommand>>,
    script: Mutex<VecDeque<Result<UnityResult, BridgeError>>>,
    timed_out_ops: Mutex<Vec<String>>,
}

impl FakeBridge {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::new(Vec::new()),
            script: Mutex::new(VecDeque::new()),
            timed_out_ops: Mutex::new(Vec::new()),
        })
    }

    fn push(&self, outcome: Result<UnityResult, BridgeError>) {
        self.script.lock().unwrap().push_back(outcome);
    }

    /// Every call to the named Unity operation answers `TimedOut` — the
    /// "Unity never wrote a result" case.
    fn fail_timed_out(&self, operation: &str) {
        self.timed_out_ops
            .lock()
            .unwrap()
            .push(operation.to_owned());
    }

    fn clear_timed_out(&self) {
        self.timed_out_ops.lock().unwrap().clear();
    }

    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

impl UnityBridge for FakeBridge {
    fn execute(
        &self,
        _project: &ProjectRef,
        command: &UnityCommand,
    ) -> Result<UnityResult, BridgeError> {
        let call_index = {
            let mut calls = self.calls.lock().unwrap();
            calls.push(command.clone());
            calls.len()
        };
        if self
            .timed_out_ops
            .lock()
            .unwrap()
            .iter()
            .any(|operation| *operation == command.operation.to_string())
        {
            return Err(BridgeError::TimedOut);
        }
        match self.script.lock().unwrap().pop_front() {
            Some(result) => result,
            None => Ok(UnityResult {
                schema_version: 1,
                command_id: command.command_id.clone(),
                status: vua_orchestrator::ResultStatus::Succeeded,
                changed_paths: Vec::new(),
                diagnostics: Vec::new(),
                data: serde_json::json!({
                    "projectFingerprint": format!("v1:fake-{call_index}")
                }),
                steps: Vec::new(),
                replayed: None,
                snapshot_id: None,
                restored_from: None,
                project_fingerprint_before: None,
            }),
        }
    }
}

fn make_project(root: &Path) {
    fs::create_dir_all(root.join("Packages")).unwrap();
    fs::create_dir_all(root.join("ProjectSettings")).unwrap();
    fs::write(
        root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1",
    )
    .unwrap();
    fs::write(
        root.join("vpm-manifest.json"),
        r#"{"dependencies": {}, "locked": {}}"#,
    )
    .unwrap();
}

fn manifest_text(root: &Path) -> String {
    fs::read_to_string(root.join("vpm-manifest.json")).unwrap()
}

fn engine_for(
    project_root: PathBuf,
    bridge: Arc<FakeBridge>,
    backend: Arc<FakeVpmBackend>,
) -> AssemblyEngine {
    AssemblyEngine::new(
        bridge,
        backend,
        Arc::new(FixedClock::new(&["2026-08-30T10:00:00.000Z"])),
        "proj-assembly",
        project_root,
    )
}

/// E-VPM-DUAL: the backend port fake -- create copies a skeleton, preview
/// returns the one-package change set, apply writes the manifest.
struct FakeVpmBackend {
    can_create: bool,
    created: Mutex<Vec<(PathBuf, String)>>,
    applied: Mutex<Vec<String>>,
    previews: Mutex<Vec<ChangePreviewV1>>,
}

impl FakeVpmBackend {
    fn new(can_create: bool) -> Arc<Self> {
        Arc::new(Self {
            can_create,
            created: Mutex::new(Vec::new()),
            applied: Mutex::new(Vec::new()),
            previews: Mutex::new(Vec::new()),
        })
    }
}

impl VpmBackend for FakeVpmBackend {
    fn name(&self) -> &'static str {
        "fake-vpm"
    }

    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: self.can_create,
            preview_install: true,
            list_packages: false,
            remove_packages: false,
            project_registry: false,
            resolve_project: false,
        }
    }

    fn preview_install(
        &self,
        _project: &ProjectRef,
        packages: &[PackageRequestV1],
    ) -> Result<ChangePreviewV1, vua_orchestrator::AppErrorV1> {
        let preview = ChangePreviewV1 {
            items: packages
                .iter()
                .map(|package| ChangeItemV1 {
                    kind: ChangeKindV1::Install,
                    package_id: package.package_id.clone(),
                    version: package.version.clone(),
                    reason: None,
                })
                .collect(),
            conflicts: vec![],
            remove_legacy_files: vec![],
            remove_legacy_folders: vec![],
            destructive: false,
            digest: format!(
                "digest-{}",
                packages
                    .iter()
                    .map(|package| format!(
                        "{}@{}",
                        package.package_id,
                        package.version.as_deref().unwrap_or("latest")
                    ))
                    .collect::<Vec<_>>()
                    .join("+")
            ),
        };
        self.previews.lock().unwrap().push(preview.clone());
        Ok(preview)
    }

    fn apply_install(
        &self,
        project: &ProjectRef,
        packages: &[PackageRequestV1],
        confirmed_digest: &str,
    ) -> Result<serde_json::Value, vua_orchestrator::AppErrorV1> {
        let expected = format!(
            "digest-{}",
            packages
                .iter()
                .map(|package| format!(
                    "{}@{}",
                    package.package_id,
                    package.version.as_deref().unwrap_or("latest")
                ))
                .collect::<Vec<_>>()
                .join("+")
        );
        assert_eq!(
            confirmed_digest, expected,
            "digest must bind preview to apply"
        );
        let dependencies = packages
            .iter()
            .map(|package| {
                format!(
                    "\"{}\": {{\"version\": \"{}\"}}",
                    package.package_id,
                    package.version.as_deref().unwrap_or("9.9.9")
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let manifest = format!(r#"{{"dependencies": {{{dependencies}}}, "locked": {{}}}}"#,);
        fs::write(project.root.join("vpm-manifest.json"), manifest).unwrap();
        self.applied
            .lock()
            .unwrap()
            .extend(packages.iter().map(|package| package.package_id.clone()));
        Ok(serde_json::json!({ "applied": packages }))
    }

    fn create_project(
        &self,
        parent: &std::path::Path,
        name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, vua_orchestrator::AppErrorV1> {
        if !self.can_create {
            return Err(vua_orchestrator::AppErrorV1::new(
                "vua.vpm.capability_missing",
                vua_orchestrator::ErrorCategory::Unavailable,
                "errors.vpm.capabilityMissing",
                "corr-fake",
            ));
        }
        let root = parent.join(name);
        make_project_at(&root);
        self.created
            .lock()
            .unwrap()
            .push((root.clone(), name.to_owned()));
        Ok(ProjectRef {
            id: format!("proj-{name}"),
            root,
        })
    }
}

fn make_project_at(root: &std::path::Path) {
    fs::create_dir_all(root.join("Packages")).unwrap();
    fs::create_dir_all(root.join("ProjectSettings")).unwrap();
    fs::write(
        root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1",
    )
    .unwrap();
    fs::write(
        root.join("vpm-manifest.json"),
        r#"{"dependencies": {}, "locked": {}}"#,
    )
    .unwrap();
}

fn plan_and_confirm(engine: &AssemblyEngine, recipe: &RecipeV02) -> AssemblyConfirmation {
    let digest = document_digest(recipe).unwrap();
    let spec = derive_project_spec(recipe, &digest).unwrap();
    let plan = engine.derive_plan(&spec, recipe).expect("plan derivation");
    engine.confirm(&plan, &plan.plan_hash).unwrap()
}

// --- the four required O7 scenarios ---

#[test]
fn orc_wf_005_fresh_project_provisions_installs_and_assembles() {
    let base = unique_dir("fresh");
    let project_root = base.join("projects/My Avatar");
    let bridge = FakeBridge::new();
    let engine = engine_for(
        project_root.clone(),
        bridge.clone(),
        FakeVpmBackend::new(false),
    );
    let recipe = fixture_recipe();

    let confirmation = plan_and_confirm(&engine, &recipe);
    assert!(
        confirmation.plan.project_fingerprint.is_none(),
        "a fresh project plans with no manifest fingerprint"
    );
    assert_eq!(
        confirmation.plan.steps[0].operation.to_string(),
        "ProvisionProject"
    );

    // Fix 4: vrc-get CLI cannot create projects -- the engine surfaces the
    // honest guidance error instead of a phantom command (ADR-0006 draft
    // will bring the dual-backend creation story).
    let error = engine.execute(&confirmation, &recipe, None).unwrap_err();
    assert_eq!(error.code, "vua.assembly.provision_failed");
    // reason 携带后端错误码（capability_missing），前端据此诚实引导用户
    // 用 ALCOM / VCC `vpm new` / Unity Hub 创建项目。
    assert!(error
        .params
        .as_ref()
        .map(|params| matches!(params["reason"], vua_orchestrator::ParamValue::Text(ref reason) if reason.contains("capability_missing")))
        .unwrap_or(false));
}

#[test]
fn e_vpm_dual_project_creation_and_package_planning_use_independent_backends() {
    let base = unique_dir("dual-backend");
    let project_root = base.join("projects/My Avatar");
    let bridge = FakeBridge::new();
    let project_backend = FakeVpmBackend::new(true);
    let package_backend = FakeVpmBackend::new(false);
    let engine = AssemblyEngine::with_backends(
        bridge.clone(),
        project_backend.clone(),
        package_backend.clone(),
        Arc::new(FixedClock::new(&["2026-08-30T10:00:00.000Z"])),
        "proj-assembly",
        project_root.clone(),
    );
    let recipe = fixture_recipe();
    let confirmation = plan_and_confirm(&engine, &recipe);

    let payload = engine.execute(&confirmation, &recipe, None).unwrap();
    assert_eq!(payload["noop"], false);
    assert_eq!(project_backend.created.lock().unwrap().len(), 1);
    assert_eq!(project_backend.applied.lock().unwrap().len(), 0);
    assert_eq!(package_backend.applied.lock().unwrap().len(), 1);
    assert_eq!(bridge.call_count(), 6);
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_wf_005_existing_project_skips_provision_installs_and_assembles() {
    let base = unique_dir("existing");
    let project_root = base.join("projects/My Avatar");
    make_project(&project_root);
    let bridge = FakeBridge::new();
    let engine = engine_for(
        project_root.clone(),
        bridge.clone(),
        FakeVpmBackend::new(true),
    );
    let recipe = fixture_recipe();

    let confirmation = plan_and_confirm(&engine, &recipe);
    let payload = engine.execute(&confirmation, &recipe, None).unwrap();
    assert_eq!(payload["noop"], false);
    assert_eq!(payload["stepsCompleted"], 7);

    // The package installed, Unity walked all six bridge steps with typed
    // commands (no shell).
    assert!(project_root
        .join("ProjectSettings/ProjectVersion.txt")
        .is_file());
    assert!(manifest_text(&project_root).contains("dev.fixture.modular-avatar"));
    assert_eq!(bridge.call_count(), 6);
    let calls = bridge.calls.lock().unwrap().clone();
    assert_eq!(calls[0].operation.to_string(), "inspect_project");
    assert!(calls[0].dry_run);
    assert_eq!(calls[2].operation.to_string(), "install_outfit");
    assert!(!calls[2].dry_run);
    // The example instance carries neither label nor name hint, so the
    // payload falls back to the stable instance id.
    assert_eq!(calls[2].payload.outfit_global_object_id, "outfit_blue");
    assert_eq!(calls[3].payload.toggle_name, "Main outfit");
    assert!(calls
        .iter()
        .all(|command| command.command_id.starts_with(&confirmation.plan.plan_id)));
    assert_eq!(calls[0].expected_project_fingerprint, None);
    for (index, command) in calls.iter().enumerate().skip(1) {
        assert_eq!(
            command.expected_project_fingerprint.as_deref(),
            Some(format!("v1:fake-{index}").as_str()),
            "each bridge command binds the semantic fingerprint returned by its predecessor"
        );
    }

    // The completion marker exists for idempotent replay.
    let marker_path = project_root
        .join(".vua/assembly")
        .join(format!("{}.done.json", confirmation.plan.plan_id));
    assert!(marker_path.is_file());
    let marker: serde_json::Value =
        serde_json::from_slice(&fs::read(marker_path).unwrap()).unwrap();
    assert!(
        marker
            .get("finalFingerprint")
            .and_then(|value| value.as_str())
            .is_some(),
        "R2-4: a completion marker binds the post-build project tree"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_wf_011_repeating_the_same_confirmed_plan_is_an_explicit_noop() {
    let base = unique_dir("repeat");
    let project_root = base.join("project");
    make_project(&project_root);
    let bridge = FakeBridge::new();
    let engine = engine_for(
        project_root.clone(),
        bridge.clone(),
        FakeVpmBackend::new(true),
    );
    let recipe = fixture_recipe();

    let confirmation = plan_and_confirm(&engine, &recipe);
    let first = engine.execute(&confirmation, &recipe, None).unwrap();
    assert_eq!(first["noop"], false);
    let calls_after_first = bridge.call_count();

    let second = engine.execute(&confirmation, &recipe, None).unwrap();
    assert_eq!(second["noop"], true, "replay is an explicit no-op");
    assert_eq!(
        bridge.call_count(),
        calls_after_first,
        "the replay must not touch Unity again"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_wf_011_marker_replay_refuses_a_project_changed_after_completion() {
    let base = unique_dir("marker-drift");
    let project_root = base.join("project");
    make_project(&project_root);
    let bridge = FakeBridge::new();
    let engine = engine_for(
        project_root.clone(),
        bridge.clone(),
        FakeVpmBackend::new(true),
    );
    let recipe = fixture_recipe();

    let confirmation = plan_and_confirm(&engine, &recipe);
    engine.execute(&confirmation, &recipe, None).unwrap();
    let calls_after_first = bridge.call_count();
    fs::write(
        project_root.join("vpm-manifest.json"),
        r#"{"dependencies":{"user.changed":{"version":"1.0.0"}},"locked":{}}"#,
    )
    .unwrap();

    let error = engine.execute(&confirmation, &recipe, None).unwrap_err();
    assert_eq!(error.code, "vua.assembly.drift");
    assert!(error.recoverable);
    assert_eq!(
        bridge.call_count(),
        calls_after_first,
        "marker drift is rejected before any Unity command"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_wf_011_marker_replay_detects_avatar_asset_changes_without_manifest_changes() {
    let base = unique_dir("marker-assets-drift");
    let project_root = base.join("project");
    make_project(&project_root);
    fs::create_dir_all(project_root.join("Assets")).unwrap();
    fs::write(project_root.join("Assets/Avatar.prefab"), "avatar-v1").unwrap();
    let engine = engine_for(
        project_root.clone(),
        FakeBridge::new(),
        FakeVpmBackend::new(true),
    );
    let recipe = fixture_recipe();
    let confirmation = plan_and_confirm(&engine, &recipe);
    engine.execute(&confirmation, &recipe, None).unwrap();

    // Same byte length deliberately defeats the old size/manifest-only gate.
    fs::write(project_root.join("Assets/Avatar.prefab"), "avatar-v2").unwrap();
    let error = engine.execute(&confirmation, &recipe, None).unwrap_err();
    assert_eq!(error.code, "vua.assembly.drift");
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_wf_005_failed_fresh_project_is_removed_and_the_plan_can_be_retried() {
    let base = unique_dir("fresh-rollback");
    let project_root = base.join("project");
    let bridge = FakeBridge::new();
    bridge.fail_timed_out("inspect_project");
    let engine = engine_for(
        project_root.clone(),
        bridge.clone(),
        FakeVpmBackend::new(true),
    );
    let recipe = fixture_recipe();
    let confirmation = plan_and_confirm(&engine, &recipe);
    let error = engine.execute(&confirmation, &recipe, None).unwrap_err();
    assert_eq!(error.code, "vua.assembly.unity_timeout");
    assert!(
        !project_root.exists(),
        "a project created by this failed attempt must be removed"
    );

    bridge.clear_timed_out();
    engine.execute(&confirmation, &recipe, None).unwrap();
    assert!(project_root
        .join("ProjectSettings/ProjectVersion.txt")
        .is_file());
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_con_006_cancel_stops_assembly_at_the_next_safe_boundary() {
    let base = unique_dir("cancel-boundary");
    let project_root = base.join("project");
    make_project(&project_root);
    let bridge = FakeBridge::new();
    let engine = Arc::new(engine_for(
        project_root.clone(),
        bridge.clone(),
        FakeVpmBackend::new(true),
    ));
    let recipe = Arc::new(fixture_recipe());
    let confirmation = Arc::new(plan_and_confirm(&engine, &recipe));
    let gate = Arc::new(Barrier::new(2));
    let runtime = TaskRuntime::new(
        Arc::new(MemoryJournal::new()),
        Arc::new(FixedClock::new(&["2026-08-30T10:00:00.000Z"])),
        Arc::new(FixedIdGenerator::default()),
    );
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new({
                let engine = engine.clone();
                let recipe = recipe.clone();
                let confirmation = confirmation.clone();
                let gate = gate.clone();
                move |context| {
                    gate.wait();
                    engine
                        .execute(&confirmation, &recipe, Some(context))
                        .map(TaskExit::Done)
                }
            }),
        })
        .unwrap();
    runtime.cancel(&accepted.task_id).unwrap();
    gate.wait();

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state = runtime.snapshot(&accepted.task_id).unwrap().state;
        if state.is_terminal() {
            assert_eq!(state, TaskState::Cancelled);
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(bridge.call_count(), 0);
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_wf_004_project_modified_after_confirmation_refuses_to_run() {
    let base = unique_dir("drift");
    let project_root = base.join("project");
    make_project(&project_root);
    let bridge = FakeBridge::new();
    let engine = engine_for(
        project_root.clone(),
        bridge.clone(),
        FakeVpmBackend::new(true),
    );
    let recipe = fixture_recipe();

    let confirmation = plan_and_confirm(&engine, &recipe);
    // The project changes after confirmation.
    fs::write(
        project_root.join("vpm-manifest.json"),
        r#"{"dependencies": {"other": {"version": "0.1"}}}"#,
    )
    .unwrap();

    let error = engine.execute(&confirmation, &recipe, None).unwrap_err();
    assert_eq!(error.code, "vua.assembly.drift");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Conflict);
    assert_eq!(
        bridge.call_count(),
        0,
        "nothing may run past the drift gate"
    );
    assert!(
        !project_root.join(".vua/snapshots").exists()
            || fs::read_dir(project_root.join(".vua/snapshots"))
                .unwrap()
                .count()
                == 0,
        "no snapshot is taken when the gate refuses"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_con_004_unity_timeout_fails_the_task_and_restores_the_snapshot() {
    let base = unique_dir("timeout");
    let project_root = base.join("project");
    make_project(&project_root);
    let original = manifest_text(&project_root);
    let bridge = FakeBridge::new();
    // Everything succeeds until the mutating InstallOutfit step, which times
    // out — the classic "Unity never wrote a result" case.
    bridge.fail_timed_out("install_outfit");
    let engine = engine_for(
        project_root.clone(),
        bridge.clone(),
        FakeVpmBackend::new(true),
    );
    let recipe = fixture_recipe();

    let confirmation = plan_and_confirm(&engine, &recipe);
    let error = engine.execute(&confirmation, &recipe, None).unwrap_err();
    assert_eq!(error.code, "vua.assembly.unity_timeout");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Timeout);
    assert!(error.retryable);

    // Rollback restored the pre-run manifest; no marker was written.
    assert_eq!(manifest_text(&project_root), original);
    assert!(!project_root
        .join(".vua/assembly")
        .join(format!("{}.done.json", confirmation.plan.plan_id))
        .is_file());

    // Fix 6: 同一确认直接重试——新的尝试拿到新的快照 id，不会与上次
    // 遗留的快照目录撞车（AlreadyExists 曾让计划永久无法重跑）。
    bridge.clear_timed_out();
    let payload = engine.execute(&confirmation, &recipe, None).unwrap();
    assert_eq!(payload["noop"], false);
    assert!(manifest_text(&project_root).contains("dev.fixture.modular-avatar"));
    fs::remove_dir_all(&base).ok();
}

// --- command binding & plan guards ---

#[test]
fn orc_wf_002_multiple_packages_share_one_interaction_aware_preview() {
    let base = unique_dir("batch-preview");
    let project_root = base.join("project");
    make_project(&project_root);
    let backend = FakeVpmBackend::new(true);
    let engine = engine_for(project_root, FakeBridge::new(), backend.clone());
    let mut recipe_value = serde_json::to_value(fixture_recipe()).unwrap();
    recipe_value["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "packageId": "dev.fixture.avatar-optimizer",
            "versionConstraint": "=1.8.0",
            "requestedByAssetIds": ["avatar_asset"]
        }));
    recipe_value["locked"]["packages"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "packageId": "dev.fixture.avatar-optimizer",
            "version": "1.8.0",
            "depth": 0,
            "source": {
                "kind": "vpm",
                "restorable": true,
                "repositoryId": "fixture-repository",
                "repositoryRevision": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "artifactDigest": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
            }
        }));
    let recipe: RecipeV02 = serde_json::from_value(recipe_value).unwrap();
    let digest = document_digest(&recipe).unwrap();
    let spec = derive_project_spec(&recipe, &digest).unwrap();
    let plan = engine.derive_plan(&spec, &recipe).unwrap();

    assert_eq!(backend.previews.lock().unwrap().len(), 1);
    assert_eq!(plan.package_preview.unwrap().requests.len(), 2);
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_wf_008_mismatched_command_id_is_rejected_and_recovered() {
    let base = unique_dir("mismatch");
    let project_root = base.join("project");
    make_project(&project_root);
    let original = manifest_text(&project_root);
    let bridge = FakeBridge::new();
    bridge.push(Ok(UnityResult {
        schema_version: 1,
        command_id: "some-other-id".into(),
        status: vua_orchestrator::ResultStatus::Succeeded,
        changed_paths: Vec::new(),
        diagnostics: Vec::new(),
        steps: Vec::new(),
        replayed: None,
        snapshot_id: None,
        restored_from: None,
        project_fingerprint_before: None,
        data: serde_json::Value::Null,
    }));
    let engine = engine_for(
        project_root.clone(),
        bridge.clone(),
        FakeVpmBackend::new(true),
    );
    let recipe = fixture_recipe();

    let confirmation = plan_and_confirm(&engine, &recipe);
    let error = engine.execute(&confirmation, &recipe, None).unwrap_err();
    assert_eq!(error.code, "vua.assembly.unity_failed");
    assert_eq!(manifest_text(&project_root), original, "rollback ran");
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_tst_002_unsupported_relations_are_blocked_not_silently_skipped() {
    let base = unique_dir("unsupported");
    let project_root = base.join("project");
    let bridge = FakeBridge::new();
    let engine = engine_for(project_root, bridge, FakeVpmBackend::new(true));
    let mut recipe = fixture_recipe();
    recipe.relations.push(
        serde_json::from_value(serde_json::json!({
            "id": "attach_deco",
            "kind": "attach_to_bone",
            "assetInstanceId": "outfit_blue",
            "avatarInstanceId": "avatar_root",
            "bone": "head",
            "localTransform": {
                "position": {"x": 0.0, "y": 0.1, "z": 0.0},
                "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
                "scale": {"x": 1.0, "y": 1.0, "z": 1.0}
            }
        }))
        .unwrap(),
    );

    let digest = document_digest(&recipe).unwrap();
    let spec = derive_project_spec(&recipe, &digest).unwrap();
    let error = engine.derive_plan(&spec, &recipe).expect_err("must refuse");
    assert!(error
        .iter()
        .any(|issue| issue.code == "recipe.relation_unsupported"));
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_wf_003_confirm_recomputes_the_plan_hash() {
    let base = unique_dir("hash");
    let project_root = base.join("project");
    make_project(&project_root);
    let engine = engine_for(project_root, FakeBridge::new(), FakeVpmBackend::new(true));
    let recipe = fixture_recipe();
    let confirmation = plan_and_confirm(&engine, &recipe);

    let mut tampered = confirmation.plan.clone();
    tampered.steps.pop();
    let error = engine
        .confirm(&tampered, &confirmation.plan.plan_hash)
        .expect_err("content-hash mismatch must be rejected");
    assert_eq!(error.code, "vua.assembly.plan_hash_mismatch");
    fs::remove_dir_all(&base).ok();
}

// --- runtime integration (events + journal) ---

#[test]
fn orc_con_001_assembly_runs_as_a_task_with_monotonic_step_events() {
    let base = unique_dir("runtime");
    let project_root = base.join("project");
    make_project(&project_root);
    let bridge = FakeBridge::new();
    let engine = Arc::new(engine_for(
        project_root.clone(),
        bridge.clone(),
        FakeVpmBackend::new(true),
    ));
    let recipe = Arc::new(fixture_recipe());

    let runtime = TaskRuntime::new(
        Arc::new(MemoryJournal::new()),
        Arc::new(FixedClock::new(&["2026-08-30T10:00:00.000Z"])),
        Arc::new(FixedIdGenerator::default()),
    );
    let receiver = runtime.subscribe();
    let confirmation = Arc::new(plan_and_confirm(&engine, &recipe));
    let engine_for_job = engine.clone();
    let recipe_for_job = recipe.clone();
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: Some(confirmation.correlation_id.clone()),
            timeout: Some(Duration::from_secs(60)),
            job: Box::new(move |context| {
                engine_for_job
                    .execute(&confirmation, &recipe_for_job, Some(context))
                    .map(TaskExit::Done)
            }),
        })
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last_revision = 0;
    let mut completed = false;
    while !completed {
        let budget = deadline.saturating_duration_since(Instant::now());
        let event = receiver
            .recv_timeout(budget.max(Duration::from_millis(1)))
            .expect("events must arrive");
        assert_eq!(event.task_id, accepted.task_id);
        assert!(event.revision > last_revision, "revisions stay monotonic");
        last_revision = event.revision;
        if event.kind == TaskEventKind::Completed {
            completed = true;
            assert_eq!(event.state, TaskState::Succeeded);
            assert_eq!(event.payload["stepsCompleted"], 7); // existing project: no provision step
        }
    }
    assert!(manifest_text(&project_root).contains("dev.fixture.modular-avatar"));
    fs::remove_dir_all(&base).ok();
}

// --- manual smoke against the real backend (E-ASSEMB acceptance) ---
