//! Packages P2 wire tests (proposal 025 freeze batch, the packages-repos +
//! packages-catalog v0.1 read faces): the `packages.listRepos` /
//! `packages.packageCatalog` routes ride the real frame loop over the frozen
//! word list — the catalog capability declaration (`catalog_capabilities`,
//! default declared-none) gates both faces, the per-package registration is
//! validated against the SAME 013 inspection aggregate the P1 face uses
//! (the reused `vua.project.project_not_found`), the subscription list is a
//! GLOBAL face needing no registration binding, and the projections are
//! pinned by `schemas/packages-repos/v0.1/` and `schemas/packages-catalog/
//! v0.1/` (the contract-side consumer loop lives in packages_p2_consumer).
//! The capability rows flip with the catalog declaration — an engine wired
//! without the P2 implementation keeps the rows honestly unavailable. The
//! REAL backend consumption is the environment implementation slice.
//! Everything runs against synthetic data — no machine-specific facts, no
//! network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, CatalogCapabilities, CatalogVersionV01, ErrorCategory, PackageCatalogV01,
    PackageRequestV1, PackageSourceV01, ProjectRef, RepoInfoV01, VpmBackend, VpmCapabilities,
};
use vua_project_manager::ManagerRoots;
use vua_provider_host::{run_provider_host_full, ProjectOpsConfig};

fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-packages-p2-wire-{label}-{}-{nanos}",
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

/// A deterministic fake engine implementing the P2 faces behind the frozen
/// catalog capability declaration. `catalog_cap: false` models the default
/// declared-none backend (the trait-default absence arms — the faces never
/// answer success without the declaration).
struct CatalogVpm {
    catalog_cap: bool,
    repos: Vec<RepoInfoV01>,
    catalog: PackageCatalogV01,
    repos_failure: Option<AppErrorV1>,
    catalog_failure: Option<AppErrorV1>,
}

impl CatalogVpm {
    fn declaring() -> Self {
        Self {
            catalog_cap: true,
            repos: vec![
                RepoInfoV01 {
                    repo_id: Some("official".into()),
                    name: Some("Official".into()),
                    url: Some("https://vpm.example/vpm.json".into()),
                    local_path: Some("C:\\cache\\official.json".into()),
                    cached: true,
                },
                RepoInfoV01 {
                    repo_id: None,
                    name: Some("Curated".into()),
                    url: Some("https://vpm.example/curated.json".into()),
                    local_path: None,
                    cached: false,
                },
                RepoInfoV01 {
                    repo_id: Some("local-tools".into()),
                    name: None,
                    url: None,
                    local_path: Some("D:\\vpm-local\\repo.json".into()),
                    cached: true,
                },
            ],
            catalog: PackageCatalogV01 {
                project_path: String::new(), // stamped from the request below
                package_id: "com.anatawa12.avatar-optimizer".into(),
                display_name: Some("Avatar Optimizer".into()),
                source: PackageSourceV01::Repo,
                installed: true,
                update_available: Some(true),
                versions: vec![
                    CatalogVersionV01 {
                        version: "1.6.2".into(),
                        yanked: true,
                        compatible: Some(true),
                    },
                    CatalogVersionV01 {
                        version: "1.9.0-beta.1".into(),
                        yanked: false,
                        compatible: None,
                    },
                ],
            },
            repos_failure: None,
            catalog_failure: None,
        }
    }
    fn undeclaring() -> Self {
        let mut fake = Self::declaring();
        fake.catalog_cap = false;
        fake
    }
    fn repos_failing() -> Self {
        let mut fake = Self::declaring();
        fake.repos_failure = Some(AppErrorV1::new(
            "vua.vpm.backend_unavailable",
            ErrorCategory::Unavailable,
            "errors.vpm.backendUnavailable",
            "corr-fake-catalog",
        ));
        fake
    }
    fn catalog_failing() -> Self {
        let mut fake = Self::declaring();
        fake.catalog_failure = Some(AppErrorV1::new(
            "vua.vpm.no_matching_package",
            ErrorCategory::Validation,
            "errors.vpm.noMatchingPackage",
            "corr-fake-catalog",
        ));
        fake
    }
}

impl VpmBackend for CatalogVpm {
    fn name(&self) -> &'static str {
        "fake-catalog-vpm"
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
    fn catalog_capabilities(&self) -> CatalogCapabilities {
        CatalogCapabilities {
            catalog: self.catalog_cap,
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
            "corr-fake-catalog",
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
            "corr-fake-catalog",
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
            "corr-fake-catalog",
        ))
    }
    fn list_repos(&self) -> Result<Vec<RepoInfoV01>, AppErrorV1> {
        match &self.repos_failure {
            Some(error) => Err(error.clone()),
            None => Ok(self.repos.clone()),
        }
    }
    fn package_catalog(
        &self,
        project: &ProjectRef,
        package_id: &str,
    ) -> Result<PackageCatalogV01, AppErrorV1> {
        if let Some(error) = &self.catalog_failure {
            return Err(error.clone());
        }
        let mut catalog = self.catalog.clone();
        catalog.project_path = project.root.to_string_lossy().to_string();
        catalog.package_id = package_id.to_owned();
        Ok(catalog)
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
        "frameId": "frame-packages-p2",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages-p2",
            "correlationId": "corr-packages-p2",
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

/// Sends `application.getSnapshot` and extracts one capability row by
/// operationId.
fn capability_row(database: &Path, vpm: Option<Arc<dyn VpmBackend>>, operation_id: &str) -> Value {
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
        .find(|row| row["operationId"] == operation_id)
        .cloned()
        .unwrap_or_else(|| panic!("the {operation_id} capability row exists"))
}

#[test]
fn absent_wiring_answers_the_typed_honest_absence_and_unavailable_rows() {
    let root = unique_root("absent");
    let database = root.join("tasks.sqlite");

    for method in ["packages.listRepos", "packages.packageCatalog"] {
        let payload = run_query_frame(
            &database,
            None,
            None,
            method,
            json!({ "projectPath": "C:/anywhere", "packageId": "com.example.pkg" }),
        );
        assert_eq!(payload["ok"], false, "{method}: absence is a typed failure");
        assert_eq!(payload["error"]["code"], "vua.packages.unavailable");
        assert_eq!(payload["error"]["category"], "unavailable");
        assert!(
            payload.get("result").is_none(),
            "absence never carries a fabricated result"
        );
    }

    let repos_row = capability_row(&database, None, "packages.listRepos");
    let catalog_row = capability_row(&database, None, "packages.packageCatalog");
    assert_eq!(repos_row["availability"], "unavailable");
    assert_eq!(catalog_row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_routes_project_facts_inside_the_frozen_envelopes() {
    let root = unique_root("wired");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm: Arc<dyn VpmBackend> = Arc::new(CatalogVpm::declaring());

    // listRepos: the frozen envelope, nullable facts projected verbatim,
    // the REQUIRED cached fact on every row, subscription-face order kept.
    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(vpm.clone()),
        "packages.listRepos",
        json!({}),
    );
    assert_eq!(
        payload["ok"], true,
        "wired + declared must succeed: {payload}"
    );
    let value = &payload["value"];
    assert_eq!(value["schemaVersion"], "0.1");
    assert_eq!(value["operation"], "packages.listRepos");
    let result = &value["result"];
    assert_eq!(result["schemaVersion"], "vua.packages-repos/v0.1");
    let rows = result["repos"].as_array().expect("repo rows");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["repoId"], "official");
    assert_eq!(rows[0]["cached"], true);
    // Null = the honest library-Option projection (never padded).
    assert_eq!(rows[1]["repoId"], Value::Null);
    assert_eq!(
        rows[1]["cached"], false,
        "subscribed-never-refreshed rides as its own state"
    );
    assert_eq!(
        rows[2]["url"],
        Value::Null,
        "a local-directory repo carries no url"
    );
    assert_eq!(rows[2]["name"], Value::Null);

    // packageCatalog: the frozen envelope, the family const stamped by the
    // route, the two-state source word, the null-compatible judgment.
    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(vpm.clone()),
        "packages.packageCatalog",
        json!({
            "projectPath": project.to_string_lossy(),
            "packageId": "com.anatawa12.avatar-optimizer",
        }),
    );
    assert_eq!(
        payload["ok"], true,
        "registered + declared must succeed: {payload}"
    );
    let value = &payload["value"];
    assert_eq!(value["schemaVersion"], "0.1");
    assert_eq!(value["operation"], "packages.packageCatalog");
    let result = &value["result"];
    assert_eq!(result["schemaVersion"], "vua.packages-catalog/v0.1");
    assert_eq!(
        result["projectPath"],
        project.to_string_lossy().into_owned()
    );
    assert_eq!(result["packageId"], "com.anatawa12.avatar-optimizer");
    assert_eq!(result["displayName"], "Avatar Optimizer");
    assert_eq!(result["source"], "repo");
    assert_eq!(result["installed"], true);
    assert_eq!(result["updateAvailable"], true);
    let versions = result["versions"].as_array().expect("version rows");
    assert_eq!(versions[0]["yanked"], true);
    assert_eq!(
        versions[1]["compatible"],
        Value::Null,
        "unknown project Unity version = judgment not executed, never false"
    );

    // Both capability rows are available with the declaration.
    let repos_row = capability_row(&database, Some(vpm.clone()), "packages.listRepos");
    let catalog_row = capability_row(&database, Some(vpm), "packages.packageCatalog");
    assert_eq!(repos_row["availability"], "available");
    assert_eq!(catalog_row["availability"], "available");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn empty_subscription_list_is_an_honest_valid_answer() {
    let root = unique_root("empty-repos");
    let database = root.join("tasks.sqlite");
    let mut fake = CatalogVpm::declaring();
    fake.repos = vec![];
    let vpm: Arc<dyn VpmBackend> = Arc::new(fake);

    let payload = run_query_frame(&database, None, Some(vpm), "packages.listRepos", json!({}));
    assert_eq!(
        payload["ok"], true,
        "zero subscriptions is a real backend fact"
    );
    assert_eq!(payload["value"]["result"]["repos"], json!([]));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn the_subscription_list_needs_no_registration_binding() {
    // The subscription face is GLOBAL configuration: it works without any
    // 013 project-ops wiring at all (only the engine + declaration matter).
    let root = unique_root("global-face");
    let database = root.join("tasks.sqlite");
    let vpm: Arc<dyn VpmBackend> = Arc::new(CatalogVpm::declaring());

    let payload = run_query_frame(&database, None, Some(vpm), "packages.listRepos", json!({}));
    assert_eq!(
        payload["ok"], true,
        "the global face never needs the 013 aggregate: {payload}"
    );
    assert_eq!(
        payload["value"]["result"]["repos"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn undeclared_catalog_capability_answers_capability_missing_and_rows_stay_unavailable() {
    // The P2 discipline differs from P1: wiring the engine alone is NOT
    // enough — the rows flip only with the catalog declaration, and the
    // routes answer the port capability-missing code, never a stub.
    let root = unique_root("undeclared");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm: Arc<dyn VpmBackend> = Arc::new(CatalogVpm::undeclaring());

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(vpm.clone()),
        "packages.listRepos",
        json!({}),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.capability_missing");
    assert_eq!(payload["error"]["category"], "unavailable");

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(vpm.clone()),
        "packages.packageCatalog",
        json!({
            "projectPath": project.to_string_lossy(),
            "packageId": "com.anatawa12.avatar-optimizer",
        }),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.capability_missing");

    let repos_row = capability_row(&database, Some(vpm.clone()), "packages.listRepos");
    let catalog_row = capability_row(&database, Some(vpm), "packages.packageCatalog");
    assert_eq!(repos_row["availability"], "unavailable");
    assert_eq!(catalog_row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn unregistered_catalog_path_reuses_the_013_not_found_code() {
    let root = unique_root("not-found");
    let (_, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm: Arc<dyn VpmBackend> = Arc::new(CatalogVpm::declaring());

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(vpm),
        "packages.packageCatalog",
        json!({
            "projectPath": root.join("unregistered").to_string_lossy(),
            "packageId": "com.anatawa12.avatar-optimizer",
        }),
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
    let vpm: Arc<dyn VpmBackend> = Arc::new(CatalogVpm::declaring());

    // listRepos: the global face accepts the EMPTY params object only.
    for params in [
        json!({ "projectPath": "C:/anywhere" }),
        json!({ "includePrerelease": true }),
    ] {
        let payload = run_query_frame(
            &database,
            Some(&config),
            Some(vpm.clone()),
            "packages.listRepos",
            params.clone(),
        );
        assert_eq!(
            payload["error"]["code"], "vua.packages.invalid_params",
            "{params}"
        );
        assert_eq!(payload["error"]["category"], "validation");
    }

    // packageCatalog: exactly the two-key closed set, both non-empty.
    for params in [
        json!({}),
        json!({ "projectPath": project.to_string_lossy() }),
        json!({ "packageId": "com.anatawa12.avatar-optimizer" }),
        json!({ "projectPath": "", "packageId": "com.anatawa12.avatar-optimizer" }),
        json!({ "projectPath": project.to_string_lossy(), "packageId": "" }),
        json!({
            "projectPath": project.to_string_lossy(),
            "packageId": "com.anatawa12.avatar-optimizer",
            "includePrerelease": true,
        }),
    ] {
        let payload = run_query_frame(
            &database,
            Some(&config),
            Some(vpm.clone()),
            "packages.packageCatalog",
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
fn backend_typed_failures_travel_verbatim_never_an_empty_masquerade() {
    let root = unique_root("failures");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(Arc::new(CatalogVpm::repos_failing())),
        "packages.listRepos",
        json!({}),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.backend_unavailable");
    assert_eq!(payload["error"]["category"], "unavailable");

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(Arc::new(CatalogVpm::catalog_failing())),
        "packages.packageCatalog",
        json!({
            "projectPath": project.to_string_lossy(),
            "packageId": "com.example.absent",
        }),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.no_matching_package");
    fs::remove_dir_all(&root).ok();
}
