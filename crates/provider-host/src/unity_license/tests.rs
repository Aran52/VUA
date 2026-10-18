use super::*;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Mutex,
};
use vua_orchestrator::{
    FixedClock, NanosTaskIdGenerator, ProcessError, ProcessOutcome, SqliteTaskStore, TaskRuntime,
    TaskState,
};

struct TempRoot(PathBuf);
impl TempRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "vua-unity-license-{}",
            crate::provider_host::uuid_v7_identity()
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}
impl Drop for TempRoot {
    fn drop(&mut self) {
        assert!(
            self.0.starts_with(std::env::temp_dir())
                && self
                    .0
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("vua-unity-license-")
        );
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn wait(mut predicate: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !predicate() {
        assert!(
            std::time::Instant::now() < deadline,
            "synthetic check did not progress"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn hub(kind: HubType) -> HubFinding {
    Arc::new(move || vua_orchestrator::EnvironmentCheckItemV1 {
        schema_version: 1,
        id: "unity_hub".into(),
        zone: vua_orchestrator::Zone::Create,
        presence: if kind == HubType::None {
            EnvironmentPresence::NotDetected
        } else {
            EnvironmentPresence::Detected
        },
        error_code: None,
        facts: json!({"via":if kind == HubType::Msix {"package"} else {"path"},"exe":"C:/synthetic/Unity Hub.exe"}),
    })
}

#[test]
fn injected_metadata_and_process_sources_never_expose_license_contents_or_paths() {
    let root = TempRoot::new();
    let local = root.0.join("Local");
    let program = root.0.join("ProgramData");
    let relative = ["Unity/licenses/UnityEntitlementLicense.xml", "Packages/UnityTechnologies.UnityHub_2vrhnee42bhxm/LocalCache/Local/Unity/licenses/UnityEntitlementLicense.xml"];
    for path in [
        local.join(relative[0]),
        local.join(relative[1]),
        program.join("Unity/Unity_lic.ulf"),
        local.join("Unity/bin/unity.exe"),
    ] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"PRIVATE_LICENSE_CONTENT synthetic account").unwrap();
    }
    let probe = NativeLicenseProbe {
        paths: LicensePaths {
            local_app_data: Some(local.clone()),
            program_data: Some(program),
        },
        hub: hub(HubType::Msix),
        processes: Arc::new(|| {
            Some(vec![
                "Unity Hub.exe".into(),
                "Unity.Licensing.Client.exe".into(),
            ])
        }),
        editors: Arc::new(|| panic!("a metadata query never discovers or runs an Editor")),
    };
    let view = snapshot(probe.observe(), None);
    assert!(view
        .files
        .iter()
        .all(|file| file.present == Some(true) && file.modified_at.is_some()));
    assert_eq!(view.hub_type, HubType::Msix);
    assert_eq!(
        (
            view.hub_running,
            view.licensing_client_running,
            view.hub_cli_present
        ),
        (Some(true), Some(true), Some(true))
    );
    let encoded = serde_json::to_string(&view).unwrap();
    for private in [
        "PRIVATE_LICENSE_CONTENT",
        "synthetic account",
        root.0.to_str().unwrap(),
        "UnityEntitlementLicense.xml",
        "Unity_lic.ulf",
    ] {
        assert!(!encoded.contains(private));
    }
    // Keep only the packaged license, with a stopped Hub/client.
    fs::remove_file(local.join(relative[0])).unwrap();
    fs::remove_file(
        probe
            .paths
            .program_data
            .as_ref()
            .unwrap()
            .join("Unity/Unity_lic.ulf"),
    )
    .unwrap();
    let stopped = NativeLicenseProbe {
        processes: Arc::new(|| Some(Vec::new())),
        ..probe
    };
    assert_eq!(
        snapshot(stopped.observe(), None).next_action,
        LicenseNextAction::StartHub
    );
    let unknown = NativeLicenseProbe {
        processes: Arc::new(|| None),
        ..stopped
    };
    assert_eq!(
        snapshot(unknown.observe(), None).state,
        LicenseHint::Unknown
    );
}

#[test]
fn missing_empty_and_unreadable_metadata_are_distinct() {
    let root = TempRoot::new();
    let mut probe = NativeLicenseProbe {
        paths: LicensePaths {
            local_app_data: Some(root.0.join("Local")),
            program_data: Some(root.0.join("Data")),
        },
        hub: hub(HubType::None),
        processes: Arc::new(|| Some(Vec::new())),
        editors: Arc::new(|| Ok(Vec::new())),
    };
    assert_eq!(snapshot(probe.observe(), None).state, LicenseHint::NotFound);
    let path = probe
        .paths
        .local_app_data
        .as_ref()
        .unwrap()
        .join("Unity/licenses/UnityEntitlementLicense.xml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"").unwrap();
    assert_eq!(snapshot(probe.observe(), None).state, LicenseHint::Unknown);
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert_eq!(probe.observe().files[0].present, None);
    probe.paths.local_app_data = None;
    assert_eq!(snapshot(probe.observe(), None).state, LicenseHint::Unknown);
    probe.hub = hub(HubType::Classic);
    assert_eq!(probe.observe().hub_type, HubType::Classic);
}

struct Probe {
    client: AtomicBool,
    valid_editor: AtomicBool,
    ensures: AtomicUsize,
    wait_for_cancel: AtomicBool,
}
impl Default for Probe {
    fn default() -> Self {
        Self {
            client: AtomicBool::new(true),
            valid_editor: AtomicBool::new(true),
            ensures: AtomicUsize::new(0),
            wait_for_cancel: AtomicBool::new(false),
        }
    }
}
impl LicenseProbe for Probe {
    fn observe(&self) -> LicenseObservation {
        LicenseObservation {
            files: [
                LicenseFile {
                    kind: LicenseKind::ClassicEntitlement,
                    present: Some(true),
                    modified_at: Some("2026-10-19T00:00:00.000Z".into()),
                },
                LicenseFile {
                    kind: LicenseKind::MsixEntitlement,
                    present: Some(false),
                    modified_at: None,
                },
                LicenseFile {
                    kind: LicenseKind::LegacyUlf,
                    present: Some(false),
                    modified_at: None,
                },
            ],
            nonempty: [true, false, false],
            complete: true,
            hub_type: HubType::Classic,
            hub_running: Some(true),
            licensing_client_running: Some(self.client.load(Ordering::SeqCst)),
            hub_cli_present: Some(false),
        }
    }
    fn verify_editor(&self, path: &str) -> Result<ProductionEditor, AppErrorV1> {
        if !self.valid_editor.load(Ordering::SeqCst)
            || !["C:/synthetic/Unity.exe", "C:/synthetic/alternate/Unity.exe"].contains(&path)
        {
            return Err(error("editor_unavailable"));
        }
        Ok(ProductionEditor {
            path: path.into(),
            version: "2022.3.22f1".into(),
        })
    }
    fn ensure_client(&self, cancelled: &(dyn Fn() -> bool + Send + Sync)) -> bool {
        self.ensures.fetch_add(1, Ordering::SeqCst);
        if self.wait_for_cancel.load(Ordering::SeqCst) {
            wait(cancelled);
        }
        self.client.load(Ordering::SeqCst)
    }
}
struct Runner {
    calls: Mutex<Vec<ProcessSpec>>,
    outcome: ProcessOutcome,
    log: Option<String>,
    fail: bool,
    hold: AtomicBool,
}
impl Default for Runner {
    fn default() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            outcome: ProcessOutcome {
                exit_code: Some(0),
                timed_out: false,
                cancelled: false,
                process_tree_clean: true,
                stdout: "private account C:/secret".into(),
                stderr: "private token".into(),
                truncated: false,
            },
            log: Some("Editor startup finished\n".into()),
            fail: false,
            hold: AtomicBool::new(false),
        }
    }
}
impl ProcessRunner for Runner {
    fn run(&self, _: &ProcessSpec) -> Result<ProcessOutcome, ProcessError> {
        panic!("check must use cancellable execution")
    }
    fn run_cancellable(
        &self,
        spec: &ProcessSpec,
        cancelled: &(dyn Fn() -> bool + Send + Sync),
    ) -> Result<ProcessOutcome, ProcessError> {
        self.calls.lock().unwrap().push(spec.clone());
        if self.fail {
            return Err(ProcessError::Io(std::io::Error::other(
                "private account C:/secret",
            )));
        }
        if let Some(log) = &self.log {
            let position = spec.args.iter().position(|a| a == "-logFile").unwrap();
            fs::write(&spec.args[position + 1], log).unwrap();
        }
        wait(|| !self.hold.load(Ordering::SeqCst) || cancelled());
        let mut outcome = self.outcome.clone();
        outcome.cancelled |= cancelled();
        Ok(outcome)
    }
}
fn service(
    root: &Path,
    probe: Arc<Probe>,
    runner: Arc<Runner>,
) -> (Arc<UnityLicenseService>, Arc<SqliteTaskStore>) {
    let store = Arc::new(SqliteTaskStore::open(root.join("tasks.db")).unwrap());
    let runtime = TaskRuntime::with_sqlite(
        store.clone(),
        Arc::new(FixedClock::new(&["2026-10-19T01:00:00.000Z"])),
        Arc::new(NanosTaskIdGenerator::default()),
    )
    .unwrap();
    let platform = crate::amf_production_adapter::NativeAmfProduction {
        bdl: Arc::new(vua_bdl_store::BdlStore::open(root.join("bdl.db")).unwrap()),
        root: root.join("amf"),
        editor_roots: Vec::new(),
        explicit_editor: None,
        unity_hub: None,
    };
    let amf = Arc::new(crate::amf_production_routes::AmfProductionServices::new(
        vua_orchestrator::amf_production::AmfProductionEngine::new(
            root.join("amf"),
            Arc::new(platform),
        ),
        runtime,
    ));
    (
        Arc::new(UnityLicenseService {
            probe,
            runner,
            root: root.join("unity-license"),
            amf: Some(amf),
            clock: Arc::new(FixedClock::new(&["2026-10-19T01:00:00.000Z"])),
        }),
        store,
    )
}
fn run(
    service: &Arc<UnityLicenseService>,
    store: &SqliteTaskStore,
) -> vua_orchestrator::StoredTask {
    let id = service
        .submit(
            "C:/synthetic/Unity.exe",
            "synthetic",
            &crate::provider_host::uuid_v7_identity(),
        )
        .unwrap()["taskId"]
        .as_str()
        .unwrap()
        .to_owned();
    wait(|| {
        store
            .task(&id)
            .unwrap()
            .is_some_and(|t| t.state.is_terminal())
    });
    wait(|| service.amf.as_ref().unwrap().lane.is_idle());
    store.task(&id).unwrap().unwrap()
}

#[test]
fn successful_check_uses_only_owned_project_bounded_runner_and_three_field_record() {
    let root = TempRoot::new();
    let probe = Arc::new(Probe::default());
    let runner = Arc::new(Runner::default());
    let (service, store) = service(&root.0, probe, runner.clone());
    assert!(service.snapshot().last_check.is_none());
    let task = run(&service, &store);
    assert_eq!(task.state, TaskState::Succeeded);
    let result = task.result.unwrap();
    assert_eq!(result["state"], "active");
    assert_eq!(result["reason"], Value::Null);
    let calls = runner.calls.lock().unwrap();
    let spec = &calls[0];
    assert_eq!(spec.timeout, Duration::from_secs(300));
    assert_eq!(spec.output_limit, 0);
    assert_eq!(&spec.args[..3], ["-batchmode", "-nographics", "-quit"]);
    assert_eq!(
        spec.working_dir.as_ref().unwrap(),
        &service.root.join("project")
    );
    assert!(spec.removals.iter().any(|s| s == "GH_TOKEN"));
    assert!(!serde_json::to_string(&result).unwrap().contains("private"));
    let saved: Value =
        serde_json::from_slice(&fs::read(service.root.join("last-check.json")).unwrap()).unwrap();
    assert_eq!(saved.as_object().unwrap().len(), 3);
    assert_eq!(saved["state"], "active");
    assert_eq!(service.snapshot().next_action, LicenseNextAction::None);
    drop(calls);
    assert_eq!(run(&service, &store).state, TaskState::Succeeded);
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls[0].working_dir, calls[1].working_dir);
    assert!(service.last_check().is_some());
}

#[test]
fn synthetic_license_errors_timeout_failure_and_missing_log_have_sanitized_outcomes() {
    for (text, client, exit, timeout, fail, expected) in [
        (
            Some("No ULF license found"),
            false,
            1,
            false,
            false,
            "needs_hub_client",
        ),
        (
            Some("License is not active"),
            true,
            1,
            false,
            false,
            "not_activated",
        ),
        (
            Some("No valid Unity Editor license found"),
            true,
            0,
            false,
            false,
            "not_activated",
        ),
        (Some("ordinary failure"), true, 1, false, false, "unknown"),
        (Some("ordinary startup"), true, 0, true, false, "unknown"),
        (None, true, 0, false, false, "unknown"),
        (None, true, 0, false, true, "unknown"),
    ] {
        let root = TempRoot::new();
        let probe = Arc::new(Probe::default());
        probe.client.store(client, Ordering::SeqCst);
        let mut runner = Runner {
            log: text.map(|s| format!("private account/path: {s}\n")),
            fail,
            ..Runner::default()
        };
        runner.outcome.exit_code = Some(exit);
        runner.outcome.timed_out = timeout;
        let (service, store) = service(&root.0, probe, Arc::new(runner));
        let task = run(&service, &store);
        assert_eq!(task.state, TaskState::SucceededWithWarnings);
        let result = task.result.unwrap();
        assert_eq!(result["state"], expected);
        assert!(!serde_json::to_string(&result).unwrap().contains("private"));
        assert_eq!(
            serde_json::to_value(service.last_check().unwrap()).unwrap()["state"],
            expected
        );
    }
}

#[test]
fn cancellation_ends_the_check_and_persists_unknown_without_log_or_account_data() {
    let root = TempRoot::new();
    let runner = Arc::new(Runner::default());
    runner.hold.store(true, Ordering::SeqCst);
    let (service, store) = service(&root.0, Arc::new(Probe::default()), runner.clone());
    let id = service
        .submit("C:/synthetic/Unity.exe", "cancel", "cancel")
        .unwrap()["taskId"]
        .as_str()
        .unwrap()
        .to_owned();
    wait(|| runner.calls.lock().unwrap().len() == 1);
    service.amf.as_ref().unwrap().runtime.cancel(&id).unwrap();
    wait(|| store.task(&id).unwrap().unwrap().state.is_terminal());
    wait(|| service.amf.as_ref().unwrap().lane.is_idle());
    assert_eq!(
        store.task(&id).unwrap().unwrap().state,
        TaskState::Cancelled
    );
    assert_eq!(
        service.last_check().unwrap().state,
        LicenseCheckState::Unknown
    );
}

#[test]
fn cancellation_during_hub_wait_does_not_execute_editor() {
    let root = TempRoot::new();
    let probe = Arc::new(Probe::default());
    probe.client.store(false, Ordering::SeqCst);
    probe.wait_for_cancel.store(true, Ordering::SeqCst);
    let runner = Arc::new(Runner::default());
    let (service, store) = service(&root.0, probe.clone(), runner.clone());
    let id = service
        .submit("C:/synthetic/Unity.exe", "cancel-hub", "cancel-hub")
        .unwrap()["taskId"]
        .as_str()
        .unwrap()
        .to_owned();
    wait(|| probe.ensures.load(Ordering::SeqCst) == 1);
    service.amf.as_ref().unwrap().runtime.cancel(&id).unwrap();
    wait(|| store.task(&id).unwrap().unwrap().state.is_terminal());
    wait(|| service.amf.as_ref().unwrap().lane.is_idle());
    assert!(runner.calls.lock().unwrap().is_empty());
    assert_eq!(
        service.last_check().unwrap().state,
        LicenseCheckState::Unknown
    );
}

#[test]
fn check_uses_the_same_amf_lane_exclusively_and_cannot_be_overtaken() {
    let root = TempRoot::new();
    let runner = Arc::new(Runner::default());
    runner.hold.store(true, Ordering::SeqCst);
    let (service, store) = service(&root.0, Arc::new(Probe::default()), runner.clone());
    let amf = service.amf.as_ref().unwrap();
    amf.lane.set_capacity(2).unwrap();
    let (release, gate) = std::sync::mpsc::channel();
    let first = amf
        .submit(
            SubmitRequest {
                correlation_id: None,
                timeout: None,
                job: Box::new(move |_| {
                    gate.recv_timeout(Duration::from_secs(10)).unwrap();
                    Ok(TaskExit::Done(Value::Null))
                }),
            },
            "n3-first",
            "amf.production.reinspect",
        )
        .unwrap()
        .task_id;
    wait(|| amf.runtime.snapshot(&first).unwrap().state == TaskState::Running);
    let id = service
        .submit("C:/synthetic/Unity.exe", "queued", "queued")
        .unwrap()["taskId"]
        .as_str()
        .unwrap()
        .to_owned();
    let later = amf
        .submit(
            SubmitRequest {
                correlation_id: None,
                timeout: None,
                job: Box::new(|_| Ok(TaskExit::Done(Value::Null))),
            },
            "n3-later",
            "amf.production.reinspect",
        )
        .unwrap()
        .task_id;
    std::thread::sleep(Duration::from_millis(100));
    assert!(runner.calls.lock().unwrap().is_empty());
    assert_eq!(amf.lane.waiting(), vec![id.clone(), later.clone()]);
    release.send(()).unwrap();
    wait(|| runner.calls.lock().unwrap().len() == 1);
    assert_eq!(
        amf.runtime.snapshot(&later).unwrap().state,
        TaskState::Queued
    );
    assert_eq!(amf.lane.snapshot().running, vec![id.clone()]);
    runner.hold.store(false, Ordering::SeqCst);
    wait(|| store.task(&later).unwrap().unwrap().state.is_terminal());
    wait(|| amf.lane.is_idle());
    assert_eq!(
        store.task(&id).unwrap().unwrap().state,
        TaskState::Succeeded
    );
}

#[test]
fn queued_cancellation_and_editor_revalidation_never_run_unverified_programs() {
    for cancelled in [true, false] {
        let root = TempRoot::new();
        let probe = Arc::new(Probe::default());
        let runner = Arc::new(Runner::default());
        let (service, store) = service(&root.0, probe.clone(), runner.clone());
        let amf = service.amf.as_ref().unwrap();
        assert!(service
            .submit("C:/not-discovered/tool.exe", "invalid", "invalid")
            .is_err());
        let (release, gate) = std::sync::mpsc::channel();
        let first = amf
            .submit(
                SubmitRequest {
                    correlation_id: None,
                    timeout: None,
                    job: Box::new(move |_| {
                        gate.recv_timeout(Duration::from_secs(10)).unwrap();
                        Ok(TaskExit::Done(Value::Null))
                    }),
                },
                "n3-first",
                "amf.production.reinspect",
            )
            .unwrap()
            .task_id;
        wait(|| amf.runtime.snapshot(&first).unwrap().state == TaskState::Running);
        let id = service
            .submit("C:/synthetic/Unity.exe", "waiting", "waiting")
            .unwrap()["taskId"]
            .as_str()
            .unwrap()
            .to_owned();
        if cancelled {
            amf.runtime.cancel(&id).unwrap();
        } else {
            probe.valid_editor.store(false, Ordering::SeqCst);
        }
        release.send(()).unwrap();
        wait(|| store.task(&id).unwrap().unwrap().state.is_terminal());
        wait(|| amf.lane.is_idle());
        assert_eq!(
            store.task(&id).unwrap().unwrap().state,
            if cancelled {
                TaskState::Cancelled
            } else {
                TaskState::Failed
            }
        );
        assert!(runner.calls.lock().unwrap().is_empty());
        assert!(service.last_check().is_none());
    }
}

#[test]
fn unowned_projects_and_modified_manifests_are_preserved_and_refused() {
    let root = TempRoot::new();
    let runner = Arc::new(Runner::default());
    let (service, _) = service(&root.0, Arc::new(Probe::default()), runner.clone());
    fs::create_dir_all(service.root.join("project")).unwrap();
    fs::write(service.root.join("project/player-file"), b"keep").unwrap();
    assert!(service.project("2022.3.22f1").is_err());
    assert_eq!(
        fs::read(service.root.join("project/player-file")).unwrap(),
        b"keep"
    );
    assert!(runner.calls.lock().unwrap().is_empty());
    let second = TempRoot::new();
    let (service, _) = self::service(&second.0, Arc::new(Probe::default()), runner);
    let project = service.project("2022.3.22f1").unwrap();
    fs::write(project.join("Packages/manifest.json"), b"user changes").unwrap();
    assert!(service.project("2022.3.22f1").is_err());
    assert_eq!(
        fs::read(project.join("Packages/manifest.json")).unwrap(),
        b"user changes"
    );
}

#[test]
fn route_validates_before_absence_and_query_reads_the_persisted_sanitized_record() {
    let root = TempRoot::new();
    let runner = Arc::new(Runner::default());
    let (service, store) = service(&root.0, Arc::new(Probe::default()), runner);
    run(&service, &store);
    let request = json!({"kind":"query","params":{"schemaVersion":"0.1"}});
    let crate::provider_host::FrameOutcome::Response(response) =
        crate::unity_license_routes::request(
            Some(&service),
            "environment.unityLicense",
            &request,
            "read",
            "read",
        )
    else {
        panic!("query response")
    };
    assert_eq!(response["ok"], true);
    assert_eq!(response["value"]["lastCheck"]["state"], "active");
    let schema: Value = serde_json::from_slice(
        &fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../schemas/unity-license/v0.1/result.schema.json"
        ))
        .unwrap(),
    )
    .unwrap();
    assert!(jsonschema::validator_for(&schema)
        .unwrap()
        .is_valid(&response["value"]));
    for bad in [
        json!({"kind":"command","params":{"schemaVersion":"0.1"}}),
        json!({"kind":"query","params":{"schemaVersion":"0.1","licensePath":"C:/private"}}),
    ] {
        let crate::provider_host::FrameOutcome::Response(response) =
            crate::unity_license_routes::request(
                None,
                "environment.unityLicense",
                &bad,
                "bad",
                "bad",
            )
        else {
            panic!("validation response")
        };
        assert_eq!(
            response["error"]["code"],
            "vua.unity_license.invalid_params"
        );
    }
    fs::write(service.root.join("last-check.json"), b"{bad").unwrap();
    assert!(service.snapshot().last_check.is_none());
}

#[test]
fn command_retries_replay_after_restart_and_changed_arguments_conflict() {
    let root = TempRoot::new();
    let runner = Arc::new(Runner::default());
    runner.hold.store(true, Ordering::SeqCst);
    let (service, store) = service(&root.0, Arc::new(Probe::default()), runner.clone());
    let first = service
        .submit("C:/synthetic/Unity.exe", "first", "once")
        .unwrap();
    wait(|| runner.calls.lock().unwrap().len() == 1);
    assert_eq!(
        service
            .submit("C:/synthetic/Unity.exe", "retry", "once")
            .unwrap(),
        first
    );
    assert_eq!(
        service
            .submit("C:/synthetic/alternate/Unity.exe", "conflict", "once")
            .unwrap_err()
            .code,
        "vua.task.idempotency_conflict"
    );
    runner.hold.store(false, Ordering::SeqCst);
    wait(|| service.amf.as_ref().unwrap().lane.is_idle());
    assert_eq!(runner.calls.lock().unwrap().len(), 1);
    drop(service);
    drop(store);
    let second_runner = Arc::new(Runner::default());
    let (restarted, _) = self::service(&root.0, Arc::new(Probe::default()), second_runner.clone());
    assert_eq!(
        restarted
            .submit("C:/synthetic/Unity.exe", "restart", "once")
            .unwrap(),
        first
    );
    assert!(second_runner.calls.lock().unwrap().is_empty());
}
