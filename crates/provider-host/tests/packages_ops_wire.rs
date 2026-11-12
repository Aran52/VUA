//! Packages-ops wire tests (proposal 026 A1 wiring, 2026-09-19): the
//! removal write face rides the real frame loop over the frozen
//! `schemas/packages-ops/v0.1/` word list — the synchronous preview arm,
//! the nine-state task arm with its terminal reflux, the double-digest
//! refusal as a Done-payload `rejected` document (a recoverable conflict
//! the task completes honestly around — honesty rule 3), the closed-set
//! port-code projection, the reused 013 not-found code, and the
//! `packages.removeOps` capability row gated on the port's
//! `remove_packages` bit. The contract-side consumer loop (schema vectors
//! plus the port-to-wire projection shapes) is pinned by
//! `packages_ops_consumer.rs`; this file pins the ROUTES. The real backend
//! consumption is the environment implementation-verification slice.
//! Everything runs against synthetic directory trees — no machine-specific
//! facts, no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, ChangeItemV1, ChangeKindV1, ChangePreviewV1, ErrorCategory, ProjectRef,
    SqliteTaskStore, VpmBackend, VpmCapabilities,
};
use vua_project_manager::ManagerRoots;
use vua_provider_host::{run_provider_host_full, ProjectOpsConfig};

const FAKE_DIGEST: &str = "fnv1a-9e3779b9";

fn read_ops_schema(name: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .join("../..")
        .join("schemas/packages-ops/v0.1")
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
        "vua-packages-ops-wire-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// Seeds a minimal real Unity project plus a VCC settings.json registering
/// it (the 013 aggregate's world). Returns (project path, vcc settings path).
fn seed_registered_project(root: &Path) -> (PathBuf, PathBuf) {
    let project = root.join("registered-project");
    fs::create_dir_all(project.join("Assets")).expect("Assets dir");
    fs::create_dir_all(project.join("ProjectSettings")).expect("ProjectSettings dir");
    fs::create_dir_all(project.join("Packages")).expect("Packages dir");
    fs::write(
        project.join("ProjectSettings").join("ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1",
    )
    .expect("project version");
    fs::write(
        project.join("Packages").join("vpm-manifest.json"),
        r#"{"dependencies":{},"locked":{}}"#,
    )
    .expect("vpm manifest");
    let vcc_settings = root.join("vcc").join("settings.json");
    fs::create_dir_all(vcc_settings.parent().unwrap()).expect("vcc dir");
    fs::write(
        &vcc_settings,
        serde_json::to_string(&json!({
            "userProjects": [project.to_string_lossy()],
        }))
        .unwrap(),
    )
    .expect("vcc settings");
    (project, vcc_settings)
}

fn project_ops_config(vcc_settings: &Path) -> ProjectOpsConfig {
    ProjectOpsConfig {
        vcc_settings_candidates: vec![vcc_settings.to_path_buf()],
        manager_roots: ManagerRoots {
            alcom_settings_candidates: Vec::new(),
        },
        editor_roots: Vec::new(),
    }
}

fn fake_preview() -> ChangePreviewV1 {
    ChangePreviewV1 {
        items: vec![
            ChangeItemV1 {
                kind: ChangeKindV1::Remove,
                package_id: "com.lilxyzw.liltoon".into(),
                version: None,
                reason: None,
            },
            ChangeItemV1 {
                kind: ChangeKindV1::Remove,
                package_id: "jp.lilxyzw.avatarloader".into(),
                version: None,
                reason: Some("transitive_dependency".into()),
            },
        ],
        conflicts: vec!["com.another.avatarbase depends on com.lilxyzw.liltoon".into()],
        remove_legacy_files: vec!["Assets/lilToon/Editor/OldMetaFile.txt".into()],
        remove_legacy_folders: vec!["Assets/lilToon".into()],
        destructive: true,
        digest: FAKE_DIGEST.into(),
    }
}

/// A deterministic fake engine: the wire test pins routing and projection,
/// not the vrc-get library (the core-domain consumer test
/// `packages_ops_consumer` pins the library shapes against the same
/// schemas).
struct FakeRemove {
    remove_cap: bool,
    preview_failure: Option<AppErrorV1>,
    apply_failure: Option<AppErrorV1>,
}

impl FakeRemove {
    fn wired() -> Self {
        Self {
            remove_cap: true,
            preview_failure: None,
            apply_failure: None,
        }
    }
}

impl VpmBackend for FakeRemove {
    fn name(&self) -> &'static str {
        "fake-remove-wire"
    }
    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: false,
            preview_install: false,
            list_packages: false,
            remove_packages: self.remove_cap,
            project_registry: false,
            resolve_project: false,
        }
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[vua_orchestrator::PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[vua_orchestrator::PackageRequestV1],
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
    fn preview_remove(
        &self,
        _project: &ProjectRef,
        _package_ids: &[String],
    ) -> Result<ChangePreviewV1, AppErrorV1> {
        if let Some(error) = &self.preview_failure {
            return Err(error.clone());
        }
        Ok(fake_preview())
    }
    fn apply_remove(
        &self,
        _project: &ProjectRef,
        _package_ids: &[String],
        confirmed_digest: &str,
    ) -> Result<serde_json::Value, AppErrorV1> {
        if let Some(error) = &self.apply_failure {
            return Err(error.clone());
        }
        // Defense in depth behind the wire-layer re-computation: the wire
        // layer refuses on drift BEFORE this is ever called, so a mismatch
        // here would mean the wire guard is broken — fail loudly.
        assert_eq!(
            confirmed_digest, FAKE_DIGEST,
            "wire layer must gate on the digest"
        );
        let preview = fake_preview();
        Ok(json!({ "removed": preview.items }))
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
        "frameId": "frame-packages-ops",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages-ops",
            "correlationId": "corr-packages-ops",
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

/// Sends `application.getSnapshot` and extracts the `packages.removeOps`
/// capability row.
fn remove_ops_capability_row(database: &Path, vpm: Option<Arc<dyn VpmBackend>>) -> Value {
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
        .find(|row| row["operationId"] == "packages.removeOps")
        .cloned()
        .expect("the packages.removeOps capability row exists")
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
            "the apply-remove task did not finish"
        );
        drop(store);
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn preview_params(project_path: &str, ids: &[&str]) -> Value {
    json!({
        "projectPath": project_path,
        "packageIds": ids,
    })
}

#[test]
fn absent_wiring_answers_the_typed_honest_absence_and_unavailable_capability() {
    let root = unique_root("absent");
    let database = root.join("tasks.sqlite");

    let payload = run_frame(
        &database,
        None,
        None,
        "query",
        "packages.previewRemove",
        preview_params("C:/anywhere", &["com.example.pkg"]),
    );
    assert_eq!(payload["ok"], false, "absence is a typed failure");
    assert_eq!(payload["error"]["code"], "vua.packages.unavailable");
    assert_eq!(payload["error"]["category"], "unavailable");
    assert!(
        payload.get("result").is_none(),
        "absence never fabricates a plan"
    );

    let payload = run_frame(
        &database,
        None,
        None,
        "command",
        "packages.applyRemove",
        json!({
            "projectPath": "C:/anywhere",
            "packageIds": ["com.example.pkg"],
            "confirmedDigest": FAKE_DIGEST,
        }),
    );
    assert_eq!(payload["ok"], false, "absence is a typed failure");
    assert_eq!(payload["error"]["code"], "vua.packages.unavailable");

    let row = remove_ops_capability_row(&database, None);
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_preview_projects_the_plan_inside_the_frozen_envelope() {
    let validator = ops_result_validator();
    let root = unique_root("preview");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeRemove::wired())),
        "query",
        "packages.previewRemove",
        preview_params(project.to_string_lossy().as_ref(), &["com.lilxyzw.liltoon"]),
    );
    assert_eq!(payload["ok"], true, "the wired preview answers: {payload}");
    let envelope = &payload["value"];
    // The frozen result schema describes the whole envelope
    // (schemaVersion/operation/result).
    assert!(
        validator.is_valid(envelope),
        "plan must match the frozen result schema: {envelope}"
    );
    assert_eq!(envelope["schemaVersion"], "0.1");
    assert_eq!(envelope["operation"], "packages.previewRemove");
    let plan = &envelope["result"];
    assert_eq!(plan["schemaVersion"], "vua.packages-ops/v0.1");
    assert_eq!(plan["kind"], "plan");
    assert_eq!(plan["projectPath"], project.to_string_lossy().as_ref());
    // The frozen camelCase projection of the port facts.
    assert_eq!(plan["digest"], FAKE_DIGEST);
    assert_eq!(plan["destructive"], serde_json::Value::Bool(true));
    let items = plan["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["kind"], "remove");
    assert_eq!(items[0]["packageId"], "com.lilxyzw.liltoon");
    assert_eq!(items[0]["version"], serde_json::Value::Null);
    assert_eq!(items[1]["reason"], "transitive_dependency");
    assert_eq!(
        plan["removeLegacyFolders"][0], "Assets/lilToon",
        "legacy folder cleanups travel verbatim"
    );

    let row = remove_ops_capability_row(&database, Some(Arc::new(FakeRemove::wired())));
    assert_eq!(row["availability"], "available");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn preview_unregistered_path_reuses_the_013_not_found_code() {
    let root = unique_root("unregistered");
    let (_project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeRemove::wired())),
        "query",
        "packages.previewRemove",
        preview_params("C:/not/registered", &["com.example.pkg"]),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.project.project_not_found");
    assert_eq!(payload["error"]["category"], "validation");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn preview_param_violations_answer_invalid_params() {
    let root = unique_root("params");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm = Some(Arc::new(FakeRemove::wired()) as Arc<dyn VpmBackend>);
    let project_path = project.to_string_lossy().into_owned();

    let violations: Vec<Value> = vec![
        // A preview request carrying a digest: the digest is the preview's
        // product — carrying one is a shape violation.
        json!({ "projectPath": project_path, "packageIds": ["com.example.pkg"], "confirmedDigest": FAKE_DIGEST }),
        // Empty packageIds: no wildcard, no "remove everything" shorthand.
        json!({ "projectPath": project_path, "packageIds": [] }),
        // Duplicate entries violate the closed explicit list.
        json!({ "projectPath": project_path, "packageIds": ["com.a", "com.a"] }),
        // Unknown key: the closed param set admits nothing else.
        json!({ "projectPath": project_path, "packageIds": ["com.a"], "wildcard": true }),
    ];
    for params in violations {
        let payload = run_frame(
            &database,
            Some(&config),
            vpm.clone(),
            "query",
            "packages.previewRemove",
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
fn preview_without_the_capability_answers_the_generic_capability_missing() {
    let root = unique_root("uncapped");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm = FakeRemove {
        remove_cap: false,
        preview_failure: None,
        apply_failure: None,
    };

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "query",
        "packages.previewRemove",
        preview_params(project.to_string_lossy().as_ref(), &["com.example.pkg"]),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.capability_missing");
    assert_eq!(payload["error"]["category"], "unavailable");

    let row = remove_ops_capability_row(
        &database,
        Some(Arc::new(FakeRemove {
            remove_cap: false,
            preview_failure: None,
            apply_failure: None,
        })),
    );
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn preview_package_not_installed_projects_onto_the_closed_set_code() {
    let root = unique_root("not-installed");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm = FakeRemove {
        remove_cap: true,
        preview_failure: Some(AppErrorV1::new(
            "vua.vpm.package_not_installed",
            ErrorCategory::Validation,
            "errors.vpm.packageNotInstalled",
            "corr-fake",
        )),
        apply_failure: None,
    };

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "query",
        "packages.previewRemove",
        preview_params(project.to_string_lossy().as_ref(), &["com.missing.pkg"]),
    );
    assert_eq!(
        payload["ok"], false,
        "preview failures are envelope errors, never result arms"
    );
    assert_eq!(payload["error"]["code"], "vua.packages.package_not_found");
    assert_eq!(
        payload["error"]["messageKey"],
        "errors.packages.packageNotFound"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn apply_receipt_travels_inside_the_task_done_payload() {
    let validator = ops_result_validator();
    let root = unique_root("receipt");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    // The command we send must match the frozen command schema shape; the
    // acceptance answers the task, the reflux carries the frozen receipt.
    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeRemove::wired())),
        "command",
        "packages.applyRemove",
        json!({
            "projectPath": project.to_string_lossy(),
            "packageIds": ["com.lilxyzw.liltoon"],
            "confirmedDigest": FAKE_DIGEST,
        }),
    );
    assert_eq!(payload["ok"], true, "the wired command accepts: {payload}");
    let acceptance = &payload["value"];
    assert_eq!(acceptance["operation"], "packages.applyRemove");
    let task_id = acceptance["taskId"].as_str().expect("taskId").to_owned();
    assert!(!acceptance["correlationId"]
        .as_str()
        .unwrap_or_default()
        .is_empty());

    let done = wait_done(&database, &task_id);
    assert_eq!(done["schemaVersion"], "0.1");
    assert_eq!(done["operation"], "packages.applyRemove");
    assert!(
        validator.is_valid(&done),
        "receipt must match the frozen result schema: {done}"
    );
    let receipt = &done["result"];
    assert_eq!(receipt["schemaVersion"], "vua.packages-ops/v0.1");
    assert_eq!(receipt["kind"], "receipt");
    // The audit receipt: the confirmed digest echo + the request list +
    // the actually removed items (port facts verbatim).
    assert_eq!(receipt["confirmedDigest"], FAKE_DIGEST);
    assert_eq!(
        receipt["requestedPackageIds"],
        json!(["com.lilxyzw.liltoon"])
    );
    let removed = receipt["removedItems"].as_array().expect("removed items");
    assert_eq!(removed.len(), 2);
    assert_eq!(removed[0]["packageId"], "com.lilxyzw.liltoon");
    assert_eq!(removed[1]["reason"], "transitive_dependency");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn apply_digest_drift_refuses_inside_the_task_as_preview_drift() {
    let validator = ops_result_validator();
    let root = unique_root("drift");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeRemove::wired())),
        "command",
        "packages.applyRemove",
        json!({
            "projectPath": project.to_string_lossy(),
            "packageIds": ["com.lilxyzw.liltoon"],
            "confirmedDigest": "fnv1a-stale",
        }),
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
    assert_eq!(rejected["guard"], "preview_drift");
    assert_eq!(rejected["code"], "vua.packages.preview_drift");
    assert!(
        !rejected["detail"].as_str().unwrap_or_default().is_empty(),
        "the refusal carries its evidence: {rejected}"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn apply_apply_failure_folds_into_execution_failed_with_port_provenance() {
    let root = unique_root("apply-failed");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm = FakeRemove {
        remove_cap: true,
        preview_failure: None,
        apply_failure: Some(AppErrorV1::new(
            "vua.vpm.apply_failed",
            ErrorCategory::ExternalFailure,
            "errors.vpm.applyFailed",
            "corr-fake",
        )),
    };

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "command",
        "packages.applyRemove",
        json!({
            "projectPath": project.to_string_lossy(),
            "packageIds": ["com.lilxyzw.liltoon"],
            "confirmedDigest": FAKE_DIGEST,
        }),
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
    // Honest provenance: the original port code travels inside detail.
    let detail = rejected["detail"].as_str().expect("detail");
    assert!(detail.contains("vua.vpm.apply_failed"), "{detail}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn apply_unregistered_path_refuses_at_the_route_layer_never_inside_a_task() {
    let root = unique_root("apply-unregistered");
    let (_project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeRemove::wired())),
        "command",
        "packages.applyRemove",
        json!({
            "projectPath": "C:/not/registered",
            "packageIds": ["com.example.pkg"],
            "confirmedDigest": FAKE_DIGEST,
        }),
    );
    assert_eq!(payload["ok"], false);
    // The reused 013 code can never travel inside a rejected document (the
    // rejected arm's code schema locks ^vua\.packages\.) — the refusal is
    // the typed envelope error, no task is created.
    assert_eq!(payload["error"]["code"], "vua.project.project_not_found");
    assert!(
        payload["value"].is_null(),
        "no task is accepted for an unregistered path"
    );
    fs::remove_dir_all(&root).ok();
}
