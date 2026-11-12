//! Assembly-adapter tests (proposal 023 assembly slice, BOARD #30):
//! `EditorHandoffAdapter` translates the production-domain mechanism port
//! (`EditorHandoffPort`, unity-bridge handoff module) into the frozen
//! core use-case port (`ReleaseHandoffPort`). These tests pin the
//! translation with injected mechanism parts (ruling-15 local-first; the
//! real OS mechanism is exercised by the production-domain slice's own
//! tests and the real-machine evidence belongs to W25):
//!
//! - closed project → windowed launch → handshake arrival → `HandshakeArrived`
//!   with a best-effort focus on the trail's pid;
//! - already-open project (valid live trail) → arrival WITHOUT launching a
//!   second editor (the trail IS the completion fact, ruling 3);
//! - launch failure → the `Err` class carrying the real cause (never a
//!   guessed outcome);
//! - budget exhausted without a handshake → the honest `HandshakeTimeout`
//!   outcome (never a guessed success);
//! - probe I/O failure → reported, never folded into the launch path;
//! - a dead trail (stale pid) honestly routes to the launch path.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use vua_orchestrator::{HandoffLaunch, HandoffOutcome, ReleaseHandoffPort};
use vua_provider_host::EditorHandoffAdapter;
use vua_unity_bridge::handoff::{
    handshake_path, DefaultEditorHandoff, EditorSpawner, FocusOutcome, HandoffWait,
    ProcessLiveness, WindowFocus, HANDSHAKE_SCHEMA_VERSION,
};

const VALID_HANDSHAKE_JSON: &str = r#"{
    "schemaVersion": "1.0",
    "pid": 4212,
    "editorVersion": "2022.3.22f1",
    "occurredAt": "2026-09-16T05:40:00.000Z"
}"#;

struct FakeSpawner {
    calls: Mutex<Vec<(PathBuf, Vec<String>)>>,
    next_pid: AtomicU32,
    fail: std::sync::atomic::AtomicBool,
    /// Deterministic "editor finished loading" compression: when set, the
    /// spawner writes this trail at spawn time (the real editor writes it
    /// on project load — the wait mechanic itself is pinned by the
    /// production-domain slice's tests, so the adapter tests keep the
    /// sequence deterministic).
    trail_on_spawn: Mutex<Option<PathBuf>>,
}

impl FakeSpawner {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            next_pid: AtomicU32::new(700),
            fail: std::sync::atomic::AtomicBool::new(false),
            trail_on_spawn: Mutex::new(None),
        }
    }
    fn writes_trail_at_spawn(&self, project_root: PathBuf) {
        *self.trail_on_spawn.lock().expect("trail config") = Some(project_root);
    }
    fn launch_count(&self) -> usize {
        self.calls.lock().expect("spawner calls").len()
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
        if let Some(root) = self.trail_on_spawn.lock().expect("trail config").clone() {
            let mut file =
                std::fs::File::create(handshake_path(&root)).expect("create handshake file");
            file.write_all(
                format!(
                    r#"{{"schemaVersion":"{HANDSHAKE_SCHEMA_VERSION}","pid":700,"editorVersion":"2022.3.22f1","occurredAt":"2026-09-16T05:41:00.000Z"}}"#
                )
                .as_bytes(),
            )
            .expect("write handshake");
        }
        Ok(self.next_pid.fetch_add(1, Ordering::SeqCst))
    }
}

/// Virtual clock: each poll sleep advances the reading, so the production
/// default budget elapses without real waiting.
struct ScriptedWait {
    now: AtomicU32,
}

impl HandoffWait for ScriptedWait {
    fn now_millis(&self) -> u64 {
        self.now.load(Ordering::SeqCst) as u64
    }
    fn sleep(&self, duration: std::time::Duration) {
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

/// Records which pid the best-effort focus side action targeted; the
/// outcome itself is always "unavailable" to prove focus never gates the
/// completion judgment.
struct RecordingFocus {
    focused: Mutex<Vec<u32>>,
}

impl WindowFocus for RecordingFocus {
    fn focus(&self, pid: u32) -> FocusOutcome {
        self.focused.lock().expect("focus log").push(pid);
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
        let root = std::env::temp_dir().join(format!(
            "vua-handoff-adapter-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
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

fn launch_for(root: &Path) -> HandoffLaunch {
    HandoffLaunch {
        build_id: "019513e7-7a2b-7cd1-9f3a-4d8e21b90c99".to_owned(),
        project_id: "proj-synthetic-avatar-a".to_owned(),
        project_root: root.to_path_buf(),
        editor_exe: PathBuf::from("C:/Unity/2022.3.22f1/Editor/Unity.exe"),
        editor_version: "2022.3.22f1".to_owned(),
    }
}

fn adapter_with(
    spawner: Arc<FakeSpawner>,
    wait: Arc<ScriptedWait>,
    liveness: Arc<FakeLiveness>,
    focus: Arc<RecordingFocus>,
) -> EditorHandoffAdapter {
    EditorHandoffAdapter::with_editor(Arc::new(DefaultEditorHandoff::with_parts(
        spawner, wait, liveness, focus,
    )))
}

#[test]
fn closed_project_launches_then_arrives_and_focuses_the_trail_pid() {
    let workspace = Workspace::new("closed-arrives");
    let spawner = Arc::new(FakeSpawner::new());
    // The "launched editor" finishes loading and writes its trail (pid 700).
    spawner.writes_trail_at_spawn(workspace.root.clone());
    let focus = Arc::new(RecordingFocus {
        focused: Mutex::new(Vec::new()),
    });
    let adapter = adapter_with(
        spawner.clone(),
        Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        Arc::new(FakeLiveness::all_alive()),
        focus.clone(),
    );

    let outcome = adapter
        .open_for_handoff(&launch_for(&workspace.root))
        .expect("the handoff attempt itself succeeds");

    assert_eq!(outcome, HandoffOutcome::HandshakeArrived);
    assert_eq!(spawner.launch_count(), 1, "exactly one editor launched");
    let calls = spawner.calls.lock().expect("spawner calls");
    assert_eq!(
        calls[0].0,
        PathBuf::from("C:/Unity/2022.3.22f1/Editor/Unity.exe"),
        "the resolved editor identity's executable is used verbatim"
    );
    assert_eq!(
        calls[0].1,
        vec![
            "-projectPath".to_string(),
            workspace.root.to_string_lossy().into_owned()
        ],
        "windowed -projectPath launch"
    );
    assert_eq!(
        *focus.focused.lock().expect("focus log"),
        vec![700],
        "focus targets the handshake trail's pid (best-effort side action)"
    );
}

#[test]
fn already_open_focuses_and_arrives_without_launching() {
    let workspace = Workspace::new("already-open");
    workspace.write_handshake(VALID_HANDSHAKE_JSON);
    let spawner = Arc::new(FakeSpawner::new());
    let focus = Arc::new(RecordingFocus {
        focused: Mutex::new(Vec::new()),
    });
    let adapter = adapter_with(
        spawner.clone(),
        Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        Arc::new(FakeLiveness::all_alive()),
        focus.clone(),
    );

    let outcome = adapter
        .open_for_handoff(&launch_for(&workspace.root))
        .expect("the handoff attempt itself succeeds");

    // The valid, live trail IS the completion fact (ruling 3): no second
    // editor is launched over a live session; focus is a courtesy.
    assert_eq!(outcome, HandoffOutcome::HandshakeArrived);
    assert_eq!(
        spawner.launch_count(),
        0,
        "a live session is never re-launched"
    );
    assert_eq!(*focus.focused.lock().expect("focus log"), vec![4212]);
}

#[test]
fn launch_failure_is_the_err_class_with_the_real_cause() {
    let workspace = Workspace::new("launch-fails");
    let spawner = Arc::new(FakeSpawner::new());
    spawner.fail.store(true, Ordering::SeqCst);
    let adapter = adapter_with(
        spawner.clone(),
        Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        Arc::new(FakeLiveness::all_alive()),
        Arc::new(RecordingFocus {
            focused: Mutex::new(Vec::new()),
        }),
    );

    let error = adapter
        .open_for_handoff(&launch_for(&workspace.root))
        .expect_err("a launch that could not be attempted is the Err class");
    assert!(
        error.detail.contains("no editor"),
        "the real cause travels as detail: {}",
        error.detail
    );
}

#[test]
fn budget_exhausted_without_a_handshake_is_the_honest_timeout() {
    let workspace = Workspace::new("honest-timeout");
    let adapter = adapter_with(
        Arc::new(FakeSpawner::new()),
        Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        Arc::new(FakeLiveness::all_alive()),
        Arc::new(RecordingFocus {
            focused: Mutex::new(Vec::new()),
        }),
    );

    // No trail is ever written: the wait runs to the production default
    // budget (virtual clock) and reports the honest timeout — never a
    // guessed success.
    let outcome = adapter
        .open_for_handoff(&launch_for(&workspace.root))
        .expect("the wait itself happened");
    assert_eq!(outcome, HandoffOutcome::HandshakeTimeout);
}

#[test]
fn probe_io_failure_is_reported_never_folded_into_the_launch_path() {
    let workspace = Workspace::new("probe-io-error");
    // The trail location is a DIRECTORY: reading it fails with a non-NotFound
    // I/O error, which the mechanism reports instead of interpreting.
    std::fs::create_dir_all(handshake_path(&workspace.root)).expect("blocker dir creates");
    let spawner = Arc::new(FakeSpawner::new());
    let adapter = adapter_with(
        spawner.clone(),
        Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        Arc::new(FakeLiveness::all_alive()),
        Arc::new(RecordingFocus {
            focused: Mutex::new(Vec::new()),
        }),
    );

    let error = adapter
        .open_for_handoff(&launch_for(&workspace.root))
        .expect_err("a probe that cannot read the trail is reported, not guessed");
    assert!(!error.detail.is_empty(), "the real cause travels as detail");
    assert_eq!(
        spawner.launch_count(),
        0,
        "an unreadable trail never triggers the launch path"
    );
}

#[test]
fn dead_trail_routes_to_the_launch_path_and_arrives_on_the_new_trail() {
    let workspace = Workspace::new("dead-trail");
    // A stale trail from a dead previous session (pid 4212): honestly
    // "not open" despite the file being present.
    workspace.write_handshake(VALID_HANDSHAKE_JSON);
    let liveness = Arc::new(FakeLiveness::all_alive());
    liveness.kill(4212);

    let spawner = Arc::new(FakeSpawner::new());
    // The "launched editor" writes its own trail (pid 700, alive) on load.
    spawner.writes_trail_at_spawn(workspace.root.clone());
    let focus = Arc::new(RecordingFocus {
        focused: Mutex::new(Vec::new()),
    });
    let adapter = adapter_with(
        spawner.clone(),
        Arc::new(ScriptedWait {
            now: AtomicU32::new(0),
        }),
        liveness.clone(),
        focus.clone(),
    );

    let outcome = adapter
        .open_for_handoff(&launch_for(&workspace.root))
        .expect("the handoff attempt itself succeeds");

    assert_eq!(outcome, HandoffOutcome::HandshakeArrived);
    assert_eq!(
        spawner.launch_count(),
        1,
        "a dead trail means honestly closed: launch"
    );
    assert_eq!(
        *focus.focused.lock().expect("focus log"),
        vec![700],
        "focus follows the NEW trail's pid"
    );
}
