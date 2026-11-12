//! packages-ops v0.2 consumer tests (proposal 026 freeze batch, slice A2
//! = install/upgrade, 2026-09-19): the core-side consumer of the install
//! word face. The frozen `schemas/packages-ops/v0.2/` schemas pin the
//! wire command/result shapes; the example vectors drive JSON-Schema
//! validation directly, and a fake backend's port facts
//! (`ChangePreviewV1` with install AND conflict-triggered remove rows /
//! the `apply_install` `{applied: items}` value / the `PackageRequestV1`
//! version-selection rows) are projected through serde and validated
//! against the same schemas — the port→wire consumer loop is nailed here
//! first. The REAL backend consumption (`VrcGetLibBackend` already
//! carrying `preview_install` / `apply_install`) is the environment
//! implementation-verification slice and lands with its own tests; this
//! file pins the contract side, the capability-absence word face (a
//! backend without the `preview_install` bit answers the generic
//! capability_missing — the VccCliBackend semantic), and the recovery
//! word face (digest drift = recoverable conflict, never a silent
//! overwrite — honesty rule 3). The wire routes for this family do not
//! exist yet (wiring is the next core slice); nothing here exercises a
//! running engine. Everything runs against synthetic data — no
//! machine-specific facts, no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use vua_orchestrator::{AppErrorV1, ChangeItemV1, ErrorCategory};
use vua_orchestrator::{ChangeKindV1, ChangePreviewV1, PackageRequestV1, ProjectRef, VpmBackend};

fn read_ops_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn ops_command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.2/command.schema.json",
    ))
    .unwrap()
}

fn ops_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.2/result.schema.json",
    ))
    .unwrap()
}

fn violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

/// A backend that implements the A2 install faces with synthetic facts
/// (the contract-side consumer loop; the real facts are verified by the
/// environment implementation-verification slice). The preview carries
/// an install row, a transitive-dependency install row, AND a
/// conflict-triggered remove row — the port fact that the v0.2 plan
/// shape serves both kinds on the install face.
struct FakeInstallBackend;

const FAKE_DIGEST: &str = "fnv1a-7f3a91c2";

fn fake_requests() -> Vec<PackageRequestV1> {
    vec![
        // version = None → the resolver picks (latest stable).
        PackageRequestV1 {
            package_id: "com.lilxyzw.liltoon".into(),
            version: None,
        },
        // version = Some → the exact pin (upgrade and downgrade share the
        // pin syntax).
        PackageRequestV1 {
            package_id: "com.another.toolkit".into(),
            version: Some("3.1.4".into()),
        },
    ]
}

fn fake_preview() -> ChangePreviewV1 {
    ChangePreviewV1 {
        items: vec![
            ChangeItemV1 {
                kind: ChangeKindV1::Install,
                package_id: "com.another.toolkit".into(),
                version: Some("3.1.4".into()),
                reason: None,
            },
            ChangeItemV1 {
                kind: ChangeKindV1::Install,
                package_id: "com.lilxyzw.liltoon".into(),
                version: Some("7.3.150".into()),
                reason: None,
            },
            // The conflict-triggered removal: the port reports it as a
            // Remove change on the install face — the plan covers every
            // change the backend will make (ORC-WF-002).
            ChangeItemV1 {
                kind: ChangeKindV1::Remove,
                package_id: "com.example.conflictingpackage".into(),
                version: None,
                reason: Some("conflict".into()),
            },
        ],
        conflicts: vec!["com.example.conflictingpackage conflicts with com.lilxyzw.liltoon".into()],
        remove_legacy_files: vec![],
        remove_legacy_folders: vec!["Assets/lilToon".into()],
        destructive: true,
        digest: FAKE_DIGEST.into(),
    }
}

impl VpmBackend for FakeInstallBackend {
    fn name(&self) -> &'static str {
        "fake-install"
    }
    fn capabilities(&self) -> vua_orchestrator::VpmCapabilities {
        vua_orchestrator::VpmCapabilities {
            create_project: false,
            preview_install: true,
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
    ) -> Result<ChangePreviewV1, AppErrorV1> {
        Ok(fake_preview())
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        confirmed_digest: &str,
    ) -> Result<serde_json::Value, AppErrorV1> {
        if confirmed_digest != FAKE_DIGEST {
            // The double-digest discipline word face: drift is a
            // RECOVERABLE conflict (re-preview, never implicit resumption).
            return Err(AppErrorV1::new(
                "vua.vpm.preview_drift",
                ErrorCategory::Conflict,
                "errors.vpm.previewDrift",
                "corr-vpm-apply",
            )
            .with_recoverable(true));
        }
        let preview = fake_preview();
        // The port fact: apply_install answers {applied: items} verbatim.
        Ok(json!({ "applied": preview.items }))
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

/// A backend WITHOUT the install capability: the honest word face is the
/// generic capability_missing (the VccCliBackend semantic — no preview
/// capability, no plan-confirmed installs, ADR-0006).
struct NoInstallBackend;

impl VpmBackend for NoInstallBackend {
    fn name(&self) -> &'static str {
        "no-install-fake"
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
    ) -> Result<ChangePreviewV1, AppErrorV1> {
        Err(AppErrorV1::new(
            "vua.vpm.capability_missing",
            ErrorCategory::Unavailable,
            "errors.vpm.capabilityMissing",
            "corr-vpm-backend",
        ))
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        _confirmed_digest: &str,
    ) -> Result<serde_json::Value, AppErrorV1> {
        Err(AppErrorV1::new(
            "vua.vpm.capability_missing",
            ErrorCategory::Unavailable,
            "errors.vpm.capabilityMissing",
            "corr-vpm-backend",
        ))
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
fn ops_v02_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = ops_command_validator();
    let result = ops_result_validator();

    for positive in [
        "schemas/packages-ops/v0.2/examples/packages-preview-install.request.json",
        "schemas/packages-ops/v0.2/examples/packages-apply-install.request.json",
    ] {
        let doc = read_ops_json(positive);
        assert!(
            command.is_valid(&doc),
            "{positive}: {:?}",
            violations(&command, &doc)
        );
    }
    for positive in [
        "schemas/packages-ops/v0.2/examples/packages-preview-install.result.json",
        "schemas/packages-ops/v0.2/examples/packages-apply-install-result-receipt.result.json",
    ] {
        let doc = read_ops_json(positive);
        assert!(
            result.is_valid(&doc),
            "{positive}: {:?}",
            violations(&result, &doc)
        );
    }

    // The explicit closed request list: no empty "install nothing".
    let empty_packages = read_ops_json(
        "schemas/packages-ops/v0.2/examples/invalid-preview-install-empty-packages.request.json",
    );
    assert!(
        !command.is_valid(&empty_packages),
        "empty packages must be invalid"
    );

    // A preview request carries NO digest slot: the digest is the
    // preview's product, carrying one is a shape violation.
    let carries_digest = read_ops_json(
        "schemas/packages-ops/v0.2/examples/invalid-preview-install-carries-digest.request.json",
    );
    assert!(
        !command.is_valid(&carries_digest),
        "a preview request with a digest must be invalid"
    );

    // apply without the confirmed digest has no confirmation to bind to.
    let missing_digest = read_ops_json(
        "schemas/packages-ops/v0.2/examples/invalid-apply-install-missing-digest.request.json",
    );
    assert!(
        !command.is_valid(&missing_digest),
        "apply without confirmedDigest must be invalid"
    );

    // The request row is a closed two-key shape; invented presentation
    // facts are invalid by schema, not merely discouraged.
    let row_extra = read_ops_json(
        "schemas/packages-ops/v0.2/examples/invalid-install-package-row-extra-key.request.json",
    );
    assert!(
        !command.is_valid(&row_extra),
        "an invented displayName on the request row must be invalid"
    );

    // The false-assertion guard on the plan: no invented read-face facts.
    let plan_invented = read_ops_json(
        "schemas/packages-ops/v0.2/examples/invalid-install-plan-invented-field.result.json",
    );
    assert!(
        !result.is_valid(&plan_invented),
        "invented updateAvailable on the install plan must be invalid"
    );

    let receipt_invented = read_ops_json(
        "schemas/packages-ops/v0.2/examples/invalid-install-receipt-invented-field.result.json",
    );
    assert!(
        !result.is_valid(&receipt_invented),
        "invented downloadedBytes on the receipt must be invalid"
    );

    // The error-code family is vua.packages.* for this word face; the
    // port-level vua.vpm.* code never enters a rejected document.
    let code_outside = read_ops_json("schemas/packages-ops/v0.2/examples/invalid-install-rejected-code-outside-family.result.json");
    assert!(
        !result.is_valid(&code_outside),
        "a non-vua.packages.* code must be invalid"
    );

    // Operation/kind lock: the synchronous install preview answers
    // kind=plan exactly, never a task-terminal arm.
    let kind_mismatch = read_ops_json(
        "schemas/packages-ops/v0.2/examples/invalid-preview-install-answer-receipt.result.json",
    );
    assert!(
        !result.is_valid(&kind_mismatch),
        "previewInstall must never answer kind=receipt"
    );

    // The explicit closed request list: an exactly repeated row is a
    // word-face violation at the SCHEMA layer too (uniqueItems). NOTE
    // (026 A2 shape-approval pin-gap closure, wt-3 2026-09-19): the same
    // id across rows with DIFFERENT versions is beyond JSON Schema's
    // cross-row expressiveness — it is pinned by the TS guard narrowing
    // and the wire-layer request check, never admitted end to end.
    let duplicate_row = read_ops_json(
        "schemas/packages-ops/v0.2/examples/invalid-install-package-row-duplicate.request.json",
    );
    assert!(
        !command.is_valid(&duplicate_row),
        "an exactly repeated request row must be invalid"
    );
}

#[test]
fn install_capability_absence_answers_the_generic_capability_missing() {
    let backend = NoInstallBackend;
    assert!(!backend.capabilities().preview_install);

    let project = ProjectRef {
        id: "demo".into(),
        root: std::path::PathBuf::from("C:/proj"),
    };
    let requests = fake_requests();

    // A backend without the install capability refuses BOTH methods with
    // the generic code — the honest absence the wire gate projects
    // (the frozen command schema's serving gate: one bit, both methods).
    let err = backend.preview_install(&project, &requests).unwrap_err();
    assert_eq!(err.code, "vua.vpm.capability_missing");

    let err = backend
        .apply_install(&project, &requests, FAKE_DIGEST)
        .unwrap_err();
    assert_eq!(err.code, "vua.vpm.capability_missing");
}

#[test]
fn fake_install_backend_port_facts_project_onto_the_frozen_wire_shapes() {
    let backend = FakeInstallBackend;
    assert!(backend.capabilities().preview_install);
    let project = ProjectRef {
        id: "demo".into(),
        root: std::path::PathBuf::from("C:/VRChat/Projects/Chiffon"),
    };
    let requests = fake_requests();

    // Port facts → serde projection → plan-arm wire envelope → schema.
    let preview = backend.preview_install(&project, &requests).unwrap();
    let mut plan = serde_json::to_value(&preview).unwrap();
    plan["schemaVersion"] = json!("vua.packages-ops/v0.2");
    plan["kind"] = json!("plan");
    plan["projectPath"] = json!(project.root.to_string_lossy());
    let envelope = json!({
        "schemaVersion": "0.2",
        "operation": "packages.previewInstall",
        "result": plan,
    });
    let validator = ops_result_validator();
    assert!(
        validator.is_valid(&envelope),
        "{:?}",
        violations(&validator, &envelope)
    );
    assert_eq!(envelope["result"]["schemaVersion"], "vua.packages-ops/v0.2");
    // The frozen camelCase projection of the port facts, including the
    // conflict-triggered remove row on the install face.
    assert_eq!(
        envelope["result"]["destructive"],
        serde_json::Value::Bool(true)
    );
    let items = envelope["result"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["kind"], "install");
    assert_eq!(items[0]["packageId"], "com.another.toolkit");
    assert_eq!(items[0]["version"], "3.1.4");
    assert_eq!(items[1]["version"], "7.3.150");
    assert_eq!(items[2]["kind"], "remove");
    assert_eq!(items[2]["packageId"], "com.example.conflictingpackage");
    assert_eq!(items[2]["version"], serde_json::Value::Null);
    assert_eq!(items[2]["reason"], "conflict");
    assert_eq!(envelope["result"]["digest"], FAKE_DIGEST);

    // The receipt: the confirmed digest echo + the request rows verbatim
    // (version-selection semantics included) + the backend's actual
    // applied items, validated against the same schema.
    let applied = backend
        .apply_install(&project, &requests, FAKE_DIGEST)
        .unwrap();
    let receipt = json!({
        "schemaVersion": "vua.packages-ops/v0.2",
        "kind": "receipt",
        "projectPath": project.root.to_string_lossy(),
        "confirmedDigest": FAKE_DIGEST,
        "requestedPackages": serde_json::to_value(&requests).unwrap(),
        "appliedItems": applied["applied"],
    });
    let envelope = json!({
        "schemaVersion": "0.2",
        "operation": "packages.applyInstall",
        "result": receipt,
    });
    let validator = ops_result_validator();
    assert!(
        validator.is_valid(&envelope),
        "{:?}",
        violations(&validator, &envelope)
    );
    // The request rows project through the same camelCase shape
    // ({packageId, version}) — the resolver-picked null rides verbatim.
    let requested = envelope["result"]["requestedPackages"].as_array().unwrap();
    assert_eq!(requested.len(), 2);
    assert_eq!(requested[0]["packageId"], "com.lilxyzw.liltoon");
    assert_eq!(requested[0]["version"], serde_json::Value::Null);
    assert_eq!(requested[1]["version"], "3.1.4");
    let applied_items = envelope["result"]["appliedItems"].as_array().unwrap();
    assert_eq!(applied_items.len(), 3);
}

#[test]
fn install_digest_drift_is_a_recoverable_conflict_word_face() {
    let backend = FakeInstallBackend;
    let project = ProjectRef {
        id: "demo".into(),
        root: std::path::PathBuf::from("C:/proj"),
    };
    let requests = fake_requests();

    // The apply binds to the confirmed digest; drift is refused with the
    // RECOVERABLE conflict word face (re-preview and re-confirm, never a
    // silent overwrite, never an implicit resumption — honesty rule 3).
    let err = backend
        .apply_install(&project, &requests, "fnv1a-stale")
        .unwrap_err();
    assert_eq!(err.code, "vua.vpm.preview_drift");
    assert!(err.recoverable, "drift must surface as recoverable");

    // The matching digest executes and the receipt's appliedItems carry
    // the change items verbatim (the consumer loop's audit linkage).
    let applied = backend
        .apply_install(&project, &requests, FAKE_DIGEST)
        .unwrap();
    assert!(applied["applied"].is_array());
}
