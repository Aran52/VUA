//! Synthetic release tests: turntable records and managed uploads. No Unity, SDK or VRChat call.
#![allow(clippy::result_large_err)]
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use vua_orchestrator::{
    amf_production::*, AppErrorV1, NanosTaskIdGenerator, SqliteTaskStore, SubmitRequest,
    SystemClock, TaskContext, TaskExit, TaskRuntime,
};

const RUN: &str = "n3-release";
const AVATAR_ID: &str = "avtr_0d1e2f3a-4b5c-4d6e-8f90-a1b2c3d4e5f6";

struct ReleasePlatform {
    editor_open: AtomicBool,
    turntable_scene: Mutex<String>,
    turntable_folder: Mutex<String>,
    receipt: Mutex<Option<UploadReceipt>>,
    requests: Mutex<Vec<UploadRequest>>,
    local_receipt: Mutex<Option<LocalTestReceipt>>,
    local_requests: Mutex<Vec<LocalTestRequest>>,
}
impl ReleasePlatform {
    fn new() -> Self {
        Self {
            editor_open: AtomicBool::new(false),
            turntable_scene: Mutex::new("scene-1".into()),
            turntable_folder: Mutex::new(".vua/turntables/cmd-2".into()),
            receipt: Mutex::new(None),
            requests: Mutex::new(Vec::new()),
            local_receipt: Mutex::new(None),
            local_requests: Mutex::new(Vec::new()),
        }
    }
}
fn unused<T>() -> Result<T, AppErrorV1> {
    Err(production_error("test_unused", "Not part of release tests."))
}
impl AmfProductionPlatform for ReleasePlatform {
    fn sources(&self) -> Result<Vec<ProductionSource>, AppErrorV1> { Ok(Vec::new()) }
    fn editors(&self) -> Result<Vec<ProductionEditor>, AppErrorV1> { Ok(vec![self.verify_editor("synthetic-editor")?]) }
    fn verify_editor(&self, path: &str) -> Result<ProductionEditor, AppErrorV1> {
        Ok(ProductionEditor { path: path.into(), version: "2022.3.22f1c1".into() })
    }
    fn create_project(&self, _: &ProductionRun, _: &TaskContext) -> Result<(), AppErrorV1> { unused() }
    fn import_sources(&self, _: &ProductionRun, _: &TaskContext) -> Result<Vec<PreparedSource>, AppErrorV1> { unused() }
    fn inspect(&self, _: &ProductionRun, _: &UnityWatch) -> Result<ObservedProject, AppErrorV1> { unused() }
    fn assemble(&self, _: &ProductionRun, _: &AssemblySelection, _: &[OutfitFit], _: &UnityWatch) -> Result<ObservedProject, AppErrorV1> { unused() }
    fn fingerprint(&self, project: &Path) -> Result<String, AppErrorV1> {
        fs::read_to_string(project.join("Assets/state")).map_err(|e| production_error("test_io", e.to_string()))
    }
    fn fingerprint_legacy(&self, project: &Path) -> Result<String, AppErrorV1> {
        self.fingerprint(project)
    }
    fn snapshot(&self, _: &ProductionRun) -> Result<String, AppErrorV1> { unused() }
    fn restore(&self, _: &ProductionRun, _: &str) -> Result<(), AppErrorV1> { unused() }
    fn open(&self, _: &ProductionRun, _: &TaskContext) -> Result<serde_json::Value, AppErrorV1> { unused() }
    fn turntable(&self, run: &ProductionRun, _: &UnityWatch) -> Result<ObservedProject, AppErrorV1> {
        let folder = self.turntable_folder.lock().unwrap().clone();
        let target = Path::new(&run.project_root).join(&folder);
        if fs::create_dir_all(&target).is_ok() {
            for index in 0..TURNTABLE_FRAMES {
                fs::write(target.join(format!("frame-{index:02}.png")), b"\x89PNG\r\n\x1a\nframe").unwrap();
            }
        }
        Ok(ObservedProject {
            prefabs: Vec::new(),
            unity_fingerprint: self.turntable_scene.lock().unwrap().clone(),
            editor_version: "2022.3.22f1c1".into(),
            result: json!({"facts":{"turntable":{"folder":folder,"frames":60,"size":512},"previews":[]}}),
        })
    }
    fn upload(&self, _: &ProductionRun, request: &UploadRequest, _: &TaskContext, progress: &mut dyn FnMut(UploadProgress)) -> Result<UploadReceipt, AppErrorV1> {
        self.requests.lock().unwrap().push(request.clone());
        for (stage, share) in [("sdk_panel", None), ("building", None), ("uploading", Some(0.5)), ("uploading", Some(0.5)), ("unknown_stage", None)] {
            progress(UploadProgress { stage: stage.into(), progress: share, status: "Synthetic\u{0007} status".into() });
        }
        self.receipt.lock().unwrap().clone().ok_or_else(|| production_error("upload_timeout", "Unity did not answer."))
    }
    fn editor_open(&self, _: &Path) -> bool { self.editor_open.load(Ordering::SeqCst) }
    fn local_test(&self, _: &ProductionRun, request: &LocalTestRequest, _: &TaskContext, progress: &mut dyn FnMut(UploadProgress)) -> Result<LocalTestReceipt, AppErrorV1> {
        self.local_requests.lock().unwrap().push(request.clone());
        for stage in ["sdk_panel", "sign_in", "building", "building", "uploading"] {
            progress(UploadProgress { stage: stage.into(), progress: None, status: "Synthetic\u{0007} test".into() });
        }
        self.local_receipt.lock().unwrap().clone().ok_or_else(|| production_error("local_test_timeout", "Unity did not answer."))
    }
}

struct Harness {
    root: PathBuf,
    engine: Arc<AmfProductionEngine>,
    platform: Arc<ReleasePlatform>,
    runtime: TaskRuntime,
}
impl Harness {
    fn new() -> Self {
        static NEXT_ROOT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!("vua-release-test-{}-{}-{}", std::process::id(), NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&root).unwrap();
        let platform = Arc::new(ReleasePlatform::new());
        let engine = Arc::new(AmfProductionEngine::new(root.join("production"), platform.clone()));
        let runtime = TaskRuntime::with_sqlite(Arc::new(SqliteTaskStore::open(root.join("tasks.db")).unwrap()),
            Arc::new(SystemClock), Arc::new(NanosTaskIdGenerator::default())).unwrap();
        let harness = Self { root, engine, platform, runtime };
        harness.finished_run("succeeded");
        harness
    }
    fn project(&self) -> PathBuf { self.root.join("production/projects").join(RUN) }
    /// A finished run as assembly leaves it: scene "scene-1", project state "assembled".
    fn finished_run(&self, phase: &str) {
        let project = self.project();
        fs::create_dir_all(project.join("Assets")).unwrap();
        fs::create_dir_all(project.join(".vua/previews/cmd-0")).unwrap();
        fs::write(project.join("Assets/state"), "assembled").unwrap();
        fs::write(project.join(".vua/previews/cmd-0/front.png"), b"\x89PNG\r\n\x1a\nfront").unwrap();
        let run = json!({"schemaVersion":"0.1", "runId":RUN, "phase":phase, "taskId":"task-0",
            "editor":{"path":"synthetic-editor","version":"2022.3.22f1c1"}, "projectRoot":project.to_string_lossy(),
            "choices":[], "sources":[], "prefabs":[], "projectFingerprint":"assembled", "unityFingerprint":"scene-1",
            "steps":[], "selection":null, "confirmedDigest":null, "error":null, "recovery":null,
            "result":{"facts":{"avatarGlobalObjectId":"GlobalObjectId_V1-avatar", "previews":[{"view":"front","path":".vua/previews/cmd-0/front.png"}]}}});
        fs::create_dir_all(self.root.join("production/runs")).unwrap();
        fs::write(self.root.join("production/runs").join(format!("{RUN}.json")), serde_json::to_vec(&run).unwrap()).unwrap();
    }
    fn job(&self, action: impl FnOnce(&AmfProductionEngine, &TaskContext) -> Result<(), AppErrorV1> + Send + 'static) -> Result<(), String> {
        let engine = self.engine.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.runtime.submit(SubmitRequest { correlation_id: None, timeout: None, job: Box::new(move |ctx| {
            let result = action(&engine, ctx);
            tx.send(result.as_ref().map(|_| ()).map_err(|error| error.code.clone())).unwrap();
            result.map(|()| TaskExit::Done(json!({"synthetic":true})))
        }) }).unwrap();
        rx.recv_timeout(Duration::from_secs(5)).unwrap()
    }
    fn bake(&self) -> Result<(), String> { self.job(|engine, ctx| engine.bake_turntable(RUN, ctx)) }
    fn upload(&self, name: &'static str) -> Result<(), String> { self.job(move |engine, ctx| engine.upload(RUN, name, ctx)) }
    fn local_test(&self) -> Result<(), String> { self.job(|engine, ctx| engine.local_test(RUN, ctx)) }
    fn local_receipt(&self, outcome: &str, reason: Option<&str>, eligibility: Option<bool>) {
        *self.platform.local_receipt.lock().unwrap() = Some(LocalTestReceipt {
            outcome: outcome.into(), reason: reason.map(str::to_owned), detail: "SDK\u{0000} detail".into(),
            errors: vec!["Synthetic SDK issue".into()], can_publish_avatars: eligibility, sdk_sign_in: Some("panel".into()),
            sdk_review: Some(json!({"state":"checked","platform":"pc","overall":"Good","detail":"","stats":[],"issues":[]})),
        });
    }
    fn receipt(&self, outcome: &str, blueprint: Option<&str>, reason: Option<&str>) {
        *self.platform.receipt.lock().unwrap() = Some(UploadReceipt { outcome: outcome.into(), blueprint_id: blueprint.map(str::to_owned),
            first_upload: true, reason: reason.map(str::to_owned), detail: "SDK said\u{0000} so".into(), errors: vec!["Too many materials".into()],
            sdk_sign_in: Some("panel".into()), sdk_review: None, can_publish_avatars: None });
    }
}
impl Drop for Harness {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.root); }
}

#[test]
fn a_baked_turntable_is_recorded_and_its_frames_are_read_from_its_folder() {
    let h = Harness::new();
    fs::create_dir_all(h.project().join(".vua/turntables/cmd-1")).unwrap();
    assert_eq!(h.bake(), Ok(()));
    let state = h.engine.release_state(RUN).unwrap();
    assert_eq!(state["turntable"]["frames"], 60);
    assert_eq!(state["turntable"]["size"], 512);
    assert_eq!(state["turntable"]["current"], true);
    assert!(!h.project().join(".vua/turntables/cmd-1").exists(), "earlier turntables are removed");
    assert!(h.engine.turntable_frame(RUN, 0).unwrap().starts_with(b"\x89PNG"));
    assert!(h.engine.turntable_frame(RUN, 59).is_ok());
    assert_eq!(h.engine.turntable_frame(RUN, 60).unwrap_err().code, "vua.amf_production.turntable_invalid");
    fs::write(h.project().join(".vua/turntables/cmd-2/frame-03.png"), b"not an image").unwrap();
    assert_eq!(h.engine.turntable_frame(RUN, 3).unwrap_err().code, "vua.amf_production.turntable_invalid");
    let run = h.engine.get(RUN).unwrap();
    assert_eq!(run.phase, "succeeded");
    assert_eq!((run.steps.last().unwrap().key.as_str(), run.steps.last().unwrap().state.as_str()), ("turntable", "succeeded"));
}

#[test]
fn a_turntable_waits_for_a_closed_unchanged_project_and_a_finished_avatar() {
    let h = Harness::new();
    h.platform.editor_open.store(true, Ordering::SeqCst);
    assert_eq!(h.engine.precheck_turntable(RUN).unwrap_err().code, "vua.amf_production.project_open");
    h.platform.editor_open.store(false, Ordering::SeqCst);
    fs::write(h.project().join("Assets/state"), "edited in Unity").unwrap();
    assert_eq!(h.engine.precheck_turntable(RUN).unwrap_err().code, "vua.amf_production.project_drift");
    h.finished_run("ready");
    assert_eq!(h.engine.precheck_turntable(RUN).unwrap_err().code, "vua.amf_production.result_not_ready");
}

#[test]
fn a_turntable_of_another_scene_or_outside_its_folder_is_not_recorded() {
    let h = Harness::new();
    *h.platform.turntable_scene.lock().unwrap() = "scene-2".into();
    assert_eq!(h.bake(), Err("vua.amf_production.project_drift".into()));
    assert!(h.engine.release_state(RUN).unwrap()["turntable"].is_null());
    let run = h.engine.get(RUN).unwrap();
    assert_eq!(run.phase, "succeeded", "a failed bake leaves the finished Avatar as it is");
    assert_eq!(run.steps.last().unwrap().state, "failed");

    *h.platform.turntable_scene.lock().unwrap() = "scene-1".into();
    for folder in [".vua/turntables/../previews", ".vua/previews/cmd-9", ".vua/turntables/a/b", ".vua/turntables/"] {
        *h.platform.turntable_folder.lock().unwrap() = folder.into();
        assert_eq!(h.bake(), Err("vua.amf_production.receipt_invalid".into()), "{folder}");
    }
    let project = h.project();
    let facts = json!({"folder":".vua/turntables/cmd-3","frames":60,"size":512});
    fs::create_dir_all(project.join(".vua/turntables/cmd-3")).unwrap();
    for index in 0..59 {
        fs::write(project.join(format!(".vua/turntables/cmd-3/frame-{index:02}.png")), b"\x89PNG\r\n\x1a\n").unwrap();
    }
    assert_eq!(turntable_from_facts(&facts, &project, "scene-1", "now").unwrap_err().code, "vua.amf_production.receipt_invalid", "a missing frame");
    fs::write(project.join(".vua/turntables/cmd-3/frame-59.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    assert!(turntable_from_facts(&facts, &project, "scene-1", "now").is_ok());
    assert!(turntable_from_facts(&json!({"folder":".vua/turntables/cmd-3","frames":30,"size":512}), &project, "scene-1", "now").is_err());
    assert!(turntable_from_facts(&json!({"folder":".vua/turntables/cmd-3","frames":60,"size":1024}), &project, "scene-1", "now").is_err());
    assert_eq!(turntable_file(&project, "../cmd-3", 0).unwrap_err().code, "vua.amf_production.turntable_invalid");
}

#[test]
fn an_upload_records_the_sdk_receipt_and_keeps_the_finished_avatar() {
    let h = Harness::new();
    h.receipt("succeeded", Some(AVATAR_ID), None);
    assert_eq!(h.upload("Meiyun T-shirt"), Ok(()));
    let request = h.platform.requests.lock().unwrap()[0].clone();
    assert_eq!(request.avatar_name, "Meiyun T-shirt");
    assert_eq!(request.thumbnail.as_deref(), Some(".vua/previews/cmd-0/front.png"));
    let state = h.engine.release_state(RUN).unwrap();
    let upload = &state["uploads"][0];
    assert_eq!(upload["state"], "succeeded");
    assert_eq!(upload["blueprintId"], AVATAR_ID);
    assert_eq!(upload["firstUpload"], true);
    assert_eq!(upload["avatarName"], "Meiyun T-shirt");
    assert_eq!(upload["detail"], "SDK said so", "control characters are left out");
    let run = h.engine.get(RUN).unwrap();
    assert_eq!(run.phase, "succeeded");
    assert_eq!((run.steps.last().unwrap().key.as_str(), run.steps.last().unwrap().state.as_str()), ("upload", "succeeded"));
}

#[test]
fn a_failed_upload_names_its_reason_and_success_needs_an_avatar_id() {
    let h = Harness::new();
    h.receipt("failed", None, Some("validation"));
    assert_eq!(h.upload("Meiyun"), Err("vua.amf_production.upload_failed".into()));
    let state = h.engine.release_state(RUN).unwrap();
    assert_eq!(state["uploads"][0]["state"], "failed");
    assert_eq!(state["uploads"][0]["reason"], "validation");
    assert_eq!(state["uploads"][0]["errors"], json!(["Too many materials"]));
    assert_eq!(h.engine.get(RUN).unwrap().phase, "succeeded");

    h.receipt("failed", None, Some("invented_reason"));
    assert!(h.upload("Meiyun").is_err());
    assert_eq!(h.engine.release_state(RUN).unwrap()["uploads"][0]["reason"], "unknown");

    h.receipt("succeeded", Some("avtr_not-an-id"), None);
    assert_eq!(h.upload("Meiyun"), Err("vua.amf_production.receipt_invalid".into()));
    assert_eq!(h.engine.release_state(RUN).unwrap()["uploads"][0]["state"], "failed");

    *h.platform.receipt.lock().unwrap() = None;
    assert_eq!(h.upload("Meiyun"), Err("vua.amf_production.upload_timeout".into()));
    let state = h.engine.release_state(RUN).unwrap();
    assert_eq!(state["uploads"][0]["reason"], "timeout");
    assert_eq!(state["uploads"].as_array().unwrap().len(), 4, "newest first, each upload kept");
}

#[test]
fn an_upload_needs_a_name_and_the_checked_project_unless_unity_holds_it() {
    let h = Harness::new();
    let long = "x".repeat(65);
    for name in ["", " Meiyun", "Meiyun\n", long.as_str()] {
        assert_eq!(h.engine.precheck_upload(RUN, name).unwrap_err().code, "vua.amf_production.avatar_name_invalid", "{name:?}");
    }
    assert!(h.engine.precheck_upload(RUN, &"名".repeat(64)).is_ok());
    fs::write(h.project().join("Assets/state"), "edited in Unity").unwrap();
    assert_eq!(h.engine.precheck_upload(RUN, "Meiyun").unwrap_err().code, "vua.amf_production.project_drift");
    h.platform.editor_open.store(true, Ordering::SeqCst);
    assert!(h.engine.precheck_upload(RUN, "Meiyun").is_ok(), "the SDK uploads the scene open in Unity");
    assert_eq!(h.engine.release_state(RUN).unwrap()["editorOpen"], true);
    h.finished_run("failed");
    assert_eq!(h.engine.precheck_upload(RUN, "Meiyun").unwrap_err().code, "vua.amf_production.result_not_ready");
}

#[test]
fn progress_is_recorded_once_per_change_and_an_unfinished_upload_reads_interrupted() {
    let h = Harness::new();
    let mut record = h.engine.release_record(RUN).unwrap();
    record.uploads.push(serde_json::from_value(json!({"uploadId":"upload-old","avatarName":"Meiyun","startedAt":"2026-10-17T00:00:00Z",
        "state":"running","stage":"building"})).unwrap());
    fs::create_dir_all(h.root.join("production/releases")).unwrap();
    fs::write(h.root.join(format!("production/releases/{RUN}.json")), serde_json::to_vec(&record).unwrap()).unwrap();
    h.receipt("succeeded", Some(AVATAR_ID), None);
    assert_eq!(h.upload("Meiyun"), Ok(()));
    let state = h.engine.release_state(RUN).unwrap();
    assert_eq!(state["uploads"][1]["state"], "failed");
    assert_eq!(state["uploads"][1]["reason"], "interrupted");
    // The last reported stage before the receipt stays readable.
    assert_eq!(state["uploads"][0]["status"], "Synthetic status");
}

#[test]
fn avatar_ids_and_names_have_a_fixed_shape() {
    assert!(is_blueprint_id(AVATAR_ID));
    assert!(is_blueprint_id("avtr_0D1E2F3A-4B5C-4D6E-8F90-A1B2C3D4E5F6"));
    for id in ["", "avtr_", "wrld_0d1e2f3a-4b5c-4d6e-8f90-a1b2c3d4e5f6", "avtr_0d1e2f3a4b5c4d6e8f90a1b2c3d4e5f6", "avtr_0d1e2f3a-4b5c-4d6e-8f90-a1b2c3d4e5fg"] {
        assert!(!is_blueprint_id(id), "{id}");
    }
    assert_eq!(bounded_text("a\u{0001}b\nc", 3), "ab\n");
}

#[test]
fn the_sdk_check_read_after_an_upload_replaces_the_line_facts() {
    let h = Harness::new();
    let fixed = json!({"state":"checked","platform":"pc","overall":"Good","detail":"","stats":[],
        "issues":[{"level":"warning","text":"This avatar is shorter than average.","canFix":false}]});
    h.receipt("failed", None, Some("validation"));
    h.platform.receipt.lock().unwrap().as_mut().unwrap().sdk_review = Some(fixed.clone());
    assert!(h.upload("Meiyun T-shirt").is_err());
    assert_eq!(h.engine.get(RUN).unwrap().result["facts"]["sdkReview"], fixed, "the player's Auto Fix shows even when the upload failed");

    // A review the Editor could not read leaves the facts as they were.
    h.receipt("succeeded", Some(AVATAR_ID), None);
    h.platform.receipt.lock().unwrap().as_mut().unwrap().sdk_review = Some(json!({"state":"unavailable"}));
    assert_eq!(h.upload("Meiyun T-shirt"), Ok(()));
    let run = h.engine.get(RUN).unwrap();
    assert_eq!(run.result["facts"]["sdkReview"], fixed);
    assert_eq!(run.result["facts"]["avatarGlobalObjectId"], "GlobalObjectId_V1-avatar");
}

#[test]
fn a_visitor_local_test_keeps_the_production_result_and_records_observed_eligibility() {
    let h = Harness::new();
    let before = h.engine.get(RUN).unwrap();
    assert!(h.engine.release_state(RUN).unwrap()["uploadEligibility"].is_null());
    h.local_receipt("succeeded", None, Some(false));
    assert_eq!(h.local_test(), Ok(()));
    let state = h.engine.release_state(RUN).unwrap();
    let test = &state["localTests"][0];
    assert_eq!(test["state"], "succeeded");
    assert_eq!(test["stage"], "finishing");
    assert_eq!(test["detail"], "SDK detail");
    assert_eq!(test["sdkReview"]["state"], "checked");
    assert_eq!(state["uploadEligibility"]["canPublish"], false);
    assert_eq!(state["uploads"], json!([]));
    let after = h.engine.get(RUN).unwrap();
    assert_eq!(after.phase, before.phase);
    assert_eq!(after.result, before.result);
    assert_eq!(after.unity_fingerprint, before.unity_fingerprint);
    assert_eq!(after.project_fingerprint, before.project_fingerprint);
    assert_eq!(after.steps.last().unwrap().key, "local_test");
    let stored: serde_json::Value = serde_json::from_slice(&fs::read(h.root.join("production/upload-eligibility.json")).unwrap()).unwrap();
    assert_eq!(stored.as_object().unwrap().len(), 3, "no account identity is persisted");
    assert_eq!(stored["schemaVersion"], "0.1");

    h.local_receipt("failed", Some("validation"), None);
    assert!(h.local_test().is_err());
    assert_eq!(h.engine.release_state(RUN).unwrap()["uploadEligibility"]["canPublish"], false, "no observation does not overwrite one");
    h.receipt("succeeded", Some(AVATAR_ID), None);
    h.platform.receipt.lock().unwrap().as_mut().unwrap().can_publish_avatars = Some(true);
    assert_eq!(h.upload("Avatar"), Ok(()));
    assert_eq!(h.engine.release_state(RUN).unwrap()["uploadEligibility"]["canPublish"], true, "uploads refresh the shared observation too");
}

#[test]
fn local_test_admission_matches_upload_without_name_or_thumbnail() {
    let h = Harness::new();
    fs::remove_file(h.project().join(".vua/previews/cmd-0/front.png")).unwrap();
    assert!(h.engine.precheck_local_test(RUN).is_ok());
    fs::write(h.project().join("Assets/state"), "edited").unwrap();
    assert_eq!(h.engine.precheck_local_test(RUN).unwrap_err().code, "vua.amf_production.project_drift");
    h.platform.editor_open.store(true, Ordering::SeqCst);
    assert!(h.engine.precheck_local_test(RUN).is_ok());
    h.finished_run("ready");
    assert_eq!(h.engine.precheck_local_test(RUN).unwrap_err().code, "vua.amf_production.result_not_ready");
}

#[test]
fn local_test_failures_cancellation_and_timeouts_keep_the_finished_run() {
    let h = Harness::new();
    let original = h.engine.get(RUN).unwrap().result;
    for (outcome, reason, code) in [("failed", "validation", "local_test_failed"), ("failed", "platform_unsupported", "local_test_failed"),
        ("cancelled", "cancelled", "local_test_cancelled")] {
        h.local_receipt(outcome, Some(reason), None);
        assert_eq!(h.local_test(), Err(format!("vua.amf_production.{code}")));
        let state = h.engine.release_state(RUN).unwrap();
        assert_eq!(state["localTests"][0]["reason"], reason);
        assert_eq!(state["localTests"][0]["stage"], "building", "unknown progress stages are ignored");
        assert_eq!(h.engine.get(RUN).unwrap().result, original);
        assert_eq!(h.engine.get(RUN).unwrap().phase, "succeeded");
    }
    *h.platform.local_receipt.lock().unwrap() = None;
    assert_eq!(h.local_test(), Err("vua.amf_production.local_test_timeout".into()));
    assert_eq!(h.engine.release_state(RUN).unwrap()["localTests"][0]["reason"], "timeout");
}

#[test]
fn old_release_records_load_and_local_test_history_is_bounded() {
    let h = Harness::new();
    fs::create_dir_all(h.root.join("production/releases")).unwrap();
    fs::write(h.root.join("production/releases").join(format!("{RUN}.json")),
        serde_json::to_vec(&json!({"schemaVersion":"0.1","runId":RUN,"turntable":null,"uploads":[]})).unwrap()).unwrap();
    assert!(h.engine.release_record(RUN).unwrap().local_tests.is_empty());
    h.local_receipt("succeeded", None, None);
    for _ in 0..23 { h.local_test().unwrap(); }
    let record = h.engine.release_record(RUN).unwrap();
    let state = h.engine.release_state(RUN).unwrap();
    assert_eq!(record.local_tests.len(), 20);
    assert_eq!(state["localTests"].as_array().unwrap().len(), 10);
    assert_eq!(state["localTests"][0]["testId"], record.local_tests.last().unwrap().test_id);
    fs::write(h.root.join("production/upload-eligibility.json"), b"broken").unwrap();
    assert!(h.engine.release_state(RUN).unwrap()["uploadEligibility"].is_null());
    fs::write(h.root.join("production/upload-eligibility.json"),
        br#"{"schemaVersion":"0.1","canPublish":false,"observedAt":"invalid"}"#).unwrap();
    assert!(h.engine.release_state(RUN).unwrap()["uploadEligibility"].is_null());
}

#[test]
fn an_unwritable_eligibility_cache_does_not_lose_the_sdk_outcome() {
    let h = Harness::new();
    fs::create_dir_all(h.root.join("production/upload-eligibility.pending")).unwrap();
    h.local_receipt("succeeded", None, Some(false));
    assert_eq!(h.local_test(), Ok(()));
    h.receipt("succeeded", Some(AVATAR_ID), None);
    h.platform.receipt.lock().unwrap().as_mut().unwrap().can_publish_avatars = Some(true);
    assert_eq!(h.upload("Avatar"), Ok(()));
    let state = h.engine.release_state(RUN).unwrap();
    for record in [&state["uploads"][0], &state["localTests"][0]] {
        assert_eq!(record["state"], "succeeded");
        assert!(record["errors"].as_array().unwrap().iter().any(|value| value.as_str().unwrap().contains("eligibility could not be saved")));
    }
    assert_eq!(state["uploads"][0]["blueprintId"], AVATAR_ID);
    assert!(state["uploadEligibility"].is_null());
}
