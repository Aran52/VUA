//! Bounded workers for the two AMF reads that can scan large local materials.
//! Commands stay on the frame loop; only that loop writes protocol responses.
use super::{
    application_error, application_error_with_params, FrameOutcome, InboundFrame,
    APPLICATION_CONTRACT_VERSION,
};
use crate::amf_production_routes::AmfProductionServices;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};

#[cfg(test)]
mod frame_tests;

const WORKERS: usize = 2;
const QUEUED: usize = 32;
type Read = Box<dyn FnOnce() -> Value + Send>;
struct Job {
    id: u64,
    read: Read,
    failure: Value,
}
struct Pending {
    frame_id: String,
    request_id: String,
    correlation_id: String,
}

pub(super) fn eligible(frame: &InboundFrame) -> bool {
    frame.kind == "request"
        && frame.payload["contractVersion"] == APPLICATION_CONTRACT_VERSION
        && frame.payload["kind"] == "query"
        && matches!(
            frame.payload["method"].as_str(),
            Some("amf.production.list" | "amf.production.recipeMaterialPlan")
        )
}

pub(super) struct ReadQueries {
    sender: mpsc::SyncSender<Job>,
    receiver: mpsc::Receiver<(u64, Value)>,
    stopped: Arc<AtomicBool>,
    pending: BTreeMap<u64, Pending>,
    next_id: u64,
}

impl ReadQueries {
    pub(super) fn new() -> Self {
        let (sender, jobs) = mpsc::sync_channel::<Job>(QUEUED);
        let jobs = Arc::new(Mutex::new(jobs));
        let (results, receiver) = mpsc::channel();
        let stopped = Arc::new(AtomicBool::new(false));
        for _ in 0..WORKERS {
            let jobs = jobs.clone();
            let results = results.clone();
            let stopped = stopped.clone();
            std::thread::spawn(move || loop {
                let Ok(job) = jobs.lock().expect("read queue poisoned").recv() else {
                    break;
                };
                if stopped.load(Ordering::Acquire) {
                    continue;
                }
                let value = catch_unwind(AssertUnwindSafe(job.read)).unwrap_or(job.failure);
                if results.send((job.id, value)).is_err() {
                    break;
                }
            });
        }
        Self {
            sender,
            receiver,
            stopped,
            pending: BTreeMap::new(),
            next_id: 0,
        }
    }

    /// Reuse the ordinary AMF route's parameter validation and response envelope.
    pub(super) fn submit(
        &mut self,
        frame: &InboundFrame,
        service: Arc<AmfProductionServices>,
    ) -> Option<Value> {
        let request = frame.payload.clone();
        self.enqueue(
            frame,
            Box::new(move || {
                let FrameOutcome::Response(value) = crate::amf_production_routes::request(
                    Some(service),
                    request["method"].as_str().unwrap_or(""),
                    &request,
                    request["requestId"].as_str().unwrap_or("invalid"),
                    request["correlationId"].as_str().unwrap_or("invalid"),
                ) else {
                    unreachable!("AMF queries return application responses");
                };
                value
            }),
        )
    }

    fn enqueue(&mut self, frame: &InboundFrame, read: Read) -> Option<Value> {
        let pending = Pending {
            frame_id: frame.frame_id.clone(),
            request_id: frame.payload["requestId"]
                .as_str()
                .unwrap_or("invalid")
                .into(),
            correlation_id: frame.payload["correlationId"]
                .as_str()
                .unwrap_or("invalid")
                .into(),
        };
        let busy = || {
            application_error_with_params(
                &pending.request_id,
                &pending.correlation_id,
                "vua.amf_production.busy",
                "errors.amfProduction.failed",
                "conflict",
                &[(
                    "detail",
                    vua_orchestrator::ParamValue::Text(
                        "The material read queue is full. Retry after pending reads finish.".into(),
                    ),
                )],
            )
        };
        if self.pending.len() >= WORKERS + QUEUED {
            return Some(busy());
        }
        let id = self.next_id;
        self.next_id += 1;
        let failure = unavailable(&pending);
        match self.sender.try_send(Job { id, read, failure }) {
            Ok(()) => {
                self.pending.insert(id, pending);
                None
            }
            Err(mpsc::TrySendError::Full(_)) => Some(busy()),
            Err(mpsc::TrySendError::Disconnected(_)) => Some(unavailable(&pending)),
        }
    }

    pub(super) fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub(super) fn receive(&mut self, wait: Duration) -> io::Result<Option<(String, Value)>> {
        match self.receiver.recv_timeout(wait) {
            Ok((id, value)) => Ok(self
                .pending
                .remove(&id)
                .map(|pending| (pending.frame_id, value))),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) if self.pending.is_empty() => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(io::Error::other("AMF read workers ended"))
            }
        }
    }

    /// Reads hold no project mutation lease. Shutdown rejects their outstanding calls
    /// before its receipt, without waiting for a large hash or starting queued reads.
    pub(super) fn cancel(&mut self) -> Vec<(String, Value)> {
        self.stopped.store(true, Ordering::Release);
        std::mem::take(&mut self.pending)
            .into_values()
            .map(|pending| {
                let value = unavailable(&pending);
                (pending.frame_id, value)
            })
            .collect()
    }
}

impl Drop for ReadQueries {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
    }
}

fn unavailable(pending: &Pending) -> Value {
    application_error(
        &pending.request_id,
        &pending.correlation_id,
        "vua.amf_production.unavailable",
        "errors.amfProduction.unavailable",
        "unavailable",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn frame(id: &str) -> InboundFrame {
        serde_json::from_value(json!({"frameVersion":"0.1", "frameId":format!("frame-{id}"), "kind":"request",
            "payload":{"contractVersion":"0.1", "kind":"query", "method":"amf.production.list",
                "requestId":id,"correlationId":format!("corr-{id}"),"params":{"schemaVersion":"0.1"}}})).unwrap()
    }

    #[test]
    fn only_allowlisted_queries_with_the_current_envelope_leave_the_frame_loop() {
        let mut request = frame("read");
        assert!(eligible(&request));
        for method in [
            "amf.production.prepare",
            "amf.production.setCapacity",
            "library.list",
        ] {
            request.payload["method"] = json!(method);
            assert!(!eligible(&request));
        }
        request.payload["method"] = json!("amf.production.recipeMaterialPlan");
        assert!(eligible(&request));
        request.payload["kind"] = json!("command");
        assert!(!eligible(&request));
        request.payload["kind"] = json!("query");
        request.payload["contractVersion"] = json!("future");
        assert!(!eligible(&request));
        request.payload["contractVersion"] = json!("0.1");
        request.kind = "handshake".into();
        assert!(!eligible(&request));
    }

    #[test]
    fn replies_keep_their_identity_when_the_later_read_finishes_first() {
        let mut reads = ReadQueries::new();
        let (release, held) = mpsc::channel();
        let (started, observed) = mpsc::channel();
        assert!(reads
            .enqueue(
                &frame("slow"),
                Box::new(move || {
                    started.send(()).unwrap();
                    held.recv_timeout(Duration::from_secs(10)).unwrap();
                    json!({"requestId":"slow"})
                })
            )
            .is_none());
        observed.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(reads
            .enqueue(&frame("fast"), Box::new(|| json!({"requestId":"fast"})))
            .is_none());
        assert_eq!(
            reads.receive(Duration::from_secs(5)).unwrap(),
            Some(("frame-fast".into(), json!({"requestId":"fast"})))
        );
        assert!(reads.has_pending());
        release.send(()).unwrap();
        assert_eq!(
            reads.receive(Duration::from_secs(5)).unwrap(),
            Some(("frame-slow".into(), json!({"requestId":"slow"})))
        );
        assert!(!reads.has_pending());
    }

    #[test]
    fn queue_pressure_is_bounded_and_shutdown_rejects_pending_reads() {
        let mut reads = ReadQueries::new();
        let mut release = Vec::new();
        for id in ["one", "two"] {
            let (send, held) = mpsc::channel();
            let (started, observed) = mpsc::channel();
            release.push(send);
            assert!(reads
                .enqueue(
                    &frame(id),
                    Box::new(move || {
                        started.send(()).unwrap();
                        held.recv_timeout(Duration::from_secs(10)).unwrap();
                        Value::Null
                    })
                )
                .is_none());
            observed.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        for index in 0..QUEUED {
            assert!(reads
                .enqueue(
                    &frame(&format!("queued-{index}")),
                    Box::new(|| panic!("cancelled work must not run"))
                )
                .is_none());
        }
        assert_eq!(
            reads
                .enqueue(&frame("overflow"), Box::new(|| Value::Null))
                .unwrap()["error"]["code"],
            "vua.amf_production.busy"
        );
        let cancelled = reads.cancel();
        assert_eq!(cancelled.len(), WORKERS + QUEUED);
        assert!(!reads.has_pending());
        for (id, value) in cancelled {
            assert_eq!(
                format!("frame-{}", value["requestId"].as_str().unwrap()),
                id
            );
            assert_eq!(value["error"]["code"], "vua.amf_production.unavailable");
        }
        for send in release {
            send.send(()).unwrap();
        }
    }

    #[test]
    fn a_panicking_read_returns_a_bounded_error_and_does_not_remove_a_worker() {
        let mut reads = ReadQueries::new();
        reads.enqueue(
            &frame("panic"),
            Box::new(|| panic!("synthetic read failure")),
        );
        let (id, value) = reads.receive(Duration::from_secs(5)).unwrap().unwrap();
        assert_eq!(id, "frame-panic");
        assert_eq!(value["requestId"], "panic");
        assert_eq!(value["error"]["code"], "vua.amf_production.unavailable");
        reads.enqueue(&frame("after"), Box::new(|| json!({"requestId":"after"})));
        assert_eq!(
            reads.receive(Duration::from_secs(5)).unwrap().unwrap().0,
            "frame-after"
        );
    }
}
