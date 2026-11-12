//! F2 acceptance: the production use-case surface is Gateway-callable over
//! the supervised Provider protocol — persisted tasks, commandId
//! idempotency, worker completion, restart recovery — with the real
//! material intake executor driven by a fake Bridge.

use std::fs;
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use flate2::write::GzEncoder;
use tar::{Builder, Header};

use serde_json::{json, Value};

use vua_orchestrator::{
    BuildRecordStore, FileSystemSnapshotStore, FixedClock, TaskState, UnityBridge, UnityCommand,
    UnityResult, VpmBackend, VpmCapabilities,
};
use vua_project_manager::read_pending_mutation;
use vua_provider_host::ProductionConfig;
use vua_unity_bridge::{LocalPackageIdentityStore, MaterialExecutor};

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-prodhost-{label}-{nanos}"))
}

/// A fake Bridge: every command succeeds and reports a fresh fingerprint,
/// so a direct-mode confirmed run completes in milliseconds on a worker.
#[derive(Clone)]
struct FakeBridge {
    commands: Arc<Mutex<Vec<UnityCommand>>>,
}

impl UnityBridge for FakeBridge {
    fn execute(
        &self,
        _project: &vua_orchestrator::ProjectRef,
        command: &UnityCommand,
    ) -> Result<UnityResult, vua_orchestrator::BridgeError> {
        self.commands.lock().unwrap().push(command.clone());
        // 第 158 批（BOARD #45(2)）：the material executor parses a
        // Succeeded validate receipt's loadedAssetPaths strictly (missing
        // field = honest bridge_failed), so this fake mirrors the real C#
        // handler — the Ordinal-sorted expected list IS the loaded list.
        let mut data = serde_json::json!({
            "projectFingerprint": format!("fp-{}", self.commands.lock().unwrap().len())
        });
        if command.operation == vua_orchestrator::UnityOperation::ValidateAssetPaths {
            let mut loaded = command.payload.expected_asset_paths.clone();
            loaded.sort();
            data["loadedAssetPaths"] = loaded.into_iter().map(serde_json::Value::String).collect();
        }
        Ok(UnityResult {
            schema_version: 1,
            command_id: command.command_id.clone(),
            status: vua_orchestrator::ResultStatus::Succeeded,
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
        _: &vua_orchestrator::ProjectRef,
        _: &[vua_orchestrator::PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, vua_orchestrator::AppErrorV1> {
        unreachable!("direct mode never previews")
    }
    fn apply_install(
        &self,
        _: &vua_orchestrator::ProjectRef,
        _: &[vua_orchestrator::PackageRequestV1],
        _: &str,
    ) -> Result<serde_json::Value, vua_orchestrator::AppErrorV1> {
        unreachable!("direct mode never installs")
    }
    fn create_project(
        &self,
        _: &Path,
        _: &str,
        _: Option<&str>,
    ) -> Result<vua_orchestrator::ProjectRef, vua_orchestrator::AppErrorV1> {
        unreachable!("direct mode never creates projects")
    }
}

/// Feeds frames one at a time, pausing between frames so the production
/// worker (which finishes in milliseconds) reliably completes before the
/// host processes the next frame and drains its events.
struct Paced {
    inner: std::io::Cursor<Vec<u8>>,
}

impl Read for Paced {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::thread::sleep(Duration::from_millis(120));
        self.inner.read(buf)
    }
}

impl BufRead for Paced {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.inner.fill_buf()
    }
    fn consume(&mut self, amount: usize) {
        self.inner.consume(amount);
    }
}

fn frames_input(frames: Vec<Value>) -> Paced {
    let mut bytes = Vec::new();
    for frame in frames {
        bytes.extend_from_slice(frame.to_string().as_bytes());
        bytes.push(b'\n');
    }
    Paced {
        inner: std::io::Cursor::new(bytes),
    }
}

fn frame(id: &str, payload: Value) -> Value {
    json!({
        "frameVersion": "0.1",
        "frameId": id,
        "kind": "request",
        "payload": payload,
    })
}

fn request(request_id: &str, method: &str, command_id: &str, params: Value) -> Value {
    json!({
        "contractVersion": "0.1",
        "requestId": request_id,
        "correlationId": "corr-prod",
        "kind": "command",
        "method": method,
        "commandId": command_id,
        "params": params,
    })
}

/// Parses the COMPLETE frames only. A frame is terminated by '\n', so a
/// trailing fragment means the host is still mid-frame — the frame loop's
/// `write_frame` emits the JSON body through several small `Write` calls
/// before the newline, and a concurrent `wait_for_response` poll used to
/// clone the buffer between two of those writes and panic on the fragment
/// ("EOF while parsing a string", the ph_004/ph_010 CI family at :163).
/// The protocol boundary is the newline: everything after the last one is
/// an unfinished frame that a poll simply re-reads on its next tick, and a
/// frame that never completes still fails through wait_for_response's
/// deadline or the post-join frame assertions. No assertion is weakened:
/// every complete frame parses exactly as before.
fn parse_frames(bytes: &[u8]) -> Vec<Value> {
    let text = String::from_utf8_lossy(bytes);
    let complete = match text.rfind('\n') {
        Some(end) => &text[..end],
        None => "",
    };
    complete
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn response_frame<'a>(frames: &'a [Value], request_id: &str) -> &'a Value {
    frames
        .iter()
        .find(|frame| frame["kind"] == "response" && frame["payload"]["requestId"] == request_id)
        .unwrap_or_else(|| panic!("no response for {request_id}: {frames:?}"))
}

/// The success envelope wraps the application payload under "value".
fn response_payload(frames: &[Value], request_id: &str) -> Value {
    response_frame(frames, request_id)["payload"]["value"].clone()
}

fn make_world(label: &str) -> (PathBuf, PathBuf, PathBuf) {
    let base = temp_root(label);
    let source = base.join("source");
    fs::create_dir_all(&source).unwrap();
    let package = source.join("pack.unitypackage");
    let file = fs::File::create(&package).unwrap();
    let mut builder = Builder::new(GzEncoder::new(file, flate2::Compression::default()));
    let mut header = Header::new_gnu();
    header.set_size(8);
    header.set_cksum();
    builder
        .append_data(&mut header, "Assets/Asset.prefab", &b"fixture!"[..])
        .unwrap();
    builder.finish().unwrap();

    let project_root = base.join("target");
    for dir in ["Assets", "Packages", "ProjectSettings"] {
        fs::create_dir_all(project_root.join(dir)).unwrap();
    }
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
    (base, source, project_root)
}

fn production_config(base: &Path, _project_root: &Path) -> (ProductionConfig, FakeBridge) {
    let bridge = FakeBridge {
        commands: Arc::new(Mutex::new(Vec::new())),
    };
    let executor = Arc::new(MaterialExecutor::new(
        Arc::new(bridge.clone()),
        FileSystemSnapshotStore,
        Arc::new(NoVpm),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-05T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    ));
    (
        ProductionConfig {
            executor,
            records: Arc::new(BuildRecordStore::new(base.join("records"))),
        },
        bridge,
    )
}

fn plan_params(inspection_id: &str) -> Value {
    json!({
        "inspectionId": inspection_id,
        "mode": "direct_unity_package",
    })
}

#[test]
fn ph_001_full_command_chain_runs_persists_and_replays() {
    let (base, source, project_root) = make_world("chain");
    let (config, _bridge) = production_config(&base, &project_root);

    // Run 1: Inspect. The fast stage drives inline to Succeeded.
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-inspect-1",
                    "production.startInspection",
                    "cmd-inspect-1",
                    inspect_params(&source, &project_root, &base),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let inspect = response_payload(&frames, "req-inspect-1");
    assert_eq!(inspect["task"]["state"], "succeeded", "inspect: {inspect}");
    let inspection_id = inspect["inspectionId"].as_str().unwrap().to_owned();

    // Run 2 (same store): Plan from the persisted inspection.
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-plan-1",
                    "production.requestPlan",
                    "cmd-plan-1",
                    plan_params(&inspection_id),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let plan = response_payload(&frames, "req-plan-1");
    if plan["planId"].is_null() {
        let frame = frames
            .iter()
            .find(|frame| {
                frame["kind"] == "response" && frame["payload"]["requestId"] == "req-plan-1"
            })
            .expect("plan response frame");
        panic!("plan-1 full frame: {}", frame);
    }
    assert_eq!(plan["task"]["state"], "succeeded", "plan: {plan}");
    let plan_id = plan["planId"].as_str().unwrap().to_owned();

    // Run 2b: read the plan document — its revision is the confirm binding.
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-getplan-1",
                    "production.getPlan",
                    "",
                    json!({ "planId": plan_id }),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    if response_payload(&frames, "req-getplan-1").is_null() {
        let frame = frames
            .iter()
            .find(|frame| {
                frame["kind"] == "response" && frame["payload"]["requestId"] == "req-getplan-1"
            })
            .expect("getplan response frame");
        panic!("getplan full frame: {}", frame);
    }
    let plan_document = response_payload(&frames, "req-getplan-1");
    assert_eq!(plan_document["plan"]["planId"], plan_id);
    assert_eq!(
        plan_document["plan"]["stages"]
            .as_array()
            .expect("stages")
            .len(),
        5
    );
    let revision = plan_document["plan"]["revision"]
        .as_u64()
        .expect("plan revision");

    // Run 3: Confirm. The response carries Running; the worker completes
    // through the store afterwards.
    let (config, bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-confirm-1",
                    "production.confirmPlan",
                    "cmd-confirm-1",
                    confirm_params(&plan_id, revision),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let confirm = response_payload(&frames, "req-confirm-1");
    assert_eq!(confirm["task"]["state"], "running", "confirm: {confirm}");
    let confirm_task_id = confirm["task"]["taskId"].as_str().unwrap().to_owned();

    // Reopen: the authoritative terminal state was persisted by the worker.
    let store = vua_orchestrator::SqliteTaskStore::open(base.join("provider.db")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store
            .task(&confirm_task_id)
            .unwrap()
            .expect("task persisted");
        if task.state.is_terminal() {
            break task;
        }
        assert!(Instant::now() < deadline, "worker never completed");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(task.state, TaskState::Succeeded, "{task:?}");
    let result = task.result.expect("result payload");
    assert_eq!(result["status"], "succeeded");
    assert_eq!(result["rollback"], "not_needed");
    assert_eq!(
        bridge.commands.lock().unwrap().len(),
        2,
        "import + validation"
    );

    // The published receipt is Gateway-readable via getBuildRecord.
    let record_id = result["buildRecordId"].as_str().unwrap().to_owned();
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-record-1",
                    "production.getBuildRecord",
                    "",
                    json!({ "buildRecordId": record_id }),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let record = response_payload(&frames, "req-record-1");
    assert_eq!(record["buildRecord"]["recordId"], record_id.as_str());
    assert_eq!(record["buildRecord"]["status"], "succeeded");

    // commandId idempotency: re-sending confirm replays the same task and
    // never touches Unity again.
    let (config, bridge2) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-confirm-2",
                    "production.confirmPlan",
                    "cmd-confirm-1",
                    confirm_params(&plan_id, revision),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let replay = response_payload(&frames, "req-confirm-2");
    assert_eq!(
        replay["task"]["taskId"],
        confirm_task_id.as_str(),
        "same task replays"
    );
    assert_eq!(
        bridge2.commands.lock().unwrap().len(),
        0,
        "replay touches no Unity"
    );

    let _ = fs::remove_dir_all(&base);
}

fn confirm_params(plan_id: &str, revision: u64) -> Value {
    json!({
        "planId": plan_id,
        "observedRevision": revision,
        "riskChoice": "continue",
        "rememberForSession": false,
    })
}

fn inspect_params(source: &Path, project_root: &Path, base: &Path) -> Value {
    json!({
        "sourceFolder": source.to_string_lossy(),
        "projectRoot": project_root.to_string_lossy(),
        "artifactOutputRoot": base.join("artifacts").to_string_lossy(),
        "projectId": "project",
    })
}

fn recover_params_v02(decision: &str, original_task_id: &str, plan_id: &str) -> Value {
    json!({
        "taskId": original_task_id,
        "decision": decision,
        "decisionId": format!("kernel-decision-{decision}"),
        "planId": plan_id,
    })
}

#[test]
fn ph_002_unavailable_when_production_is_not_wired() {
    let (base, source, _project_root) = make_world("unavailable");
    let mut output_buffer = Vec::new();
    vua_provider_host::run_provider_host(
        frames_input(vec![frame(
            "f1",
            request(
                "req-inspect-x",
                "production.startInspection",
                "cmd-inspect-x",
                json!({ "sourceFolder": source.to_string_lossy() }),
            ),
        )]),
        &mut output_buffer,
        base.join("provider.db"),
    )
    .unwrap();
    let frames = parse_frames(&output_buffer);
    let error = response_frame(&frames, "req-inspect-x")["payload"]["error"].clone();
    assert_eq!(error["code"], "vua.production.unavailable");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn ph_003_interrupted_production_tasks_fail_as_recoverable_on_restart() {
    let (base, _source, _project_root) = make_world("interrupted");
    let path = base.join("provider.db");
    {
        let store = vua_orchestrator::SqliteTaskStore::open(&path).unwrap();
        store
            .accept_task(&vua_orchestrator::NewTask {
                task_id: "prod-deadbeef0001".into(),
                correlation_id: "corr-dead".into(),
                occurred_at: "2026-09-05T00:00:00Z".into(),
            })
            .unwrap();
        // Simulate a worker that died mid-run: the task is left in Running.
        store
            .mutate_task(
                "prod-deadbeef0001",
                1,
                "2026-09-05T00:00:01Z",
                vua_orchestrator::TaskMutation::Transition {
                    state: TaskState::Preparing,
                    payload: json!({}),
                },
            )
            .unwrap();
        store
            .mutate_task(
                "prod-deadbeef0001",
                2,
                "2026-09-05T00:00:02Z",
                vua_orchestrator::TaskMutation::Transition {
                    state: TaskState::Running,
                    payload: json!({}),
                },
            )
            .unwrap();
        store.checkpoint().unwrap();
    }

    let (config, _bridge) = production_config(&base, &base.join("target"));
    let mut output_buffer = Vec::new();
    vua_provider_host::run_provider_host_with(
        frames_input(vec![frame(
            "f1",
            request("req-list", "task.list", "", json!({})),
        )]),
        &mut output_buffer,
        &path,
        Some(config),
    )
    .unwrap();

    let store = vua_orchestrator::SqliteTaskStore::open(&path).unwrap();
    let task = store
        .task("prod-deadbeef0001")
        .unwrap()
        .expect("task persists");
    assert_eq!(task.state, TaskState::Failed);
    let error = task.error.expect("interrupted run carries the typed error");
    assert_eq!(error.code, "vua.task.interrupted");
    let _ = fs::remove_dir_all(&base);
}

// --- dynamic harness: the host runs on a thread; the test feeds frames
// interactively so a cancel request can land while a worker is running ---

struct ChannelReader {
    receiver: std::sync::mpsc::Receiver<Vec<u8>>,
    buffer: Vec<u8>,
    position: usize,
    done: bool,
}

impl Read for ChannelReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.position >= self.buffer.len() {
            if self.done {
                return Ok(0);
            }
            match self.receiver.recv() {
                Ok(frame) => {
                    self.buffer = frame;
                    self.position = 0;
                }
                Err(_) => {
                    self.done = true;
                    return Ok(0);
                }
            }
        }
        let remaining = self.buffer.len() - self.position;
        let amount = remaining.min(buf.len());
        buf[..amount].copy_from_slice(&self.buffer[self.position..self.position + amount]);
        self.position += amount;
        Ok(amount)
    }
}

impl BufRead for ChannelReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.position >= self.buffer.len() && !self.done {
            match self.receiver.recv() {
                Ok(frame) => {
                    self.buffer = frame;
                    self.position = 0;
                }
                Err(_) => self.done = true,
            }
        }
        Ok(&self.buffer[self.position..])
    }
    fn consume(&mut self, amount: usize) {
        self.position += amount;
    }
}

#[derive(Clone)]
struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn wait_for_response(buffer: &Arc<Mutex<Vec<u8>>>, request_id: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let bytes = buffer.lock().unwrap().clone();
        let frames = parse_frames(&bytes);
        let hit = frames.iter().find(|frame| {
            frame["kind"] == "response" && frame["payload"]["requestId"] == request_id
        });
        if let Some(frame) = hit {
            return frame["payload"]["value"].clone();
        }
        assert!(Instant::now() < deadline, "no response for {request_id}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn ph_004_cancel_request_reaches_the_running_worker_token() {
    use std::sync::atomic::{AtomicBool, Ordering};

    // Blocks inside the FIRST Bridge command until released, so the run can
    // only move forward once the host bridges the cancel request to the
    // per-execution token.
    struct BlockingBridge {
        released: Arc<AtomicBool>,
    }
    impl UnityBridge for BlockingBridge {
        fn execute(
            &self,
            _project: &vua_orchestrator::ProjectRef,
            command: &UnityCommand,
        ) -> Result<UnityResult, vua_orchestrator::BridgeError> {
            let deadline = Instant::now() + Duration::from_secs(10);
            while !self.released.load(Ordering::SeqCst) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            // 第 158 批（BOARD #45(2)）：mirror the real C# handler — a
            // Succeeded validate receipt always carries the loadedAssetPaths
            // evidence, and the executor now parses it strictly.
            let mut data = serde_json::json!({"projectFingerprint": "fp-blocked"});
            if command.operation == vua_orchestrator::UnityOperation::ValidateAssetPaths {
                let mut loaded = command.payload.expected_asset_paths.clone();
                loaded.sort();
                data["loadedAssetPaths"] =
                    loaded.into_iter().map(serde_json::Value::String).collect();
            }
            Ok(UnityResult {
                schema_version: 1,
                command_id: command.command_id.clone(),
                status: vua_orchestrator::ResultStatus::Succeeded,
                changed_paths: vec![],
                diagnostics: vec![],
                data,
                steps: Vec::new(),
                replayed: None,
                snapshot_id: None,
                restored_from: None,
                project_fingerprint_before: None,
            })
        }
    }

    let (base, source, project_root) = make_world("cancel");
    let released = Arc::new(AtomicBool::new(false));
    let executor = Arc::new(MaterialExecutor::new(
        Arc::new(BlockingBridge {
            released: Arc::clone(&released),
        }),
        FileSystemSnapshotStore,
        Arc::new(NoVpm),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-05T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    ));
    let config = ProductionConfig {
        executor,
        records: Arc::new(BuildRecordStore::new(base.join("records"))),
    };

    let (sender, receiver) = std::sync::mpsc::channel::<Vec<u8>>();
    let output_buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = SharedWriter(Arc::clone(&output_buffer));
    let path = base.join("provider.db");
    let host = std::thread::spawn(move || {
        let _ = vua_provider_host::run_provider_host_with(
            ChannelReader {
                receiver,
                buffer: Vec::new(),
                position: 0,
                done: false,
            },
            writer,
            path,
            Some(config),
        );
    });

    // Inspect + plan (fast stages, inline).
    let send = |frame_value: Value| {
        let mut bytes = frame_value.to_string().into_bytes();
        bytes.push(b'\n');
        sender.send(bytes).unwrap();
    };
    send(frame(
        "f1",
        request(
            "req-inspect",
            "production.startInspection",
            "cmd-inspect",
            inspect_params(&source, &project_root, &base),
        ),
    ));
    let inspect = wait_for_response(&output_buffer, "req-inspect");
    let inspection_id = inspect["inspectionId"].as_str().unwrap().to_owned();
    send(frame(
        "f2",
        request(
            "req-plan",
            "production.requestPlan",
            "cmd-plan",
            plan_params(&inspection_id),
        ),
    ));
    let plan = wait_for_response(&output_buffer, "req-plan");
    let plan_id = plan["planId"].as_str().unwrap().to_owned();
    let plan_revision = plan["revision"].as_u64().unwrap();

    // Confirm: the worker enters the blocking Bridge command.
    send(frame(
        "f3",
        request(
            "req-confirm",
            "production.confirmPlan",
            "cmd-confirm-cancel",
            confirm_params(&plan_id, plan_revision),
        ),
    ));
    let confirm = wait_for_response(&output_buffer, "req-confirm");
    assert_eq!(confirm["task"]["state"], "running", "confirm: {confirm}");
    let confirm_task_id = confirm["task"]["taskId"].as_str().unwrap().to_owned();

    // The cancel request arrives while the worker sits inside the Bridge
    // command: the host bridges it to the token, the command returns, and
    // the run cancels at the next step boundary.
    send(frame(
        "f4",
        request("req-cancel", "task.requestCancellation", "cmd-cancel", {
            json!({ "taskId": confirm_task_id })
        }),
    ));
    let _cancel = wait_for_response(&output_buffer, "req-cancel");
    released.store(true, Ordering::SeqCst);

    // Pump one frame so the completion event drains, then close stdin.
    send(frame("f5", request("req-pump", "task.list", "", json!({}))));
    drop(sender);
    host.join().unwrap();

    let store = vua_orchestrator::SqliteTaskStore::open(base.join("provider.db")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store
            .task(&confirm_task_id)
            .unwrap()
            .expect("task persists");
        if task.state.is_terminal() {
            break task;
        }
        assert!(
            Instant::now() < deadline,
            "worker never completed the cancel"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(
        task.state,
        TaskState::Cancelled,
        "cancellation completed: {task:?}"
    );
    assert!(task.cancel_requested);
    let _ = fs::remove_dir_all(&base);
}

/// A Bridge whose every command fails — simulates a broken Unity run so a
/// confirmed confirm lands Failed with a receipt and a recovery snapshot.
struct FailingBridge;

impl UnityBridge for FailingBridge {
    fn execute(
        &self,
        _project: &vua_orchestrator::ProjectRef,
        _command: &UnityCommand,
    ) -> Result<UnityResult, vua_orchestrator::BridgeError> {
        Err(vua_orchestrator::BridgeError::TimedOut)
    }
}

/// Runs inspect + plan + a FAILING confirm; returns the confirm task id and
/// plan task id for recovery requests.
fn run_failing_confirm(label: &str) -> (PathBuf, PathBuf, PathBuf, String, u64, String) {
    let (base, source, project_root) = make_world(label);

    // Run 1: inspect + plan (real executor, instant fake bridge).
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![
                frame(
                    "f1",
                    request(
                        "req-inspect",
                        "production.startInspection",
                        "cmd-inspect",
                        inspect_params(&source, &project_root, &base),
                    ),
                ),
                frame("f2", request("req-pump", "task.list", "", json!({}))),
            ]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let inspection_id = response_payload(&frames, "req-inspect")["inspectionId"]
        .as_str()
        .unwrap()
        .to_owned();

    // Run 2: plan.
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-plan",
                    "production.requestPlan",
                    "cmd-plan",
                    plan_params(&inspection_id),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let plan_id = response_payload(&frames, "req-plan")["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    let plan_revision = response_payload(&frames, "req-plan")["revision"]
        .as_u64()
        .unwrap();

    // Run 3: confirm against the FAILING bridge. The worker persists the
    // Failed terminal state (with a receipt and recovery snapshot).
    let executor = Arc::new(MaterialExecutor::new(
        Arc::new(FailingBridge),
        FileSystemSnapshotStore,
        Arc::new(NoVpm),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-05T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    ));
    let config = ProductionConfig {
        executor,
        records: Arc::new(BuildRecordStore::new(base.join("records"))),
    };
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![
                frame(
                    "f1",
                    request(
                        "req-confirm",
                        "production.confirmPlan",
                        "cmd-confirm-fail",
                        confirm_params(&plan_id, plan_revision),
                    ),
                ),
                frame("f2", request("req-pump", "task.list", "", json!({}))),
            ]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let confirm = response_payload(&frames, "req-confirm");
    let confirm_task_id = confirm["task"]["taskId"].as_str().unwrap().to_owned();

    let store = vua_orchestrator::SqliteTaskStore::open(base.join("provider.db")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store
            .task(&confirm_task_id)
            .unwrap()
            .expect("task persists");
        if task.state.is_terminal() {
            break task;
        }
        assert!(Instant::now() < deadline, "failing confirm never completed");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(task.state, TaskState::Failed, "{task:?}");
    assert_eq!(
        task.error.as_ref().expect("typed error").code,
        "vua.material.bridge_timeout"
    );
    assert_eq!(
        task.error.as_ref().expect("typed error").message_key,
        "errors.material.executionFailed",
        "a bridge-segment failure keeps the standing face at the worker too — only the \
         provision segment presents provisionFailed (batch 150 word-face law)"
    );

    (
        base,
        source,
        project_root,
        plan_id,
        plan_revision,
        confirm_task_id,
    )
}

#[test]
fn ph_005_recover_continue_reruns_fresh_and_succeeds() {
    let (base, _source, project_root, plan_id, _plan_revision, confirm_task_id) =
        run_failing_confirm("recover-continue");

    // A succeeding host run recovers with decision=continue: the plan runs
    // again fresh and this time the fake Bridge succeeds.
    let (config, bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-recover",
                    "production.recover",
                    "cmd-recover-continue",
                    recover_params_v02("continue", &confirm_task_id, &plan_id),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    if response_payload(&frames, "req-recover").is_null() {
        let frame = frames
            .iter()
            .find(|frame| {
                frame["kind"] == "response" && frame["payload"]["requestId"] == "req-recover"
            })
            .expect("recover response frame");
        panic!("recover error frame: {}", frame);
    }
    let recover = response_payload(&frames, "req-recover");
    let recover_task_id = recover["task"]["taskId"].as_str().unwrap().to_owned();
    assert_ne!(recover_task_id, confirm_task_id, "recovery is its own task");

    let store = vua_orchestrator::SqliteTaskStore::open(base.join("provider.db")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store
            .task(&recover_task_id)
            .unwrap()
            .expect("task persists");
        if task.state.is_terminal() {
            break task;
        }
        assert!(Instant::now() < deadline, "recover never completed");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(task.state, TaskState::Succeeded, "{task:?}");
    let result = task.result.expect("result");
    assert_eq!(result["status"], "succeeded");
    // inspect (project credential) + import + validation
    assert_eq!(
        bridge.commands.lock().unwrap().len(),
        3,
        "continue really re-runs"
    );

    // The new attempt's receipt lives under the next free attempt id; the
    // failed receipt stays intact as audit history.
    let records = BuildRecordStore::new(base.join("records"));
    let record_id = result["buildRecordId"].as_str().unwrap().to_owned();
    assert_ne!(record_id, "material-unknown");
    assert!(records.read(&record_id).is_ok());

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn ph_007_recover_rollback_restores_without_rerunning() {
    let (base, _source, project_root, plan_id, _plan_revision, confirm_task_id) =
        run_failing_confirm("recover-rollback");
    let manifest_before = fs::read_to_string(project_root.join("vpm-manifest.json")).unwrap();

    let (config, bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-recover",
                    "production.recover",
                    "cmd-recover-rollback",
                    recover_params_v02("rollback", &confirm_task_id, &plan_id),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    if response_payload(&frames, "req-recover").is_null() {
        let frame = frames
            .iter()
            .find(|frame| {
                frame["kind"] == "response" && frame["payload"]["requestId"] == "req-recover"
            })
            .expect("recover response frame");
        panic!("recover error frame: {}", frame);
    }
    let recover = response_payload(&frames, "req-recover");
    let recover_task_id = recover["task"]["taskId"].as_str().unwrap().to_owned();

    let store = vua_orchestrator::SqliteTaskStore::open(base.join("provider.db")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store
            .task(&recover_task_id)
            .unwrap()
            .expect("task persists");
        if task.state.is_terminal() {
            break task;
        }
        assert!(
            Instant::now() < deadline,
            "rollback recovery never completed"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(task.state, TaskState::Succeeded, "{task:?}");
    let result = task.result.expect("result");
    assert_eq!(result["recovered"], "rollback");
    assert_eq!(result["restored"], true);

    // Rollback recovery runs NO Unity command and leaves the project as the
    // failed attempt's restore left it.
    // Rollback runs ONLY the read-only project inspection — never an import.
    assert_eq!(
        bridge.commands.lock().unwrap().len(),
        1,
        "rollback never imports"
    );
    assert_eq!(
        fs::read_to_string(project_root.join("vpm-manifest.json")).unwrap(),
        manifest_before
    );

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn ph_007a_recover_rollback_refuses_malformed_record_snapshot_id() {
    let (base, _source, project_root, plan_id, _plan_revision, confirm_task_id) =
        run_failing_confirm("recover-rollback-guard");

    // Tamper the failed attempt's receipt: a snapshot id that the
    // creation-time grammar gate never saw. The receipt is deserialized
    // payload data, and its snapshot id feeds the ownership probe, the
    // SnapshotRef path and the stale-quarantine rename — all before
    // restore_verified's containment checks run.
    let store = vua_orchestrator::SqliteTaskStore::open(base.join("provider.db")).unwrap();
    let original = store
        .task(&confirm_task_id)
        .unwrap()
        .expect("failed task persists");
    let record_id = original
        .result
        .as_ref()
        .expect("failed run carries a receipt")
        .get("buildRecordId")
        .and_then(Value::as_str)
        .expect("receipt names its record")
        .to_owned();
    let record_path = base.join("records").join(format!("{record_id}.json"));
    let mut record: Value =
        serde_json::from_str(&fs::read_to_string(&record_path).unwrap()).unwrap();
    // BuildSnapshotEvidenceV01 serializes camelCase — the field on the wire
    // is `snapshotId`.
    record["snapshot"]["snapshotId"] = json!("../evil-target");
    fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();

    // The traversal target EXISTS, so the lexical ownership probe would
    // pass and the quarantine rename would MOVE the marker directory —
    // this test pins the grammar gate, not an accidental is_dir refusal.
    let marker = project_root.join("evil-target");
    fs::create_dir_all(&marker).unwrap();

    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-recover",
                    "production.recover",
                    "cmd-recover-rollback-guard",
                    recover_params_v02("rollback", &confirm_task_id, &plan_id),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    // Run construction happens before task acceptance: a malformed receipt
    // id is a synchronous validation refusal, never a half-created task.
    let error = response_frame(&frames, "req-recover")["payload"]["error"].clone();
    assert_eq!(error["code"], "vua.production.not_recoverable", "{error}");

    // Nothing moved: the marker stays put and no superseded copy appeared.
    assert!(marker.is_dir(), "marker directory untouched");
    let mut relocated = fs::read_dir(&project_root).unwrap();
    assert!(
        !relocated.any(|entry| {
            entry.ok().is_some_and(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("evil-target.superseded-")
            })
        }),
        "no superseded relocation of the traversal target",
    );

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn ph_008_recover_requires_user_decision_and_recoverable_source() {
    let (base, _source, project_root, plan_id, _plan_revision, confirm_task_id) =
        run_failing_confirm("recover-guards");
    let (config, _bridge) = production_config(&base, &project_root);

    // Missing userDecisionId is refused.
    let output = {
        let mut output_buffer = Vec::new();
        let mut payload = recover_params_v02("continue", &confirm_task_id, &plan_id);
        payload["decisionId"] = json!("");
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-no-decision",
                    "production.recover",
                    "cmd-guard-1",
                    payload,
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let error = response_frame(&frames, "req-no-decision")["payload"]["error"].clone();
    assert_eq!(error["code"], "vua.production.decision_id_required");

    // An unknown original task is refused.
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-unknown-origin",
                    "production.recover",
                    "cmd-guard-2",
                    recover_params_v02("continue", "prod-nonexistent00", ""),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let error = response_frame(&frames, "req-unknown-origin")["payload"]["error"].clone();
    assert_eq!(error["code"], "vua.task.not_found");

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn ph_009_same_command_id_with_different_params_is_a_conflict() {
    let (base, _source, project_root, plan_id, plan_revision, _confirm_task_id) =
        run_failing_confirm("id-conflict");

    // The SAME commandId as the failed confirm, but a different risk
    // decision: a fingerprint conflict, never a silent replay.
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        let mut payload = confirm_params(&plan_id, plan_revision);
        payload["riskChoice"] = json!("snapshot_and_continue");
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-conflict",
                    "production.confirmPlan",
                    "cmd-confirm-fail",
                    payload,
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let error = response_frame(&frames, "req-conflict")["payload"]["error"].clone();
    assert_eq!(error["code"], "vua.command.id_conflict");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn ph_010_mutation_gate_holds_lock_and_marker_during_the_run() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use vua_project_manager::{
        acquire_project_lock, read_pending_mutation, LockHolder, PendingMutation,
    };

    struct BlockingBridge {
        released: Arc<AtomicBool>,
    }
    impl UnityBridge for BlockingBridge {
        fn execute(
            &self,
            _project: &vua_orchestrator::ProjectRef,
            command: &UnityCommand,
        ) -> Result<UnityResult, vua_orchestrator::BridgeError> {
            let deadline = Instant::now() + Duration::from_secs(10);
            while !self.released.load(Ordering::SeqCst) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            // 第 158 批（BOARD #45(2)）：mirror the real C# handler — a
            // Succeeded validate receipt always carries the loadedAssetPaths
            // evidence, and the executor now parses it strictly.
            let mut data = serde_json::json!({"projectFingerprint": "fp-blocked"});
            if command.operation == vua_orchestrator::UnityOperation::ValidateAssetPaths {
                let mut loaded = command.payload.expected_asset_paths.clone();
                loaded.sort();
                data["loadedAssetPaths"] =
                    loaded.into_iter().map(serde_json::Value::String).collect();
            }
            Ok(UnityResult {
                schema_version: 1,
                command_id: command.command_id.clone(),
                status: vua_orchestrator::ResultStatus::Succeeded,
                changed_paths: vec![],
                diagnostics: vec![],
                data,
                steps: Vec::new(),
                replayed: None,
                snapshot_id: None,
                restored_from: None,
                project_fingerprint_before: None,
            })
        }
    }

    let (base, source, project_root) = make_world("gate");
    let _ = &source;
    let released = Arc::new(AtomicBool::new(false));
    let executor = Arc::new(MaterialExecutor::new(
        Arc::new(BlockingBridge {
            released: Arc::clone(&released),
        }),
        FileSystemSnapshotStore,
        Arc::new(NoVpm),
        BuildRecordStore::new(base.join("records")),
        Arc::new(FixedClock::new(&["2026-09-05T00:00:00Z"])),
        base.join("temp"),
        "2022.3.22f1",
        LocalPackageIdentityStore::new(base.join("identities.json")),
    ));
    let config = ProductionConfig {
        executor,
        records: Arc::new(BuildRecordStore::new(base.join("records"))),
    };

    let (sender, receiver) = std::sync::mpsc::channel::<Vec<u8>>();
    let output_buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = SharedWriter(Arc::clone(&output_buffer));
    let path = base.join("provider.db");
    let host = std::thread::spawn(move || {
        let _ = vua_provider_host::run_provider_host_with(
            ChannelReader {
                receiver,
                buffer: Vec::new(),
                position: 0,
                done: false,
            },
            writer,
            path,
            Some(config),
        );
    });
    let send = |frame_value: Value| {
        let mut bytes = frame_value.to_string().into_bytes();
        bytes.push(b'\n');
        sender.send(bytes).unwrap();
    };

    send(frame(
        "f1",
        request(
            "req-inspect",
            "production.startInspection",
            "cmd-inspect",
            json!({
                "sourceFolder": source.to_string_lossy(),
                "projectRoot": project_root.to_string_lossy(),
                "artifactOutputRoot": base.join("artifacts").to_string_lossy(),
                "projectId": "project",
            }),
        ),
    ));
    let inspect = wait_for_response(&output_buffer, "req-inspect");
    let inspection_id = inspect["inspectionId"].as_str().unwrap().to_owned();
    send(frame(
        "f2",
        request(
            "req-plan",
            "production.requestPlan",
            "cmd-plan",
            plan_params(&inspection_id),
        ),
    ));
    let plan = wait_for_response(&output_buffer, "req-plan");
    let plan_id = plan["planId"].as_str().unwrap().to_owned();
    let plan_revision = plan["revision"].as_u64().unwrap();
    send(frame(
        "f3",
        request(
            "req-confirm",
            "production.confirmPlan",
            "cmd-confirm-gate",
            confirm_params(&plan_id, plan_revision),
        ),
    ));
    let confirm = wait_for_response(&output_buffer, "req-confirm");
    let confirm_task_id = confirm["task"]["taskId"].as_str().unwrap().to_owned();

    // While the worker sits inside the Bridge command, the mutation gate is
    // held: the pending-mutation marker exists, and a cross-profile lock
    // attempt from "another instance" is refused naming its holder.
    let deadline = Instant::now() + Duration::from_secs(10);
    let holder_seen = loop {
        if matches!(
            read_pending_mutation(&project_root),
            PendingMutation::Leftover(_)
        ) {
            let holder = LockHolder {
                channel: "other".into(),
                profile: "other".into(),
                pid: 1,
                instance_id: "other-instance".into(),
                acquired_at: "2026-09-05T00:00:00Z".into(),
            };
            if let Err(vua_project_manager::ProjectLockError::Held { previous }) =
                acquire_project_lock(&project_root, holder)
            {
                break previous.map(|holder| holder.instance_id);
            }
        }
        assert!(Instant::now() < deadline, "gate never observed");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        holder_seen.is_some(),
        "the cross-profile lock names its live holder"
    );

    // Release the run and let it finish.
    released.store(true, Ordering::SeqCst);
    send(frame("f4", request("req-pump", "task.list", "", json!({}))));
    drop(sender);
    host.join().unwrap();

    let store = vua_orchestrator::SqliteTaskStore::open(base.join("provider.db")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store
            .task(&confirm_task_id)
            .unwrap()
            .expect("task persists");
        if task.state.is_terminal() {
            break task;
        }
        assert!(Instant::now() < deadline, "worker never completed");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(task.state, TaskState::Succeeded);

    // The worker persists the authoritative terminal state BEFORE releasing
    // the mutation gate (provider_host keeps the lease visible until the run
    // is fully done, so a safe_to_stop verdict can never miss it). The
    // marker and the cross-profile lock therefore clear a few scheduler
    // ticks after the terminal state becomes visible — poll for the
    // release instead of racing it (BOARD #7: this used to be an immediate
    // assertion and flaked when the scheduler landed inside the window).
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if read_pending_mutation(&project_root) == PendingMutation::None {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "marker never cleared after the run"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    let guard = loop {
        let holder = LockHolder {
            channel: "other".into(),
            profile: "other".into(),
            pid: 1,
            instance_id: "other-instance".into(),
            acquired_at: "2026-09-05T00:00:00Z".into(),
        };
        match acquire_project_lock(&project_root, holder) {
            Ok(guard) => break guard,
            Err(vua_project_manager::ProjectLockError::Held { .. }) => {
                assert!(Instant::now() < deadline, "lock never released after run");
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("unexpected project lock error after the run: {error}"),
        }
    };
    guard.release().unwrap();
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn ph_011_completion_event_reaches_an_idle_host_without_a_request() {
    let (base, source, project_root) = make_world("idle-event");

    // Inspect + plan first.
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![
                frame(
                    "f1",
                    request(
                        "req-inspect",
                        "production.startInspection",
                        "cmd-inspect",
                        json!({
                            "sourceFolder": source.to_string_lossy(),
                            "projectRoot": project_root.to_string_lossy(),
                            "artifactOutputRoot": base.join("artifacts").to_string_lossy(),
                            "projectId": "project",
                        }),
                    ),
                ),
                frame("f2", request("req-pump", "task.list", "", json!({}))),
            ]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let inspection_id = response_payload(&frames, "req-inspect")["inspectionId"]
        .as_str()
        .unwrap()
        .to_owned();
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-plan",
                    "production.requestPlan",
                    "cmd-plan",
                    plan_params(&inspection_id),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let plan_id = response_payload(&frames, "req-plan")["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    let plan_revision = response_payload(&frames, "req-plan")["revision"]
        .as_u64()
        .unwrap();

    // Now confirm against the live host and then go COMPLETELY idle: the
    // completion event must arrive on its own through the timed pump.
    let (config, _bridge) = production_config(&base, &project_root);
    let (sender, receiver) = std::sync::mpsc::channel::<Vec<u8>>();
    let output_buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = SharedWriter(Arc::clone(&output_buffer));
    let path = base.join("provider.db");
    let host = std::thread::spawn(move || {
        let _ = vua_provider_host::run_provider_host_with(
            ChannelReader {
                receiver,
                buffer: Vec::new(),
                position: 0,
                done: false,
            },
            writer,
            path,
            Some(config),
        );
    });
    let send = |frame_value: Value| {
        let mut bytes = frame_value.to_string().into_bytes();
        bytes.push(b'\n');
        sender.send(bytes).unwrap();
    };
    send(frame(
        "f1",
        request(
            "req-confirm",
            "production.confirmPlan",
            "cmd-confirm-idle",
            confirm_params(&plan_id, plan_revision),
        ),
    ));
    let confirm = wait_for_response(&output_buffer, "req-confirm");
    let confirm_task_id = confirm["task"]["taskId"].as_str().unwrap().to_owned();

    // NO further frames: poll the wire for the terminal event.
    let deadline = Instant::now() + Duration::from_secs(10);
    let delivered = loop {
        let bytes = output_buffer.lock().unwrap().clone();
        let frames = parse_frames(&bytes);
        let hit = frames.iter().find(|frame| {
            frame["kind"] == "event"
                && frame["payload"]["taskId"] == confirm_task_id.as_str()
                && frame["payload"]["state"] == "succeeded"
        });
        if hit.is_some() {
            break true;
        }
        assert!(
            Instant::now() < deadline,
            "completion event never reached the idle host"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(delivered);

    drop(sender);
    host.join().unwrap();
    let _ = fs::remove_dir_all(&base);
}
#[test]
fn ph_012_crashed_lease_recovers_through_the_decision_path() {
    // Simulate a provider crash: a lease owned by a dead instance stays in
    // the store, and startup marks it recovery_required. A PLAIN confirm
    // must be refused (inspect-first); only a recovery decision may take
    // the lease over — after which the run succeeds.
    let (base, source, project_root) = make_world("crashed-lease");
    let path = base.join("provider.db");
    {
        let store = vua_orchestrator::SqliteTaskStore::open(&path).unwrap();
        let identity =
            vua_orchestrator::ProjectIdentity::from_existing_path(&project_root).unwrap();
        store
            .accept_task(&vua_orchestrator::NewTask {
                task_id: "prod-deadbeef".into(),
                correlation_id: "corr-dead".into(),
                occurred_at: "2026-09-05T00:00:00Z".into(),
            })
            .unwrap();
        store
            .acquire_project_lease(
                &identity,
                "provider-dead",
                "prod-deadbeef",
                "2026-09-05T00:00:00Z",
            )
            .unwrap();
        store
            .mark_other_owners_interrupted("someone", "2026-09-05T00:00:01Z")
            .unwrap();
    }

    // Inspect + plan (no gate involved).
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![
                frame(
                    "f1",
                    request(
                        "req-inspect",
                        "production.startInspection",
                        "cmd-inspect",
                        json!({
                            "sourceFolder": source.to_string_lossy(),
                            "projectRoot": project_root.to_string_lossy(),
                            "artifactOutputRoot": base.join("artifacts").to_string_lossy(),
                            "projectId": "project",
                        }),
                    ),
                ),
                frame("f2", request("req-pump", "task.list", "", json!({}))),
            ]),
            &mut output_buffer,
            &path,
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let inspection_id = response_payload(&frames, "req-inspect")["inspectionId"]
        .as_str()
        .unwrap()
        .to_owned();
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-plan",
                    "production.requestPlan",
                    "cmd-plan",
                    plan_params(&inspection_id),
                ),
            )]),
            &mut output_buffer,
            &path,
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let plan_id = response_payload(&frames, "req-plan")["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    let plan_revision = response_payload(&frames, "req-plan")["revision"]
        .as_u64()
        .unwrap();

    // A plain confirm hits the stale lease and is refused as
    // inspect_required — never a silent takeover, never a permanent lock.
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-confirm",
                    "production.confirmPlan",
                    "cmd-confirm-stale",
                    confirm_params(&plan_id, plan_revision),
                ),
            )]),
            &mut output_buffer,
            &path,
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let confirm = response_payload(&frames, "req-confirm");
    let confirm_task_id = confirm["task"]["taskId"].as_str().unwrap().to_owned();
    let store = vua_orchestrator::SqliteTaskStore::open(&path).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store.task(&confirm_task_id).unwrap().unwrap();
        if task.state.is_terminal() {
            break task;
        }
        assert!(Instant::now() < deadline, "refused confirm never finished");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(task.state, TaskState::Failed);
    assert_eq!(
        task.error.expect("typed error").code,
        "vua.project.inspect_required"
    );

    // The recovery decision carries the acknowledgment: the lease is taken
    // over after inspect and the run succeeds.
    let (config, bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-recover",
                    "production.recover",
                    "cmd-recover-takeover",
                    recover_params_v02("continue", &confirm_task_id, &plan_id),
                ),
            )]),
            &mut output_buffer,
            &path,
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    if response_payload(&frames, "req-recover").is_null() {
        let frame = frames
            .iter()
            .find(|frame| {
                frame["kind"] == "response" && frame["payload"]["requestId"] == "req-recover"
            })
            .expect("recover response frame");
        panic!("recover error frame: {}", frame);
    }
    let recover = response_payload(&frames, "req-recover");
    let recover_task_id = recover["task"]["taskId"].as_str().unwrap().to_owned();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store.task(&recover_task_id).unwrap().unwrap();
        if task.state.is_terminal() {
            break task;
        }
        assert!(Instant::now() < deadline, "recovery never completed");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(task.state, TaskState::Succeeded, "{task:?}");
    // inspect (the takeover credential) + import + validation
    assert_eq!(
        bridge.commands.lock().unwrap().len(),
        3,
        "continue re-runs after takeover"
    );

    // The lease now belongs to the live run and is released on completion.
    // The worker persists the terminal state BEFORE releasing the mutation
    // gate (safe_to_stop ordering in provider_host.rs), and the release
    // performs marker/lock file I/O before the SQLite delete — so a visible
    // terminal state does not yet imply a released lease. Poll for the
    // release instead of asserting it at first sight of the terminal state
    // (CI ph_012, 2026-09-07: under 2-core load this read won the race).
    let identity = vua_orchestrator::ProjectIdentity::from_existing_path(&project_root).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if store.project_lease(&identity).unwrap().is_none() {
            break;
        }
        assert!(Instant::now() < deadline, "lease released after success");
        std::thread::sleep(Duration::from_millis(20));
    }

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn ph_013_rollback_publishes_a_recovered_record_and_versions_the_quarantine() {
    let (base, _source, project_root, plan_id, _plan_revision, confirm_task_id) =
        run_failing_confirm("rollback-record");
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-recover",
                    "production.recover",
                    "cmd-recover-rollback",
                    recover_params_v02("rollback", &confirm_task_id, &plan_id),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    if response_payload(&frames, "req-recover").is_null() {
        let frame = frames
            .iter()
            .find(|frame| {
                frame["kind"] == "response" && frame["payload"]["requestId"] == "req-recover"
            })
            .expect("recover response frame");
        panic!("recover error frame: {}", frame);
    }
    let recover = response_payload(&frames, "req-recover");
    let recover_task_id = recover["task"]["taskId"].as_str().unwrap().to_owned();

    let store = vua_orchestrator::SqliteTaskStore::open(base.join("provider.db")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store.task(&recover_task_id).unwrap().unwrap();
        if task.state.is_terminal() {
            break task;
        }
        assert!(
            Instant::now() < deadline,
            "rollback recovery never completed"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(task.state, TaskState::Succeeded, "{task:?}");
    let result = task.result.expect("result");
    assert_eq!(result["restored"], true);

    // The recovery generated its own Build Record with status `recovered`.
    let recovered_record_id = result["buildRecordId"].as_str().expect("recorded recovery");
    let record = BuildRecordStore::new(base.join("records"))
        .read(recovered_record_id)
        .expect("recovery record exists");
    assert_eq!(
        record.status,
        vua_orchestrator::BuildRecordStatus::Recovered
    );

    // The superseded quarantine was VERSIONED aside, not deleted.
    let recovery_dir = project_root.join(".vua/recovery");
    let versions: Vec<_> = fs::read_dir(&recovery_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        versions.iter().any(|name| name.contains(".superseded-")),
        "quarantine is versioned aside: {versions:?}"
    );
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn ph_014_rollback_paths_come_from_the_binding_not_the_caller() {
    let (base, _source, project_root, plan_id, _plan_revision, confirm_task_id) =
        run_failing_confirm("rollback-binding");

    // v0.2: the caller cannot point a recovery at another project root —
    // paths live in the registry binding. A poison projectRoot field in the
    // payload is simply ignored; the rollback follows the bound path.
    let elsewhere = base.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        let mut payload = recover_params_v02("rollback", &confirm_task_id, &plan_id);
        payload["projectRoot"] = json!(elsewhere.to_string_lossy());
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-recover",
                    "production.recover",
                    "cmd-recover-x",
                    payload,
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let recover = response_payload(&frames, "req-recover");
    let recover_task_id = recover["task"]["taskId"].as_str().unwrap().to_owned();
    let store = vua_orchestrator::SqliteTaskStore::open(base.join("provider.db")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store
            .task(&recover_task_id)
            .unwrap()
            .expect("task persists");
        if task.state.is_terminal() {
            break task;
        }
        assert!(Instant::now() < deadline, "rollback never completed");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(
        task.state,
        TaskState::Succeeded,
        "rollback restores the BOUND project"
    );
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn ph_015_session_traffic_beyond_one_mib_still_processes_frames() {
    // The frame budget must be per frame: cumulative ~1 MiB of session
    // traffic used to be mistaken for EOF and silently ended the protocol.
    let (base, _source, _project_root) = make_world("budget");
    let pad = "x".repeat(400_000);
    let mut frames = Vec::new();
    for index in 0..4 {
        frames.push(frame(
            &format!("f{index}"),
            request(
                &format!("req-{index}"),
                "task.list",
                "",
                json!({ "pad": pad }),
            ),
        ));
    }
    let mut output_buffer = Vec::new();
    vua_provider_host::run_provider_host_with(
        frames_input(frames),
        &mut output_buffer,
        base.join("provider.db"),
        None,
    )
    .unwrap();
    let frames_out = parse_frames(&output_buffer);
    for index in 0..4 {
        let request_id = format!("req-{index}");
        assert!(
            frames_out.iter().any(|frame| {
                frame["kind"] == "response" && frame["payload"]["requestId"] == request_id.as_str()
            }),
            "response {request_id} missing — session died early"
        );
    }
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn ph_016_leftover_marker_is_superseded_only_by_a_recovery_decision() {
    let (base, source, project_root) = make_world("marker-supersede");

    // A crash on another profile left a pending-mutation marker behind.
    let holder = vua_project_manager::LockHolder {
        channel: "stable".into(),
        profile: "default".into(),
        pid: 424242,
        instance_id: "stable-instance".into(),
        acquired_at: "2026-09-05T00:00:00Z".into(),
    };
    let marker =
        vua_project_manager::begin_mutation(&project_root, "material_intake", &holder).unwrap();
    core::mem::forget(marker); // simulate the crash: the guard never releases

    // Inspect + plan.
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![
                frame(
                    "f1",
                    request(
                        "req-inspect",
                        "production.startInspection",
                        "cmd-inspect",
                        json!({
                            "sourceFolder": source.to_string_lossy(),
                            "projectRoot": project_root.to_string_lossy(),
                            "artifactOutputRoot": base.join("artifacts").to_string_lossy(),
                            "projectId": "project",
                        }),
                    ),
                ),
                frame("f2", request("req-pump", "task.list", "", json!({}))),
            ]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let inspection_id = response_payload(&frames, "req-inspect")["inspectionId"]
        .as_str()
        .unwrap()
        .to_owned();
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-plan",
                    "production.requestPlan",
                    "cmd-plan",
                    plan_params(&inspection_id),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let plan_id = response_payload(&frames, "req-plan")["planId"]
        .as_str()
        .unwrap()
        .to_owned();
    let plan_revision = response_payload(&frames, "req-plan")["revision"]
        .as_u64()
        .unwrap();

    // A plain confirm is refused: the leftover marker demands inspect-first.
    let (config, bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-confirm",
                    "production.confirmPlan",
                    "cmd-confirm-marker",
                    confirm_params(&plan_id, plan_revision),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    let confirm = response_payload(&frames, "req-confirm");
    let confirm_task_id = confirm["task"]["taskId"].as_str().unwrap().to_owned();
    let store = vua_orchestrator::SqliteTaskStore::open(base.join("provider.db")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store.task(&confirm_task_id).unwrap().unwrap();
        if task.state.is_terminal() {
            break task;
        }
        assert!(Instant::now() < deadline, "refused confirm never finished");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(task.state, TaskState::Failed);
    assert_eq!(
        task.error.expect("typed error").code,
        "vua.project.inspect_required"
    );
    assert_eq!(bridge.commands.lock().unwrap().len(), 0);

    // The recovery decision supersedes the marker (archived, not deleted)
    // and the run proceeds.
    let (config, _bridge) = production_config(&base, &project_root);
    let output = {
        let mut output_buffer = Vec::new();
        vua_provider_host::run_provider_host_with(
            frames_input(vec![frame(
                "f1",
                request(
                    "req-recover",
                    "production.recover",
                    "cmd-recover-marker",
                    recover_params_v02("continue", &confirm_task_id, &plan_id),
                ),
            )]),
            &mut output_buffer,
            base.join("provider.db"),
            Some(config),
        )
        .unwrap();
        output_buffer
    };
    let frames = parse_frames(&output);
    if response_payload(&frames, "req-recover").is_null() {
        let frame = frames
            .iter()
            .find(|frame| {
                frame["kind"] == "response" && frame["payload"]["requestId"] == "req-recover"
            })
            .expect("recover response frame");
        panic!("recover error frame: {}", frame);
    }
    let recover = response_payload(&frames, "req-recover");
    let recover_task_id = recover["task"]["taskId"].as_str().unwrap().to_owned();
    let deadline = Instant::now() + Duration::from_secs(15);
    let task = loop {
        let task = store.task(&recover_task_id).unwrap().unwrap();
        if task.state.is_terminal() {
            break task;
        }
        assert!(Instant::now() < deadline, "recovery never completed");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(task.state, TaskState::Succeeded, "{task:?}");

    // The old marker was archived; the live marker was cleaned up on
    // completion.
    let vua_dir = project_root.join(".vua");
    let archived: Vec<_> = fs::read_dir(&vua_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("pending-mutation.json.superseded-"))
        .collect();
    assert!(
        !archived.is_empty(),
        "superseded marker archived: {archived:?}"
    );
    assert_eq!(
        read_pending_mutation(&project_root),
        vua_project_manager::PendingMutation::None
    );
    let _ = fs::remove_dir_all(&base);
}
