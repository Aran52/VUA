//! Cross-profile project mutation lock and unfinished-mutation marker
//! (instance-identity ADR, decision 7). Exclusion comes from the OS
//! byte-range lock: a second handle in the SAME process is refused while
//! the first holds it, which is exactly what a second VUA instance on
//! another machine/channel profile would experience.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use vua_project_manager::{
    acquire_project_lock, begin_mutation, read_pending_mutation, LockHolder, PendingMutation,
    ProjectLockError, LOCK_FILE_NAME, MARKER_FILE_NAME,
};

fn temp_project(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vua-plock-{label}-{nanos}"));
    fs::create_dir_all(root.join("Assets")).unwrap();
    root
}

fn holder(instance: &str) -> LockHolder {
    LockHolder {
        channel: "dev".into(),
        profile: format!("profile-{instance}"),
        pid: std::process::id(),
        instance_id: instance.into(),
        acquired_at: "2026-09-05T00:00:00Z".into(),
    }
}

#[test]
fn plk_001_second_holder_is_refused_with_diagnostics_then_wins_after_release() {
    let root = temp_project("exclusive");
    let guard = acquire_project_lock(&root, holder("a")).expect("first acquire");
    let lock_path = root.join(".vua").join(LOCK_FILE_NAME);
    assert_eq!(guard.lock_path(), lock_path);

    // A second contender (same process, fresh handle — the OS lock, not
    // file presence, is the exclusion) is refused, and the refusal names
    // the holder it saw.
    match acquire_project_lock(&root, holder("b")) {
        Err(ProjectLockError::Held { previous }) => {
            assert_eq!(previous.map(|h| h.instance_id).as_deref(), Some("a"));
        }
        other => panic!("second acquire must be Held, got {other:?}"),
    }

    // The envelope on disk is the versioned diagnostic of the holder.
    let raw = fs::read_to_string(&lock_path).unwrap();
    let envelope: vua_project_manager::LockEnvelopeV1 = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        envelope.lock_version,
        vua_project_manager::PROJECT_LOCK_SCHEMA_VERSION
    );
    assert_eq!(envelope.holder.profile, "profile-a");

    guard.release().expect("release");
    // Release clears the diagnostics; a new contender acquires cleanly.
    assert_eq!(fs::read_to_string(&lock_path).unwrap(), "");
    let second = acquire_project_lock(&root, holder("b")).expect("acquire after release");
    second.release().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn plk_002_dropped_guard_releases_the_lock() {
    let root = temp_project("drop");
    {
        let _guard = acquire_project_lock(&root, holder("a")).expect("acquire");
    }
    let guard = acquire_project_lock(&root, holder("b")).expect("drop released it");
    guard.release().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn plk_003_marker_lifecycle_and_leftover_findings() {
    let root = temp_project("marker");
    assert_eq!(read_pending_mutation(&root), PendingMutation::None);

    let marker = begin_mutation(&root, "material_intake", &holder("a")).expect("begin");
    assert_eq!(
        marker.marker_path(),
        root.join(".vua").join(MARKER_FILE_NAME)
    );
    match read_pending_mutation(&root) {
        PendingMutation::Leftover(left) => {
            assert_eq!(left.mutation_kind, "material_intake");
            assert_eq!(
                left.marker_version,
                vua_project_manager::MUTATION_MARKER_SCHEMA_VERSION
            );
            assert_eq!(left.holder.instance_id, "a");
        }
        other => panic!("live marker must read as Leftover, got {other:?}"),
    }

    marker.release().unwrap();
    assert_eq!(read_pending_mutation(&root), PendingMutation::None);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn plk_004_unreadable_marker_is_evidence_not_clean() {
    let root = temp_project("torn");
    let marker_path = root.join(".vua").join(MARKER_FILE_NAME);
    fs::create_dir_all(marker_path.parent().unwrap()).unwrap();
    // A crash mid-write can leave a torn or empty marker; the gate must
    // treat that as "inspect first", never as clean.
    fs::write(&marker_path, "{\"marker_vers").unwrap();
    assert_eq!(read_pending_mutation(&root), PendingMutation::Unreadable);

    // A future marker version is equally not-understood: inspect first.
    fs::write(
        &marker_path,
        "{\"markerVersion\":99,\"mutationKind\":\"x\"}",
    )
    .unwrap();
    assert_eq!(read_pending_mutation(&root), PendingMutation::Unreadable);

    // A new mutation overwrites the leftover once the caller decided to
    // proceed, and the new guard's release clears it.
    let marker = begin_mutation(&root, "recipe_apply", &holder("b")).unwrap();
    assert!(matches!(
        read_pending_mutation(&root),
        PendingMutation::Leftover(_)
    ));
    drop(marker);
    assert_eq!(read_pending_mutation(&root), PendingMutation::None);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn plk_005_lock_and_marker_are_independent_gates() {
    // The ADR keeps them separate: the lock is live exclusion, the marker
    // is crash evidence. Holding one says nothing about the other.
    let root = temp_project("independent");
    let guard = acquire_project_lock(&root, holder("a")).expect("lock");
    let marker = begin_mutation(&root, "material_intake", &holder("a")).unwrap();

    guard.release().unwrap();
    assert!(
        matches!(read_pending_mutation(&root), PendingMutation::Leftover(_)),
        "releasing the lock must not clear crash evidence"
    );

    marker.release().unwrap();
    let guard2 = acquire_project_lock(&root, holder("b")).expect("re-lock");
    drop(guard2);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn plk_006_unreadable_marker_read_errors_are_not_clean() {
    let root = temp_project("unreadable");
    let marker_path = root.join(".vua").join(MARKER_FILE_NAME);
    fs::create_dir_all(&marker_path).unwrap();
    // The marker path exists but cannot be read as a file (it IS a
    // directory here) — a permission-denied elsewhere behaves the same.
    assert_eq!(read_pending_mutation(&root), PendingMutation::Unreadable);
    fs::remove_dir_all(&root).unwrap();
}
