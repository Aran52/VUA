//! B3 executor tests: the plan state machine against fake ports, with the
//! REAL verified-snapshot store so rollback semantics are proven against
//! actual file trees. Real-Unity calibration (timeout budget, rejection
//! payloads, restore equivalence) is the separate local matrix (M3 gate).

use flate2::write::GzEncoder;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tar::{Builder, Header};

use vua_orchestrator::{
    BridgeError, BuildRecordStore, ChangePreviewV1, FileSystemSnapshotStore, FixedClock,
    MaterialEntryMode, PackageRequestV1, ProjectRef, ResultStatus, RiskDecisionChoice,
    SourceFolderInspectionV01, UnityBridge, UnityCommand, UnityResult, VpmBackend, VpmCapabilities,
};
use vua_unity_bridge::{
    LocalPackageIdentityStore, MaterialCancelToken, MaterialExecutionStatus, MaterialExecutor,
    MaterialIntakeConfirmationV01, MaterialIntakeEngine, MaterialIntakePlanV01, RiskDecisionV01,
    RollbackOutcome,
};

// --- fixtures ---

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-mexec-{label}-{nanos}"))
}

fn append(builder: &mut Builder<GzEncoder<fs::File>>, path: &str, bytes: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append_data(&mut header, path, bytes).unwrap();
}

fn unitypackage(path: &Path, assets: &[&str]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut builder = Builder::new(GzEncoder::new(file, flate2::Compression::default()));
    for asset in assets {
        append(&mut builder, asset, b"synthetic asset bytes");
    }
    builder.finish().unwrap();
}

fn make_world(label: &str) -> (PathBuf, ProjectRef) {
    let base = temp_dir(label);
    let source = base.join("source");
    fs::create_dir_all(&source).unwrap();
    unitypackage(&source.join("pack.unitypackage"), &["Assets/Asset.prefab"]);
    let project_root = base.join("target");
    fs::create_dir_all(project_root.join("Assets")).unwrap();
    fs::create_dir_all(project_root.join("Packages")).unwrap();
    fs::create_dir_all(project_root.join("ProjectSettings")).unwrap();
    // Already-provisioned target: the standing fixtures plan and execute the
    // UNCHANGED v0.1 step set (zero-change law for provisioned projects).
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
    (base, project)
}

/// An EMPTY target directory: no ProjectSettings/, no Assets/. Plans on this
/// path carry the conditional provision step (plan v0.2).
fn make_empty_target(base: &Path) -> ProjectRef {
    let project_root = base.join("empty-target");
    fs::create_dir_all(&project_root).unwrap();
    ProjectRef {
        id: "project".into(),
        root: project_root,
    }
}

fn inspection(folder: &Path) -> SourceFolderInspectionV01 {
    MaterialIntakeEngine
        .inspect_folder(folder, "corr")
        .expect("source inspects")
}

fn plan(mode: MaterialEntryMode, folder: &Path, project_root: &Path) -> MaterialIntakePlanV01 {
    MaterialIntakeEngine
        .plan(
            mode,
            "project",
            "project-fingerprint",
            inspection(folder),
            project_root,
            "corr",
        )
        .expect("plan builds")
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
        confirmed_at: "2026-09-04T00:00:00Z".into(),
        correlation_id: "corr".into(),
    }
}

// --- fake ports ---

struct FakeBridgeState {
    script: VecDeque<Result<UnityResult, BridgeError>>,
    commands: Vec<UnityCommand>,
    /// Batch 146: whether the resolve-order sentinel
    /// (`<root>/.vua/batch146-resolve-marker`, dropped by the
    /// `CreatingVpm::resolve_project` arm) existed when the command was
    /// recorded — false for every command a run issued BEFORE the resolve
    /// leg, true from the first command after it. The index pairs with
    /// `commands`.
    resolve_marker_seen: Vec<bool>,
}

#[derive(Clone)]
struct FakeBridge {
    state: Arc<Mutex<FakeBridgeState>>,
    /// S2 cancel-injection hook: cancels the shared token the moment the
    /// given operation is dispatched, so the executor's observation points
    /// that sit AFTER a Bridge command (the register/preview tail) see a
    /// cancellation decided mid-run. None in every standing test.
    cancel_on: Option<(vua_orchestrator::UnityOperation, MaterialCancelToken)>,
}

impl FakeBridge {
    fn new(script: Vec<Result<UnityResult, BridgeError>>) -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeBridgeState {
                script: script.into(),
                commands: Vec::new(),
                resolve_marker_seen: Vec::new(),
            })),
            cancel_on: None,
        }
    }

    /// S2 cancel-injection builder: fire `token.cancel()` when `operation`
    /// is dispatched.
    fn with_cancel_on(
        mut self,
        operation: vua_orchestrator::UnityOperation,
        token: MaterialCancelToken,
    ) -> Self {
        self.cancel_on = Some((operation, token));
        self
    }

    fn command_count(&self) -> usize {
        self.state.lock().unwrap().commands.len()
    }
}

impl UnityBridge for FakeBridge {
    fn execute(
        &self,
        project: &ProjectRef,
        command: &UnityCommand,
    ) -> Result<UnityResult, BridgeError> {
        let mut state = self.state.lock().unwrap();
        state.commands.push(command.clone());
        state.resolve_marker_seen.push(
            project
                .root
                .join(".vua")
                .join("batch146-resolve-marker")
                .is_file(),
        );
        // S2 cancel-injection: the request lands while the command is in
        // flight, so only an observation point after this dispatch can see
        // it — exactly the mid-run shape the new tail observations cover.
        if let Some((operation, token)) = &self.cancel_on {
            if *operation == command.operation {
                token.cancel();
            }
        }
        // The real Bridge produces the package layout inside the staging
        // project at Packages/<packageId>/; the fake reproduces just enough
        // of that side effect for the deterministic publication step.
        if command.operation == vua_orchestrator::UnityOperation::CreateLocalVpmPackage {
            let package_id = command.payload.package_id.clone().unwrap_or_default();
            let package_dir = project.root.join("Packages").join(&package_id);
            fs::create_dir_all(package_dir.join("Runtime")).unwrap();
            fs::write(
                package_dir.join("package.json"),
                serde_json::json!({ "name": "synthetic.local", "version": "0.1.0" }).to_string(),
            )
            .unwrap();
            // A loadable asset plus its .meta sidecar: the validation list
            // must carry the asset but never the sidecar (review P1: meta
            // files are not loadable assets).
            fs::write(
                package_dir.join("Runtime").join("Asset.prefab"),
                "synthetic",
            )
            .unwrap();
            fs::write(
                package_dir.join("Runtime").join("Asset.prefab.meta"),
                "meta",
            )
            .unwrap();
        }
        // The real Bridge materializes the extracted layout into Assets/;
        // the fake mirrors that side effect so restores are observable.
        if command.operation == vua_orchestrator::UnityOperation::ImportUnityPackage {
            if let Some(extracted) = &command.payload.source_package_path {
                for entry in fs::read_dir(Path::new(extracted)).into_iter().flatten() {
                    let guid_dir = entry.expect("dir entry").path();
                    let logical = match fs::read_to_string(guid_dir.join("pathname")) {
                        Ok(logical) => logical.trim().to_owned(),
                        Err(_) => continue,
                    };
                    let target = project.root.join(&logical);
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent).unwrap();
                    }
                    fs::copy(guid_dir.join("asset"), &target).unwrap();
                    let _ = fs::copy(guid_dir.join("asset.meta"), target.with_extension("meta"));
                }
            }
        }
        match state.script.pop_front() {
            Some(outcome) => outcome,
            None => {
                // 第 158 批（BOARD #45(2)）：the executor now parses a
                // Succeeded validate receipt's loadedAssetPaths strictly
                // (missing field = honest bridge_failed), so the default
                // fake mirrors the real C# handler: the Ordinal-sorted
                // expected list IS the loaded list on success.
                let mut data = serde_json::json!({
                    "projectFingerprint": format!("fp-{}", state.commands.len())
                });
                if command.operation == vua_orchestrator::UnityOperation::ValidateAssetPaths {
                    let mut loaded = command.payload.expected_asset_paths.clone();
                    loaded.sort();
                    data["loadedAssetPaths"] =
                        loaded.into_iter().map(serde_json::Value::String).collect();
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
    }
}

struct FakeVpm {
    installs: AtomicUsize,
    registrations: AtomicUsize,
    /// S2 cancel-injection hook: a token cancelled inside preview_install,
    /// so only the second tail observation point (between preview and
    /// apply) can still see the request. None in every standing test.
    cancel_on_preview: Option<MaterialCancelToken>,
}

impl FakeVpm {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            installs: AtomicUsize::new(0),
            registrations: AtomicUsize::new(0),
            cancel_on_preview: None,
        })
    }

    /// S2 cancel-injection constructor: cancel the token from inside
    /// preview_install.
    fn new_cancelling_on_preview(token: MaterialCancelToken) -> Arc<Self> {
        Arc::new(Self {
            installs: AtomicUsize::new(0),
            registrations: AtomicUsize::new(0),
            cancel_on_preview: Some(token),
        })
    }
}

impl VpmBackend for FakeVpm {
    fn name(&self) -> &'static str {
        "fake-vpm"
    }

    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: false,
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
        if let Some(token) = &self.cancel_on_preview {
            token.cancel();
        }
        Ok(ChangePreviewV1 {
            items: vec![],
            conflicts: vec![],
            remove_legacy_files: vec![],
            remove_legacy_folders: vec![],
            destructive: false,
            digest: format!("digest-{}", packages.len()),
        })
    }

    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        confirmed_digest: &str,
    ) -> Result<serde_json::Value, vua_orchestrator::AppErrorV1> {
        self.installs.fetch_add(1, Ordering::SeqCst);
        assert_eq!(
            confirmed_digest, "digest-1",
            "apply binds the confirmed digest"
        );
        Ok(serde_json::json!({ "installed": true }))
    }

    fn register_local_package(
        &self,
        _package_root: &Path,
    ) -> Result<(), vua_orchestrator::AppErrorV1> {
        self.registrations.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn create_project(
        &self,
        _parent: &Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, vua_orchestrator::AppErrorV1> {
        // The bundled-template path never asks a backend to create projects.
        Err(vua_orchestrator::AppErrorV1::new(
            "vua.vpm.capability_missing",
            vua_orchestrator::ErrorCategory::Unavailable,
            "errors.vpm.capabilityMissing",
            "corr",
        ))
    }
}

fn executor(base: &Path, bridge: FakeBridge, vpm: Arc<FakeVpm>) -> MaterialExecutor {
    MaterialExecutor::new(
        Arc::new(bridge),
        FileSystemSnapshotStore,
        vpm,
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-04T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    )
}

// --- the state machine ---

#[test]
fn b3_exec_001_direct_mode_happy_path_and_idempotent_replay() {
    let (base, project) = make_world("direct-happy");
    let source = base.join("source");
    let bridge = FakeBridge::new(vec![]);
    let vpm = FakeVpm::new();
    let executor = executor(&base, bridge.clone(), vpm.clone());

    let confirmation = confirmation(&plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    ));
    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(report.status, MaterialExecutionStatus::Succeeded);
    assert!(!report.replayed);
    assert_eq!(
        report.completed_steps,
        vec![
            vua_unity_bridge::MaterialIntakeStepKind::VerifySource,
            vua_unity_bridge::MaterialIntakeStepKind::CreateSnapshot,
            vua_unity_bridge::MaterialIntakeStepKind::ImportUnityPackages,
            vua_unity_bridge::MaterialIntakeStepKind::ValidateMinimumStructure,
            vua_unity_bridge::MaterialIntakeStepKind::WriteBuildRecord,
        ]
    );
    assert_eq!(report.rollback, RollbackOutcome::NotNeeded);

    // One import + one validation command; mutating command binds the
    // plan's project fingerprint and the package digests.
    assert_eq!(bridge.command_count(), 2);
    let commands = bridge.state.lock().unwrap().commands.clone();
    assert_eq!(
        commands[0].operation,
        vua_orchestrator::UnityOperation::MaterializeExtractedPackage
    );
    assert_eq!(
        commands[0].expected_project_fingerprint.as_deref(),
        Some("project-fingerprint")
    );
    assert_eq!(
        commands[0].payload.source_package_sha256.as_deref(),
        Some(inspection(&source).packages[0].sha256.as_str())
    );
    assert_eq!(
        commands[1].operation,
        vua_orchestrator::UnityOperation::ValidateAssetPaths
    );
    assert!(commands[1].dry_run);

    // The receipt replays idempotently: no further Unity traffic.
    let replay = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );
    assert!(replay.replayed);
    assert_eq!(replay.status, MaterialExecutionStatus::Succeeded);
    assert_eq!(bridge.command_count(), 2, "replay must not touch Unity");

    // The receipt record carries the final fingerprint the fake returned.
    let record = vua_orchestrator::BuildRecordStore::new(base.join("records"))
        .read(&format!("material-{}", confirmation.plan.plan_id))
        .expect("receipt published");
    assert_eq!(record.final_project_fingerprint.as_deref(), Some("fp-1"));
    // Note: the flat fixture archive carries no guid/pathname entries, so
    // the expectations list — and hence the evidence list — is honestly
    // empty here; the non-empty evidence path is pinned by
    // b3_batch158_validation_evidence_carries_the_loaded_list_into_the_record.
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

// --- batch 148 reverse-review pins ---

fn scripted_result(fingerprint: Option<&str>) -> UnityResult {
    UnityResult {
        schema_version: 1,
        command_id: "scripted".into(),
        status: ResultStatus::Succeeded,
        changed_paths: vec![],
        diagnostics: vec![],
        steps: Vec::new(),
        replayed: None,
        snapshot_id: None,
        restored_from: None,
        project_fingerprint_before: None,
        data: match fingerprint {
            Some(value) => serde_json::json!({ "projectFingerprint": value }),
            None => serde_json::json!({}),
        },
    }
}

/// 第 148 批（重试撞号缺陷）：Bridge 把 mutating 成功回执按 commandId 持久
/// 存于 `<project>/.vua/bridge/completed/`（快照作用域之外，回滚不清除），
/// 同 id 同内容即回放既有收据而**不重执行**。plan_id 跨 attempt 相同、
/// bridge_jobs.len() 每次 execute 从零起算——不加盐时重试会撞出同 id，
/// 已完成包命中幽灵回放（文件已被回滚清掉）后，指纹链在新包上必然
/// stale_project 拒绝，重试永久卡死。钉：首 attempt 的 id 保持历史形态；
/// 重试 attempt 的 mutating id 注入 `-r{attempt}` 盐，绝不与上一 attempt
/// 重合；只读 validate 不进回执库，id 不加盐。
#[test]
fn b3_batch148_retry_attempt_salts_mutating_command_ids_against_ghost_replay() {
    let (base, project) = make_world("retry-salt");
    let source = base.join("source");
    unitypackage(&source.join("one.unitypackage"), &["Assets/One.prefab"]);
    let confirmation = confirmation(&plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    ));
    // Attempt 1: import #0 succeeds (fp-1), import #1 times out → rollback.
    let bridge = FakeBridge::new(vec![
        Ok(scripted_result(Some("fp-1"))),
        Err(BridgeError::TimedOut),
    ]);
    let executor_first = executor(&base, bridge.clone(), FakeVpm::new());
    let first = executor_first.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(first.status, MaterialExecutionStatus::Failed);
    assert_eq!(first.rollback, RollbackOutcome::Restored);
    let first_ids: Vec<String> = bridge
        .state
        .lock()
        .unwrap()
        .commands
        .iter()
        .map(|command| command.command_id.clone())
        .collect();
    assert!(
        first_ids
            .iter()
            .any(|id| id == &format!("{}-import-0", confirmation.plan.plan_id)),
        "first attempt keeps the historical id shape (no salt)"
    );
    assert!(first_ids.iter().all(|id| !id.contains("-r")));

    // Retry the SAME confirmation (same plan → same plan_id): attempt 2.
    let bridge_retry = FakeBridge::new(vec![]);
    let executor_retry = executor(&base, bridge_retry.clone(), FakeVpm::new());
    let second = executor_retry.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(second.status, MaterialExecutionStatus::Succeeded);
    assert_eq!(second.rollback, RollbackOutcome::NotNeeded);
    let retry_import_ids: Vec<String> = bridge_retry
        .state
        .lock()
        .unwrap()
        .commands
        .iter()
        .filter(|command| {
            command.operation == vua_orchestrator::UnityOperation::MaterializeExtractedPackage
        })
        .map(|command| command.command_id.clone())
        .collect();
    assert_eq!(retry_import_ids.len(), 2);
    assert!(
        retry_import_ids.iter().all(|id| !first_ids.contains(id)),
        "retried mutating ids must never collide with the prior attempt (ghost replay)"
    );
    for (index, id) in retry_import_ids.iter().enumerate() {
        assert_eq!(
            id,
            &format!("{}-r2-import-{index}", confirmation.plan.plan_id),
            "retry salt shape"
        );
    }
    // The read-only validate command keeps the plain id.
    assert!(bridge_retry
        .state
        .lock()
        .unwrap()
        .commands
        .iter()
        .any(|command| command.command_id == format!("{}-validate", confirmation.plan.plan_id)));
    // And the retry publishes under the attempt-2 receipt id.
    assert_eq!(
        second.build_record_id.as_deref(),
        Some(format!("material-{}-attempt2", confirmation.plan.plan_id).as_str())
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

/// 第 148 批（指纹硬要求诚实化）：成功 mutating 命令必须携带
/// `data.projectFingerprint`——缺失或空串是收据形状漂移（#36 族的表亲）。
/// 旧代码 `unwrap_or_else(保持旧值)` 会静默骑旧链，末包场景更是把**假**的
/// final 指纹写进回执。钉：形状漂移在这里诚实失败，回滚照跑，回执照发，
/// 且 final 指纹保持空（绝不落假值）。
#[test]
fn b3_batch148_mutating_success_without_fingerprint_fails_honestly() {
    let (base, project) = make_world("fp-shape-drift");
    let source = base.join("source");
    let bridge = FakeBridge::new(vec![Ok(scripted_result(None))]);
    let executor = executor(&base, bridge, FakeVpm::new());
    let confirmation = confirmation(&plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    ));

    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(report.status, MaterialExecutionStatus::Failed);
    assert!(
        report
            .error_code
            .as_deref()
            .unwrap_or("")
            .starts_with("vua.material.bridge_failed"),
        "shape drift fails under the standing bridge_failed family: {:?}",
        report.error_code
    );
    assert_eq!(report.rollback, RollbackOutcome::Restored);
    // The receipt is still published — and records NO final fingerprint.
    let record = vua_orchestrator::BuildRecordStore::new(base.join("records"))
        .read(
            report
                .build_record_id
                .as_deref()
                .expect("receipt published"),
        )
        .expect("receipt readable");
    assert_eq!(record.status, vua_orchestrator::BuildRecordStatus::Failed);
    assert_eq!(record.final_project_fingerprint, None);
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

// --- batch 158 pins (BOARD #45(2) evidence face + #45(4) residue policy) ---

/// 第 158 批（BOARD #45(2) 证据面收口）：旧代码对 Succeeded validate 回执缺
/// `data.loadedAssetPaths` 时 `unwrap_or_default` 静默记成「已验证、零素材
/// 加载」——Bridge 从未报告过的空清单被写成证据。钉：缺字段（臂一）与含
/// 非字符串条目（臂二）都按 bridge_failed 族诚实失败，回滚照跑、Failed
/// 回执照发，validation 节保持 None（从未宣称过验证证据）。
#[test]
fn b3_batch158_validate_success_without_loaded_asset_paths_evidence_fails_honestly() {
    for (label, data) in [
        (
            "missing-field",
            serde_json::json!({ "projectFingerprint": "fp-validate" }),
        ),
        (
            "non-string-entry",
            serde_json::json!({
                "projectFingerprint": "fp-validate",
                "loadedAssetPaths": ["Assets/Asset.prefab", 42],
            }),
        ),
    ] {
        let (base, project) = make_world(label);
        let source = base.join("source");
        // Script: the import leg gets a scripted success; the validate leg
        // gets a Succeeded receipt WITHOUT usable loadedAssetPaths evidence.
        let bridge = FakeBridge::new(vec![
            Ok(scripted_result(Some("fp-import"))),
            Ok(UnityResult {
                schema_version: 1,
                command_id: "validate-no-evidence".into(),
                status: ResultStatus::Succeeded,
                changed_paths: vec![],
                diagnostics: vec![],
                steps: Vec::new(),
                replayed: None,
                snapshot_id: None,
                restored_from: None,
                project_fingerprint_before: None,
                data,
            }),
        ]);
        let executor = executor(&base, bridge, FakeVpm::new());
        let confirmation = confirmation(&plan(
            MaterialEntryMode::DirectUnityPackage,
            &source,
            &project.root,
        ));

        let report = executor.execute(
            &confirmation,
            &source,
            &project,
            &base.join("artifacts"),
            &MaterialCancelToken::new(),
        );

        assert_eq!(report.status, MaterialExecutionStatus::Failed, "{label}");
        assert!(
            report
                .error_code
                .as_deref()
                .unwrap_or("")
                .starts_with("vua.material.bridge_failed"),
            "{label}: fails under the standing bridge_failed family: {:?}",
            report.error_code
        );
        assert_eq!(report.rollback, RollbackOutcome::Restored, "{label}");
        let record = vua_orchestrator::BuildRecordStore::new(base.join("records"))
            .read(
                report
                    .build_record_id
                    .as_deref()
                    .expect("receipt published"),
            )
            .expect("receipt readable");
        assert_eq!(
            record.status,
            vua_orchestrator::BuildRecordStatus::Failed,
            "{label}"
        );
        assert_eq!(
            record.validation, None,
            "{label}: no validation section is fabricated"
        );
        if base.exists() {
            fs::remove_dir_all(&base).unwrap();
        }
    }
}

/// 第 158 批（BOARD #45(4) 残留清理策略）三钉合一：
/// ① 链开始前清扫——上一运行（模拟进程死亡）留在 `.vua/imports` 的解包
///    残留在新链第一次解包前被清走（唯一没有在跑代码能清理的路径；安全性
///    依据＝MutationGate 全程持有，同项目根无并发链）；
/// ② 失败臂接线——本运行解包落地后导入失败（超时），失败出口清走本运行
///    自己的解包根：失败运行不再留下 `.vua/imports` 磁盘残留（回滚只恢复
///    作用域面 Assets/Packages/ProjectSettings/vpm-manifest.json，本就管
///    不到 `.vua`）；
/// ③ 清扫有界——`.vua/snapshots` 下的哨兵文件不被波及。
#[test]
fn b3_batch158_import_extractions_leave_no_residue_on_failure() {
    let (base, project) = make_world("residue");
    let source = base.join("source");
    // Stale residue from a run that died mid-chain, plus a sentinel OUTSIDE
    // the sweep scope that must survive.
    let stale = project
        .root
        .join(".vua")
        .join("imports")
        .join("material-staleplan-import-99");
    fs::create_dir_all(&stale).unwrap();
    fs::write(stale.join("stale.bin"), "stale").unwrap();
    let sentinel = project
        .root
        .join(".vua")
        .join("snapshots")
        .join("keep-me.txt");
    fs::create_dir_all(sentinel.parent().unwrap()).unwrap();
    fs::write(&sentinel, "sentinel").unwrap();

    // The import leg times out AFTER this run's extraction has landed.
    let bridge = FakeBridge::new(vec![Err(BridgeError::TimedOut)]);
    let executor = executor(&base, bridge, FakeVpm::new());
    let confirmation = confirmation(&plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    ));

    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(report.status, MaterialExecutionStatus::Failed);
    let imports_root = project.root.join(".vua").join("imports");
    let leftovers: Vec<std::path::PathBuf> = fs::read_dir(&imports_root)
        .expect("imports root still exists")
        .flatten()
        .map(|entry| entry.path())
        .collect();
    assert!(
        leftovers.is_empty(),
        "no extraction residue survives a failed run: {leftovers:?}"
    );
    assert_eq!(fs::read_to_string(&sentinel).unwrap(), "sentinel");
    let record = vua_orchestrator::BuildRecordStore::new(base.join("records"))
        .read(
            report
                .build_record_id
                .as_deref()
                .expect("receipt published"),
        )
        .expect("receipt readable");
    assert_eq!(record.status, vua_orchestrator::BuildRecordStatus::Failed);
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

/// 第 158 批（BOARD #45(2)）正例钉：guid 布局归档（真实 .unitypackage 形态，
/// 平面夹具归档从不产生非空期望）下，validate 成功回执的 loadedAssetPaths
/// 被严格解析并逐字进入 build record 的 validation 节——证据携带 Bridge
/// 实际报告的清单（fake 照 C# 面同形：Ordinal 排序的期望清单）。
#[test]
fn b3_batch158_validation_evidence_carries_the_loaded_list_into_the_record() {
    let (base, project) = make_world("batch158-evidence");
    let source = base.join("source");
    // Replace the flat fixture archive with a guid-layout one; the plan
    // helper re-inspects, so the expectations (and the echoed evidence)
    // are non-empty.
    guid_layout_package(
        &source.join("pack.unitypackage"),
        &[
            ("0123456789abcdef0123456789abcdef", "Assets/second.prefab"),
            ("ffffffffffffffffffffffffffffffff", "Assets/first.prefab"),
        ],
    );
    let plan = plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    );
    let bridge = FakeBridge::new(vec![]);
    let executor = executor(&base, bridge, FakeVpm::new());

    let report = executor.execute(
        &confirmation(&plan),
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(report.status, MaterialExecutionStatus::Succeeded);
    let record = vua_orchestrator::BuildRecordStore::new(base.join("records"))
        .read(&format!("material-{}", plan.plan_id))
        .expect("receipt published");
    let validation = record
        .validation
        .as_ref()
        .expect("validation evidence present");
    assert!(validation.unity_validated);
    // Ordinal-sorted, exactly as the C# handler reports it.
    assert_eq!(
        validation.expected_assets_loaded,
        vec![
            "Assets/first.prefab".to_owned(),
            "Assets/second.prefab".to_owned()
        ]
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_exec_002_source_drift_fails_before_the_first_mutation() {
    let (base, project) = make_world("drift");
    let source = base.join("source");
    let bridge = FakeBridge::new(vec![]);
    let executor = executor(&base, bridge.clone(), FakeVpm::new());

    let plan = plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    );
    let confirmation = confirmation(&plan);
    // Drift: the source grows after Inspect/plan.
    unitypackage(&source.join("late.unitypackage"), &["Assets/Late.prefab"]);

    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(report.status, MaterialExecutionStatus::Failed);
    assert!(
        report
            .error_code
            .as_deref()
            .unwrap_or("")
            .contains("source_drift"),
        "the drift code is the finding: {:?}",
        report.error_code
    );
    assert_eq!(
        bridge.command_count(),
        0,
        "no Unity command may run on drift"
    );
    assert_eq!(
        report.rollback,
        RollbackOutcome::NotNeeded,
        "nothing mutated yet"
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_exec_003_cancellation_before_the_first_step_touches_nothing() {
    let (base, project) = make_world("cancel");
    let source = base.join("source");
    let bridge = FakeBridge::new(vec![]);
    let executor = executor(&base, bridge.clone(), FakeVpm::new());

    let confirmation = confirmation(&plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    ));
    let token = MaterialCancelToken::new();
    token.cancel();
    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &token,
    );

    assert_eq!(report.status, MaterialExecutionStatus::Cancelled);
    assert_eq!(report.completed_steps, vec![]);
    assert_eq!(bridge.command_count(), 0);
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_exec_004_bridge_rejection_restores_the_verified_snapshot() {
    let (base, project) = make_world("reject");
    let source = base.join("source");
    // First command (the import) is rejected by the real Bridge semantics.
    let bridge = FakeBridge::new(vec![Ok(UnityResult {
        schema_version: 1,
        command_id: "rejected".into(),
        status: ResultStatus::Rejected,
        changed_paths: vec![],
        diagnostics: vec![],
        data: serde_json::json!({}),
        steps: Vec::new(),
        replayed: None,
        snapshot_id: None,
        restored_from: None,
        project_fingerprint_before: None,
    })]);
    let executor = executor(&base, bridge, FakeVpm::new());

    let manifest_before = fs::read_to_string(project.root.join("vpm-manifest.json")).unwrap();
    let confirmation = confirmation(&plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    ));
    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(report.status, MaterialExecutionStatus::Failed);
    assert_eq!(
        report.error_code.as_deref(),
        Some("vua.material.bridge_rejected")
    );
    assert_eq!(report.rollback, RollbackOutcome::Restored);

    // The receipt is published for the failed run too — it is what makes a
    // later re-run a replay instead of a blind second mutation.
    let record = vua_orchestrator::BuildRecordStore::new(base.join("records"))
        .read(&format!("material-{}", confirmation.plan.plan_id))
        .expect("failed runs are recorded");
    assert_eq!(record.status, vua_orchestrator::BuildRecordStatus::Failed);
    let snapshot = record.snapshot.expect("snapshot evidence");
    assert!(snapshot.restore_attempted);
    assert_eq!(snapshot.restore_succeeded, Some(true));

    // The restore must leave the project as it was.
    assert_eq!(
        fs::read_to_string(project.root.join("vpm-manifest.json")).unwrap(),
        manifest_before
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_exec_005_bridge_timeout_is_a_typed_failure_with_restore() {
    let (base, project) = make_world("timeout");
    let source = base.join("source");
    let bridge = FakeBridge::new(vec![Err(BridgeError::TimedOut)]);
    let executor = executor(&base, bridge, FakeVpm::new());

    let confirmation = confirmation(&plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    ));
    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(report.status, MaterialExecutionStatus::Failed);
    assert_eq!(
        report.error_code.as_deref(),
        Some("vua.material.bridge_timeout")
    );
    assert_eq!(report.rollback, RollbackOutcome::Restored);
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_exec_006_vpm_mode_runs_the_staging_contract_and_cleans_up() {
    let (base, project) = make_world("vpm-mode");
    let source = base.join("source");
    // Curated dependency declarations travel verbatim into the produced
    // package's package.json — never auto-detected.
    fs::write(
        source.join("vua-dependencies.json"),
        r#"{ "com.vrchat.avatars": "3.10.x", "nadena.dev.modular-avatar": "^1.13.0" }"#,
    )
    .unwrap();
    let vpm = FakeVpm::new();
    let bridge = FakeBridge::new(vec![]);
    let executor = executor(&base, bridge.clone(), vpm.clone());

    let confirmation = confirmation(&plan(
        MaterialEntryMode::LocalReusableVpm,
        &source,
        &project.root,
    ));
    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(report.status, MaterialExecutionStatus::Succeeded);
    let steps = &report.completed_steps;
    for expected in [
        vua_unity_bridge::MaterialIntakeStepKind::CreateLocalVpmPackage,
        vua_unity_bridge::MaterialIntakeStepKind::PreviewVpmInstall,
        vua_unity_bridge::MaterialIntakeStepKind::ApplyVpmInstall,
    ] {
        assert!(steps.contains(&expected), "missing {expected:?}");
    }

    // The Bridge command sequence inside staging: inspect → import →
    // create; all scoped to the staging project and token-bound. The final
    // validation then runs against the TARGET project.
    let commands = bridge.state.lock().unwrap().commands.clone();
    assert_eq!(commands.len(), 4);
    assert_eq!(
        commands[0].operation,
        vua_orchestrator::UnityOperation::InspectProject
    );
    assert_eq!(
        commands[1].operation,
        vua_orchestrator::UnityOperation::MaterializeExtractedPackage
    );
    assert_eq!(
        commands[2].operation,
        vua_orchestrator::UnityOperation::CreateLocalVpmPackage
    );
    for command in &commands[..3] {
        assert!(command.project_id.ends_with("-staging"));
    }
    assert_eq!(
        commands[3].operation,
        vua_orchestrator::UnityOperation::ValidateAssetPaths
    );
    assert_eq!(commands[3].project_id, "project");
    assert_eq!(
        commands[2].payload.staging_token.as_deref(),
        Some(confirmation.correlation_id.as_str())
    );
    // Declared dependencies reach the Bridge payload verbatim, sorted.
    let validated = &commands[3].payload.expected_asset_paths;
    assert!(
        validated
            .iter()
            .any(|path| path.ends_with("Runtime/Asset.prefab")),
        "loadable assets are validated: {validated:?}"
    );
    assert!(
        validated.iter().all(|path| !path.ends_with(".meta")),
        "meta sidecars are never validated as assets: {validated:?}"
    );
    let declared: Vec<(String, String)> = commands[2]
        .payload
        .package_dependencies
        .iter()
        .map(|dependency| (dependency.package_id.clone(), dependency.version.clone()))
        .collect();
    assert_eq!(
        declared,
        vec![
            ("com.vrchat.avatars".to_owned(), "3.10.x".to_owned()),
            ("nadena.dev.modular-avatar".to_owned(), "^1.13.0".to_owned()),
        ]
    );
    // The staging import chained the staging fingerprint from the inspect
    // (the fake bridge numbers its returned fingerprints fp-1, fp-2, …).
    assert_eq!(
        commands[1].expected_project_fingerprint.as_deref(),
        Some("fp-1")
    );
    assert_eq!(
        commands[2].expected_project_fingerprint.as_deref(),
        Some("fp-2")
    );

    // The package was registered and installed with the bound digest.
    assert_eq!(vpm.registrations.load(Ordering::SeqCst), 1);
    assert_eq!(vpm.installs.load(Ordering::SeqCst), 1);

    // The staging directory is destroyed on the success path.
    let staging =
        vua_unity_bridge::staging_root(&base.join("temp"), "corr").expect("valid session id");
    assert!(!staging.exists(), "staging leftovers poison later runs");

    // The receipt carries the local VPM evidence.
    let record = vua_orchestrator::BuildRecordStore::new(base.join("records"))
        .read(&format!("material-{}", confirmation.plan.plan_id))
        .expect("receipt published");
    let local_vpm = record.local_vpm.expect("local vpm evidence");
    assert!(
        local_vpm
            .package_id
            .starts_with("com.ph-r.vua.local.source."),
        "machine id comes from the identity store: {}",
        local_vpm.package_id
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

// --- review findings: failed receipts never replay as success; same-name
// sources get distinct machine identities ---

#[test]
fn b3_exec_007_failed_receipt_is_never_replayed_as_success() {
    let (base, project) = make_world("retry-after-failure");
    let source = base.join("source");

    // Attempt 1: the real Bridge rejects the import; the receipt records
    // the failure.
    let rejection: Vec<Result<UnityResult, BridgeError>> = vec![Ok(UnityResult {
        schema_version: 1,
        command_id: "rejected".into(),
        status: ResultStatus::Rejected,
        changed_paths: vec![],
        diagnostics: vec![],
        data: serde_json::json!({}),
        steps: Vec::new(),
        replayed: None,
        snapshot_id: None,
        restored_from: None,
        project_fingerprint_before: None,
    })];
    let bridge = FakeBridge::new(rejection);
    let first_executor = executor(&base, bridge.clone(), FakeVpm::new());
    let confirmation = confirmation(&plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    ));
    let first = first_executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(first.status, MaterialExecutionStatus::Failed);
    let failed_record_id = first.build_record_id.expect("failed run records");

    // Attempt 2: a retry runs FRESH — Unity is invoked again, and the new
    // receipt is published under the next free attempt id without touching
    // the failed one.
    let bridge = FakeBridge::new(vec![]);
    let retry_executor = executor(&base, bridge.clone(), FakeVpm::new());
    let second = retry_executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );
    println!("second: {second:?}");

    assert_eq!(second.status, MaterialExecutionStatus::Succeeded);
    assert!(
        !second.replayed,
        "a failed receipt must never replay as success"
    );
    assert_eq!(bridge.command_count(), 2, "the retry really executed");
    assert_ne!(
        second.build_record_id.as_deref(),
        Some(failed_record_id.as_str())
    );

    let store = vua_orchestrator::BuildRecordStore::new(base.join("records"));
    let failed = store
        .read(&failed_record_id)
        .expect("failed receipt intact");
    assert_eq!(failed.status, vua_orchestrator::BuildRecordStatus::Failed);
    let retried = store
        .read(second.build_record_id.as_deref().unwrap())
        .unwrap();
    assert_eq!(
        retried.status,
        vua_orchestrator::BuildRecordStatus::Succeeded
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_exec_008_same_name_sources_resolve_distinct_machine_identities() {
    let base = temp_dir("identity");
    let source_one = base.join("one").join("source");
    let source_two = base.join("two").join("source");
    for folder in [&source_one, &source_two] {
        fs::create_dir_all(folder).unwrap();
        unitypackage(&folder.join("pack.unitypackage"), &["Assets/Asset.prefab"]);
    }
    let store = LocalPackageIdentityStore::new(base.join("identities.json"));

    let identity_one = store.resolve(&source_one, "source").expect("identity one");
    let identity_two = store.resolve(&source_two, "source").expect("identity two");

    assert_ne!(identity_one.package_id, identity_two.package_id);
    assert_eq!(identity_one.display_name, "source");
    assert_eq!(identity_two.display_name, "source (2)");
    // Stable across repeat resolution.
    let again = store.resolve(&source_one, "source").unwrap();
    assert_eq!(again.package_id, identity_one.package_id);
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

/// Simulates the worst production failure: the recovery point vanishes
/// between CreateSnapshot and the restore attempt (e.g. disk loss under
/// `.vua/`). The first command fails AND takes the snapshot with it, so the
/// executor's restore itself errors — the receipt must still be published.
struct SnapshotDestroyingBridge {
    inner: FakeBridge,
}

impl UnityBridge for SnapshotDestroyingBridge {
    fn execute(
        &self,
        project: &ProjectRef,
        command: &UnityCommand,
    ) -> Result<UnityResult, BridgeError> {
        let snapshots = project.root.join(".vua/snapshots");
        if snapshots.is_dir() {
            for entry in fs::read_dir(&snapshots).into_iter().flatten() {
                let _ = fs::remove_dir_all(entry.expect("dir entry").path());
            }
        }
        self.inner.execute(project, command)
    }
}

#[test]
fn b3_exec_007_restore_failure_still_publishes_the_receipt() {
    let (base, project) = make_world("restore-fail");
    let source = base.join("source");
    let bridge = FakeBridge::new(vec![Err(BridgeError::TimedOut)]);
    let executor = MaterialExecutor::new(
        Arc::new(SnapshotDestroyingBridge { inner: bridge }),
        FileSystemSnapshotStore,
        FakeVpm::new(),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-04T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    );

    let confirmation = confirmation(&plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    ));
    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(report.status, MaterialExecutionStatus::Failed);
    assert!(
        report
            .error_code
            .as_deref()
            .unwrap_or("")
            .starts_with("vua.material.rollback_failed"),
        "the rollback_failed code is the finding: {:?}",
        report.error_code
    );
    assert_eq!(report.rollback, RollbackOutcome::Failed);

    // The worst outcome is precisely when an immutable receipt matters: the
    // record must carry restore_attempted=true, restore_succeeded=false.
    let record = vua_orchestrator::BuildRecordStore::new(base.join("records"))
        .read(&format!("material-{}", confirmation.plan.plan_id))
        .expect("a failed restore must still be recorded");
    assert_eq!(record.status, vua_orchestrator::BuildRecordStatus::Failed);
    let snapshot = record.snapshot.expect("snapshot evidence");
    assert!(snapshot.restore_attempted);
    assert_eq!(snapshot.restore_succeeded, Some(false));
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_exec_008_editing_the_declarations_after_planning_is_drift() {
    let (base, project) = make_world("deps-drift");
    let source = base.join("source");
    fs::write(
        source.join("vua-dependencies.json"),
        r#"{ "com.vrchat.avatars": "3.10.x" }"#,
    )
    .unwrap();
    let bridge = FakeBridge::new(vec![]);
    let executor = executor(&base, bridge.clone(), FakeVpm::new());

    let plan = plan(MaterialEntryMode::LocalReusableVpm, &source, &project.root);
    let confirmation = confirmation(&plan);

    // The user edits the declarations after planning: the produced
    // package.json would differ from what was confirmed, so the run must
    // refuse before the first mutation.
    fs::write(
        source.join("vua-dependencies.json"),
        r#"{ "com.vrchat.avatars": "3.12.x" }"#,
    )
    .unwrap();

    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(report.status, MaterialExecutionStatus::Failed);
    assert!(
        report
            .error_code
            .as_deref()
            .unwrap_or("")
            .contains("source_drift"),
        "declaration edits are source drift: {:?}",
        report.error_code
    );
    assert_eq!(
        bridge.command_count(),
        0,
        "no Unity command may run on drift"
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

// --- W25 provision vectors (plan v0.2) ---

/// Creates a minimal valid project skeleton and records the call — the
/// successful-provisioning fake (the vrc-get lib template copy, faked to the
/// smallest observable shape). Batch 146: also implements the resolve leg —
/// it counts the calls and drops the ordering sentinel
/// (`<root>/.vua/batch146-resolve-marker`) so the tests can prove the
/// baseline re-read happened AFTER the resolve.
struct CreatingVpm {
    creates: AtomicUsize,
    resolves: AtomicUsize,
    /// S2 cancel-injection hook: a token cancelled inside create_project,
    /// so only the between-create-and-resolve observation point can still
    /// see the request. None in every standing test.
    cancel_on_create: Option<MaterialCancelToken>,
}

impl CreatingVpm {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            creates: AtomicUsize::new(0),
            resolves: AtomicUsize::new(0),
            cancel_on_create: None,
        })
    }

    /// S2 cancel-injection constructor: cancel the token from inside
    /// create_project.
    fn new_cancelling_on_create(token: MaterialCancelToken) -> Arc<Self> {
        Arc::new(Self {
            creates: AtomicUsize::new(0),
            resolves: AtomicUsize::new(0),
            cancel_on_create: Some(token),
        })
    }
}

impl VpmBackend for CreatingVpm {
    fn name(&self) -> &'static str {
        "creating-vpm"
    }

    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: true,
            preview_install: true,
            list_packages: false,
            remove_packages: false,
            project_registry: false,
            resolve_project: true,
        }
    }

    fn preview_install(
        &self,
        _project: &ProjectRef,
        packages: &[PackageRequestV1],
    ) -> Result<ChangePreviewV1, vua_orchestrator::AppErrorV1> {
        Ok(ChangePreviewV1 {
            items: vec![],
            conflicts: vec![],
            remove_legacy_files: vec![],
            remove_legacy_folders: vec![],
            destructive: false,
            digest: format!("digest-{}", packages.len()),
        })
    }

    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        confirmed_digest: &str,
    ) -> Result<serde_json::Value, vua_orchestrator::AppErrorV1> {
        assert_eq!(confirmed_digest, "digest-1");
        Ok(serde_json::json!({ "installed": true }))
    }

    fn create_project(
        &self,
        parent: &Path,
        name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, vua_orchestrator::AppErrorV1> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        if let Some(token) = &self.cancel_on_create {
            token.cancel();
        }
        let root = parent.join(name);
        fs::create_dir_all(root.join("ProjectSettings")).unwrap();
        fs::write(
            root.join("ProjectSettings").join("ProjectVersion.txt"),
            "2022.3.22f1",
        )
        .unwrap();
        fs::create_dir_all(root.join("Assets")).unwrap();
        Ok(ProjectRef {
            id: name.to_owned(),
            root,
        })
    }

    /// The batch-146 resolve leg: counted, and it drops the ordering
    /// sentinel so a later test can verify the baseline re-read (the first
    /// Bridge command) happened after the resolve.
    fn resolve_project(
        &self,
        project_root: &Path,
    ) -> Result<vua_orchestrator::ResolveReceiptV01, vua_orchestrator::AppErrorV1> {
        self.resolves.fetch_add(1, Ordering::SeqCst);
        let marker_dir = project_root.join(".vua");
        fs::create_dir_all(&marker_dir).unwrap();
        fs::write(marker_dir.join("batch146-resolve-marker"), b"resolved").unwrap();
        Ok(vua_orchestrator::ResolveReceiptV01 {
            resolved: vec![vua_orchestrator::ResolvedPackageV01 {
                id: "com.vrchat.base".to_owned(),
                version: "3.10.1".to_owned(),
                source_repo: "official".to_owned(),
            }],
            already_satisfied: Vec::new(),
            failed: Vec::new(),
        })
    }
}

/// Creates the ProjectSettings skeleton and THEN fails — the
/// half-initialized-creation compensation vector (the assembly "delete the
/// half-initialized project" semantics, through the snapshot rollback).
struct HalfwayCreateVpm;

impl VpmBackend for HalfwayCreateVpm {
    fn name(&self) -> &'static str {
        "halfway-create"
    }

    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: true,
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
        Ok(ChangePreviewV1 {
            items: vec![],
            conflicts: vec![],
            remove_legacy_files: vec![],
            remove_legacy_folders: vec![],
            destructive: false,
            digest: format!("digest-{}", packages.len()),
        })
    }

    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        confirmed_digest: &str,
    ) -> Result<serde_json::Value, vua_orchestrator::AppErrorV1> {
        assert_eq!(confirmed_digest, "digest-1");
        Ok(serde_json::json!({ "installed": true }))
    }

    fn create_project(
        &self,
        parent: &Path,
        name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, vua_orchestrator::AppErrorV1> {
        // Half of the template copy lands, then the backend refuses.
        let root = parent.join(name);
        fs::create_dir_all(root.join("ProjectSettings")).unwrap();
        fs::write(
            root.join("ProjectSettings").join("ProjectVersion.txt"),
            "2022.3.22f1",
        )
        .unwrap();
        Err(vua_orchestrator::AppErrorV1::new(
            "vua.vpm.template_missing",
            vua_orchestrator::ErrorCategory::ExternalFailure,
            "errors.vpm.templateMissing",
            "corr",
        )
        .with_param(
            "reason",
            vua_orchestrator::ParamValue::Text("template copy failed".to_owned()),
        ))
    }
}

#[test]
fn b3_w25_empty_target_provisions_through_the_backend_port_before_import() {
    let (base, _provisioned) = make_world("w25-exec-empty");
    let source = base.join("source");
    let empty = make_empty_target(&base);

    let bridge = FakeBridge::new(vec![]);
    let vpm = CreatingVpm::new();
    let executor = MaterialExecutor::new(
        Arc::new(bridge.clone()),
        FileSystemSnapshotStore,
        vpm.clone(),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-04T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    );

    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            "project",
            "project-fingerprint",
            inspection(&source),
            &empty.root,
            "corr",
        )
        .unwrap();
    assert!(plan
        .steps
        .iter()
        .any(|step| step.kind == vua_unity_bridge::MaterialIntakeStepKind::ProvisionProject));

    let report = executor.execute(
        &confirmation(&plan),
        &source,
        &empty,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(
        report.status,
        MaterialExecutionStatus::Succeeded,
        "{:?}",
        report.error_code
    );
    // The creation went through the backend port, not around it.
    assert_eq!(vpm.creates.load(Ordering::SeqCst), 1);
    // Batch 146: the fresh-creation path resolves the declared SDK
    // dependencies EXACTLY ONCE, through the same port.
    assert_eq!(vpm.resolves.load(Ordering::SeqCst), 1);
    assert_eq!(
        report.completed_steps,
        vec![
            vua_unity_bridge::MaterialIntakeStepKind::VerifySource,
            vua_unity_bridge::MaterialIntakeStepKind::CreateSnapshot,
            vua_unity_bridge::MaterialIntakeStepKind::ProvisionProject,
            vua_unity_bridge::MaterialIntakeStepKind::ImportUnityPackages,
            vua_unity_bridge::MaterialIntakeStepKind::ValidateMinimumStructure,
            vua_unity_bridge::MaterialIntakeStepKind::WriteBuildRecord,
        ]
    );
    assert_eq!(report.rollback, RollbackOutcome::NotNeeded);

    // Three Bridge commands: the post-provision baseline inspect, the import,
    // the read-only validation. The import must expect the BASELINE the
    // inspect returned (fp-1) — never the plan-time tree digest — because
    // the freshly created project's state is not the plan-time state.
    assert_eq!(bridge.command_count(), 3);
    let commands = bridge.state.lock().unwrap().commands.clone();
    assert_eq!(
        commands[0].operation,
        vua_orchestrator::UnityOperation::InspectProject
    );
    assert!(commands[0].dry_run);
    assert_eq!(
        commands[1].operation,
        vua_orchestrator::UnityOperation::MaterializeExtractedPackage
    );
    assert_eq!(
        commands[1].expected_project_fingerprint.as_deref(),
        Some("fp-1"),
        "the first mutating command binds the post-provision baseline fingerprint"
    );
    // Batch 146 ordering proof: the resolve sentinel was already on disk when
    // the FIRST Bridge command (the baseline re-read) was recorded — the
    // fingerprint re-read therefore covers the resolved final state, never a
    // pre-resolve snapshot.
    let marker_seen = bridge.state.lock().unwrap().resolve_marker_seen.clone();
    assert_eq!(marker_seen.len(), 3);
    assert!(
        marker_seen[0],
        "the baseline re-read must run AFTER the resolve leg (sentinel observed)"
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_w25_failed_provision_restores_the_empty_state_and_honestly_reports() {
    let (base, _provisioned) = make_world("w25-exec-provision-fail");
    let source = base.join("source");
    let empty = make_empty_target(&base);

    let bridge = FakeBridge::new(vec![]);
    let executor = MaterialExecutor::new(
        Arc::new(bridge.clone()),
        FileSystemSnapshotStore,
        Arc::new(HalfwayCreateVpm),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-04T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    );

    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            "project",
            "project-fingerprint",
            inspection(&source),
            &empty.root,
            "corr",
        )
        .unwrap();

    let report = executor.execute(
        &confirmation(&plan),
        &source,
        &empty,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    // Honest failure face: the family code, the restore-eligible rollback,
    // and NO provision step in completed_steps.
    assert_eq!(report.status, MaterialExecutionStatus::Failed);
    assert!(
        report
            .error_code
            .as_deref()
            .unwrap_or("")
            .starts_with("vua.material.provision_failed"),
        "the provision code is the finding: {:?}",
        report.error_code
    );
    assert!(
        report
            .error_code
            .as_deref()
            .unwrap_or("")
            .contains("vua.vpm.template_missing"),
        "the backend's original code travels inside the message: {:?}",
        report.error_code
    );
    assert!(!report
        .completed_steps
        .contains(&vua_unity_bridge::MaterialIntakeStepKind::ProvisionProject));
    assert_eq!(report.rollback, RollbackOutcome::Restored);

    // Compensation: the half-initialized creation left the target — the
    // empty-state snapshot restore moved it into the recovery quarantine.
    assert!(
        !empty.root.join("ProjectSettings").exists(),
        "the half-initialized project must not survive the rollback"
    );
    assert!(empty.root.join(".vua/recovery").is_dir());

    // The failed run still gets a receipt (audit history, never bypassed).
    let receipt = BuildRecordStore::new(base.join("records"))
        .read(&format!("material-{}", plan.plan_id))
        .expect("the failed run still publishes its receipt");
    assert_eq!(receipt.status, vua_orchestrator::BuildRecordStatus::Failed);
    assert_eq!(
        bridge.command_count(),
        0,
        "no Unity command may run after the failed provision"
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

/// Batch 146: creates the project skeleton fine, but the resolve leg answers
/// the receipt with a non-empty `failed` set — the resolve face's honest
/// INCOMPLETE answer (a dependency no enabled repository satisfies).
struct ResolveIncompleteVpm {
    creates: AtomicUsize,
    resolves: AtomicUsize,
}

impl ResolveIncompleteVpm {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            creates: AtomicUsize::new(0),
            resolves: AtomicUsize::new(0),
        })
    }
}

impl VpmBackend for ResolveIncompleteVpm {
    fn name(&self) -> &'static str {
        "resolve-incomplete"
    }

    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: true,
            preview_install: true,
            list_packages: false,
            remove_packages: false,
            project_registry: false,
            resolve_project: true,
        }
    }

    fn preview_install(
        &self,
        _project: &ProjectRef,
        packages: &[PackageRequestV1],
    ) -> Result<ChangePreviewV1, vua_orchestrator::AppErrorV1> {
        Ok(ChangePreviewV1 {
            items: vec![],
            conflicts: vec![],
            remove_legacy_files: vec![],
            remove_legacy_folders: vec![],
            destructive: false,
            digest: format!("digest-{}", packages.len()),
        })
    }

    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        confirmed_digest: &str,
    ) -> Result<serde_json::Value, vua_orchestrator::AppErrorV1> {
        assert_eq!(confirmed_digest, "digest-1");
        Ok(serde_json::json!({ "installed": true }))
    }

    fn create_project(
        &self,
        parent: &Path,
        name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, vua_orchestrator::AppErrorV1> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        let root = parent.join(name);
        fs::create_dir_all(root.join("ProjectSettings")).unwrap();
        fs::write(
            root.join("ProjectSettings").join("ProjectVersion.txt"),
            "2022.3.22f1",
        )
        .unwrap();
        Ok(ProjectRef {
            id: name.to_owned(),
            root,
        })
    }

    fn resolve_project(
        &self,
        _project_root: &Path,
    ) -> Result<vua_orchestrator::ResolveReceiptV01, vua_orchestrator::AppErrorV1> {
        self.resolves.fetch_add(1, Ordering::SeqCst);
        Ok(vua_orchestrator::ResolveReceiptV01 {
            resolved: Vec::new(),
            already_satisfied: Vec::new(),
            failed: vec![vua_orchestrator::ResolveFailureV01 {
                id: "com.vrchat.avatars".to_owned(),
                reason_code: "vua.vpm.no_matching_package".to_owned(),
            }],
        })
    }
}

/// Batch 146: the backend grows `create_project` but NOT the resolve face —
/// the trait-default absence arm must answer the capability_missing family
/// through the executor's provision_failed fold (never a guessed success).
struct NoResolveVpm {
    creates: AtomicUsize,
}

impl NoResolveVpm {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            creates: AtomicUsize::new(0),
        })
    }
}

impl VpmBackend for NoResolveVpm {
    fn name(&self) -> &'static str {
        "no-resolve"
    }

    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: true,
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
        Ok(ChangePreviewV1 {
            items: vec![],
            conflicts: vec![],
            remove_legacy_files: vec![],
            remove_legacy_folders: vec![],
            destructive: false,
            digest: format!("digest-{}", packages.len()),
        })
    }

    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        confirmed_digest: &str,
    ) -> Result<serde_json::Value, vua_orchestrator::AppErrorV1> {
        assert_eq!(confirmed_digest, "digest-1");
        Ok(serde_json::json!({ "installed": true }))
    }

    fn create_project(
        &self,
        parent: &Path,
        name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, vua_orchestrator::AppErrorV1> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        let root = parent.join(name);
        fs::create_dir_all(root.join("ProjectSettings")).unwrap();
        fs::write(
            root.join("ProjectSettings").join("ProjectVersion.txt"),
            "2022.3.22f1",
        )
        .unwrap();
        Ok(ProjectRef {
            id: name.to_owned(),
            root,
        })
    }
    // resolve_project deliberately NOT overridden: the trait-default
    // declared-none absence arm is exactly the face under test.
}

#[test]
fn b3_batch146_provisioned_target_plans_and_runs_zero_resolve_calls() {
    let (base, project) = make_world("batch146-provisioned-no-resolve");
    let source = base.join("source");
    let bridge = FakeBridge::new(vec![]);
    let vpm = CreatingVpm::new();
    let executor = MaterialExecutor::new(
        Arc::new(bridge.clone()),
        FileSystemSnapshotStore,
        vpm.clone(),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-04T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    );

    // The ALREADY-PROVISIONED make_world target: the plan carries NO
    // provision step and the run creates AND resolves zero times — the
    // standing behavior on that path is byte-identical to pre-146 (minimal
    // blast radius).
    let plan = plan(
        MaterialEntryMode::DirectUnityPackage,
        &source,
        &project.root,
    );
    assert!(!plan
        .steps
        .iter()
        .any(|step| step.kind == vua_unity_bridge::MaterialIntakeStepKind::ProvisionProject));

    let report = executor.execute(
        &confirmation(&plan),
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(
        report.status,
        MaterialExecutionStatus::Succeeded,
        "{:?}",
        report.error_code
    );
    assert_eq!(vpm.creates.load(Ordering::SeqCst), 0);
    assert_eq!(
        vpm.resolves.load(Ordering::SeqCst),
        0,
        "the already-provisioned path must not resolve (idempotency law)"
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_batch146_incomplete_resolve_reports_the_honest_face_and_restores() {
    let (base, _provisioned) = make_world("batch146-resolve-incomplete");
    let source = base.join("source");
    let empty = make_empty_target(&base);

    let bridge = FakeBridge::new(vec![]);
    let vpm = ResolveIncompleteVpm::new();
    let executor = MaterialExecutor::new(
        Arc::new(bridge.clone()),
        FileSystemSnapshotStore,
        vpm.clone(),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-04T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    );

    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            "project",
            "project-fingerprint",
            inspection(&source),
            &empty.root,
            "corr",
        )
        .unwrap();

    let report = executor.execute(
        &confirmation(&plan),
        &source,
        &empty,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    // Honest failure face: the family code wraps the receipt's FIRST reason
    // code and the unresolved dependency id; the create succeeded (1) and
    // the resolve RAN (1) — the incompleteness is what failed the run.
    assert_eq!(report.status, MaterialExecutionStatus::Failed);
    let code = report.error_code.as_deref().unwrap_or("");
    assert!(
        code.starts_with("vua.material.provision_failed"),
        "the provision code is the family wrapper: {code}"
    );
    assert!(
        code.contains("vua.vpm.no_matching_package"),
        "the receipt's reason code travels inside the message: {code}"
    );
    assert!(
        code.contains("com.vrchat.avatars"),
        "the unresolved dependency id travels inside the message: {code}"
    );
    assert!(!report
        .completed_steps
        .contains(&vua_unity_bridge::MaterialIntakeStepKind::ProvisionProject));
    assert_eq!(report.rollback, RollbackOutcome::Restored);
    assert_eq!(vpm.creates.load(Ordering::SeqCst), 1);
    assert_eq!(vpm.resolves.load(Ordering::SeqCst), 1);

    // Compensation: the half-provisioned creation left the target — the
    // empty-state snapshot restore moved it into the recovery quarantine.
    assert!(
        !empty.root.join("ProjectSettings").exists(),
        "the half-provisioned project must not survive the rollback"
    );
    assert!(empty.root.join(".vua/recovery").is_dir());

    // The failed run still publishes its receipt, and no Unity command ran
    // after the failed resolve.
    let receipt = BuildRecordStore::new(base.join("records"))
        .read(&format!("material-{}", plan.plan_id))
        .expect("the failed run still publishes its receipt");
    assert_eq!(receipt.status, vua_orchestrator::BuildRecordStatus::Failed);
    assert_eq!(
        bridge.command_count(),
        0,
        "no Unity command may run after an incomplete resolve"
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_batch146_capability_absent_backend_fails_the_provision_honestly() {
    let (base, _provisioned) = make_world("batch146-capability-absent");
    let source = base.join("source");
    let empty = make_empty_target(&base);

    let bridge = FakeBridge::new(vec![]);
    let vpm = NoResolveVpm::new();
    let executor = MaterialExecutor::new(
        Arc::new(bridge.clone()),
        FileSystemSnapshotStore,
        vpm.clone(),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-04T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    );

    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            "project",
            "project-fingerprint",
            inspection(&source),
            &empty.root,
            "corr",
        )
        .unwrap();

    let report = executor.execute(
        &confirmation(&plan),
        &source,
        &empty,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    // The trait-default absence arm answers the capability_missing family;
    // the executor folds it into the provision_failed wrapper with the
    // original code inside the message. Creation succeeded (1) — the refusal
    // is the resolve leg's, reported as a failed provision, never papered
    // over.
    assert_eq!(report.status, MaterialExecutionStatus::Failed);
    let code = report.error_code.as_deref().unwrap_or("");
    assert!(
        code.starts_with("vua.material.provision_failed"),
        "the family wrapper holds: {code}"
    );
    assert!(
        code.contains("vua.vpm.capability_missing"),
        "the absence arm's original code travels inside the message: {code}"
    );
    assert_eq!(report.rollback, RollbackOutcome::Restored);
    assert_eq!(vpm.creates.load(Ordering::SeqCst), 1);
    assert_eq!(
        bridge.command_count(),
        0,
        "no Unity command may run after the refused resolve"
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

// --- 第 150 批：Packages/ 通道边界（操作者裁定） ---

/// Builds a guid-layout archive whose pathname entries carry the given
/// logical paths (the layout the real .unitypackage uses, which the plain
/// `unitypackage` helper's flat entries never exercise).
fn guid_layout_package(path: &Path, folders: &[(&str, &str)]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut builder = Builder::new(GzEncoder::new(file, flate2::Compression::default()));
    for (guid, logical) in folders {
        for (suffix, bytes) in [
            ("pathname", format!("{logical}\n").into_bytes()),
            ("asset", b"synthetic".to_vec()),
            ("asset.meta", b"meta".to_vec()),
        ] {
            let mut header = Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, format!("{guid}/{suffix}"), &bytes[..])
                .unwrap();
        }
    }
    builder.finish().unwrap();
}

#[test]
fn b3_batch150_intake_blocks_a_packages_prefixed_archive_as_a_finding() {
    let base = temp_dir("batch150-intake-block");
    let source = base.join("source");
    fs::create_dir_all(&source).unwrap();
    guid_layout_package(
        &source.join("pack.unitypackage"),
        &[(
            "0123456789abcdef0123456789abcdef",
            "Packages/com.evil/thing.asset",
        )],
    );

    // 发现面呈现：检查面即如实阻断（既有 archive_invalid 族，零新码），
    // 计划与确认根本不会形成——绕过 vpm-manifest 追踪的写入不可能起跑。
    let error = MaterialIntakeEngine
        .inspect_folder(&source, "corr")
        .expect_err("a Packages/-carrying package is blocked at inspection");
    assert_eq!(error.code, "vua.material.archive_invalid");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_batch150_execution_refuses_a_packages_archive_added_after_planning() {
    let (base, project) = make_world("batch150-exec-refuse");
    let source = base.join("source");

    // Plan against the clean folder, THEN a Packages/-carrying archive
    // appears (drift-by-addition). The run's VerifySource re-inspection is
    // the execution-arm defense: the honest refusal fires before the
    // snapshot, before any mutation, before any receipt.
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            "project-fingerprint",
            inspection(&source),
            &project.root,
            "corr",
        )
        .unwrap();
    guid_layout_package(
        &source.join("late.unitypackage"),
        &[(
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "Packages/com.late/x.asset",
        )],
    );

    let bridge = FakeBridge::new(vec![]);
    let executor = executor(&base, bridge.clone(), FakeVpm::new());
    let report = executor.execute(
        &confirmation(&plan),
        &source,
        &project,
        &base.join("artifacts"),
        &MaterialCancelToken::new(),
    );

    assert_eq!(report.status, MaterialExecutionStatus::Failed);
    let code = report.error_code.as_deref().unwrap_or("");
    assert!(
        code.starts_with("vua.material.archive_invalid"),
        "the channel boundary rides the standing family: {code}"
    );
    assert_eq!(
        report.rollback,
        RollbackOutcome::NotNeeded,
        "the refusal fires before the snapshot exists — nothing to restore"
    );
    assert!(report.completed_steps.is_empty());
    assert_eq!(
        bridge.command_count(),
        0,
        "no Unity command may run against a refused source"
    );
    assert!(
        BuildRecordStore::new(base.join("records"))
            .read(&format!("material-{}", plan.plan_id))
            .is_err(),
        "a pre-mutation refusal leaves no receipt"
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

// --- S2 cancellation observations (design registration
// collab/design/2026-09-23-port-cancellation-points.md): the executor's
// new token observations short-circuit honestly under an injected cancel
// request and ride the STANDING compensation arms — no new cancellation
// layer, no port-face change. ---

fn read_receipt(base: &Path, plan_id: &str) -> vua_orchestrator::BuildRecordV01 {
    BuildRecordStore::new(base.join("records"))
        .read(&format!("material-{plan_id}"))
        .expect("the receipt publishes for cancellation like any other exit")
}

#[test]
fn b3_s2_provision_cancel_between_create_and_resolve_skips_the_network_leg() {
    let (base, _provisioned) = make_world("s2-provision-cancel");
    let source = base.join("source");
    let empty = make_empty_target(&base);

    // The cancel request lands inside create_project — after the observation
    // point's preceding step, before the resolve leg it guards.
    let token = MaterialCancelToken::new();
    let vpm = CreatingVpm::new_cancelling_on_create(token.clone());
    let bridge = FakeBridge::new(vec![]);
    let executor = MaterialExecutor::new(
        Arc::new(bridge),
        FileSystemSnapshotStore,
        vpm.clone(),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-04T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    );

    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            "project",
            "project-fingerprint",
            inspection(&source),
            &empty.root,
            "corr",
        )
        .unwrap();
    let report = executor.execute(
        &confirmation(&plan),
        &source,
        &empty,
        &base.join("artifacts"),
        &token,
    );

    assert_eq!(report.status, MaterialExecutionStatus::Cancelled);
    // The observation sits between create and resolve: create happened, the
    // unbounded network leg never started.
    assert_eq!(vpm.creates.load(Ordering::SeqCst), 1);
    assert_eq!(
        vpm.resolves.load(Ordering::SeqCst),
        0,
        "a decided cancellation must not launch the resolve leg"
    );
    // The standing compensation ran: the half-provisioned content is rolled
    // back through the verified snapshot (empty-state quarantine semantics).
    assert_eq!(report.rollback, RollbackOutcome::Restored);
    let record = read_receipt(&base, &plan.plan_id);
    assert_eq!(
        record.status,
        vua_orchestrator::BuildRecordStatus::Cancelled
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_s2_tail_cancel_before_register_skips_registration_preview_and_apply() {
    let (base, project) = make_world("s2-tail-before-register");
    let source = base.join("source");
    fs::write(
        source.join("vua-dependencies.json"),
        r#"{ "com.vrchat.avatars": "3.10.x" }"#,
    )
    .unwrap();

    // The cancel request lands while the CreateLocalVpmPackage Bridge
    // command is in flight — the first tail observation (before register)
    // is the next point that can see it.
    let token = MaterialCancelToken::new();
    let bridge = FakeBridge::new(vec![]).with_cancel_on(
        vua_orchestrator::UnityOperation::CreateLocalVpmPackage,
        token.clone(),
    );
    let vpm = FakeVpm::new();
    let executor = executor(&base, bridge.clone(), vpm.clone());

    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::LocalReusableVpm,
            "project",
            "project-fingerprint",
            inspection(&source),
            &project.root,
            "corr",
        )
        .unwrap();
    let report = executor.execute(
        &confirmation(&plan),
        &source,
        &project,
        &base.join("artifacts"),
        &token,
    );

    assert_eq!(report.status, MaterialExecutionStatus::Cancelled);
    // Cancelled during staging: the target is never touched — registration,
    // preview and install are all skipped.
    assert_eq!(
        vpm.registrations.load(Ordering::SeqCst),
        0,
        "register must not run after a decided cancellation"
    );
    assert_eq!(vpm.installs.load(Ordering::SeqCst), 0);
    // The standing compensation still ran against the target.
    assert_eq!(report.rollback, RollbackOutcome::Restored);
    let record = read_receipt(&base, &plan.plan_id);
    assert_eq!(
        record.status,
        vua_orchestrator::BuildRecordStatus::Cancelled
    );
    assert!(
        record.local_vpm.is_none(),
        "no install evidence when the tail never ran"
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}

#[test]
fn b3_s2_tail_cancel_between_preview_and_apply_skips_the_install() {
    let (base, project) = make_world("s2-tail-before-apply");
    let source = base.join("source");
    fs::write(
        source.join("vua-dependencies.json"),
        r#"{ "com.vrchat.avatars": "3.10.x" }"#,
    )
    .unwrap();

    // The cancel request lands inside preview_install — the second tail
    // observation (between preview and apply) must skip the install itself.
    let token = MaterialCancelToken::new();
    let vpm = FakeVpm::new_cancelling_on_preview(token.clone());
    let bridge = FakeBridge::new(vec![]);
    let executor = executor(&base, bridge.clone(), vpm.clone());

    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::LocalReusableVpm,
            "project",
            "project-fingerprint",
            inspection(&source),
            &project.root,
            "corr",
        )
        .unwrap();
    let report = executor.execute(
        &confirmation(&plan),
        &source,
        &project,
        &base.join("artifacts"),
        &token,
    );

    assert_eq!(report.status, MaterialExecutionStatus::Cancelled);
    // Registration and preview happened; the install itself was skipped —
    // the target is spared an install it would only have to be rolled back.
    assert_eq!(vpm.registrations.load(Ordering::SeqCst), 1);
    assert_eq!(
        vpm.installs.load(Ordering::SeqCst),
        0,
        "apply must not run after a decided cancellation"
    );
    // The honest fact both tail options share: the published artifact lives
    // OUTSIDE the project (the output root), beyond the snapshot's reach —
    // it stays, and the receipt says Cancelled rather than pretending the
    // tail never ran.
    assert!(
        fs::read_dir(base.join("artifacts"))
            .unwrap()
            .next()
            .is_some(),
        "the published artifact stays"
    );
    assert_eq!(report.rollback, RollbackOutcome::Restored);
    let record = read_receipt(&base, &plan.plan_id);
    assert_eq!(
        record.status,
        vua_orchestrator::BuildRecordStatus::Cancelled
    );
    assert!(
        record.local_vpm.is_none(),
        "no install evidence when the install never ran"
    );
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
}
