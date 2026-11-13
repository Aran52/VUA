//! Packages-ops v0.4 wire tests (proposal 026 A4 wiring, 2026-09-19): the
//! repository add/remove write face rides the real frame loop over the
//! frozen `schemas/packages-ops/v0.4/` word list — the three methods
//! `packages.addRemoteRepo` {url, name} / `packages.addLocalRepo`
//! {path, name} / `packages.removeRepo` {repoId} as nine-state task
//! commands with their terminal reflux, the minimal honest `repoReceipt`
//! (remote {url, name echo} / local {path, name echo}) and `removed`
//! {repoId echo} receipts (the request echo and nothing else — the ports
//! answer unit, no payload invented), the all-refusals-fold-into-
//! `execution_failed` projection with the original port code carried
//! inside detail (A4 adds NO guard; the add face claims NO idempotence —
//! where the backend refuses a duplicate the wire answers the refusal
//! honestly), the closed param faces (a carried `confirmedDigest` or
//! `projectPath` is a shape violation), the per-method capability gate on
//! the NEW defaulted accessor `repo_write_capabilities()` with THREE
//! INDEPENDENT bits answered BEFORE submit (capability absence never
//! reaches a task; the gate is per method, never per face), and the
//! `packages.repoOps` capability row (one row serving the three methods,
//! available when ANY bit is declared). The frozen v0.1 A1 removal row,
//! v0.2 A2 install row and v0.3 A3 registration row keep serving
//! untouched through `packages_ops_wire.rs`, `packages_ops_wire_v02.rs`
//! and `packages_ops_wire_v03.rs`; this file pins the v0.4 ROUTE. The
//! contract-side consumer loop is pinned by `packages_ops_consumer_v04`;
//! the real backend consumption is the environment
//! implementation-verification slice. Everything runs against synthetic
//! values — no machine-specific facts, no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, ErrorCategory, PackageRequestV1, ProjectRef, RepoWriteCapabilities,
    SqliteTaskStore, VpmBackend, VpmCapabilities,
};
use vua_project_manager::ManagerRoots;
use vua_provider_host::{run_provider_host_full, ProjectOpsConfig};

fn read_ops_schema(name: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .join("../..")
        .join("schemas/packages-ops/v0.4")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn ops_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_schema("result.schema.json")).unwrap()
}

fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-packages-ops-wire-v04-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// The A4 face never touches a project: the task authority comes with an
/// EMPTY registration world on purpose — the routes perform no
/// registered-project check, so an empty world must change nothing.
fn empty_ops_config() -> ProjectOpsConfig {
    ProjectOpsConfig {
        vcc_settings_candidates: Vec::new(),
        manager_roots: ManagerRoots {
            alcom_settings_candidates: Vec::new(),
        },
        editor_roots: Vec::new(),
    }
}

/// A deterministic fake engine: the wire test pins routing and projection,
/// not the vrc-get library (the core-domain consumer test
/// `packages_ops_consumer_v04` pins the library shapes against the same
/// schemas). Three INDEPENDENT capability bits + per-method failure
/// injection — the gate is per method, never per face, and the tests pin
/// exactly that.
struct FakeRepo {
    add_remote_cap: bool,
    add_local_cap: bool,
    remove_cap: bool,
    add_remote_failure: Option<AppErrorV1>,
    add_local_failure: Option<AppErrorV1>,
    remove_failure: Option<AppErrorV1>,
}

impl FakeRepo {
    fn wired_all() -> Self {
        Self {
            add_remote_cap: true,
            add_local_cap: true,
            remove_cap: true,
            add_remote_failure: None,
            add_local_failure: None,
            remove_failure: None,
        }
    }

    fn remove_only() -> Self {
        Self {
            add_remote_cap: false,
            add_local_cap: false,
            remove_cap: true,
            add_remote_failure: None,
            add_local_failure: None,
            remove_failure: None,
        }
    }

    fn declared_none() -> Self {
        Self {
            add_remote_cap: false,
            add_local_cap: false,
            remove_cap: false,
            add_remote_failure: None,
            add_local_failure: None,
            remove_failure: None,
        }
    }
}

impl VpmBackend for FakeRepo {
    fn name(&self) -> &'static str {
        "fake-repo-wire"
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
    fn repo_write_capabilities(&self) -> RepoWriteCapabilities {
        RepoWriteCapabilities {
            add_remote_repo: self.add_remote_cap,
            add_local_repo: self.add_local_cap,
            remove_repo: self.remove_cap,
        }
    }
    fn add_remote_repo(&self, url: &str, name: &str) -> Result<(), AppErrorV1> {
        if let Some(error) = &self.add_remote_failure {
            return Err(error.clone());
        }
        // The facts must reach the backend verbatim — the wire never
        // rewrites, resolves or re-encodes the request's url/name.
        assert_eq!(
            url, "https://example.com/vpm.json",
            "verbatim url transport"
        );
        assert_eq!(name, "Example Repo", "verbatim name transport");
        Ok(())
    }
    fn add_local_repo(&self, path: &Path, name: &str) -> Result<(), AppErrorV1> {
        if let Some(error) = &self.add_local_failure {
            return Err(error.clone());
        }
        assert_eq!(
            path.file_name().and_then(|part| part.to_str()),
            Some("repo-dir"),
            "verbatim path transport"
        );
        assert_eq!(name, "Local Repo", "verbatim name transport");
        Ok(())
    }
    fn remove_repo(&self, repo_id: &str) -> Result<(), AppErrorV1> {
        if let Some(error) = &self.remove_failure {
            return Err(error.clone());
        }
        assert_eq!(repo_id, "repo.example", "verbatim repoId transport");
        Ok(())
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
        unreachable!("the A4 face has no preview arm")
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        _confirmed_digest: &str,
    ) -> Result<serde_json::Value, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn create_project(
        &self,
        _parent: &std::path::Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
}

/// Runs one frame through the real host loop and returns the response
/// payload.
fn run_frame(
    database: &Path,
    config: Option<&ProjectOpsConfig>,
    vpm: Option<Arc<dyn VpmBackend>>,
    frame_kind: &str,
    method: &str,
    params: Value,
) -> Value {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-packages-ops-v04",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages-ops-v04",
            "correlationId": "corr-packages-ops-v04",
            "kind": frame_kind,
            "method": method,
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
        config.cloned(),
        None,
        None,
        vpm,
    )
    .expect("frame loop runs");
    let frames: Vec<Value> = String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect();
    frames[0]["payload"].clone()
}

/// Sends `application.getSnapshot` and extracts the `packages.repoOps`
/// capability row.
fn repo_ops_capability_row(database: &Path, vpm: Option<Arc<dyn VpmBackend>>) -> Value {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-snapshot",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-snapshot",
            "correlationId": "corr-snapshot",
            "kind": "query",
            "method": "application.getSnapshot",
            "params": {},
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
        None,
        vpm,
    )
    .expect("frame loop runs");
    let frames: Vec<Value> = String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect();
    let payload = &frames[0]["payload"]["value"];
    payload["capabilities"]["operations"]
        .as_array()
        .expect("operations array")
        .iter()
        .find(|row| row["operationId"] == "packages.repoOps")
        .cloned()
        .expect("the packages.repoOps capability row exists")
}

/// Polls the tasked command to its terminal state and returns the Done
/// payload (the frozen result envelope travels inside it). A typed guard
/// refusal still completes the task: the state stays `succeeded`.
fn wait_done(database_path: &Path, task_id: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let store = SqliteTaskStore::open(database_path).expect("store opens");
        let task = store
            .task(task_id)
            .expect("store readable")
            .expect("task exists");
        if task.state.is_terminal() {
            assert_eq!(
                serde_json::to_value(task.state).unwrap(),
                "succeeded",
                "a typed guard refusal still completes the task: {:?}",
                task.error
            );
            return task.result.expect("done payload");
        }
        assert!(Instant::now() < deadline, "the repo task did not finish");
        drop(store);
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn add_remote_params() -> Value {
    json!({ "url": "https://example.com/vpm.json", "name": "Example Repo" })
}

fn add_local_params() -> Value {
    json!({ "path": "C:/anywhere/repo-dir", "name": "Local Repo" })
}

fn remove_params() -> Value {
    json!({ "repoId": "repo.example" })
}

#[test]
fn absent_repo_wiring_answers_the_typed_honest_absence_and_unavailable_capability() {
    let root = unique_root("absent");
    let database = root.join("tasks.sqlite");

    for (method, params) in [
        ("packages.addRemoteRepo", add_remote_params()),
        ("packages.addLocalRepo", add_local_params()),
        ("packages.removeRepo", remove_params()),
    ] {
        let payload = run_frame(&database, None, None, "command", method, params);
        assert_eq!(payload["ok"], false, "{method}: absence is a typed failure");
        assert_eq!(
            payload["error"]["code"], "vua.packages.unavailable",
            "{method}"
        );
        assert_eq!(payload["error"]["category"], "unavailable", "{method}");
        assert!(
            payload.get("result").is_none(),
            "{method}: absence never fabricates a receipt"
        );
    }

    let row = repo_ops_capability_row(&database, None);
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_add_remote_accepts_and_the_reflux_carries_the_minimal_remote_receipt() {
    let validator = ops_result_validator();
    let root = unique_root("add-remote");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeRepo::wired_all())),
        "command",
        "packages.addRemoteRepo",
        add_remote_params(),
    );
    assert_eq!(payload["ok"], true, "the wired command accepts: {payload}");
    let acceptance = &payload["value"];
    assert_eq!(acceptance["operation"], "packages.addRemoteRepo");
    assert_eq!(acceptance["schemaVersion"], "0.4");
    let task_id = acceptance["taskId"].as_str().expect("taskId").to_owned();
    assert!(!acceptance["correlationId"]
        .as_str()
        .unwrap_or_default()
        .is_empty());

    let done = wait_done(&database, &task_id);
    assert_eq!(done["schemaVersion"], "0.4");
    assert_eq!(done["operation"], "packages.addRemoteRepo");
    assert!(
        validator.is_valid(&done),
        "receipt must match the frozen result schema: {done}"
    );
    let receipt = &done["result"];
    assert_eq!(receipt["schemaVersion"], "vua.packages-ops/v0.4");
    assert_eq!(receipt["kind"], "repoReceipt");
    assert_eq!(receipt["repoType"], "remote");
    // The minimal honest audit shape: the request echo and NOTHING else —
    // additionalProperties:false makes an invented fact (fetched-manifest
    // contents, timestamp, row position) a schema violation, not a nitpick.
    let receipt_keys: Vec<String> = receipt
        .as_object()
        .expect("receipt object")
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        receipt_keys,
        vec!["schemaVersion", "kind", "repoType", "url", "name"]
    );
    assert_eq!(receipt["url"], "https://example.com/vpm.json");
    assert_eq!(receipt["name"], "Example Repo");

    let row = repo_ops_capability_row(&database, Some(Arc::new(FakeRepo::wired_all())));
    assert_eq!(row["availability"], "available");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_add_local_accepts_and_the_reflux_carries_the_minimal_local_receipt() {
    let validator = ops_result_validator();
    let root = unique_root("add-local");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeRepo::wired_all())),
        "command",
        "packages.addLocalRepo",
        add_local_params(),
    );
    assert_eq!(payload["ok"], true, "the wired command accepts: {payload}");
    let task_id = payload["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();

    let done = wait_done(&database, &task_id);
    assert_eq!(done["schemaVersion"], "0.4");
    assert_eq!(done["operation"], "packages.addLocalRepo");
    assert!(validator.is_valid(&done), "{done}");
    let receipt = &done["result"];
    assert_eq!(receipt["kind"], "repoReceipt");
    assert_eq!(receipt["repoType"], "local");
    let receipt_keys: Vec<String> = receipt
        .as_object()
        .expect("receipt object")
        .keys()
        .cloned()
        .collect();
    // The local variant's key set is disjoint from the remote variant and
    // every prior receipt arm.
    assert_eq!(
        receipt_keys,
        vec!["schemaVersion", "kind", "repoType", "path", "name"]
    );
    assert_eq!(receipt["path"], "C:/anywhere/repo-dir");
    assert_eq!(receipt["name"], "Local Repo");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_remove_accepts_and_the_reflux_carries_the_minimal_removed_receipt() {
    let validator = ops_result_validator();
    let root = unique_root("remove");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeRepo::wired_all())),
        "command",
        "packages.removeRepo",
        remove_params(),
    );
    assert_eq!(payload["ok"], true, "the wired command accepts: {payload}");
    let task_id = payload["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();

    let done = wait_done(&database, &task_id);
    assert_eq!(done["schemaVersion"], "0.4");
    assert_eq!(done["operation"], "packages.removeRepo");
    assert!(validator.is_valid(&done), "{done}");
    let receipt = &done["result"];
    assert_eq!(receipt["kind"], "removed");
    // The echo IS the audit link: the removed row's id and NOTHING else —
    // no removed-row snapshot is invented (a row may carry absent-id facts
    // this face never round-trips).
    let receipt_keys: Vec<String> = receipt
        .as_object()
        .expect("receipt object")
        .keys()
        .cloned()
        .collect();
    assert_eq!(receipt_keys, vec!["schemaVersion", "kind", "repoId"]);
    assert_eq!(receipt["repoId"], "repo.example");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn repo_param_violations_answer_invalid_params() {
    let root = unique_root("params");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let vpm = Some(Arc::new(FakeRepo::wired_all()) as Arc<dyn VpmBackend>);

    let violations: Vec<(&str, Value)> = vec![
        // addRemoteRepo: the closed two-key set — absent keys are violations.
        ("packages.addRemoteRepo", json!({})),
        (
            "packages.addRemoteRepo",
            json!({ "url": "https://example.com/vpm.json" }),
        ),
        ("packages.addRemoteRepo", json!({ "name": "Example Repo" })),
        // Empty url / empty name are violations (non-empty string facts).
        (
            "packages.addRemoteRepo",
            json!({ "url": "", "name": "Example Repo" }),
        ),
        (
            "packages.addRemoteRepo",
            json!({ "url": "https://example.com/vpm.json", "name": "" }),
        ),
        // NO projectPath is taken — the subscription face writes the
        // backend's ISOLATED environment only.
        (
            "packages.addRemoteRepo",
            json!({
                "url": "https://example.com/vpm.json",
                "name": "Example Repo",
                "projectPath": "C:/some/project",
            }),
        ),
        // No digest, no confirmation chain — a carried confirmedDigest is
        // a shape violation.
        (
            "packages.addRemoteRepo",
            json!({
                "url": "https://example.com/vpm.json",
                "name": "Example Repo",
                "confirmedDigest": "fnv1a-whatever",
            }),
        ),
        // addLocalRepo: the same closed-set law.
        ("packages.addLocalRepo", json!({})),
        (
            "packages.addLocalRepo",
            json!({ "path": "", "name": "Local Repo" }),
        ),
        (
            "packages.addLocalRepo",
            json!({
                "path": "C:/anywhere/repo-dir",
                "name": "Local Repo",
                "confirmedDigest": "fnv1a-whatever",
            }),
        ),
        // removeRepo: the closed single-key set.
        ("packages.removeRepo", json!({})),
        ("packages.removeRepo", json!({ "repoId": "" })),
        (
            "packages.removeRepo",
            json!({ "repoId": "repo.example", "projectPath": "C:/some/project" }),
        ),
        // A string fact is a string — a number is a violation.
        ("packages.removeRepo", json!({ "repoId": 42 })),
    ];
    for (method, params) in violations {
        let payload = run_frame(
            &database,
            Some(&config),
            vpm.clone(),
            "command",
            method,
            params,
        );
        assert_eq!(
            payload["ok"], false,
            "{method}: violations are typed failures"
        );
        assert_eq!(
            payload["error"]["code"], "vua.packages.invalid_params",
            "{method}: {payload}"
        );
        assert_eq!(payload["error"]["category"], "validation", "{method}");
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn repo_capability_gate_is_per_method_and_declared_none_stays_unavailable() {
    let root = unique_root("declared-none");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let none = Some(Arc::new(FakeRepo::declared_none()) as Arc<dyn VpmBackend>);

    for (method, params) in [
        ("packages.addRemoteRepo", add_remote_params()),
        ("packages.addLocalRepo", add_local_params()),
        ("packages.removeRepo", remove_params()),
    ] {
        let payload = run_frame(
            &database,
            Some(&config),
            none.clone(),
            "command",
            method,
            params,
        );
        assert_eq!(payload["ok"], false, "{method}");
        // The generic capability-missing arm, answered BEFORE submit.
        assert_eq!(
            payload["error"]["code"], "vua.vpm.capability_missing",
            "{method}"
        );
        assert_eq!(payload["error"]["category"], "unavailable", "{method}");
        assert!(
            payload["value"].is_null(),
            "{method}: capability absence never reaches a task"
        );
    }

    let row = repo_ops_capability_row(&database, none);
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn three_independent_bits_serve_a_subset_backend() {
    // A backend may serve a SUBSET of the face: with only the remove bit
    // declared the row is available (ANY bit), the two add methods answer
    // the generic capability-missing arm BEFORE submit, and removeRepo
    // accepts and refluxes normally — the gate is per method, never per
    // face.
    let validator = ops_result_validator();
    let root = unique_root("subset");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let remove_only = Some(Arc::new(FakeRepo::remove_only()) as Arc<dyn VpmBackend>);

    for (method, params) in [
        ("packages.addRemoteRepo", add_remote_params()),
        ("packages.addLocalRepo", add_local_params()),
    ] {
        let payload = run_frame(
            &database,
            Some(&config),
            remove_only.clone(),
            "command",
            method,
            params,
        );
        assert_eq!(payload["ok"], false, "{method}");
        assert_eq!(
            payload["error"]["code"], "vua.vpm.capability_missing",
            "{method}"
        );
        assert!(
            payload["value"].is_null(),
            "{method}: absence never reaches a task"
        );
    }

    let payload = run_frame(
        &database,
        Some(&config),
        remove_only.clone(),
        "command",
        "packages.removeRepo",
        remove_params(),
    );
    assert_eq!(
        payload["ok"], true,
        "the served subset method accepts: {payload}"
    );
    let task_id = payload["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();
    let done = wait_done(&database, &task_id);
    assert!(validator.is_valid(&done), "{done}");
    assert_eq!(done["result"]["kind"], "removed");

    let row = repo_ops_capability_row(&database, remove_only);
    assert_eq!(
        row["availability"], "available",
        "ANY declared bit keeps the row honest"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn repo_invalid_refusal_folds_into_execution_failed_with_port_provenance() {
    let validator = ops_result_validator();
    let root = unique_root("invalid");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let vpm = FakeRepo {
        add_remote_failure: Some(AppErrorV1::new(
            "vua.vpm.repo_invalid",
            ErrorCategory::Validation,
            "errors.vpm.repoInvalid",
            "corr-fake",
        )),
        ..FakeRepo::wired_all()
    };

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.addRemoteRepo",
        add_remote_params(),
    );
    let task_id = payload["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();

    let done = wait_done(&database, &task_id);
    assert!(
        validator.is_valid(&done),
        "the refusal is a frozen result document: {done}"
    );
    let rejected = &done["result"];
    assert_eq!(rejected["kind"], "rejected");
    // A4 adds NO guard: the duplicate/shape refusal folds into the frozen
    // execution_failed guard — the add face claims NO idempotence, no
    // idempotent success is invented where the backend refuses.
    assert_eq!(rejected["guard"], "execution_failed");
    assert_eq!(rejected["code"], "vua.packages.execution_failed");
    // Honest provenance: the original port code travels inside detail —
    // the reused vua.vpm.* code never enters the code key.
    let detail = rejected["detail"].as_str().expect("detail");
    assert!(detail.contains("vua.vpm.repo_invalid"), "{detail}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn not_found_fetch_and_write_failures_fold_into_execution_failed_with_provenance() {
    let validator = ops_result_validator();
    let config = empty_ops_config();

    // removeRepo → repo_not_found (the honest failure mode of an unknown
    // or drifting id — never a digest ritual).
    let root = unique_root("not-found");
    let database = root.join("tasks.sqlite");
    let vpm = FakeRepo {
        remove_failure: Some(AppErrorV1::new(
            "vua.vpm.repo_not_found",
            ErrorCategory::Unavailable,
            "errors.vpm.repoNotFound",
            "corr-fake",
        )),
        ..FakeRepo::wired_all()
    };
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.removeRepo",
        remove_params(),
    );
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert!(validator.is_valid(&done), "{done}");
    let rejected = &done["result"];
    assert_eq!(rejected["kind"], "rejected");
    assert_eq!(rejected["guard"], "execution_failed");
    assert_eq!(rejected["code"], "vua.packages.execution_failed");
    assert!(rejected["detail"]
        .as_str()
        .unwrap()
        .contains("vua.vpm.repo_not_found"));
    fs::remove_dir_all(&root).ok();

    // addRemoteRepo → repo_fetch_failed (the inherent network segment).
    let root = unique_root("fetch-failed");
    let database = root.join("tasks.sqlite");
    let vpm = FakeRepo {
        add_remote_failure: Some(AppErrorV1::new(
            "vua.vpm.repo_fetch_failed",
            ErrorCategory::ExternalFailure,
            "errors.vpm.repoFetchFailed",
            "corr-fake",
        )),
        ..FakeRepo::wired_all()
    };
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.addRemoteRepo",
        add_remote_params(),
    );
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert!(validator.is_valid(&done), "{done}");
    let rejected = &done["result"];
    assert_eq!(rejected["guard"], "execution_failed");
    assert!(rejected["detail"]
        .as_str()
        .unwrap()
        .contains("vua.vpm.repo_fetch_failed"));
    fs::remove_dir_all(&root).ok();

    // addLocalRepo → repo_write_failed (the isolated-environment write-back).
    let root = unique_root("write-failed");
    let database = root.join("tasks.sqlite");
    let vpm = FakeRepo {
        add_local_failure: Some(AppErrorV1::new(
            "vua.vpm.repo_write_failed",
            ErrorCategory::ExternalFailure,
            "errors.vpm.repoWriteFailed",
            "corr-fake",
        )),
        ..FakeRepo::wired_all()
    };
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.addLocalRepo",
        add_local_params(),
    );
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert!(validator.is_valid(&done), "{done}");
    let rejected = &done["result"];
    assert_eq!(rejected["guard"], "execution_failed");
    assert!(rejected["detail"]
        .as_str()
        .unwrap()
        .contains("vua.vpm.repo_write_failed"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn unimplemented_repo_port_capability_missing_folds_into_execution_failed() {
    // A backend that declares the capability bit but never overrode the
    // port method answers the trait default (capability_missing) INSIDE
    // the task — the wire projection folds it like any other refusal:
    // the guard set stays frozen, the provenance stays honest.
    struct DeclaredButUnimplemented;
    impl VpmBackend for DeclaredButUnimplemented {
        fn name(&self) -> &'static str {
            "fake-declared-unimplemented"
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
        fn repo_write_capabilities(&self) -> RepoWriteCapabilities {
            RepoWriteCapabilities {
                add_remote_repo: true,
                add_local_repo: true,
                remove_repo: true,
            }
        }
        fn preview_install(
            &self,
            _project: &ProjectRef,
            _packages: &[PackageRequestV1],
        ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
            unreachable!("the A4 face has no preview arm")
        }
        fn apply_install(
            &self,
            _project: &ProjectRef,
            _packages: &[PackageRequestV1],
            _confirmed_digest: &str,
        ) -> Result<serde_json::Value, AppErrorV1> {
            unreachable!("not exercised in this suite")
        }
        fn create_project(
            &self,
            _parent: &std::path::Path,
            _name: &str,
            _template: Option<&str>,
        ) -> Result<ProjectRef, AppErrorV1> {
            unreachable!("not exercised in this suite")
        }
    }

    let root = unique_root("unimplemented");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(DeclaredButUnimplemented)),
        "command",
        "packages.removeRepo",
        remove_params(),
    );
    let task_id = payload["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();

    let done = wait_done(&database, &task_id);
    let rejected = &done["result"];
    assert_eq!(rejected["kind"], "rejected");
    assert_eq!(rejected["guard"], "execution_failed");
    assert_eq!(rejected["code"], "vua.packages.execution_failed");
    let detail = rejected["detail"].as_str().expect("detail");
    assert!(detail.contains("vua.vpm.capability_missing"), "{detail}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn repo_add_claims_no_idempotence_the_duplicate_refusal_travels_honestly() {
    // Unlike the A3 registration face (the library's AlreadyAdded
    // collapsing into one success fact), the A4 add face claims NO
    // idempotence: the library's add guards REFUSE duplicates, and the
    // wire answers that refusal honestly as an execution_failed rejected
    // carrying the original port code — never an invented idempotent
    // success.
    let validator = ops_result_validator();
    let root = unique_root("no-idempotence");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    // Round one: the backend accepts the first add.
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeRepo::wired_all())),
        "command",
        "packages.addLocalRepo",
        add_local_params(),
    );
    assert_eq!(payload["ok"], true);
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert_eq!(done["result"]["kind"], "repoReceipt");
    assert!(validator.is_valid(&done), "{done}");

    // Round two: the backend refuses the duplicate (repo_invalid, the
    // frozen guard's authority) — the wire carries the refusal verbatim
    // as a rejected, not a second identical receipt.
    let vpm = FakeRepo {
        add_local_failure: Some(AppErrorV1::new(
            "vua.vpm.repo_invalid",
            ErrorCategory::Validation,
            "errors.vpm.repoInvalid",
            "corr-fake",
        )),
        ..FakeRepo::wired_all()
    };
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.addLocalRepo",
        add_local_params(),
    );
    assert_eq!(payload["ok"], true, "the duplicate still ACCEPTS as a task");
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert!(validator.is_valid(&done), "{done}");
    let rejected = &done["result"];
    assert_eq!(
        rejected["kind"], "rejected",
        "the duplicate refusal travels honestly"
    );
    assert_eq!(rejected["guard"], "execution_failed");
    assert!(rejected["detail"]
        .as_str()
        .unwrap()
        .contains("vua.vpm.repo_invalid"));
    fs::remove_dir_all(&root).ok();
}
