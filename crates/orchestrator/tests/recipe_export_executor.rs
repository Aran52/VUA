//! Proposal 029 B-face loop-3 executor tests: the REAL on-disk
//! `OnDiskProjectDraftExporter` over a synthetic project matrix — every
//! observation the frozen word face names (declared VPM dependencies +
//! locked pins, the observed editor version, the VUA identity tri-state,
//! the project-name source fact) is exercised against temp-dir fake
//! projects. No machine-specific facts, no network, no Bridge, no user VCC
//! home. The honest-behavior law under test: observation failures ride as
//! FACTS (empty rows / null constraint + the environmentUnityVersion
//! marker), never as fabricated values and never as invented errors (the
//! face's error closed set reserves no code for on-disk findings — the
//! wire-level chain over the real route lives in
//! `crates/provider-host/tests/recipe_export_wire_v01.rs`).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use vua_orchestrator::{
    MissingDimensionV01, OnDiskProjectDraftExporter, ProjectDraftDocumentV01,
    ProjectDraftExportPort, VuaIdentityStatusV01,
};

const FIXED_INSTANT: &str = "2026-09-22T03:40:00.000Z";

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/recipe-export/v0.1")
}

fn read_json(relative: &str) -> Value {
    let bytes = std::fs::read(schema_dir().join(relative)).expect("schema must exist");
    serde_json::from_slice(&bytes).expect("schema must be valid JSON")
}

/// Validates a typed document against the REAL frozen result schema: the
/// route stamps the family const at envelope assembly, so the projection
/// under test is the document plus that const wrapped in the envelope shape.
fn assert_schema_valid(draft: &ProjectDraftDocumentV01) {
    let schema = read_json("result.schema.json");
    let validator = jsonschema::validator_for(&schema).expect("frozen schema must compile");
    let mut result = serde_json::to_value(draft).expect("document serializes");
    result["schemaVersion"] = json!("vua.recipe-export/v0.1");
    let envelope = json!({
        "schemaVersion": "0.1",
        "operation": "recipe.exportProjectDraft",
        "result": result,
    });
    let problems: Vec<String> = validator
        .iter_errors(&envelope)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        problems.is_empty(),
        "frozen schema must admit the draft: {problems:?}"
    );
}

fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-recipe-export-executor-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// Seeds the directory skeleton every fake project carries; the caller
/// layers the observation-specific files on top.
fn seed_skeleton(root: &Path, label: &str) -> PathBuf {
    let project = root.join(label);
    fs::create_dir_all(project.join("Assets")).expect("Assets dir");
    fs::create_dir_all(project.join("ProjectSettings")).expect("ProjectSettings dir");
    fs::create_dir_all(project.join("Packages")).expect("Packages dir");
    project
}

fn write_version(project: &Path, text: &str) {
    fs::write(
        project.join("ProjectSettings").join("ProjectVersion.txt"),
        text,
    )
    .expect("project version writes");
}

fn write_manifest(project: &Path, text: &str) {
    fs::write(project.join("Packages").join("vpm-manifest.json"), text)
        .expect("vpm manifest writes");
}

fn write_identity(project: &Path, text: &str) {
    fs::create_dir_all(project.join(".vua")).expect(".vua dir");
    fs::write(project.join(".vua").join("project.json"), text).expect("identity writes");
}

fn exporter() -> OnDiskProjectDraftExporter {
    OnDiskProjectDraftExporter::with_clock(Arc::new(vua_orchestrator::FixedClock::new(&[
        FIXED_INSTANT,
    ])))
}

fn missing_values(draft: &ProjectDraftDocumentV01) -> Vec<String> {
    draft
        .missing
        .iter()
        .map(|dimension| match dimension {
            MissingDimensionV01::Assets => "assets",
            MissingDimensionV01::Instances => "instances",
            MissingDimensionV01::Relations => "relations",
            MissingDimensionV01::WardrobeGroups => "wardrobeGroups",
            MissingDimensionV01::TargetAvatar => "targetAvatar",
            MissingDimensionV01::AssetRoles => "assetRoles",
            MissingDimensionV01::AssetLabels => "assetLabels",
            MissingDimensionV01::SourceRefs => "sourceRefs",
            MissingDimensionV01::TitleSemantics => "titleSemantics",
            MissingDimensionV01::EnvironmentUnityVersion => "environmentUnityVersion",
        })
        .map(str::to_owned)
        .collect()
}

const UUID_V7_PATTERN: &str =
    "^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$";

#[test]
fn the_loop_three_override_flips_the_declared_none_default() {
    // The implementation exists, so the capability accessor answers declared
    // (the F5 law: an adapter overrides exactly when it implements) — this
    // flip is what turns the served row available.
    let capabilities = exporter().export_capabilities();
    assert!(
        capabilities.export_project_draft,
        "the real executor declares the face"
    );
}

#[test]
fn a_normal_project_projects_the_full_frozen_word_face() {
    let root = unique_root("normal");
    let project = seed_skeleton(&root, "comfy-avatar-2026");
    write_version(&project, "m_EditorVersion: 2022.3.22f1\n");
    // Deliberately NON-ascending key order in the file: the ascending row
    // order is the exporter's frozen presentation fact, not the file's.
    write_manifest(
        &project,
        r#"{
            "dependencies": {
                "dev.example.coolshader": "^1.2.0",
                "com.vrchat.avatars": "3.7.x",
                "com.vrchat.base": "3.7.x"
            },
            "locked": {
                "com.vrchat.avatars": "3.7.12",
                "dev.example.coolshader": "1.2.7",
                "locked.only.transitive": "9.9.9"
            }
        }"#,
    );
    write_identity(
        &project,
        r#"{"identityVersion": 1, "markedAt": "2026-09-01T10:00:00.000Z", "note": null}"#,
    );

    let draft = exporter()
        .export_project_draft(project.to_string_lossy().as_ref())
        .expect("an on-disk observation is not an error of this face");
    assert_schema_valid(&draft);

    // Origin: the registered path echoed verbatim, the final component as
    // the name source fact, the VUA-native tri-state.
    assert_eq!(draft.origin.project_path, project.to_string_lossy());
    assert_eq!(
        draft.origin.project_name.as_deref(),
        Some("comfy-avatar-2026")
    );
    assert_eq!(
        draft.origin.vua_identity_status,
        VuaIdentityStatusV01::Present
    );

    // Environment: the observed version verbatim, no marker.
    assert_eq!(
        draft.environment.unity_version_constraint.as_deref(),
        Some("2022.3.22f1")
    );

    // Dependencies: the DECLARED set only (the locked-only transitive entry
    // produces NO row), packageId ascending, pins only where one exists.
    let ids: Vec<&str> = draft
        .dependencies
        .iter()
        .map(|row| row.package_id.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "com.vrchat.avatars",
            "com.vrchat.base",
            "dev.example.coolshader"
        ]
    );
    assert_eq!(draft.dependencies[0].version_constraint, "3.7.x");
    assert_eq!(
        draft.dependencies[0].locked_version.as_deref(),
        Some("3.7.12")
    );
    assert_eq!(
        draft.dependencies[1].locked_version,
        Option::<String>::None,
        "an unpinned declaration carries no locked pin"
    );
    assert_eq!(
        draft.dependencies[2].locked_version.as_deref(),
        Some("1.2.7")
    );

    // Missing: the nine constant dimensions only — the relation face is
    // never scanned (zero-bridge ruling) and the version was readable.
    assert_eq!(
        missing_values(&draft),
        [
            "assets",
            "instances",
            "relations",
            "wardrobeGroups",
            "targetAvatar",
            "assetRoles",
            "assetLabels",
            "sourceRefs",
            "titleSemantics"
        ],
    );
    assert_eq!(
        draft.exported_at, FIXED_INSTANT,
        "the injected clock stamps the scan"
    );
    // The draft instance identity: minted per call, uuid-v7-shaped, never a
    // recipeId.
    let id_validator = jsonschema::validator_for(&json!({ "pattern": UUID_V7_PATTERN }))
        .expect("pattern compiles");
    assert!(
        id_validator.is_valid(&json!(draft.draft_id)),
        "{}",
        draft.draft_id
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn manifest_absent_and_empty_declared_project_the_honest_empty_rows() {
    let root = unique_root("empty");
    // Absent manifest, zero declared, and locked-only manifest: three paths,
    // one honest answer — an EMPTY rows vec as a SUCCESS (never padded).
    let absent = seed_skeleton(&root, "no-manifest");
    write_version(&absent, "m_EditorVersion: 2022.3.22f1\n");
    let zero = seed_skeleton(&root, "zero-declared");
    write_version(&zero, "m_EditorVersion: 2022.3.22f1\n");
    write_manifest(&zero, r#"{"dependencies": {}}"#);
    let locked_only = seed_skeleton(&root, "locked-only");
    write_version(&locked_only, "m_EditorVersion: 2022.3.22f1\n");
    write_manifest(
        &locked_only,
        r#"{"locked": {"com.example.transitive": "1.0.0"}}"#,
    );

    for (label, project) in [
        ("absent", absent),
        ("zero", zero),
        ("locked-only", locked_only),
    ] {
        let draft = exporter()
            .export_project_draft(project.to_string_lossy().as_ref())
            .unwrap_or_else(|error| panic!("{label} must be a success fact: {}", error.code));
        assert!(draft.dependencies.is_empty(), "{label}: honest empty rows");
        assert_eq!(
            draft.environment.unity_version_constraint.as_deref(),
            Some("2022.3.22f1"),
            "{label}: the version was readable, no marker"
        );
        assert_eq!(
            missing_values(&draft).len(),
            9,
            "{label}: nine constants only"
        );
        assert_schema_valid(&draft);
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn corrupted_and_shape_broken_manifests_project_the_honest_empty_rows() {
    // A parse failure is an honest warning finding in the 013 aggregate and
    // carries NO code on this face: the projection is the empty rows vec —
    // never an invented package list, never an invented error. The REST of
    // the facts keep riding (a broken manifest does not blind the export).
    let root = unique_root("corrupt");
    let corrupted = seed_skeleton(&root, "corrupted-json");
    write_version(&corrupted, "m_EditorVersion: 2022.3.22f1\n");
    write_manifest(&corrupted, r#"{"dependencies": { "com.example" "#);
    let not_object = seed_skeleton(&root, "json-array");
    write_version(&not_object, "m_EditorVersion: 2022.3.22f1\n");
    write_manifest(&not_object, r#"[1, 2, 3]"#);

    for (label, project) in [("corrupted", corrupted), ("not-object", not_object)] {
        let draft = exporter()
            .export_project_draft(project.to_string_lossy().as_ref())
            .unwrap_or_else(|error| panic!("{label} must ride as a success fact: {}", error.code));
        assert!(
            draft.dependencies.is_empty(),
            "{label}: no rows derivable, none invented"
        );
        assert_eq!(
            draft.environment.unity_version_constraint.as_deref(),
            Some("2022.3.22f1"),
            "{label}: the rest of the facts keep riding"
        );
        assert_schema_valid(&draft);
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn non_string_version_entries_are_skipped_never_guessed() {
    // The aggregate's manifest document-read rule: string-valued entries
    // only — a numeric version value is not a declared constraint and never
    // becomes one.
    let root = unique_root("non-string");
    let project = seed_skeleton(&root, "mixed-types");
    write_version(&project, "m_EditorVersion: 2022.3.22f1\n");
    write_manifest(
        &project,
        r#"{"dependencies": {"com.example.numeric": 3, "com.example.string": "1.0.0"}}"#,
    );
    let draft = exporter()
        .export_project_draft(project.to_string_lossy().as_ref())
        .expect("observation succeeds");
    let ids: Vec<&str> = draft
        .dependencies
        .iter()
        .map(|row| row.package_id.as_str())
        .collect();
    assert_eq!(
        ids,
        ["com.example.string"],
        "only the string-valued declaration rides"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn incomplete_and_absent_version_lines_carry_null_constraint_and_the_marker() {
    // The frozen bidirectional iff: a null constraint MUST carry the
    // environmentUnityVersion marker (and nothing else may).
    let root = unique_root("version");
    let incomplete = seed_skeleton(&root, "incomplete-version");
    write_version(&incomplete, "m_EditorVersion: 2022.3\n");
    write_manifest(&incomplete, r#"{"dependencies": {}}"#);
    let absent_file = seed_skeleton(&root, "no-version-file");
    write_manifest(&absent_file, r#"{"dependencies": {}}"#);

    for (label, project) in [("incomplete", incomplete), ("absent", absent_file)] {
        let draft = exporter()
            .export_project_draft(project.to_string_lossy().as_ref())
            .unwrap_or_else(|error| panic!("{label} must ride as a success fact: {}", error.code));
        assert_eq!(
            draft.environment.unity_version_constraint, None,
            "{label}: unreadable version is the honest null"
        );
        assert_eq!(
            missing_values(&draft).len(),
            10,
            "{label}: the conditional tenth marker joined"
        );
        assert!(missing_values(&draft).contains(&"environmentUnityVersion".to_owned()));
        assert_schema_valid(&draft);
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn the_identity_tri_state_follows_the_aggregate_verdict_boundaries() {
    let root = unique_root("identity");
    let absent = seed_skeleton(&root, "non-vua");
    write_version(&absent, "m_EditorVersion: 2022.3.22f1\n");
    let garbage = seed_skeleton(&root, "garbage-identity");
    write_version(&garbage, "m_EditorVersion: 2022.3.22f1\n");
    write_identity(&garbage, "not json at all");
    let unknown_version = seed_skeleton(&root, "unknown-identity-version");
    write_version(&unknown_version, "m_EditorVersion: 2022.3.22f1\n");
    write_identity(
        &unknown_version,
        r#"{"identityVersion": 99, "markedAt": "x"}"#,
    );
    let missing_member = seed_skeleton(&root, "identity-missing-marked-at");
    write_version(&missing_member, "m_EditorVersion: 2022.3.22f1\n");
    write_identity(&missing_member, r#"{"identityVersion": 1}"#);

    let cases = [
        ("absent", absent, VuaIdentityStatusV01::Absent),
        ("garbage", garbage, VuaIdentityStatusV01::Unreadable),
        ("unknown", unknown_version, VuaIdentityStatusV01::Unreadable),
        (
            "missing-member",
            missing_member,
            VuaIdentityStatusV01::Unreadable,
        ),
    ];
    for (label, project, expected) in cases {
        let draft = exporter()
            .export_project_draft(project.to_string_lossy().as_ref())
            .unwrap_or_else(|error| panic!("{label} must ride as a success fact: {}", error.code));
        assert_eq!(draft.origin.vua_identity_status, expected, "{label}");
        // An absent identity is NOT a gate (proposal 029 open item 2): the
        // draft rides the fact either way.
        assert_eq!(
            draft.environment.unity_version_constraint.as_deref(),
            Some("2022.3.22f1")
        );
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_path_without_a_final_component_has_no_name_fact() {
    // The honest null: a root-shaped path spells no name component — the
    // schema's "null when no name fact is readable" (the aggregate's
    // whole-path echo is a list-visibility display concern, not a name
    // fact).
    let root = unique_root("no-name");
    let draft = exporter()
        .export_project_draft("C:/")
        .expect("observation succeeds");
    assert_eq!(draft.origin.project_name, None);
    assert_eq!(
        draft.origin.project_path, "C:/",
        "the path itself echoes verbatim"
    );
    assert_eq!(
        draft.origin.vua_identity_status,
        VuaIdentityStatusV01::Absent
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn repeated_exports_pin_determinism_except_the_per_call_draft_identity() {
    // The determinism pin: identical on-disk facts + identical clock =>
    // identical documents except the per-call draft instance identity (the
    // frozen per-export mint).
    let root = unique_root("determinism");
    let project = seed_skeleton(&root, "deterministic-source");
    write_version(&project, "m_EditorVersion: 2022.3.22f1\n");
    write_manifest(
        &project,
        r#"{"dependencies": {"zeta.last": "0.1.0", "alpha.first": "2.0.0"}, "locked": {"alpha.first": "2.0.1"}}"#,
    );
    let path = project.to_string_lossy().into_owned();
    let first = exporter()
        .export_project_draft(&path)
        .expect("first export");
    let second = exporter()
        .export_project_draft(&path)
        .expect("second export");

    let mut first_without_id = first.clone();
    first_without_id.draft_id = String::new();
    let mut second_without_id = second.clone();
    second_without_id.draft_id = String::new();
    assert_eq!(
        first_without_id, second_without_id,
        "the fact projection is deterministic"
    );
    assert_ne!(
        first.draft_id, second.draft_id,
        "each export mints its own draft identity"
    );
    let ids: Vec<&str> = first
        .dependencies
        .iter()
        .map(|row| row.package_id.as_str())
        .collect();
    assert_eq!(
        ids,
        ["alpha.first", "zeta.last"],
        "packageId ascending both calls"
    );
    fs::remove_dir_all(&root).ok();
}
