//! Characterization tests for the pre-O1 prototype (E-S0).
//!
//! These tests lock the current observable behavior of the vertical prototype
//! before any boundary split. Each test name cites the ORC requirement it
//! protects (ORC-TST-006). When behavior is deliberately changed later, these
//! tests are updated in the same PR that changes the behavior.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use vua_orchestrator::{
    AvatarSetupRequest, AvatarSetupWorkflow, FileSystemProjectStore, FileSystemSnapshotStore,
    ProjectRef, SnapshotRef, SnapshotStore, UnityResult, VpmProjectProvisioner, WorkflowError,
    WorkflowStage,
};

/// Unique per call: parallel tests can read the same clock value on Windows, so the
/// process id and a counter keep their temp projects apart.
fn unique_suffix() -> String {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{}-{nanos}-{serial}", std::process::id())
}

fn project() -> ProjectRef {
    let suffix = unique_suffix();
    ProjectRef {
        id: format!("char-{suffix}"),
        root: std::env::temp_dir().join(format!("vua-char-{suffix}")),
    }
}

#[test]
fn orc_sto_009_bridge_command_id_cannot_escape_the_bridge_directory() {
    let project = project();
    fs::create_dir_all(project.root.join("ProjectSettings")).unwrap();
    fs::write(
        project.root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1",
    )
    .unwrap();
    FileSystemProjectStore::initialize(&project).unwrap();

    let outside = project.root.parent().unwrap().join("escaped.request.json");
    let result = FileSystemProjectStore::write_bridge_command(
        &project,
        "../../../escaped",
        &serde_json::json!({"safe": false}),
    );
    assert!(result.is_err());
    assert!(!outside.exists());
    fs::remove_dir_all(&project.root).ok();
}

fn initialize(project: &ProjectRef) {
    fs::create_dir_all(project.root.join("Assets")).unwrap();
    fs::create_dir_all(project.root.join("Packages")).unwrap();
    fs::create_dir_all(project.root.join("ProjectSettings")).unwrap();
    fs::write(
        project.root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1",
    )
    .unwrap();
    FileSystemProjectStore::initialize(project).unwrap();
}

fn request() -> AvatarSetupRequest {
    AvatarSetupRequest {
        workflow_id: "char-demo".into(),
        project: project(),
        avatar_global_object_id: "avatar".into(),
        avatar_armature_global_object_id: "avatar-armature".into(),
        outfit_global_object_id: "outfit".into(),
        outfit_armature_global_object_id: "outfit-armature".into(),
        toggle_name: "测试衣装".into(),
    }
}

fn succeeded_result(command_id: &str) -> UnityResult {
    UnityResult {
        schema_version: 1,
        command_id: command_id.into(),
        status: vua_orchestrator::ResultStatus::Succeeded,
        changed_paths: Vec::new(),
        diagnostics: Vec::new(),
        data: serde_json::Value::Null,
        steps: Vec::new(),
        replayed: None,
        snapshot_id: None,
        restored_from: None,
        project_fingerprint_before: None,
    }
}

/// Drives a confirmed workflow through all six operations with success results.
fn drive_to_completion(workflow: &mut AvatarSetupWorkflow) {
    while let Some(command) = workflow.next_command(None).unwrap() {
        workflow
            .accept_result(succeeded_result(&command.command_id))
            .unwrap();
    }
}

struct FailingSnapshots;

impl SnapshotStore for FailingSnapshots {
    type Error = String;

    fn create(&self, _project: &ProjectRef, _snapshot_id: &str) -> Result<SnapshotRef, String> {
        Err("simulated snapshot failure".into())
    }

    fn restore(&self, _project: &ProjectRef, _snapshot: &SnapshotRef) -> Result<(), String> {
        Err("simulated restore failure".into())
    }
}

// --- Workflow stage transitions (ORC-WF-001, ORC-TST-002) ---

#[test]
fn orc_wf_001_next_command_is_rejected_outside_execute_and_validate() {
    let mut workflow = AvatarSetupWorkflow::new(request());
    let error = workflow
        .next_command(None)
        .expect_err("Plan stage must reject next_command");
    assert_eq!(
        error,
        WorkflowError::InvalidStage {
            expected: WorkflowStage::Execute,
            actual: WorkflowStage::Plan,
        }
    );

    workflow.plan();
    let error = workflow
        .next_command(None)
        .expect_err("AwaitConfirmation stage must reject next_command");
    assert_eq!(
        error,
        WorkflowError::InvalidStage {
            expected: WorkflowStage::Execute,
            actual: WorkflowStage::AwaitConfirmation,
        }
    );
}

#[test]
fn orc_tst_002_second_confirm_is_rejected() {
    let request = request();
    let root = request.project.root.clone();
    initialize(&request.project);
    let mut workflow = AvatarSetupWorkflow::new(request);
    workflow.plan();
    workflow.confirm(&FileSystemSnapshotStore).unwrap();
    let error = workflow
        .confirm(&FileSystemSnapshotStore)
        .expect_err("confirm twice must be rejected");
    assert_eq!(
        error,
        WorkflowError::InvalidStage {
            expected: WorkflowStage::AwaitConfirmation,
            actual: WorkflowStage::Execute,
        }
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn orc_wf_002_completed_workflow_returns_no_further_command() {
    let request = request();
    let root = request.project.root.clone();
    initialize(&request.project);
    let mut workflow = AvatarSetupWorkflow::new(request);
    workflow.plan();
    workflow.confirm(&FileSystemSnapshotStore).unwrap();
    drive_to_completion(&mut workflow);

    assert_eq!(workflow.stage(), WorkflowStage::Completed);
    assert!(workflow.next_command(None).unwrap().is_none());
    assert_eq!(workflow.stage(), WorkflowStage::Completed);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn orc_con_003_pending_command_blocks_a_second_next_command() {
    let request = request();
    let root = request.project.root.clone();
    initialize(&request.project);
    let mut workflow = AvatarSetupWorkflow::new(request);
    workflow.plan();
    workflow.confirm(&FileSystemSnapshotStore).unwrap();

    workflow.next_command(None).unwrap();
    let error = workflow
        .next_command(None)
        .expect_err("a pending command must block the next one");
    assert_eq!(
        error,
        WorkflowError::CommandFailed("a command is already pending".into())
    );
    fs::remove_dir_all(root).unwrap();
}

// --- Result handling (ORC-WF-008, ORC-WF-010) ---

#[test]
fn orc_wf_008_result_without_pending_command_is_rejected() {
    let mut workflow = AvatarSetupWorkflow::new(request());
    let error = workflow
        .accept_result(succeeded_result("char-demo-01"))
        .expect_err("a result without a pending command must be rejected");
    assert_eq!(
        error,
        WorkflowError::CommandFailed("received a result with no pending command".into())
    );
}

#[test]
fn orc_wf_008_mismatched_command_id_is_rejected_and_pending_is_preserved() {
    let request = request();
    let root = request.project.root.clone();
    initialize(&request.project);
    let mut workflow = AvatarSetupWorkflow::new(request);
    workflow.plan();
    workflow.confirm(&FileSystemSnapshotStore).unwrap();

    let command = workflow.next_command(None).unwrap().unwrap();
    let error = workflow
        .accept_result(succeeded_result("wrong-id"))
        .expect_err("a mismatched command id must be rejected");
    assert_eq!(
        error,
        WorkflowError::MismatchedCommand {
            expected: command.command_id.clone(),
            actual: "wrong-id".into(),
        }
    );

    // The original pending command is preserved: the correct result is still accepted.
    workflow
        .accept_result(succeeded_result(&command.command_id))
        .unwrap();
    assert_eq!(workflow.report().completed_operations.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn orc_wf_010_rejected_result_enters_recovery_with_first_diagnostic() {
    let request = request();
    let root = request.project.root.clone();
    initialize(&request.project);
    let mut workflow = AvatarSetupWorkflow::new(request);
    workflow.plan();
    workflow.confirm(&FileSystemSnapshotStore).unwrap();

    let command = workflow.next_command(None).unwrap().unwrap();
    let error = workflow
        .accept_result(UnityResult {
            schema_version: 1,
            command_id: command.command_id,
            status: vua_orchestrator::ResultStatus::Rejected,
            changed_paths: Vec::new(),
            diagnostics: vec![vua_orchestrator::Diagnostic {
                code: "unity.rejected".into(),
                severity: vua_orchestrator::DiagnosticSeverity::Error,
                message: "outfit mesh missing".into(),
            }],
            data: serde_json::Value::Null,
            steps: Vec::new(),
            replayed: None,
            snapshot_id: None,
            restored_from: None,
            project_fingerprint_before: None,
        })
        .expect_err("a rejected result must fail the workflow");
    assert_eq!(
        error,
        WorkflowError::CommandFailed("outfit mesh missing".into())
    );
    assert_eq!(workflow.stage(), WorkflowStage::Recover);
    fs::remove_dir_all(root).unwrap();
}

// --- Snapshot failure and recovery (ORC-WF-005, recovery matrix) ---

#[test]
fn orc_wf_005_snapshot_failure_stops_in_recover_without_verified_snapshot() {
    let mut workflow = AvatarSetupWorkflow::new(request());
    workflow.plan();

    let error = workflow
        .confirm(&FailingSnapshots)
        .expect_err("a failed snapshot must not enter execution");
    assert!(matches!(error, WorkflowError::Snapshot(_)));
    assert_eq!(workflow.stage(), WorkflowStage::Recover);

    let error = workflow
        .recover(&FailingSnapshots)
        .expect_err("recover without a verified snapshot must fail");
    assert!(matches!(error, WorkflowError::Snapshot(_)));
}

#[test]
fn orc_wf_005_recover_is_rejected_outside_the_recover_stage() {
    let mut workflow = AvatarSetupWorkflow::new(request());
    let error = workflow
        .recover(&FileSystemSnapshotStore)
        .expect_err("recover outside the Recover stage must be rejected");
    assert_eq!(
        error,
        WorkflowError::InvalidStage {
            expected: WorkflowStage::Recover,
            actual: WorkflowStage::Plan,
        }
    );
}

// --- Project provisioning input validation (ORC-ADP-001, ORC-ADP-004) ---

#[test]
fn orc_adp_001_invalid_project_names_are_rejected_before_any_process_or_io() {
    let provisioner = VpmProjectProvisioner::new("vrc-get-does-not-exist");
    for name in [
        "", " ", "  a", "a  ", ".", "..", "a/b", "a\\b", "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b",
        "a|b",
    ] {
        let error = provisioner
            .create_avatar_project(Path::new("Z:/definitely-missing-parent"), name, "id")
            .expect_err("invalid names must be rejected");
        assert!(
            matches!(error, vua_orchestrator::ProjectProvisionError::InvalidName),
            "name {name:?} must be classified InvalidName, got {error:?}"
        );
    }
}

#[test]
fn orc_adp_004_missing_parent_is_detected_without_spawning_vpm() {
    let provisioner = VpmProjectProvisioner::new("vrc-get-does-not-exist");
    let error = provisioner
        .create_avatar_project(
            Path::new("Z:/definitely-missing-parent"),
            "valid-name",
            "id",
        )
        .expect_err("a missing parent must be rejected");
    assert!(matches!(
        error,
        vua_orchestrator::ProjectProvisionError::ParentMissing
    ));
}

#[test]
fn orc_adp_004_existing_target_is_rejected_without_spawning_vpm() {
    let suffix = unique_suffix();
    let parent = std::env::temp_dir().join(format!("vua-char-parent-{suffix}"));
    fs::create_dir_all(parent.join("taken")).unwrap();

    let provisioner = VpmProjectProvisioner::new("vrc-get-does-not-exist");
    let error = provisioner
        .create_avatar_project(&parent, "taken", "id")
        .expect_err("an existing target must be rejected");
    assert!(matches!(
        error,
        vua_orchestrator::ProjectProvisionError::TargetExists
    ));
    fs::remove_dir_all(&parent).unwrap();
}

// --- Snapshot store boundaries (ORC-STO-003, ORC-STO-009) ---

#[test]
fn orc_sto_009_snapshot_identifier_rejects_traversal_and_invalid_characters() {
    let project_ref = project();
    initialize(&project_ref);
    let store = FileSystemSnapshotStore;
    for id in [
        "",
        "../escape",
        "a/b",
        "a\\b",
        "a b",
        "快照",
        "a.b",
        ".hidden",
    ] {
        let error = store
            .create(&project_ref, id)
            .expect_err("unsafe identifiers must be rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "id {id:?}");
    }
    store
        .create(&project_ref, "safe-Id_1")
        .expect("alphanumeric, dash and underscore identifiers must be accepted");
    // The public grammar gate (for callers re-validating ids recovered from
    // persisted payloads) shares the same vectors — single grammar source.
    for id in ["", "../escape", "a/b", "a\\b", "a b", "快照", "a.b", ".hidden"] {
        assert_eq!(
            FileSystemSnapshotStore::validate_snapshot_id(id)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidInput,
            "id {id:?}"
        );
    }
    assert!(FileSystemSnapshotStore::validate_snapshot_id("safe-Id_1").is_ok());
    fs::remove_dir_all(project_ref.root).unwrap();
}

#[test]
fn orc_sto_003_duplicate_snapshot_identifier_is_rejected() {
    let project_ref = project();
    initialize(&project_ref);
    let store = FileSystemSnapshotStore;
    store.create(&project_ref, "snap").unwrap();
    let error = store
        .create(&project_ref, "snap")
        .expect_err("a duplicate snapshot id must be rejected");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    fs::remove_dir_all(project_ref.root).unwrap();
}

#[test]
fn orc_sto_009_restore_rejects_snapshots_outside_the_project() {
    let project_ref = project();
    initialize(&project_ref);
    let store = FileSystemSnapshotStore;

    let outside = SnapshotRef {
        id: "elsewhere".into(),
        path: PathBuf::from("Z:/elsewhere/elsewhere"),
    };
    let error = store
        .restore(&project_ref, &outside)
        .expect_err("a snapshot outside the project must be rejected");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);

    let foreign = SnapshotRef {
        id: "foreign".into(),
        path: project_ref.root.join("Assets/foreign"),
    };
    let error = store
        .restore(&project_ref, &foreign)
        .expect_err("a directory not under .vua/snapshots must be rejected");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    fs::remove_dir_all(project_ref.root).unwrap();
}

#[test]
fn orc_sto_003_initialize_rejects_a_non_unity_directory() {
    let suffix = unique_suffix();
    let root = std::env::temp_dir().join(format!("vua-char-empty-{suffix}"));
    fs::create_dir_all(&root).unwrap();
    let bare = ProjectRef {
        id: format!("char-empty-{suffix}"),
        root: root.clone(),
    };
    let error = FileSystemProjectStore::initialize(&bare)
        .expect_err("a directory without ProjectVersion.txt must be rejected");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn orc_adp_005_bridge_requests_are_written_into_the_versioned_job_directory() {
    let project_ref = project();
    initialize(&project_ref);

    let command = vua_orchestrator::UnityCommand {
        schema_version: 1,
        command_id: "char-req-01".into(),
        operation: vua_orchestrator::UnityOperation::InspectProject,
        project_id: project_ref.id.clone(),
        dry_run: true,
        expected_project_fingerprint: None,
        payload: vua_orchestrator::UnityPayload {
            avatar_global_object_id: "a".into(),
            avatar_armature_global_object_id: "aa".into(),
            outfit_global_object_id: "o".into(),
            outfit_armature_global_object_id: "oa".into(),
            toggle_name: "t".into(),
            ..vua_orchestrator::UnityPayload::default()
        },
    };
    let path =
        FileSystemProjectStore::write_bridge_command(&project_ref, &command.command_id, &command)
            .unwrap();
    assert_eq!(
        path,
        project_ref
            .root
            .join(".vua/bridge")
            .join("char-req-01.request.json")
    );
    let bytes = fs::read(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(parsed["schemaVersion"], 1);
    assert_eq!(parsed["operation"], "inspect_project");
    fs::remove_dir_all(project_ref.root).unwrap();
}

// --- Wire shape defaults (ORC-TYP-007) ---

#[test]
fn orc_typ_007_unity_result_defaults_missing_data_to_null() {
    let result: UnityResult = serde_json::from_str(
        r#"{
            "schemaVersion": 1,
            "commandId": "w-1",
            "status": "succeeded",
            "changedPaths": [],
            "diagnostics": []
        }"#,
    )
    .unwrap();
    assert_eq!(result.data, serde_json::Value::Null);
    assert_eq!(result.command_id, "w-1");
}
