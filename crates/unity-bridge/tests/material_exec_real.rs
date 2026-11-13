//! B3 real-Unity local matrix (M3 gate). These tests launch the actual
//! Unity 2022.3.22f1 in batchmode against synthetic projects built in the
//! temp directory, so they are ignored by default and never run in CI:
//!
//!   VUA_UNITY_EXECUTABLE="C:\Program Files\Unity\Hub\Editor\2022.3.22f1\Editor\Unity.exe" \
//!   cargo test -p vua-orchestrator --test material_exec_real -- --ignored --nocapture
//!
//! The synthetic target project embeds the repository Bridge package plus a
//! minimal stub for the Modular Avatar assembly reference — the stub exists
//! only inside the temp project, never in the repository.

use flate2::write::GzEncoder;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tar::{Builder, Header};

use vua_orchestrator::{
    BuildRecordStore, FileSystemSnapshotStore, MaterialEntryMode, ProjectRef, RiskDecisionChoice,
    UnityBridge, VpmBackend,
};
use vua_unity_bridge::{
    MaterialCancelToken, MaterialExecutionStatus, MaterialExecutor, MaterialIntakeConfirmationV01,
    MaterialIntakeEngine, MaterialIntakeStepKind, RiskDecisionV01, RollbackOutcome,
    UnityBatchBridge,
};

const UNITY_VERSION_LINE: &str =
    "m_EditorVersion: 2022.3.22f1\nm_EditorVersionWithEdition: 2022.3.22f1\n";

/// Type-complete Modular Avatar stub: the Bridge's install_outfit and
/// create_toggle reference these members, so the staging/target compile
/// needs them even though B3 never invokes those operations.
const STUB_COMPONENTS_CS: &str = r#"using System.Collections.Generic;
using UnityEngine;

namespace nadena.dev.modular_avatar.core
{
    public class AvatarObjectReference
    {
        public AvatarObjectReference() { }
        public AvatarObjectReference(GameObject target) { }
    }

    public enum PortableControlType { Menu, Toggle, Submenu, Action }

    public class PortableControl
    {
        public PortableControlType Type;
        public int Value;
    }

    public class ToggledObject
    {
        public AvatarObjectReference Object;
        public bool Active;
    }

    public class ModularAvatarMergeArmature : MonoBehaviour
    {
        public AvatarObjectReference mergeTarget;
        public void InferPrefixSuffix() { }
    }

    public class ModularAvatarMenuInstaller : MonoBehaviour { }

    public class ModularAvatarMenuItem : MonoBehaviour
    {
        public string label;
        public PortableControl PortableControl = new PortableControl();
        public bool isDefault;
    }

    public class ModularAvatarObjectToggle : MonoBehaviour
    {
        public List<ToggledObject> Objects;
    }
}
"#;
// --- scaffolding ---

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-mexec-real-{label}-{nanos}"))
}

fn copy_dir_recursive(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Builds a minimal but compilable Unity project embedding the repository
/// Bridge package. The Modular Avatar reference is satisfied by an empty
/// stub package — the Bridge's B3 operations never touch MA types.
fn build_target_project(label: &str) -> (PathBuf, ProjectRef) {
    let root = temp_dir(label);
    let bridge_package_src =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../unity/Packages/com.ph-r.vua");
    fs::create_dir_all(root.join("Assets")).unwrap();
    fs::create_dir_all(root.join("ProjectSettings")).unwrap();
    fs::create_dir_all(root.join("Packages")).unwrap();
    fs::write(
        root.join("ProjectSettings/ProjectVersion.txt"),
        UNITY_VERSION_LINE,
    )
    .unwrap();
    fs::write(
        root.join("Packages/manifest.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "dependencies": { "com.unity.textmeshpro": "3.0.6" }
        }))
        .unwrap(),
    )
    .unwrap();
    // A real AMF production project carries the VRChat SDK (VRC expression
    // assets inside material packages need its types to load) plus the real
    // Modular Avatar + NDMF stack (real assets ship NDMF plugin Editor
    // scripts that the minimal stub cannot satisfy). Everything is copied
    // from an existing local VCC project — discovered through the standard
    // VCC settings file — instead of a registry round-trip.
    let vcc_project = find_vcc_project_with_sdk();
    let sdk_seeded = vcc_project
        .as_deref()
        .map(|project| seed_vrc_sdk(project, &root))
        .unwrap_or(false);
    let real_ma = vcc_project
        .as_deref()
        .map(|project| copy_real_avatar_stack(project, &root.join("Packages")))
        .unwrap_or(false);
    println!(
        "sdk seeded from local VCC project: {sdk_seeded}; real Modular Avatar stack: {real_ma}"
    );
    // Copy only the runtime pieces of the Bridge package; the NUnit test
    // assembly has no framework to resolve against in this bare project.
    copy_dir_recursive(
        &bridge_package_src.join("Editor"),
        &root.join("Packages/com.ph-r.vua/Editor"),
    )
    .unwrap();
    fs::copy(
        bridge_package_src.join("package.json"),
        root.join("Packages/com.ph-r.vua/package.json"),
    )
    .unwrap();
    // Stub package satisfying the Bridge asmdef reference — fallback only:
    // with the real MA stack present (copied above) the stub would collide
    // with the real nadena.dev.modular-avatar.core assembly.
    if !real_ma {
        write_ma_stub(&root.join("Packages"));
    }
    (
        root.clone(),
        ProjectRef {
            id: "project".into(),
            root,
        },
    )
}

/// Writes a minimal valid `.unitypackage` (tar.gz with guid folders,
/// `asset`, `asset.meta`, `pathname` entries per Unity convention).
fn write_unitypackage(path: &Path, entries: &[(&str, &str)]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut tar = Builder::new(GzEncoder::new(file, flate2::Compression::default()));
    for (guid, pathname) in entries {
        let mut meta = Header::new_gnu();
        let meta_bytes = format!("fileFormatVersion: 2\nguid: {guid}\n").into_bytes();
        meta.set_size(meta_bytes.len() as u64);
        meta.set_cksum();
        tar.append_data(
            &mut meta,
            format!("{guid}/asset.meta"),
            meta_bytes.as_slice(),
        )
        .unwrap();
        let mut body = Header::new_gnu();
        body.set_size(b"synthetic asset payload".len() as u64);
        body.set_cksum();
        tar.append_data(
            &mut body,
            format!("{guid}/asset"),
            b"synthetic asset payload".as_slice(),
        )
        .unwrap();
        let mut path_header = Header::new_gnu();
        let path_bytes = pathname.as_bytes();
        path_header.set_size(path_bytes.len() as u64);
        path_header.set_cksum();
        tar.append_data(&mut path_header, format!("{guid}/pathname"), path_bytes)
            .unwrap();
    }
    tar.finish().unwrap();
}

/// Discovers a local VCC-managed project that carries the VRChat SDK, so the
/// bare test project can be seeded the way a real user project looks.
fn find_vcc_project_with_sdk() -> Option<PathBuf> {
    let vcc_settings = std::env::var("LOCALAPPDATA")
        .ok()
        .map(|local| PathBuf::from(local).join("VRChatCreatorCompanion/settings.json"))?;
    let settings = fs::read_to_string(&vcc_settings).ok()?;
    let value = serde_json::from_str::<Value>(&settings).ok()?;
    for candidate in value.get("userProjects")?.as_array()? {
        if let Some(path) = candidate.as_str() {
            if Path::new(path).join("Packages/com.vrchat.avatars").is_dir() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

/// Copies the VRChat SDK packages from the discovered VCC project and merges
/// its UPM manifest dependencies, so the copied SDK compiles in the bare
/// project. Returns true when any SDK package was seeded.
fn seed_vrc_sdk(vcc_project: &Path, root: &Path) -> bool {
    let mut seeded = false;
    for package in ["com.vrchat.avatars", "com.vrchat.base"] {
        let from = vcc_project.join("Packages").join(package);
        if from.is_dir() {
            copy_dir_recursive(&from, &root.join("Packages").join(package)).unwrap();
            seeded = true;
        }
    }
    if seeded {
        if let Ok(candidate_manifest) =
            fs::read_to_string(vcc_project.join("Packages/manifest.json"))
        {
            if let Ok(manifest) = serde_json::from_str::<Value>(&candidate_manifest) {
                if let Some(deps) = manifest
                    .get("dependencies")
                    .and_then(Value::as_object)
                    .cloned()
                {
                    let manifest_path = root.join("Packages/manifest.json");
                    let mut merged =
                        serde_json::json!({ "dependencies": { "com.unity.textmeshpro": "3.0.6" } });
                    if let Ok(existing) = fs::read_to_string(&manifest_path) {
                        if let Ok(value) = serde_json::from_str::<Value>(&existing) {
                            merged = value;
                        }
                    }
                    if let Some(target) = merged
                        .get_mut("dependencies")
                        .and_then(Value::as_object_mut)
                    {
                        for (key, dep) in deps {
                            target.insert(key, dep);
                        }
                    }
                    fs::write(
                        &manifest_path,
                        serde_json::to_string_pretty(&merged).unwrap(),
                    )
                    .unwrap();
                }
            }
        }
    }
    seeded
}

/// Copies the developer's real Modular Avatar + NDMF packages when the
/// discovered VCC project carries them. Real assets commonly ship NDMF
/// plugin Editor scripts (ExportsPluginAttribute/Plugin<>/BuildContext);
/// the minimal compile stub cannot satisfy those, and chasing every type a
/// real asset may reference is not viable. With the real stack the bare
/// project compiles exactly like a real user project. Returns true when the
/// real MA package was copied (the stub is then skipped).
fn copy_real_avatar_stack(vcc_project: &Path, packages_dir: &Path) -> bool {
    let ma_src = vcc_project.join("Packages/nadena.dev.modular-avatar");
    if !ma_src.is_dir() {
        return false;
    }
    let _ = copy_dir_recursive(&ma_src, &packages_dir.join("nadena.dev.modular-avatar"));
    let ndmf_src = vcc_project.join("Packages/nadena.dev.ndmf");
    if ndmf_src.is_dir() {
        let _ = copy_dir_recursive(&ndmf_src, &packages_dir.join("nadena.dev.ndmf"));
    }
    true
}

/// Writes the fallback Modular Avatar stub (used only when no real MA stack
/// is available locally) and returns the stub's source, shared with the
/// staging template path.
fn write_ma_stub(packages_dir: &Path) {
    let stub = packages_dir.join("nadena.dev.modular-avatar.core");
    fs::create_dir_all(&stub).unwrap();
    fs::write(
        stub.join("package.json"),
        r#"{ "name": "nadena.dev.modular-avatar.core", "version": "0.0.0-stub" }"#,
    )
    .unwrap();
    fs::write(
        stub.join("nadena.dev.modular-avatar.core.asmdef"),
        r#"{ "name": "nadena.dev.modular-avatar.core", "rootNamespace": "" }"#,
    )
    .unwrap();
    fs::write(stub.join("StubComponents.cs"), STUB_COMPONENTS_CS).unwrap();
}

fn unity_executable() -> PathBuf {
    PathBuf::from(
        std::env::var("VUA_UNITY_EXECUTABLE")
            .expect("VUA_UNITY_EXECUTABLE must point at Unity.exe (2022.3.22f1)"),
    )
}

fn fingerprint(bridge: &dyn UnityBridge, project: &ProjectRef) -> String {
    let result = bridge
        .execute(
            project,
            &vua_orchestrator::UnityCommand {
                schema_version: 1,
                command_id: format!("harness-inspect-{}", std::process::id()),
                operation: vua_orchestrator::UnityOperation::InspectProject,
                project_id: project.id.clone(),
                dry_run: true,
                expected_project_fingerprint: None,
                payload: Default::default(),
            },
        )
        .expect("real inspect must succeed");
    assert_eq!(result.status, vua_orchestrator::ResultStatus::Succeeded);
    result
        .data
        .get("projectFingerprint")
        .and_then(Value::as_str)
        .expect("inspect returns the project fingerprint")
        .to_owned()
}

use serde_json::Value;

fn confirmation_for(
    plan: &vua_unity_bridge::MaterialIntakePlanV01,
) -> MaterialIntakeConfirmationV01 {
    MaterialIntakeConfirmationV01 {
        plan: plan.clone(),
        risk_decision: RiskDecisionV01 {
            choice: RiskDecisionChoice::Continue,
            source_fingerprint: plan.source.source_fingerprint.clone(),
            risk_fingerprint: plan.source.risk_fingerprint.clone(),
            remember_for_session: false,
        },
        confirmed_at: "2026-09-04T00:00:00Z".into(),
        correlation_id: "real-corr".into(),
    }
}

// --- the real vertical slice ---

#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 (VUA_UNITY_EXECUTABLE) against a real source folder (VUA_REAL_SOURCE_FOLDER)"]
fn m3_real_direct_vertical_slice_succeeds_and_records() {
    let unity = unity_executable();
    let source = PathBuf::from(
        std::env::var("VUA_REAL_SOURCE_FOLDER")
            .expect("VUA_REAL_SOURCE_FOLDER must hold the .unitypackage files"),
    );
    let (project_root, project) = build_target_project("direct");

    let bridge: Arc<dyn UnityBridge> = Arc::new(UnityBatchBridge::new(unity));
    let fingerprint = fingerprint(bridge.as_ref(), &project);
    println!("real project fingerprint: {fingerprint}");

    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, "real-corr")
        .expect("source inspects");
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            fingerprint,
            inspection.clone(),
            &project.root,
            "real-corr",
        )
        .expect("plan builds");
    let confirmation = confirmation_for(&plan);

    let started = Instant::now();
    let executor = MaterialExecutor::new(
        bridge,
        FileSystemSnapshotStore,
        Arc::new(NoVpm),
        BuildRecordStore::new(project_root.join(".vua/records")),
        Arc::new(vua_orchestrator::SystemClock),
        temp_dir("direct-temp"),
        "2022.3.22f1",
        vua_unity_bridge::LocalPackageIdentityStore::new(project_root.join(".vua/identities.json")),
    );
    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &project_root.join(".vua/artifacts"),
        &vua_unity_bridge::MaterialCancelToken::new(),
    );
    let elapsed = started.elapsed();
    println!("executor wall time: {elapsed:?}");

    assert_eq!(
        report.status,
        vua_unity_bridge::MaterialExecutionStatus::Succeeded,
        "report: {report:?}"
    );
    assert_eq!(
        report.rollback,
        vua_unity_bridge::RollbackOutcome::NotNeeded
    );
    assert!(report
        .completed_steps
        .contains(&MaterialIntakeStepKind::WriteBuildRecord));

    let record = vua_orchestrator::BuildRecordStore::new(project_root.join(".vua/records"))
        .read(&report.build_record_id.expect("record id"))
        .expect("receipt published");
    let validation = record.validation.expect("validation evidence");
    assert!(
        validation.unity_validated,
        "a real Unity validated the assets"
    );
    let import_job = record
        .bridge_jobs
        .iter()
        .find(|job| job.operation.contains("MaterializeExtractedPackage"))
        .expect("import evidence");
    assert!(
        !import_job.changed_paths.is_empty(),
        "a real package imports a real asset tree: {:?}",
        import_job.changed_paths.len()
    );
    println!("imported: {:?}", import_job.changed_paths.len());

    // The snapshot stays on disk as the recovery point of record.
    assert!(project_root.join(".vua/snapshots").exists());

    // The source folder comes from VUA_REAL_SOURCE_FOLDER — a developer's
    // real asset directory. It is NEVER deleted; only the temp project is.
    if project_root.exists() {
        fs::remove_dir_all(&project_root).unwrap();
    }
}

/// A stale project fingerprint must be rejected by the real Bridge and the
/// verified snapshot must be restored — the typed path a concurrent change
/// would take in production.
#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 (VUA_UNITY_EXECUTABLE)"]
fn m3_real_stale_fingerprint_is_rejected_and_restored() {
    let unity = unity_executable();
    let (project_root, project) = build_target_project("reject");
    let source = temp_dir("reject-source");
    fs::create_dir_all(&source).unwrap();
    write_unitypackage(
        &source.join("outfit-pack.unitypackage"),
        &[(
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "Assets/VuaSynthetic/SampleOutfit.txt",
        )],
    );

    let bridge: Arc<dyn UnityBridge> = Arc::new(UnityBatchBridge::new(unity));
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, "real-corr")
        .expect("source inspects");
    // Deliberately stale: the fingerprint cannot match the real project.
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            "stale-fingerprint",
            inspection,
            &project.root,
            "real-corr",
        )
        .expect("plan builds");
    let confirmation = confirmation_for(&plan);

    let executor = MaterialExecutor::new(
        bridge,
        FileSystemSnapshotStore,
        Arc::new(NoVpm),
        BuildRecordStore::new(project_root.join(".vua/records")),
        Arc::new(vua_orchestrator::SystemClock),
        temp_dir("reject-temp"),
        "2022.3.22f1",
        vua_unity_bridge::LocalPackageIdentityStore::new(project_root.join(".vua/identities.json")),
    );
    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &project_root.join(".vua/artifacts"),
        &vua_unity_bridge::MaterialCancelToken::new(),
    );

    assert_eq!(
        report.status,
        vua_unity_bridge::MaterialExecutionStatus::Failed,
        "the real Bridge rejects the stale fingerprint"
    );
    assert_eq!(
        report.error_code.as_deref(),
        Some("vua.material.bridge_rejected")
    );
    assert_eq!(report.rollback, vua_unity_bridge::RollbackOutcome::Restored);

    let record = vua_orchestrator::BuildRecordStore::new(project_root.join(".vua/records"))
        .read(&report.build_record_id.expect("record id"))
        .expect("failed runs are recorded");
    assert_eq!(record.status, vua_orchestrator::BuildRecordStatus::Failed);
    assert!(
        record.bridge_jobs[0]
            .status
            .eq_ignore_ascii_case("rejected"),
        "the real Bridge rejected the stale fingerprint: {:?}",
        record.bridge_jobs[0]
    );

    if project_root.exists() {
        fs::remove_dir_all(&project_root).unwrap();
    }
    if source.exists() {
        fs::remove_dir_all(&source).unwrap();
    }
}

/// Placeholder backend: the direct path never touches VPM installation.
struct NoVpm;

impl VpmBackend for NoVpm {
    fn name(&self) -> &'static str {
        "none"
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

    fn create_project(
        &self,
        _parent: &Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, vua_orchestrator::AppErrorV1> {
        panic!("the direct path must not reach the vpm backend")
    }

    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[vua_orchestrator::PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, vua_orchestrator::AppErrorV1> {
        panic!("the direct path must not reach the vpm backend")
    }

    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[vua_orchestrator::PackageRequestV1],
        _confirmed_digest: &str,
    ) -> Result<serde_json::Value, vua_orchestrator::AppErrorV1> {
        panic!("the direct path must not reach the vpm backend")
    }
}

// --- local_reusable: staging → local VPM package → vrc-get install ---

/// The full local_reusable slice on a real machine: the batch is imported
/// into a token-bound staging project built from the bundled template, the
/// Bridge produces the local-reusable package layout, the package is
/// published deterministically and installed into the target through the
/// REAL vrc-get lib backend (isolated environment root — the user's VCC
/// settings are never touched), and the installed package is validated by
/// real AssetDatabase.
#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 and the real vrc-get lib (VUA_UNITY_EXECUTABLE + VUA_REAL_SOURCE_FOLDER)"]
fn m3_real_local_reusable_vertical_slice() {
    let unity = unity_executable();
    let source = PathBuf::from(
        std::env::var("VUA_REAL_SOURCE_FOLDER")
            .expect("VUA_REAL_SOURCE_FOLDER must hold the .unitypackage files"),
    );
    let correlation = "real-corr";
    let vcc_project = find_vcc_project_with_sdk();

    // Target project: Bridge + MA stub + SDK seeded, plus the VPM manifest
    // layer vrc-get manages.
    let (project_root, project) = build_target_project("vpm-target");
    fs::write(
        project_root.join("Packages/vpm-manifest.json"),
        r#"{ "dependencies": {} }"#,
    )
    .unwrap();

    // Staging template override: the bundled template is text-only, while
    // the staging Unity launches need the Bridge package plus the same real
    // dependency stack as the target (VRC SDK + real Modular Avatar/NDMF,
    // stub only as fallback) — real assets imported into staging compile
    // their NDMF plugin Editor scripts exactly like in the target. The
    // harness authors that scaffold once; the executor unpacks it via
    // with_staging_template_override.
    let base = temp_dir("vpm-staging-base");
    let staging_template = base.join("staging-template");
    let bridge_package_src =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../unity/Packages/com.ph-r.vua");
    copy_dir_recursive(
        &bridge_package_src.join("Editor"),
        &staging_template.join("Packages/com.ph-r.vua/Editor"),
    )
    .unwrap();
    fs::copy(
        bridge_package_src.join("package.json"),
        staging_template.join("Packages/com.ph-r.vua/package.json"),
    )
    .unwrap();
    fs::write(
        staging_template.join("Packages/manifest.json"),
        r#"{ "dependencies": {} }"#,
    )
    .unwrap();
    let staging_sdk_seeded = vcc_project
        .as_deref()
        .map(|project| seed_vrc_sdk(project, &staging_template))
        .unwrap_or(false);
    let staging_real_ma = vcc_project
        .as_deref()
        .map(|project| copy_real_avatar_stack(project, &staging_template.join("Packages")))
        .unwrap_or(false);
    println!(
        "staging scaffold: sdk {staging_sdk_seeded}, real Modular Avatar stack: {staging_real_ma}"
    );
    if !staging_real_ma {
        write_ma_stub(&staging_template.join("Packages"));
    }
    fs::create_dir_all(staging_template.join("Assets")).unwrap();
    fs::create_dir_all(staging_template.join("ProjectSettings")).unwrap();
    fs::write(
        staging_template.join("ProjectSettings/ProjectVersion.txt"),
        UNITY_VERSION_LINE,
    )
    .unwrap();

    let bridge: Arc<dyn UnityBridge> = Arc::new(UnityBatchBridge::new(unity));
    let fingerprint = fingerprint(bridge.as_ref(), &project);
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, correlation)
        .expect("source inspects");
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::LocalReusableVpm,
            project.id.clone(),
            fingerprint,
            inspection,
            &project.root,
            correlation,
        )
        .expect("plan builds");
    let confirmation = confirmation_for(&plan);

    let vpm_environment = temp_dir("vpm-env");
    let vpm_backend: Arc<dyn VpmBackend> = Arc::new(
        vua_project_manager::VrcGetLibBackend::with_environment_root(base.join("vpm-env"), false)
            .expect("vrc-get lib backend initializes"),
    );
    // Resolve the machine identity first (persisted on disk; the executor's
    // own store instance re-reads the same file).
    let identity = vua_unity_bridge::LocalPackageIdentityStore::new(base.join("identities.json"))
        .resolve(&source, "outfit")
        .expect("identity resolves");
    let identity_store =
        vua_unity_bridge::LocalPackageIdentityStore::new(base.join("identities.json"));
    let executor = MaterialExecutor::new(
        bridge,
        FileSystemSnapshotStore,
        vpm_backend,
        BuildRecordStore::new(project_root.join(".vua/records")),
        Arc::new(vua_orchestrator::SystemClock),
        base.join("temp"),
        "2022.3.22f1",
        identity_store,
    )
    .with_staging_template_override(&staging_template);

    let started = Instant::now();
    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &project_root.join(".vua/artifacts"),
        &vua_unity_bridge::MaterialCancelToken::new(),
    );
    let elapsed = started.elapsed();
    println!("executor wall time (4+ Unity launches + vrc-get install): {elapsed:?}");

    assert_eq!(
        report.status,
        vua_unity_bridge::MaterialExecutionStatus::Succeeded,
        "report: {report:?}"
    );
    for expected in [
        MaterialIntakeStepKind::ImportUnityPackages,
        MaterialIntakeStepKind::CreateLocalVpmPackage,
        MaterialIntakeStepKind::PreviewVpmInstall,
        MaterialIntakeStepKind::ApplyVpmInstall,
        MaterialIntakeStepKind::ValidateMinimumStructure,
        MaterialIntakeStepKind::WriteBuildRecord,
    ] {
        assert!(
            report.completed_steps.contains(&expected),
            "missing {expected:?}"
        );
    }

    // The staging project is destroyed — no leftovers.
    let staging_expected =
        vua_unity_bridge::staging_root(&base.join("temp"), correlation).expect("valid session id");
    assert!(
        !staging_expected.exists(),
        "staging leftovers poison later runs"
    );

    // The identity machine id owns the installed package folder in the
    // target project.
    let installed = project_root.join("Packages").join(&identity.package_id);
    assert!(
        installed.join("package.json").is_file(),
        "installed package.json must exist"
    );
    assert!(
        installed.join("Runtime").is_dir(),
        "the Editor/Runtime layout must survive install"
    );

    // The target's VPM manifest records the install.
    let manifest = fs::read_to_string(project_root.join("Packages/vpm-manifest.json")).unwrap();
    assert!(
        manifest.contains(&identity.package_id),
        "vpm-manifest must record the install"
    );

    // The receipt carries the local VPM evidence.
    let record = vua_orchestrator::BuildRecordStore::new(project_root.join(".vua/records"))
        .read(&report.build_record_id.expect("record id"))
        .expect("receipt published");
    let local_vpm = record.local_vpm.expect("local vpm evidence");
    assert_eq!(local_vpm.package_id, identity.package_id);
    assert_eq!(local_vpm.installed_version, "0.1.0");
    let validation = record.validation.expect("validation evidence");
    assert!(validation.unity_validated);

    // Cleanup: the source folder is the developer's real asset directory —
    // never deleted. Only local temp scaffolding goes away.
    let _ = vpm_environment;
    let _ = fs::remove_dir_all(&base);
    let _ = fs::remove_dir_all(&project_root);
}

// --- P1 remaining cells: replay (S8), cancel (S2), drift (S3), timeout (S4),
// rollback failure (S7). These use a synthetic self-contained package so a
// single cell runs in one Unity launch; the S1/S5/S6 cells above already
// cover the real-asset path end to end.

fn synthetic_source(label: &str) -> PathBuf {
    let source = temp_dir(label);
    fs::create_dir_all(&source).unwrap();
    write_unitypackage(
        &source.join("synthetic-pack.unitypackage"),
        &[(
            "cccccccccccccccccccccccccccccccc",
            "Assets/VuaSynthetic/Cell.txt",
        )],
    );
    source
}

fn direct_executor(
    bridge: Arc<dyn UnityBridge>,
    project_root: &Path,
    temp_label: &str,
) -> MaterialExecutor {
    MaterialExecutor::new(
        bridge,
        FileSystemSnapshotStore,
        Arc::new(NoVpm),
        BuildRecordStore::new(project_root.join(".vua/records")),
        Arc::new(vua_orchestrator::SystemClock),
        temp_dir(temp_label),
        "2022.3.22f1",
        vua_unity_bridge::LocalPackageIdentityStore::new(project_root.join(".vua/identities.json")),
    )
}

fn bridge_request_count(project_root: &Path) -> usize {
    fs::read_dir(project_root.join(".vua/bridge"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().ends_with(".request.json"))
                .count()
        })
        .unwrap_or(0)
}

fn remove_snapshot_manifests(project_root: &Path) {
    let snapshots = project_root.join(".vua/snapshots");
    if let Ok(entries) = fs::read_dir(&snapshots) {
        for entry in entries.filter_map(Result::ok) {
            let _ = fs::remove_file(entry.path().join("manifest.json"));
        }
    }
}

/// S8: a succeeded receipt for the same plan identity must replay as success
/// without touching Unity again — the receipt is the replay guard.
#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 (VUA_UNITY_EXECUTABLE)"]
fn m3_real_direct_replay_of_succeeded_receipt_skips_unity() {
    let unity = unity_executable();
    let source = synthetic_source("replay-source");
    let (project_root, project) = build_target_project("replay");
    let bridge: Arc<dyn UnityBridge> = Arc::new(UnityBatchBridge::new(unity));
    let fingerprint = fingerprint(bridge.as_ref(), &project);
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, "real-corr")
        .expect("source inspects");
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            fingerprint,
            inspection,
            &project.root,
            "real-corr",
        )
        .expect("plan builds");
    let confirmation = confirmation_for(&plan);
    let executor = direct_executor(bridge.clone(), &project_root, "replay-temp");

    let first = executor.execute(
        &confirmation,
        &source,
        &project,
        &project_root.join(".vua/artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(
        first.status,
        MaterialExecutionStatus::Succeeded,
        "first run: {first:?}"
    );
    assert!(!first.replayed, "the first run must not be a replay");
    let requests_after_first = bridge_request_count(&project_root);
    assert!(
        requests_after_first > 0,
        "the first run must have talked to Unity"
    );

    let started = Instant::now();
    let second = executor.execute(
        &confirmation,
        &source,
        &project,
        &project_root.join(".vua/artifacts"),
        &MaterialCancelToken::new(),
    );
    println!("replay wall time: {:?}", started.elapsed());
    assert_eq!(second.status, MaterialExecutionStatus::Succeeded);
    assert!(
        second.replayed,
        "a succeeded receipt must replay: {second:?}"
    );
    assert_eq!(second.completed_steps, Vec::new());
    assert_eq!(second.build_record_id, first.build_record_id);
    assert_eq!(
        bridge_request_count(&project_root),
        requests_after_first,
        "the replay must not touch Unity"
    );

    let _ = fs::remove_dir_all(&project_root);
    let _ = fs::remove_dir_all(&source);
}

/// S2: cancellation observed at a step boundary reports Cancelled with the
/// steps completed so far and still publishes the receipt.
#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 (VUA_UNITY_EXECUTABLE); cancels mid-run"]
fn m3_real_direct_cancel_at_step_boundary_records_facts() {
    let unity = unity_executable();
    let source = synthetic_source("cancel-source");
    let (project_root, project) = build_target_project("cancel");
    let bridge: Arc<dyn UnityBridge> = Arc::new(UnityBatchBridge::new(unity));
    let fingerprint = fingerprint(bridge.as_ref(), &project);
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, "real-corr")
        .expect("source inspects");
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            fingerprint,
            inspection,
            &project.root,
            "real-corr",
        )
        .expect("plan builds");
    let confirmation = confirmation_for(&plan);
    let executor = direct_executor(bridge, &project_root, "cancel-temp");

    let token = MaterialCancelToken::new();
    let cancel_handle = token.clone();
    std::thread::spawn(move || {
        // The snapshot completes in seconds; Unity's first launch takes
        // 30s+. Cancelling at 2s lands the request at a mid-run boundary.
        std::thread::sleep(Duration::from_secs(2));
        cancel_handle.cancel();
    });
    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &project_root.join(".vua/artifacts"),
        &token,
    );
    println!(
        "cancelled run completed steps: {:?}",
        report.completed_steps
    );
    assert_eq!(
        report.status,
        MaterialExecutionStatus::Cancelled,
        "report: {report:?}"
    );
    assert_eq!(
        report.error_code.as_deref(),
        Some("vua.material.cancelled"),
        "a cancelled report carries the cancelled code"
    );
    assert!(
        report.build_record_id.is_some(),
        "cancelled runs still get a receipt"
    );
    let record = vua_orchestrator::BuildRecordStore::new(project_root.join(".vua/records"))
        .read(report.build_record_id.as_deref().expect("record id"))
        .expect("receipt published");
    assert_eq!(
        record.status,
        vua_orchestrator::BuildRecordStatus::Cancelled
    );

    if project_root.exists() {
        fs::remove_dir_all(&project_root).unwrap();
    }
    if source.exists() {
        fs::remove_dir_all(&source).unwrap();
    }
}

/// S3: recomputed source digests that no longer match the plan fail the run
/// with `vua.material.source_drift` before the first write — no Unity, no
/// snapshot, no mutation.
#[test]
#[ignore = "manual: real filesystem drift probe against the real intake pipeline"]
fn m3_real_direct_source_drift_fails_before_first_write() {
    let unity = unity_executable();
    let source = synthetic_source("drift-source");
    let (project_root, project) = build_target_project("drift");
    let bridge: Arc<dyn UnityBridge> = Arc::new(UnityBatchBridge::new(unity));
    let fingerprint = fingerprint(bridge.as_ref(), &project);
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, "real-corr")
        .expect("source inspects");
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            fingerprint,
            inspection,
            &project.root,
            "real-corr",
        )
        .expect("plan builds");
    let confirmation = confirmation_for(&plan);
    let executor = direct_executor(bridge, &project_root, "drift-temp");

    // Drift the source after confirm, before execute.
    let pack = source.join("synthetic-pack.unitypackage");
    let mut bytes = fs::read(&pack).unwrap();
    bytes.extend_from_slice(b"drift");
    fs::write(&pack, &bytes).unwrap();

    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &project_root.join(".vua/artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(
        report.status,
        MaterialExecutionStatus::Failed,
        "report: {report:?}"
    );
    assert_eq!(
        report.error_code.as_deref(),
        Some("vua.material.source_drift")
    );
    assert_eq!(
        report.rollback,
        RollbackOutcome::NotNeeded,
        "drift fails before the first write"
    );
    assert!(
        !project_root.join(".vua/snapshots").exists(),
        "no snapshot may exist when drift is detected before the first write"
    );

    if project_root.exists() {
        fs::remove_dir_all(&project_root).unwrap();
    }
    if source.exists() {
        fs::remove_dir_all(&source).unwrap();
    }
}

/// S4: a 1s bridge timeout budget cannot survive a real Unity launch, so the
/// first mutating step fails with `vua.material.bridge_timeout` and the
/// verified snapshot is restored.
#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 (VUA_UNITY_EXECUTABLE) under a 1s budget"]
fn m3_real_direct_bridge_timeout_budget_is_enforced() {
    let unity = unity_executable();
    let source = synthetic_source("timeout-source");
    let (project_root, project) = build_target_project("timeout");
    let bridge: Arc<dyn UnityBridge> = Arc::new(UnityBatchBridge::new(unity.clone()));
    let fingerprint = fingerprint(bridge.as_ref(), &project);
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, "real-corr")
        .expect("source inspects");
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            fingerprint,
            inspection,
            &project.root,
            "real-corr",
        )
        .expect("plan builds");
    let confirmation = confirmation_for(&plan);
    let tight_bridge: Arc<dyn UnityBridge> =
        Arc::new(UnityBatchBridge::new(unity.clone()).with_timeout(Duration::from_secs(1)));
    let executor = direct_executor(tight_bridge, &project_root, "timeout-temp");

    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &project_root.join(".vua/artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(
        report.status,
        MaterialExecutionStatus::Failed,
        "report: {report:?}"
    );
    assert!(report
        .error_code
        .as_deref()
        .unwrap_or("")
        .starts_with("vua.material.bridge_timeout"));
    assert_eq!(
        report.rollback,
        RollbackOutcome::Restored,
        "a post-snapshot timeout restores the verified snapshot"
    );

    if project_root.exists() {
        fs::remove_dir_all(&project_root).unwrap();
    }
    if source.exists() {
        fs::remove_dir_all(&source).unwrap();
    }
}

/// S7: a restore that itself fails is recorded as `RollbackOutcome::Failed`
/// with a `rollback_failed` receipt — the worst outcome still gets a receipt
/// and is never hidden behind a clean-looking failure.
#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 (VUA_UNITY_EXECUTABLE); injects snapshot loss mid-run"]
fn m3_real_direct_rollback_failure_is_recorded_not_hidden() {
    let unity = unity_executable();
    let source = synthetic_source("rbfail-source");
    let (project_root, project) = build_target_project("rbfail");
    let bridge: Arc<dyn UnityBridge> = Arc::new(UnityBatchBridge::new(unity));
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, "real-corr")
        .expect("source inspects");
    // Deliberately stale fingerprint: the Bridge rejects, the executor then
    // attempts the restore — whose manifest a background thread removes.
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::DirectUnityPackage,
            project.id.clone(),
            "stale-fingerprint",
            inspection,
            &project.root,
            "real-corr",
        )
        .expect("plan builds");
    let confirmation = confirmation_for(&plan);
    let executor = direct_executor(bridge, &project_root, "rbfail-temp");

    let injector_root = project_root.clone();
    std::thread::spawn(move || {
        // The snapshot is created before Unity's first launch (seconds);
        // the Bridge rejection lands after the Unity round-trip (30s+).
        std::thread::sleep(Duration::from_secs(15));
        remove_snapshot_manifests(&injector_root);
    });
    let report = executor.execute(
        &confirmation,
        &source,
        &project,
        &project_root.join(".vua/artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(
        report.status,
        MaterialExecutionStatus::Failed,
        "report: {report:?}"
    );
    assert_eq!(report.rollback, RollbackOutcome::Failed);
    assert!(report
        .error_code
        .as_deref()
        .unwrap_or("")
        .contains("vua.material.rollback_failed"));
    let record = vua_orchestrator::BuildRecordStore::new(project_root.join(".vua/records"))
        .read(report.build_record_id.as_deref().expect("record id"))
        .expect("the worst outcome still gets a receipt");
    assert_eq!(record.status, vua_orchestrator::BuildRecordStatus::Failed);

    if project_root.exists() {
        fs::remove_dir_all(&project_root).unwrap();
    }
    if source.exists() {
        fs::remove_dir_all(&source).unwrap();
    }
}

// --- P2 remaining cells (local_reusable): cancel (S2), drift (S3), timeout
// (S4), Bridge rejection (S5), rollback success (S6), rollback failure (S7),
// replay (S8). All reuse the real self-contained subset source and the real
// vrc-get lib backend exactly like the P2xS1 slice above.

struct P2Harness {
    project_root: PathBuf,
    project: ProjectRef,
    source: PathBuf,
    base: PathBuf,
    confirmation: MaterialIntakeConfirmationV01,
    executor: MaterialExecutor,
}

fn p2_harness(label: &str) -> P2Harness {
    let unity = unity_executable();
    let source = PathBuf::from(
        std::env::var("VUA_REAL_SOURCE_FOLDER")
            .expect("VUA_REAL_SOURCE_FOLDER must hold the .unitypackage files"),
    );
    let correlation = format!("real-corr-{}", label);
    let (project_root, project) = build_target_project(label);
    fs::write(
        project_root.join("Packages/vpm-manifest.json"),
        r#"{ "dependencies": {} }"#,
    )
    .unwrap();

    let base = temp_dir(&format!("{label}-base"));
    let staging_template = base.join("staging-template");
    let bridge_package_src =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../unity/Packages/com.ph-r.vua");
    copy_dir_recursive(
        &bridge_package_src.join("Editor"),
        &staging_template.join("Packages/com.ph-r.vua/Editor"),
    )
    .unwrap();
    fs::copy(
        bridge_package_src.join("package.json"),
        staging_template.join("Packages/com.ph-r.vua/package.json"),
    )
    .unwrap();
    fs::write(
        staging_template.join("Packages/manifest.json"),
        r#"{ "dependencies": {} }"#,
    )
    .unwrap();
    let vcc_project = find_vcc_project_with_sdk();
    // The staging scaffold needs the same real dependency stack as the
    // target: the VRC SDK (its embedded Managed dlls carry
    // System.Collections.Immutable etc. that NDMF compiles against) plus the
    // real Modular Avatar + NDMF packages.
    let staging_sdk_seeded = vcc_project
        .as_deref()
        .map(|project| seed_vrc_sdk(project, &staging_template))
        .unwrap_or(false);
    let staging_real_ma = vcc_project
        .as_deref()
        .map(|project| copy_real_avatar_stack(project, &staging_template.join("Packages")))
        .unwrap_or(false);
    println!(
        "staging scaffold: sdk {staging_sdk_seeded}, real Modular Avatar stack: {staging_real_ma}"
    );
    if !staging_real_ma {
        write_ma_stub(&staging_template.join("Packages"));
    }
    fs::create_dir_all(staging_template.join("Assets")).unwrap();
    fs::create_dir_all(staging_template.join("ProjectSettings")).unwrap();
    fs::write(
        staging_template.join("ProjectSettings/ProjectVersion.txt"),
        UNITY_VERSION_LINE,
    )
    .unwrap();

    let bridge: Arc<dyn UnityBridge> = Arc::new(UnityBatchBridge::new(unity));
    let fingerprint = fingerprint(bridge.as_ref(), &project);
    let inspection = MaterialIntakeEngine
        .inspect_folder(&source, &correlation)
        .expect("source inspects");
    let plan = MaterialIntakeEngine
        .plan(
            MaterialEntryMode::LocalReusableVpm,
            project.id.clone(),
            fingerprint,
            inspection,
            &project.root,
            &correlation,
        )
        .expect("plan builds");
    let confirmation = confirmation_for(&plan);

    let vpm_backend: Arc<dyn VpmBackend> = Arc::new(
        vua_project_manager::VrcGetLibBackend::with_environment_root(base.join("vpm-env"), false)
            .expect("vrc-get lib backend initializes"),
    );
    let identity_store =
        vua_unity_bridge::LocalPackageIdentityStore::new(base.join("identities.json"));
    let executor = MaterialExecutor::new(
        bridge,
        FileSystemSnapshotStore,
        vpm_backend,
        BuildRecordStore::new(project_root.join(".vua/records")),
        Arc::new(vua_orchestrator::SystemClock),
        base.join("temp"),
        "2022.3.22f1",
        identity_store,
    )
    .with_staging_template_override(&staging_template);

    P2Harness {
        project_root,
        project,
        source,
        base,
        confirmation,
        executor,
    }
}

/// S2: cancellation observed at a staging-import boundary reports Cancelled
/// with a receipt — the staging project is destroyed, nothing is installed.
#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 (VUA_UNITY_EXECUTABLE + VUA_REAL_SOURCE_FOLDER); cancels mid-run"]
fn m3_real_local_reusable_cancel_at_step_boundary_records_facts() {
    let harness = p2_harness("vpm-cancel");
    let token = MaterialCancelToken::new();
    let cancel_handle = token.clone();
    std::thread::spawn(move || {
        // Staging creation + the first Unity inspect take 30s+; cancelling
        // at 2s lands the request before the first staging import lands.
        std::thread::sleep(Duration::from_secs(2));
        cancel_handle.cancel();
    });
    let report = harness.executor.execute(
        &harness.confirmation,
        &harness.source,
        &harness.project,
        &harness.project_root.join(".vua/artifacts"),
        &token,
    );
    println!(
        "cancelled P2 run completed steps: {:?}",
        report.completed_steps
    );
    assert_eq!(
        report.status,
        MaterialExecutionStatus::Cancelled,
        "report: {report:?}"
    );
    assert_eq!(report.error_code.as_deref(), Some("vua.material.cancelled"));
    assert!(
        report.build_record_id.is_some(),
        "cancelled runs get a receipt"
    );
    let staging_expected = vua_unity_bridge::staging_root(
        &harness.base.join("temp"),
        &harness.confirmation.correlation_id,
    )
    .expect("valid session id");
    assert!(
        !staging_expected.exists(),
        "cancelled staging must be destroyed"
    );

    let _ = fs::remove_dir_all(&harness.base);
    let _ = fs::remove_dir_all(&harness.project_root);
}

/// S3: a source archive changed after confirm fails the P2 run with
/// `vua.material.source_drift` before that archive is imported.
#[test]
#[ignore = "manual: real filesystem drift probe against the real local-reusable pipeline"]
fn m3_real_local_reusable_source_drift_fails_the_run() {
    let harness = p2_harness("vpm-drift");
    // Drift one archive after confirm, before execute.
    let pack = harness.source.join("Cineon_Meiyun_v1.00.unitypackage");
    let mut bytes = fs::read(&pack).unwrap();
    bytes.extend_from_slice(b"drift");
    fs::write(&pack, &bytes).unwrap();

    let report = harness.executor.execute(
        &harness.confirmation,
        &harness.source,
        &harness.project,
        &harness.project_root.join(".vua/artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(
        report.status,
        MaterialExecutionStatus::Failed,
        "report: {report:?}"
    );
    assert_eq!(
        report.error_code.as_deref(),
        Some("vua.material.source_drift")
    );

    let _ = fs::remove_dir_all(&harness.base);
    let _ = fs::remove_dir_all(&harness.project_root);
}

/// S4: a 1s bridge budget cannot survive the staging inspect, so the P2 run
/// fails with `vua.material.bridge_timeout` and the verified target snapshot
/// is restored.
#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 (VUA_UNITY_EXECUTABLE + VUA_REAL_SOURCE_FOLDER) under a 1s budget"]
fn m3_real_local_reusable_bridge_timeout_budget_is_enforced() {
    let unity = unity_executable();
    let harness = p2_harness("vpm-timeout");
    let tight_bridge: Arc<dyn UnityBridge> =
        Arc::new(UnityBatchBridge::new(unity).with_timeout(Duration::from_secs(1)));
    let executor = MaterialExecutor::new(
        tight_bridge,
        FileSystemSnapshotStore,
        Arc::new(NoVpm),
        BuildRecordStore::new(harness.project_root.join(".vua/records")),
        Arc::new(vua_orchestrator::SystemClock),
        harness.base.join("temp-tight"),
        "2022.3.22f1",
        vua_unity_bridge::LocalPackageIdentityStore::new(harness.base.join("identities.json")),
    );
    let report = executor.execute(
        &harness.confirmation,
        &harness.source,
        &harness.project,
        &harness.project_root.join(".vua/artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(
        report.status,
        MaterialExecutionStatus::Failed,
        "report: {report:?}"
    );
    assert!(report
        .error_code
        .as_deref()
        .unwrap_or("")
        .starts_with("vua.material.bridge_timeout"));
    assert_eq!(report.rollback, RollbackOutcome::Restored);

    let _ = fs::remove_dir_all(&harness.base);
    let _ = fs::remove_dir_all(&harness.project_root);
}

/// S5+S6: an external mutation of the staging project between its commands
/// breaks the chained fingerprint, the Bridge rejects, and the verified
/// target snapshot is restored.
#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 (VUA_UNITY_EXECUTABLE + VUA_REAL_SOURCE_FOLDER); injects a staging mutation mid-run"]
fn m3_real_local_reusable_staging_rejection_restores_target() {
    let harness = p2_harness("vpm-reject");
    let staging_dir = vua_unity_bridge::staging_root(
        &harness.base.join("temp"),
        &harness.confirmation.correlation_id,
    )
    .expect("valid session id");
    let injector = staging_dir.clone();
    std::thread::spawn(move || {
        // Wait until the first staging import has returned (its result file
        // lands in the staging bridge exchange), then mutate the staging
        // tree: the next staging command's chained fingerprint no longer
        // matches and the real Bridge rejects it. A fixed sleep cannot do
        // this reliably — a mutation landing during the staging inspect is
        // absorbed into that inspect's fingerprint.
        // The project fingerprint tracks the active scene hierarchy, not
        // loose Assets files, so a file drop cannot move it. Instead, corrupt
        // the second staging import's unpacked manifest.sha256 once it
        // lands: the Bridge's manifest validation then rejects the command
        // for real.
        let imports_dir = injector.join(".vua/imports");
        let deadline = Instant::now() + Duration::from_secs(900);
        loop {
            if Instant::now() > deadline {
                break;
            }
            let mut manifests: Vec<std::path::PathBuf> = Vec::new();
            if let Ok(entries) = fs::read_dir(&imports_dir) {
                for entry in entries.filter_map(Result::ok) {
                    let manifest = entry.path().join("manifest.sha256");
                    if manifest.is_file() {
                        manifests.push(manifest);
                    }
                }
            }
            if manifests.len() >= 2 {
                manifests.sort_by_key(|path| fs::metadata(path).and_then(|m| m.modified()).ok());
                if let Some(latest) = manifests.last() {
                    let mut bytes = fs::read(latest).unwrap_or_default();
                    bytes.truncate(bytes.len().saturating_sub(64));
                    let _ = fs::write(latest, &bytes);
                }
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
    let report = harness.executor.execute(
        &harness.confirmation,
        &harness.source,
        &harness.project,
        &harness.project_root.join(".vua/artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(
        report.status,
        MaterialExecutionStatus::Failed,
        "report: {report:?}"
    );
    assert_eq!(
        report.error_code.as_deref(),
        Some("vua.material.bridge_rejected"),
        "the real Bridge must reject the drifted staging fingerprint"
    );
    assert_eq!(
        report.rollback,
        RollbackOutcome::Restored,
        "a post-snapshot rejection restores the verified target snapshot"
    );

    let _ = fs::remove_dir_all(&harness.base);
    let _ = fs::remove_dir_all(&harness.project_root);
}

/// S7: a failed restore is recorded as `RollbackOutcome::Failed` with a
/// `rollback_failed` receipt - injected by removing the target snapshot
/// manifest while the staging pipeline runs.
#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 (VUA_UNITY_EXECUTABLE + VUA_REAL_SOURCE_FOLDER); injects snapshot loss mid-run"]
fn m3_real_local_reusable_rollback_failure_is_recorded_not_hidden() {
    let harness = p2_harness("vpm-rbfail");
    let staging_dir = vua_unity_bridge::staging_root(
        &harness.base.join("temp"),
        &harness.confirmation.correlation_id,
    )
    .expect("valid session id");
    let injector_root = harness.project_root.clone();
    let injector_staging = staging_dir.clone();
    std::thread::spawn(move || {
        // 1) Force a real failure: mutate the staging tree once its first
        //    import has returned, so the Bridge rejects the next command.
        // The project fingerprint tracks the active scene hierarchy, not
        // loose Assets files, so a file drop cannot move it. Instead, corrupt
        // the second staging import's unpacked manifest.sha256 once it
        // lands: the Bridge's manifest validation then rejects the command
        // for real.
        let imports_dir = injector_staging.join(".vua/imports");
        let deadline = Instant::now() + Duration::from_secs(900);
        loop {
            if Instant::now() > deadline {
                break;
            }
            let mut manifests: Vec<std::path::PathBuf> = Vec::new();
            if let Ok(entries) = fs::read_dir(&imports_dir) {
                for entry in entries.filter_map(Result::ok) {
                    let manifest = entry.path().join("manifest.sha256");
                    if manifest.is_file() {
                        manifests.push(manifest);
                    }
                }
            }
            if manifests.len() >= 2 {
                manifests.sort_by_key(|path| fs::metadata(path).and_then(|m| m.modified()).ok());
                if let Some(latest) = manifests.last() {
                    let mut bytes = fs::read(latest).unwrap_or_default();
                    bytes.truncate(bytes.len().saturating_sub(64));
                    let _ = fs::write(latest, &bytes);
                }
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        // 2) The target snapshot was created before the pipeline started;
        //    removing its manifest turns the now-attempted restore into a
        //    recorded failure.
        let snapshots = injector_root.join(".vua/snapshots");
        if let Ok(entries) = fs::read_dir(&snapshots) {
            for entry in entries.filter_map(Result::ok) {
                let _ = fs::remove_file(entry.path().join("manifest.json"));
            }
        }
    });
    let report = harness.executor.execute(
        &harness.confirmation,
        &harness.source,
        &harness.project,
        &harness.project_root.join(".vua/artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(
        report.status,
        MaterialExecutionStatus::Failed,
        "report: {report:?}"
    );
    assert_eq!(report.rollback, RollbackOutcome::Failed);
    assert!(report
        .error_code
        .as_deref()
        .unwrap_or("")
        .contains("vua.material.rollback_failed"));
    let record = vua_orchestrator::BuildRecordStore::new(harness.project_root.join(".vua/records"))
        .read(report.build_record_id.as_deref().expect("record id"))
        .expect("the worst outcome still gets a receipt");
    assert_eq!(record.status, vua_orchestrator::BuildRecordStatus::Failed);

    let _ = fs::remove_dir_all(&harness.base);
    let _ = fs::remove_dir_all(&harness.project_root);
}

/// S8: a succeeded local-reusable receipt replays without touching Unity.
#[test]
#[ignore = "manual: launches real Unity 2022.3.22f1 (VUA_UNITY_EXECUTABLE + VUA_REAL_SOURCE_FOLDER)"]
fn m3_real_local_reusable_replay_of_succeeded_receipt_skips_unity() {
    let harness = p2_harness("vpm-replay");
    let first = harness.executor.execute(
        &harness.confirmation,
        &harness.source,
        &harness.project,
        &harness.project_root.join(".vua/artifacts"),
        &MaterialCancelToken::new(),
    );
    assert_eq!(
        first.status,
        MaterialExecutionStatus::Succeeded,
        "first run: {first:?}"
    );
    assert!(!first.replayed);
    let requests_after_first = bridge_request_count(&harness.project_root);

    let started = Instant::now();
    let second = harness.executor.execute(
        &harness.confirmation,
        &harness.source,
        &harness.project,
        &harness.project_root.join(".vua/artifacts"),
        &MaterialCancelToken::new(),
    );
    println!("P2 replay wall time: {:?}", started.elapsed());
    assert!(
        second.replayed,
        "a succeeded receipt must replay: {second:?}"
    );
    assert_eq!(second.status, MaterialExecutionStatus::Succeeded);
    assert_eq!(second.completed_steps, Vec::new());
    assert_eq!(second.build_record_id, first.build_record_id);
    assert_eq!(
        bridge_request_count(&harness.project_root),
        requests_after_first,
        "the replay must not touch Unity"
    );

    let _ = fs::remove_dir_all(&harness.base);
    let _ = fs::remove_dir_all(&harness.project_root);
}
