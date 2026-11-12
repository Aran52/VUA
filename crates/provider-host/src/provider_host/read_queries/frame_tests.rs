use super::super::*;
use std::{io::Cursor, sync::mpsc};
use vua_orchestrator::{amf_production::*, TaskContext};

struct HeldSources(Mutex<mpsc::Receiver<()>>);
impl AmfProductionPlatform for HeldSources {
    fn sources(&self) -> Result<Vec<ProductionSource>, AppErrorV1> {
        self.0
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(15))
            .unwrap();
        Ok(vec![])
    }
    fn editors(&self) -> Result<Vec<ProductionEditor>, AppErrorV1> {
        Ok(vec![])
    }
    fn verify_editor(&self, _: &str) -> Result<ProductionEditor, AppErrorV1> {
        unreachable!()
    }
    fn create_project(&self, _: &ProductionRun, _: &TaskContext) -> Result<(), AppErrorV1> {
        unreachable!()
    }
    fn import_sources(
        &self,
        _: &ProductionRun,
        _: &TaskContext,
    ) -> Result<Vec<PreparedSource>, AppErrorV1> {
        unreachable!()
    }
    fn inspect(&self, _: &ProductionRun, _: &UnityWatch) -> Result<ObservedProject, AppErrorV1> {
        unreachable!()
    }
    fn assemble(
        &self,
        _: &ProductionRun,
        _: &AssemblySelection,
        _: &[OutfitFit],
        _: &UnityWatch,
    ) -> Result<ObservedProject, AppErrorV1> {
        unreachable!()
    }
    fn fingerprint(&self, _: &Path) -> Result<String, AppErrorV1> {
        unreachable!()
    }
    fn fingerprint_legacy(&self, _: &Path) -> Result<String, AppErrorV1> {
        unreachable!()
    }
    fn snapshot(&self, _: &ProductionRun) -> Result<String, AppErrorV1> {
        unreachable!()
    }
    fn restore(&self, _: &ProductionRun, _: &str) -> Result<(), AppErrorV1> {
        unreachable!()
    }
    fn open(&self, _: &ProductionRun, _: &TaskContext) -> Result<Value, AppErrorV1> {
        unreachable!()
    }
}

struct FrameWriter(mpsc::Sender<Value>, Vec<u8>);
impl Write for FrameWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.1.extend_from_slice(bytes);
        while let Some(end) = self.1.iter().position(|byte| *byte == b'\n') {
            let value = serde_json::from_slice(&self.1[..end])?;
            self.0
                .send(value)
                .map_err(|_| std::io::Error::other("test reader ended"))?;
            self.1.drain(..=end);
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn state(root: &Path, held: mpsc::Receiver<()>) -> HostState {
    std::fs::create_dir_all(root).unwrap();
    let store = Arc::new(SqliteTaskStore::open(root.join("tasks.db")).unwrap());
    let runtime = TaskRuntime::with_sqlite(
        store.clone(),
        Arc::new(SystemClock),
        Arc::new(NanosTaskIdGenerator::default()),
    )
    .unwrap();
    HostState {
        amf_production: Some(Arc::new(
            crate::amf_production_routes::AmfProductionServices::new(
                AmfProductionEngine::new(root.join("amf"), Arc::new(HeldSources(Mutex::new(held)))),
                runtime,
            ),
        )),
        unity_license: None,
        store,
        provider_instance_id: "synthetic-reads".into(),
        recovered_nonterminal_tasks: HashSet::new(),
        production: None,
        downloads: None,
        warehouse: None,
        use_cases: None,
        project_ops: None,
        environment: None,
        vpm: None,
        runtime_events: Arc::new(Mutex::new(vec![])),
        editor_verify: Arc::new(|_| unreachable!()),
    }
}

fn request(id: &str, method: &str, kind: &str, params: Value) -> Value {
    json!({"frameVersion":"0.1", "frameId":format!("frame-{id}"), "kind":"request", "payload":{
        "contractVersion":"0.1", "requestId":id, "correlationId":format!("corr-{id}"),
        "method":method, "kind":kind, "params":params, "commandId":format!("command-{id}")}})
}
fn input(frames: &[Value]) -> Cursor<String> {
    Cursor::new(
        frames
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
}

#[test]
fn the_frame_loop_serves_fast_queries_and_commands_before_a_held_read_and_drains_eof() {
    let root = std::env::temp_dir().join(format!("vua-query-frames-{}", uuid_v7_identity()));
    let (release, held) = mpsc::channel();
    let state = state(&root, held);
    let service = state.amf_production.clone().unwrap();
    let frames = input(&[
        request(
            "slow",
            "amf.production.list",
            "query",
            json!({"schemaVersion":"0.1"}),
        ),
        request("fast", "task.list", "query", json!({})),
        request(
            "first",
            "amf.production.setCapacity",
            "command",
            json!({"schemaVersion":"0.1","capacity":1}),
        ),
        request(
            "second",
            "amf.production.setCapacity",
            "command",
            json!({"schemaVersion":"0.1","capacity":2}),
        ),
    ]);
    let (output, responses) = mpsc::channel();
    let host = std::thread::spawn(move || serve_frames(frames, FrameWriter(output, vec![]), state));
    let mut seen = Vec::new();
    for expected in ["fast", "first", "second"] {
        let response = responses.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(response["frameId"], format!("frame-{expected}"));
        assert_eq!(response["payload"]["requestId"], expected);
        assert_eq!(response["payload"]["ok"], true, "{response}");
        seen.push(response);
    }
    assert_eq!(seen[1]["payload"]["value"]["capacity"], 1);
    assert_eq!(seen[2]["payload"]["value"]["capacity"], 2);
    assert_eq!(service.engine.capacity(), 2);
    release.send(()).unwrap();
    let response = responses.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(response["frameId"], "frame-slow");
    assert_eq!(response["payload"]["requestId"], "slow");
    assert_eq!(response["payload"]["ok"], true);
    host.join().unwrap().unwrap();
    drop(service);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn shutdown_answers_pending_reads_before_its_receipt_without_waiting_for_them() {
    let root = std::env::temp_dir().join(format!("vua-query-stop-{}", uuid_v7_identity()));
    let (release, held) = mpsc::channel();
    let state = state(&root, held);
    let frames = input(&[
        request(
            "slow",
            "amf.production.list",
            "query",
            json!({"schemaVersion":"0.1"}),
        ),
        json!({"frameVersion":"0.1","frameId":"shutdown","kind":"prepare_shutdown","payload":{"timeoutMs":1}}),
    ]);
    let (output, responses) = mpsc::channel();
    let host = std::thread::spawn(move || serve_frames(frames, FrameWriter(output, vec![]), state));
    let cancelled = responses.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(cancelled["frameId"], "frame-slow");
    assert_eq!(
        cancelled["payload"]["error"]["code"],
        "vua.amf_production.unavailable"
    );
    let stopped = responses.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(stopped["frameId"], "shutdown");
    assert_eq!(stopped["payload"]["outcome"], "safe_to_stop");
    host.join().unwrap().unwrap();
    let _ = release.send(());
    // A running advisory read can still own its local store until the process exits.
    // Keep this isolated fixture for the OS temp cleanup rather than race that worker.
}
