//! packages-repo-catalog v0.1 wire tests (proposal 027 F2 wiring,
//! 2026-09-20): the `packages.repoCatalog` read route rides the real frame
//! loop over the frozen `schemas/packages-repo-catalog/v0.1/` word list —
//! the NEW defaulted accessor `repo_catalog_capabilities` (default
//! declared-none) gates the route BEFORE the port call (absence answers the
//! generic `vua.vpm.capability_missing`; unlike the A5 face the port method
//! HAS a default body, so a declared-but-unimplemented backend CAN exist at
//! the type level — both layers answer `capability_missing`, the route gate
//! first), the closed two-key REQUIRED-nullable params cross verbatim
//! (null `repoId` = the port None over the whole collection world; a
//! non-null id scopes to one repo row; null `packageIds` = the port's empty
//! slice = no filter; a non-null array is the batch requirement-set filter),
//! every shape violation answers `vua.packages.invalid_params` at the route
//! layer, the port's typed refusals travel VERBATIM (the P2 read-face
//! pass-through discipline — the reused `vua.vpm.repo_not_found` rides this
//! channel, the A4 removeRepo same-fact precedent), and the projections are
//! pinned by the frozen schemas (the contract-side consumer loop lives in
//! `packages_repo_catalog_consumer_v01`). The served row
//! `packages.repoCatalogOps` flips ONLY with the accessor declaration —
//! default declared-none keeps it honestly unavailable until the
//! environment implementation-verification slice flips it with the
//! VrcGetLib override (the A4 accessor law; the CLI backend has no
//! repo-scale listing and stays honestly false). The REAL backend
//! consumption is the environment implementation slice. Everything runs
//! against synthetic values — no machine-specific facts, no network, no
//! user VCC home.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, ErrorCategory, PackageRequestV1, ProjectRef, RepoCatalogCapabilities,
    RepoCatalogPackageV01, RepoCatalogRepoV01, RepoCatalogV01, VpmBackend, VpmCapabilities,
};
use vua_provider_host::{
    provider_host::{
        PACKAGES_REPO_CATALOG_ENVELOPE_SCHEMA_VERSION_V01, PACKAGES_REPO_CATALOG_SCHEMA_VERSION_V01,
    },
    run_provider_host_full,
};

fn read_repo_catalog_schema(name: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .join("../..")
        .join("schemas/packages-repo-catalog/v0.1")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn repo_catalog_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_catalog_schema("result.schema.json")).unwrap()
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
        "vua-packages-repo-catalog-wire-v01-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// A deterministic fake engine implementing the F2 read face behind the
/// frozen capability declaration. `repo_catalog_cap: false` models the
/// default declared-none backend (the trait-default absence arm — the route
/// must answer the gate BEFORE the port, so the port body panics if it is
/// ever reached with the bit false). Per-case failure injection and
/// expected-transport assertions pin the verbatim param crossing.
struct RepoCatalogVpm {
    repo_catalog_cap: bool,
    catalog: Option<RepoCatalogV01>,
    failure: Option<AppErrorV1>,
    /// When set, the port must receive exactly (repo_id, package_ids):
    /// null params cross as (None, empty slice), non-null params verbatim.
    expect_transport: Option<(Option<String>, Vec<String>)>,
}

impl RepoCatalogVpm {
    fn declaring() -> Self {
        Self {
            repo_catalog_cap: true,
            catalog: Some(RepoCatalogV01 {
                repos: vec![
                    RepoCatalogRepoV01 {
                        repo_id: Some("official".into()),
                        name: Some("Official".into()),
                        cached: true,
                        packages: vec![
                            RepoCatalogPackageV01 {
                                package_id: "com.anatawa12.avatar-optimizer".into(),
                                display_name: Some("Avatar Optimizer".into()),
                                description: Some("Optimization tools for avatars".into()),
                                latest_version: Some("1.7.0".into()),
                                version_count: 14,
                            },
                            // The honest absence arms: all-yanked row (null
                            // latest), manifest-missing display/description.
                            RepoCatalogPackageV01 {
                                package_id: "com.example.all-yanked".into(),
                                display_name: None,
                                description: None,
                                latest_version: None,
                                version_count: 3,
                            },
                        ],
                    },
                    // Subscribed but never refreshed: its own honest state,
                    // rendered with an EMPTY packages array (never hidden).
                    RepoCatalogRepoV01 {
                        repo_id: Some("curated".into()),
                        name: Some("Curated".into()),
                        cached: false,
                        packages: vec![],
                    },
                    // A row whose repoId the library carries none of:
                    // honest null projection, never padded.
                    RepoCatalogRepoV01 {
                        repo_id: None,
                        name: Some("Local tools".into()),
                        cached: true,
                        packages: vec![RepoCatalogPackageV01 {
                            package_id: "com.example.local-tool".into(),
                            display_name: Some("Local Tool".into()),
                            description: Some("A locally hosted package".into()),
                            latest_version: Some("0.2.0".into()),
                            version_count: 2,
                        }],
                    },
                ],
                // The REQUIRED born-in disclosure: this fake answers through
                // the cache-degradation path.
                cache_sourced: true,
            }),
            failure: None,
            expect_transport: None,
        }
    }

    fn undeclaring() -> Self {
        let mut fake = Self::declaring();
        fake.repo_catalog_cap = false;
        fake
    }

    fn failing(code: &'static str) -> Self {
        let mut fake = Self::declaring();
        fake.failure = Some(AppErrorV1::new(
            code,
            ErrorCategory::Validation,
            "errors.vpm.repoNotFound",
            "corr-fake-repo-catalog",
        ));
        fake
    }

    fn expecting_transport(repo_id: Option<&str>, package_ids: &[&str]) -> Self {
        let mut fake = Self::declaring();
        fake.expect_transport = Some((
            repo_id.map(str::to_owned),
            package_ids.iter().map(|id| (*id).to_owned()).collect(),
        ));
        fake
    }
}

impl VpmBackend for RepoCatalogVpm {
    fn name(&self) -> &'static str {
        "fake-repo-catalog-wire"
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
    fn repo_catalog_capabilities(&self) -> RepoCatalogCapabilities {
        RepoCatalogCapabilities {
            repo_catalog: self.repo_catalog_cap,
        }
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
        unreachable!("the F2 face has no preview arm")
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
    fn repo_catalog(
        &self,
        repo_id: Option<&str>,
        package_ids: &[String],
    ) -> Result<RepoCatalogV01, AppErrorV1> {
        assert!(
            self.repo_catalog_cap,
            "the route gate must answer capability_missing BEFORE the port"
        );
        if let Some((expect_repo, expect_ids)) = &self.expect_transport {
            assert_eq!(
                repo_id.map(str::to_owned).as_deref(),
                expect_repo.as_deref(),
                "verbatim repoId transport (None = the whole collection world)"
            );
            assert_eq!(
                package_ids,
                expect_ids.as_slice(),
                "verbatim packageIds transport (empty slice = the null lens)"
            );
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(self
            .catalog
            .clone()
            .expect("declaring fake carries a catalog"))
    }
}

/// Sends one query frame through the real host loop and returns the
/// response payload.
fn run_query_frame(
    database: &Path,
    vpm: Option<Arc<dyn VpmBackend>>,
    method: &str,
    params: Value,
) -> Value {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-packages-repo-catalog-v01",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages-repo-catalog-v01",
            "correlationId": "corr-packages-repo-catalog-v01",
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
    frames[0]["payload"].clone()
}

/// Sends `application.getSnapshot` and extracts the `packages.repoCatalogOps`
/// capability row.
fn repo_catalog_capability_row(database: &Path, vpm: Option<Arc<dyn VpmBackend>>) -> Value {
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
        .find(|row| row["operationId"] == "packages.repoCatalogOps")
        .cloned()
        .expect("the packages.repoCatalogOps capability row exists")
}

fn browse_params() -> Value {
    json!({ "repoId": null, "packageIds": null })
}

#[test]
fn absent_repo_catalog_wiring_answers_the_typed_honest_absence_and_unavailable_row() {
    let root = unique_root("absent");
    let database = root.join("tasks.sqlite");

    let payload = run_query_frame(&database, None, "packages.repoCatalog", browse_params());
    assert_eq!(payload["ok"], false, "absence is a typed failure");
    assert_eq!(payload["error"]["code"], "vua.packages.unavailable");
    assert_eq!(payload["error"]["category"], "unavailable");
    assert!(
        payload.get("result").is_none(),
        "absence never carries a fabricated result"
    );

    let row = repo_catalog_capability_row(&database, None);
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_route_projects_the_frozen_envelope_and_transports_null_params_verbatim() {
    let validator = repo_catalog_result_validator();
    let root = unique_root("wired");
    let database = root.join("tasks.sqlite");
    // Null/null crosses as the port's (None, empty slice) — the unscoped,
    // unfiltered browse of the whole collection world.
    let vpm = RepoCatalogVpm::expecting_transport(None, &[]);

    let payload = run_query_frame(
        &database,
        Some(Arc::new(vpm)),
        "packages.repoCatalog",
        browse_params(),
    );
    assert_eq!(
        payload["ok"], true,
        "wired + declared must succeed: {payload}"
    );
    let value = &payload["value"];
    assert_eq!(
        value["schemaVersion"],
        PACKAGES_REPO_CATALOG_ENVELOPE_SCHEMA_VERSION_V01
    );
    assert_eq!(value["operation"], "packages.repoCatalog");
    assert!(
        validator.is_valid(value),
        "the frozen envelope must match the result schema: {:?} {value}",
        schema_violations(&validator, value)
    );
    let result = &value["result"];
    assert_eq!(
        result["schemaVersion"],
        PACKAGES_REPO_CATALOG_SCHEMA_VERSION_V01
    );
    assert_eq!(result["cacheSourced"], Value::Bool(true));
    let repos = result["repos"].as_array().expect("repo rows");
    assert_eq!(repos.len(), 3);
    assert_eq!(repos[0]["packages"][0]["latestVersion"], "1.7.0");
    assert_eq!(repos[0]["packages"][0]["versionCount"], 14);
    // Honest absence arms: null latestVersion (all-yanked under the current
    // setting), null displayName/description, and the null repoId row —
    // none of them padded, none of them errors.
    assert_eq!(repos[0]["packages"][1]["latestVersion"], Value::Null);
    assert_eq!(repos[0]["packages"][1]["displayName"], Value::Null);
    assert_eq!(repos[2]["repoId"], Value::Null);
    // The uncached subscription row: cached=false with an EMPTY packages
    // array — its own honest state, never hidden.
    assert_eq!(repos[1]["cached"], Value::Bool(false));
    assert_eq!(repos[1]["packages"].as_array().unwrap().len(), 0);
    // The deliberate ceiling: no compatible fact exists on this face.
    assert!(repos[0]["packages"][0].get("compatible").is_none());

    let row = repo_catalog_capability_row(&database, Some(Arc::new(RepoCatalogVpm::declaring())));
    assert_eq!(row["availability"], "available");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn scoped_and_filtered_params_cross_verbatim_and_a_filter_miss_stays_honest_empty() {
    let validator = repo_catalog_result_validator();
    let root = unique_root("scoped");
    let database = root.join("tasks.sqlite");
    // The scoped + filtered browse: both keys verbatim into the port
    // (Some("official"), the exact requirement set).
    let vpm = RepoCatalogVpm::expecting_transport(
        Some("official"),
        &["com.anatawa12.avatar-optimizer", "com.example.local-tool"],
    );

    let payload = run_query_frame(
        &database,
        Some(Arc::new(vpm)),
        "packages.repoCatalog",
        json!({
            "repoId": "official",
            "packageIds": ["com.anatawa12.avatar-optimizer", "com.example.local-tool"],
        }),
    );
    assert_eq!(
        payload["ok"], true,
        "scoped + filtered must succeed: {payload}"
    );
    assert!(
        validator.is_valid(&payload["value"]),
        "{:?}",
        schema_violations(&validator, &payload["value"])
    );

    // The lens law on the live wire: a filter that matches nothing is an
    // HONEST EMPTY answer (an empty packages array on the scoped row),
    // never a no_matching_package error — the filter is a lens, not an
    // existence assertion.
    let mut empty_match = RepoCatalogVpm::declaring();
    empty_match.catalog = Some(RepoCatalogV01 {
        repos: vec![RepoCatalogRepoV01 {
            repo_id: Some("official".into()),
            name: Some("Official".into()),
            cached: true,
            packages: vec![],
        }],
        cache_sourced: false,
    });
    let payload = run_query_frame(
        &database,
        Some(Arc::new(empty_match)),
        "packages.repoCatalog",
        json!({ "repoId": "official", "packageIds": ["com.example.not-there"] }),
    );
    assert_eq!(
        payload["ok"], true,
        "a filter miss is an honest success: {payload}"
    );
    assert!(
        validator.is_valid(&payload["value"]),
        "{:?}",
        schema_violations(&validator, &payload["value"])
    );
    let repos = payload["value"]["result"]["repos"].as_array().unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0]["packages"].as_array().unwrap().len(), 0);
    assert_eq!(
        payload["value"]["result"]["cacheSourced"],
        Value::Bool(false)
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn empty_repos_array_is_an_honest_schema_valid_answer() {
    let validator = repo_catalog_result_validator();
    let root = unique_root("empty");
    let database = root.join("tasks.sqlite");
    let mut fake = RepoCatalogVpm::declaring();
    fake.catalog = Some(RepoCatalogV01 {
        repos: vec![],
        cache_sourced: false,
    });

    let payload = run_query_frame(
        &database,
        Some(Arc::new(fake)),
        "packages.repoCatalog",
        browse_params(),
    );
    assert_eq!(
        payload["ok"], true,
        "zero repository caches is a real backend fact"
    );
    assert!(
        validator.is_valid(&payload["value"]),
        "{:?}",
        schema_violations(&validator, &payload["value"])
    );
    assert_eq!(payload["value"]["result"]["repos"], json!([]));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn repo_catalog_param_violations_answer_invalid_params() {
    let root = unique_root("params");
    let database = root.join("tasks.sqlite");
    let vpm = Some(Arc::new(RepoCatalogVpm::declaring()) as Arc<dyn VpmBackend>);

    let violations: Vec<Value> = vec![
        // The closed two-key set — absent keys are violations (both keys
        // are REQUIRED-nullable: the KEY must be present even when null).
        json!({}),
        json!({ "repoId": null }),
        json!({ "packageIds": null }),
        // An empty repoId string is a violation (minLength 1 on the string
        // arm).
        json!({ "repoId": "", "packageIds": null }),
        // Any non-null non-string repoId is a violation.
        json!({ "repoId": 42, "packageIds": null }),
        // An empty packageIds array is a shape violation, NOT a third
        // state next to null.
        json!({ "repoId": null, "packageIds": [] }),
        // Duplicate ids break the uniqueItems law.
        json!({ "repoId": null, "packageIds": ["com.example.a", "com.example.a"] }),
        // An empty id inside the array is a violation.
        json!({ "repoId": null, "packageIds": [""] }),
        // A non-string item inside the array is a violation.
        json!({ "repoId": null, "packageIds": [7] }),
        // A non-array packageIds is a violation.
        json!({ "repoId": null, "packageIds": "com.example.a" }),
        // Any third key breaks the closed set (no projectPath on this face
        // — the v0.1 word face carries deliberately none).
        json!({ "repoId": null, "packageIds": null, "projectPath": "C:/some/project" }),
    ];
    for params in violations {
        let payload = run_query_frame(
            &database,
            vpm.clone(),
            "packages.repoCatalog",
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
fn undeclared_capability_answers_capability_missing_before_the_port_and_row_stays_unavailable() {
    // The F2 gate reads the NEW defaulted accessor BEFORE the port call —
    // default declared-none keeps the route on the generic capability-
    // missing arm. Unlike the A5 face the port method HAS a default body,
    // so the honest structural difference holds: a declared-but-
    // unimplemented backend CAN exist at the type level (the consumer suite
    // pins the trait default answering capability_missing too) — the route
    // gate simply answers first and the port is never reached.
    let root = unique_root("undeclared");
    let database = root.join("tasks.sqlite");
    let vpm = Arc::new(RepoCatalogVpm::undeclaring());

    let payload = run_query_frame(
        &database,
        Some(vpm.clone()),
        "packages.repoCatalog",
        browse_params(),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.capability_missing");
    assert_eq!(payload["error"]["category"], "unavailable");

    let row = repo_catalog_capability_row(&database, Some(vpm));
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn port_refusals_travel_verbatim_and_repo_not_found_carries_the_reused_code() {
    // The P2 read-face pass-through discipline: no read-face fold exists —
    // the port's typed refusal (here the REUSED vua.vpm.repo_not_found, the
    // A4 removeRepo same-fact precedent for an unknown repoId) travels
    // verbatim as code + message key + category.
    let root = unique_root("verbatim");
    let database = root.join("tasks.sqlite");
    let vpm = RepoCatalogVpm::failing("vua.vpm.repo_not_found");

    let payload = run_query_frame(
        &database,
        Some(Arc::new(vpm)),
        "packages.repoCatalog",
        json!({ "repoId": "not-a-known-repo", "packageIds": null }),
    );
    assert_eq!(
        payload["ok"], false,
        "the refusal travels as a typed failure"
    );
    assert_eq!(payload["error"]["code"], "vua.vpm.repo_not_found");
    assert_eq!(payload["error"]["messageKey"], "errors.vpm.repoNotFound");
    assert!(
        payload.get("result").is_none(),
        "a refusal never carries a fabricated result"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn envelope_consts_are_detectable_and_match_the_frozen_schema_consts() {
    // The A3/A4/A5 precedent closed at the wiring batch: the envelope const
    // and the family const are named HERE and pinned against the frozen
    // schema consts — a consumer can detect the row version from the wire
    // alone (the route stamps both consts verbatim, never literals).
    let command = read_repo_catalog_schema("command.schema.json");
    let result = read_repo_catalog_schema("result.schema.json");
    assert_eq!(
        command["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_REPO_CATALOG_ENVELOPE_SCHEMA_VERSION_V01),
        "the envelope const equals the frozen command schema's const"
    );
    assert_eq!(PACKAGES_REPO_CATALOG_ENVELOPE_SCHEMA_VERSION_V01, "0.1");
    assert_eq!(
        result["$defs"]["repoCatalogResult"]["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_REPO_CATALOG_SCHEMA_VERSION_V01),
        "the family const equals the frozen result def's const"
    );
    assert_eq!(
        PACKAGES_REPO_CATALOG_SCHEMA_VERSION_V01,
        "vua.packages-repo-catalog/v0.1"
    );

    // The live stamp: the one response carries both consts (pinned against
    // the constants, not against string literals).
    let root = unique_root("envelope");
    let database = root.join("tasks.sqlite");
    let payload = run_query_frame(
        &database,
        Some(Arc::new(RepoCatalogVpm::declaring())),
        "packages.repoCatalog",
        browse_params(),
    );
    assert_eq!(
        payload["value"]["schemaVersion"],
        PACKAGES_REPO_CATALOG_ENVELOPE_SCHEMA_VERSION_V01
    );
    assert_eq!(
        payload["value"]["result"]["schemaVersion"],
        PACKAGES_REPO_CATALOG_SCHEMA_VERSION_V01
    );
    fs::remove_dir_all(&root).ok();
}
