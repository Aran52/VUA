use serde_json::{json, Value};
use std::{fs, io::Cursor, path::PathBuf};
use vua_provider_host::run_provider_host;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/scenario-environment/v0.1")
}
fn root_v02() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/scenario-environment/v0.2")
}
fn read(path: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn read_v02(path: &str) -> Value {
    serde_json::from_slice(&fs::read(root_v02().join(path)).unwrap()).unwrap()
}
#[test]
fn shared_request_and_result_schema_vectors() {
    for (dir, load) in [(root(), read as fn(&str) -> Value), (root_v02(), read_v02 as fn(&str) -> Value)] {
        for (schema, directory) in [
            ("request.schema.json", "examples"),
            ("result.schema.json", "results"),
        ] {
            let validator = jsonschema::validator_for(&load(schema)).unwrap();
            for file in fs::read_dir(dir.join(directory)).unwrap() {
                let path = file.unwrap().path();
                let value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                assert_eq!(
                    validator.is_valid(&value),
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .contains(".valid."),
                    "{}",
                    path.display()
                );
            }
        }
    }
}
#[test]
fn pure_sop_is_served_without_environment_services_and_rejections_stay_honest() {
    let cases = [
        "sop-desktop.valid.json",
        "sop-wifi.valid.json",
        "sop-invalid-connection.invalid.json",
        "sop-injection.invalid.json",
        "check-c1.valid.json",
        "check-path.invalid.json",
        "check-schema.invalid.json",
        "check-command.invalid.json",
    ];
    let mut input = String::new();
    for (index, name) in cases.iter().enumerate() {
        input.push_str(&format!("{}\n",json!({"frameVersion":"0.1","frameId":format!("scenario-{index}"),"kind":"request","payload":read(&format!("examples/{name}"))})));
    }
    let temporary = std::env::temp_dir().join(format!(
        "vua-scenario-wire-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&temporary).unwrap();
    let mut output = Vec::new();
    run_provider_host(Cursor::new(input), &mut output, temporary.join("tasks.db")).unwrap();
    let frames: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let reply = |index| {
        frames
            .iter()
            .find(|v| v["frameId"] == format!("scenario-{index}"))
            .unwrap()["payload"]
            .clone()
    };
    let validator = jsonschema::validator_for(&read("result.schema.json")).unwrap();
    for index in [0, 1] {
        assert_eq!(reply(index)["ok"], true);
        assert!(validator.is_valid(&reply(index)["value"]));
        assert_eq!(reply(index)["value"]["performed"], "sop_only");
    }
    assert_eq!(reply(4)["error"]["code"], "vua.scenario.unavailable");
    for index in [2, 3, 5, 6, 7] {
        assert_eq!(reply(index)["ok"], false);
        assert_ne!(reply(index)["error"]["code"], "vua.scenario.unavailable");
    }
    assert!(!temporary.join("amf-production").exists());
    assert!(
        temporary.starts_with(std::env::temp_dir())
            && temporary
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("vua-scenario-wire-")
    );
    fs::remove_dir_all(temporary).unwrap();
}

#[test]
fn play_setups_and_sop_v02_cross_the_wire_without_services_and_reject_invalid_combinations() {
    let cases = [
        "list-play-setups.valid.json",
        "sop-vrchat-pc.valid.json",
        "sop-vrchat-pico-connect.valid.json",
        "sop-vrchat-quest-standalone.valid.json",
        "sop-chilloutvr-index.valid.json",
        "list-play-setups-command.invalid.json",
        "list-play-setups-extra.invalid.json",
        "sop-reborn-pc.invalid.json",
        "sop-chilloutvr-standalone.invalid.json",
        "sop-quest-pico-connect.invalid.json",
        "sop-quest-null-connection.invalid.json",
        "sop-pc-with-connection.invalid.json",
    ];
    let mut input = String::new();
    for (index, name) in cases.iter().enumerate() {
        input.push_str(&format!(
            "{}\n",
            json!({
                "frameVersion": "0.1",
                "frameId": format!("scenario-v02-{index}"),
                "kind": "request",
                "payload": read_v02(&format!("examples/{name}"))
            })
        ));
    }
    let temporary = std::env::temp_dir().join(format!(
        "vua-scenario-v02-wire-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&temporary).unwrap();
    let mut output = Vec::new();
    run_provider_host(Cursor::new(input), &mut output, temporary.join("tasks.db")).unwrap();
    let frames: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let reply = |index| {
        frames
            .iter()
            .find(|v| v["frameId"] == format!("scenario-v02-{index}"))
            .unwrap()["payload"]
            .clone()
    };
    let validator = jsonschema::validator_for(&read_v02("result.schema.json")).unwrap();
    for index in 0..5 {
        assert_eq!(reply(index)["ok"], true, "{}", reply(index));
        assert!(validator.is_valid(&reply(index)["value"]), "{}", reply(index));
    }
    assert_eq!(reply(0)["value"]["platforms"].as_array().unwrap().len(), 3);
    assert_eq!(reply(0)["value"]["devices"].as_array().unwrap().len(), 15);
    assert_eq!(reply(0)["value"]["connections"].as_array().unwrap().len(), 17);
    assert_eq!(reply(1)["value"]["engine"]["route"], "desktop_play");
    assert_eq!(reply(2)["value"]["engine"]["route"], "pico_pcvr");
    assert_eq!(reply(3)["value"]["engine"]["kind"], "headset");
    assert_eq!(reply(4)["value"]["engine"]["toolId"], "chilloutvr");
    for index in 5..cases.len() {
        assert_eq!(reply(index)["ok"], false, "{}", reply(index));
        assert_eq!(reply(index)["error"]["code"], "vua.scenario.invalid_params");
    }
    assert!(
        temporary.starts_with(std::env::temp_dir())
            && temporary
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("vua-scenario-v02-wire-")
    );
    fs::remove_dir_all(temporary).unwrap();
}
