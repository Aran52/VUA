//! packages-repo-catalog v0.1 consumer tests (proposal 027 F2 freeze
//! batch, 2026-09-20): the core-side consumer of the repo-catalog read
//! face. The frozen `schemas/packages-repo-catalog/v0.1/` schemas pin the
//! wire request/result shapes; the example vectors drive JSON-Schema
//! validation directly, and a fake backend's port facts are projected
//! through serde and validated against the same schemas — the port->wire
//! consumer loop is nailed here first. The REAL backend consumption
//! (`VrcGetLibBackend` implementing `repo_catalog`) is the environment
//! implementation slice and lands with its own tests; this file pins the
//! contract side and the trait default absence arms (a backend that does
//! not implement the F2 face answers `capability_missing` and declares
//! `RepoCatalogCapabilities::NONE`). The word-face rulings pinned here:
//! `author` deliberately absent (the library's manifest closed set has no
//! author field — environment verification 3bd4f12 s1(b)); NO compatible
//! fact (no project context, a constant null is not a fact); the batch
//! `packageIds` filter is a lens whose empty answer is honest empty, never
//! an error. Everything runs against synthetic data — no machine-specific
//! facts, no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use vua_orchestrator::AppErrorV1;
use vua_orchestrator::{
    PackageRequestV1, ProjectRef, RepoCatalogCapabilities, RepoCatalogPackageV01,
    RepoCatalogRepoV01, RepoCatalogV01, VpmBackend,
};

fn read_repo_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-repo-catalog/v0.1/command.schema.json",
    ))
    .unwrap()
}

fn result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-repo-catalog/v0.1/result.schema.json",
    ))
    .unwrap()
}

fn violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

/// The trait-default backend: implements only the methods without defaults,
/// inheriting every default absence arm (the F2 face and its capability
/// declaration among them).
struct MinimalBackend;

impl VpmBackend for MinimalBackend {
    fn name(&self) -> &'static str {
        "minimal-fake"
    }
    fn capabilities(&self) -> vua_orchestrator::VpmCapabilities {
        vua_orchestrator::VpmCapabilities {
            create_project: false,
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

/// A backend that implements the F2 read face with synthetic facts (the
/// contract-side consumer loop; the real facts come with the environment
/// implementation slice).
struct FakeRepoCatalogBackend;

impl VpmBackend for FakeRepoCatalogBackend {
    fn name(&self) -> &'static str {
        "fake-repo-catalog"
    }
    fn capabilities(&self) -> vua_orchestrator::VpmCapabilities {
        vua_orchestrator::VpmCapabilities {
            create_project: false,
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
    fn repo_catalog_capabilities(&self) -> RepoCatalogCapabilities {
        RepoCatalogCapabilities { repo_catalog: true }
    }
    fn repo_catalog(
        &self,
        _repo_id: Option<&str>,
        _package_ids: &[String],
    ) -> Result<RepoCatalogV01, AppErrorV1> {
        Ok(RepoCatalogV01 {
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
                // A row whose repoId the library carries none of: honest
                // null projection, never padded.
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
            // The REQUIRED born-in disclosure: this fake answers through the
            // cache-degradation path.
            cache_sourced: true,
        })
    }
}

#[test]
fn repo_catalog_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = command_validator();
    let result = result_validator();

    for positive in [
        "schemas/packages-repo-catalog/v0.1/examples/packages-repo-catalog.request.json",
        "schemas/packages-repo-catalog/v0.1/examples/packages-repo-catalog-scoped.request.json",
        "schemas/packages-repo-catalog/v0.1/examples/packages-repo-catalog-filtered.request.json",
    ] {
        let doc = read_repo_json(positive);
        assert!(
            command.is_valid(&doc),
            "{positive}: {:?}",
            violations(&command, &doc)
        );
    }

    for positive in [
        "schemas/packages-repo-catalog/v0.1/examples/packages-repo-catalog.result.json",
        "schemas/packages-repo-catalog/v0.1/examples/packages-repo-catalog-empty.result.json",
        "schemas/packages-repo-catalog/v0.1/examples/packages-repo-catalog-cache-sourced.result.json",
    ] {
        let doc = read_repo_json(positive);
        assert!(result.is_valid(&doc), "{positive}: {:?}", violations(&result, &doc));
    }

    for negative in [
        "schemas/packages-repo-catalog/v0.1/examples/invalid-repo-catalog-extra-param.request.json",
        "schemas/packages-repo-catalog/v0.1/examples/invalid-repo-catalog-empty-packageids.request.json",
        "schemas/packages-repo-catalog/v0.1/examples/invalid-repo-catalog-duplicate-packageids.request.json",
    ] {
        let doc = read_repo_json(negative);
        assert!(!command.is_valid(&doc), "{negative} must be invalid");
    }

    for negative in [
        "schemas/packages-repo-catalog/v0.1/examples/invalid-repo-catalog-row-author-field.result.json",
        "schemas/packages-repo-catalog/v0.1/examples/invalid-repo-catalog-row-compatible-invented.result.json",
        "schemas/packages-repo-catalog/v0.1/examples/invalid-repo-catalog-row-missing-versioncount.result.json",
        "schemas/packages-repo-catalog/v0.1/examples/invalid-repo-catalog-repo-missing-cached.result.json",
        "schemas/packages-repo-catalog/v0.1/examples/invalid-repo-catalog-missing-cachesourced.result.json",
    ] {
        let doc = read_repo_json(negative);
        assert!(!result.is_valid(&doc), "{negative} must be invalid");
    }
}

#[test]
fn trait_default_answers_capability_missing_and_declares_none() {
    let backend = MinimalBackend;
    assert_eq!(
        backend.repo_catalog_capabilities(),
        RepoCatalogCapabilities::NONE
    );

    let err = backend.repo_catalog(None, &[]).unwrap_err();
    assert_eq!(err.code, "vua.vpm.capability_missing");
}

#[test]
fn fake_backend_port_facts_project_onto_the_frozen_wire_shape() {
    let backend = FakeRepoCatalogBackend;
    assert!(backend.repo_catalog_capabilities().repo_catalog);

    // Port facts -> serde projection -> result schema; the family const is
    // an envelope-assembly fact (P1 discipline), stamped here exactly as
    // the route stamps it.
    let catalog = backend.repo_catalog(None, &[]).unwrap();
    let mut result = serde_json::to_value(&catalog).unwrap();
    result["schemaVersion"] = json!("vua.packages-repo-catalog/v0.1");
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "packages.repoCatalog",
        "result": result,
    });
    let validator = result_validator();
    assert!(
        validator.is_valid(&envelope),
        "{:?}",
        violations(&validator, &envelope)
    );
    assert_eq!(
        envelope["result"]["schemaVersion"],
        "vua.packages-repo-catalog/v0.1"
    );
    // camelCase projection pins (serde rename_all verbatim).
    assert_eq!(
        envelope["result"]["cacheSourced"],
        serde_json::Value::Bool(true)
    );
    let repos = envelope["result"]["repos"].as_array().unwrap();
    assert_eq!(repos.len(), 3);
    assert_eq!(repos[0]["packages"][0]["latestVersion"], "1.7.0");
    assert_eq!(repos[0]["packages"][0]["versionCount"], 14);
    // Honest absence arms: null latestVersion (all-yanked under the current
    // setting), null displayName/description (manifest carries none), and
    // the null repoId row — none of them padded, none of them errors.
    assert_eq!(
        repos[0]["packages"][1]["latestVersion"],
        serde_json::Value::Null
    );
    assert_eq!(
        repos[0]["packages"][1]["displayName"],
        serde_json::Value::Null
    );
    assert_eq!(repos[2]["repoId"], serde_json::Value::Null);
    // The uncached subscription row: cached=false with an EMPTY packages
    // array — its own honest state, never hidden.
    assert_eq!(repos[1]["cached"], serde_json::Value::Bool(false));
    assert_eq!(repos[1]["packages"].as_array().unwrap().len(), 0);
    // The deliberate ceiling: no compatible fact exists on this face.
    assert!(envelope["result"]["repos"][0]["packages"][0]
        .get("compatible")
        .is_none());
}

#[test]
fn batch_filter_semantics_are_params_shape_not_port_behavior() {
    // The wire shape carries the Recipe requirement-set filter as a CLOSED
    // params shape (unique non-empty ids, null = unfiltered); the port
    // method receives the resolved slice and answers the same facts either
    // way — the filter is a lens whose empty answer is honest empty, never
    // a no_matching_package error (declared protocol fact, pinned here by
    // the fake answering the identical shape for filtered and unfiltered
    // calls).
    let backend = FakeRepoCatalogBackend;
    let unfiltered = backend.repo_catalog(None, &[]).unwrap();
    let filtered = backend
        .repo_catalog(
            Some("official"),
            &["com.anatawa12.avatar-optimizer".to_owned()],
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(&unfiltered).unwrap(),
        serde_json::to_value(&filtered).unwrap()
    );

    // The command shape admits all three forms; the fake projection above
    // validates against the result schema in the other test — here the
    // scoped/filtered command vectors ride the same validator once more to
    // keep the lens semantics adjacent to their port-side law.
    let command = command_validator();
    for doc in [
        "schemas/packages-repo-catalog/v0.1/examples/packages-repo-catalog-scoped.request.json",
        "schemas/packages-repo-catalog/v0.1/examples/packages-repo-catalog-filtered.request.json",
    ] {
        let request = read_repo_json(doc);
        assert!(
            command.is_valid(&request),
            "{doc}: {:?}",
            violations(&command, &request)
        );
    }
}
