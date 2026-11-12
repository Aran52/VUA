//! Handshake wire-contract tests (proposal 001, provider-frame-v0.1).
//!
//! The handshake request/response frames are frozen by
//! `schemas/orchestrator/provider-frame-v0.1/*.schema.json`; the Rust host and
//! the TypeScript supervisor (packages/orchestrator-provider) both consume the
//! same positive and negative vectors from the sibling `fixtures/` directory.
//! Changing a handshake shape without updating schema, vectors and both
//! consumers fails here first.

use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::Value;
use vua_provider_host::run_provider_host;

const REQUEST_SCHEMA: &str = "handshake-request.schema.json";
const RESPONSE_SCHEMA: &str = "handshake-response.schema.json";

/// Monotonic suffix so parallel tests never collide on the temp store
/// (nanos alone can tie when tests start in the same instant).
static SEQUENCE: AtomicU32 = AtomicU32::new(0);

fn frame_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/orchestrator/provider-frame-v0.1")
}

fn read_json(relative: &str) -> Value {
    let bytes = fs::read(frame_dir().join(relative)).expect("schema/vector must exist");
    serde_json::from_slice(&bytes).expect("schema/vector must be valid JSON")
}

fn validator(schema: &Value) -> jsonschema::Validator {
    jsonschema::validator_for(schema).expect("frozen schema must compile")
}

/// Feeds one frame at a time through the real frame loop against a throwaway
/// store and returns the parsed output frames.
fn run_frames(frames: &[Value]) -> Vec<Value> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let serial = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "vua-provider-handshake-{}-{nanos}-{serial}.db",
        std::process::id()
    ));
    let mut input = String::new();
    for frame in frames {
        input.push_str(&frame.to_string());
        input.push('\n');
    }
    let mut output = Vec::new();
    run_provider_host(Cursor::new(input), &mut output, &path)
        .expect("the frame loop must stay alive for handshake vectors");
    let parsed: Vec<Value> = String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect();
    drop(fs::remove_file(&path));
    parsed
}

#[test]
fn handshake_schemas_and_vectors_exist_in_pairs() {
    for name in [REQUEST_SCHEMA, RESPONSE_SCHEMA] {
        let schema = read_json(name);
        assert_eq!(
            schema["properties"]["frameVersion"]["const"], "0.1",
            "{name}"
        );
    }
    for name in [
        "fixtures/handshake.request.valid.json",
        "fixtures/handshake.response.valid.json",
        "fixtures/handshake.response.valid-no-downloads.json",
        "fixtures/handshake.request.invalid-frame-version.json",
        "fixtures/handshake.request.invalid-empty-frame-id.json",
        "fixtures/handshake.request.invalid-non-null-payload.json",
        "fixtures/handshake.response.invalid-missing-download-ingest.json",
        "fixtures/handshake.response.invalid-contract-version.json",
        "fixtures/handshake.response.invalid-unsupported-contract-only.json",
        "fixtures/handshake.response.invalid-empty-provider-build-id.json",
        "fixtures/handshake.response.invalid-kind-event.json",
    ] {
        read_json(name);
    }
}

#[test]
fn handshake_request_vectors_adhere_to_the_frozen_request_schema() {
    let schema = read_json(REQUEST_SCHEMA);
    let validator = validator(&schema);
    let valid = read_json("fixtures/handshake.request.valid.json");
    assert!(
        validator.is_valid(&valid),
        "the positive request vector is normative"
    );

    for name in [
        "fixtures/handshake.request.invalid-frame-version.json",
        "fixtures/handshake.request.invalid-empty-frame-id.json",
        "fixtures/handshake.request.invalid-non-null-payload.json",
    ] {
        let invalid = read_json(name);
        assert!(
            !validator.is_valid(&invalid),
            "{name} must be rejected by the frozen request schema"
        );
    }
}

#[test]
fn handshake_response_vectors_adhere_to_the_frozen_response_schema() {
    let schema = read_json(RESPONSE_SCHEMA);
    let validator = validator(&schema);

    for name in [
        "fixtures/handshake.response.valid.json",
        "fixtures/handshake.response.valid-no-downloads.json",
    ] {
        let valid = read_json(name);
        assert!(validator.is_valid(&valid), "{name} is normative");
    }
    for name in [
        "fixtures/handshake.response.invalid-missing-download-ingest.json",
        "fixtures/handshake.response.invalid-contract-version.json",
        "fixtures/handshake.response.invalid-unsupported-contract-only.json",
        "fixtures/handshake.response.invalid-empty-provider-build-id.json",
        "fixtures/handshake.response.invalid-kind-event.json",
    ] {
        let invalid = read_json(name);
        assert!(
            !validator.is_valid(&invalid),
            "{name} must be rejected by the frozen response schema"
        );
    }
}

/// The host consumes the positive request vector and produces a response that
/// validates against the frozen response schema, reuses the request frameId
/// and carries the mandatory downloadIngest bit (false without a download
/// domain — the vector pair pins both shapes).
#[test]
fn host_answers_the_valid_handshake_vector_with_a_frozen_response() {
    let request = read_json("fixtures/handshake.request.valid.json");
    let frames = run_frames(&[request]);
    assert_eq!(frames.len(), 1, "one handshake frame in, one frame out");
    let response = &frames[0];

    let schema = read_json(RESPONSE_SCHEMA);
    let validator = validator(&schema);
    assert!(
        validator.is_valid(response),
        "the host response must match the frozen response schema: {response}"
    );

    let no_downloads = read_json("fixtures/handshake.response.valid-no-downloads.json");
    assert_eq!(response["frameVersion"], no_downloads["frameVersion"]);
    assert_eq!(
        response["frameId"], no_downloads["frameId"],
        "the response reuses the request frameId"
    );
    assert_eq!(response["kind"], no_downloads["kind"]);
    assert_eq!(
        response["payload"]["contractVersion"],
        no_downloads["payload"]["contractVersion"]
    );
    assert_eq!(
        response["payload"]["supportedContractVersions"],
        no_downloads["payload"]["supportedContractVersions"]
    );
    assert_eq!(
        response["payload"]["downloadIngest"], no_downloads["payload"]["downloadIngest"],
        "no download domain configured: the bit must be honestly false"
    );
    assert!(!response["payload"]["providerBuildId"]
        .as_str()
        .unwrap()
        .is_empty());
    assert!(!response["payload"]["providerInstanceId"]
        .as_str()
        .unwrap()
        .is_empty());
}

/// Negative request vectors are answered with protocol_error — never with a
/// handshake response, never silently ignored.
#[test]
fn host_rejects_the_negative_handshake_request_vectors_with_protocol_error() {
    let cases = [
        (
            "fixtures/handshake.request.invalid-frame-version.json",
            "vua.provider.unsupported_frame_version",
        ),
        (
            "fixtures/handshake.request.invalid-empty-frame-id.json",
            "vua.provider.unsupported_frame_version",
        ),
        (
            "fixtures/handshake.request.invalid-non-null-payload.json",
            "vua.provider.invalid_handshake",
        ),
    ];
    for (name, code) in cases {
        let frame = read_json(name);
        let frames = run_frames(&[frame]);
        assert_eq!(frames.len(), 1, "{name}: exactly one protocol_error frame");
        assert_eq!(frames[0]["kind"], "protocol_error", "{name}");
        assert_eq!(frames[0]["payload"]["code"], code, "{name}");
        assert!(
            frames[0]["payload"].get("contractVersion").is_none(),
            "{name}: a protocol error must never masquerade as a handshake"
        );
    }
}
