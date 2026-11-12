//! packages-ops v0.3 consumer tests (proposal 026 freeze batch, slice A3
//! = register_local_package, 2026-09-19): the core-side consumer of the
//! local-package registration word face. The frozen
//! `schemas/packages-ops/v0.3/` schemas pin the wire command/result
//! shapes; the example vectors drive JSON-Schema validation directly,
//! and a fake backend's port fact (`register_local_package` answering
//! `Result<(), _>`) is projected onto the `kind=registered` receipt —
//! the request echo alone, the minimal honest audit shape — and
//! validated against the same schema. The port→wire consumer loop is
//! nailed here first. The REAL backend consumption (`VrcGetLibBackend`
//! already carrying `register_local_package` with the library's
//! `AlreadyAdded` idempotence) is the environment
//! implementation-verification slice and lands with its own tests; this
//! file pins the contract side, the capability-absence word face (the
//! NEW defaulted accessor `register_capabilities` defaults to
//! declared-none — the 025 catalog_capabilities law — and a backend
//! without the capability answers the generic capability_missing via
//! the trait default), and the idempotence word face (AlreadyAdded is a
//! success; the receipt carries no first-vs-repeat fact and no
//! invented payload). The wire routes for this method do not exist yet
//! (wiring is the next core slice); nothing here exercises a running
//! engine. Everything runs against synthetic data — no
//! machine-specific facts, no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use vua_orchestrator::{AppErrorV1, ErrorCategory, PackageRequestV1};
use vua_orchestrator::{ProjectRef, RegisterCapabilities, VpmBackend};

fn read_ops_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn ops_command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.3/command.schema.json",
    ))
    .unwrap()
}

fn ops_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.3/result.schema.json",
    ))
    .unwrap()
}

fn violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

const FAKE_PACKAGE_ROOT: &str = "C:/synthetic/generated/com.example.toolkit-1.4.0";

fn register_request() -> Value {
    json!({
        "schemaVersion": "0.3",
        "operation": "packages.registerLocalPackage",
        "params": { "packageRoot": FAKE_PACKAGE_ROOT },
    })
}

/// A backend that implements the A3 registration face with the port's
/// honest fact: `register_local_package` answers `Result<(), _>` — no
/// payload, idempotent success whether the package was added or was
/// already present (the library's AlreadyAdded collapse).
struct FakeRegisterBackend;

impl VpmBackend for FakeRegisterBackend {
    fn name(&self) -> &'static str {
        "fake-register"
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
    fn register_capabilities(&self) -> RegisterCapabilities {
        RegisterCapabilities {
            register_local_package: true,
        }
    }
    fn register_local_package(&self, _package_root: &Path) -> Result<(), AppErrorV1> {
        // The port fact verbatim: unit success. First registration and
        // the AlreadyAdded re-registration are one success fact.
        Ok(())
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

/// A backend WITHOUT the registration capability: the honest word face
/// is the trait default — `register_capabilities` stays declared-none
/// (the 025 accessor law) and `register_local_package` answers the
/// generic capability_missing via the `unsupported` default.
struct NoRegisterBackend;

impl VpmBackend for NoRegisterBackend {
    fn name(&self) -> &'static str {
        "no-register-fake"
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

#[test]
fn ops_v03_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = ops_command_validator();
    let result = ops_result_validator();

    let positive_request = read_ops_json(
        "schemas/packages-ops/v0.3/examples/packages-register-local-package.request.json",
    );
    assert!(
        command.is_valid(&positive_request),
        "{:?}",
        violations(&command, &positive_request)
    );

    let positive_result = read_ops_json("schemas/packages-ops/v0.3/examples/packages-register-local-package-result-registered.result.json");
    assert!(
        result.is_valid(&positive_result),
        "{:?}",
        violations(&result, &positive_result)
    );

    // The closed single-key params: a missing packageRoot has nothing to
    // register.
    let missing_root = read_ops_json(
        "schemas/packages-ops/v0.3/examples/invalid-register-missing-package-root.request.json",
    );
    assert!(
        !command.is_valid(&missing_root),
        "a missing packageRoot must be invalid"
    );

    // An empty packageRoot is not a path fact.
    let empty_root = read_ops_json(
        "schemas/packages-ops/v0.3/examples/invalid-register-empty-package-root.request.json",
    );
    assert!(
        !command.is_valid(&empty_root),
        "an empty packageRoot must be invalid"
    );

    // NO projectPath is taken: registration never touches a project —
    // the A3 face mutates only the backend's isolated environment.
    let extra_param = read_ops_json(
        "schemas/packages-ops/v0.3/examples/invalid-register-extra-param.request.json",
    );
    assert!(
        !command.is_valid(&extra_param),
        "an invented projectPath param must be invalid"
    );

    // NO digest is taken: registration has no preview to drift, and the
    // user's explicit submission IS the confirmation — a carried
    // confirmedDigest is a shape violation.
    let carries_digest = read_ops_json(
        "schemas/packages-ops/v0.3/examples/invalid-register-carries-digest.request.json",
    );
    assert!(
        !command.is_valid(&carries_digest),
        "a registration carrying a digest must be invalid"
    );

    // The false-assertion guard on the receipt: no first-vs-repeat fact
    // exists (AlreadyAdded collapses into success) — an invented `added`
    // boolean is invalid by schema, not merely discouraged.
    let invented = read_ops_json(
        "schemas/packages-ops/v0.3/examples/invalid-register-invented-field.result.json",
    );
    assert!(
        !result.is_valid(&invented),
        "an invented added flag on the register receipt must be invalid"
    );

    // Operation/kind lock: the registration task never answers kind=plan
    // (no preview arm exists on this face at all).
    let kind_mismatch = read_ops_json(
        "schemas/packages-ops/v0.3/examples/invalid-register-answer-plan.result.json",
    );
    assert!(
        !result.is_valid(&kind_mismatch),
        "registerLocalPackage must never answer kind=plan"
    );

    // The reused-code law: the original port code travels inside detail,
    // never inside the code key — the pattern stays locked to
    // ^vua\.packages\..
    let code_outside = read_ops_json("schemas/packages-ops/v0.3/examples/invalid-register-rejected-code-outside-family.result.json");
    assert!(
        !result.is_valid(&code_outside),
        "a non-vua.packages.* code must be invalid"
    );
}

#[test]
fn register_capability_absence_keeps_the_declared_none_word_face() {
    let backend = NoRegisterBackend;
    assert_eq!(backend.name(), "no-register-fake");

    // The accessor DEFAULT: declared-none (the 025 catalog_capabilities
    // law — no implementation, no reservation). The served wire row
    // stays honestly unavailable until a backend overrides.
    assert_eq!(backend.register_capabilities(), RegisterCapabilities::NONE);

    // The trait-default registration answers the generic
    // capability_missing (the honest absence the wire gate projects).
    let err = backend
        .register_local_package(Path::new(FAKE_PACKAGE_ROOT))
        .unwrap_err();
    assert_eq!(err.code, "vua.vpm.capability_missing");
    assert_eq!(err.category, ErrorCategory::Unavailable);
}

#[test]
fn fake_register_backend_port_fact_projects_onto_the_registered_wire_shape() {
    let backend = FakeRegisterBackend;
    assert!(backend.register_capabilities().register_local_package);

    // Port fact → the request echo projection → registered-arm wire
    // envelope → schema. The port answers unit; the receipt carries the
    // request echo (packageRoot) and NOTHING else — the minimal honest
    // audit shape.
    backend
        .register_local_package(Path::new(FAKE_PACKAGE_ROOT))
        .expect("the synthetic registration succeeds");
    let envelope = json!({
        "schemaVersion": "0.3",
        "operation": "packages.registerLocalPackage",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.3",
            "kind": "registered",
            "packageRoot": FAKE_PACKAGE_ROOT,
        },
    });
    let validator = ops_result_validator();
    assert!(
        validator.is_valid(&envelope),
        "{:?}",
        violations(&validator, &envelope)
    );

    // The camelCase pin: the port's package_root projects as packageRoot.
    let request = register_request();
    assert_eq!(
        request["params"]["packageRoot"],
        json!(FAKE_PACKAGE_ROOT),
        "the request row the receipt echoes is the camelCase word-face row"
    );
}

#[test]
fn register_idempotence_is_one_success_fact_word_face() {
    let backend = FakeRegisterBackend;
    let root = Path::new(FAKE_PACKAGE_ROOT);

    // First registration: success.
    backend.register_local_package(root).unwrap();
    // Re-registration (the library's AlreadyAdded branch): the port
    // answers the SAME unit success — the wire face has no
    // first-vs-repeat fact to project, and the receipt schema forbids
    // inventing one (the invalid-register-invented-field negative
    // vector pins the refusal).
    backend.register_local_package(root).unwrap();
}
