//! packages-query v0.2 consumer tests (proposal 027 F3 freeze batch,
//! 2026-09-20): the core-side consumer of the installed-set update-awareness
//! increment. The frozen `schemas/packages-query/v0.2/` schemas pin the wire
//! request/result shapes; the example vectors drive JSON-Schema validation
//! directly, and a fake backend's port facts are projected through serde and
//! validated against the same schemas — the port->wire consumer loop is
//! nailed here first. The REAL backend consumption
//! (`VrcGetLibBackend` implementing `list_packages_v02`) is the environment
//! implementation slice and lands with its own tests; this file pins the
//! contract side and the trait default absence arms. The word-face rulings
//! pinned here: the judgment facts are REQUIRED at v0.2 (a v0.1-shaped row
//! is INVALID under the v0.2 schema — the family const makes the version
//! machine-detectable, never a guess); the null arms are honest absences
//! (judgment not executed — never "already latest", the 024 stance-2
//! false-assertion line); the negotiation is additive (`query_v02` default
//! false keeps serving the frozen v0.1 family, the `catalog_v02` precedent).
//! Everything runs against synthetic data — no machine-specific facts, no
//! network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, InstalledListingV02, InstalledPackageV02, PackageRequestV1, ProjectRef, VpmBackend,
};

fn read_repo_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-query/v0.2/command.schema.json",
    ))
    .unwrap()
}

fn result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-query/v0.2/result.schema.json",
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
/// inheriting every default absence arm — among them the F3 declaration
/// (`query_v02` default false) and the F3 method (`list_packages_v02`
/// default unsupported).
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

/// A backend that implements BOTH generations of the installed-set read
/// face: the frozen v0.1 family through `list_packages` and the v0.2 word
/// face through `list_packages_v02` (the contract-side consumer loop; the
/// real facts come with the environment implementation slice).
struct FakeQueryBackend;

impl VpmBackend for FakeQueryBackend {
    fn name(&self) -> &'static str {
        "fake-query"
    }
    fn capabilities(&self) -> vua_orchestrator::VpmCapabilities {
        vua_orchestrator::VpmCapabilities {
            create_project: false,
            preview_install: false,
            list_packages: true,
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
    fn list_packages(
        &self,
        _project: &ProjectRef,
    ) -> Result<Vec<vua_orchestrator::InstalledPackageV1>, AppErrorV1> {
        // The frozen v0.1 projection: three-key rows, no judgment facts.
        Ok(vec![vua_orchestrator::InstalledPackageV1 {
            package_id: "com.vrchat.base".into(),
            version: "3.7.4".into(),
            dependencies: vec![],
        }])
    }
    fn query_v02(&self) -> bool {
        true
    }
    fn list_packages_v02(&self, _project: &ProjectRef) -> Result<InstalledListingV02, AppErrorV1> {
        // Synthetic facts covering every arm of the frozen judgment word
        // face: judged-true, judged-false, judgment-not-executed (local
        // source), and the prerelease-boundary false (installed prerelease
        // newer than the stable-set qualifying latest; the word face says
        // `false` = no strictly newer version under the CURRENT filter,
        // never a generalized "no update"). Sorted by packageId ascending —
        // the frozen presentation fact.
        Ok(InstalledListingV02 {
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
            // The REQUIRED born-in disclosure: this fake answers through the
            // cache-degradation path.
            cache_sourced: true,
        })
    }
}

#[test]
fn query_v02_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = command_validator();
    let result = result_validator();

    // The command face is byte-for-byte the v0.1 face; the single positive
    // request vector rides the v0.2 validator once.
    let request =
        read_repo_json("schemas/packages-query/v0.2/examples/packages-list-installed.request.json");
    assert!(
        command.is_valid(&request),
        "{:?}",
        violations(&command, &request)
    );

    for positive in [
        "schemas/packages-query/v0.2/examples/packages-list-installed.result.json",
        "schemas/packages-query/v0.2/examples/packages-list-installed-judgment-not-executed.result.json",
        "schemas/packages-query/v0.2/examples/packages-list-installed-empty.result.json",
    ] {
        let doc = read_repo_json(positive);
        assert!(result.is_valid(&doc), "{positive}: {:?}", violations(&result, &doc));
    }

    for negative in [
        "schemas/packages-query/v0.2/examples/invalid-empty-project-path.request.json",
        "schemas/packages-query/v0.2/examples/invalid-extra-params.request.json",
        "schemas/packages-query/v0.2/examples/invalid-missing-project-path.request.json",
    ] {
        let doc = read_repo_json(negative);
        assert!(!command.is_valid(&doc), "{negative} must be invalid");
    }

    for negative in [
        "schemas/packages-query/v0.2/examples/invalid-row-invented-field.result.json",
        "schemas/packages-query/v0.2/examples/invalid-row-missing-judgment-keys.result.json",
        "schemas/packages-query/v0.2/examples/invalid-result-missing-cachesourced.result.json",
        "schemas/packages-query/v0.2/examples/invalid-updateavailable-type.result.json",
    ] {
        let doc = read_repo_json(negative);
        assert!(!result.is_valid(&doc), "{negative} must be invalid");
    }
}

#[test]
fn trait_default_keeps_serving_v01_and_declares_no_v02() {
    let backend = MinimalBackend;
    // The additive negotiation law (the `catalog_v02` precedent): the
    // default declaration is false — the frozen v0.1 family keeps being
    // served — and the unimplemented v0.2 method answers the generic
    // capability absence.
    assert!(!backend.query_v02());
    let err = backend
        .list_packages_v02(&ProjectRef {
            id: "minimal".into(),
            root: std::path::PathBuf::from("C:/nowhere"),
        })
        .unwrap_err();
    assert_eq!(err.code, "vua.vpm.capability_missing");
}

#[test]
fn fake_backend_port_facts_project_onto_the_frozen_v02_wire_shape() {
    let backend = FakeQueryBackend;
    assert!(backend.query_v02());

    // Port facts -> serde projection -> result schema; the family const and
    // projectPath are envelope-assembly facts (P1 discipline), stamped here
    // exactly as the route stamps them.
    let listing = backend
        .list_packages_v02(&ProjectRef {
            id: "fake-query".into(),
            root: std::path::PathBuf::from("C:/demo/MyAvatarProject"),
        })
        .unwrap();
    let mut result = serde_json::to_value(&listing).unwrap();
    result["schemaVersion"] = json!("vua.packages-installed/v0.2");
    result["projectPath"] = json!("C:/demo/MyAvatarProject");
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "packages.listInstalled",
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
        "vua.packages-installed/v0.2"
    );
    // camelCase projection pins (serde rename_all verbatim).
    assert_eq!(
        envelope["result"]["cacheSourced"],
        serde_json::Value::Bool(true)
    );
    let packages = envelope["result"]["packages"].as_array().unwrap();
    assert_eq!(packages.len(), 4);
    // packageId-ascending order — the frozen presentation fact, unchanged.
    let ids: Vec<&str> = packages
        .iter()
        .map(|row| row["packageId"].as_str().unwrap())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "rows stay packageId-ascending");
    // The judged-true arm.
    assert_eq!(
        packages[0]["updateAvailable"],
        serde_json::Value::Bool(true)
    );
    assert_eq!(packages[0]["latestVersion"], "1.8.2");
    // The judgment-not-executed arm: null/null for a local-source package —
    // honest absence, NEVER a boolean false, never an invented fact.
    assert_eq!(packages[1]["latestVersion"], serde_json::Value::Null);
    assert_eq!(packages[1]["updateAvailable"], serde_json::Value::Null);
    // The prerelease-boundary arm: installed prerelease 2.0.0-beta.4, stable
    // qualifying latest 1.9.0 — `false` under the CURRENT filter, paired
    // with the version fact that keeps the boundary readable.
    assert_eq!(
        packages[2]["updateAvailable"],
        serde_json::Value::Bool(false)
    );
    assert_eq!(packages[2]["latestVersion"], "1.9.0");
    // The deliberate ceiling: no invented catalog facts ride this face.
    for key in ["changelogUrl", "displayName", "source", "versions"] {
        assert!(
            packages[0].get(key).is_none(),
            "the v0.2 word list carries NO {key} field — never invented"
        );
    }
}

#[test]
fn v01_rows_are_invalid_under_v02_making_the_version_machine_detectable() {
    // The version-detectability law (the catalog v0.2 precedent): the
    // judgment keys are REQUIRED at v0.2, so a v0.1-shaped row inside a
    // v0.2-stamped document FAILS the frozen schema — the stamped family
    // const plus the schema together make the word face machine-detectable,
    // never a guess. The negative vector pins it; here the same fact is
    // derived live from the v0.1 fixture to keep the two generations
    // adjacent.
    let v01_fixture =
        read_repo_json("schemas/packages-query/v0.1/examples/packages-list-installed.result.json");
    let validator = result_validator();
    assert!(
        !validator.is_valid(&v01_fixture),
        "a v0.1 result must NOT validate against the v0.2 schema (missing judgment keys + missing cacheSourced)"
    );
    // And the same-generation law holds: the v0.1 family keeps validating
    // against its own frozen schema (zero in-place revision).
    let v01_validator = jsonschema::validator_for(&read_repo_json(
        "schemas/packages-query/v0.1/result.schema.json",
    ))
    .unwrap();
    assert!(
        v01_validator.is_valid(&v01_fixture),
        "the frozen v0.1 schema is never revised in place"
    );
}
