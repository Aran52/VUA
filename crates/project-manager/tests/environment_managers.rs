//! B6 tests: editor classification, ALCOM/VCC project-manager capability
//! detection, and the versioned snapshot contract. Everything
//! runs against synthetic directory trees, so no test depends on this
//! machine's real installs; real-machine evidence flows through the
//! explicitly ignored manual test (ORC-TST-006). Moved from the core with
//! the module (proposal 004): the VCC settings candidates are passed in
//! explicitly so the resolution-order invariant stays single-sourced in
//! the core `EnvironmentRoots`.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use vua_orchestrator::{
    classify_version_string, env_managers_codes, project_tree_fingerprint, EditorClass,
    FindingSeverity, FixedClock, ManagerPresence, PRODUCTION_TARGET,
};
use vua_project_manager::{
    collect_environment_managers_snapshot, EnvironmentManagersSnapshotV01, ManagerRoots,
    ProjectAssociation,
};

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-env-managers-{label}-{nanos}"))
}

fn cleanup(base: &Path) {
    if base.exists() {
        fs::remove_dir_all(base).unwrap();
    }
}

fn synthetic_managers(base: &Path) -> ManagerRoots {
    ManagerRoots {
        alcom_settings_candidates: vec![base.join("alcom/setting.json")],
    }
}

fn synthetic_vcc_candidates(base: &Path) -> Vec<PathBuf> {
    vec![base.join("vcc/settings.json")]
}

fn synthetic_editors(base: &Path) -> Vec<PathBuf> {
    vec![base.join("editors")]
}

fn probe(base: &Path) -> EnvironmentManagersSnapshotV01 {
    collect_environment_managers_snapshot(
        &synthetic_vcc_candidates(base),
        &synthetic_managers(base),
        &synthetic_editors(base),
        &FixedClock::new(&["2026-09-04T02:00:00.000Z"]),
    )
}

fn install_editor_dir(base: &Path, version: &str) {
    fs::create_dir_all(base.join("editors").join(version).join("Editor")).unwrap();
}

fn install_vpm_project(base: &Path, name: &str, editor_version: &str) -> String {
    let project = base.join("projects").join(name);
    fs::create_dir_all(project.join("Packages")).unwrap();
    fs::create_dir_all(project.join("ProjectSettings")).unwrap();
    fs::write(project.join("Packages").join("vpm-manifest.json"), "{}").unwrap();
    fs::write(
        project.join("ProjectSettings").join("ProjectVersion.txt"),
        format!(
            "m_EditorVersion: {editor_version}\nm_EditorVersionWithEdition: {editor_version} (f1)\n"
        ),
    )
    .unwrap();
    project.to_string_lossy().into_owned()
}

// --- editor classification table (E-COMP support matrix) ---

#[test]
fn env_managers_001_editor_classification_follows_the_support_matrix() {
    let cases: [(&str, Option<EditorClass>, bool); 7] = [
        (
            PRODUCTION_TARGET,
            Some(EditorClass::ProductionTarget),
            false,
        ),
        ("2019.4.31f1", Some(EditorClass::MigrationSource), false),
        ("2022.3.6f1", Some(EditorClass::MigrationSource), false),
        ("2022.3.22f1c1", Some(EditorClass::OtherUnityVersion), true),
        ("6000.0.23f1", Some(EditorClass::OtherUnityVersion), false),
        ("2022.3.22t1", Some(EditorClass::TuanjieFamily), false),
        ("garbage", None, false),
    ];
    for (version, expected_class, china) in cases {
        let classification = classify_version_string(version);
        assert_eq!(
            classification.map(|(class, _)| class),
            expected_class,
            "{version}"
        );
        if let Some(parsed) = vua_orchestrator::parse_editor_version(version) {
            assert_eq!(parsed.china_suffix.is_some(), china, "{version}");
        }
    }

    // Guidance codes are stable and specific: the China build of the
    // target version earns the distribution diagnosis, not the generic
    // off-target one; the production target never carries a code path
    // that suggests replacement.
    let (_, target_code) = classify_version_string(PRODUCTION_TARGET).expect("target must parse");
    assert_eq!(
        target_code,
        vua_orchestrator::editor_target_codes::EDITOR_PRODUCTION_TARGET
    );
    let (_, china_code) = classify_version_string("2022.3.22f1c1").expect("china build must parse");
    assert_eq!(
        china_code,
        vua_orchestrator::editor_target_codes::EDITOR_CHINA_DISTRIBUTION
    );
    let (_, off_code) = classify_version_string("6000.0.23f1").expect("off target must parse");
    assert_eq!(
        off_code,
        vua_orchestrator::editor_target_codes::EDITOR_OFF_TARGET
    );
    let (_, tuanjie_code) =
        classify_version_string("2022.3.22t1").expect("tuanjie build must parse");
    assert_eq!(
        tuanjie_code,
        vua_orchestrator::editor_target_codes::EDITOR_TUANJIE_UNSUPPORTED
    );
}

// --- editor root probing ---

#[test]
fn env_managers_002_editor_root_probe_classifies_and_stays_deterministic() {
    let base = unique_dir("editors");
    for version in [
        "2022.3.22f1",
        "2019.4.31f1",
        "2022.3.6f1",
        "2022.3.22f1c1",
        "2022.3.22t1",
        "6000.0.23f1",
    ] {
        install_editor_dir(&base, version);
    }
    // Not editors: a directory without an Editor child and a plain file.
    fs::create_dir_all(base.join("editors").join("not-an-editor")).unwrap();
    fs::write(base.join("editors").join("2023.1.1f1"), "file").unwrap();

    let snapshot = probe(&base);
    assert_eq!(snapshot.editors.len(), 6, "garbage entries are skipped");
    let find = |version: &str| {
        snapshot
            .editors
            .iter()
            .find(|finding| finding.version == version)
            .unwrap_or_else(|| panic!("{version} must be listed"))
    };
    assert_eq!(
        find("2022.3.22f1").classification,
        EditorClass::ProductionTarget
    );
    assert_eq!(
        find("2019.4.31f1").classification,
        EditorClass::MigrationSource
    );
    assert_eq!(
        find("2022.3.6f1").classification,
        EditorClass::MigrationSource
    );
    let china = find("2022.3.22f1c1");
    assert_eq!(china.classification, EditorClass::OtherUnityVersion);
    assert!(china.china_distribution);
    assert_eq!(
        china.guidance_code,
        vua_orchestrator::editor_target_codes::EDITOR_CHINA_DISTRIBUTION
    );
    assert_eq!(
        find("2022.3.22t1").classification,
        EditorClass::TuanjieFamily
    );
    assert_eq!(
        find("6000.0.23f1").classification,
        EditorClass::OtherUnityVersion
    );

    // Deterministic order: production target first, then classes and
    // versions in stable sequence regardless of OS read_dir order.
    assert_eq!(snapshot.editors[0].version, PRODUCTION_TARGET);
    let again = probe(&base);
    assert_eq!(again.editors, snapshot.editors);
    cleanup(&base);
}

// --- VCC userProjects (current format) ---

#[test]
fn env_managers_003_vcc_user_projects_are_discovered_and_classified() {
    let base = unique_dir("vcc-user-projects");
    let production = install_vpm_project(&base, "prod-av", PRODUCTION_TARGET);
    let migration = install_vpm_project(&base, "mig-av", "2019.4.31f1");
    let not_a_project = base.join("projects").join("not-a-project");
    fs::create_dir_all(&not_a_project).unwrap();
    let missing = base.join("projects").join("missing");
    let settings_path = base.join("vcc/settings.json");
    fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    fs::write(
        &settings_path,
        serde_json::json!({
            "userProjects": [production, migration, not_a_project.to_string_lossy(), missing.to_string_lossy()],
            "localProjectFolders": []
        })
        .to_string(),
    )
    .unwrap();

    let snapshot = probe(&base);
    assert_eq!(snapshot.vcc.presence, ManagerPresence::Found);
    assert_eq!(snapshot.vcc.projects_source, Some("userProjects"));
    assert_eq!(snapshot.vcc.error_code, None);

    let projects: Vec<_> = snapshot
        .projects
        .iter()
        .map(|finding| (finding.path.as_str(), finding.unity_classification))
        .collect();
    assert_eq!(
        projects.len(),
        2,
        "missing and marker-less entries are not projects"
    );
    // Projects are sorted by path for deterministic snapshots regardless
    // of userProjects order.
    assert_eq!(
        projects[0].0,
        Path::new(&migration).to_string_lossy().to_string()
    );
    assert_eq!(projects[0].1, Some(EditorClass::MigrationSource));
    assert_eq!(
        projects[1].0,
        Path::new(&production).to_string_lossy().to_string()
    );
    assert_eq!(
        snapshot.projects[1].unity_classification,
        Some(EditorClass::ProductionTarget)
    );
    assert!(snapshot
        .projects
        .iter()
        .all(|finding| finding.associations == vec![ProjectAssociation::VccRegistered]));

    let codes: Vec<_> = snapshot
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&env_managers_codes::PROJECT_PATH_MISSING));
    assert!(codes.contains(&env_managers_codes::PROJECT_MARKERS_INCOMPLETE));

    // Stale or broken registered entries are warnings, never errors:
    // the snapshot itself succeeded.
    assert!(snapshot
        .diagnostics
        .iter()
        .all(|diagnostic| diagnostic.severity != FindingSeverity::Error));
    cleanup(&base);
}

// --- VCC legacy localProjectFolders format ---

#[test]
fn env_managers_004_vcc_legacy_folder_format_scans_registered_folders() {
    let base = unique_dir("vcc-legacy");
    let folder = base.join("registered-folder");
    fs::create_dir_all(folder.join("RandomDir")).unwrap();
    // A legacy folder registers project *directories*.
    let legacy_project = folder.join("legacy-av");
    fs::create_dir_all(legacy_project.join("Packages")).unwrap();
    fs::create_dir_all(legacy_project.join("ProjectSettings")).unwrap();
    fs::write(
        legacy_project.join("Packages").join("vpm-manifest.json"),
        "{}",
    )
    .unwrap();
    fs::write(
        legacy_project
            .join("ProjectSettings")
            .join("ProjectVersion.txt"),
        format!("m_EditorVersion: {PRODUCTION_TARGET}\n"),
    )
    .unwrap();

    let settings_path = base.join("vcc/settings.json");
    fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    fs::write(
        &settings_path,
        serde_json::json!({ "localProjectFolders": [folder.to_string_lossy()] }).to_string(),
    )
    .unwrap();

    let snapshot = probe(&base);
    assert_eq!(snapshot.vcc.presence, ManagerPresence::Found);
    assert_eq!(snapshot.vcc.projects_source, Some("localProjectFolders"));
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(
        snapshot.projects[0].unity_classification,
        Some(EditorClass::ProductionTarget)
    );
    // RandomDir has no VPM markers: a warning diagnostic, not a finding.
    assert!(snapshot
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == env_managers_codes::PROJECT_MARKERS_INCOMPLETE));
    cleanup(&base);
}

// --- deterministic missing and failed observations ---

#[test]
fn env_managers_005_missing_managers_are_findings_not_errors() {
    let base = unique_dir("vcc-missing");
    install_editor_dir(&base, PRODUCTION_TARGET);
    let snapshot = probe(&base);
    assert_eq!(snapshot.vcc.presence, ManagerPresence::NotFound);
    assert_eq!(snapshot.alcom.presence, ManagerPresence::NotFound);
    assert!(snapshot.projects.is_empty());
    assert!(
        snapshot.diagnostics.is_empty(),
        "a missing component is a normal finding: {:?}",
        snapshot.diagnostics
    );
    cleanup(&base);
}

#[test]
fn env_managers_006_unparseable_vcc_settings_is_a_read_failed_diagnostic() {
    let base = unique_dir("vcc-broken");
    let settings_path = base.join("vcc/settings.json");
    fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    fs::write(&settings_path, "{ not json").unwrap();
    install_vpm_project(&base, "orphan", PRODUCTION_TARGET);

    let snapshot = probe(&base);
    assert_eq!(snapshot.vcc.presence, ManagerPresence::ReadFailed);
    assert_eq!(
        snapshot.vcc.error_code,
        Some(env_managers_codes::VCC_SETTINGS_SCHEMA_UNEXPECTED)
    );
    assert!(snapshot.projects.is_empty());
    assert!(snapshot
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == FindingSeverity::Error));
    cleanup(&base);
}

// --- read-only discipline ---

#[test]
fn env_managers_007_probe_leaves_the_observed_tree_byte_identical() {
    let base = unique_dir("read-only");
    install_editor_dir(&base, PRODUCTION_TARGET);
    install_vpm_project(&base, "watched", PRODUCTION_TARGET);
    let settings_path = base.join("vcc/settings.json");
    fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    let project = base.join("projects").join("watched");
    fs::write(
        &settings_path,
        serde_json::json!({ "userProjects": [project.to_string_lossy()] }).to_string(),
    )
    .unwrap();

    let scopes = ["editors", "vcc", "projects"];
    let before = project_tree_fingerprint(&base, &scopes)
        .unwrap()
        .expect("tree exists");
    let snapshot = probe(&base);
    let after = project_tree_fingerprint(&base, &scopes)
        .unwrap()
        .expect("tree exists");
    assert_eq!(before, after, "the probe must not write a single byte");
    assert_eq!(snapshot.projects.len(), 1);
    cleanup(&base);
}

// --- v0.3: ALCOM project association and dual-manager merge ---

#[test]
fn env_managers_010_alcom_user_projects_are_discovered_and_associated() {
    let base = unique_dir("alcom-projects");
    install_vpm_project(&base, "alcom-av", PRODUCTION_TARGET);
    let settings_path = base.join("alcom/setting.json");
    fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    fs::write(
        &settings_path,
        serde_json::json!({
            "userProjects": [base.join("projects").join("alcom-av").to_string_lossy()]
        })
        .to_string(),
    )
    .unwrap();

    let snapshot = probe(&base);
    assert_eq!(snapshot.alcom.presence, ManagerPresence::Found);
    assert_eq!(snapshot.alcom.user_projects.len(), 1);
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(
        snapshot.projects[0].associations,
        vec![ProjectAssociation::AlcomRegistered]
    );
    assert_eq!(
        snapshot.projects[0].unity_version.as_deref(),
        Some(PRODUCTION_TARGET)
    );
    cleanup(&base);
}

#[test]
fn env_managers_011_a_path_registered_by_both_managers_merges_into_one_finding() {
    let base = unique_dir("dual-managed");
    install_vpm_project(&base, "shared-av", PRODUCTION_TARGET);
    let project_path = base
        .join("projects")
        .join("shared-av")
        .to_string_lossy()
        .into_owned();

    let vcc_settings = base.join("vcc/settings.json");
    fs::create_dir_all(vcc_settings.parent().unwrap()).unwrap();
    fs::write(
        &vcc_settings,
        serde_json::json!({ "userProjects": [project_path] }).to_string(),
    )
    .unwrap();
    let alcom_settings = base.join("alcom/setting.json");
    fs::create_dir_all(alcom_settings.parent().unwrap()).unwrap();
    fs::write(
        &alcom_settings,
        serde_json::json!({ "userProjects": [project_path] }).to_string(),
    )
    .unwrap();

    let snapshot = probe(&base);
    assert_eq!(snapshot.projects.len(), 1, "one folder, one finding");
    assert_eq!(
        snapshot.projects[0].associations,
        vec![
            ProjectAssociation::VccRegistered,
            ProjectAssociation::AlcomRegistered
        ]
    );
    cleanup(&base);
}

#[test]
fn env_managers_012_unrecognized_alcom_settings_warn_without_blocking_presence() {
    let base = unique_dir("alcom-unknown");
    let settings_path = base.join("alcom/setting.json");
    fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    fs::write(
        &settings_path,
        serde_json::json!({ "unrelated": true }).to_string(),
    )
    .unwrap();

    let snapshot = probe(&base);
    assert_eq!(snapshot.alcom.presence, ManagerPresence::Found);
    assert!(snapshot.alcom.user_projects.is_empty());
    assert!(snapshot.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == env_managers_codes::ALCOM_PROJECTS_NOT_RECOGNIZED
            && diagnostic.severity == FindingSeverity::Warning
    }));
    cleanup(&base);
}

// --- versioned contract ---

#[test]
fn env_managers_008_synthetic_snapshot_conforms_to_the_versioned_schema() {
    let base = unique_dir("schema-synthetic");
    install_editor_dir(&base, PRODUCTION_TARGET);
    install_vpm_project(&base, "schema-av", PRODUCTION_TARGET);
    let settings_path = base.join("vcc/settings.json");
    fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    fs::write(
        &settings_path,
        serde_json::json!({
            "userProjects": [base.join("projects").join("schema-av").to_string_lossy()]
        })
        .to_string(),
    )
    .unwrap();

    let snapshot = probe(&base);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/environment-managers/v0.1/snapshot.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let payload: serde_json::Value = serde_json::to_value(&snapshot).unwrap();
    let errors: Vec<_> = validator.iter_errors(&payload).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    cleanup(&base);
}

#[test]
fn env_managers_009_schema_fixtures_conform_to_the_versioned_schema() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/environment-managers/v0.1/snapshot.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let fixture: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/environment-managers/v0.1/fixtures/snapshot.valid.json"))
            .unwrap(),
    )
    .unwrap();
    let errors: Vec<_> = validator.iter_errors(&fixture).collect();
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
#[ignore = "manual local evidence paths are supplied through VUA_ENV_MANAGERS_SNAPSHOTS"]
fn manual_local_snapshot_conforms_to_the_versioned_schema() {
    let paths = std::env::var_os("VUA_ENV_MANAGERS_SNAPSHOTS")
        .expect("VUA_ENV_MANAGERS_SNAPSHOTS is required");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("schemas/environment-managers/v0.1/snapshot.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for path in std::env::split_paths(&paths) {
        let fixture: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let errors: Vec<_> = validator.iter_errors(&fixture).collect();
        assert!(errors.is_empty(), "{}: {errors:#?}", path.display());
    }
}

/// BG-12 家族环境域成员（wt-6 反向审查批 2026-09-24）：settings 数组携带
/// 非字符串项时，可读字符串照常保留，被丢弃项以 VCC_SETTINGS_SCHEMA_
/// UNEXPECTED 警告如实申报——静默丢弃会让已注册项目从发现面消失且无迹
/// 可寻。发现照旧推进：幸存条目仍是注册事实。
#[test]
fn mixed_type_user_projects_keeps_strings_and_announces_the_dropped_entries() {
    let base = unique_dir("mixed-projects");
    let good = install_vpm_project(&base, "Good Project", "2022.3.22f1");
    let settings = base.join("vcc/settings.json");
    fs::create_dir_all(settings.parent().unwrap()).unwrap();
    let good_json = serde_json::to_string(&good).unwrap();
    fs::write(
        &settings,
        format!(r#"{{ "userProjects": [{good_json}, 42, null] }}"#),
    )
    .unwrap();

    let snapshot = probe(&base);
    assert_eq!(snapshot.vcc.presence, ManagerPresence::Found);
    assert_eq!(snapshot.vcc.projects_source, Some("userProjects"));
    assert_eq!(
        snapshot.vcc.user_projects,
        vec![good.clone()],
        "the readable string survives; nothing is invented"
    );

    let announced = snapshot.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == env_managers_codes::VCC_SETTINGS_SCHEMA_UNEXPECTED
            && diagnostic.severity == FindingSeverity::Warning
            && diagnostic.detail.contains("userProjects")
            && diagnostic.detail.contains("2 non-string entries")
    });
    assert!(
        announced,
        "the skipped entries must be announced: {:?}",
        snapshot.diagnostics
    );

    // Discovery proceeds with the surviving registration.
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(snapshot.projects[0].path, good);

    cleanup(&base);
}
