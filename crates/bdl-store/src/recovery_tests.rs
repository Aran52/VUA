use super::*;
const TOKEN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const BINDING: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
fn maintain(profile: &Path, token: &str, binding: &str, action: &str) -> Result<String> {
    let receipt = if action == "snapshot" {
        None
    } else {
        Some(backup_receipt(profile, token, binding)?)
    };
    super::maintain(profile, token, binding, action, receipt.as_deref())
}

#[test]
fn changed_presence_or_layout_metadata_cannot_redirect_restore() {
    let temp = tempfile::tempdir().unwrap();
    let profile = temp.path();
    let bdl = database(profile, "bdl", 7);
    maintain(profile, TOKEN, BINDING, "snapshot").unwrap();
    let receipt = backup_receipt(profile, TOKEN, BINDING).unwrap();
    write(&bdl, 8);
    let file = profile.join(format!("modules/amf/update-backups/{TOKEN}/snapshot.json"));
    let original: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    for changed in ["presence", "layout"] {
        let mut value = original.clone();
        if changed == "presence" {
            value["files"][0]["present"] = serde_json::json!(false);
        } else {
            value["layout"] = serde_json::json!("legacy");
        }
        fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(super::maintain(profile, TOKEN, BINDING, "restore", Some(&receipt)).is_err());
        assert_eq!(read(&bdl), 8);
    }
}

fn database(profile: &Path, name: &str, value: i64) -> PathBuf {
    let target = database_path(profile, "isolated", name).unwrap();
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    let connection = Connection::open(&target).unwrap();
    connection
        .execute_batch("CREATE TABLE test_value(value INTEGER);")
        .unwrap();
    connection
        .execute("INSERT INTO test_value VALUES (?1)", [value])
        .unwrap();
    target
}
fn read(path: &Path) -> i64 {
    Connection::open(path)
        .unwrap()
        .query_row("SELECT value FROM test_value", [], |row| row.get(0))
        .unwrap()
}
fn write(path: &Path, value: i64) {
    Connection::open(path)
        .unwrap()
        .execute("UPDATE test_value SET value=?1", [value])
        .unwrap();
}

#[test]
fn snapshot_includes_wal_and_restore_removes_trial_only_database() {
    let temp = tempfile::tempdir().unwrap();
    let profile = temp.path();
    let bdl = database(profile, "bdl", 7);
    let connection = Connection::open(&bdl).unwrap();
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; UPDATE test_value SET value=8;",
        )
        .unwrap();
    let assets = profile.join("paid-assets");
    fs::create_dir(&assets).unwrap();
    fs::write(assets.join("keep"), "original").unwrap();
    assert_eq!(
        maintain(profile, TOKEN, BINDING, "snapshot").unwrap(),
        "prepared"
    );
    drop(connection);
    write(&bdl, 99);
    let tasks = database(profile, "tasks", 55);
    assert_eq!(
        maintain(profile, TOKEN, BINDING, "restore").unwrap(),
        "restored"
    );
    assert_eq!(read(&bdl), 8);
    assert!(!tasks.exists());
    assert_eq!(fs::read(assets.join("keep")).unwrap(), b"original");
    write(&bdl, 10);
    assert_eq!(
        maintain(profile, TOKEN, BINDING, "restore").unwrap(),
        "restored"
    );
    assert_eq!(
        read(&bdl),
        10,
        "an idempotent retry must not erase later work"
    );
}

#[test]
fn corrupt_backup_or_identity_mismatch_never_changes_either_live_database() {
    let temp = tempfile::tempdir().unwrap();
    let profile = temp.path();
    let bdl = database(profile, "bdl", 1);
    let tasks = database(profile, "tasks", 2);
    maintain(profile, TOKEN, BINDING, "snapshot").unwrap();
    write(&bdl, 11);
    write(&tasks, 22);
    assert!(maintain(profile, TOKEN, &"c".repeat(64), "restore").is_err());
    fs::write(
        profile.join(format!("modules/amf/update-backups/{TOKEN}/tasks.db")),
        b"corrupt",
    )
    .unwrap();
    assert!(maintain(profile, TOKEN, BINDING, "restore").is_err());
    assert_eq!(read(&bdl), 11);
    assert_eq!(read(&tasks), 22);
    assert!(maintain(profile, "../escape", BINDING, "snapshot").is_err());
}

#[test]
fn committed_snapshot_cannot_erase_post_update_work() {
    let temp = tempfile::tempdir().unwrap();
    let profile = temp.path();
    let bdl = database(profile, "bdl", 1);
    maintain(profile, TOKEN, BINDING, "snapshot").unwrap();
    maintain(profile, TOKEN, BINDING, "commit").unwrap();
    write(&bdl, 123);
    assert!(maintain(profile, TOKEN, BINDING, "restore").is_err());
    assert_eq!(read(&bdl), 123);
}

#[test]
fn migration_trial_restores_the_original_format_without_opening_an_old_reader() {
    let temp = tempfile::tempdir().unwrap();
    let profile = temp.path();
    let bdl = database_path(profile, "legacy", "bdl").unwrap();
    fs::create_dir_all(bdl.parent().unwrap()).unwrap();
    let connection = Connection::open(&bdl).unwrap();
    for sql in [
        include_str!("../../../schemas/bdl/v0.1/001_initial.sql"),
        include_str!("../../../schemas/bdl/v0.2/002_dependency_observations.sql"),
        include_str!("../../../schemas/bdl/v0.3/003_library_type.sql"),
        include_str!("../../../schemas/bdl/v0.3/004_variant_name.sql"),
        include_str!("../../../schemas/bdl/v0.4/005_product_downloadables.sql"),
        include_str!("../../../schemas/bdl/v0.5/006_managed_library_files.sql"),
        include_str!("../../../schemas/bdl/v0.6/007_archive_members.sql"),
        include_str!("../../../schemas/bdl/v0.7/008_library_entry_metadata.sql"),
        include_str!("../../../schemas/bdl/v0.8/009_removed_local_entries.sql"),
        include_str!("../../../schemas/bdl/v0.9/010_removed_library_cards.sql"),
        include_str!("../../../schemas/bdl/v0.10/011_description_links.sql"),
    ] {
        connection.execute_batch(sql).unwrap();
    }
    connection.pragma_update(None, "user_version", 11).unwrap();
    drop(connection);
    maintain(profile, TOKEN, BINDING, "snapshot").unwrap();
    maintain(profile, TOKEN, BINDING, "migrate").unwrap();
    let format = |path: &Path| -> String {
        Connection::open(path)
            .unwrap()
            .query_row(
                "SELECT value FROM bdl_meta WHERE key='format_version'",
                [],
                |row| row.get(0),
            )
            .unwrap()
    };
    assert_eq!(format(&bdl), "0.11");
    maintain(profile, TOKEN, BINDING, "restore").unwrap();
    assert_eq!(format(&bdl), "0.10");
    assert_eq!(
        Connection::open(&bdl)
            .unwrap()
            .pragma_query_value::<i64, _>(None, "user_version", |row| row.get(0))
            .unwrap(),
        11
    );
}
