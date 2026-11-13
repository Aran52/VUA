//! packages-templates v0.1 consumer tests (proposal 027 F5 freeze batch,
//! 2026-09-20): the core-side consumer of the template-enumeration read
//! face. The frozen `schemas/packages-templates/v0.1/` schemas pin the wire
//! request/result shapes; the example vectors drive JSON-Schema validation
//! directly, and a fake backend's port facts are projected through serde and
//! validated against the same schemas — the port->wire consumer loop is
//! nailed here first. The REAL backend consumption (`VrcGetLibBackend`
//! implementing `list_templates` over the two pinned directory roots) is the
//! environment implementation slice and lands with its own tests; this file
//! pins the contract side and the trait default absence arms. The word-face
//! rulings pinned here: the enumeration is a local directory scan (the
//! environment-verification enumeration-shape ruling — NOT listed as a
//! non-goal; the founding authority is U14 ruling 3 + the 027 core stance
//! 5); the explicit-path leg of create's three-candidate resolution has no
//! reach on this face; duplicate names across the two roots enumerate ONCE,
//! resolved to the root the creation resolution order would pick; rows are
//! id-ascending (the frozen presentation fact); `name` is the frozen
//! same-value display projection of `id` (same-value enforcement is a
//! producer contract — not expressible across keys in JSON Schema draft-07,
//! so it is pinned HERE against the port facts); there is NO
//! description/metadata field (no v0.1 producer — ORC-DEV-004, the P1
//! displayName precedent) and NO sourceRoot field (which root serves an id
//! is create's frozen resolution order, not a per-row fact) — invention is
//! INVALID by schema, negative vectors pin it; an EMPTY listing is the
//! honest zero-templates answer (a missing root is a fact, never an error —
//! the R4 precedent); the face is zero-network so NO cacheSourced disclosure
//! exists (the packages-repos v0.1 law: a constant informational field is
//! not a fact); zero new error codes (the standing envelope set). Everything
//! runs against synthetic data — no machine-specific facts, no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use vua_orchestrator::{AppErrorV1, PackageRequestV1, ProjectRef, TemplateEntryV01, VpmBackend};

fn read_repo_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-templates/v0.1/command.schema.json",
    ))
    .unwrap()
}

fn result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-templates/v0.1/result.schema.json",
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
/// inheriting every default absence arm — among them the F5 declaration
/// (`template_capabilities` default NONE) and the F5 method (`list_templates`
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

/// A backend that implements the F5 template-enumeration face (the
/// contract-side consumer loop; the real directory scan comes with the
/// environment implementation slice). The synthetic facts cover the frozen
/// word face: entries from both roots (the duplicate-name rule collapsed
/// VRCTemplates-first), id-ascending, and the honest-empty arm.
struct FakeTemplatesBackend;

impl VpmBackend for FakeTemplatesBackend {
    fn name(&self) -> &'static str {
        "fake-templates"
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
        _parent: &std::path::Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn template_capabilities(&self) -> vua_orchestrator::TemplateCapabilities {
        vua_orchestrator::TemplateCapabilities {
            list_templates: true,
        }
    }
    fn list_templates(&self) -> Result<Vec<TemplateEntryV01>, AppErrorV1> {
        // Synthetic facts: Avatar exists under BOTH roots and enumerates ONCE
        // (resolved to the root the creation resolution order would pick —
        // VRCTemplates-first); Base/World are plain entries. id-ascending —
        // the frozen presentation fact.
        Ok(vec![
            TemplateEntryV01 {
                id: "Avatar".into(),
                name: "Avatar".into(),
            },
            TemplateEntryV01 {
                id: "Base".into(),
                name: "Base".into(),
            },
            TemplateEntryV01 {
                id: "World".into(),
                name: "World".into(),
            },
        ])
    }
}

#[test]
fn templates_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = command_validator();
    let result = result_validator();

    // The single positive request vector: the closed empty params set.
    let request = read_repo_json(
        "schemas/packages-templates/v0.1/examples/packages-list-templates.request.json",
    );
    assert!(
        command.is_valid(&request),
        "{:?}",
        violations(&command, &request)
    );

    for positive in [
        "schemas/packages-templates/v0.1/examples/packages-list-templates.result.json",
        "schemas/packages-templates/v0.1/examples/packages-list-templates-empty.result.json",
    ] {
        let doc = read_repo_json(positive);
        assert!(
            result.is_valid(&doc),
            "{positive}: {:?}",
            violations(&result, &doc)
        );
    }

    for negative in [
        "schemas/packages-templates/v0.1/examples/invalid-templates-params-extra-key.request.json",
        "schemas/packages-templates/v0.1/examples/invalid-templates-unknown-operation.request.json",
    ] {
        let doc = read_repo_json(negative);
        assert!(!command.is_valid(&doc), "{negative} must be invalid");
    }

    for negative in [
        "schemas/packages-templates/v0.1/examples/invalid-templates-row-invented-description.result.json",
        "schemas/packages-templates/v0.1/examples/invalid-templates-row-invented-sourceroot.result.json",
        "schemas/packages-templates/v0.1/examples/invalid-templates-row-missing-name.result.json",
        "schemas/packages-templates/v0.1/examples/invalid-templates-row-empty-id.result.json",
    ] {
        let doc = read_repo_json(negative);
        assert!(!result.is_valid(&doc), "{negative} must be invalid");
    }
}

#[test]
fn trait_default_declares_no_template_face_and_answers_the_standing_absence() {
    let backend = MinimalBackend;
    // The 025 accessor law (ORC-DEV-004): the default declaration is
    // declared-none, and the unimplemented method answers the generic
    // capability absence — never a fabricated listing, never an empty
    // array posing as a fact.
    assert!(!backend.template_capabilities().list_templates);
    let err = backend.list_templates().unwrap_err();
    assert_eq!(err.code, "vua.vpm.capability_missing");
}

#[test]
fn fake_backend_port_facts_project_onto_the_frozen_templates_wire_shape() {
    let backend = FakeTemplatesBackend;
    assert!(backend.template_capabilities().list_templates);

    // Port facts -> serde projection -> result schema; the family const is
    // an envelope-assembly fact, stamped here exactly as the route stamps it
    // (the wire route/envelope consts land with the NEXT core wiring slice,
    // per the A3/A4/A5/F2/F3 precedent — the freeze batch pins the word
    // face, not the route).
    let templates = backend.list_templates().unwrap();
    let result = serde_json::to_value(&templates).unwrap();
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "packages.listTemplates",
        "result": {
            "schemaVersion": "vua.packages-templates/v0.1",
            "templates": result,
        },
    });
    let validator = result_validator();
    assert!(
        validator.is_valid(&envelope),
        "{:?}",
        violations(&validator, &envelope)
    );

    let rows = envelope["result"]["templates"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    // id-ascending order — the frozen presentation fact.
    let ids: Vec<&str> = rows.iter().map(|row| row["id"].as_str().unwrap()).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "rows stay id-ascending");
    // The same-value display projection: name === id for every row (the
    // producer contract the JSON Schema cannot express across keys —
    // pinned HERE against the port facts).
    for row in rows {
        assert_eq!(
            row["name"], row["id"],
            "name is the same-value display projection of id"
        );
    }
    // The deliberate ceiling: no invented metadata/root facts ride this face.
    for key in [
        "description",
        "sourceRoot",
        "author",
        "license",
        "cacheSourced",
    ] {
        assert!(
            rows[0].get(key).is_none(),
            "the v0.1 word list carries NO {key} field — never invented"
        );
    }
}

#[test]
fn empty_listing_is_the_honest_zero_templates_answer() {
    // A missing root or an empty pair of roots is a FACT, never an error
    // (the R4 precedent): the empty listing validates against the frozen
    // schema and the word face carries no error code for it.
    let empty: Vec<TemplateEntryV01> = vec![];
    let result = serde_json::to_value(&empty).unwrap();
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "packages.listTemplates",
        "result": {
            "schemaVersion": "vua.packages-templates/v0.1",
            "templates": result,
        },
    });
    let validator = result_validator();
    assert!(
        validator.is_valid(&envelope),
        "{:?}",
        violations(&validator, &envelope)
    );
    assert_eq!(
        envelope["result"]["templates"].as_array().unwrap().len(),
        0,
        "the empty listing is a valid honest answer"
    );
}
