//! Packages-ops v0.6 wire tests (proposal 027 F4 wiring, 2026-09-21): the
//! repository-lifecycle write face rides the real frame loop over the
//! frozen `schemas/packages-ops/v0.6/` word list — the three methods
//! `packages.enableRepo` {repoId} / `packages.disableRepo` {repoId} /
//! `packages.refreshRepo` {repoId} as nine-state task commands with their
//! terminal reflux, the minimal honest receipts (`enabled` {repoId echo} /
//! `disabled` {repoId echo} / `refreshed` {repoId echo + REQUIRED
//! `cacheUpdated`} — the ports answer unit or the one refresh-outcome
//! fact, nothing invented), the all-refusals-fold-into-`execution_failed`
//! projection with the original port code carried inside detail (the F4
//! adds NO guard; the fold stamps the v0.6 family const, the reused
//! `vua.vpm.*` codes never travel in the rejected `code` key), the closed
//! single-key param faces (a carried `confirmedDigest` or `projectPath` is
//! a shape violation), the per-method capability gate on the NEW defaulted
//! accessor `repo_lifecycle_capabilities()` with THREE INDEPENDENT bits
//! answered BEFORE submit (capability absence never reaches a task; the
//! gate is per method, never per face), and the `packages.repoLifecycleOps`
//! capability row (one row serving the three methods, available when ANY
//! bit is declared). The five frozen predecessor rows (v0.1 A1 removal,
//! v0.2 A2 install, v0.3 A3 registration, v0.4 A4 repository add/remove,
//! v0.5 A5 project creation) keep serving untouched through their own
//! suites — this file pins the v0.6 ROUTE and the cross-row isolation (the
//! A4 row still stamps "0.4"/`vua.packages-ops/v0.4` after this wiring).
//! The contract-side consumer loop is pinned by `packages_ops_consumer_v06`
//! and `packages_repos_consumer_v02`; the REAL backend consumption
//! (VUA-owned `.vua/vpm-repo-state.json` toggles + the etag-conditional
//! refresh) is the environment implementation-verification slice.
//! Everything runs against synthetic values — no machine-specific facts,
//! no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, ErrorCategory, PackageRequestV1, ProjectRef, RepoLifecycleCapabilities,
    RepoRefreshOutcomeV01, SqliteTaskStore, VpmBackend, VpmCapabilities,
};
use vua_project_manager::ManagerRoots;
use vua_provider_host::provider_host::{
    PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V06, PACKAGES_OPS_SCHEMA_VERSION_V06,
};
use vua_provider_host::{run_provider_host_full, ProjectOpsConfig};

fn read_ops_schema(name: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .join("../..")
        .join("schemas/packages-ops/v0.6")
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
        "vua-packages-ops-wire-v06-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// The F4 face never touches a project: the task authority comes with an
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
/// not the vrc-get library (the core-domain consumer tests pin the library
/// shapes against the same schemas). Three INDEPENDENT capability bits +
/// per-method failure injection — the gate is per method, never per face,
/// and the tests pin exactly that.
struct FakeLifecycle {
    enable_cap: bool,
    disable_cap: bool,
    refresh_cap: bool,
    remove_cap: bool,
    enable_failure: Option<AppErrorV1>,
    disable_failure: Option<AppErrorV1>,
    refresh_failure: Option<AppErrorV1>,
    refresh_cache_updated: bool,
}

impl FakeLifecycle {
    fn wired_all() -> Self {
        Self {
            enable_cap: true,
            disable_cap: true,
            refresh_cap: true,
            remove_cap: true,
            enable_failure: None,
            disable_failure: None,
            refresh_failure: None,
            refresh_cache_updated: true,
        }
    }

    fn refresh_only() -> Self {
        Self {
            enable_cap: false,
            disable_cap: false,
            refresh_cap: true,
            remove_cap: false,
            enable_failure: None,
            disable_failure: None,
            refresh_failure: None,
            refresh_cache_updated: true,
        }
    }

    fn declared_none() -> Self {
        Self {
            enable_cap: false,
            disable_cap: false,
            refresh_cap: false,
            remove_cap: false,
            enable_failure: None,
            disable_failure: None,
            refresh_failure: None,
            refresh_cache_updated: true,
        }
    }
}

impl VpmBackend for FakeLifecycle {
    fn name(&self) -> &'static str {
        "fake-lifecycle-wire"
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
    fn repo_lifecycle_capabilities(&self) -> RepoLifecycleCapabilities {
        RepoLifecycleCapabilities {
            enable_repo: self.enable_cap,
            disable_repo: self.disable_cap,
            refresh_repo: self.refresh_cap,
        }
    }
    fn repo_write_capabilities(&self) -> vua_orchestrator::RepoWriteCapabilities {
        vua_orchestrator::RepoWriteCapabilities {
            add_remote_repo: false,
            add_local_repo: false,
            remove_repo: self.remove_cap,
        }
    }
    fn remove_repo(&self, repo_id: &str) -> Result<(), AppErrorV1> {
        assert_eq!(repo_id, "repo.example", "verbatim repoId transport");
        Ok(())
    }
    fn enable_repo(&self, repo_id: &str) -> Result<(), AppErrorV1> {
        if let Some(error) = &self.enable_failure {
            return Err(error.clone());
        }
        assert_eq!(repo_id, "repo.example", "verbatim repoId transport");
        Ok(())
    }
    fn disable_repo(&self, repo_id: &str) -> Result<(), AppErrorV1> {
        if let Some(error) = &self.disable_failure {
            return Err(error.clone());
        }
        assert_eq!(repo_id, "repo.example", "verbatim repoId transport");
        Ok(())
    }
    fn refresh_repo(&self, repo_id: &str) -> Result<RepoRefreshOutcomeV01, AppErrorV1> {
        if let Some(error) = &self.refresh_failure {
            return Err(error.clone());
        }
        assert_eq!(repo_id, "repo.example", "verbatim repoId transport");
        Ok(RepoRefreshOutcomeV01 {
            cache_updated: self.refresh_cache_updated,
        })
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
        unreachable!("the F4 face has no preview arm")
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
        "frameId": "frame-packages-ops-v06",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages-ops-v06",
            "correlationId": "corr-packages-ops-v06",
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

/// Sends `application.getSnapshot` and extracts the
/// `packages.repoLifecycleOps` capability row.
fn lifecycle_capability_row(database: &Path, vpm: Option<Arc<dyn VpmBackend>>) -> Value {
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
        .find(|row| row["operationId"] == "packages.repoLifecycleOps")
        .cloned()
        .expect("the packages.repoLifecycleOps capability row exists")
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
        assert!(
            Instant::now() < deadline,
            "the lifecycle task did not finish"
        );
        drop(store);
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn lifecycle_params() -> Value {
    json!({ "repoId": "repo.example" })
}

#[test]
fn absent_lifecycle_wiring_answers_the_typed_honest_absence_and_unavailable_row() {
    let root = unique_root("absent");
    let database = root.join("tasks.sqlite");

    for method in [
        "packages.enableRepo",
        "packages.disableRepo",
        "packages.refreshRepo",
    ] {
        let payload = run_frame(&database, None, None, "command", method, lifecycle_params());
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

    let row = lifecycle_capability_row(&database, None);
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_enable_accepts_and_the_reflux_carries_the_minimal_enabled_receipt() {
    let validator = ops_result_validator();
    let root = unique_root("enable");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeLifecycle::wired_all())),
        "command",
        "packages.enableRepo",
        lifecycle_params(),
    );
    assert_eq!(payload["ok"], true, "the wired command accepts: {payload}");
    let acceptance = &payload["value"];
    assert_eq!(acceptance["operation"], "packages.enableRepo");
    assert_eq!(
        acceptance["schemaVersion"],
        PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V06
    );
    let task_id = acceptance["taskId"].as_str().expect("taskId").to_owned();
    assert!(!acceptance["correlationId"]
        .as_str()
        .unwrap_or_default()
        .is_empty());

    let done = wait_done(&database, &task_id);
    assert_eq!(
        done["schemaVersion"],
        PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V06
    );
    assert_eq!(done["operation"], "packages.enableRepo");
    assert!(
        validator.is_valid(&done),
        "receipt must match the frozen result schema: {done}"
    );
    let receipt = &done["result"];
    assert_eq!(receipt["schemaVersion"], PACKAGES_OPS_SCHEMA_VERSION_V06);
    assert_eq!(receipt["kind"], "enabled");
    // The minimal honest audit shape: the echo and NOTHING else — the new
    // state itself is read back on the packages-repos v0.2 subscription
    // face, never duplicated into the receipt (additionalProperties:false
    // makes an invented toggle timestamp or previous-state echo a schema
    // violation, not a nitpick).
    let receipt_keys: Vec<String> = receipt
        .as_object()
        .expect("receipt object")
        .keys()
        .cloned()
        .collect();
    assert_eq!(receipt_keys, vec!["schemaVersion", "kind", "repoId"]);
    assert_eq!(receipt["repoId"], "repo.example");

    let row = lifecycle_capability_row(&database, Some(Arc::new(FakeLifecycle::wired_all())));
    assert_eq!(row["availability"], "available");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_disable_accepts_and_the_reflux_carries_the_minimal_disabled_receipt() {
    let validator = ops_result_validator();
    let root = unique_root("disable");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeLifecycle::wired_all())),
        "command",
        "packages.disableRepo",
        lifecycle_params(),
    );
    assert_eq!(payload["ok"], true, "the wired command accepts: {payload}");
    let task_id = payload["value"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_owned();

    let done = wait_done(&database, &task_id);
    assert_eq!(
        done["schemaVersion"],
        PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V06
    );
    assert_eq!(done["operation"], "packages.disableRepo");
    assert!(validator.is_valid(&done), "{done}");
    let receipt = &done["result"];
    assert_eq!(receipt["kind"], "disabled");
    // The disabled row stays subscribed and listed — the receipt carries
    // the echo only, no state snapshot is invented.
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
fn wired_refresh_accepts_and_both_cache_updated_arms_are_successes() {
    let validator = ops_result_validator();
    let config = empty_ops_config();

    // Arm one: the etag-conditional fetch wrote a new cache file.
    let root = unique_root("refresh-updated");
    let database = root.join("tasks.sqlite");
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeLifecycle {
            refresh_cache_updated: true,
            ..FakeLifecycle::wired_all()
        })),
        "command",
        "packages.refreshRepo",
        lifecycle_params(),
    );
    assert_eq!(payload["ok"], true, "the wired command accepts: {payload}");
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert_eq!(
        done["schemaVersion"],
        PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V06
    );
    assert!(validator.is_valid(&done), "{done}");
    let receipt = &done["result"];
    assert_eq!(receipt["kind"], "refreshed");
    // The REQUIRED refresh-outcome fact — the receipt without it is
    // invalid by schema, and so is one padded with fetched-byte counts or
    // package lists.
    let receipt_keys: Vec<String> = receipt
        .as_object()
        .expect("receipt object")
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        receipt_keys,
        vec!["schemaVersion", "kind", "repoId", "cacheUpdated"]
    );
    assert_eq!(receipt["repoId"], "repo.example");
    assert_eq!(receipt["cacheUpdated"], true);
    fs::remove_dir_all(&root).ok();

    // Arm two: etag unchanged, "already up to date" — BOTH arms are
    // success, "no new data" is an outcome never an error.
    let root = unique_root("refresh-unchanged");
    let database = root.join("tasks.sqlite");
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeLifecycle {
            refresh_cache_updated: false,
            ..FakeLifecycle::wired_all()
        })),
        "command",
        "packages.refreshRepo",
        lifecycle_params(),
    );
    assert_eq!(
        payload["ok"], true,
        "the unchanged arm still ACCEPTS as a task"
    );
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert!(validator.is_valid(&done), "{done}");
    assert_eq!(done["result"]["kind"], "refreshed");
    assert_eq!(done["result"]["cacheUpdated"], false);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn lifecycle_param_violations_answer_invalid_params() {
    let root = unique_root("params");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let vpm = Some(Arc::new(FakeLifecycle::wired_all()) as Arc<dyn VpmBackend>);

    let violations: Vec<(&str, Value)> = vec![
        // The closed single-key set: absent keys are violations.
        ("packages.enableRepo", json!({})),
        ("packages.disableRepo", json!({})),
        ("packages.refreshRepo", json!({})),
        ("packages.enableRepo", json!({ "repoId": "" })),
        // A string fact is a string — a number is a violation.
        ("packages.disableRepo", json!({ "repoId": 42 })),
        // NO projectPath is taken — the lifecycle face addresses
        // SUBSCRIPTION rows only.
        (
            "packages.enableRepo",
            json!({ "repoId": "repo.example", "projectPath": "C:/some/project" }),
        ),
        // No digest, no confirmation chain — a carried confirmedDigest is
        // a shape violation (a state toggle diffs no pre-existing summary
        // and refresh IS the network act).
        (
            "packages.disableRepo",
            json!({ "repoId": "repo.example", "confirmedDigest": "fnv1a-whatever" }),
        ),
        (
            "packages.refreshRepo",
            json!({ "repoId": "repo.example", "confirmedDigest": "fnv1a-whatever" }),
        ),
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
fn lifecycle_gate_is_per_method_and_declared_none_stays_unavailable() {
    let root = unique_root("declared-none");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let none = Some(Arc::new(FakeLifecycle::declared_none()) as Arc<dyn VpmBackend>);

    for method in [
        "packages.enableRepo",
        "packages.disableRepo",
        "packages.refreshRepo",
    ] {
        let payload = run_frame(
            &database,
            Some(&config),
            none.clone(),
            "command",
            method,
            lifecycle_params(),
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

    let row = lifecycle_capability_row(&database, none);
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn three_independent_bits_serve_a_subset_backend() {
    // A backend may serve a SUBSET of the face: with only the refresh bit
    // declared the row is available (ANY bit), the two toggle methods
    // answer the generic capability-missing arm BEFORE submit, and
    // refreshRepo accepts and refluxes normally — the gate is per method,
    // never per face.
    let validator = ops_result_validator();
    let root = unique_root("subset");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let refresh_only = Some(Arc::new(FakeLifecycle::refresh_only()) as Arc<dyn VpmBackend>);

    for method in ["packages.enableRepo", "packages.disableRepo"] {
        let payload = run_frame(
            &database,
            Some(&config),
            refresh_only.clone(),
            "command",
            method,
            lifecycle_params(),
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
        refresh_only.clone(),
        "command",
        "packages.refreshRepo",
        lifecycle_params(),
    );
    assert_eq!(
        payload["ok"], true,
        "the served subset method accepts: {payload}"
    );
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert!(validator.is_valid(&done), "{done}");
    assert_eq!(done["result"]["kind"], "refreshed");

    let row = lifecycle_capability_row(&database, refresh_only);
    assert_eq!(
        row["availability"], "available",
        "ANY declared bit keeps the row honest"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn lifecycle_refusals_fold_into_execution_failed_with_port_provenance() {
    let validator = ops_result_validator();
    let config = empty_ops_config();

    // enableRepo → repo_not_found (the honest failure mode of an unknown
    // or drifting id — never a digest ritual).
    let root = unique_root("not-found");
    let database = root.join("tasks.sqlite");
    let vpm = FakeLifecycle {
        enable_failure: Some(AppErrorV1::new(
            "vua.vpm.repo_not_found",
            ErrorCategory::Unavailable,
            "errors.vpm.repoNotFound",
            "corr-fake",
        )),
        ..FakeLifecycle::wired_all()
    };
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.enableRepo",
        lifecycle_params(),
    );
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert!(
        validator.is_valid(&done),
        "the refusal is a frozen result document: {done}"
    );
    let rejected = &done["result"];
    assert_eq!(rejected["kind"], "rejected");
    assert_eq!(rejected["guard"], "execution_failed");
    assert_eq!(rejected["code"], "vua.packages.execution_failed");
    let detail = rejected["detail"].as_str().expect("detail");
    assert!(detail.contains("vua.vpm.repo_not_found"), "{detail}");
    fs::remove_dir_all(&root).ok();

    // disableRepo → repo_write_failed (the VUA-owned state-file write-back).
    let root = unique_root("write-failed");
    let database = root.join("tasks.sqlite");
    let vpm = FakeLifecycle {
        disable_failure: Some(AppErrorV1::new(
            "vua.vpm.repo_write_failed",
            ErrorCategory::ExternalFailure,
            "errors.vpm.repoWriteFailed",
            "corr-fake",
        )),
        ..FakeLifecycle::wired_all()
    };
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.disableRepo",
        lifecycle_params(),
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

    // refreshRepo → repo_fetch_failed (the inherent network segment).
    let root = unique_root("fetch-failed");
    let database = root.join("tasks.sqlite");
    let vpm = FakeLifecycle {
        refresh_failure: Some(AppErrorV1::new(
            "vua.vpm.repo_fetch_failed",
            ErrorCategory::ExternalFailure,
            "errors.vpm.repoFetchFailed",
            "corr-fake",
        )),
        ..FakeLifecycle::wired_all()
    };
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.refreshRepo",
        lifecycle_params(),
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
}

#[test]
fn unimplemented_lifecycle_port_capability_missing_folds_into_execution_failed() {
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
        fn repo_lifecycle_capabilities(&self) -> RepoLifecycleCapabilities {
            RepoLifecycleCapabilities {
                enable_repo: true,
                disable_repo: true,
                refresh_repo: true,
            }
        }
        fn preview_install(
            &self,
            _project: &ProjectRef,
            _packages: &[PackageRequestV1],
        ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
            unreachable!("the F4 face has no preview arm")
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
        "packages.enableRepo",
        lifecycle_params(),
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
fn envelope_consts_are_detectable_and_match_the_frozen_schema_consts() {
    // The A3/A4/A5/F2/F3 precedent closed at the wiring batch: the envelope
    // const and the family const are named HERE and pinned against the
    // frozen schema consts — a consumer can detect the row version from the
    // wire alone (the route stamps both consts verbatim, never literals).
    let command = read_ops_schema("command.schema.json");
    let result = read_ops_schema("result.schema.json");
    assert_eq!(
        command["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V06),
        "the envelope const equals the frozen command schema's const"
    );
    assert_eq!(PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V06, "0.6");
    assert_eq!(
        result["properties"]["result"]["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_OPS_SCHEMA_VERSION_V06),
        "the family const equals the frozen result schema's const"
    );
    assert_eq!(PACKAGES_OPS_SCHEMA_VERSION_V06, "vua.packages-ops/v0.6");
}

#[test]
fn the_frozen_v04_row_keeps_serving_untouched_beside_the_v06_row() {
    // Six word-face generations served side by side: after this wiring the
    // A4 removeRepo route still answers through its own "0.4" envelope and
    // its own `vua.packages-ops/v0.4` family const — the v0.6 row never
    // re-stamps a predecessor row (the cross-row isolation pin).
    let root = unique_root("v04-untouched");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeLifecycle::wired_all())),
        "command",
        "packages.removeRepo",
        lifecycle_params(),
    );
    assert_eq!(payload["ok"], true, "{payload}");
    assert_eq!(
        payload["value"]["schemaVersion"], "0.4",
        "the A4 envelope stays 0.4"
    );
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert_eq!(done["result"]["schemaVersion"], "vua.packages-ops/v0.4");
    assert_eq!(done["result"]["kind"], "removed");
    fs::remove_dir_all(&root).ok();
}
