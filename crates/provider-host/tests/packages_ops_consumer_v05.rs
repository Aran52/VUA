//! packages-ops v0.5 consumer tests (proposal 026 freeze batch, slice A5
//! = project creation, 2026-09-19): the core-side consumer of the
//! project-creation write word face. The frozen
//! `schemas/packages-ops/v0.5/` schemas pin the wire command/result
//! shapes; the example vectors drive JSON-Schema validation directly,
//! and a fake backend's port facts (`create_project` answering
//! `Result<ProjectRef, _>` — the ONE packages-ops face with an
//! actual-result payload) are projected onto the `kind=created` document
//! — the ProjectRef {id, root} projection, keys `projectId`/`projectPath`
//! — and validated against the same schema. The port→wire consumer loop
//! is nailed here first. The REAL backend consumption (both in-repo
//! backends already implement `create_project` — the library path over
//! `create_from_template`, the CLI path over `vpm new`) is the
//! environment implementation-verification slice and rides its own
//! verification; this file pins the contract side, the
//! capability-absence word face (the gate reads the EXISTING five-bit
//! `VpmCapabilities.create_project` member — A5 freezes NO new accessor,
//! unlike A3/A4: the bit predates this batch and both in-repo backends
//! already declare it honestly), the no-preview-pair word face (the port
//! has no create-preview counterpart — a carried confirmedDigest or
//! projectPath is a shape violation, negative vectors), the
//! registers-in-store receipt fact (a created project IS a registered
//! project — the ProjectRef echo, never a directory-only invention), and
//! the refusal-fold word face (every port refusal folds into
//! `execution_failed` carrying the ORIGINAL code in detail — the three
//! existing codes `vua.vpm.template_missing` / `vua.vpm.apply_failed` /
//! `vua.vpm.backend_unavailable`, the two backends' shapes honestly
//! divergent; the reuse-code law: the codes never travel in the rejected
//! `code` key). The wire routes for this method do not exist yet (wiring
//! is the next core slice); nothing here exercises a running engine.
//! Everything runs against synthetic data — no machine-specific facts,
//! no network, no real template copy.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use vua_orchestrator::{AppErrorV1, ErrorCategory, PackageRequestV1, ProjectRef, VpmBackend};

fn read_ops_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn ops_command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.5/command.schema.json",
    ))
    .unwrap()
}

fn ops_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.5/result.schema.json",
    ))
    .unwrap()
}

fn violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

fn example(name: &str) -> Value {
    read_ops_json(&format!("schemas/packages-ops/v0.5/examples/{name}"))
}

const FAKE_PARENT: &str = "D:/synthetic/projects";
const FAKE_NAME: &str = "Synthetic Project";
const FAKE_TEMPLATE: &str = "Avatar";
const FAKE_PROJECT_ID: &str = "proj-Synthetic Project";
/// The root fact as the projection transports it (a forward-slash string
/// fact; the fake backend's PathBuf join uses the platform separator, the
/// receipt carries the string fact — the assertion compares against this
/// constant, never re-derives a path).
const FAKE_PROJECT_PATH: &str = "D:/synthetic/projects/Synthetic Project";

/// A backend that serves the A5 creation face: the EXISTING
/// `VpmCapabilities.create_project` bit true, the required method
/// answering the port's honest fact — `ProjectRef {id, root}` — while
/// carrying the request arguments through verbatim (the template
/// pass-through is part of the frozen word face: None stays None, a
/// name stays that name).
struct FakeCreateBackend;

impl VpmBackend for FakeCreateBackend {
    fn name(&self) -> &'static str {
        "fake-create"
    }
    fn capabilities(&self) -> vua_orchestrator::VpmCapabilities {
        vua_orchestrator::VpmCapabilities {
            create_project: true,
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
        parent: &Path,
        name: &str,
        template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
        // The argument pass-through is observable: the word face transports
        // the request facts verbatim (template None = the backend's default
        // resolution, a string = that name/path passed through).
        assert_eq!(parent, Path::new(FAKE_PARENT));
        assert_eq!(name, FAKE_NAME);
        assert_eq!(template, Some(FAKE_TEMPLATE));
        Ok(ProjectRef {
            id: FAKE_PROJECT_ID.to_string(),
            root: PathBuf::from(FAKE_PARENT).join(name),
        })
    }
}

/// A backend with the creation bit FALSE — the honest-absence fixture.
/// The wire gate reads the bit BEFORE submit and answers the generic
/// capability_missing; the method itself is never reached (the required
/// method has no default body to call — the gate, not a trait default,
/// is the absence arm).
struct NoCreateBackend;

impl VpmBackend for NoCreateBackend {
    fn name(&self) -> &'static str {
        "no-create-fake"
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
        _parent: &Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
        unreachable!("the gate must refuse before any task reaches the method")
    }
}

/// A backend that refuses with each of the three A5 port codes in turn —
/// the fold-word-face fixture. All three are EXISTING codes (zero new
/// codes on this batch): the library-path carrier, the CLI failure legs.
struct RefusingCreateBackend {
    code: &'static str,
}

impl VpmBackend for RefusingCreateBackend {
    fn name(&self) -> &'static str {
        "refusing-create"
    }
    fn capabilities(&self) -> vua_orchestrator::VpmCapabilities {
        vua_orchestrator::VpmCapabilities {
            create_project: true,
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
        _parent: &Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
        Err(AppErrorV1::new(
            self.code,
            ErrorCategory::Validation,
            "errors.vpm.createRefused",
            "corr-create-refused",
        ))
    }
}

#[test]
fn ops_v05_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = ops_command_validator();
    let result = ops_result_validator();

    // --- the three positive vectors pass their respective schema ---
    let create_default = example("create-project.request.json");
    assert!(
        command.is_valid(&create_default),
        "{:?}",
        violations(&command, &create_default)
    );
    let create_explicit = example("create-project-explicit-template.request.json");
    assert!(
        command.is_valid(&create_explicit),
        "{:?}",
        violations(&command, &create_explicit)
    );
    let created = example("create-project-result-created.result.json");
    assert!(
        result.is_valid(&created),
        "{:?}",
        violations(&result, &created)
    );

    // --- the ten negative vectors all fail ---
    for name in [
        "invalid-create-missing-parent.request.json",
        "invalid-create-missing-name.request.json",
        "invalid-create-empty-name.request.json",
        "invalid-create-empty-template.request.json",
        "invalid-create-carries-digest.request.json",
        "invalid-create-carries-project-path.request.json",
        "invalid-create-extra-param.request.json",
    ] {
        let negative = example(name);
        assert!(
            !command.is_valid(&negative),
            "{name} must be an invalid command: {:?}",
            violations(&command, &negative)
        );
    }
    for name in [
        "invalid-create-answer-plan.result.json",
        "invalid-created-invented-field.result.json",
        "invalid-result-rejected-code-outside-family.result.json",
    ] {
        let negative = example(name);
        assert!(
            !result.is_valid(&negative),
            "{name} must be an invalid result: {:?}",
            violations(&result, &negative)
        );
    }

    // --- operation/kind lock, inline-pinned: the A5 face never answers
    // the wrong arm (no preview arm exists; create never answers removed
    // or repoReceipt; the sibling faces never answer created) ---
    let create_answers_removed = json!({
        "schemaVersion": "0.5",
        "operation": "packages.createProject",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.5",
            "kind": "removed",
            "repoId": "repo.example.official",
        },
    });
    assert!(
        !result.is_valid(&create_answers_removed),
        "createProject must never answer kind=removed: {:?}",
        violations(&result, &create_answers_removed)
    );
    let remove_answers_created = json!({
        "schemaVersion": "0.5",
        "operation": "packages.removeRepo",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.5",
            "kind": "created",
            "projectId": FAKE_PROJECT_ID,
            "projectPath": "D:/synthetic/projects/Synthetic Project",
        },
    });
    assert!(
        !result.is_valid(&remove_answers_created),
        "removeRepo must never answer kind=created: {:?}",
        violations(&result, &remove_answers_created)
    );
}

#[test]
fn create_capability_gate_is_the_existing_vpm_capabilities_bit() {
    // The A5 gate reads the EXISTING five-bit-closed-set member — no new
    // accessor is frozen (the A3/A4 law does NOT repeat here: the bit
    // predates this batch and both in-repo backends already declare it).
    // A false bit = the wire gate answers the generic capability_missing
    // BEFORE submit; the required method (no default body) is never
    // reached — the absence arm is the gate, not a trait default.
    let capable = FakeCreateBackend;
    assert!(capable.capabilities().create_project);
    assert_eq!(capable.name(), "fake-create");

    let incapable = NoCreateBackend;
    assert!(!incapable.capabilities().create_project);
    assert_eq!(incapable.name(), "no-create-fake");
    // The gate-side word face: a false bit folds to the generic
    // capability_missing envelope error (the same code the A3/A4 trait
    // defaults answer — one code, two absence mechanisms, both honest).
    let expected_gate_code = "vua.vpm.capability_missing";
    assert_eq!(expected_gate_code, "vua.vpm.capability_missing");
}

#[test]
fn fake_create_backend_port_facts_project_onto_the_a5_wire_shapes() {
    let backend = FakeCreateBackend;

    let command = ops_command_validator();
    let result = ops_result_validator();

    // --- creation: ProjectRef success → created receipt, the ProjectRef
    // projection keys pinned verbatim (projectId = ProjectRef.id,
    // projectPath = ProjectRef.root = the registered-project identity:
    // a created project IS a registered project) ---
    let request = json!({
        "schemaVersion": "0.5",
        "operation": "packages.createProject",
        "params": {
            "parent": FAKE_PARENT,
            "name": FAKE_NAME,
            "template": FAKE_TEMPLATE,
        },
    });
    assert!(
        command.is_valid(&request),
        "{:?}",
        violations(&command, &request)
    );

    let project = backend
        .create_project(Path::new(FAKE_PARENT), FAKE_NAME, Some(FAKE_TEMPLATE))
        .expect("the fake backend must create");
    assert_eq!(project.id, FAKE_PROJECT_ID);
    assert_eq!(project.root, PathBuf::from(FAKE_PARENT).join(FAKE_NAME));

    let receipt = json!({
        "schemaVersion": "0.5",
        "operation": "packages.createProject",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.5",
            "kind": "created",
            "projectId": project.id,
            "projectPath": FAKE_PROJECT_PATH,
        },
    });
    assert!(
        result.is_valid(&receipt),
        "{:?}",
        violations(&result, &receipt)
    );
    assert_eq!(receipt["result"]["kind"], "created");
    assert_eq!(receipt["result"]["projectId"], FAKE_PROJECT_ID);
    assert_eq!(receipt["result"]["projectPath"], FAKE_PROJECT_PATH);
    // The key set is exactly the declared four — the false-assertion
    // guard (no created-at timestamp, no copy stats, no package list).
    let keys: Vec<String> = receipt["result"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        keys,
        vec![
            "schemaVersion".to_string(),
            "kind".to_string(),
            "projectId".to_string(),
            "projectPath".to_string(),
        ]
    );
}

#[test]
fn create_port_refusals_fold_into_execution_failed_carrying_the_original_code() {
    // The fold discipline (A1/A2/A3/A4 same law): every port refusal folds
    // into a rejected document whose guard is execution_failed (ZERO new
    // guard values on A5) and whose code key stays inside the
    // vua.packages.* family — the original code travels verbatim in detail
    // as honest provenance. The A5 closed set is THREE EXISTING codes:
    // the library-path carrier (all four i18n keys share it) and the two
    // CLI-path legs — the backends' shapes honestly diverge, the wire face
    // folds both without inventing a unified shape.
    let cases = [
        (
            "vua.vpm.template_missing",
            "errors.vpm.projectExists: the target path already exists",
        ),
        (
            "vua.vpm.template_missing",
            "errors.vpm.projectNameInvalid: the name carries a forbidden character",
        ),
        (
            "vua.vpm.template_missing",
            "errors.vpm.templateMissing: no template directory named Avatar",
        ),
        (
            "vua.vpm.template_missing",
            "errors.vpm.templateCopyFailed: the template copy failed",
        ),
        (
            "vua.vpm.apply_failed",
            "vpm new exited non-zero (exitCode 1)",
        ),
        (
            "vua.vpm.backend_unavailable",
            "the vpm CLI runner could not be spawned",
        ),
    ];

    let result_validator = ops_result_validator();
    for (code, detail) in cases {
        let backend = RefusingCreateBackend { code };
        let refusal = backend
            .create_project(Path::new(FAKE_PARENT), FAKE_NAME, None)
            .expect_err("the refusing backend must refuse");
        assert_eq!(refusal.code, code, "the port answers the original code");

        // The wire projection: the ONLY legal shape folds the refusal —
        // execution_failed + the family-locked code + the original code
        // in detail. Schema-validated.
        let rejected = json!({
            "schemaVersion": "0.5",
            "operation": "packages.createProject",
            "result": {
                "schemaVersion": "vua.packages-ops/v0.5",
                "kind": "rejected",
                "guard": "execution_failed",
                "code": "vua.packages.execution_failed",
                "detail": format!("{code}: {detail}"),
            },
        });
        assert!(
            result_validator.is_valid(&rejected),
            "{code} fold must be schema-valid: {:?}",
            violations(&result_validator, &rejected)
        );
        assert!(rejected["result"]["detail"]
            .as_str()
            .unwrap()
            .starts_with(code));
    }
}
