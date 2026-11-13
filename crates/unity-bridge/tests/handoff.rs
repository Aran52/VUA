//! Process/window-face handoff port tests (proposal 023 production slice).
//!
//! Mechanism contracts pinned here: the launch argument face, the honest
//! no-trace semantics of probing, the handshake wait (arrival / timeout /
//! invalid-trail tolerance / per-project binding), the focus side action
//! never contaminating anything, and both ends of the handshake schema
//! vectors (this Rust reader + the C# writer tested in EditMode).

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use vua_unity_bridge::handoff::{
    handshake_path, DefaultEditorHandoff, EditorHandoffPort, EditorOpenState, FocusOutcome,
    HandoffError, WindowFocus, DEFAULT_HANDSHAKE_BUDGET, HANDSHAKE_SCHEMA_VERSION,
};
use vua_unity_bridge::handoff::{EditorSpawner, HandoffWait, ProcessLiveness};

const VALID_HANDSHAKE_JSON: &str = r#"{
    "schemaVersion": "1.0",
    "pid": 4212,
    "editorVersion": "2022.3.22f1",
    "occurredAt": "2026-09-16T03:30:00.000Z"
}"#;

struct FakeSpawner {
    calls: Mutex<Vec<(PathBuf, Vec<String>)>>,
    next_pid: AtomicU32,
    fail: std::sync::atomic::AtomicBool,
}

impl FakeSpawner {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            next_pid: AtomicU32::new(700),
            fail: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

impl EditorSpawner for FakeSpawner {
    fn spawn(&self, executable: &Path, args: &[String]) -> std::io::Result<u32> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no editor",
            ));
        }
        self.calls
            .lock()
            .expect("spawner calls")
            .push((executable.to_path_buf(), args.to_vec()));
        Ok(self.next_pid.fetch_add(1, Ordering::SeqCst))
    }
}

/// Virtual clock: sleep advances the reading, so budgets elapse without
/// real waiting and the poll loop runs deterministically.
struct ScriptedWait {
    now: AtomicU32,
}

impl HandoffWait for ScriptedWait {
    fn now_millis(&self) -> u64 {
        self.now.load(Ordering::SeqCst) as u64
    }

    fn sleep(&self, duration: Duration) {
        self.now
            .fetch_add(duration.as_millis() as u32, Ordering::SeqCst);
    }
}

struct FakeLiveness {
    dead: Mutex<std::collections::HashSet<u32>>,
}

impl FakeLiveness {
    fn all_alive() -> Self {
        Self {
            dead: Mutex::new(std::collections::HashSet::new()),
        }
    }

    fn kill(&self, pid: u32) {
        self.dead.lock().expect("dead set").insert(pid);
    }
}

impl ProcessLiveness for FakeLiveness {
    fn is_alive(&self, pid: u32) -> bool {
        !self.dead.lock().expect("dead set").contains(&pid)
    }
}

struct FakeFocus;

impl WindowFocus for FakeFocus {
    fn focus(&self, _pid: u32) -> FocusOutcome {
        FocusOutcome::Unavailable {
            reason: "test_disabled",
        }
    }
}

struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("vua-handoff-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".vua").join("bridge")).expect("workspace scaffold");
        Self { root }
    }

    fn write_handshake(&self, json: &str) {
        let path = handshake_path(&self.root);
        let mut file = std::fs::File::create(&path).expect("create handshake file");
        file.write_all(json.as_bytes())
            .expect("write handshake file");
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn port_with(
    spawner: std::sync::Arc<dyn EditorSpawner>,
    wait: std::sync::Arc<dyn HandoffWait>,
    liveness: std::sync::Arc<dyn ProcessLiveness>,
) -> DefaultEditorHandoff {
    DefaultEditorHandoff::with_parts(spawner, wait, liveness, std::sync::Arc::new(FakeFocus))
}

#[test]
fn launch_sends_windowed_project_path_launch_and_reports_pid() {
    let spawner = std::sync::Arc::new(FakeSpawner::new());
    let port = port_with(
        spawner.clone(),
        std::sync::Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        std::sync::Arc::new(FakeLiveness::all_alive()),
    );
    let project = Path::new("C:/work/AvatarProject");
    let launched = port
        .launch(Path::new("C:/Unity/2022.3.22f1/Editor/Unity.exe"), project)
        .expect("launch");

    let calls = spawner.calls.lock().expect("spawner calls");
    assert_eq!(calls.len(), 1, "exactly one editor process launched");
    let (executable, args) = &calls[0];
    assert_eq!(
        executable,
        Path::new("C:/Unity/2022.3.22f1/Editor/Unity.exe"),
        "the verified editor identity's executable is used verbatim"
    );
    assert_eq!(
        args,
        &vec![
            "-projectPath".to_string(),
            "C:/work/AvatarProject".to_string(),
        ],
        "windowed -projectPath launch: no -batchmode/-quit, no bridge command face"
    );
    assert_eq!(launched.pid, 700, "the launched process id is reported");
}

#[test]
fn launch_failure_is_typed_and_carries_the_executable() {
    let spawner = std::sync::Arc::new(FakeSpawner::new());
    spawner.fail.store(true, Ordering::SeqCst);
    let port = port_with(
        spawner,
        std::sync::Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        std::sync::Arc::new(FakeLiveness::all_alive()),
    );
    let error = port
        .launch(
            Path::new("C:/Unity/Unity.exe"),
            Path::new("C:/work/Project"),
        )
        .expect_err("spawn failure surfaces");
    match error {
        HandoffError::LaunchFailed { exe, .. } => {
            assert_eq!(exe, PathBuf::from("C:/Unity/Unity.exe"));
        }
        other => panic!("expected LaunchFailed, got {other:?}"),
    }
}

#[test]
fn probe_reports_closed_without_a_trail() {
    let workspace = Workspace::new("probe-empty");
    let port = port_with(
        std::sync::Arc::new(FakeSpawner::new()),
        std::sync::Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        std::sync::Arc::new(FakeLiveness::all_alive()),
    );
    assert_eq!(
        port.probe(&workspace.root).expect("probe"),
        EditorOpenState::Closed,
        "no handshake file means honestly not-open"
    );
}

#[test]
fn probe_reports_open_for_a_live_trail() {
    let workspace = Workspace::new("probe-open");
    workspace.write_handshake(VALID_HANDSHAKE_JSON);
    let port = port_with(
        std::sync::Arc::new(FakeSpawner::new()),
        std::sync::Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        std::sync::Arc::new(FakeLiveness::all_alive()),
    );
    let state = port.probe(&workspace.root).expect("probe");
    match state {
        EditorOpenState::Open(handshake) => {
            assert_eq!(handshake.pid, 4212);
            assert_eq!(handshake.editor_version, "2022.3.22f1");
            assert_eq!(handshake.schema_version, HANDSHAKE_SCHEMA_VERSION);
        }
        other => panic!("expected Open, got {other:?}"),
    }
}

#[test]
fn probe_treats_a_dead_pid_as_not_open() {
    let workspace = Workspace::new("probe-dead");
    workspace.write_handshake(VALID_HANDSHAKE_JSON);
    let liveness = FakeLiveness::all_alive();
    liveness.kill(4212);
    let port = port_with(
        std::sync::Arc::new(FakeSpawner::new()),
        std::sync::Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        std::sync::Arc::new(liveness),
    );
    assert_eq!(
        port.probe(&workspace.root).expect("probe"),
        EditorOpenState::Closed,
        "a stale trail (dead pid) is honestly not-open; the launch path is safe"
    );
}

#[test]
fn probe_treats_corrupt_or_unknown_version_trails_as_not_open() {
    let workspace = Workspace::new("probe-corrupt");
    workspace.write_handshake("{not json");
    let port = port_with(
        std::sync::Arc::new(FakeSpawner::new()),
        std::sync::Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        std::sync::Arc::new(FakeLiveness::all_alive()),
    );
    assert_eq!(
        port.probe(&workspace.root).expect("probe"),
        EditorOpenState::Closed
    );

    let workspace_v2 = Workspace::new("probe-v2");
    workspace_v2.write_handshake(&VALID_HANDSHAKE_JSON.replace("\"1.0\"", "\"99.0\""));
    assert_eq!(
        port.probe(&workspace_v2.root).expect("probe"),
        EditorOpenState::Closed,
        "unknown handshake schema version is not a valid trail"
    );
}

#[test]
fn await_handshake_returns_on_arrival() {
    let workspace = Workspace::new("await-arrive");
    workspace.write_handshake(VALID_HANDSHAKE_JSON);
    let port = port_with(
        std::sync::Arc::new(FakeSpawner::new()),
        std::sync::Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        std::sync::Arc::new(FakeLiveness::all_alive()),
    );
    let handshake = port
        .await_handshake(&workspace.root, Duration::from_secs(30))
        .expect("handshake arrives");
    assert_eq!(
        handshake.pid, 4212,
        "the arrival carries the editor process facts"
    );
}

#[test]
fn await_handshake_times_out_honestly_without_guessing() {
    let workspace = Workspace::new("await-timeout");
    let port = port_with(
        std::sync::Arc::new(FakeSpawner::new()),
        std::sync::Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        std::sync::Arc::new(FakeLiveness::all_alive()),
    );
    let budget = Duration::from_millis(1500);
    let error = port
        .await_handshake(&workspace.root, budget)
        .expect_err("timeout surfaces");
    match error {
        HandoffError::HandshakeTimeout { budget: reported } => {
            assert_eq!(reported, budget, "the honest timeout names its budget");
        }
        other => panic!("expected HandshakeTimeout, got {other:?}"),
    }
}

#[test]
fn await_handshake_waits_through_invalid_trails_until_valid() {
    let workspace = Workspace::new("await-invalid");
    workspace.write_handshake("{still loading");
    // The editor writes the trail while we poll: the first sleep repairs the
    // file, the next poll reads a valid trail and the wait completes.
    struct RepairOnSleep {
        trail: PathBuf,
        sleeps: AtomicU32,
    }
    impl HandoffWait for RepairOnSleep {
        fn now_millis(&self) -> u64 {
            0
        }
        fn sleep(&self, _duration: Duration) {
            if self.sleeps.fetch_add(1, Ordering::SeqCst) == 0 {
                std::fs::write(&self.trail, VALID_HANDSHAKE_JSON).expect("repair trail");
            }
        }
    }
    let port = port_with(
        std::sync::Arc::new(FakeSpawner::new()),
        std::sync::Arc::new(RepairOnSleep {
            trail: handshake_path(&workspace.root),
            sleeps: AtomicU32::new(0),
        }),
        std::sync::Arc::new(FakeLiveness::all_alive()),
    );
    let handshake = port
        .await_handshake(&workspace.root, DEFAULT_HANDSHAKE_BUDGET)
        .expect("invalid trail is waited through, not failed on");
    assert_eq!(handshake.pid, 4212);
}

#[test]
fn await_handshake_is_bound_to_its_own_project_directory() {
    let workspace_a = Workspace::new("bind-a");
    let workspace_b = Workspace::new("bind-b");
    workspace_b.write_handshake(VALID_HANDSHAKE_JSON);
    let port = port_with(
        std::sync::Arc::new(FakeSpawner::new()),
        std::sync::Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        std::sync::Arc::new(FakeLiveness::all_alive()),
    );
    let error = port
        .await_handshake(&workspace_a.root, Duration::from_millis(1000))
        .expect_err("another project's trail never completes this project's wait");
    assert!(matches!(error, HandoffError::HandshakeTimeout { .. }));
}

#[test]
fn focus_side_action_stays_independent_and_reportable() {
    let port = port_with(
        std::sync::Arc::new(FakeSpawner::new()),
        std::sync::Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        std::sync::Arc::new(FakeLiveness::all_alive()),
    );
    assert_eq!(
        port.focus(4212),
        FocusOutcome::Unavailable {
            reason: "test_disabled"
        },
        "focus is a best-effort side action; unavailability is reportable, never fatal"
    );
}

#[test]
fn handshake_schema_vectors_hold_for_this_reader() {
    // Rust-side consumption of the versioned handshake face: the positive
    // vector validates and the negative vectors are rejected. The C# writer
    // side is consumed by the EditMode tests in the bridge package.
    let schema_value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../schemas/unity-bridge/handshake/v1.0/handshake.schema.json"
        ))
        .expect("handshake schema present"),
    )
    .expect("handshake schema is valid json");
    let schema = jsonschema::validator_for(&schema_value).expect("handshake schema compiles");

    let valid: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../schemas/unity-bridge/handshake/v1.0/examples/handshake.valid.json"
        ))
        .expect("positive vector present"),
    )
    .expect("positive vector is json");
    assert!(schema.is_valid(&valid), "positive vector validates");

    for name in [
        "handshake.missing-pid.json",
        "handshake.unknown-field.json",
        "handshake.bad-schema-version.json",
    ] {
        let negative: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(format!(
                "{}/../../schemas/unity-bridge/handshake/v1.0/examples/{name}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .expect("negative vector present"),
        )
        .expect("negative vector is json");
        assert!(
            !schema.is_valid(&negative),
            "negative vector {name} is rejected"
        );
    }

    // The writer face the reader accepts matches the schema: serde can read
    // exactly the positive shape (camelCase on the wire).
    let parsed: vua_unity_bridge::handoff::EditorHandshake =
        serde_json::from_str(VALID_HANDSHAKE_JSON).expect("reader accepts the vector shape");
    assert_eq!(parsed.schema_version, "1.0");
}
