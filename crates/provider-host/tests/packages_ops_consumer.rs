//! packages-ops v0.1 consumer tests (proposal 026 freeze batch, slice A1
//! = removal, 2026-09-19): the core-side consumer of the packages write
//! face. The frozen `schemas/packages-ops/v0.1/` schemas pin the wire
//! command/result shapes; the example vectors drive JSON-Schema
//! validation directly, and a fake backend's port facts
//! (`ChangePreviewV1` / the `apply_remove` `{removed: items}` value) are
//! projected through serde and validated against the same schemas — the
//! port→wire consumer loop is nailed here first. The REAL backend
//! consumption (`VrcGetLibBackend` already carrying `preview_remove` /
//! `apply_remove`) is the environment implementation-verification slice
//! and lands with its own tests; this file pins the contract side, the
//! trait default absence arms, and the recovery word face (digest drift
//! = recoverable conflict, never a silent overwrite — honesty rule 3).
//! The wire routes for this family do not exist yet (wiring is the next
//! core slice); nothing here exercises a running engine. Everything runs
//! against synthetic data — no machine-specific facts, no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use vua_orchestrator::{AppErrorV1, ChangeItemV1, ErrorCategory};
use vua_orchestrator::{ChangeKindV1, ChangePreviewV1, ProjectRef, VpmBackend};

fn read_ops_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn ops_command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.1/command.schema.json",
    ))
    .unwrap()
}

fn ops_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_ops_json(
        "schemas/packages-ops/v0.1/result.schema.json",
    ))
    .unwrap()
}

fn violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

/// The trait-default backend: implements only the methods without
/// defaults, inheriting every default absence arm (the removal faces
/// among them).
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
        _packages: &[vua_orchestrator::PackageRequestV1],
    ) -> Result<ChangePreviewV1, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[vua_orchestrator::PackageRequestV1],
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

/// A backend that implements the A1 removal faces with synthetic facts
/// (the contract-side consumer loop; the real facts are verified by the
/// environment implementation-verification slice).
struct FakeRemoveBackend;

const FAKE_DIGEST: &str = "fnv1a-9e3779b9";

fn fake_preview() -> ChangePreviewV1 {
    ChangePreviewV1 {
        items: vec![
            ChangeItemV1 {
                kind: ChangeKindV1::Remove,
                package_id: "com.lilxyzw.liltoon".into(),
                version: None,
                reason: None,
            },
            ChangeItemV1 {
                kind: ChangeKindV1::Remove,
                package_id: "jp.lilxyzw.avatarloader".into(),
                version: None,
                reason: Some("transitive_dependency".into()),
            },
        ],
        conflicts: vec!["com.another.avatarbase depends on com.lilxyzw.liltoon".into()],
        remove_legacy_files: vec!["Assets/lilToon/Editor/OldMetaFile.txt".into()],
        remove_legacy_folders: vec!["Assets/lilToon".into()],
        destructive: true,
        digest: FAKE_DIGEST.into(),
    }
}

impl VpmBackend for FakeRemoveBackend {
    fn name(&self) -> &'static str {
        "fake-remove"
    }
    fn capabilities(&self) -> vua_orchestrator::VpmCapabilities {
        vua_orchestrator::VpmCapabilities {
            create_project: false,
            preview_install: false,
            list_packages: false,
            remove_packages: true,
            project_registry: false,
            resolve_project: false,
        }
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[vua_orchestrator::PackageRequestV1],
    ) -> Result<ChangePreviewV1, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[vua_orchestrator::PackageRequestV1],
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
    fn preview_remove(
        &self,
        _project: &ProjectRef,
        _package_ids: &[String],
    ) -> Result<ChangePreviewV1, AppErrorV1> {
        Ok(fake_preview())
    }
    fn apply_remove(
        &self,
        _project: &ProjectRef,
        _package_ids: &[String],
        confirmed_digest: &str,
    ) -> Result<serde_json::Value, AppErrorV1> {
        if confirmed_digest != FAKE_DIGEST {
            // The double-digest discipline word face: drift is a
            // RECOVERABLE conflict (re-preview, never implicit resumption).
            return Err(AppErrorV1::new(
                "vua.vpm.preview_drift",
                ErrorCategory::Conflict,
                "errors.vpm.previewDrift",
                "corr-vpm-remove",
            )
            .with_recoverable(true));
        }
        let preview = fake_preview();
        Ok(json!({ "removed": preview.items }))
    }
}

#[test]
fn ops_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = ops_command_validator();
    let result = ops_result_validator();

    for positive in [
        "schemas/packages-ops/v0.1/examples/packages-preview-remove.request.json",
        "schemas/packages-ops/v0.1/examples/packages-apply-remove.request.json",
    ] {
        let doc = read_ops_json(positive);
        assert!(
            command.is_valid(&doc),
            "{positive}: {:?}",
            violations(&command, &doc)
        );
    }
    for positive in [
        "schemas/packages-ops/v0.1/examples/packages-preview-remove.result.json",
        "schemas/packages-ops/v0.1/examples/packages-apply-remove-result-receipt.result.json",
    ] {
        let doc = read_ops_json(positive);
        assert!(
            result.is_valid(&doc),
            "{positive}: {:?}",
            violations(&result, &doc)
        );
    }

    // A preview request carries NO digest slot: the digest is the
    // preview's product, carrying one is a shape violation.
    let carries_digest = read_ops_json(
        "schemas/packages-ops/v0.1/examples/invalid-preview-extra-param.request.json",
    );
    assert!(
        !command.is_valid(&carries_digest),
        "a preview request with a digest must be invalid"
    );

    // The explicit closed list: no wildcard, no empty "remove everything".
    let empty_ids =
        read_ops_json("schemas/packages-ops/v0.1/examples/invalid-empty-package-ids.request.json");
    assert!(
        !command.is_valid(&empty_ids),
        "empty packageIds must be invalid"
    );

    // apply without the confirmed digest has no confirmation to bind to.
    let missing_digest = read_ops_json(
        "schemas/packages-ops/v0.1/examples/invalid-apply-missing-digest.request.json",
    );
    assert!(
        !command.is_valid(&missing_digest),
        "apply without confirmedDigest must be invalid"
    );

    // The false-assertion guard: facts with no port carrier are invalid
    // by schema, not merely discouraged.
    let plan_invented =
        read_ops_json("schemas/packages-ops/v0.1/examples/invalid-plan-invented-field.result.json");
    assert!(
        !result.is_valid(&plan_invented),
        "invented updateAvailable on the plan must be invalid"
    );

    let receipt_invented = read_ops_json(
        "schemas/packages-ops/v0.1/examples/invalid-receipt-invented-field.result.json",
    );
    assert!(
        !result.is_valid(&receipt_invented),
        "invented reInspection on the receipt must be invalid"
    );

    // The error-code family is vua.packages.* for this word face; a
    // reused 013 code inside the rejected arm is outside the family.
    let code_outside = read_ops_json(
        "schemas/packages-ops/v0.1/examples/invalid-rejected-code-outside-family.result.json",
    );
    assert!(
        !result.is_valid(&code_outside),
        "a non-vua.packages.* code must be invalid"
    );

    // Operation/kind lock: the synchronous preview answers kind=plan
    // exactly, never a task-terminal arm.
    let kind_mismatch = read_ops_json(
        "schemas/packages-ops/v0.1/examples/invalid-preview-result-kind-receipt.result.json",
    );
    assert!(
        !result.is_valid(&kind_mismatch),
        "previewRemove must never answer kind=receipt"
    );

    // The change-item word is the port's two-state kind; "upgrade" is
    // outside the frozen word list (A2's face, not invented here).
    let bad_kind =
        read_ops_json("schemas/packages-ops/v0.1/examples/invalid-item-bad-kind.result.json");
    assert!(
        !result.is_valid(&bad_kind),
        "item kind=upgrade is outside the two-state word"
    );
}

#[test]
fn trait_default_answers_capability_missing_and_declares_false() {
    let backend = MinimalBackend;
    assert!(!backend.capabilities().remove_packages);

    let project = ProjectRef {
        id: "demo".into(),
        root: std::path::PathBuf::from("C:/proj"),
    };
    let ids = vec!["com.example.pkg".to_string()];

    let err = backend.preview_remove(&project, &ids).unwrap_err();
    assert_eq!(err.code, "vua.vpm.capability_missing");

    let err = backend
        .apply_remove(&project, &ids, FAKE_DIGEST)
        .unwrap_err();
    assert_eq!(err.code, "vua.vpm.capability_missing");
}

#[test]
fn fake_backend_port_facts_project_onto_the_frozen_wire_shapes() {
    let backend = FakeRemoveBackend;
    assert!(backend.capabilities().remove_packages);
    let project = ProjectRef {
        id: "demo".into(),
        root: std::path::PathBuf::from("C:/VRChat/Projects/Chiffon"),
    };
    let ids = vec!["com.lilxyzw.liltoon".to_string()];

    // Port facts → serde projection → plan-arm wire envelope → schema.
    let preview = backend.preview_remove(&project, &ids).unwrap();
    let mut plan = serde_json::to_value(&preview).unwrap();
    plan["schemaVersion"] = json!("vua.packages-ops/v0.1");
    plan["kind"] = json!("plan");
    plan["projectPath"] = json!(project.root.to_string_lossy());
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "packages.previewRemove",
        "result": plan,
    });
    let validator = ops_result_validator();
    assert!(
        validator.is_valid(&envelope),
        "{:?}",
        violations(&validator, &envelope)
    );
    assert_eq!(envelope["result"]["schemaVersion"], "vua.packages-ops/v0.1");
    // The frozen camelCase projection of the port facts.
    assert_eq!(
        envelope["result"]["destructive"],
        serde_json::Value::Bool(true)
    );
    assert_eq!(
        envelope["result"]["removeLegacyFolders"][0],
        "Assets/lilToon"
    );
    let items = envelope["result"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["packageId"], "com.lilxyzw.liltoon");
    assert_eq!(items[0]["kind"], "remove");
    assert_eq!(items[0]["version"], serde_json::Value::Null);
    assert_eq!(items[1]["reason"], "transitive_dependency");
    assert_eq!(envelope["result"]["digest"], FAKE_DIGEST);

    // The receipt: the confirmed digest echo + the request list + the
    // backend's actual result, validated against the same schema.
    let applied = backend.apply_remove(&project, &ids, FAKE_DIGEST).unwrap();
    let mut receipt = json!({
        "schemaVersion": "vua.packages-ops/v0.1",
        "kind": "receipt",
        "projectPath": project.root.to_string_lossy(),
        "confirmedDigest": FAKE_DIGEST,
        "requestedPackageIds": ids,
        "removedItems": applied["removed"],
    });
    receipt["requestedPackageIds"] = json!(ids);
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "packages.applyRemove",
        "result": receipt,
    });
    let validator = ops_result_validator();
    assert!(
        validator.is_valid(&envelope),
        "{:?}",
        violations(&validator, &envelope)
    );
    let removed = envelope["result"]["removedItems"].as_array().unwrap();
    assert_eq!(removed.len(), 2);
    assert_eq!(removed[0]["packageId"], "com.lilxyzw.liltoon");
}

#[test]
fn digest_drift_is_a_recoverable_conflict_word_face() {
    let backend = FakeRemoveBackend;
    let project = ProjectRef {
        id: "demo".into(),
        root: std::path::PathBuf::from("C:/proj"),
    };
    let ids = vec!["com.lilxyzw.liltoon".to_string()];

    // The apply binds to the confirmed digest; drift is refused with the
    // RECOVERABLE conflict word face (re-preview and re-confirm, never a
    // silent overwrite, never an implicit resumption — honesty rule 3).
    let err = backend
        .apply_remove(&project, &ids, "fnv1a-stale")
        .unwrap_err();
    assert_eq!(err.code, "vua.vpm.preview_drift");
    assert!(err.recoverable, "drift must surface as recoverable");

    // The matching digest executes and the receipt's removedItems carry
    // the change items verbatim (the consumer loop's audit linkage).
    let applied = backend.apply_remove(&project, &ids, FAKE_DIGEST).unwrap();
    assert!(applied["removed"].is_array());
}
