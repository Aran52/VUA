//! Packages-repos v0.2 wire tests (proposal 027 F4 wiring, 2026-09-21):
//! the subscription-list read face's additive dual-version negotiation
//! rides the real frame loop over the frozen `schemas/packages-repos/v0.2/`
//! result schema — a backend that declares `repos_v02` answers the v0.2
//! result family through `packages.listRepos` (rows carry the REQUIRED
//! VUA-owned `enabled` state bit; the disabled row stays LISTED, never
//! hidden; the id-absent row projects `enabled: true` ALWAYS), every other
//! backend keeps answering the frozen `vua.packages-repos/v0.1` family
//! through `list_repos`, and the COMMAND face stays byte-for-byte the
//! frozen v0.1 face (the closed EMPTY params law holds unchanged, the
//! envelope const stays "0.1" — the v0.2 increment is a result-document
//! fact only; the stamped family const tells the consumer which word face
//! answered, never a guess — the `catalog_v02`/`query_v02` precedent).
//! The route stamps the named family const
//! (`PACKAGES_REPOS_SCHEMA_VERSION_V02`), the backend facts stay verbatim
//! (the P1 discipline). The contract-side consumer loop is pinned by
//! `packages_repos_consumer_v02`; the v0.1 route stays pinned by
//! `packages_p2_wire`; the REAL backend projection (state read from VUA's
//! own storage under the environment root) is the environment
//! implementation-verification slice. Everything runs against synthetic
//! values — no machine-specific facts, no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, PackageRequestV1, ProjectRef, RepoInfoV01, RepoInfoV02, VpmBackend, VpmCapabilities,
};
use vua_provider_host::provider_host::PACKAGES_REPOS_SCHEMA_VERSION_V02;
use vua_provider_host::run_provider_host_full;

fn read_repos_schema(name: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .join("../..")
        .join("schemas/packages-repos/v0.2")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn repos_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repos_schema("result.schema.json")).unwrap()
}

fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-packages-repos-wire-v02-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// A backend that serves the subscription-list face at the v0.2 word face:
/// the negotiation accessor true, `list_repos_v02` answering rows that
/// cover the three state laws (an enabled row, a disabled row that stays
/// listed, and the id-absent row that is always enabled).
struct ReposV02Backend;

impl VpmBackend for ReposV02Backend {
    fn name(&self) -> &'static str {
        "fake-repos-v02-wire"
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
    fn catalog_capabilities(&self) -> vua_orchestrator::CatalogCapabilities {
        vua_orchestrator::CatalogCapabilities { catalog: true }
    }
    fn repos_v02(&self) -> bool {
        true
    }
    fn list_repos_v02(&self) -> Result<Vec<RepoInfoV02>, AppErrorV1> {
        Ok(vec![
            RepoInfoV02 {
                repo_id: Some("repo.example.official".to_string()),
                name: Some("Official Repo".to_string()),
                url: Some("https://example.invalid/vrc-official.json".to_string()),
                local_path: Some("C:/anywhere/Repos/official-cache.json".to_string()),
                cached: true,
                enabled: true,
            },
            RepoInfoV02 {
                repo_id: Some("repo.example.community".to_string()),
                name: Some("Community Repo".to_string()),
                url: Some("https://example.invalid/community.json".to_string()),
                local_path: Some("C:/anywhere/Repos/community-cache.json".to_string()),
                cached: true,
                enabled: false,
            },
            RepoInfoV02 {
                repo_id: None,
                name: Some("Local Dir Repo".to_string()),
                url: None,
                local_path: None,
                cached: false,
                // The id-absent-row law: outside the toggle faces' reach,
                // so enabled is this row's honest PERMANENT fact.
                enabled: true,
            },
        ])
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

/// A backend that never adopted v0.2: the negotiation accessor stays on
/// its declared-false default and the frozen v0.1 word face keeps being
/// served through `list_repos`.
struct ReposV01OnlyBackend;

impl VpmBackend for ReposV01OnlyBackend {
    fn name(&self) -> &'static str {
        "fake-repos-v01-only-wire"
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
    fn catalog_capabilities(&self) -> vua_orchestrator::CatalogCapabilities {
        vua_orchestrator::CatalogCapabilities { catalog: true }
    }
    fn list_repos(&self) -> Result<Vec<RepoInfoV01>, AppErrorV1> {
        Ok(vec![RepoInfoV01 {
            repo_id: Some("repo.example.official".to_string()),
            name: Some("Official Repo".to_string()),
            url: Some("https://example.invalid/vrc-official.json".to_string()),
            local_path: None,
            cached: true,
        }])
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

/// Runs one query frame through the real host loop and returns the
/// response payload.
fn run_query_frame(
    database: &Path,
    vpm: Option<Arc<dyn VpmBackend>>,
    method: &str,
    params: Value,
) -> Value {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-packages-repos-v02",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages-repos-v02",
            "correlationId": "corr-packages-repos-v02",
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

#[test]
fn a_v02_declaring_backend_answers_the_v02_family_through_list_repos() {
    let validator = repos_result_validator();
    let root = unique_root("v02");
    let database = root.join("tasks.sqlite");

    let payload = run_query_frame(
        &database,
        Some(Arc::new(ReposV02Backend)),
        "packages.listRepos",
        json!({}),
    );
    assert_eq!(
        payload["ok"], true,
        "wired + declared must succeed: {payload}"
    );
    let value = &payload["value"];
    // The COMMAND face stays byte-for-byte the frozen v0.1 face: the
    // envelope const stays "0.1" — the increment is a result-document fact.
    assert_eq!(value["schemaVersion"], "0.1");
    assert_eq!(value["operation"], "packages.listRepos");
    let result = &value["result"];
    assert_eq!(
        result["schemaVersion"], PACKAGES_REPOS_SCHEMA_VERSION_V02,
        "the stamped family const tells the consumer which word face answered"
    );
    assert_eq!(PACKAGES_REPOS_SCHEMA_VERSION_V02, "vua.packages-repos/v0.2");
    assert!(
        validator.is_valid(value),
        "the v0.2 result document must match the frozen schema: {value}"
    );

    let rows = result["repos"].as_array().expect("repo rows");
    assert_eq!(
        rows.len(),
        3,
        "the row order is the backend's subscription-face order"
    );
    // The disabled row is LISTED, never hidden — the subscription face
    // keeps showing every subscribed row with its state bit.
    assert_eq!(rows[0]["repoId"], "repo.example.official");
    assert_eq!(rows[0]["enabled"], true);
    assert_eq!(rows[1]["repoId"], "repo.example.community");
    assert_eq!(
        rows[1]["enabled"], false,
        "disabled = subscribed and listed, never resolved"
    );
    // The id-absent row projects enabled:true ALWAYS — outside the toggle
    // faces' reach, true is its honest permanent fact.
    assert_eq!(rows[2]["repoId"], Value::Null);
    assert_eq!(rows[2]["enabled"], true);
    // Nulls stay honest absences, never padded.
    assert_eq!(rows[2]["url"], Value::Null);
    assert_eq!(rows[2]["localPath"], Value::Null);

    // The row key set is exactly the declared six — the frozen v0.1 five
    // facts plus the ONE new state bit, nothing invented (no health, no
    // status, no lastRefreshed, no disabledAt).
    let row_keys: Vec<String> = rows[0]
        .as_object()
        .expect("row object")
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        row_keys,
        vec![
            "repoId".to_string(),
            "name".to_string(),
            "url".to_string(),
            "localPath".to_string(),
            "cached".to_string(),
            "enabled".to_string(),
        ]
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_backend_that_never_adopted_v02_keeps_answering_the_v01_family() {
    let root = unique_root("v01-only");
    let database = root.join("tasks.sqlite");

    let payload = run_query_frame(
        &database,
        Some(Arc::new(ReposV01OnlyBackend)),
        "packages.listRepos",
        json!({}),
    );
    assert_eq!(payload["ok"], true, "{payload}");
    let value = &payload["value"];
    assert_eq!(value["schemaVersion"], "0.1");
    assert_eq!(
        value["result"]["schemaVersion"], "vua.packages-repos/v0.1",
        "the frozen v0.1 word face keeps being served untouched"
    );
    let rows = value["result"]["repos"].as_array().expect("repo rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["repoId"], "repo.example.official");
    // The v0.1 row carries NO enabled bit — a v0.1-family stamp with a
    // v0.2-shaped row would be a version-increment violation (the
    // consumer test pins the negative direction).
    assert!(
        rows[0].get("enabled").is_none(),
        "the v0.1 face invents no state bit"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn the_closed_empty_params_law_holds_unchanged_at_the_v02_negotiation() {
    // The v0.2 increment is a RESULT-family fact only: the command face
    // (closed EMPTY params — any key or an absent params is a shape
    // violation) stays byte-for-byte the frozen v0.1 law, whatever
    // generation answers the result.
    let root = unique_root("params");
    let database = root.join("tasks.sqlite");
    let vpm = Some(Arc::new(ReposV02Backend) as Arc<dyn VpmBackend>);

    for params in [
        json!({ "projectPath": "C:/anywhere" }),
        json!({ "repoId": "repo.example.official" }),
        json!({ "enabled": true }),
    ] {
        let payload = run_query_frame(&database, vpm.clone(), "packages.listRepos", params);
        assert_eq!(payload["ok"], false, "any key is a violation: {payload}");
        assert_eq!(payload["error"]["code"], "vua.packages.invalid_params");
        assert_eq!(payload["error"]["category"], "validation");
    }
    fs::remove_dir_all(&root).ok();
}
