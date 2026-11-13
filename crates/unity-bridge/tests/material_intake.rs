use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tar::{Builder, Header};
use vua_orchestrator::{MaterialEntryMode, RiskDecisionChoice};
use vua_unity_bridge::{MaterialIntakeEngine, RiskDecisionV01};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "vua-b3-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn append(builder: &mut Builder<GzEncoder<fs::File>>, path: &str, bytes: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append_data(&mut header, path, bytes).unwrap();
}

fn unitypackage(path: &Path, assets: &[&str]) {
    let file = fs::File::create(path).unwrap();
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);
    for (index, asset) in assets.iter().enumerate() {
        append(
            &mut builder,
            &format!("guid{index}/pathname"),
            asset.as_bytes(),
        );
        append(&mut builder, &format!("guid{index}/asset"), b"fixture");
    }
    builder.into_inner().unwrap().finish().unwrap();
}

/// An already-provisioned target (ProjectSettings/ProjectVersion.txt
/// present): plans on this path carry NO provision step — the zero-change
/// law for provisioned projects.
fn provisioned_project(label: &str) -> PathBuf {
    let root = temp_dir(label).join("target");
    fs::create_dir_all(root.join("ProjectSettings")).unwrap();
    fs::write(
        root.join("ProjectSettings").join("ProjectVersion.txt"),
        "2022.3.22f1",
    )
    .unwrap();
    root
}

#[test]
fn b3_folder_batch_includes_every_unitypackage_and_binds_one_risk_decision() {
    let root = temp_dir("all-packages").join("Cute Outfit");
    fs::create_dir_all(root.join("variants")).unwrap();
    unitypackage(
        &root.join("base.unitypackage"),
        &["Assets/Outfit/base.prefab"],
    );
    unitypackage(
        &root.join("variants/extra.unitypackage"),
        &["Assets/Outfit/Editor/Setup.cs", "Assets/Outfit/plugin.dll"],
    );

    let engine = MaterialIntakeEngine;
    let inspection = engine.inspect_folder(&root, "corr-inspect").unwrap();
    assert_eq!(inspection.display_name, "Cute Outfit");
    assert_eq!(inspection.packages.len(), 2);
    assert_eq!(inspection.packages[0].relative_path, "base.unitypackage");
    assert_eq!(
        inspection.packages[1].relative_path,
        "variants/extra.unitypackage"
    );
    assert!(inspection.executable_risks.len() >= 3);

    let plan = engine
        .plan(
            MaterialEntryMode::LocalReusableVpm,
            "project-1",
            "sha256:project",
            inspection.clone(),
            &provisioned_project("plan-target"),
            "corr-plan",
        )
        .unwrap();
    assert!(plan.risk_decision_required);
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/amf-production/v0.2/material-plan.schema.json");
    let schema: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let plan_value = serde_json::to_value(&plan).unwrap();
    let errors = validator
        .iter_errors(&plan_value)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "{errors:?}");
    let missing = engine
        .confirm(
            &plan,
            &plan.plan_hash,
            RiskDecisionV01 {
                choice: RiskDecisionChoice::NotRequired,
                source_fingerprint: inspection.source_fingerprint.clone(),
                risk_fingerprint: inspection.risk_fingerprint.clone(),
                remember_for_session: false,
            },
            "2026-09-03T00:00:00Z",
            "corr-confirm",
        )
        .unwrap_err();
    assert_eq!(missing.code, "vua.material.risk_decision_required");

    let confirmation = engine
        .confirm(
            &plan,
            &plan.plan_hash,
            RiskDecisionV01 {
                choice: RiskDecisionChoice::SnapshotAndContinue,
                source_fingerprint: inspection.source_fingerprint.clone(),
                risk_fingerprint: inspection.risk_fingerprint.clone(),
                remember_for_session: true,
            },
            "2026-09-03T00:00:00Z",
            "corr-confirm",
        )
        .unwrap();
    assert_eq!(confirmation.plan.source.packages.len(), 2);
    assert!(confirmation.snapshot_scopes().contains(&"UserSettings"));
    fs::remove_dir_all(root.parent().unwrap()).ok();
}

#[test]
fn b3_source_or_inventory_drift_invalidates_confirmation() {
    let base = temp_dir("drift");
    let root = base.join("Asset Folder");
    fs::create_dir_all(&root).unwrap();
    unitypackage(&root.join("asset.unitypackage"), &["Assets/Asset.prefab"]);
    let engine = MaterialIntakeEngine;
    let inspected = engine.inspect_folder(&root, "corr").unwrap();

    unitypackage(
        &root.join("asset.unitypackage"),
        &["Assets/Asset.prefab", "Assets/Editor/New.cs"],
    );
    let error = engine
        .verify_source_unchanged(&root, &inspected, "corr")
        .unwrap_err();
    assert_eq!(error.code, "vua.material.source_drift");
    fs::remove_dir_all(base).ok();
}

#[test]
fn b3_direct_and_vpm_plans_have_distinct_mutation_paths() {
    let base = temp_dir("modes");
    let root = base.join("Asset");
    fs::create_dir_all(&root).unwrap();
    unitypackage(&root.join("asset.unitypackage"), &["Assets/Asset.prefab"]);
    let engine = MaterialIntakeEngine;
    let source = engine.inspect_folder(&root, "corr").unwrap();
    let target = provisioned_project("modes-target");
    let direct = engine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            "project",
            "project-fingerprint",
            source.clone(),
            &target,
            "corr",
        )
        .unwrap();
    let vpm = engine
        .plan(
            MaterialEntryMode::LocalReusableVpm,
            "project",
            "project-fingerprint",
            source,
            &target,
            "corr",
        )
        .unwrap();
    assert_ne!(direct.plan_hash, vpm.plan_hash);
    assert_eq!(
        direct
            .steps
            .iter()
            .filter(|step| step.mutates_target_project)
            .count(),
        1
    );
    assert_eq!(
        vpm.steps
            .iter()
            .filter(|step| step.mutates_target_project)
            .count(),
        1
    );
    fs::remove_dir_all(base).ok();
}

#[test]
fn b3_w25_unprovisioned_target_plans_provision_step_between_snapshot_and_import() {
    let base = temp_dir("w25-plan");
    let source_root = base.join("Asset");
    fs::create_dir_all(&source_root).unwrap();
    unitypackage(
        &source_root.join("asset.unitypackage"),
        &["Assets/Asset.prefab"],
    );
    let engine = MaterialIntakeEngine;
    let source = engine.inspect_folder(&source_root, "corr").unwrap();

    // EMPTY target: no ProjectSettings/ProjectVersion.txt anywhere below.
    let empty_target = base.join("empty-target");
    fs::create_dir_all(&empty_target).unwrap();

    let plan = engine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            "project",
            "sha256:empty-tree",
            source.clone(),
            &empty_target,
            "corr",
        )
        .unwrap();

    // The provision step is present, sits AFTER the snapshot and BEFORE the
    // first project mutation, and names the mutation honestly.
    let kinds: Vec<_> = plan.steps.iter().map(|step| step.kind).collect();
    let snapshot = kinds
        .iter()
        .position(|kind| *kind == vua_unity_bridge::MaterialIntakeStepKind::CreateSnapshot)
        .expect("snapshot step");
    let provision = kinds
        .iter()
        .position(|kind| *kind == vua_unity_bridge::MaterialIntakeStepKind::ProvisionProject)
        .expect("provision step planned for an unprovisioned target");
    let import = kinds
        .iter()
        .position(|kind| *kind == vua_unity_bridge::MaterialIntakeStepKind::ImportUnityPackages)
        .expect("import step");
    assert!(
        snapshot < provision && provision < import,
        "provision must sit between snapshot and first mutation, got {kinds:?}"
    );
    let provision_step = &plan.steps[provision];
    assert!(provision_step.mutates_target_project);
    assert!(provision_step.safe_boundary_after);

    // The SAME source against an already-provisioned target plans WITHOUT
    // the step and the two plans hash differently (the assembly.rs
    // conditional-plan law: conditional presence is plan content).
    let provisioned_target = provisioned_project("w25-plan-provisioned");
    let provisioned_plan = engine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            "project",
            "sha256:provisioned-tree",
            source,
            &provisioned_target,
            "corr",
        )
        .unwrap();
    assert!(
        !provisioned_plan
            .steps
            .iter()
            .any(|step| step.kind == vua_unity_bridge::MaterialIntakeStepKind::ProvisionProject),
        "a provisioned target must plan the unchanged v0.1 step set"
    );
    assert_ne!(plan.plan_hash, provisioned_plan.plan_hash);
    fs::remove_dir_all(base).ok();
}

#[test]
fn b3_w25_material_plan_v02_vectors_validate_against_the_frozen_schema() {
    let vectors_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/amf-production/v0.2/vectors/material-plan");
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/amf-production/v0.2/material-plan.schema.json");
    let schema: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    for name in [
        "material-plan.valid.unprovisioned.json",
        "material-plan.valid.provisioned.json",
    ] {
        let vector: serde_json::Value =
            serde_json::from_slice(&fs::read(vectors_dir.join(name)).unwrap()).unwrap();
        let errors = validator
            .iter_errors(&vector["instance"])
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{name}: {errors:?}");
    }

    let negative: serde_json::Value = serde_json::from_slice(
        &fs::read(vectors_dir.join("material-plan.invalid.unknown-step-kind.json")).unwrap(),
    )
    .unwrap();
    assert!(
        validator
            .iter_errors(&negative["instance"])
            .next()
            .is_some(),
        "an invented step kind must be INVALID by the frozen v0.2 schema"
    );
}
