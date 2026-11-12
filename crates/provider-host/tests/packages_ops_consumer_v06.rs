//! packages-ops v0.6 consumer tests (proposal 027 freeze batch, slice F4
//! = repository lifecycle, 2026-09-20): the core-side consumer of the
//! repo enable/disable/refresh write word face. The frozen
//! `schemas/packages-ops/v0.6/` schemas pin the wire command/result
//! shapes; the example vectors drive JSON-Schema validation directly,
//! and a fake backend's port facts (`enable_repo` / `disable_repo`
//! answering `Result<(), _>`, `refresh_repo` answering
//! `Result<RepoRefreshOutcomeV01, _>` — the library's own two-arm
//! refresh outcome) are projected onto the `kind=enabled` /
//! `kind=disabled` / `kind=refreshed` documents and validated against
//! the same schema. The port→wire consumer loop is nailed here first.
//! The REAL backend consumption (`VrcGetLibBackend` implementing the
//! toggle faces over VUA-owned storage under the environment root and
//! the refresh face over the library's etag-conditional update) is the
//! environment implementation-verification slice and lands with its own
//! tests; this file pins the contract side: the ruling-(c) word face
//! (the W25 read-only evidence record settled that VCC carries NO
//! enable/disable state anywhere — the toggle is VUA-owned semantics,
//! stored in VUA's own storage, never a settings.json key, never a
//! userRepos[i] key), the no-preview / no-digest word face (the A3/A4
//! law: the user's explicit submission IS the confirmation — a carried
//! confirmedDigest or projectPath is a shape violation, negative
//! vectors), the disabled-row semantics (excluded from the
//! package-collection world, still listed with the packages-repos v0.2
//! `enabled` bit), the minimal-echo receipts (the A4 removed-arm
//! isomorph — `cacheUpdated` is the ONE fact a receipt adds, and both
//! of its arms are success), the zero-new-codes word face (the three
//! port codes `vua.vpm.repo_not_found` / `vua.vpm.repo_write_failed` /
//! `vua.vpm.repo_fetch_failed` all predate this batch and fold into
//! `execution_failed` carrying the original code in detail — the
//! reuse-code law: the codes never travel in the rejected `code` key),
//! and the three-independent-bits gate (`repo_lifecycle_capabilities`,
//! default declared-none — the trait default IS the absence arm for
//! these defaulted methods, the wire gate reads the same bits BEFORE
//! submit and answers the same generic capability_missing). The wire
//! routes for these methods do not exist yet (wiring is the next core
//! slice); nothing here exercises a running engine. Everything runs
//! against synthetic data — no machine-specific facts, no network, no
//! real repository cache.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, ErrorCategory, PackageRequestV1, ProjectRef, RepoLifecycleCapabilities,
    RepoRefreshOutcomeV01, VpmBackend,
};

fn read_ops_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn ops_command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.6/command.schema.json",
    ))
    .unwrap()
}

fn ops_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.6/result.schema.json",
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
    read_ops_json(&format!("schemas/packages-ops/v0.6/examples/{name}"))
}

const FAKE_REPO_ID: &str = "repo.example.community";

/// A backend that serves the full F4 lifecycle face: all three bits true,
/// the toggle methods answering the port's unit success and the refresh
/// method answering the port's outcome fact while carrying the request
/// argument through verbatim (the repoId pass-through is observable).
struct FakeLifecycleBackend {
    refresh_outcome: bool,
}

impl VpmBackend for FakeLifecycleBackend {
    fn name(&self) -> &'static str {
        "fake-lifecycle"
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
    fn repo_lifecycle_capabilities(&self) -> RepoLifecycleCapabilities {
        RepoLifecycleCapabilities {
            enable_repo: true,
            disable_repo: true,
            refresh_repo: true,
        }
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
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
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        _confirmed_digest: &str,
    ) -> Result<serde_json::Value, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn enable_repo(&self, repo_id: &str) -> Result<(), AppErrorV1> {
        assert_eq!(repo_id, FAKE_REPO_ID, "the id passes through verbatim");
        Ok(())
    }
    fn disable_repo(&self, repo_id: &str) -> Result<(), AppErrorV1> {
        assert_eq!(repo_id, FAKE_REPO_ID, "the id passes through verbatim");
        Ok(())
    }
    fn refresh_repo(&self, repo_id: &str) -> Result<RepoRefreshOutcomeV01, AppErrorV1> {
        assert_eq!(repo_id, FAKE_REPO_ID, "the id passes through verbatim");
        Ok(RepoRefreshOutcomeV01 {
            cache_updated: self.refresh_outcome,
        })
    }
}

/// A backend with the default declared-none lifecycle face — the
/// honest-absence fixture. The defaulted accessors answer NONE and the
/// defaulted methods answer the standing capability_missing; the wire
/// gate reads the same bits BEFORE submit and answers the same code.
struct NoLifecycleBackend;

impl VpmBackend for NoLifecycleBackend {
    fn name(&self) -> &'static str {
        "no-lifecycle-fake"
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
    fn create_project(
        &self,
        _parent: &std::path::Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
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
}

/// A backend that refuses with each of the three F4 port codes in turn —
/// the fold-word-face fixture. All three are EXISTING codes (zero new
/// codes on this batch): the A4-batch repository codes reused.
struct RefusingLifecycleBackend {
    code: &'static str,
}

impl VpmBackend for RefusingLifecycleBackend {
    fn name(&self) -> &'static str {
        "refusing-lifecycle"
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
    fn repo_lifecycle_capabilities(&self) -> RepoLifecycleCapabilities {
        RepoLifecycleCapabilities {
            enable_repo: true,
            disable_repo: true,
            refresh_repo: true,
        }
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
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
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        _confirmed_digest: &str,
    ) -> Result<serde_json::Value, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn enable_repo(&self, _repo_id: &str) -> Result<(), AppErrorV1> {
        Err(self.refusal())
    }
    fn disable_repo(&self, _repo_id: &str) -> Result<(), AppErrorV1> {
        Err(self.refusal())
    }
    fn refresh_repo(&self, _repo_id: &str) -> Result<RepoRefreshOutcomeV01, AppErrorV1> {
        Err(self.refusal())
    }
}

impl RefusingLifecycleBackend {
    fn refusal(&self) -> AppErrorV1 {
        AppErrorV1::new(
            self.code,
            ErrorCategory::Validation,
            "errors.vpm.repoNotFound",
            "corr-lifecycle-refused",
        )
        .with_param(
            "repoId",
            vua_orchestrator::ParamValue::Text(FAKE_REPO_ID.to_string()),
        )
    }
}

#[test]
fn ops_v06_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = ops_command_validator();
    let result = ops_result_validator();

    // --- the five positive vectors pass their respective schema ---
    for name in [
        "repo-enable.request.json",
        "repo-disable.request.json",
        "repo-refresh.request.json",
    ] {
        let positive = example(name);
        assert!(
            command.is_valid(&positive),
            "{name}: {:?}",
            violations(&command, &positive)
        );
    }
    for name in [
        "repo-enable-result-enabled.result.json",
        "repo-refresh-result-refreshed.result.json",
    ] {
        let positive = example(name);
        assert!(
            result.is_valid(&positive),
            "{name}: {:?}",
            violations(&result, &positive)
        );
    }

    // --- the ten negative vectors all fail ---
    for name in [
        "invalid-enable-missing-repo-id.request.json",
        "invalid-disable-empty-repo-id.request.json",
        "invalid-enable-extra-param.request.json",
        "invalid-refresh-carries-digest.request.json",
        "invalid-disable-carries-project-path.request.json",
    ] {
        let negative = example(name);
        assert!(
            !command.is_valid(&negative),
            "{name} must be an invalid command: {:?}",
            violations(&command, &negative)
        );
    }
    for name in [
        "invalid-result-enabled-invented-field.result.json",
        "invalid-result-refreshed-missing-cacheupdated.result.json",
        "invalid-result-refreshed-invented-payload.result.json",
        "invalid-result-rejected-code-outside-family.result.json",
        "invalid-result-enabled-stale-family.result.json",
    ] {
        let negative = example(name);
        assert!(
            !result.is_valid(&negative),
            "{name} must be an invalid result: {:?}",
            violations(&result, &negative)
        );
    }

    // --- operation/kind lock, inline-pinned: the F4 faces never answer
    // the wrong arm (enable never answers disabled or refreshed; refresh
    // never answers enabled; the sibling faces never answer the F4 arms)
    // ---
    let enable_answers_disabled = json!({
        "schemaVersion": "0.6",
        "operation": "packages.enableRepo",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.6",
            "kind": "disabled",
            "repoId": FAKE_REPO_ID,
        },
    });
    assert!(
        !result.is_valid(&enable_answers_disabled),
        "enableRepo must never answer kind=disabled: {:?}",
        violations(&result, &enable_answers_disabled)
    );
    let refresh_answers_enabled = json!({
        "schemaVersion": "0.6",
        "operation": "packages.refreshRepo",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.6",
            "kind": "enabled",
            "repoId": FAKE_REPO_ID,
        },
    });
    assert!(
        !result.is_valid(&refresh_answers_enabled),
        "refreshRepo must never answer kind=enabled: {:?}",
        violations(&result, &refresh_answers_enabled)
    );
    let disable_answers_plan = json!({
        "schemaVersion": "0.6",
        "operation": "packages.disableRepo",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.6",
            "kind": "plan",
        },
    });
    assert!(
        !result.is_valid(&disable_answers_plan),
        "disableRepo has no preview arm and must never answer kind=plan: {:?}",
        violations(&result, &disable_answers_plan)
    );
}

#[test]
fn lifecycle_gate_is_the_new_defaulted_accessor_three_independent_bits() {
    // The F4 gate reads the NEW defaulted accessor — the A4
    // RepoWriteCapabilities law: three INDEPENDENT bits (a backend may
    // serve a subset of the face, and the honest gate is per method,
    // never per face), default declared-none (ORC-DEV-004: no
    // implementation, no reservation). For these DEFAULTED methods the
    // trait default IS the absence arm: the standing capability_missing;
    // the wire gate reads the same bits BEFORE submit and answers the
    // same code — one code, two absence mechanisms, both honest (the
    // A3/A4 law).
    let capable = FakeLifecycleBackend {
        refresh_outcome: true,
    };
    let bits = capable.repo_lifecycle_capabilities();
    assert!(bits.enable_repo && bits.disable_repo && bits.refresh_repo);
    assert_eq!(capable.name(), "fake-lifecycle");

    let incapable = NoLifecycleBackend;
    let none = incapable.repo_lifecycle_capabilities();
    assert_eq!(none, RepoLifecycleCapabilities::NONE);
    assert!(!none.enable_repo && !none.disable_repo && !none.refresh_repo);
    // The trait-default absence arm: the defaulted methods answer the
    // standing capability_missing, never a fabricated success.
    for refusal in [
        incapable
            .enable_repo(FAKE_REPO_ID)
            .expect_err("default must refuse"),
        incapable
            .disable_repo(FAKE_REPO_ID)
            .expect_err("default must refuse"),
    ] {
        assert_eq!(refusal.code, "vua.vpm.capability_missing");
    }
    let refresh_refusal = incapable
        .refresh_repo(FAKE_REPO_ID)
        .expect_err("default must refuse");
    assert_eq!(refresh_refusal.code, "vua.vpm.capability_missing");
}

#[test]
fn fake_lifecycle_backend_port_facts_project_onto_the_f4_wire_shapes() {
    let command = ops_command_validator();
    let result = ops_result_validator();

    // --- enable: unit success → enabled receipt, the echo pinned (the
    // A4 removed-arm isomorph: the id IS the audit link) ---
    let enable_request = json!({
        "schemaVersion": "0.6",
        "operation": "packages.enableRepo",
        "params": { "repoId": FAKE_REPO_ID },
    });
    assert!(
        command.is_valid(&enable_request),
        "{:?}",
        violations(&command, &enable_request)
    );
    FakeLifecycleBackend {
        refresh_outcome: true,
    }
    .enable_repo(FAKE_REPO_ID)
    .expect("the fake backend must enable");
    let enabled_receipt = json!({
        "schemaVersion": "0.6",
        "operation": "packages.enableRepo",
        "result": {
            "schemaVersion": "vua.packages-ops/v0.6",
            "kind": "enabled",
            "repoId": FAKE_REPO_ID,
        },
    });
    assert!(
        result.is_valid(&enabled_receipt),
        "{:?}",
        violations(&result, &enabled_receipt)
    );
    let enabled_keys: Vec<String> = enabled_receipt["result"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        enabled_keys,
        vec![
            "schemaVersion".to_string(),
            "kind".to_string(),
            "repoId".to_string(),
        ],
        "the enabled receipt carries exactly the echo — no state duplicate, no timestamp"
    );

    // --- refresh: the outcome fact → refreshed receipt; BOTH arms of the
    // library outcome are success and both projections are schema-valid
    // (the etag-unchanged arm is an honest outcome, never an error) ---
    for outcome in [true, false] {
        let backend = FakeLifecycleBackend {
            refresh_outcome: outcome,
        };
        let fact = backend
            .refresh_repo(FAKE_REPO_ID)
            .expect("the fake backend must refresh");
        assert_eq!(fact.cache_updated, outcome);
        let refreshed_receipt = json!({
            "schemaVersion": "0.6",
            "operation": "packages.refreshRepo",
            "result": {
                "schemaVersion": "vua.packages-ops/v0.6",
                "kind": "refreshed",
                "repoId": FAKE_REPO_ID,
                "cacheUpdated": fact.cache_updated,
            },
        });
        assert!(
            result.is_valid(&refreshed_receipt),
            "cacheUpdated={outcome}: {:?}",
            violations(&result, &refreshed_receipt)
        );
        let refreshed_keys: Vec<String> = refreshed_receipt["result"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(
            refreshed_keys,
            vec![
                "schemaVersion".to_string(),
                "kind".to_string(),
                "repoId".to_string(),
                "cacheUpdated".to_string(),
            ],
            "cacheUpdated is the ONE fact a refreshed receipt adds — no byte counts, no package lists"
        );
    }
}

#[test]
fn lifecycle_port_refusals_fold_into_execution_failed_carrying_the_original_code() {
    // The fold discipline (A1/A2/A3/A4/A5 same law): every port refusal
    // folds into a rejected document whose guard is execution_failed (ZERO
    // new guard values on F4) and whose code key stays inside the
    // vua.packages.* family — the original code travels verbatim in detail
    // as honest provenance. The F4 closed set is THREE EXISTING codes (the
    // A4-batch repository codes, reused): unknown id, state/cache
    // write-back failure, refresh network segment.
    let cases = [
        (
            "vua.vpm.repo_not_found",
            "no subscription row carries this repoId",
        ),
        (
            "vua.vpm.repo_write_failed",
            "the VUA-owned state-file write-back failed",
        ),
        (
            "vua.vpm.repo_fetch_failed",
            "the etag-conditional fetch failed",
        ),
    ];

    let result_validator = ops_result_validator();
    for (code, detail) in cases {
        let backend = RefusingLifecycleBackend { code };
        let refusal = backend
            .enable_repo(FAKE_REPO_ID)
            .expect_err("the refusing backend must refuse");
        assert_eq!(refusal.code, code, "the port answers the original code");

        // The wire projection: the ONLY legal shape folds the refusal —
        // execution_failed + the family-locked code + the original code
        // in detail. Schema-validated. The reused vua.vpm.* code itself
        // in the code key is INVALID by schema (the negative vector pins
        // it; the inline pin re-asserts the law).
        let rejected = json!({
            "schemaVersion": "0.6",
            "operation": "packages.enableRepo",
            "result": {
                "schemaVersion": "vua.packages-ops/v0.6",
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
