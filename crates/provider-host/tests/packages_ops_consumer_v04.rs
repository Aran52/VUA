//! packages-ops v0.4 consumer tests (proposal 026 freeze batch, slice A4
//! = repository add/remove, 2026-09-19): the core-side consumer of the
//! repo subscription write word face. The frozen
//! `schemas/packages-ops/v0.4/` schemas pin the wire command/result
//! shapes; the example vectors drive JSON-Schema validation directly,
//! and a fake backend's port facts (`add_remote_repo` / `add_local_repo`
//! / `remove_repo` each answering `Result<(), _>`) are projected onto
//! the `kind=repoReceipt` (remote/local variants, key sets disjoint) and
//! `kind=removed` documents — request echoes alone, the minimal honest
//! audit shapes — and validated against the same schema. The
//! port→wire consumer loop is nailed here first. The REAL backend
//! consumption (`VrcGetLibBackend` overriding `repo_write_capabilities`
//! and the three write methods over the library's Settings add/remove)
//! is the environment implementation-verification slice and lands with
//! its own tests; this file pins the contract side, the
//! capability-absence word face (the NEW defaulted accessor
//! `repo_write_capabilities` defaults to declared-none — the 025
//! catalog_capabilities law, THREE INDEPENDENT bits: a backend without
//! a bit answers the generic capability_missing via the trait default),
//! and the refusal-fold word face (every port refusal folds into
//! `execution_failed` carrying the ORIGINAL `vua.vpm.repo_*` code in
//! detail — the reuse-code law: the codes never travel in the rejected
//! `code` key). The wire routes for these methods do not exist yet
//! (wiring is the next core slice); nothing here exercises a running
//! engine. Everything runs against synthetic data — no
//! machine-specific facts, no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use vua_orchestrator::{vpm_backend_error_codes, AppErrorV1, ErrorCategory, PackageRequestV1};
use vua_orchestrator::{ProjectRef, RepoWriteCapabilities, VpmBackend};

fn read_ops_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn ops_command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.4/command.schema.json",
    ))
    .unwrap()
}

fn ops_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.4/result.schema.json",
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
    read_ops_json(&format!("schemas/packages-ops/v0.4/examples/{name}"))
}

const FAKE_REMOTE_URL: &str = "https://example.vpm/repos/official.json";
const FAKE_LOCAL_PATH: &str = "D:/synthetic/local-repo";
const FAKE_REPO_ID: &str = "repo.example.official";
const FAKE_REPO_NAME: &str = "Example Official";

/// A backend that implements the FULL A4 repo write face with the port's
/// honest fact: all three methods answer `Result<(), _>` — no payload.
/// The receipt shapes carry the request echo only.
struct FakeRepoWriteBackend;

impl VpmBackend for FakeRepoWriteBackend {
    fn name(&self) -> &'static str {
        "fake-repo-write"
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
    fn repo_write_capabilities(&self) -> RepoWriteCapabilities {
        RepoWriteCapabilities {
            add_remote_repo: true,
            add_local_repo: true,
            remove_repo: true,
        }
    }
    fn add_remote_repo(&self, _url: &str, _name: &str) -> Result<(), AppErrorV1> {
        // The port fact verbatim: unit success. The receipt is the echo.
        Ok(())
    }
    fn add_local_repo(&self, _path: &Path, _name: &str) -> Result<(), AppErrorV1> {
        Ok(())
    }
    fn remove_repo(&self, _repo_id: &str) -> Result<(), AppErrorV1> {
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

/// A backend with ONLY the local-add bit — the three bits are independent,
/// never a face-level flag (a backend may serve a subset of the face).
struct LocalOnlyRepoBackend;

impl VpmBackend for LocalOnlyRepoBackend {
    fn name(&self) -> &'static str {
        "local-only-repo"
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
    fn repo_write_capabilities(&self) -> RepoWriteCapabilities {
        RepoWriteCapabilities {
            add_remote_repo: false,
            add_local_repo: true,
            remove_repo: false,
        }
    }
    fn add_local_repo(&self, _path: &Path, _name: &str) -> Result<(), AppErrorV1> {
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

/// A backend that refuses with each of the four A4 port codes in turn —
/// the fold-word-face fixture.
struct RefusingRepoBackend {
    code: &'static str,
}

impl VpmBackend for RefusingRepoBackend {
    fn name(&self) -> &'static str {
        "refusing-repo"
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
    fn repo_write_capabilities(&self) -> RepoWriteCapabilities {
        RepoWriteCapabilities {
            add_remote_repo: true,
            add_local_repo: true,
            remove_repo: true,
        }
    }
    fn add_remote_repo(&self, _url: &str, _name: &str) -> Result<(), AppErrorV1> {
        Err(AppErrorV1::new(
            self.code,
            ErrorCategory::Validation,
            "errors.vpm.repoRefused",
            "corr-repo-refused",
        ))
    }
    fn add_local_repo(&self, _path: &Path, _name: &str) -> Result<(), AppErrorV1> {
        Err(AppErrorV1::new(
            self.code,
            ErrorCategory::Validation,
            "errors.vpm.repoRefused",
            "corr-repo-refused",
        ))
    }
    fn remove_repo(&self, _repo_id: &str) -> Result<(), AppErrorV1> {
        Err(AppErrorV1::new(
            self.code,
            ErrorCategory::Validation,
            "errors.vpm.repoRefused",
            "corr-repo-refused",
        ))
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
fn ops_v04_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = ops_command_validator();
    let result = ops_result_validator();

    // --- the six positive vectors pass their respective schema ---
    let add_remote = example("repo-add-remote.request.json");
    assert!(
        command.is_valid(&add_remote),
        "{:?}",
        violations(&command, &add_remote)
    );
    let add_remote_receipt = example("repo-add-remote-result-added.result.json");
    assert!(
        result.is_valid(&add_remote_receipt),
        "{:?}",
        violations(&result, &add_remote_receipt)
    );
    let add_local = example("repo-add-local.request.json");
    assert!(
        command.is_valid(&add_local),
        "{:?}",
        violations(&command, &add_local)
    );
    let add_local_receipt = example("repo-add-local-result-added.result.json");
    assert!(
        result.is_valid(&add_local_receipt),
        "{:?}",
        violations(&result, &add_local_receipt)
    );
    let remove_repo = example("repo-remove.request.json");
    assert!(
        command.is_valid(&remove_repo),
        "{:?}",
        violations(&command, &remove_repo)
    );
    let remove_receipt = example("repo-remove-result-removed.result.json");
    assert!(
        result.is_valid(&remove_receipt),
        "{:?}",
        violations(&result, &remove_receipt)
    );

    // --- the eleven negative vectors all fail ---
    for name in [
        "invalid-add-remote-missing-url.request.json",
        "invalid-add-remote-missing-name.request.json",
        "invalid-add-remote-carries-digest.request.json",
        "invalid-add-remote-carries-project-path.request.json",
        "invalid-add-local-empty-path.request.json",
        "invalid-add-local-extra-param.request.json",
        "invalid-remove-missing-repo-id.request.json",
        "invalid-remove-empty-repo-id.request.json",
    ] {
        let negative = example(name);
        assert!(
            !command.is_valid(&negative),
            "{name} must be an invalid command: {:?}",
            violations(&command, &negative)
        );
    }
    for name in [
        "invalid-add-answer-plan.result.json",
        "invalid-result-invented-field.result.json",
        "invalid-result-rejected-code-outside-family.result.json",
    ] {
        let negative = example(name);
        assert!(
            !result.is_valid(&negative),
            "{name} must be an invalid result: {:?}",
            violations(&result, &negative)
        );
    }

    // --- operation/kind lock, inline-pinned: the A4 faces never answer
    // the wrong arm (no preview arm exists; add never answers removed;
    // remove never answers repoReceipt) ---
    let add_answers_removed = json!({
        "schemaVersion": "0.4",
        "operation": "packages.addRemoteRepo",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.4",
            "kind": "removed",
            "repoId": FAKE_REPO_ID,
        },
    });
    assert!(
        !result.is_valid(&add_answers_removed),
        "addRemoteRepo must never answer kind=removed: {:?}",
        violations(&result, &add_answers_removed)
    );
    let remove_answers_repo_receipt = json!({
        "schemaVersion": "0.4",
        "operation": "packages.removeRepo",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.4",
            "kind": "repoReceipt",
            "repoType": "remote",
            "url": FAKE_REMOTE_URL,
            "name": FAKE_REPO_NAME,
        },
    });
    assert!(
        !result.is_valid(&remove_answers_repo_receipt),
        "removeRepo must never answer kind=repoReceipt: {:?}",
        violations(&result, &remove_answers_repo_receipt)
    );
}

#[test]
fn repo_write_capability_absence_keeps_the_declared_none_word_face() {
    // A plain fake with NO A4 override at all: the defaulted accessor
    // answers declared-none, the trait-default methods answer the
    // generic capability_missing — the 025 accessor law.
    struct NoRepoBackend;
    impl VpmBackend for NoRepoBackend {
        fn name(&self) -> &'static str {
            "no-repo-fake"
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

    let backend = NoRepoBackend;
    assert_eq!(backend.name(), "no-repo-fake");
    // Declared-none: all three bits false — the honest absence the wire
    // row projects as unavailable until a real override flips it.
    assert_eq!(
        backend.repo_write_capabilities(),
        RepoWriteCapabilities::NONE
    );

    // The trait-default methods answer the generic capability_missing —
    // never a fabricated repo fact.
    let err = backend
        .add_remote_repo(FAKE_REMOTE_URL, FAKE_REPO_NAME)
        .expect_err("the trait default must refuse");
    assert_eq!(err.code, "vua.vpm.capability_missing");
    assert_eq!(err.category, ErrorCategory::Unavailable);
    let err = backend
        .add_local_repo(Path::new(FAKE_LOCAL_PATH), FAKE_REPO_NAME)
        .expect_err("the trait default must refuse");
    assert_eq!(err.code, "vua.vpm.capability_missing");
    let err = backend
        .remove_repo(FAKE_REPO_ID)
        .expect_err("the trait default must refuse");
    assert_eq!(err.code, "vua.vpm.capability_missing");
}

#[test]
fn fake_repo_backend_port_facts_project_onto_the_a4_wire_shapes() {
    let backend = FakeRepoWriteBackend;
    let caps = backend.repo_write_capabilities();
    assert!(caps.add_remote_repo && caps.add_local_repo && caps.remove_repo);

    let command = ops_command_validator();
    let result = ops_result_validator();

    // --- remote add: unit success → repoReceipt/remote, request echo
    // only, camelCase keys pinned verbatim ---
    let request = json!({
        "schemaVersion": "0.4",
        "operation": "packages.addRemoteRepo",
        "params": { "url": FAKE_REMOTE_URL, "name": FAKE_REPO_NAME },
    });
    assert!(
        command.is_valid(&request),
        "{:?}",
        violations(&command, &request)
    );
    assert_eq!(
        backend
            .add_remote_repo(FAKE_REMOTE_URL, FAKE_REPO_NAME)
            .unwrap(),
        ()
    );
    let receipt = json!({
        "schemaVersion": "0.4",
        "operation": "packages.addRemoteRepo",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.4",
            "kind": "repoReceipt",
            "repoType": "remote",
            "url": FAKE_REMOTE_URL,
            "name": FAKE_REPO_NAME,
        },
    });
    assert!(
        result.is_valid(&receipt),
        "{:?}",
        violations(&result, &receipt)
    );
    assert_eq!(receipt["result"]["repoType"], "remote");
    assert_eq!(receipt["result"]["url"], FAKE_REMOTE_URL);
    assert_eq!(receipt["result"]["name"], FAKE_REPO_NAME);
    // The key set is exactly the declared five — the false-assertion guard.
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
            "repoType".to_string(),
            "url".to_string(),
            "name".to_string(),
        ]
    );

    // --- local add: unit success → repoReceipt/local, path echo ---
    let local_receipt = json!({
        "schemaVersion": "0.4",
        "operation": "packages.addLocalRepo",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.4",
            "kind": "repoReceipt",
            "repoType": "local",
            "path": FAKE_LOCAL_PATH,
            "name": FAKE_REPO_NAME,
        },
    });
    assert!(
        result.is_valid(&local_receipt),
        "{:?}",
        violations(&result, &local_receipt)
    );
    assert_eq!(local_receipt["result"]["repoType"], "local");
    assert_eq!(local_receipt["result"]["path"], FAKE_LOCAL_PATH);

    // --- remove: unit success → removed/{repoId echo}, nothing else ---
    let removed = json!({
        "schemaVersion": "0.4",
        "operation": "packages.removeRepo",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.4",
            "kind": "removed",
            "repoId": FAKE_REPO_ID,
        },
    });
    assert!(
        result.is_valid(&removed),
        "{:?}",
        violations(&result, &removed)
    );
    let removed_keys: Vec<String> = removed["result"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        removed_keys,
        vec![
            "schemaVersion".to_string(),
            "kind".to_string(),
            "repoId".to_string(),
        ]
    );
}

#[test]
fn a4_port_refusals_fold_into_execution_failed_carrying_the_original_code() {
    // The fold discipline (A1/A2/A3 same law): every port refusal folds
    // into a rejected document whose guard is execution_failed (ZERO new
    // guard values on A4) and whose code key stays inside the
    // vua.packages.* family — the original vua.vpm.repo_* code travels
    // verbatim in detail as honest provenance.
    let cases = [
        (
            vpm_backend_error_codes::REPO_INVALID,
            "duplicate repository url",
            "packages.addRemoteRepo",
        ),
        (
            vpm_backend_error_codes::REPO_FETCH_FAILED,
            "remote manifest fetch failed",
            "packages.addRemoteRepo",
        ),
        (
            vpm_backend_error_codes::REPO_WRITE_FAILED,
            "settings write-back failed",
            "packages.addLocalRepo",
        ),
        (
            vpm_backend_error_codes::REPO_NOT_FOUND,
            "no subscription row carries this repoId",
            "packages.removeRepo",
        ),
    ];

    let result_validator = ops_result_validator();
    for (code, detail, operation) in cases {
        let backend = RefusingRepoBackend { code };
        let refusal = match operation {
            "packages.addRemoteRepo" => backend
                .add_remote_repo(FAKE_REMOTE_URL, FAKE_REPO_NAME)
                .err(),
            "packages.addLocalRepo" => backend
                .add_local_repo(Path::new(FAKE_LOCAL_PATH), FAKE_REPO_NAME)
                .err(),
            _ => backend.remove_repo(FAKE_REPO_ID).err(),
        }
        .expect("the refusing backend must refuse");
        assert_eq!(refusal.code, code, "the port answers the original code");

        // The wire projection: the ONLY legal shape folds the refusal —
        // execution_failed + the family-locked code + the original code
        // in detail. Schema-validated.
        let rejected = json!({
            "schemaVersion": "0.4",
            "operation": operation,
            "result": {
                "schemaVersion": "vua.packages-ops/v0.4",
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

#[test]
fn repo_write_bits_are_independent_not_face_level() {
    // A backend that serves ONLY local-add: the other two bits stay
    // declared-none and the corresponding trait-default methods answer
    // the generic capability_missing — the gate is per method, never per
    // face (the honest-subset discipline).
    let backend = LocalOnlyRepoBackend;
    let caps = backend.repo_write_capabilities();
    assert!(!caps.add_remote_repo);
    assert!(caps.add_local_repo);
    assert!(!caps.remove_repo);

    let err = backend
        .add_remote_repo(FAKE_REMOTE_URL, FAKE_REPO_NAME)
        .expect_err("an unimplemented bit must refuse");
    assert_eq!(err.code, "vua.vpm.capability_missing");
    assert_eq!(
        backend
            .add_local_repo(Path::new(FAKE_LOCAL_PATH), FAKE_REPO_NAME)
            .unwrap(),
        ()
    );
    let err = backend
        .remove_repo(FAKE_REPO_ID)
        .expect_err("an unimplemented bit must refuse");
    assert_eq!(err.code, "vua.vpm.capability_missing");
}
