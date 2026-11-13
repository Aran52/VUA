//! Packages-ops v0.5 wire tests (proposal 026 A5 wiring, 2026-09-19): the
//! project-creation write face rides the real frame loop over the frozen
//! `schemas/packages-ops/v0.5/` word list — the one method
//! `packages.createProject` {parent, name, template} as a nine-state task
//! command with its terminal reflux, the `created` receipt (the ONE
//! packages-ops receipt with an actual-result payload: the port
//! `ProjectRef {id, root}` projected onto keys `projectId`/`projectPath`,
//! the exact four-key pin, additionalProperties:false makes an invented
//! fact a schema violation), the all-refusals-fold-into-`execution_failed`
//! projection with the original port code carried inside detail (A5 adds
//! NO guard; the three-code closed set template_missing / apply_failed /
//! backend_unavailable are EXISTING codes; the create face claims NO
//! idempotence — where the backend refuses an existing target the wire
//! answers the refusal honestly), the closed param face (REQUIRED-nullable
//! template: the key must be present, null = the port None fact, an empty
//! string is a violation; a carried `confirmedDigest` or `projectPath` is
//! a shape violation), the capability gate on the EXISTING five-bit
//! `capabilities().create_project` member answered BEFORE submit (A5
//! freezes NO new accessor, unlike A3/A4 — and because the port method is
//! REQUIRED with no default body the gate IS the absence arm: a declared
//! but unimplemented backend cannot exist at the type level), and the
//! `packages.createOps` capability row (one row serving the one method).
//! The frozen v0.1 A1 removal row, v0.2 A2 install row, v0.3 A3
//! registration row and v0.4 A4 repository add/remove row keep serving
//! untouched through `packages_ops_wire.rs`, `packages_ops_wire_v02.rs`,
//! `packages_ops_wire_v03.rs` and `packages_ops_wire_v04.rs`; this file
//! pins the v0.5 ROUTE. The contract-side consumer loop is pinned by
//! `packages_ops_consumer_v05`; the real backend consumption (both
//! in-repo backends already implement `create_project`) is the environment
//! implementation-verification slice. Everything runs against synthetic
//! values — no machine-specific facts, no network, no template copy.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, ErrorCategory, PackageRequestV1, ProjectRef, SqliteTaskStore, VpmBackend,
    VpmCapabilities,
};
use vua_project_manager::ManagerRoots;
use vua_provider_host::{
    provider_host::{PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V05, PACKAGES_OPS_SCHEMA_VERSION_V05},
    run_provider_host_full, ProjectOpsConfig,
};

fn read_ops_schema(name: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .join("../..")
        .join("schemas/packages-ops/v0.5")
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
        "vua-packages-ops-wire-v05-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// The A5 face never touches an existing project: the task authority comes
/// with an EMPTY registration world on purpose — the route performs no
/// registered-project check, so an empty world must change nothing (and
/// the CREATED project is a registered project by the backend's own
/// initialize side effect, never by a wire-layer registration invention).
fn empty_ops_config() -> ProjectOpsConfig {
    ProjectOpsConfig {
        vcc_settings_candidates: Vec::new(),
        manager_roots: ManagerRoots {
            alcom_settings_candidates: Vec::new(),
        },
        editor_roots: Vec::new(),
    }
}

const FAKE_PARENT: &str = "D:/synthetic/projects";
const FAKE_NAME: &str = "Synthetic Project";
const FAKE_TEMPLATE: &str = "Avatar";
const FAKE_PROJECT_ID: &str = "proj-Synthetic Project";
/// The root fact as the wire transports it — the PathBuf's string form,
/// pinned here and NEVER re-derived (the route echoes the port's
/// `ProjectRef.root` verbatim, no separator rewriting, no resolution).
const FAKE_PROJECT_PATH: &str = "D:/synthetic/projects/Synthetic Project";

/// A deterministic fake engine: the wire test pins routing and projection,
/// not the vrc-get library or the vpm CLI (the core-domain consumer test
/// `packages_ops_consumer_v05` pins the port shapes against the same
/// schemas). The EXISTING five-bit create_project member + per-case
/// failure injection — the gate reads the bit, never a new accessor.
struct FakeCreate {
    create_cap: bool,
    create_failure: Option<AppErrorV1>,
    /// When set, the template argument must equal it verbatim (a None
    /// inside = the port must receive the default-resolution None).
    expect_template: Option<Option<String>>,
}

impl FakeCreate {
    fn wired() -> Self {
        Self {
            create_cap: true,
            create_failure: None,
            expect_template: None,
        }
    }

    fn declared_false() -> Self {
        Self {
            create_cap: false,
            create_failure: None,
            expect_template: None,
        }
    }
}

impl VpmBackend for FakeCreate {
    fn name(&self) -> &'static str {
        "fake-create-wire"
    }
    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: self.create_cap,
            preview_install: false,
            list_packages: false,
            remove_packages: false,
            project_registry: false,
            resolve_project: false,
        }
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
        unreachable!("the A5 face has no preview arm")
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
        parent: &Path,
        name: &str,
        template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
        if let Some(error) = &self.create_failure {
            return Err(error.clone());
        }
        // The facts must reach the backend verbatim — the wire never
        // rewrites, resolves or re-encodes the request's parent/name, and
        // the REQUIRED-nullable template crosses as the port Option.
        assert_eq!(parent, Path::new(FAKE_PARENT), "verbatim parent transport");
        assert_eq!(name, FAKE_NAME, "verbatim name transport");
        if let Some(expected) = &self.expect_template {
            assert_eq!(
                template.map(str::to_owned).as_deref(),
                expected.as_deref(),
                "verbatim template transport (None = default resolution)"
            );
        }
        Ok(ProjectRef {
            id: FAKE_PROJECT_ID.to_string(),
            // Built from the forward-slash string so the wire's string
            // echo is deterministic across platforms (PathBuf preserves
            // the given string form; no separator normalization happens).
            root: PathBuf::from(FAKE_PROJECT_PATH),
        })
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
        "frameId": "frame-packages-ops-v05",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages-ops-v05",
            "correlationId": "corr-packages-ops-v05",
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

/// Sends `application.getSnapshot` and extracts the `packages.createOps`
/// capability row.
fn create_ops_capability_row(database: &Path, vpm: Option<Arc<dyn VpmBackend>>) -> Value {
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
        .find(|row| row["operationId"] == "packages.createOps")
        .cloned()
        .expect("the packages.createOps capability row exists")
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
        assert!(Instant::now() < deadline, "the create task did not finish");
        drop(store);
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn create_params() -> Value {
    json!({ "parent": FAKE_PARENT, "name": FAKE_NAME, "template": FAKE_TEMPLATE })
}

fn create_params_default_template() -> Value {
    json!({ "parent": FAKE_PARENT, "name": FAKE_NAME, "template": null })
}

#[test]
fn absent_create_wiring_answers_the_typed_honest_absence_and_unavailable_capability() {
    let root = unique_root("absent");
    let database = root.join("tasks.sqlite");

    let payload = run_frame(
        &database,
        None,
        None,
        "command",
        "packages.createProject",
        create_params(),
    );
    assert_eq!(payload["ok"], false, "absence is a typed failure");
    assert_eq!(payload["error"]["code"], "vua.packages.unavailable");
    assert_eq!(payload["error"]["category"], "unavailable");
    assert!(
        payload.get("result").is_none(),
        "absence never fabricates a receipt"
    );

    let row = create_ops_capability_row(&database, None);
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_create_accepts_and_the_reflux_carries_the_created_receipt() {
    let validator = ops_result_validator();
    let root = unique_root("create");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeCreate::wired())),
        "command",
        "packages.createProject",
        create_params(),
    );
    assert_eq!(payload["ok"], true, "the wired command accepts: {payload}");
    let acceptance = &payload["value"];
    assert_eq!(acceptance["operation"], "packages.createProject");
    assert_eq!(acceptance["schemaVersion"], "0.5");
    let task_id = acceptance["taskId"].as_str().expect("taskId").to_owned();
    assert!(!acceptance["correlationId"]
        .as_str()
        .unwrap_or_default()
        .is_empty());

    let done = wait_done(&database, &task_id);
    assert_eq!(done["schemaVersion"], "0.5");
    assert_eq!(done["operation"], "packages.createProject");
    assert!(
        validator.is_valid(&done),
        "receipt must match the frozen result schema: {done}"
    );
    let receipt = &done["result"];
    assert_eq!(receipt["schemaVersion"], "vua.packages-ops/v0.5");
    assert_eq!(receipt["kind"], "created");
    // The ProjectRef projection, exactly: the ONE packages-ops receipt
    // with an actual-result payload — four keys and nothing else
    // (additionalProperties:false makes an invented created-at timestamp,
    // copy statistic or package list a schema violation, not a nitpick).
    let receipt_keys: Vec<String> = receipt
        .as_object()
        .expect("receipt object")
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        receipt_keys,
        vec!["schemaVersion", "kind", "projectId", "projectPath"]
    );
    assert_eq!(
        receipt["projectId"], FAKE_PROJECT_ID,
        "the ProjectRef.id echo"
    );
    assert_eq!(
        receipt["projectPath"], FAKE_PROJECT_PATH,
        "the ProjectRef.root echo = the registered-path identity"
    );

    let row = create_ops_capability_row(&database, Some(Arc::new(FakeCreate::wired())));
    assert_eq!(row["availability"], "available");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn null_template_passes_the_port_none_and_the_receipt_still_projects_verbatim() {
    // The REQUIRED-nullable law on the live wire: the key PRESENT with
    // null crosses the route as the port's Option None (the backend's
    // default template resolution — the frozen word-face FACT, not a
    // picker), and the created receipt projects exactly the same way.
    let validator = ops_result_validator();
    let root = unique_root("null-template");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeCreate {
            expect_template: Some(None),
            ..FakeCreate::wired()
        })),
        "command",
        "packages.createProject",
        create_params_default_template(),
    );
    assert_eq!(
        payload["ok"], true,
        "the null-template command accepts: {payload}"
    );
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert!(validator.is_valid(&done), "{done}");
    assert_eq!(done["result"]["kind"], "created");
    assert_eq!(done["result"]["projectPath"], FAKE_PROJECT_PATH);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn create_param_violations_answer_invalid_params() {
    let root = unique_root("params");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let vpm = Some(Arc::new(FakeCreate::wired()) as Arc<dyn VpmBackend>);

    let violations: Vec<Value> = vec![
        // The closed three-key set — absent keys are violations.
        json!({}),
        json!({ "parent": FAKE_PARENT, "name": FAKE_NAME }),
        json!({ "name": FAKE_NAME, "template": null }),
        json!({ "parent": FAKE_PARENT, "template": null }),
        // Empty parent / name / template-string are violations.
        json!({ "parent": "", "name": FAKE_NAME, "template": null }),
        json!({ "parent": FAKE_PARENT, "name": "", "template": null }),
        json!({ "parent": FAKE_PARENT, "name": FAKE_NAME, "template": "" }),
        // REQUIRED-nullable cuts both ways: any non-null non-empty-string
        // value is a violation.
        json!({ "parent": FAKE_PARENT, "name": FAKE_NAME, "template": 42 }),
        // NO projectPath is taken — creation addresses no registered
        // project (the 013 reuse does not apply on this face).
        json!({
            "parent": FAKE_PARENT,
            "name": FAKE_NAME,
            "template": null,
            "projectPath": "C:/some/project",
        }),
        // No digest, no confirmation chain — a carried confirmedDigest is
        // a shape violation (the explicit form submission IS the
        // confirmation).
        json!({
            "parent": FAKE_PARENT,
            "name": FAKE_NAME,
            "template": null,
            "confirmedDigest": "fnv1a-whatever",
        }),
        // Any fourth key breaks the closed set.
        json!({
            "parent": FAKE_PARENT,
            "name": FAKE_NAME,
            "template": null,
            "extra": "param",
        }),
    ];
    for params in violations {
        let payload = run_frame(
            &database,
            Some(&config),
            vpm.clone(),
            "command",
            "packages.createProject",
            params.clone(),
        );
        assert_eq!(payload["ok"], false, "violations are typed failures");
        assert_eq!(
            payload["error"]["code"], "vua.packages.invalid_params",
            "{params}: {payload}"
        );
        assert_eq!(payload["error"]["category"], "validation");
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn create_capability_gate_reads_the_existing_bit_and_absence_never_reaches_a_task() {
    // The A5 gate reads the EXISTING five-bit `capabilities().create_
    // project` member BEFORE submit — no new accessor exists on this face
    // (unlike A3/A4), and because the port method is REQUIRED with no
    // default body the gate IS the absence arm: a declared-but-
    // unimplemented backend cannot exist at the type level.
    let root = unique_root("declared-false");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeCreate::declared_false())),
        "command",
        "packages.createProject",
        create_params(),
    );
    assert_eq!(payload["ok"], false);
    // The generic capability-missing arm, answered BEFORE submit.
    assert_eq!(payload["error"]["code"], "vua.vpm.capability_missing");
    assert_eq!(payload["error"]["category"], "unavailable");
    assert!(
        payload["value"].is_null(),
        "capability absence never reaches a task"
    );

    let row = create_ops_capability_row(&database, Some(Arc::new(FakeCreate::declared_false())));
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn three_port_codes_fold_into_execution_failed_with_provenance() {
    // The fold discipline (A1–A4 same law): every port refusal folds into
    // a rejected document whose guard is execution_failed (ZERO new guard
    // values on A5) and whose code key stays inside the vua.packages.*
    // family — the original code travels verbatim in detail as honest
    // provenance. The A5 closed set is THREE EXISTING codes: the
    // library-path carrier (all four i18n keys share it) and the two
    // CLI-path legs — the backends' shapes honestly diverge, the wire face
    // folds both without inventing a unified shape.
    let validator = ops_result_validator();
    let config = empty_ops_config();

    for code in [
        "vua.vpm.template_missing",
        "vua.vpm.apply_failed",
        "vua.vpm.backend_unavailable",
    ] {
        let root = unique_root("fold");
        let database = root.join("tasks.sqlite");
        let vpm = FakeCreate {
            create_failure: Some(AppErrorV1::new(
                code,
                ErrorCategory::ExternalFailure,
                "errors.vpm.createRefused",
                "corr-fake",
            )),
            ..FakeCreate::wired()
        };
        let payload = run_frame(
            &database,
            Some(&config),
            Some(Arc::new(vpm)),
            "command",
            "packages.createProject",
            create_params(),
        );
        assert_eq!(
            payload["ok"], true,
            "{code}: the refusal still ACCEPTS as a task"
        );
        let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
        assert!(
            validator.is_valid(&done),
            "{code}: the refusal is a frozen result document: {done}"
        );
        let rejected = &done["result"];
        assert_eq!(rejected["kind"], "rejected");
        assert_eq!(rejected["guard"], "execution_failed");
        assert_eq!(rejected["code"], "vua.packages.execution_failed");
        let detail = rejected["detail"].as_str().expect("detail");
        assert!(
            detail.contains(code),
            "{code} provenance travels in detail: {detail}"
        );
        fs::remove_dir_all(&root).ok();
    }
}

#[test]
fn create_claims_no_idempotence_the_duplicate_refusal_travels_honestly() {
    // Creation is NOT idempotent (the backend's existing-target guard
    // refuses at execution): round one creates, round two with the SAME
    // params is refused — and the wire carries that refusal honestly as an
    // execution_failed rejected carrying the original port code, never an
    // invented idempotent success (the A3 AlreadyAdded collapse
    // deliberately NOT copied).
    let validator = ops_result_validator();
    let root = unique_root("no-idempotence");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();

    // Round one: the backend creates.
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeCreate::wired())),
        "command",
        "packages.createProject",
        create_params(),
    );
    assert_eq!(payload["ok"], true);
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert_eq!(done["result"]["kind"], "created");
    assert!(validator.is_valid(&done), "{done}");

    // Round two: the backend refuses the existing target
    // (template_missing is the library path's carrier for the
    // projectExists key) — the refusal travels verbatim.
    let vpm = FakeCreate {
        create_failure: Some(AppErrorV1::new(
            "vua.vpm.template_missing",
            ErrorCategory::Validation,
            "errors.vpm.projectExists",
            "corr-fake",
        )),
        ..FakeCreate::wired()
    };
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.createProject",
        create_params(),
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
        .contains("vua.vpm.template_missing"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn envelope_version_is_detectable_consts_match_the_frozen_word_face_and_stamp_both_frames() {
    // The A3/A4 precedent closed at the wiring batch: the envelope const
    // and the family const are named HERE and pinned against the frozen
    // schema consts — a consumer can detect the row version from the wire
    // alone (the acceptance answer, the Done payload and the receipt each
    // carry their const verbatim).
    let command = read_ops_schema("command.schema.json");
    let result = read_ops_schema("result.schema.json");
    assert_eq!(
        command["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V05),
        "the envelope const equals the frozen command schema's const"
    );
    assert_eq!(PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V05, "0.5");
    assert_eq!(
        result["$defs"]["projectCreated"]["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_OPS_SCHEMA_VERSION_V05),
        "the family const equals the frozen created def's const"
    );
    assert_eq!(
        result["$defs"]["rejected"]["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_OPS_SCHEMA_VERSION_V05),
        "the family const equals the frozen rejected def's const"
    );
    assert_eq!(PACKAGES_OPS_SCHEMA_VERSION_V05, "vua.packages-ops/v0.5");

    // Both live stamps: the acceptance answer AND the Done payload carry
    // the envelope const; the receipt carries the family const (pinned
    // against the consts, not against string literals).
    let root = unique_root("envelope");
    let database = root.join("tasks.sqlite");
    let config = empty_ops_config();
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeCreate::wired())),
        "command",
        "packages.createProject",
        create_params(),
    );
    assert_eq!(
        payload["value"]["schemaVersion"],
        PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V05
    );
    let done = wait_done(&database, payload["value"]["taskId"].as_str().unwrap());
    assert_eq!(
        done["schemaVersion"],
        PACKAGES_OPS_ENVELOPE_SCHEMA_VERSION_V05
    );
    assert_eq!(
        done["result"]["schemaVersion"],
        PACKAGES_OPS_SCHEMA_VERSION_V05
    );
    fs::remove_dir_all(&root).ok();
}
