use super::*;
use crate::component_feed::ComponentKind::Amf;
use rusqlite::Connection;

#[tokio::test]
#[ignore = "Requires actual independently built AMF UI/provider and recovery launcher"]
async fn actual_data_recovery_migrates_before_trial_and_restores_before_an_old_reader() {
    let executable = std::path::PathBuf::from(
        std::env::var_os("VUA_TEST_AMF_COMPONENT_EXECUTABLE").expect("AMF executable"),
    );
    let ui =
        std::path::PathBuf::from(std::env::var_os("VUA_TEST_AMF_COMPONENT_UI").expect("AMF UI"));
    let launcher =
        fs::read(std::env::var_os("VUA_TEST_COMPONENT_LAUNCHER").expect("Recovery launcher"))
            .unwrap();
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(ui.join("component.json")).unwrap()).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input");
    let (mut current, _, key) = fixture(&input);
    current.host.version = "0.7.0".into();
    current.release_version = "0.7.0".into();
    current.compatibility.host = ">=0.7.0, <0.8.0".into();
    let host_metadata = serde_json::to_vec(&serde_json::json!({"schemaVersion":"0.1", "dataTrialAbi":"amf-data/0.1", "build":current.host.build})).unwrap();
    let (files, bytes, pack) = pack(&[
        ("dist/electron/main.js", b"main"),
        ("dist/electron/preload.js", b"preload"),
        ("dist/renderer/index.html", b"host"),
        ("dist/renderer/update-runtime.json", &host_metadata),
        ("provider/vua-orchestrator-provider.exe", b"host-exe"),
        ("bootstrap/app.asar", b"bootstrap"),
        ("launcher/VUA.exe", &launcher),
    ]);
    current.host.files = files;
    current.host.pack = pack;
    fs::write(input.join("host.bin"), bytes).unwrap();
    let installation =
        Installation::with_key(&temp.path().join("installation"), true, key).unwrap();
    installation
        .install(&signed(&current).0, &input, &mut transfer())
        .await
        .unwrap();
    fs::write(installation.root.join("VUA.exe"), launcher).unwrap();
    let before = installation.active().unwrap();
    let provider = fs::read(executable).unwrap();
    let entry = fs::read(ui.join("amf-entry.js")).unwrap();
    let css = fs::read(ui.join("style.css")).unwrap();
    let sidecar = fs::read(ui.join("component.json")).unwrap();
    let (files, bytes, pack) = super::pack(&[
        ("ui/amf-entry.js", &entry),
        ("ui/style.css", &css),
        ("ui/component.json", &sidecar),
        ("provider/vua-amf-provider.exe", &provider),
    ]);
    let mut target = current.clone();
    target.amf.files = files;
    target.amf.pack = pack;
    target.amf.version = metadata["version"].as_str().unwrap().into();
    target.amf.build = metadata["build"].as_str().unwrap().into();
    target.compatibility.bdl_write = metadata["bdlWrite"].as_str().unwrap().into();
    target
        .compatibility
        .bdl_read
        .push(target.compatibility.bdl_write.clone());
    let id = installation.store_manifest(&signed(&target).0).unwrap().0;
    installation
        .accept_component_index(Amf, &component_index(&target, Amf, 1))
        .unwrap();
    let pack_file = input.join("candidate.bin");
    fs::write(&pack_file, bytes).unwrap();
    let profile = temp.path().join("profile");
    let bdl = profile.join("bdl/bdl.db");
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
    connection
        .execute_batch("CREATE TABLE trial_test(value INTEGER); INSERT INTO trial_test VALUES(7);")
        .unwrap();
    drop(connection);
    let assets = profile.join("paid-assets");
    fs::create_dir(&assets).unwrap();
    fs::write(assets.join("keep"), b"keep").unwrap();
    let format = || -> String {
        Connection::open(&bdl)
            .unwrap()
            .query_row(
                "SELECT value FROM bdl_meta WHERE key='format_version'",
                [],
                |row| row.get(0),
            )
            .unwrap()
    };
    for healthy in [false, true] {
        assert_eq!(format(), "0.10");
        installation.register_data_profile(&profile).unwrap();
        installation
            .stage_component(Amf, &id, &mut transfer(), Some(&pack_file))
            .await
            .unwrap();
        installation.request_component_activation(Amf).unwrap();
        installation
            .activate_component(Amf, &mut transfer())
            .await
            .unwrap();
        assert_eq!(format(), "0.11");
        assert_eq!(
            installation.data_trial_status(&profile).unwrap()["pending"],
            true
        );
        assert!(installation.data_trial_status(temp.path()).is_err());
        Connection::open(&bdl)
            .unwrap()
            .execute_batch("UPDATE trial_test SET value=99;")
            .unwrap();
        if healthy {
            installation
                .acknowledge_component_health(&target.host.build, &target.amf.build)
                .unwrap();
            assert_eq!(
                installation.data_trial_status(&profile).unwrap()["pending"],
                false
            );
            assert!(installation.restore_component(Amf).is_err());
            assert_eq!(
                Connection::open(&bdl)
                    .unwrap()
                    .query_row::<i64, _, _>("SELECT value FROM trial_test", [], |row| row.get(0))
                    .unwrap(),
                99
            );
        } else {
            assert!(installation.recover_component_activation().unwrap());
            assert_eq!(installation.active().unwrap(), before);
            assert_eq!(format(), "0.10");
            assert_eq!(
                Connection::open(&bdl)
                    .unwrap()
                    .query_row::<i64, _, _>("SELECT value FROM trial_test", [], |row| row.get(0))
                    .unwrap(),
                7
            );
        }
        assert_eq!(fs::read(assets.join("keep")).unwrap(), b"keep");
    }
}
