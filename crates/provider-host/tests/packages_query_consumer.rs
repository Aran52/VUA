//! packages-query v0.1 consumer tests (proposal 024 P1 freeze batch,
//! 2026-09-17): the library-side fact source of the P1 read face is
//! `VrcGetLibBackend::list_packages` — the VPM manifest + lock projection
//! (`packageId` / `version` / `dependencies`, sorted). The frozen
//! `schemas/packages-query/v0.1/` schemas pin the wire result shape and the
//! example vectors drive JSON-Schema validation directly; changing that
//! word list without this consumer fails here first. Everything runs
//! against synthetic directory trees — no test depends on this machine's
//! real installs, and no network is touched (offline environment root).

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use vua_orchestrator::{ProjectRef, VpmBackend};
use vua_project_manager::VrcGetLibBackend;

fn read_repo_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-query/v0.1/command.schema.json",
    ))
    .unwrap()
}

fn result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/packages-query/v0.1/result.schema.json",
    ))
    .unwrap()
}

fn violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

fn unique_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-packages-query-{label}-{nanos}"))
}

/// A synthetic registered-shape VPM project with two locked packages, one
/// of which declares a direct dependency on the other (the dependency face
/// is a real fact of the P1 projection). Each package needs a real
/// package.json body: the backend lists the package entities it finds in
/// the Packages tree, reconciled against the lock.
fn synthetic_vpm_project(base: &Path) -> ProjectRef {
    let root = base.join("MyAvatarProject");
    fs::create_dir_all(root.join("Assets")).unwrap();
    fs::create_dir_all(root.join("Packages")).unwrap();
    fs::create_dir_all(root.join("ProjectSettings")).unwrap();
    fs::write(
        root.join("ProjectSettings").join("ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1\n",
    )
    .unwrap();
    fs::write(
        root.join("Packages").join("vpm-manifest.json"),
        r#"{
  "dependencies": {
    "com.demo.avatar-kit": { "version": "1.2.3" }
  },
  "locked": {
    "com.demo.avatar-kit": {
      "version": "1.2.3",
      "vpmDependencies": { "com.demo.core-lib": "0.9.0" }
    },
    "com.demo.core-lib": {
      "version": "0.9.0",
      "vpmDependencies": {}
    }
  }
}"#,
    )
    .unwrap();
    let avatar_kit = root.join("Packages").join("com.demo.avatar-kit");
    fs::create_dir_all(&avatar_kit).unwrap();
    fs::write(
        avatar_kit.join("package.json"),
        r#"{
  "name": "com.demo.avatar-kit",
  "version": "1.2.3",
  "vpmDependencies": { "com.demo.core-lib": "0.9.0" }
}"#,
    )
    .unwrap();
    let core_lib = root.join("Packages").join("com.demo.core-lib");
    fs::create_dir_all(&core_lib).unwrap();
    fs::write(
        core_lib.join("package.json"),
        r#"{ "name": "com.demo.core-lib", "version": "0.9.0" }"#,
    )
    .unwrap();
    ProjectRef {
        id: "packages-query-consumer".to_owned(),
        root,
    }
}

fn offline_backend(base: &Path) -> VrcGetLibBackend {
    VrcGetLibBackend::with_environment_root(base.to_path_buf(), true)
        .expect("offline vrc-get backend constructs")
}

/// The frozen example vectors drive both schemas directly: every
/// `invalid-*` file must FAIL its schema, every other file must validate.
#[test]
fn frozen_example_vectors_match_the_schemas() {
    let examples_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/packages-query/v0.1/examples");
    let mut checked = 0;
    for entry in fs::read_dir(&examples_dir).expect("examples directory exists") {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let instance: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let (validator, kind) = if name.ends_with(".request.json") {
            (command_validator(), "command")
        } else {
            (result_validator(), "result")
        };
        let valid = validator.is_valid(&instance);
        if name.starts_with("invalid-") {
            assert!(
                !valid,
                "{name} must be REFUSED by the frozen {kind} schema (false-assertion guard): {instance}"
            );
        } else {
            assert!(
                valid,
                "{name} must match the frozen {kind} schema: {:?}",
                violations(&validator, &instance)
            );
        }
        checked += 1;
    }
    assert!(
        checked >= 6,
        "the frozen vector set must stay populated (found {checked})"
    );
}

/// The library fact source serializes exactly into the frozen result
/// envelope: family const, echoed projectPath, sorted package rows, and
/// the declared dependency as a real fact.
#[test]
fn list_packages_output_matches_the_frozen_result_schema() {
    let base = unique_dir("envelope");
    let project = synthetic_vpm_project(&base);
    let backend = offline_backend(&base);

    let packages = backend.list_packages(&project).expect("listing succeeds");

    let rows: Vec<Value> = packages
        .iter()
        .map(|package| serde_json::to_value(package).expect("InstalledPackageV1 serializes"))
        .collect();
    let result = json!({
        "schemaVersion": "vua.packages-installed/v0.1",
        "projectPath": project.root.to_string_lossy(),
        "packages": rows,
    });
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "packages.listInstalled",
        "result": result,
    });

    let validator = result_validator();
    assert!(
        validator.is_valid(&envelope),
        "the library projection must match the frozen result schema: {:?}",
        violations(&validator, &envelope)
    );

    assert_eq!(
        envelope["result"]["schemaVersion"],
        "vua.packages-installed/v0.1"
    );
    let listed = envelope["result"]["packages"].as_array().unwrap();
    assert_eq!(listed.len(), 2, "both locked packages are listed");
    assert_eq!(listed[0]["packageId"], "com.demo.avatar-kit");
    assert_eq!(listed[0]["version"], "1.2.3");
    assert_eq!(
        listed[0]["dependencies"],
        json!(["com.demo.core-lib"]),
        "the declared dependency travels as a fact"
    );
    assert_eq!(listed[1]["packageId"], "com.demo.core-lib");
    assert_eq!(listed[1]["dependencies"], json!([]));
    for row in listed {
        assert!(
            row.get("displayName").is_none(),
            "the P1 word list carries NO displayName field — never invented"
        );
    }

    fs::remove_dir_all(&base).ok();
}

/// A registered project with an empty lock lists an EMPTY package array —
/// the honest empty state, never padded.
#[test]
fn empty_lock_lists_an_honest_empty_package_array() {
    let base = unique_dir("empty");
    let root = base.join("FreshProject");
    fs::create_dir_all(root.join("Packages")).unwrap();
    fs::create_dir_all(root.join("ProjectSettings")).unwrap();
    fs::write(
        root.join("ProjectSettings").join("ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1\n",
    )
    .unwrap();
    fs::write(
        root.join("Packages").join("vpm-manifest.json"),
        r#"{"dependencies":{},"locked":{}}"#,
    )
    .unwrap();
    let backend = offline_backend(&base);

    let packages = backend
        .list_packages(&ProjectRef {
            id: "packages-query-empty".to_owned(),
            root: root.clone(),
        })
        .expect("listing succeeds");

    assert!(packages.is_empty(), "an empty lock is an honest empty list");
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "packages.listInstalled",
        "result": {
            "schemaVersion": "vua.packages-installed/v0.1",
            "projectPath": root.to_string_lossy(),
            "packages": packages,
        },
    });
    assert!(
        result_validator().is_valid(&envelope),
        "the empty result must still match the frozen schema"
    );

    fs::remove_dir_all(&base).ok();
}

/// A broken project answers the typed load failure — a real error, never
/// a fabricated empty list (honesty discipline 2).
#[test]
fn broken_project_maps_to_the_typed_load_failure() {
    let base = unique_dir("broken");
    let root = base.join("BrokenProject");
    fs::create_dir_all(root.join("Packages")).unwrap();
    // A vpm-manifest that is not valid JSON: the backend must refuse, not
    // answer an empty projection.
    fs::write(root.join("Packages").join("vpm-manifest.json"), "{not json").unwrap();
    let backend = offline_backend(&base);

    let error = backend
        .list_packages(&ProjectRef {
            id: "packages-query-broken".to_owned(),
            root: root.clone(),
        })
        .expect_err("a broken manifest is a typed failure");

    assert_eq!(error.code, "vua.vpm.project_load_failed");
    fs::remove_dir_all(&base).ok();
}
