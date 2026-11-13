//! Packages-ops v0.2 wire tests (proposal 026 A2 wiring, 2026-09-19): the
//! install/upgrade write face rides the real frame loop over the frozen
//! `schemas/packages-ops/v0.2/` word list — the synchronous preview arm,
//! the nine-state task arm with its terminal reflux, the double-digest
//! refusal as a Done-payload `rejected` document (a recoverable conflict
//! the task completes honestly around — honesty rule 3), the closed-set
//! port-code projection (`no_matching_package` → package_not_found,
//! `preview_failed` → the one new envelope-face code, `apply_failed` and
//! word-out codes folding into execution_failed with provenance), the
//! reused 013 not-found code, the version-selection params face
//! (`{packageId, version}` rows with nullable version, per-id uniqueness),
//! and the `packages.installOps` capability row gated on the port's
//! `preview_install` bit. The frozen v0.1 A1 removal row keeps serving
//! untouched through `packages_ops_wire.rs`; this file pins the v0.2
//! ROUTES. The contract-side consumer loop (schema vectors plus the
//! port-to-wire projection shapes) is pinned by
//! `packages_ops_consumer_v02.rs`; the real backend consumption is the
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
    AppErrorV1, ChangeItemV1, ChangeKindV1, ChangePreviewV1, ErrorCategory, PackageRequestV1,
    ProjectRef, SqliteTaskStore, VpmBackend, VpmCapabilities,
};
use vua_project_manager::ManagerRoots;
use vua_provider_host::{run_provider_host_full, ProjectOpsConfig};

const FAKE_DIGEST: &str = "fnv1a-a2install";

fn read_ops_schema(name: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .join("../..")
        .join("schemas/packages-ops/v0.2")
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
        "vua-packages-ops-wire-v02-{label}-{}-{nanos}",
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
                kind: ChangeKindV1::Install,
                package_id: "com.another.toolkit".into(),
                version: Some("3.1.4".into()),
                reason: None,
            },
            ChangeItemV1 {
                kind: ChangeKindV1::Install,
                package_id: "jp.lilxyzw.avatarloader".into(),
                version: Some("7.3.150".into()),
                reason: Some("transitive_dependency".into()),
            },
            // ORC-WF-002: an install plan covers EVERY change the backend
            // will make — conflict-triggered removals travel verbatim.
            ChangeItemV1 {
                kind: ChangeKindV1::Remove,
                package_id: "com.example.conflictingpackage".into(),
                version: None,
                reason: Some("conflict".into()),
            },
        ],
        conflicts: vec!["com.another.avatarbase depends on com.example.conflictingpackage".into()],
        remove_legacy_files: vec!["Assets/OldPackage/LegacyFile.txt".into()],
        remove_legacy_folders: vec!["Assets/OldPackage".into()],
        destructive: true,
        digest: FAKE_DIGEST.into(),
    }
}

/// A deterministic fake engine: the wire test pins routing and projection,
/// not the vrc-get library (the core-domain consumer test
/// `packages_ops_consumer_v02` pins the library shapes against the same
/// schemas).
struct FakeInstall {
    install_cap: bool,
    preview_failure: Option<AppErrorV1>,
    apply_failure: Option<AppErrorV1>,
}

impl FakeInstall {
    fn wired() -> Self {
        Self {
            install_cap: true,
            preview_failure: None,
            apply_failure: None,
        }
    }
}

impl VpmBackend for FakeInstall {
    fn name(&self) -> &'static str {
        "fake-install-wire"
    }
    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: false,
            preview_install: self.install_cap,
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
    ) -> Result<ChangePreviewV1, AppErrorV1> {
        if let Some(error) = &self.preview_failure {
            return Err(error.clone());
        }
        Ok(fake_preview())
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
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
        Ok(json!({ "applied": preview.items }))
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
        "frameId": "frame-packages-ops-v02",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages-ops-v02",
            "correlationId": "corr-packages-ops-v02",
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

/// Sends `application.getSnapshot` and extracts the `packages.installOps`
/// capability row.
fn install_ops_capability_row(database: &Path, vpm: Option<Arc<dyn VpmBackend>>) -> Value {
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
        .find(|row| row["operationId"] == "packages.installOps")
        .cloned()
        .expect("the packages.installOps capability row exists")
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
            "the apply-install task did not finish"
        );
        drop(store);
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn install_params(project_path: &str) -> Value {
    json!({
        "projectPath": project_path,
        "packages": [
            { "packageId": "com.lilxyzw.liltoon", "version": null },
            { "packageId": "com.another.toolkit", "version": "3.1.4" },
        ],
    })
}

#[test]
fn absent_install_wiring_answers_the_typed_honest_absence_and_unavailable_capability() {
    let root = unique_root("absent");
    let database = root.join("tasks.sqlite");

    let payload = run_frame(
        &database,
        None,
        None,
        "query",
        "packages.previewInstall",
        install_params("C:/anywhere"),
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
        "packages.applyInstall",
        json!({
            "projectPath": "C:/anywhere",
            "packages": [{ "packageId": "com.example.pkg", "version": null }],
            "confirmedDigest": FAKE_DIGEST,
        }),
    );
    assert_eq!(payload["ok"], false, "absence is a typed failure");
    assert_eq!(payload["error"]["code"], "vua.packages.unavailable");

    let row = install_ops_capability_row(&database, None);
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_install_preview_projects_the_plan_inside_the_frozen_v02_envelope() {
    let validator = ops_result_validator();
    let root = unique_root("preview");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeInstall::wired())),
        "query",
        "packages.previewInstall",
        install_params(project.to_string_lossy().as_ref()),
    );
    assert_eq!(payload["ok"], true, "the wired preview answers: {payload}");
    let envelope = &payload["value"];
    // The frozen result schema describes the whole envelope
    // (schemaVersion/operation/result).
    assert!(
        validator.is_valid(envelope),
        "plan must match the frozen result schema: {envelope}"
    );
    assert_eq!(envelope["schemaVersion"], "0.2");
    assert_eq!(envelope["operation"], "packages.previewInstall");
    let plan = &envelope["result"];
    assert_eq!(plan["schemaVersion"], "vua.packages-ops/v0.2");
    assert_eq!(plan["kind"], "plan");
    assert_eq!(plan["projectPath"], project.to_string_lossy().as_ref());
    // The frozen camelCase projection of the port facts, including the
    // conflict-triggered remove row on the install face (ORC-WF-002).
    assert_eq!(plan["digest"], FAKE_DIGEST);
    assert_eq!(plan["destructive"], serde_json::Value::Bool(true));
    let items = plan["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["kind"], "install");
    assert_eq!(items[0]["packageId"], "com.another.toolkit");
    assert_eq!(items[0]["version"], "3.1.4");
    assert_eq!(items[1]["reason"], "transitive_dependency");
    assert_eq!(items[2]["kind"], "remove");
    assert_eq!(items[2]["packageId"], "com.example.conflictingpackage");
    assert_eq!(items[2]["version"], serde_json::Value::Null);
    assert_eq!(items[2]["reason"], "conflict");
    assert_eq!(
        plan["removeLegacyFolders"][0], "Assets/OldPackage",
        "legacy folder cleanups travel verbatim"
    );

    let row = install_ops_capability_row(&database, Some(Arc::new(FakeInstall::wired())));
    assert_eq!(row["availability"], "available");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn install_preview_unregistered_path_reuses_the_013_not_found_code() {
    let root = unique_root("unregistered");
    let (_project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeInstall::wired())),
        "query",
        "packages.previewInstall",
        install_params("C:/not/registered"),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.project.project_not_found");
    assert_eq!(payload["error"]["category"], "validation");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn install_preview_param_violations_answer_invalid_params() {
    let root = unique_root("params");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm = Some(Arc::new(FakeInstall::wired()) as Arc<dyn VpmBackend>);
    let project_path = project.to_string_lossy().into_owned();

    let violations: Vec<Value> = vec![
        // A preview request carrying a digest: the digest is the preview's
        // product — carrying one is a shape violation.
        json!({
            "projectPath": project_path,
            "packages": [{ "packageId": "com.example.pkg", "version": null }],
            "confirmedDigest": FAKE_DIGEST,
        }),
        // Empty packages: no wildcard, no "install everything" shorthand.
        json!({ "projectPath": project_path, "packages": [] }),
        // A repeated packageId across rows is a word-face violation even
        // when the versions differ.
        json!({
            "projectPath": project_path,
            "packages": [
                { "packageId": "com.a", "version": null },
                { "packageId": "com.a", "version": "1.0.0" },
            ],
        }),
        // version is REQUIRED and nullable — an absent key is a violation.
        json!({
            "projectPath": project_path,
            "packages": [{ "packageId": "com.example.pkg" }],
        }),
        // version must be a string or null — a number is a violation.
        json!({
            "projectPath": project_path,
            "packages": [{ "packageId": "com.example.pkg", "version": 3 }],
        }),
        // Unknown key: the closed param set admits nothing else.
        json!({
            "projectPath": project_path,
            "packages": [{ "packageId": "com.a", "version": null }],
            "wildcard": true,
        }),
    ];
    for params in violations {
        let payload = run_frame(
            &database,
            Some(&config),
            vpm.clone(),
            "query",
            "packages.previewInstall",
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
fn install_preview_without_the_capability_answers_the_generic_capability_missing() {
    let root = unique_root("uncapped");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm = FakeInstall {
        install_cap: false,
        preview_failure: None,
        apply_failure: None,
    };

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "query",
        "packages.previewInstall",
        install_params(project.to_string_lossy().as_ref()),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.capability_missing");
    assert_eq!(payload["error"]["category"], "unavailable");

    let row = install_ops_capability_row(
        &database,
        Some(Arc::new(FakeInstall {
            install_cap: false,
            preview_failure: None,
            apply_failure: None,
        })),
    );
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn install_preview_no_matching_package_projects_onto_package_not_found() {
    let root = unique_root("no-matching");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm = FakeInstall {
        install_cap: true,
        preview_failure: Some(AppErrorV1::new(
            "vua.vpm.no_matching_package",
            ErrorCategory::Dependency,
            "errors.vpm.noMatchingPackage",
            "corr-fake",
        )),
        apply_failure: None,
    };

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "query",
        "packages.previewInstall",
        install_params(project.to_string_lossy().as_ref()),
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
fn install_preview_failure_folds_into_the_preview_failed_envelope_code() {
    let root = unique_root("preview-failed");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm = FakeInstall {
        install_cap: true,
        preview_failure: Some(AppErrorV1::new(
            "vua.vpm.preview_failed",
            ErrorCategory::ExternalFailure,
            "errors.vpm.previewFailed",
            "corr-fake",
        )),
        apply_failure: None,
    };

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(vpm)),
        "query",
        "packages.previewInstall",
        install_params(project.to_string_lossy().as_ref()),
    );
    assert_eq!(payload["ok"], false);
    // The one new envelope-face code the A2 freeze batch declared.
    assert_eq!(payload["error"]["code"], "vua.packages.preview_failed");
    assert_eq!(
        payload["error"]["messageKey"],
        "errors.packages.previewFailed"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn install_receipt_travels_inside_the_task_done_payload() {
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
        Some(Arc::new(FakeInstall::wired())),
        "command",
        "packages.applyInstall",
        json!({
            "projectPath": project.to_string_lossy(),
            "packages": [
                { "packageId": "com.lilxyzw.liltoon", "version": null },
                { "packageId": "com.another.toolkit", "version": "3.1.4" },
            ],
            "confirmedDigest": FAKE_DIGEST,
        }),
    );
    assert_eq!(payload["ok"], true, "the wired command accepts: {payload}");
    let acceptance = &payload["value"];
    assert_eq!(acceptance["operation"], "packages.applyInstall");
    let task_id = acceptance["taskId"].as_str().expect("taskId").to_owned();
    assert!(!acceptance["correlationId"]
        .as_str()
        .unwrap_or_default()
        .is_empty());

    let done = wait_done(&database, &task_id);
    assert_eq!(done["schemaVersion"], "0.2");
    assert_eq!(done["operation"], "packages.applyInstall");
    assert!(
        validator.is_valid(&done),
        "receipt must match the frozen result schema: {done}"
    );
    let receipt = &done["result"];
    assert_eq!(receipt["schemaVersion"], "vua.packages-ops/v0.2");
    assert_eq!(receipt["kind"], "receipt");
    // The audit receipt: the confirmed digest echo + the request rows
    // verbatim (the version-selection semantics included) + the actually
    // applied items (port facts verbatim).
    assert_eq!(receipt["confirmedDigest"], FAKE_DIGEST);
    let requested = receipt["requestedPackages"]
        .as_array()
        .expect("requested rows");
    assert_eq!(requested.len(), 2);
    assert_eq!(requested[0]["packageId"], "com.lilxyzw.liltoon");
    assert_eq!(requested[0]["version"], serde_json::Value::Null);
    assert_eq!(requested[1]["packageId"], "com.another.toolkit");
    assert_eq!(requested[1]["version"], "3.1.4");
    let applied = receipt["appliedItems"].as_array().expect("applied items");
    assert_eq!(applied.len(), 3);
    assert_eq!(applied[0]["packageId"], "com.another.toolkit");
    assert_eq!(applied[2]["kind"], "remove");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn install_digest_drift_refuses_inside_the_task_as_preview_drift() {
    let validator = ops_result_validator();
    let root = unique_root("drift");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeInstall::wired())),
        "command",
        "packages.applyInstall",
        json!({
            "projectPath": project.to_string_lossy(),
            "packages": [{ "packageId": "com.lilxyzw.liltoon", "version": null }],
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
fn install_apply_failure_folds_into_execution_failed_with_port_provenance() {
    let root = unique_root("apply-failed");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm = FakeInstall {
        install_cap: true,
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
        "packages.applyInstall",
        json!({
            "projectPath": project.to_string_lossy(),
            "packages": [{ "packageId": "com.lilxyzw.liltoon", "version": null }],
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
fn install_apply_unregistered_path_refuses_at_the_route_layer_never_inside_a_task() {
    let root = unique_root("apply-unregistered");
    let (_project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let payload = run_frame(
        &database,
        Some(&config),
        Some(Arc::new(FakeInstall::wired())),
        "command",
        "packages.applyInstall",
        json!({
            "projectPath": "C:/not/registered",
            "packages": [{ "packageId": "com.example.pkg", "version": null }],
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
