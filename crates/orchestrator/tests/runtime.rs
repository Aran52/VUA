//! Task runtime integration tests (E-S0). Each test cites its ORC requirement
//! (ORC-TST-006).

// Matches the crate-level exception: jobs return the fat AppErrorV1 envelope.
#![allow(clippy::result_large_err)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use vua_orchestrator::{
    recover_from_journal, AppErrorV1, ErrorCategory, FixedClock, FixedIdGenerator,
    JournalEntryKind, JournalPayload, JournalSink, JournalWriter, MemoryJournal,
    RecoveredDisposition, SubmitRequest, SystemClock, TaskEventKind, TaskExit, TaskLane, TaskRuntime,
    TaskState,
};

fn journal_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-runtime-{label}-{nanos}.jsonl"))
}

fn file_runtime(label: &str) -> (TaskRuntime, PathBuf) {
    let path = journal_path(label);
    let writer = Arc::new(JournalWriter::open(&path).unwrap());
    (
        TaskRuntime::new(
            writer,
            Arc::new(SystemClock),
            Arc::new(FixedIdGenerator::default()),
        ),
        path,
    )
}

fn wait_for_terminal(runtime: &TaskRuntime, task_id: &str) -> vua_orchestrator::TaskSnapshot {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let snapshot = runtime.snapshot(task_id).expect("task must exist");
        if snapshot.state.is_terminal() {
            return snapshot;
        }
        assert!(Instant::now() < deadline, "task did not finish in time");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn wait_for_state(runtime: &TaskRuntime, task_id: &str, state: TaskState) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let snapshot = runtime.snapshot(task_id).expect("task must exist");
        if snapshot.state == state {
            return;
        }
        assert!(
            Instant::now() < deadline && !snapshot.state.is_terminal(),
            "task never reached {state:?} (now {:?})",
            snapshot.state
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

struct FailingJournal;

impl JournalSink for FailingJournal {
    fn append(
        &self,
        _entry: &vua_orchestrator::JournalEntryV1,
    ) -> Result<(), vua_orchestrator::JournalError> {
        Err(vua_orchestrator::JournalError::Io(std::io::Error::other(
            "simulated disk failure",
        )))
    }

    fn path(&self) -> Option<&std::path::Path> {
        None
    }
}

#[derive(Default)]
struct FailCompletedJournal {
    entries: Mutex<Vec<vua_orchestrator::JournalEntryV1>>,
    completed_attempted: AtomicBool,
}

impl JournalSink for FailCompletedJournal {
    fn append(
        &self,
        entry: &vua_orchestrator::JournalEntryV1,
    ) -> Result<(), vua_orchestrator::JournalError> {
        if entry.kind == JournalEntryKind::Completed {
            self.completed_attempted.store(true, Ordering::SeqCst);
            return Err(vua_orchestrator::JournalError::Io(std::io::Error::other(
                "simulated terminal fsync failure",
            )));
        }
        self.entries.lock().unwrap().push(entry.clone());
        Ok(())
    }

    fn path(&self) -> Option<&std::path::Path> {
        None
    }
}

#[test]
fn orc_sta_006_acceptance_is_journaled_before_the_caller_learns() {
    let (runtime, path) = file_runtime("accept-first");
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: Some("corr-accept-first".into()),
            timeout: None,
            job: Box::new(|_context| Ok(TaskExit::Done(serde_json::json!({"ok": true})))),
        })
        .unwrap();

    // The journal already contains the acceptance entry for the returned id,
    // even though the job may already have started advancing (the guarantee
    // is ordering of acceptance, not that the job has not yet run).
    let report = recover_from_journal(&path);
    assert!(
        report
            .tasks
            .iter()
            .any(|task| task.task_id == accepted.task_id),
        "acceptance must be journaled before submit() returns"
    );
    wait_for_terminal(&runtime, &accepted.task_id);
    std::fs::remove_file(&path).ok();
}

#[test]
fn orc_sta_006_journal_write_failure_rejects_the_command() {
    let runtime = TaskRuntime::new(
        Arc::new(FailingJournal),
        Arc::new(SystemClock),
        Arc::new(FixedIdGenerator::default()),
    );
    let error = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(|_context| Ok(TaskExit::Done(serde_json::Value::Null))),
        })
        .expect_err("a journal failure must reject the command");
    assert_eq!(error.code, "vua.task.journal_write_failed");
    assert_eq!(error.category, ErrorCategory::Internal);
    assert!(error.recoverable);
    assert!(
        runtime.snapshot_all().is_empty(),
        "a rejected command must not register a task"
    );
    let recovery_error = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(|_| Ok(TaskExit::Done(serde_json::Value::Null))),
        })
        .expect_err("a failed append must quarantine the journal globally");
    assert_eq!(recovery_error.code, "vua.task.journal_recovery_required");
}

#[test]
fn orc_sto_005_corrupt_journal_refuses_new_acceptance() {
    let path = journal_path("corrupt-refuses");
    std::fs::write(&path, b"{broken\n").unwrap();
    let runtime = TaskRuntime::new(
        Arc::new(JournalWriter::open(&path).unwrap()),
        Arc::new(SystemClock),
        Arc::new(FixedIdGenerator::default()),
    );
    let error = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(|_| Ok(TaskExit::Done(serde_json::Value::Null))),
        })
        .expect_err("manual journal recovery must precede new acceptance");
    assert_eq!(error.code, "vua.task.journal_recovery_required");
    assert!(runtime.snapshot_all().is_empty());
    std::fs::remove_file(path).ok();
}

#[test]
fn orc_sta_006_concurrent_tasks_append_globally_ordered_sequences() {
    let journal = Arc::new(MemoryJournal::new());
    let runtime = Arc::new(TaskRuntime::new(
        journal.clone(),
        Arc::new(SystemClock),
        Arc::new(vua_orchestrator::NanosTaskIdGenerator::default()),
    ));
    let mut submitters = Vec::new();
    for _ in 0..16 {
        let runtime = runtime.clone();
        submitters.push(std::thread::spawn(move || {
            runtime
                .submit(SubmitRequest {
                    correlation_id: None,
                    timeout: None,
                    job: Box::new(|context| {
                        context.emit_progress(serde_json::json!({"step": 1}));
                        Ok(TaskExit::Done(serde_json::Value::Null))
                    }),
                })
                .unwrap()
        }));
    }
    let accepted: Vec<_> = submitters
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    for task in accepted {
        wait_for_terminal(&runtime, &task.task_id);
    }
    let entries = journal.snapshot();
    assert!(entries.windows(2).all(|pair| pair[0].seq < pair[1].seq));
}

#[test]
fn orc_sto_002_terminal_journal_failure_never_publishes_success() {
    let journal = Arc::new(FailCompletedJournal::default());
    let runtime = TaskRuntime::new(
        journal.clone(),
        Arc::new(SystemClock),
        Arc::new(FixedIdGenerator::default()),
    );
    let receiver = runtime.subscribe();
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(|_context| Ok(TaskExit::Done(serde_json::json!({"durable": false})))),
        })
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(10);
    while !journal.completed_attempted.load(Ordering::SeqCst) {
        assert!(
            Instant::now() < deadline,
            "terminal write was not attempted"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    let snapshot = runtime.snapshot(&accepted.task_id).unwrap();
    assert_eq!(
        snapshot.state,
        TaskState::Running,
        "an undurable terminal result must not become authoritative in memory"
    );
    let events: Vec<_> = receiver.try_iter().collect();
    assert!(
        events
            .iter()
            .all(|event| event.kind != TaskEventKind::Completed),
        "the UI must never observe an undurable success event"
    );
    assert!(journal
        .entries
        .lock()
        .unwrap()
        .iter()
        .all(|entry| entry.kind != JournalEntryKind::Completed));
}

#[test]
fn orc_sta_003_events_carry_strictly_monotonic_revisions_and_states() {
    let (runtime, path) = file_runtime("monotonic");
    let receiver: mpsc::Receiver<vua_orchestrator::TaskEventV1> = runtime.subscribe();
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(|context| {
                context.emit_progress(serde_json::json!({"step": "half"}));
                Ok(TaskExit::Done(serde_json::json!({"ok": true})))
            }),
        })
        .unwrap();

    let mut events = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while events
        .last()
        .map(|event: &vua_orchestrator::TaskEventV1| event.kind != TaskEventKind::Completed)
        .unwrap_or(true)
    {
        let event = receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("events must arrive");
        assert_eq!(event.task_id, accepted.task_id);
        events.push(event);
    }

    let kinds: Vec<TaskEventKind> = events.iter().map(|event| event.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TaskEventKind::Accepted,
            TaskEventKind::StateChanged,
            TaskEventKind::StateChanged,
            TaskEventKind::Progress,
            TaskEventKind::Completed,
        ]
    );
    let states: Vec<TaskState> = events.iter().map(|event| event.state).collect();
    assert_eq!(
        states,
        vec![
            TaskState::Queued,
            TaskState::Preparing,
            TaskState::Running,
            TaskState::Running,
            TaskState::Succeeded,
        ]
    );
    for pair in events.windows(2) {
        assert!(
            pair[1].revision > pair[0].revision,
            "revisions must be strictly monotonic: {} then {}",
            pair[0].revision,
            pair[1].revision
        );
    }
    assert_eq!(events.last().unwrap().payload["ok"], true);
    wait_for_terminal(&runtime, &accepted.task_id);
    std::fs::remove_file(&path).ok();
}

#[test]
fn orc_con_006_cancel_request_is_distinguishable_from_cancellation() {
    let (runtime, path) = file_runtime("cancel");
    let receiver = runtime.subscribe();
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(move |context| {
                while !context.check_cancel() {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Ok(TaskExit::Cancelled)
            }),
        })
        .unwrap();

    // Cancellation is requested while the job is running.
    wait_for_state(&runtime, &accepted.task_id, TaskState::Running);
    runtime.cancel(&accepted.task_id).unwrap();
    let snapshot = wait_for_terminal(&runtime, &accepted.task_id);
    assert_eq!(snapshot.state, TaskState::Cancelled);
    assert!(snapshot.cancel_requested, "the request flag stays visible");

    // The event stream shows request and completion separately.
    let mut saw_request = false;
    let mut saw_completed_cancelled = false;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(saw_request && saw_completed_cancelled) {
        let event = receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("events must arrive");
        match event.kind {
            TaskEventKind::CancelRequested => {
                saw_request = true;
                assert_eq!(event.state, TaskState::Running);
            }
            TaskEventKind::Completed => {
                saw_completed_cancelled = true;
                assert_eq!(event.state, TaskState::Cancelled);
            }
            _ => {}
        }
    }

    // The journal records the request and the terminal outcome.
    let report = recover_from_journal(&path);
    let task = report
        .tasks
        .iter()
        .find(|task| task.task_id == accepted.task_id)
        .unwrap();
    assert!(task.cancel_requested);
    assert_eq!(task.disposition, RecoveredDisposition::Terminal);

    // Idempotent cancel on a terminal task is a no-op that succeeds.
    runtime.cancel(&accepted.task_id).unwrap();
    std::fs::remove_file(&path).ok();
}

#[test]
fn orc_wf_010_failed_job_is_recorded_and_recovered_as_terminal_with_error() {
    let (runtime, path) = file_runtime("failed");
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: Some("corr-failed".into()),
            timeout: None,
            job: Box::new(|_context| {
                Err(AppErrorV1::new(
                    "vua.test.deliberate",
                    ErrorCategory::ExternalFailure,
                    "errors.test.deliberate",
                    "corr-failed",
                ))
            }),
        })
        .unwrap();

    let snapshot = wait_for_terminal(&runtime, &accepted.task_id);
    assert_eq!(snapshot.state, TaskState::Failed);

    let report = recover_from_journal(&path);
    let task = report
        .tasks
        .iter()
        .find(|task| task.task_id == accepted.task_id)
        .unwrap();
    assert_eq!(task.disposition, RecoveredDisposition::Terminal);
    let error = task.error.as_ref().expect("failure must carry the error");
    assert_eq!(error.code, "vua.test.deliberate");
    assert_eq!(error.correlation_id, "corr-failed");
    std::fs::remove_file(&path).ok();
}

#[test]
fn orc_sta_006_a_restarted_runtime_takes_over_the_journal() {
    let (runtime, path) = file_runtime("restart");
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(|_context| Ok(TaskExit::Done(serde_json::json!({"result": "baked"})))),
        })
        .unwrap();
    let snapshot = wait_for_terminal(&runtime, &accepted.task_id);
    assert_eq!(snapshot.state, TaskState::Succeeded);

    // "Restart": a NEW runtime over the same journal file takes over (Fix 3).
    // The old task must be visible in snapshot_all, not just in a manual
    // recovery report.
    let restarted = TaskRuntime::new(
        Arc::new(JournalWriter::open(&path).unwrap()),
        Arc::new(SystemClock),
        // 纳秒生成器保证新任务的 id 与恢复进来的旧任务不同（真实部署同理）
        Arc::new(vua_orchestrator::NanosTaskIdGenerator::default()),
    );
    let tasks = restarted.snapshot_all();
    assert_eq!(tasks.len(), 1, "restarted runtime knows the old task");
    assert_eq!(tasks[0].task_id, accepted.task_id);
    assert_eq!(tasks[0].state, TaskState::Succeeded);

    // The sequence continues across the restart boundary: submitting a new
    // task appends entries whose seq never duplicates or regresses.
    let accepted2 = restarted
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(|_context| Ok(TaskExit::Done(serde_json::Value::Null))),
        })
        .unwrap();
    assert_ne!(accepted2.task_id, accepted.task_id);
    wait_for_terminal(&restarted, &accepted2.task_id);
    let report = recover_from_journal(&path);
    assert!(!report.corrupt_tail, "seq must stay strictly increasing");
    assert_eq!(report.tasks.len(), 2);
    std::fs::remove_file(&path).ok();
}

#[test]
fn orc_sta_006_interrupted_tasks_register_into_the_restarted_runtime() {
    // Simulate a crash mid-run: a journal that ends while the task was
    // Running (written directly, no runtime involved).
    let path = journal_path("interrupted");
    let writer = JournalWriter::open(&path).unwrap();
    let entry = |seq: u64, kind: vua_orchestrator::JournalEntryKind, payload| {
        vua_orchestrator::JournalEntryV1 {
            schema_version: vua_orchestrator::JOURNAL_SCHEMA_VERSION,
            seq,
            task_id: "task-crashed".into(),
            kind,
            occurred_at: "2026-08-31T00:00:00.000Z".into(),
            payload,
        }
    };
    writer
        .append(&entry(
            1,
            vua_orchestrator::JournalEntryKind::Accepted,
            vua_orchestrator::JournalPayload::Accepted {
                accepted: vua_orchestrator::CommandAcceptedV1 {
                    schema_version: 1,
                    task_id: "task-crashed".into(),
                    accepted_revision: 1,
                    initial_state: TaskState::Queued,
                },
            },
        ))
        .unwrap();
    writer
        .append(&entry(
            2,
            vua_orchestrator::JournalEntryKind::StateChanged,
            vua_orchestrator::JournalPayload::StateChanged {
                state: TaskState::Running,
            },
        ))
        .unwrap();

    // The restarted runtime registers the interrupted task with its last
    // journaled state; H-STATE will add explicit continue/recover actions.
    let restarted = TaskRuntime::new(
        Arc::new(JournalWriter::open(&path).unwrap()),
        Arc::new(SystemClock),
        // 纳秒生成器保证新任务的 id 与恢复进来的旧任务不同（真实部署同理）
        Arc::new(vua_orchestrator::NanosTaskIdGenerator::default()),
    );
    let tasks = restarted.snapshot_all();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task_id, "task-crashed");
    assert_eq!(tasks[0].state, TaskState::Running);
    assert!(!tasks[0].cancel_requested);

    // A regression check: a journal whose seq goes backwards is treated as a
    // corrupt tail and NOT registered (Fix 3, 拒绝带病回放).
    let bad_path = journal_path("regressed");
    let bad_writer = JournalWriter::open(&bad_path).unwrap();
    let accepted_entry = entry(
        5,
        vua_orchestrator::JournalEntryKind::Accepted,
        vua_orchestrator::JournalPayload::Accepted {
            accepted: vua_orchestrator::CommandAcceptedV1 {
                schema_version: 1,
                task_id: "task-good".into(),
                accepted_revision: 1,
                initial_state: TaskState::Queued,
            },
        },
    );
    bad_writer.append(&accepted_entry).unwrap();
    let regressed = entry(
        2,
        vua_orchestrator::JournalEntryKind::StateChanged,
        vua_orchestrator::JournalPayload::StateChanged {
            state: TaskState::Running,
        },
    );
    bad_writer.append(&regressed).unwrap();

    let after_bad = TaskRuntime::new(
        Arc::new(JournalWriter::open(&bad_path).unwrap()),
        Arc::new(SystemClock),
        Arc::new(vua_orchestrator::NanosTaskIdGenerator::default()),
    );
    assert!(
        after_bad.snapshot_all().is_empty(),
        "a regressed journal must not be trusted"
    );
    let report = recover_from_journal(&bad_path);
    assert!(report.corrupt_tail);
    std::fs::remove_file(&path).ok();
    std::fs::remove_file(&bad_path).ok();
}

#[test]
fn orc_wf_011_warning_marking_yields_succeeded_with_warnings() {
    let runtime = TaskRuntime::new(
        Arc::new(MemoryJournal::new()),
        Arc::new(FixedClock::new(&["2026-08-30T00:00:00.000Z"])),
        Arc::new(FixedIdGenerator::default()),
    );
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(|context| {
                context.warn();
                Ok(TaskExit::Done(serde_json::Value::Null))
            }),
        })
        .unwrap();
    let snapshot = wait_for_terminal(&runtime, &accepted.task_id);
    assert_eq!(snapshot.state, TaskState::SucceededWithWarnings);
}

#[test]
fn orc_ipc_002_snapshots_are_readable_without_side_effects() {
    let (runtime, path) = file_runtime("snapshots");
    assert!(runtime.snapshot("does-not-exist").is_none());
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(|_context| Ok(TaskExit::Done(serde_json::Value::Null))),
        })
        .unwrap();
    wait_for_terminal(&runtime, &accepted.task_id);
    let snapshot = runtime.snapshot(&accepted.task_id).unwrap();
    assert_eq!(
        snapshot.correlation_id,
        format!("corr-{}", accepted.task_id)
    );
    assert!(snapshot.revision >= 4);
    assert_eq!(runtime.snapshot_all().len(), 1);
    std::fs::remove_file(&path).ok();
}

#[test]
fn journal_payload_completed_roundtrips_through_the_wire_shape() {
    let entry = vua_orchestrator::JournalEntryV1 {
        schema_version: vua_orchestrator::JOURNAL_SCHEMA_VERSION,
        seq: 7,
        task_id: "task-x".into(),
        kind: JournalEntryKind::Completed,
        occurred_at: "2026-08-30T00:00:00.000Z".into(),
        payload: JournalPayload::Completed {
            state: TaskState::Failed,
            error: Some(AppErrorV1::new(
                "vua.test.x",
                ErrorCategory::Timeout,
                "errors.test.x",
                "corr-x",
            )),
            result: None,
        },
    };
    let line = serde_json::to_string(&entry).unwrap();
    let parsed: vua_orchestrator::JournalEntryV1 = serde_json::from_str(&line).unwrap();
    assert_eq!(parsed, entry);
    assert!(line.contains("\"payloadKind\":\"completed\""));
}

/* ---- poisoned-task visibility (honesty discipline #2): a persistence
 * failure must surface as a failure at the moment it happens, not as a task
 * that silently stops progressing until the next restart. ---- */

#[derive(Default)]
struct FailAfterAcceptanceJournal {
    entries: Mutex<Vec<vua_orchestrator::JournalEntryV1>>,
}

impl JournalSink for FailAfterAcceptanceJournal {
    fn append(
        &self,
        entry: &vua_orchestrator::JournalEntryV1,
    ) -> Result<(), vua_orchestrator::JournalError> {
        if entry.kind == JournalEntryKind::Accepted {
            self.entries.lock().unwrap().push(entry.clone());
            return Ok(());
        }
        Err(vua_orchestrator::JournalError::Io(std::io::Error::other(
            "simulated mid-task disk failure",
        )))
    }

    fn path(&self) -> Option<&std::path::Path> {
        None
    }
}

#[test]
fn poisoned_task_publishes_persistence_failed_and_reads_inspect_required() {
    let runtime = TaskRuntime::new(
        Arc::new(FailAfterAcceptanceJournal::default()),
        Arc::new(SystemClock),
        Arc::new(FixedIdGenerator::default()),
    );
    let receiver = runtime.subscribe();
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(|_context| Ok(TaskExit::Done(serde_json::Value::Null))),
        })
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let snapshot = runtime.snapshot(&accepted.task_id).unwrap();
        if snapshot.poisoned {
            break;
        }
        assert!(Instant::now() < deadline, "task was never poisoned");
        std::thread::sleep(Duration::from_millis(5));
    }

    let snapshot = runtime.snapshot(&accepted.task_id).unwrap();
    assert!(
        !snapshot.state.is_terminal(),
        "a poisoned task never claims a terminal state"
    );
    assert_eq!(
        snapshot.recovery_disposition,
        vua_orchestrator::TaskRecoveryDisposition::InspectRequired,
        "a poisoned task must read as awaiting inspection, not as running"
    );

    let events: Vec<_> = receiver.try_iter().collect();
    let accepted_revision = events
        .iter()
        .find(|event| event.kind == TaskEventKind::Accepted)
        .expect("acceptance event must have been published")
        .revision;
    let poison = events
        .iter()
        .find(|event| event.kind == TaskEventKind::PersistenceFailed)
        .expect("the freeze must be published as PersistenceFailed");
    assert!(poison.revision > accepted_revision);
    assert_eq!(poison.task_id, accepted.task_id);
    assert_eq!(poison.payload["code"], "vua.task.journal_write_failed");
    assert_eq!(
        poison.payload["recoverable"].as_bool(),
        Some(true),
        "the payload is the typed error envelope"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind == TaskEventKind::PersistenceFailed)
            .count(),
        1,
        "exactly one freeze notification per task"
    );
}

#[test]
fn cancelling_a_poisoned_task_returns_a_typed_error_instead_of_acknowledging() {
    let runtime = TaskRuntime::new(
        Arc::new(FailAfterAcceptanceJournal::default()),
        Arc::new(SystemClock),
        Arc::new(FixedIdGenerator::default()),
    );
    let accepted = runtime
        .submit(SubmitRequest {
            correlation_id: None,
            timeout: None,
            job: Box::new(|_context| Ok(TaskExit::Done(serde_json::Value::Null))),
        })
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let snapshot = runtime.snapshot(&accepted.task_id).unwrap();
        if snapshot.poisoned {
            break;
        }
        assert!(Instant::now() < deadline, "task was never poisoned");
        std::thread::sleep(Duration::from_millis(5));
    }

    let error = runtime
        .cancel(&accepted.task_id)
        .expect_err("a frozen task cannot record a cancellation request");
    assert_eq!(error.code, "vua.task.journal_write_failed");
    assert_eq!(error.category, ErrorCategory::Internal);
    assert!(error.recoverable);
}

/// A job that waits for its release, recording when it runs.
fn gated_job(name: &'static str, order: Arc<Mutex<Vec<&'static str>>>, gate: mpsc::Receiver<()>) -> SubmitRequest {
    SubmitRequest {
        correlation_id: None,
        timeout: None,
        job: Box::new(move |_ctx| {
            order.lock().unwrap().push(name);
            let _ = gate.recv_timeout(Duration::from_secs(10));
            Ok(TaskExit::Done(serde_json::json!({ "job": name })))
        }),
    }
}

#[test]
fn a_lane_runs_its_jobs_one_at_a_time_in_order() {
    let (runtime, _path) = file_runtime("lane-order");
    let lane = TaskLane::new();
    let order = Arc::new(Mutex::new(Vec::new()));
    let (release_a, gate_a) = mpsc::channel();
    let (release_b, gate_b) = mpsc::channel();
    let (release_c, gate_c) = mpsc::channel();
    let a = runtime.submit_in_lane(gated_job("a", order.clone(), gate_a), &lane).unwrap().task_id;
    wait_for_state(&runtime, &a, TaskState::Running);
    let b = runtime.submit_in_lane(gated_job("b", order.clone(), gate_b), &lane).unwrap().task_id;
    let c = runtime.submit_in_lane(gated_job("c", order.clone(), gate_c), &lane).unwrap().task_id;
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(runtime.snapshot(&b).unwrap().state, TaskState::Queued);
    assert_eq!(runtime.snapshot(&c).unwrap().state, TaskState::Queued);
    assert_eq!(lane.holder().as_deref(), Some(a.as_str()));
    assert_eq!(lane.waiting(), vec![b.clone(), c.clone()]);

    release_a.send(()).unwrap();
    assert_eq!(wait_for_terminal(&runtime, &a).state, TaskState::Succeeded);
    wait_for_state(&runtime, &b, TaskState::Running);
    assert_eq!(runtime.snapshot(&c).unwrap().state, TaskState::Queued);
    release_b.send(()).unwrap();
    release_c.send(()).unwrap();
    assert_eq!(wait_for_terminal(&runtime, &c).state, TaskState::Succeeded);
    assert_eq!(*order.lock().unwrap(), vec!["a", "b", "c"]);
    assert!(lane.is_idle());
}

#[test]
fn a_waiting_job_cancelled_ends_cancelled_without_running() {
    let (runtime, _path) = file_runtime("lane-cancel");
    let lane = TaskLane::new();
    let order = Arc::new(Mutex::new(Vec::new()));
    let (release_a, gate_a) = mpsc::channel();
    let (_release_b, gate_b) = mpsc::channel();
    let a = runtime.submit_in_lane(gated_job("a", order.clone(), gate_a), &lane).unwrap().task_id;
    wait_for_state(&runtime, &a, TaskState::Running);
    let b = runtime.submit_in_lane(gated_job("b", order.clone(), gate_b), &lane).unwrap().task_id;
    runtime.cancel(&b).unwrap();
    assert_eq!(wait_for_terminal(&runtime, &b).state, TaskState::Cancelled);
    assert!(lane.waiting().is_empty());
    release_a.send(()).unwrap();
    assert_eq!(wait_for_terminal(&runtime, &a).state, TaskState::Succeeded);
    assert_eq!(*order.lock().unwrap(), vec!["a"]);
    assert!(lane.is_idle());
}
