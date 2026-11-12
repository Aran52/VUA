//! Packages-ops v0.3 wire tests (proposal 026 A3 wiring, 2026-09-19): the
//! local-package registration write face rides the real frame loop over
//! the frozen `schemas/packages-ops/v0.3/` word list — the single method
//! `packages.registerLocalPackage` as a nine-state task command with its
//! terminal reflux, the minimal honest `registered` receipt (the request
//! echo and nothing else), the all-refusals-fold-into-`execution_failed`
//! projection with the original port code carried inside detail (A3 adds
//! NO guard), the closed single-key params face `{packageRoot}` (a
//! carried `confirmedDigest` or `projectPath` is a shape violation), the
//! capability gate on the NEW defaulted accessor
//! `register_capabilities().register_local_package` answered BEFORE
//! submit (capability absence never reaches a task), and the
//! `packages.registerOps` capability row. The frozen v0.1 A1 removal row
//! keeps serving untouched through `packages_ops_wire.rs` and the v0.2
//! A2 install row through `packages_ops_wire_v02.rs`; this file pins the
//! v0.3 ROUTE. The contract-side consumer loop (schema vectors plus the
//! port-to-wire projection shapes) is pinned by
//! `packages_ops_consumer_v03.rs`; the real backend consumption is the
//! environment implementation-verification slice. Everything runs against
//! synthetic directory trees — no machine-specific facts, no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, ErrorCategory, PackageRequestV1, ProjectRef, RegisterCapabilities, SqliteTaskStore,
    VpmBackend, VpmCapabilities,
};
use vua_project_manager::ManagerRoots;
use vua_provider_host::{run_provider_host_full, ProjectOpsConfig};

const GENERATED_PACKAGE_ID: &str = "com.example.generated";

fn read_ops_schema(name: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .join("../..")
        .join("schemas/packages-ops/v0.3")
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
        "vua-packages-ops-wire-v03-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// Seeds a minimal generated local-package root (the directory containing
/// the package `package.json`). Purely synthetic — the wire fake never
/// reads it, but the path fact must reach the backend verbatim.
fn seed_local_package(root: &Path) -> PathBuf {
    let package_root = root.join("generated").join(GENERATED_PACKAGE_ID);
    fs::create_dir_all(&package_root).expect("package root dir");
    fs::write(
        package_root.join("package.json"),
        format!(r#"{{"name":"{GENERATED_PACKAGE_ID}","version":"1.0.0","vpm":{{}}}}"#),
    )
    .expect("package manifest");
    package_root
}

/// The A3 face never touches a project: the task authority comes with an
/// EMPTY registration world on purpose — the route performs no
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
/// `packages_ops_consumer_v03` pins the library shapes against the same
/// schemas).
struct FakeRegister {
    register_cap: bool,
    register_failure: Option<AppErrorV1>,
}

impl FakeRegister {
    fn wired() -> Self {
        Self {
            register_cap: true,
            register_failure: None,
        }
    }
}

impl VpmBackend for FakeRegister {
    fn name(&self) -> &'static str {
        "fake-register-wire"
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
    fn register_capabilities(&self) -> RegisterCapabilities {
        RegisterCapabilities {
            register_local_package: self.register_cap,
        }
    }
    fn register_local_package(&self, package_root: &Path) -> Result<(), AppErrorV1> {
        if let Some(error) = &self.register_failure {
            return Err(error.clone());
        }
        // The path fact must reach the backend verbatim — the wire never
        // rewrites, resolves or re-roots the request's packageRoot.
        assert_eq!(
            package_root.file_name().and_then(|name| name.to_str()),
            Some(GENERATED_PACKAGE_ID),
            "the wire transports the packageRoot fact verbatim"
        );
        Ok(())
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
        unreachable!("the A3 face has no preview arm")
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
        "frameId": "frame-packages-ops-v03",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages-ops-v03",
            "correlationId": "corr-packages-ops-v03",
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

/// Sends `application.getSnapshot` and extracts the `packages.registerOps`
/// capability row.
fn register_ops_capability_row(database: &Path, vpm: Option<Arc<dyn VpmBackend>>) -> Value {
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
        .find(|row| row["operationId"] == "packages.registerOps")
        .cloned()
        .expect("the packages.registerOps capability row exists")
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
            "the register task did not finish"
        );
        drop(store);
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn register_params(package_root: &Path) -> Value {
    json!({ "packageRoot": package_root.to_string_lossy() })
}

#[test]
fn absent_register_wiring_answers_the_typed_honest_absence_and_unavailable_capability() {
    let root = unique_root("absent");
    let database = root.join("tasks.sqlite");

    let payload = run_frame(
        &database,
        None,
        None,
        "command",
        "packages.registerLocalPackage",
        register_params(Path::new("C:/anywhere/generated")),
    );
    assert_eq!(payload["ok"], false, "absence is a typed failure");
    assert_eq!(payload["error"]["code"], "vua.packages.unavailable");
    assert_eq!(payload["error"]["category"], "unavailable");
    assert!(
        payload.get("result").is_none(),
        "absence never fabricates a receipt"
    );

    let row = register_ops_capability_row(&database, None);
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_register_accepts_the_task_and_the_reflux_carries_the_minimal_registered_receipt() {
    let validator = ops_result_validator();
    let root = unique_root("receipt");
    let package_root = seed_local_package(&root);
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeRegister::wired())),
        "command",
        "packages.registerLocalPackage",
        register_params(&package_root),
    );
    assert_eq!(payload["ok"], true, "the wired command accepts: {payload}");
    let acceptance = &payload["value"];
    assert_eq!(acceptance["operation"], "packages.registerLocalPackage");
    assert_eq!(acceptance["schemaVersion"], "0.3");
    let task_id = acceptance["taskId"].as_str().expect("taskId").to_owned();
    assert!(!acceptance["correlationId"]
        .as_str()
        .unwrap_or_default()
        .is_empty());

    let done = wait_done(&database, &task_id);
    assert_eq!(done["schemaVersion"], "0.3");
    assert_eq!(done["operation"], "packages.registerLocalPackage");
    assert!(
        validator.is_valid(&done),
        "receipt must match the frozen result schema: {done}"
    );
    let receipt = &done["result"];
    assert_eq!(receipt["schemaVersion"], "vua.packages-ops/v0.3");
    assert_eq!(receipt["kind"], "registered");
    // The minimal honest audit shape: the request echo and NOTHING else —
    // additionalProperties:false makes an invented fact (added flag,
    // timestamp, manifest contents) a schema violation, not a nitpick.
    let receipt_keys: Vec<String> = receipt
        .as_object()
        .expect("receipt object")
        .keys()
        .cloned()
        .collect();
    assert_eq!(receipt_keys, vec!["schemaVersion", "kind", "packageRoot"]);
    assert_eq!(
        receipt["packageRoot"],
        package_root.to_string_lossy().as_ref()
    );

    let row = register_ops_capability_row(&database, Some(Arc::new(FakeRegister::wired())));
    assert_eq!(row["availability"], "available");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn register_param_violations_answer_invalid_params() {
    let root = unique_root("params");
    let package_root = seed_local_package(&root);
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let vpm = Some(Arc::new(FakeRegister::wired()) as Arc<dyn VpmBackend>);

    let violations: Vec<Value> = vec![
        // The closed single-key set: an absent packageRoot is a violation.
        json!({}),
        // An empty packageRoot is a violation (non-empty path fact).
        json!({ "packageRoot": "" }),
        // NO projectPath is taken — registration never touches a project.
        json!({
            "packageRoot": package_root.to_string_lossy(),
            "projectPath": "C:/some/project",
        }),
        // No digest, no confirmation chain — a carried confirmedDigest is
        // a shape violation (the user's explicit submission IS the
        // confirmation).
        json!({
            "packageRoot": package_root.to_string_lossy(),
            "confirmedDigest": "fnv1a-whatever",
        }),
        // A path fact is a string — a number is a violation.
        json!({ "packageRoot": 42 }),
    ];
    for params in violations {
        let payload = run_frame(
            &database,
            Some(&config),
            vpm.clone(),
            "command",
            "packages.registerLocalPackage",
            params,
        );
        assert_eq!(payload["ok"], false, "violations are typed failures");
        assert_eq!(
            payload["error"]["code"], "vua.packages.invalid_params",
            "{payload}"
        );
        assert_eq!(payload["error"]["category"], "validation");
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn register_without_the_capability_answers_capability_missing_and_never_reaches_a_task() {
    let root = unique_root("uncapped");
    let package_root = seed_local_package(&root);
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeRegister {
            register_cap: false,
            register_failure: None,
        })),
        "command",
        "packages.registerLocalPackage",
        register_params(&package_root),
    );
    assert_eq!(payload["ok"], false);
    // The generic capability-missing arm, answered BEFORE submit.
    assert_eq!(payload["error"]["code"], "vua.vpm.capability_missing");
    assert_eq!(payload["error"]["category"], "unavailable");
    assert!(
        payload["value"].is_null(),
        "capability absence never reaches a task"
    );

    let row = register_ops_capability_row(
        &database,
        Some(Arc::new(FakeRegister {
            register_cap: false,
            register_failure: None,
        })),
    );
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn register_invalid_refusal_folds_into_execution_failed_with_port_provenance() {
    let validator = ops_result_validator();
    let root = unique_root("invalid");
    let package_root = seed_local_package(&root);
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let vpm = FakeRegister {
        register_cap: true,
        register_failure: Some(AppErrorV1::new(
            "vua.vpm.local_package_invalid",
            ErrorCategory::Validation,
            "errors.vpm.localPackageInvalid",
            "corr-fake",
        )),
    };

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.registerLocalPackage",
        register_params(&package_root),
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
    // A3 adds NO guard: the path/shape refusal folds into the frozen
    // execution_failed guard.
    assert_eq!(rejected["guard"], "execution_failed");
    assert_eq!(rejected["code"], "vua.packages.execution_failed");
    // Honest provenance: the original port code travels inside detail —
    // the reused vua.vpm.* code never enters the code key.
    let detail = rejected["detail"].as_str().expect("detail");
    assert!(detail.contains("vua.vpm.local_package_invalid"), "{detail}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn register_failed_refusal_folds_into_execution_failed_with_port_provenance() {
    let root = unique_root("register-failed");
    let package_root = seed_local_package(&root);
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let vpm = FakeRegister {
        register_cap: true,
        register_failure: Some(AppErrorV1::new(
            "vua.vpm.local_package_register_failed",
            ErrorCategory::ExternalFailure,
            "errors.vpm.localPackageRegisterFailed",
            "corr-fake",
        )),
    };

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.registerLocalPackage",
        register_params(&package_root),
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
    assert!(
        detail.contains("vua.vpm.local_package_register_failed"),
        "{detail}"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn unimplemented_register_port_capability_missing_folds_into_execution_failed() {
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
        fn register_capabilities(&self) -> RegisterCapabilities {
            RegisterCapabilities {
                register_local_package: true,
            }
        }
        fn preview_install(
            &self,
            _project: &ProjectRef,
            _packages: &[PackageRequestV1],
        ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
            unreachable!("the A3 face has no preview arm")
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
    let package_root = seed_local_package(&root);
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(DeclaredButUnimplemented)),
        "command",
        "packages.registerLocalPackage",
        register_params(&package_root),
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
fn register_idempotence_is_one_success_fact_on_the_wire_face() {
    // First registration and re-registration are ONE success fact: the
    // two refluxes carry the exact same minimal receipt — no added
    // boolean, no first-vs-repeat distinction (the negative vector pins
    // the invented one).
    let validator = ops_result_validator();
    let root = unique_root("idempotent");
    let package_root = seed_local_package(&root);
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let mut receipts = Vec::new();
    for round in ["first", "repeat"] {
        let payload = run_frame(
            &database,
            Some(&config),
            Some(Arc::new(FakeRegister::wired())),
            "command",
            "packages.registerLocalPackage",
            register_params(&package_root),
        );
        assert_eq!(
            payload["ok"], true,
            "the {round} registration accepts: {payload}"
        );
        let task_id = payload["value"]["taskId"]
            .as_str()
            .expect("taskId")
            .to_owned();
        let done = wait_done(&database, &task_id);
        assert!(
            validator.is_valid(&done),
            "{round} reflux must match the frozen schema: {done}"
        );
        assert_eq!(done["result"]["kind"], "registered", "{round}");
        receipts.push(done["result"].clone());
    }
    assert_eq!(
        receipts[0], receipts[1],
        "first and repeat collapse into ONE success fact"
    );
    fs::remove_dir_all(&root).ok();
}
