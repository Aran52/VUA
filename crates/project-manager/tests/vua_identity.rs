//! VUA-native identity file tests (user rulings 2026-09-09 items 7/12):
//! `.vua/project.json` — read tri-state, note lifecycle, and the
//! marking discipline (marking never invents a note; notes attach to
//! VUA-native projects only). Everything runs against synthetic trees.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use vua_project_manager::{
    mark_vua_native, read_identity, set_note, SetNoteError, VuaIdentity, VuaProjectIdentityV1,
    IDENTITY_SCHEMA_VERSION,
};

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-identity-{label}-{nanos}"))
}

fn cleanup(base: &Path) {
    if base.exists() {
        fs::remove_dir_all(base).unwrap();
    }
}

#[test]
fn unmarked_projects_read_as_absent() {
    let base = unique_dir("absent");
    let project = base.join("Plain Project");
    fs::create_dir_all(&project).unwrap();

    assert_eq!(read_identity(&project), VuaIdentity::Absent);

    cleanup(&base);
}

#[test]
fn marking_then_reading_round_trips_and_preserves_the_note_on_remark() {
    let base = unique_dir("roundtrip");
    let project = base.join("Migrated World");
    fs::create_dir_all(&project).unwrap();

    let marked = mark_vua_native(&project, "2026-09-09T15:04:05Z").unwrap();
    assert_eq!(marked.marked_at, "2026-09-09T15:04:05Z");
    assert_eq!(marked.note, None, "marking never invents a note");

    // A note is set, then a re-mark (e.g. a second import pass) keeps it.
    let updated = set_note(&project, Some("用户的迁移项目")).unwrap();
    assert_eq!(updated.note.as_deref(), Some("用户的迁移项目"));
    assert_eq!(
        updated.marked_at, "2026-09-09T15:04:05Z",
        "set_note keeps the marking time"
    );

    let marked_again = mark_vua_native(&project, "2026-09-10T00:00:00Z").unwrap();
    assert_eq!(
        marked_again.note.as_deref(),
        Some("用户的迁移项目"),
        "re-marking preserves the user note"
    );
    assert_eq!(marked_again.marked_at, "2026-09-10T00:00:00Z");

    match read_identity(&project) {
        VuaIdentity::Present(identity) => {
            assert_eq!(identity.identity_version, IDENTITY_SCHEMA_VERSION);
            assert_eq!(identity.note.as_deref(), Some("用户的迁移项目"));
        }
        other => panic!("expected a present identity, got {other:?}"),
    }

    // Clearing the note round-trips too.
    let cleared = set_note(&project, None).unwrap();
    assert_eq!(cleared.note, None);

    cleanup(&base);
}

#[test]
fn unreadable_identity_files_are_evidence_never_absent() {
    let base = unique_dir("unreadable");
    let project = base.join("Broken");
    let vua_dir = project.join(".vua");
    fs::create_dir_all(&vua_dir).unwrap();

    // Garbage content.
    fs::write(vua_dir.join("project.json"), "{ not json").unwrap();
    assert_eq!(read_identity(&project), VuaIdentity::Unreadable);

    // Valid JSON but an unknown identity version.
    fs::write(
        vua_dir.join("project.json"),
        r#"{ "identityVersion": 99, "markedAt": "x", "note": null }"#,
    )
    .unwrap();
    assert_eq!(read_identity(&project), VuaIdentity::Unreadable);

    // The identity path being a directory reads as unreadable as well.
    fs::remove_file(vua_dir.join("project.json")).unwrap();
    fs::create_dir_all(vua_dir.join("project.json")).unwrap();
    assert_eq!(read_identity(&project), VuaIdentity::Unreadable);

    cleanup(&base);
}

#[test]
fn notes_attach_to_vua_native_projects_only() {
    let base = unique_dir("note-gates");
    let plain = base.join("Not VUA");
    fs::create_dir_all(&plain).unwrap();

    match set_note(&plain, Some("nope")) {
        Err(SetNoteError::NotVuaNative) => {}
        other => panic!("expected NotVuaNative, got {other:?}"),
    }

    let broken = base.join("Broken VUA");
    let vua_dir = broken.join(".vua");
    fs::create_dir_all(&vua_dir).unwrap();
    fs::write(vua_dir.join("project.json"), "garbage").unwrap();
    match set_note(&broken, Some("nope")) {
        Err(SetNoteError::Unreadable) => {}
        other => panic!("expected Unreadable, got {other:?}"),
    }
    // The unreadable file is not silently overwritten by the refused note.
    assert_eq!(
        fs::read_to_string(vua_dir.join("project.json")).unwrap(),
        "garbage"
    );

    cleanup(&base);
}

#[test]
fn written_identity_documents_serialize_the_versioned_shape() {
    let identity = VuaProjectIdentityV1 {
        identity_version: IDENTITY_SCHEMA_VERSION,
        marked_at: "2026-09-09T15:04:05Z".to_owned(),
        note: Some("备注".to_owned()),
    };
    let json = serde_json::to_value(&identity).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "identityVersion": 1,
            "markedAt": "2026-09-09T15:04:05Z",
            "note": "备注",
        })
    );
}
