//! Offline roundtrip through the real AMF provider, using a synthetic image only.
use serde_json::{json, Value};
use std::{fs, io::Cursor, sync::Arc};
use vua_bdl_store::{ArtifactMode, BdlStore};
use vua_provider_host::{
    run_provider_host_with_services,
    share_image::{self, Position},
    WarehouseConfig,
};

#[test]
fn shared_image_uses_the_owned_cache_and_returns_no_native_paths() {
    let root = std::env::temp_dir().join(format!(
        "vua-share-wire-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cache = root.join("amf-production/share-images");
    fs::create_dir_all(&cache).unwrap();
    let input = "input-00000000-0000-0000-0000-000000000000.bin";
    let image = image::RgbaImage::from_pixel(1024, 1024, image::Rgba([100, 100, 100, 255]));
    let bytes = share_image::png(
        &share_image::decorate(&image, Some("3P3J7H0EGWP1"), Position::BottomRight).unwrap(),
    )
    .unwrap();
    fs::write(cache.join(input), bytes).unwrap();
    let bdl = Arc::new(BdlStore::open(root.join("bdl.db")).unwrap());
    let cases = [
        json!({"schemaVersion":"0.1","inputId":input}),
        json!({"schemaVersion":"0.1","inputId":"../outside"}),
        json!({"schemaVersion":"0.1","inputId":input,"path":"C:/outside.png"}),
        json!({"schemaVersion":"0.1","inputId":"input-🐈00000000000000000000000000000000.bin"}),
    ];
    let requests=cases.iter().enumerate().map(|(index,params)|json!({"frameVersion":"0.1","frameId":format!("share-{index}"),"kind":"request","payload":{"contractVersion":"0.1","requestId":format!("share-{index}"),"kind":"query","method":"amf.production.readShareImage","params":params}}).to_string()).collect::<Vec<_>>().join("\n")+"\n";
    let mut output = Vec::new();
    run_provider_host_with_services(
        Cursor::new(requests),
        &mut output,
        root.join("tasks.db"),
        None,
        None,
        Some(WarehouseConfig {
            bdl: bdl.clone(),
            warehouse_root: root.join("warehouse"),
            global_default: ArtifactMode::UseOriginalUnitypackage,
            executor: None,
            dependencies_queries: None,
        }),
        None,
    )
    .unwrap();
    let frames = String::from_utf8(output).unwrap();
    let values = frames
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    let reply = |index| {
        values
            .iter()
            .find(|frame| frame["frameId"] == format!("share-{index}"))
            .unwrap()["payload"]
            .clone()
    };
    assert_eq!(reply(0)["ok"], true, "{}", reply(0));
    assert_eq!(reply(0)["value"]["code"], "3P3J7H0EGWP1");
    let artifact = reply(0)["value"]["artifactId"].as_str().unwrap().to_owned();
    assert!(artifact.starts_with("share-"));
    assert!(cache.join(artifact).is_file());
    for index in 1..cases.len() {
        assert_eq!(reply(index)["ok"], false);
    }
    assert!(!frames.contains(&root.to_string_lossy().replace('\\', "\\\\")));
    drop(bdl);
    assert!(
        root.starts_with(std::env::temp_dir())
            && root
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("vua-share-wire-")
    );
    fs::remove_dir_all(root).unwrap();
}
