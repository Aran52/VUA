//! packages-repos / packages-catalog v0.1 consumer tests (proposal 025 P2
//! freeze batch, 2026-09-17): the core-side consumer of the P2 read faces.
//! The frozen `schemas/packages-repos/v0.1/` and `schemas/packages-catalog/
//! v0.1/` schemas pin the wire result shapes; the example vectors drive
//! JSON-Schema validation directly, and a fake backend's port facts are
//! projected through serde and validated against the same schemas — the
//! port→wire consumer loop is nailed here first. The REAL backend
//! consumption (`VrcGetLibBackend` implementing `list_repos` /
//! `package_catalog`) is the environment implementation slice and lands
//! with its own tests; this file pins the contract side and the trait
//! default absence arms (a backend that does not implement the P2 faces
//! answers `capability_missing` and declares `CatalogCapabilities::NONE`).
//! The v0.2 catalog increment (proposal 025 inline ruling, 2026-09-17) is
//! pinned in the same spirit: `schemas/packages-catalog/v0.2/` (frozen v0.1
//! result + the REQUIRED cacheSourced disclosure), the additive
//! declaration/default arms (`catalog_v02` / `package_catalog_v02`), and
//! the detectability of the version bump (a v0.1-shaped result is INVALID
//! against the v0.2 schema — consumers can never confuse the word faces).
//! Everything runs against synthetic data — no machine-specific facts, no
//! network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use vua_orchestrator::AppErrorV1;
use vua_orchestrator::{
    CatalogCapabilities, CatalogVersionV01, PackageCatalogV01, PackageCatalogV02, PackageRequestV1,
    PackageSourceV01, ProjectRef, RepoInfoV01, VpmBackend,
};

fn read_repo_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn repos_command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-repos/v0.1/command.schema.json",
    ))
    .unwrap()
}

fn repos_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-repos/v0.1/result.schema.json",
    ))
    .unwrap()
}

fn catalog_command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-catalog/v0.1/command.schema.json",
    ))
    .unwrap()
}

fn catalog_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-catalog/v0.1/result.schema.json",
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
/// inheriting every default absence arm (the P2 faces and the catalog
/// capability declaration among them).
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

/// A backend that implements the P2 read faces with synthetic facts (the
/// contract-side consumer loop; the real facts come with the environment
/// implementation slice).
struct FakeCatalogBackend;

impl VpmBackend for FakeCatalogBackend {
    fn name(&self) -> &'static str {
        "fake-catalog"
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
    fn catalog_capabilities(&self) -> CatalogCapabilities {
        CatalogCapabilities { catalog: true }
    }
    fn list_repos(&self) -> Result<Vec<RepoInfoV01>, AppErrorV1> {
        Ok(vec![
            RepoInfoV01 {
                repo_id: Some("official".into()),
                name: Some("Official".into()),
                url: Some("https://vpm.example/vpm.json".into()),
                local_path: Some("C:\\cache\\official.json".into()),
                cached: true,
            },
            RepoInfoV01 {
                repo_id: Some("curated".into()),
                name: Some("Curated".into()),
                url: Some("https://vpm.example/curated.json".into()),
                local_path: None,
                cached: false,
            },
            RepoInfoV01 {
                repo_id: Some("local-tools".into()),
                name: Some("Local tools".into()),
                url: None,
                local_path: Some("D:\\vpm-local\\repo.json".into()),
                cached: true,
            },
        ])
    }
    fn package_catalog(
        &self,
        project: &ProjectRef,
        package_id: &str,
    ) -> Result<PackageCatalogV01, AppErrorV1> {
        Ok(PackageCatalogV01 {
            project_path: project.root.to_string_lossy().to_string(),
            package_id: package_id.to_owned(),
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
                    version: "1.7.0".into(),
                    yanked: false,
                    compatible: Some(true),
                },
                CatalogVersionV01 {
                    version: "1.9.0-beta.1".into(),
                    yanked: false,
                    compatible: None,
                },
            ],
        })
    }
}

#[test]
fn repos_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = repos_command_validator();
    let result = repos_result_validator();

    let request =
        read_repo_json("schemas/packages-repos/v0.1/examples/packages-list-repos.request.json");
    assert!(
        command.is_valid(&request),
        "{:?}",
        violations(&command, &request)
    );

    let listing =
        read_repo_json("schemas/packages-repos/v0.1/examples/packages-list-repos.result.json");
    assert!(
        result.is_valid(&listing),
        "{:?}",
        violations(&result, &listing)
    );

    let empty = read_repo_json(
        "schemas/packages-repos/v0.1/examples/packages-list-repos-empty.result.json",
    );
    assert!(result.is_valid(&empty), "{:?}", violations(&result, &empty));

    // A global configuration face accepts NO parameter key.
    let extra_param = read_repo_json(
        "schemas/packages-repos/v0.1/examples/invalid-repos-params-extra-key.request.json",
    );
    assert!(
        !command.is_valid(&extra_param),
        "an extra params key must be a shape violation"
    );

    // The false-assertion guard: the health face is a P2 non-goal, an
    // invented health field invalidates the row.
    let health_row = read_repo_json(
        "schemas/packages-repos/v0.1/examples/invalid-repo-row-health-field.result.json",
    );
    assert!(
        !result.is_valid(&health_row),
        "invented health field must be invalid"
    );

    // cached is the REQUIRED per-repo cache-hit fact: a row without it is
    // invalid, never silently tolerated.
    let missing_cached = read_repo_json(
        "schemas/packages-repos/v0.1/examples/invalid-repo-row-missing-cached.result.json",
    );
    assert!(
        !result.is_valid(&missing_cached),
        "missing cached fact must be invalid"
    );
}

#[test]
fn catalog_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = catalog_command_validator();
    let result = catalog_result_validator();

    let request = read_repo_json(
        "schemas/packages-catalog/v0.1/examples/packages-package-catalog.request.json",
    );
    assert!(
        command.is_valid(&request),
        "{:?}",
        violations(&command, &request)
    );

    for positive in [
        "schemas/packages-catalog/v0.1/examples/packages-package-catalog.result.json",
        "schemas/packages-catalog/v0.1/examples/packages-package-catalog-local.result.json",
        "schemas/packages-catalog/v0.1/examples/packages-package-catalog-not-installed.result.json",
    ] {
        let doc = read_repo_json(positive);
        assert!(
            result.is_valid(&doc),
            "{positive}: {:?}",
            violations(&result, &doc)
        );
    }

    let missing_id = read_repo_json(
        "schemas/packages-catalog/v0.1/examples/invalid-catalog-missing-package-id.request.json",
    );
    assert!(
        !command.is_valid(&missing_id),
        "missing packageId must be invalid"
    );

    // On-demand granularity: no pagination / preference switches exist —
    // an invented params key is a shape violation.
    let extra_param = read_repo_json(
        "schemas/packages-catalog/v0.1/examples/invalid-catalog-extra-param.request.json",
    );
    assert!(
        !command.is_valid(&extra_param),
        "an extra params key must be a shape violation"
    );

    let invented_field = read_repo_json(
        "schemas/packages-catalog/v0.1/examples/invalid-catalog-version-extra-field.result.json",
    );
    assert!(
        !result.is_valid(&invented_field),
        "invented changelogUrl must be invalid"
    );

    // source is a two-state word; the desktop third state composes with
    // `installed` and "installed" as a source VALUE is outside the word
    // list.
    let word_outside = read_repo_json(
        "schemas/packages-catalog/v0.1/examples/invalid-catalog-source-word-outside.result.json",
    );
    assert!(
        !result.is_valid(&word_outside),
        "source=installed is outside the two-state word"
    );
}

#[test]
fn trait_default_answers_capability_missing_and_declares_none() {
    let backend = MinimalBackend;
    assert_eq!(backend.catalog_capabilities(), CatalogCapabilities::NONE);

    let err = backend.list_repos().unwrap_err();
    assert_eq!(err.code, "vua.vpm.capability_missing");

    let err = backend
        .package_catalog(
            &ProjectRef {
                id: "demo".into(),
                root: std::path::PathBuf::from("C:/proj"),
            },
            "com.example.pkg",
        )
        .unwrap_err();
    assert_eq!(err.code, "vua.vpm.capability_missing");
}

#[test]
fn fake_backend_port_facts_project_onto_the_frozen_wire_shapes() {
    let backend = FakeCatalogBackend;
    assert!(backend.catalog_capabilities().catalog);

    // Port facts → serde projection → repos result schema.
    let repos = backend.list_repos().unwrap();
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "packages.listRepos",
        "result": {
            "schemaVersion": "vua.packages-repos/v0.1",
            "repos": repos,
        },
    });
    let validator = repos_result_validator();
    assert!(
        validator.is_valid(&envelope),
        "{:?}",
        violations(&validator, &envelope)
    );
    assert_eq!(
        envelope["result"]["schemaVersion"],
        "vua.packages-repos/v0.1"
    );
    let rows = envelope["result"]["repos"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    // Nullable identifier/location facts project verbatim (null = honest
    // absence), and the cache-hit fact rides every row.
    assert_eq!(rows[2]["url"], serde_json::Value::Null);
    assert_eq!(rows[1]["cached"], serde_json::Value::Bool(false));

    // Port facts → serde projection → catalog result schema. The family
    // const is an envelope-assembly fact (P1 discipline), stamped here.
    let catalog = backend
        .package_catalog(
            &ProjectRef {
                id: "demo".into(),
                root: std::path::PathBuf::from("C:/proj"),
            },
            "com.anatawa12.avatar-optimizer",
        )
        .unwrap();
    let mut result = serde_json::to_value(&catalog).unwrap();
    result["schemaVersion"] = json!("vua.packages-catalog/v0.1");
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "packages.packageCatalog",
        "result": result,
    });
    let validator = catalog_result_validator();
    assert!(
        validator.is_valid(&envelope),
        "{:?}",
        violations(&validator, &envelope)
    );
    assert_eq!(
        envelope["result"]["schemaVersion"],
        "vua.packages-catalog/v0.1"
    );
    // Enum projection: the port snake_case source lands as the frozen
    // two-state word.
    assert_eq!(envelope["result"]["source"], "repo");
    // The judgment-conclusion null discipline: versions carry per-version
    // compatibility with null = not evaluated (unknown project Unity
    // version), never false.
    let versions = envelope["result"]["versions"].as_array().unwrap();
    assert_eq!(versions[2]["compatible"], serde_json::Value::Null);
}

fn catalog_v02_command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-catalog/v0.2/command.schema.json",
    ))
    .unwrap()
}

fn catalog_v02_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-catalog/v0.2/result.schema.json",
    ))
    .unwrap()
}

/// A backend that has adopted the v0.2 catalog word face: declares
/// `catalog_v02` and serves `package_catalog_v02` with the REQUIRED
/// cacheSourced fact (the contract-side consumer loop; the real adoption is
/// the environment incremental batch).
struct FakeCatalogV02Backend;

impl VpmBackend for FakeCatalogV02Backend {
    fn name(&self) -> &'static str {
        "fake-catalog-v02"
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
    fn catalog_capabilities(&self) -> CatalogCapabilities {
        CatalogCapabilities { catalog: true }
    }
    fn package_catalog(
        &self,
        _project: &ProjectRef,
        _package_id: &str,
    ) -> Result<PackageCatalogV01, AppErrorV1> {
        unreachable!("a v0.2-declaring backend is asked through package_catalog_v02")
    }
    fn catalog_v02(&self) -> bool {
        true
    }
    fn package_catalog_v02(
        &self,
        project: &ProjectRef,
        package_id: &str,
    ) -> Result<PackageCatalogV02, AppErrorV1> {
        Ok(PackageCatalogV02 {
            project_path: project.root.to_string_lossy().to_string(),
            package_id: package_id.to_owned(),
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
                    version: "1.7.0".into(),
                    yanked: false,
                    compatible: Some(true),
                },
            ],
            // The v0.2 REQUIRED disclosure fact: true = served through the
            // cache-degradation path (offline -> load_cache, or online load
            // failed and degraded) — informational, never an error.
            cache_sourced: true,
        })
    }
}

#[test]
fn catalog_v02_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = catalog_v02_command_validator();
    let result = catalog_v02_result_validator();

    let request = read_repo_json(
        "schemas/packages-catalog/v0.2/examples/packages-package-catalog.request.json",
    );
    assert!(
        command.is_valid(&request),
        "{:?}",
        violations(&command, &request)
    );

    for positive in [
        "schemas/packages-catalog/v0.2/examples/packages-package-catalog.result.json",
        "schemas/packages-catalog/v0.2/examples/packages-package-catalog-local.result.json",
        "schemas/packages-catalog/v0.2/examples/packages-package-catalog-not-installed.result.json",
        // The honest degraded answer: cacheSourced=true, informational and
        // schema-valid — the disclosure the v0.2 increment exists for.
        "schemas/packages-catalog/v0.2/examples/packages-package-catalog-cache-sourced.result.json",
    ] {
        let doc = read_repo_json(positive);
        assert!(
            result.is_valid(&doc),
            "{positive}: {:?}",
            violations(&result, &doc)
        );
    }

    for negative in [
        "schemas/packages-catalog/v0.2/examples/invalid-catalog-missing-package-id.request.json",
        "schemas/packages-catalog/v0.2/examples/invalid-catalog-extra-param.request.json",
        "schemas/packages-catalog/v0.2/examples/invalid-catalog-source-word-outside.result.json",
        "schemas/packages-catalog/v0.2/examples/invalid-catalog-version-extra-field.result.json",
    ] {
        let doc = read_repo_json(negative);
        let validator = if negative.ends_with("request.json") {
            &command
        } else {
            &result
        };
        assert!(!validator.is_valid(&doc), "{negative} must be invalid");
    }

    // The REQUIRED cacheSourced fact: a wrong-typed value is a shape
    // violation, never a tolerated default.
    let wrong_type = read_repo_json(
        "schemas/packages-catalog/v0.2/examples/invalid-catalog-cachesourced-type.result.json",
    );
    assert!(
        !result.is_valid(&wrong_type),
        "a non-boolean cacheSourced must be invalid"
    );

    // The version bump is DETECTABLE: the frozen v0.1 result shape (no
    // cacheSourced) is INVALID against the v0.2 schema — consumers can
    // never confuse the two word faces, which is exactly why the increment
    // versions the family instead of revising v0.1 in place.
    let v01_shaped = read_repo_json(
        "schemas/packages-catalog/v0.1/examples/packages-package-catalog.result.json",
    );
    assert!(
        !result.is_valid(&v01_shaped),
        "a v0.1-shaped result must be invalid against the v0.2 schema"
    );
}

#[test]
fn catalog_v02_default_answers_capability_missing_and_declares_false() {
    let backend = MinimalBackend;
    // The default declaration stays false: the frozen v0.1 word face is
    // served until a backend adopts v0.2 (ORC-DEV-004: no implementation,
    // no reservation).
    assert!(!backend.catalog_v02());

    let err = backend
        .package_catalog_v02(
            &ProjectRef {
                id: "demo".into(),
                root: std::path::PathBuf::from("C:/proj"),
            },
            "com.example.pkg",
        )
        .unwrap_err();
    assert_eq!(err.code, "vua.vpm.capability_missing");
}

#[test]
fn v02_backend_port_facts_project_onto_the_v02_wire_shape() {
    let backend = FakeCatalogV02Backend;
    // Negotiation facts: the family bit and the capability bit both carry.
    assert!(backend.catalog_v02());
    assert!(backend.catalog_capabilities().catalog);

    // Port facts → serde projection → v0.2 result schema; the family const
    // is an envelope-assembly fact (P1 discipline), stamped here exactly as
    // the route stamps it.
    let catalog = backend
        .package_catalog_v02(
            &ProjectRef {
                id: "demo".into(),
                root: std::path::PathBuf::from("C:/proj"),
            },
            "com.anatawa12.avatar-optimizer",
        )
        .unwrap();
    let mut result = serde_json::to_value(&catalog).unwrap();
    result["schemaVersion"] = json!("vua.packages-catalog/v0.2");
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "packages.packageCatalog",
        "result": result,
    });
    let validator = catalog_v02_result_validator();
    assert!(
        validator.is_valid(&envelope),
        "{:?}",
        violations(&validator, &envelope)
    );
    assert_eq!(
        envelope["result"]["schemaVersion"],
        "vua.packages-catalog/v0.2"
    );
    // The REQUIRED disclosure fact rides the result verbatim.
    assert_eq!(
        envelope["result"]["cacheSourced"],
        serde_json::Value::Bool(true)
    );
}
