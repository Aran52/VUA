//! Packages wire tests (proposal 024 P1, the packages-query v0.1 read
//! face): the `packages.listInstalled` route rides the real frame loop over
//! the frozen word list — registration validated against the SAME 013
//! inspection aggregate `project.inspectProject` uses (the reused
//! `vua.project.project_not_found`), the listing projected from a fake
//! `VpmBackend` whose shapes are pinned by
//! `schemas/packages-query/v0.1/`, and the honest absence face when no
//! engine is assembled. Changing that word list without this consumer
//! fails here first. Everything runs against synthetic directory trees.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, ErrorCategory, InstalledPackageV1, PackageRequestV1, ProjectRef, VpmBackend,
    VpmCapabilities,
};
use vua_project_manager::ManagerRoots;
use vua_provider_host::{run_provider_host_full, ProjectOpsConfig};

fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-packages-wire-{label}-{}-{nanos}",
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

/// A deterministic fake engine: the word-list freeze concerns the WIRE, so
/// the wire test pins routing and projection, not the vrc-get library (the
/// core-domain consumer test packages_query_consumer pins the library
/// shapes against the same schemas).
struct FakeVpm {
    list_packages_cap: bool,
    listing: Vec<InstalledPackageV1>,
    failure: Option<AppErrorV1>,
}

impl FakeVpm {
    fn listing(list_packages_cap: bool) -> Self {
        Self {
            list_packages_cap,
            listing: vec![
                InstalledPackageV1 {
                    package_id: "com.vrchat.avatars".into(),
                    version: "3.7.4".into(),
                    dependencies: vec!["com.vrchat.base".into()],
                },
                InstalledPackageV1 {
                    package_id: "com.vrchat.base".into(),
                    version: "3.7.4".into(),
                    dependencies: vec![],
                },
            ],
            failure: None,
        }
    }
    fn failing() -> Self {
        Self {
            list_packages_cap: true,
            listing: vec![],
            failure: Some(AppErrorV1::new(
                "vua.vpm.project_load_failed",
                ErrorCategory::ExternalFailure,
                "errors.vpm.projectLoadFailed",
                "corr-fake-vpm",
            )),
        }
    }
}

impl VpmBackend for FakeVpm {
    fn name(&self) -> &'static str {
        "fake-vpm"
    }
    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: false,
            preview_install: false,
            list_packages: self.list_packages_cap,
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
        Err(AppErrorV1::new(
            "vua.vpm.capability_missing",
            ErrorCategory::Unavailable,
            "errors.vpm.capabilityMissing",
            "corr-fake-vpm",
        ))
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        _confirmed_digest: &str,
    ) -> Result<Value, AppErrorV1> {
        Err(AppErrorV1::new(
            "vua.vpm.capability_missing",
            ErrorCategory::Unavailable,
            "errors.vpm.capabilityMissing",
            "corr-fake-vpm",
        ))
    }
    fn create_project(
        &self,
        _parent: &Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
        Err(AppErrorV1::new(
            "vua.vpm.capability_missing",
            ErrorCategory::Unavailable,
            "errors.vpm.capabilityMissing",
            "corr-fake-vpm",
        ))
    }
    fn list_packages(&self, _project: &ProjectRef) -> Result<Vec<InstalledPackageV1>, AppErrorV1> {
        match &self.failure {
            Some(error) => Err(error.clone()),
            None => Ok(self.listing.clone()),
        }
    }
}

/// Sends one query frame through the real host loop and returns the
/// response payload.
fn run_query_frame(
    database: &Path,
    config: Option<&ProjectOpsConfig>,
    vpm: Option<Arc<dyn VpmBackend>>,
    method: &str,
    params: Value,
) -> Value {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-packages",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages",
            "correlationId": "corr-packages",
            "kind": "query",
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

/// Sends `application.getSnapshot` and extracts the packages capability row.
fn packages_capability_row(database: &Path, vpm: Option<Arc<dyn VpmBackend>>) -> Value {
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
        .find(|row| row["operationId"] == "packages.query")
        .cloned()
        .expect("the packages.query capability row exists")
}

#[test]
fn absent_wiring_answers_the_typed_honest_absence_and_unavailable_capability() {
    let root = unique_root("absent");
    let database = root.join("tasks.sqlite");

    let payload = run_query_frame(
        &database,
        None,
        None,
        "packages.listInstalled",
        json!({ "projectPath": "C:/anywhere" }),
    );
    assert_eq!(payload["ok"], false, "absence is a typed failure");
    assert_eq!(payload["error"]["code"], "vua.packages.unavailable");
    assert_eq!(payload["error"]["category"], "unavailable");
    assert_eq!(
        payload["error"]["messageKey"],
        "errors.packages.unavailable"
    );
    assert!(
        payload.get("result").is_none(),
        "absence never carries a fabricated listing"
    );

    let row = packages_capability_row(&database, None);
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_route_projects_the_fake_listing_inside_the_frozen_envelope() {
    let root = unique_root("wired");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm: Arc<dyn VpmBackend> = Arc::new(FakeVpm::listing(true));

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(vpm.clone()),
        "packages.listInstalled",
        json!({ "projectPath": project.to_string_lossy() }),
    );
    assert_eq!(
        payload["ok"], true,
        "wired + registered must succeed: {payload}"
    );
    let value = &payload["value"];
    assert_eq!(value["schemaVersion"], "0.1");
    assert_eq!(value["operation"], "packages.listInstalled");
    let result = &value["result"];
    assert_eq!(result["schemaVersion"], "vua.packages-installed/v0.1");
    assert_eq!(
        result["projectPath"],
        project.to_string_lossy().into_owned()
    );
    let rows = result["packages"].as_array().expect("package rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["packageId"], "com.vrchat.avatars");
    assert_eq!(rows[0]["version"], "3.7.4");
    assert_eq!(rows[0]["dependencies"], json!(["com.vrchat.base"]));
    assert_eq!(rows[1]["packageId"], "com.vrchat.base");

    let row = packages_capability_row(&database, Some(vpm));
    assert_eq!(row["availability"], "available");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn unregistered_path_reuses_the_013_not_found_code() {
    let root = unique_root("not-found");
    let (_, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm: Arc<dyn VpmBackend> = Arc::new(FakeVpm::listing(true));

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(vpm),
        "packages.listInstalled",
        json!({ "projectPath": root.join("unregistered").to_string_lossy() }),
    );
    assert_eq!(payload["ok"], false);
    // Same fact, same code: the reused 013 typed not-found (validation).
    assert_eq!(payload["error"]["code"], "vua.project.project_not_found");
    assert_eq!(payload["error"]["category"], "validation");
    assert_eq!(
        payload["error"]["messageKey"],
        "errors.project.projectNotFound"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn params_violations_answer_invalid_params_never_absence() {
    let root = unique_root("params");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm: Arc<dyn VpmBackend> = Arc::new(FakeVpm::listing(true));

    for params in [
        json!({}),
        json!({ "projectPath": "" }),
        json!({ "projectPath": project.to_string_lossy(), "includePrerelease": true }),
    ] {
        let payload = run_query_frame(
            &database,
            Some(&config),
            Some(vpm.clone()),
            "packages.listInstalled",
            params.clone(),
        );
        assert_eq!(
            payload["error"]["code"], "vua.packages.invalid_params",
            "{params}"
        );
        assert_eq!(payload["error"]["category"], "validation");
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn backend_without_the_capability_answers_the_port_capability_missing_code() {
    let root = unique_root("capability");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm: Arc<dyn VpmBackend> = Arc::new(FakeVpm::listing(false));

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(vpm),
        "packages.listInstalled",
        json!({ "projectPath": project.to_string_lossy() }),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.capability_missing");
    assert_eq!(payload["error"]["category"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn backend_failure_travels_as_the_typed_error_never_an_empty_masquerade() {
    let root = unique_root("failure");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm: Arc<dyn VpmBackend> = Arc::new(FakeVpm::failing());

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(vpm),
        "packages.listInstalled",
        json!({ "projectPath": project.to_string_lossy() }),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.project_load_failed");
    assert_eq!(payload["error"]["category"], "external_failure");
    fs::remove_dir_all(&root).ok();
}
