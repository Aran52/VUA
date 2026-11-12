//! environment.verifyEditor wire tests (proposal 021 core routing batch):
//! the provider-host route maps the detection-domain primitive of record
//! (`verify_editor_path_system`, project-manager) onto the draft-frozen
//! two-state result of `schemas/editor-verify/v0.1` and nothing else. The
//! consumer consumes the frozen schema face directly, so the route and the
//! schema cannot drift silently.
//!
//! The ruling's implementation nails are pinned here at the wire face:
//! - Nail 1: a refusal NEVER surfaces as an application error envelope —
//!   refused is a normal in-result finding (`ok:true` + `verdict:"refused"`);
//! - Nail 2: the refusal detail travels verbatim — the wire layer adds no
//!   interpretation;
//! - Nail 3: the envelope schemaVersion is the row's own
//!   EDITOR_VERIFY_SCHEMA_VERSION const ("0.1"), never a borrowed version.
//!
//! The honest absence code `vua.environment.verify_unavailable` (ruling
//! point 5) is reserved for an unwired route / unreachable primitive and is
//! asserted never to appear as a verification refusal.
//!
//! The verified branch is exercised through an injected deterministic
//! verifier (the wire face under test is the mapping, not the PE resource
//! reader); the refused branch additionally runs through the real system
//! wiring against a nonexistent path, which refuses deterministically
//! (target_missing on Windows; on non-Windows the system wiring itself
//! refuses with unsupported_platform before any path work).

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use jsonschema::Validator;
use serde_json::{json, Value};
use vua_orchestrator::editor_target_codes;
use vua_project_manager::{EditorPathIdentity, EditorPathRefusal, EditorPathVerdict};
use vua_provider_host::{
    run_provider_host_full, EditorPathVerifier, EDITOR_VERIFY_SCHEMA_VERSION,
    ENVIRONMENT_VERIFY_UNAVAILABLE,
};

fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/editor-verify/v0.1")
}

fn read_json_at(dir: &Path, relative: &str) -> Value {
    let bytes = fs::read(dir.join(relative)).expect("schema file must exist");
    serde_json::from_slice(&bytes).expect("schema file must be valid JSON")
}

/// Compiles one $defs entry of the method schema with the full $defs
/// re-attached, so nested $ref targets resolve (same compilation shape as
/// the draft-side contract test
/// crates/project-manager/tests/editor_verify_wire.rs).
fn sub_schema_validator(def_suffix: &str) -> Validator {
    let schema = read_json_at(
        &schemas_dir().join("methods"),
        "environment-verify-editor.schema.json",
    );
    let defs = schema
        .get("$defs")
        .expect("method schema has $defs")
        .clone();
    let sub = defs
        .get(format!("environment-verifyEditor{def_suffix}").as_str())
        .expect("sub schema exists")
        .clone();
    let combined = json!({ "allOf": [sub], "$defs": defs });
    jsonschema::validator_for(&combined).expect("sub schema compiles")
}

fn result_validator() -> Validator {
    sub_schema_validator("Result")
}

fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-editor-verify-wire-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// Sends one `environment.verifyEditor` query frame through the real host
/// loop and returns the response payload.
fn run_verify_frame(database: &Path, verifier: Option<EditorPathVerifier>, params: Value) -> Value {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-editor-verify",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-editor-verify",
            "correlationId": "corr-editor-verify",
            "kind": "query",
            "method": "environment.verifyEditor",
            "params": params,
        },
    });
    let mut output = Vec::new();
    run_provider_host_full(
        Cursor::new(format!("{frame}\n")),
        &mut output,
        database,
        None,
        None,
        None,
        None,
        None,
        None,
        verifier,
        None,
    )
    .expect("frame loop runs");
    let frames: Vec<Value> = String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect();
    frames[0]["payload"].clone()
}

fn verifier_of(verdict: EditorPathVerdict) -> EditorPathVerifier {
    Arc::new(move |_input: &Path| verdict.clone())
}

/// Captures the path the route hands to the primitive (verbatim-carry
/// assertion) and answers with the given verdict.
fn capturing_verifier(
    verdict: EditorPathVerdict,
    captured: Arc<Mutex<String>>,
) -> EditorPathVerifier {
    Arc::new(move |input: &Path| {
        *captured.lock().expect("capture lock") = input.to_string_lossy().into_owned();
        verdict.clone()
    })
}

fn verified_fixture() -> EditorPathVerdict {
    EditorPathVerdict::Verified(EditorPathIdentity {
        editor_root: "C:\\Hub\\2022.3.22f1".to_string(),
        exe_path: "C:\\Hub\\2022.3.22f1\\Editor\\Unity.exe".to_string(),
        version: "2022.3.22f1".to_string(),
        classification: vua_orchestrator::EditorClass::ProductionTarget,
        guidance_code: editor_target_codes::EDITOR_PRODUCTION_TARGET,
        china_distribution: false,
    })
}

#[test]
fn verified_verdict_maps_to_the_frozen_result_shape() {
    let root = unique_root("verified");
    let database = root.join("tasks.sqlite");
    let payload = run_verify_frame(
        &database,
        Some(verifier_of(verified_fixture())),
        json!({ "path": "C:\\Hub\\2022.3.22f1" }),
    );

    // A verified verdict is a normal successful response (never an error).
    assert_eq!(payload["ok"], true, "payload: {payload}");
    let result = &payload["value"];
    // The mapped result IS the frozen schema face (draft drift protection).
    let errors: Vec<String> = result_validator()
        .iter_errors(result)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "result violates the frozen schema: {errors:?}"
    );

    // Field-by-field: the six identity fields travel camelCase, the
    // classification stays the core snake_case closed set, and the envelope
    // schemaVersion is the row's OWN const (nail 3).
    assert_eq!(result["verdict"], "verified");
    assert_eq!(result["editorRoot"], "C:\\Hub\\2022.3.22f1");
    assert_eq!(result["exePath"], "C:\\Hub\\2022.3.22f1\\Editor\\Unity.exe");
    assert_eq!(result["version"], "2022.3.22f1");
    assert_eq!(result["classification"], "production_target");
    assert_eq!(
        result["guidanceCode"],
        "vua.env_managers.editor_production_target"
    );
    assert_eq!(result["chinaDistribution"], false);
    // Nail 3 keyed on the core-owned constant (never a private literal).
    assert_eq!(result["schemaVersion"], EDITOR_VERIFY_SCHEMA_VERSION);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn refusal_is_a_result_state_never_an_error_envelope() {
    // Nail 1 through the REAL system wiring: a nonexistent path refuses
    // deterministically — on Windows before any identity read
    // (target_missing); on non-Windows the system wiring itself refuses
    // (unsupported_platform). Either way the refusal must arrive as
    // ok:true + verdict:"refused", never as an application error envelope.
    let root = unique_root("refused");
    let database = root.join("tasks.sqlite");
    let missing = root.join("nowhere").display().to_string();
    let payload = run_verify_frame(&database, None, json!({ "path": missing }));

    assert_eq!(
        payload["ok"], true,
        "a refusal is a finding, not a failure: {payload}"
    );
    assert!(
        payload.get("error").is_none(),
        "no error envelope on a refusal: {payload}"
    );
    let result = &payload["value"];
    let errors: Vec<String> = result_validator()
        .iter_errors(result)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "refused result violates the frozen schema: {errors:?}"
    );
    assert_eq!(result["verdict"], "refused");
    #[cfg(windows)]
    assert_eq!(result["code"], "vua.editor_verify.target_missing");
    #[cfg(not(windows))]
    assert_eq!(result["code"], "vua.editor_verify.unsupported_platform");
    assert_eq!(result["exePath"], Value::Null);
    // Nail 3 keyed on the core-owned constant (never a private literal).
    assert_eq!(result["schemaVersion"], EDITOR_VERIFY_SCHEMA_VERSION);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn refusal_detail_travels_verbatim() {
    // Nail 2: the wire layer carries the primitive's diagnosis string
    // unchanged — raw resource text (quotes, pipes, whitespace) is never
    // re-interpreted.
    let root = unique_root("detail");
    let database = root.join("tasks.sqlite");
    let raw_detail = "version resource readable but no complete Unity editor version in: \"7.7.7x9\" | \"T 9 9\"";
    let verdict = EditorPathVerdict::Refused(EditorPathRefusal {
        exe_path: Some("C:\\fake\\2022.3.22f1\\Editor\\Unity.exe".to_string()),
        code: "vua.editor_verify.not_an_editor",
        detail: raw_detail.to_string(),
    });
    let payload = run_verify_frame(
        &database,
        Some(verifier_of(verdict)),
        json!({ "path": "C:\\fake\\2022.3.22f1" }),
    );

    assert_eq!(payload["ok"], true);
    let result = &payload["value"];
    assert_eq!(result["verdict"], "refused");
    assert_eq!(result["detail"], raw_detail, "detail must travel verbatim");
    assert_eq!(
        result["exePath"],
        "C:\\fake\\2022.3.22f1\\Editor\\Unity.exe"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn picked_path_travels_verbatim_to_the_primitive() {
    // Ruling point 3: all three accepted layouts travel verbatim — the
    // route normalizes nothing (normalization is the primitive's job) and
    // invents no path of its own.
    let root = unique_root("verbatim");
    let database = root.join("tasks.sqlite");
    let picked = "C:\\Program Files\\Unity Hub\\2022.3.6f1\\Editor";
    let captured = Arc::new(Mutex::new(String::new()));
    let verdict = EditorPathVerdict::Refused(EditorPathRefusal {
        exe_path: None,
        code: "vua.editor_verify.exe_missing",
        detail: "probe".to_string(),
    });
    let payload = run_verify_frame(
        &database,
        Some(capturing_verifier(verdict, captured.clone())),
        json!({ "path": picked }),
    );

    assert_eq!(payload["ok"], true);
    assert_eq!(
        *captured.lock().expect("capture lock"),
        picked,
        "the route must hand the picked path to the primitive unchanged"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn params_closed_set_violations_answer_validation_errors() {
    // Request-shape violations are typed validation errors — they are
    // NEVER verification refusals (a refusal needs the primitive to have
    // run; a shape violation means the request never reached it).
    let root = unique_root("params");
    let database = root.join("tasks.sqlite");
    let violations = vec![
        json!({}),                                  // missing path
        json!({ "Path": "C:\\x" }),                 // wrong key casing
        json!({ "path": "" }),                      // minLength 1
        json!({ "path": "C:\\x", "follow": true }), // speculative extra key
        json!({ "path": 42 }),                      // wrong type
        json!("C:\\x"),                             // params not an object
    ];
    for params in violations {
        let payload = run_verify_frame(&database, None, params.clone());
        assert_eq!(
            payload["ok"], false,
            "params {params} must be a validation error"
        );
        assert_eq!(
            payload["error"]["code"], "vua.environment.invalid_params",
            "params: {params}"
        );
        assert_eq!(
            payload["error"]["category"], "validation",
            "params: {params}"
        );
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn absence_code_is_never_a_refusal_code() {
    // Ruling point 5: the honest absence code is reserved for an unwired
    // route / unreachable primitive. Every refusal this route emits carries
    // a closed-set vua.editor_verify.* code — the absence code never
    // overloads a verification refusal (and this route has no absence path
    // at all: the primitive is a stateless direct call).
    let root = unique_root("absence");
    let database = root.join("tasks.sqlite");
    let refusals = vec![
        "vua.editor_verify.target_missing",
        "vua.editor_verify.exe_missing",
        "vua.editor_verify.identity_unreadable",
        "vua.editor_verify.not_an_editor",
        "vua.editor_verify.unsupported_platform",
    ];
    for code in refusals {
        let verdict = EditorPathVerdict::Refused(EditorPathRefusal {
            exe_path: Some("C:\\probe\\Unity.exe".to_string()),
            code,
            detail: "probe".to_string(),
        });
        let payload = run_verify_frame(
            &database,
            Some(verifier_of(verdict)),
            json!({ "path": "C:\\probe" }),
        );
        let emitted = payload["value"]["code"].as_str().expect("refusal code");
        assert_eq!(emitted, code);
        assert_ne!(
            emitted, ENVIRONMENT_VERIFY_UNAVAILABLE,
            "the absence code must never surface as a verification refusal"
        );
        assert!(
            emitted.starts_with("vua.editor_verify."),
            "refusal codes stay in the primitive's closed family: {emitted}"
        );
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn envelope_const_is_the_core_owned_row_version() {
    // Nail 3 anchor: the published constant IS the frozen draft schema's
    // envelope const — the constant and the schema cannot drift apart
    // silently (the result validators above enforce the schema face; this
    // pins the constant face).
    assert_eq!(EDITOR_VERIFY_SCHEMA_VERSION, "0.1");
}

#[test]
fn capability_row_declares_the_route_available() {
    // The vocabulary row face: the route is always wired (stateless direct
    // primitive, no service composition), so the capability row is
    // unconditionally available.
    let root = unique_root("caps");
    let database = root.join("tasks.sqlite");
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-caps",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-caps",
            "correlationId": "corr-caps",
            "kind": "query",
            "method": "application.getSnapshot",
            "params": {},
        },
    });
    let mut output = Vec::new();
    run_provider_host_full(
        Cursor::new(format!("{frame}\n")),
        &mut output,
        &database,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("frame loop runs");
    let frames: Vec<Value> = String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect();
    let operations = frames[0]["payload"]["value"]["capabilities"]["operations"]
        .as_array()
        .expect("operations array");
    assert!(
        operations.contains(&json!({
            "operationId": "environment.verifyEditor",
            "availability": "available"
        })),
        "capability row missing: {operations:?}"
    );
    let _ = fs::remove_dir_all(&root);
}
