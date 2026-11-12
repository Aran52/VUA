//! Lifecycle recovery regression (BOARD #20): a task left non-terminal by
//! a hard process death must never keep reading as `running` after a
//! restart — every tasked face the host runs (production AND the demo
//! face alike) is swept to the interrupted semantics on startup, so the
//! snapshot presents the honest state and recovery stays an explicit
//! decision. exercised over two REAL provider processes on one database.

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

struct Provider {
    child: Child,
    reader: std::io::BufReader<std::process::ChildStdout>,
    writer: BufWriter<std::process::ChildStdin>,
    next_id: u32,
}

impl Provider {
    fn spawn(database: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vua-orchestrator-provider"))
            .arg("--database")
            .arg(database)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("provider spawns");
        let reader = BufReader::new(child.stdout.take().expect("stdout"));
        let writer = BufWriter::new(child.stdin.take().expect("stdin"));
        let mut provider = Self {
            child,
            reader,
            writer,
            next_id: 0,
        };
        provider.expect_handshake();
        provider
    }

    fn read_frame(&mut self) -> Value {
        let mut line = String::new();
        let read = self.reader.read_line(&mut line).expect("stdout readable");
        assert!(read > 0, "the provider closed its stdout unexpectedly");
        serde_json::from_str(line.trim()).expect("frames are valid JSON")
    }

    fn send(&mut self, frame: &Value) {
        self.writer
            .write_all((serde_json::to_string(frame).expect("serializable") + "\n").as_bytes())
            .expect("stdin writable");
        self.writer.flush().expect("flush");
    }

    fn expect_handshake(&mut self) {
        let frame_id = "handshake";
        self.send(&json!({
            "frameVersion": "0.1",
            "frameId": frame_id,
            "kind": "handshake",
            "payload": null,
        }));
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            assert!(Instant::now() < deadline, "handshake did not arrive");
            let frame = self.read_frame();
            if frame["frameId"] == frame_id {
                assert_eq!(frame["kind"], "response");
                return;
            }
            // Events for tasks from earlier sessions may arrive first; skip.
        }
    }

    fn request(&mut self, method: &str, params: Value, extra: Value) -> Value {
        self.next_id += 1;
        let frame_id = format!("req-{}", self.next_id);
        let mut payload = json!({
            "contractVersion": "0.1",
            "requestId": frame_id,
            "correlationId": format!("corr-{frame_id}"),
            "kind": "command",
            "method": method,
            "params": params,
        });
        for (key, value) in extra.as_object().expect("extra is an object") {
            payload[key] = value.clone();
        }
        self.send(&json!({
            "frameVersion": "0.1",
            "frameId": frame_id,
            "kind": "request",
            "payload": payload,
        }));
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            assert!(Instant::now() < deadline, "{method} did not answer");
            let frame = self.read_frame();
            if frame["frameId"] == frame_id {
                assert_eq!(
                    frame["kind"], "response",
                    "{method} answered a response: {frame}"
                );
                return frame;
            }
        }
    }

    fn kill(&mut self) {
        // A hard death the process did not expect — the #20 shape.
        self.child.kill().expect("kill");
        self.child.wait().expect("reaped");
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn a_hard_killed_demo_task_never_keeps_reading_running_after_a_restart() {
    let root = std::env::temp_dir().join(format!(
        "vua-lifecycle-recovery-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("root creates");
    let database = root.join("tasks.sqlite");

    // Session one: accept the demo task and wait until it is running.
    {
        let mut provider = Provider::spawn(&database);
        let command_id = format!("spike-{}", std::process::id());
        let acceptance = provider.request(
            "task.startDemo",
            json!({}),
            json!({ "commandId": command_id }),
        );
        assert_eq!(acceptance["payload"]["ok"], true, "{acceptance}");
        let task_id = acceptance["payload"]["value"]["task"]["taskId"]
            .as_str()
            .expect("taskId")
            .to_owned();

        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let listing = provider.request("task.list", json!({}), json!({}));
            let tasks = listing["payload"]["value"]["tasks"]
                .as_array()
                .expect("tasks");
            let state = tasks
                .iter()
                .find(|task| task["taskId"] == json!(task_id))
                .map(|task| task["state"].clone())
                .expect("the accepted task lists");
            if state == "running" {
                break;
            }
            assert!(Instant::now() < deadline, "the demo task never ran");
            std::thread::sleep(Duration::from_millis(20));
        }

        // The #20 shape: a hard death the process did not expect, with the
        // task non-terminal.
        provider.kill();

        // Session two on the same database: the interrupted task must read
        // its honest post-mortem state — never `running`.
        let mut successor = Provider::spawn(&database);
        let listing = successor.request("task.list", json!({}), json!({}));
        let tasks = listing["payload"]["value"]["tasks"]
            .as_array()
            .expect("tasks");
        let state = tasks
            .iter()
            .find(|task| task["taskId"] == json!(task_id))
            .map(|task| task["state"].clone())
            .expect("the interrupted task still lists after the restart");
        assert_ne!(
            state, "running",
            "a dead process's task must never keep reading as running (BOARD #20)"
        );
        assert!(
            state == "failed" || state == "cancelled",
            "the swept state is the interrupted semantics: {state}"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}
