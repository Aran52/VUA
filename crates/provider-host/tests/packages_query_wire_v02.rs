//! packages-query v0.2 wire tests (proposal 027 F3 wiring, 2026-09-20):
//! the `packages.listInstalled` route's additive dual-version negotiation
//! rides the real frame loop over the frozen
//! `schemas/packages-query/v0.2/` word list. The face-level gate
//! (`capabilities().list_packages`) precedes the negotiation — without the
//! face there is no word face at all, whatever its generation; a backend
//! declaring `query_v02` is answered at the v0.2 result family (rows carry
//! the REQUIRED judgment pair, the listing carries the REQUIRED
//! cacheSourced disclosure, projectPath is the ROUTE's envelope-assembly
//! fact stamped over the backend's verbatim facts); every other backend —
//! including the trait-default `query_v02 -> false` — keeps answering the
//! frozen v0.1 family (the zero-regression pin: the v0.1 word face is
//! never revised in place). Both family consts are named core-owned
//! constants stamped at envelope assembly (the A3/A4/A5/F2 precedent —
//! the consumer reads the const, never guesses the word face); a v0.1-
//! shaped row is INVALID under the v0.2 schema, which is what makes the
//! version machine-detectable. The P1 shared preconditions are re-pinned
//! under a v0.2-declaring backend: registration reuse
//! (`vua.project.project_not_found`), closed params
//! (`vua.packages.invalid_params`), and verbatim typed refusals. The
//! REAL backend consumption (`VrcGetLibBackend::list_packages_v02`) is the
//! environment implementation slice; the contract-side consumer loop lives
//! in `packages_query_consumer_v02`. Everything runs against synthetic
//! directory trees — no machine-specific facts, no network, no user VCC
//! home.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, ErrorCategory, InstalledListingV02, InstalledPackageV02, InstalledPackageV1,
    PackageRequestV1, ProjectRef, VpmBackend, VpmCapabilities,
};
use vua_project_manager::ManagerRoots;
use vua_provider_host::{
    provider_host::{
        PACKAGES_INSTALLED_SCHEMA_VERSION_V01, PACKAGES_INSTALLED_SCHEMA_VERSION_V02,
        PACKAGES_QUERY_SCHEMA_VERSION,
    },
    run_provider_host_full, ProjectOpsConfig,
};

fn read_query_schema(version: &str, name: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .join("../..")
        .join("schemas/packages-query")
        .join(version)
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn result_validator(version: &str) -> jsonschema::Validator {
    jsonschema::validator_for(&read_query_schema(version, "result.schema.json")).unwrap()
}

fn schema_violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-packages-query-wire-v02-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// Seeds a minimal real Unity project plus a VCC settings.json registering
/// it (the 013 aggregate's world — the same P1 shared precondition, re-used
/// verbatim so the negotiation arms ride the identical gate chain).
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

/// A deterministic fake engine speaking BOTH generations of the
/// installed-set read face. `declare_v02: false` models the trait-default
/// backend (the negotiation's v0.1 arm — the default accessor law); the
/// v0.1 body panics if the route ever reaches it with `declare_v02` true,
/// and the v0.2 body panics on the reverse, pinning that exactly one arm
/// serves each negotiation outcome.
struct QueryV02Vpm {
    list_packages_cap: bool,
    declare_v02: bool,
    listing_v01: Vec<InstalledPackageV1>,
    listing_v02: InstalledListingV02,
    failure: Option<AppErrorV1>,
}

impl QueryV02Vpm {
    fn v01_only() -> Self {
        Self {
            list_packages_cap: true,
            declare_v02: false,
            listing_v01: vec![
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
            listing_v02: InstalledListingV02 {
                packages: vec![],
                cache_sourced: false,
            },
            failure: None,
        }
    }

    fn declaring_v02() -> Self {
        let mut fake = Self::v01_only();
        fake.declare_v02 = true;
        fake.listing_v02 = InstalledListingV02 {
            // Every arm of the frozen judgment word face: judged-true,
            // judged-false, judgment-not-executed (local source — the null
            // pair), and the prerelease-boundary false (installed
            // prerelease newer than the stable-set qualifying latest;
            // `false` = no strictly newer version under the CURRENT
            // filter, never a generalized "no update"). packageId
            // ascending — the frozen presentation fact.
            packages: vec![
                InstalledPackageV02 {
                    package_id: "com.anatawa12.avatar-optimizer".into(),
                    version: "1.7.0".into(),
                    dependencies: vec!["com.anatawa12.gists".into()],
                    latest_version: Some("1.8.2".into()),
                    update_available: Some(true),
                },
                InstalledPackageV02 {
                    package_id: "com.demo.local-tool".into(),
                    version: "0.2.0".into(),
                    dependencies: vec![],
                    latest_version: None,
                    update_available: None,
                },
                InstalledPackageV02 {
                    package_id: "com.demo.prerelease-addon".into(),
                    version: "2.0.0-beta.4".into(),
                    dependencies: vec![],
                    latest_version: Some("1.9.0".into()),
                    update_available: Some(false),
                },
                InstalledPackageV02 {
                    package_id: "com.vrchat.base".into(),
                    version: "3.7.4".into(),
                    dependencies: vec![],
                    latest_version: Some("3.7.4".into()),
                    update_available: Some(false),
                },
            ],
            // The REQUIRED born-in disclosure: this fake answers through
            // the cache-degradation path.
            cache_sourced: true,
        };
        fake
    }

    fn without_face() -> Self {
        let mut fake = Self::declaring_v02();
        fake.list_packages_cap = false;
        fake
    }

    fn failing(code: &'static str) -> Self {
        let mut fake = Self::declaring_v02();
        fake.failure = Some(AppErrorV1::new(
            code,
            ErrorCategory::ExternalFailure,
            "errors.vpm.projectLoadFailed",
            "corr-fake-query-v02",
        ));
        fake
    }
}

impl VpmBackend for QueryV02Vpm {
    fn name(&self) -> &'static str {
        "fake-query-v02-wire"
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
        unreachable!("not exercised in this suite")
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        _confirmed_digest: &str,
    ) -> Result<Value, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn create_project(
        &self,
        _parent: &Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn query_v02(&self) -> bool {
        self.declare_v02
    }
    fn list_packages(&self, _project: &ProjectRef) -> Result<Vec<InstalledPackageV1>, AppErrorV1> {
        assert!(
            !self.declare_v02,
            "the v0.1 arm must never serve a query_v02-declaring backend"
        );
        assert!(
            self.list_packages_cap,
            "the face gate must answer capability_missing BEFORE the port"
        );
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(self.listing_v01.clone())
    }
    fn list_packages_v02(&self, _project: &ProjectRef) -> Result<InstalledListingV02, AppErrorV1> {
        assert!(
            self.declare_v02,
            "the v0.2 arm must never serve a backend that did not declare query_v02"
        );
        assert!(
            self.list_packages_cap,
            "the face gate must answer capability_missing BEFORE the port"
        );
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(self.listing_v02.clone())
    }
}

/// Sends one query frame through the real host loop and returns the
/// response payload.
fn run_query_frame(
    database: &Path,
    config: Option<&ProjectOpsConfig>,
    vpm: Option<Arc<dyn VpmBackend>>,
    params: Value,
) -> Value {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-packages-query-v02",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages-query-v02",
            "correlationId": "corr-packages-query-v02",
            "kind": "query",
            "method": "packages.listInstalled",
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

#[test]
fn a_v02_declaring_backend_is_answered_at_the_v02_family_inside_the_frozen_envelope() {
    let validator = result_validator("v0.2");
    let root = unique_root("v02");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(Arc::new(QueryV02Vpm::declaring_v02())),
        json!({ "projectPath": project.to_string_lossy() }),
    );
    assert_eq!(payload["ok"], true, "declared v0.2 must succeed: {payload}");
    let value = &payload["value"];
    // The envelope stays on the shared word-list-row generation (the frozen
    // v0.2 COMMAND schema locks "0.1" — the catalog v0.2 precedent).
    assert_eq!(value["schemaVersion"], PACKAGES_QUERY_SCHEMA_VERSION);
    assert_eq!(value["schemaVersion"], "0.1");
    assert_eq!(value["operation"], "packages.listInstalled");
    assert!(
        validator.is_valid(value),
        "the frozen v0.2 result schema must admit the response: {:?} {value}",
        schema_violations(&validator, value)
    );
    let result = &value["result"];
    assert_eq!(
        result["schemaVersion"],
        PACKAGES_INSTALLED_SCHEMA_VERSION_V02
    );
    // projectPath is the ROUTE's envelope-assembly fact, stamped over the
    // backend's verbatim facts (the P1 discipline).
    assert_eq!(
        result["projectPath"],
        project.to_string_lossy().into_owned()
    );
    assert_eq!(result["cacheSourced"], Value::Bool(true));
    let rows = result["packages"].as_array().expect("package rows");
    assert_eq!(rows.len(), 4);
    // Ascending pin (the frozen presentation fact) + every judgment arm:
    assert_eq!(rows[0]["packageId"], "com.anatawa12.avatar-optimizer");
    assert_eq!(rows[0]["latestVersion"], "1.8.2");
    assert_eq!(rows[0]["updateAvailable"], Value::Bool(true));
    // The null pair = judgment NOT executed (the local-source row): the
    // honest absence travels verbatim, never a default false.
    assert_eq!(rows[1]["packageId"], "com.demo.local-tool");
    assert_eq!(rows[1]["latestVersion"], Value::Null);
    assert_eq!(rows[1]["updateAvailable"], Value::Null);
    // The prerelease-boundary false: no strictly newer version under the
    // CURRENT filter — never a generalized "already latest".
    assert_eq!(rows[2]["updateAvailable"], Value::Bool(false));
    // The no-invented-fields pin: only the frozen five keys per row.
    for row in rows {
        let keys: Vec<&str> = row
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "packageId",
                "version",
                "dependencies",
                "latestVersion",
                "updateAvailable"
            ],
            "no invented fields at the v0.2 word face"
        );
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_trait_default_backend_keeps_answering_the_frozen_v01_family() {
    let validator = result_validator("v0.1");
    let root = unique_root("v01");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    // query_v02 stays at its trait default (false): the frozen v0.1 word
    // face keeps being served — the zero-regression pin.
    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(Arc::new(QueryV02Vpm::v01_only())),
        json!({ "projectPath": project.to_string_lossy() }),
    );
    assert_eq!(payload["ok"], true, "v0.1 must keep succeeding: {payload}");
    let value = &payload["value"];
    assert_eq!(value["schemaVersion"], PACKAGES_QUERY_SCHEMA_VERSION);
    let result = &value["result"];
    assert_eq!(
        result["schemaVersion"],
        PACKAGES_INSTALLED_SCHEMA_VERSION_V01
    );
    assert!(
        validator.is_valid(value),
        "the frozen v0.1 result schema must still admit the response: {:?} {value}",
        schema_violations(&validator, value)
    );
    let rows = result["packages"].as_array().expect("package rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["packageId"], "com.vrchat.avatars");
    // The v0.1 word face carries NO judgment facts and NO disclosure:
    assert!(rows[0].get("latestVersion").is_none());
    assert!(rows[0].get("updateAvailable").is_none());
    assert!(result.get("cacheSourced").is_none());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn the_face_gate_precedes_the_negotiation_and_answers_capability_missing() {
    let root = unique_root("gate");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);

    // A backend that DECLARES v0.2 but carries no face capability: the
    // face-level gate answers BEFORE the negotiation (the port bodies
    // panic if ever reached — see the arm asserts).
    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(Arc::new(QueryV02Vpm::without_face())),
        json!({ "projectPath": project.to_string_lossy() }),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.capability_missing");
    assert_eq!(payload["error"]["category"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn port_refusals_travel_verbatim_on_both_negotiation_arms() {
    let root = unique_root("verbatim");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let params = json!({ "projectPath": project.to_string_lossy() });

    // The v0.2 arm: the backend's typed refusal rides code + message key +
    // category verbatim — no read-face fold exists, on either generation.
    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(Arc::new(QueryV02Vpm::failing(
            "vua.vpm.project_load_failed",
        ))),
        params.clone(),
    );
    assert_eq!(
        payload["ok"], false,
        "the refusal travels as a typed failure"
    );
    assert_eq!(payload["error"]["code"], "vua.vpm.project_load_failed");
    assert_eq!(
        payload["error"]["messageKey"],
        "errors.vpm.projectLoadFailed"
    );
    assert_eq!(payload["error"]["category"], "external_failure");
    assert!(
        payload.get("result").is_none(),
        "a refusal never carries a fabricated listing"
    );

    // The v0.1 arm: the same verbatim discipline (the frozen v0.1 face's
    // error face is unchanged by the negotiation).
    let mut v01_failing = QueryV02Vpm::v01_only();
    v01_failing.failure = Some(AppErrorV1::new(
        "vua.vpm.project_load_failed",
        ErrorCategory::ExternalFailure,
        "errors.vpm.projectLoadFailed",
        "corr-fake-query-v01",
    ));
    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(Arc::new(v01_failing)),
        params,
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.project_load_failed");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn the_shared_p1_preconditions_precede_the_negotiation_under_a_v02_backend() {
    let root = unique_root("preconditions");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let vpm: Arc<dyn VpmBackend> = Arc::new(QueryV02Vpm::declaring_v02());

    // An off-aggregate path never reaches the backend, whatever the word
    // face generation (the reused 013 typed not-found).
    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(vpm.clone()),
        json!({ "projectPath": root.join("unregistered").to_string_lossy() }),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.project.project_not_found");
    assert_eq!(payload["error"]["category"], "validation");

    // The closed single-key params set: any shape violation answers
    // invalid_params at the route layer, before any arm negotiation.
    for params in [
        json!({}),
        json!({ "projectPath": "" }),
        json!({ "projectPath": project.to_string_lossy(), "includePrerelease": true }),
    ] {
        let payload = run_query_frame(&database, Some(&config), Some(vpm.clone()), params.clone());
        assert_eq!(
            payload["error"]["code"], "vua.packages.invalid_params",
            "{params}"
        );
        assert_eq!(payload["error"]["category"], "validation");
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn family_consts_are_detectable_and_match_the_frozen_schema_consts() {
    // The A3/A4/A5/F2 precedent closed at the wiring batch: both family
    // consts are named HERE and pinned against the frozen schema consts —
    // a consumer can detect the word face from the wire alone (the route
    // stamps the consts verbatim, never literals).
    let result_v01 = read_query_schema("v0.1", "result.schema.json");
    let result_v02 = read_query_schema("v0.2", "result.schema.json");
    let command_v02 = read_query_schema("v0.2", "command.schema.json");
    assert_eq!(
        result_v01["$defs"]["listInstalledResult"]["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_INSTALLED_SCHEMA_VERSION_V01),
        "the v0.1 family const equals the frozen v0.1 result def's const"
    );
    assert_eq!(
        PACKAGES_INSTALLED_SCHEMA_VERSION_V01,
        "vua.packages-installed/v0.1"
    );
    assert_eq!(
        result_v02["$defs"]["listInstalledResult"]["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_INSTALLED_SCHEMA_VERSION_V02),
        "the v0.2 family const equals the frozen v0.2 result def's const"
    );
    assert_eq!(
        PACKAGES_INSTALLED_SCHEMA_VERSION_V02,
        "vua.packages-installed/v0.2"
    );
    // The envelope generation is SHARED: the frozen v0.2 command schema
    // locks the same "0.1" const the v0.1 face serves (the catalog v0.2
    // precedent — only the result family moved).
    assert_eq!(
        command_v02["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_QUERY_SCHEMA_VERSION)
    );
    assert_eq!(PACKAGES_QUERY_SCHEMA_VERSION, "0.1");

    // The live stamps: each negotiation outcome carries its family const
    // (pinned against the constants, not against string literals).
    let root = unique_root("envelope");
    let (project, vcc_settings) = seed_registered_project(&root);
    let database = root.join("tasks.sqlite");
    let config = project_ops_config(&vcc_settings);
    let params = json!({ "projectPath": project.to_string_lossy() });

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(Arc::new(QueryV02Vpm::declaring_v02())),
        params.clone(),
    );
    assert_eq!(
        payload["value"]["schemaVersion"],
        PACKAGES_QUERY_SCHEMA_VERSION
    );
    assert_eq!(
        payload["value"]["result"]["schemaVersion"],
        PACKAGES_INSTALLED_SCHEMA_VERSION_V02
    );

    let payload = run_query_frame(
        &database,
        Some(&config),
        Some(Arc::new(QueryV02Vpm::v01_only())),
        params,
    );
    assert_eq!(
        payload["value"]["schemaVersion"],
        PACKAGES_QUERY_SCHEMA_VERSION
    );
    assert_eq!(
        payload["value"]["result"]["schemaVersion"],
        PACKAGES_INSTALLED_SCHEMA_VERSION_V01
    );
    fs::remove_dir_all(&root).ok();
}
