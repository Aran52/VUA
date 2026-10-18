use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

fn link() -> TermsLink {
    TermsLink {
        source_id: "drive:synthetic".into(),
        source_kind: "drive".into(),
        source_url: "https://drive.google.com/file/d/synthetic/view".into(),
        google_file_id: Some("synthetic".into()),
    }
}
fn server(
    response: Vec<u8>,
    delay: Duration,
    count: usize,
) -> (
    String,
    mpsc::Receiver<(Instant, String)>,
    std::thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/document", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut accepted = 0;
        while accepted < count && Instant::now() < deadline {
            let (mut stream, _) = match listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(e) => panic!("loopback accept: {e}"),
            };
            // On Windows an accepted socket inherits the listener's non-blocking mode.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut bytes = [0; 4096];
            let n = stream.read(&mut bytes).unwrap();
            let _ = tx.send((Instant::now(), String::from_utf8_lossy(&bytes[..n]).into()));
            std::thread::sleep(delay);
            let _ = stream.write_all(&response);
            accepted += 1;
        }
    });
    (url, rx, worker)
}
fn response(status: &str, headers: &str, body: &[u8]) -> Vec<u8> {
    let mut bytes = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n",
        body.len()
    )
    .into_bytes();
    bytes.extend_from_slice(body);
    bytes
}
fn downloader(url: String) -> PublicTermsDownloader {
    PublicTermsDownloader {
        pacer: Arc::new(Mutex::new(None)),
        interval: Duration::from_millis(1),
        timeout: Duration::from_secs(1),
        limit: 1024,
        endpoint: Some(url),
    }
}

#[test]
fn loopback_pdf_and_non_pdf_results_never_send_credentials() {
    for (body, expected) in [
        (b"%PDF-1.4 synthetic".as_slice(), "pdf"),
        (b"<html>Sign in</html>".as_slice(), "not_public"),
        (b"unsupported plain text".as_slice(), "unsupported"),
    ] {
        let (url, requests, worker) = server(
            response("200 OK", "Content-Type: application/octet-stream\r\n", body),
            Duration::ZERO,
            1,
        );
        let result = downloader(url).fetch(&link(), &|| false);
        match result {
            FetchOutcome::Pdf(bytes) => {
                assert_eq!(expected, "pdf");
                assert_eq!(bytes, body)
            }
            FetchOutcome::Observed { status, .. } => assert_eq!(status, expected),
            FetchOutcome::Cancelled => panic!("unexpected cancellation"),
        }
        let headers = requests.recv().unwrap().1.to_ascii_lowercase();
        assert!(!headers.contains("cookie:"));
        assert!(!headers.contains("authorization:"));
        worker.join().unwrap();
    }
}
#[test]
fn redirect_size_limit_and_timeout_are_classified() {
    let (url, _, worker) = server(
        response(
            "302 Found",
            "Location: http://127.0.0.1:1/forbidden\r\n",
            b"",
        ),
        Duration::ZERO,
        1,
    );
    assert!(matches!(
        downloader(url).fetch(&link(), &|| false),
        FetchOutcome::Observed {
            status: "unsupported",
            problem: "redirect_not_supported"
        }
    ));
    worker.join().unwrap();
    let (url, _, worker) = server(response("200 OK", "", &[b'x'; 64]), Duration::ZERO, 1);
    let mut client = downloader(url);
    client.limit = 16;
    assert!(matches!(
        client.fetch(&link(), &|| false),
        FetchOutcome::Observed {
            status: "failed",
            problem: "size_limit"
        }
    ));
    worker.join().unwrap();
    let (url, _, worker) = server(
        response("200 OK", "", b"%PDF-1.4"),
        Duration::from_millis(150),
        1,
    );
    let mut client = downloader(url);
    client.timeout = Duration::from_millis(40);
    assert!(matches!(
        client.fetch(&link(), &|| false),
        FetchOutcome::Observed {
            status: "failed",
            problem: "timeout"
        }
    ));
    worker.join().unwrap();
}
#[test]
fn cancellation_interrupts_active_request_and_shared_pacing() {
    let (url, requests, worker) = server(
        response("200 OK", "", b"%PDF-1.4"),
        Duration::from_millis(200),
        1,
    );
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let cancel = std::thread::spawn(move || {
        requests.recv().unwrap();
        flag.store(true, Ordering::SeqCst);
    });
    assert!(matches!(
        downloader(url).fetch(&link(), &|| cancelled.load(Ordering::SeqCst)),
        FetchOutcome::Cancelled
    ));
    cancel.join().unwrap();
    worker.join().unwrap();
    let (url, requests, worker) = server(response("200 OK", "", b"%PDF-1.4"), Duration::ZERO, 2);
    let mut first = downloader(url.clone());
    first.interval = Duration::from_millis(60);
    let mut second = downloader(url);
    second.pacer = first.pacer.clone();
    second.interval = first.interval;
    assert!(matches!(
        first.fetch(&link(), &|| false),
        FetchOutcome::Pdf(_)
    ));
    assert!(matches!(
        second.fetch(&link(), &|| false),
        FetchOutcome::Pdf(_)
    ));
    let a = requests.recv().unwrap().0;
    let b = requests.recv().unwrap().0;
    assert!(b.duration_since(a) >= Duration::from_millis(60));
    assert!(matches!(
        second.fetch(&link(), &|| true),
        FetchOutcome::Cancelled
    ));
    worker.join().unwrap();
}
