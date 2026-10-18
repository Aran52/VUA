//! v5 is separate from frozen v1-v4. These vectors do not claim a Unity run.
use serde_json::Value;
use std::{fs, path::PathBuf};
use vua_orchestrator::{UnityCommand, UnityOperation, UnityResult};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/unity-bridge/v5")
}
fn read(file: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(file)).unwrap()).unwrap()
}

#[test]
fn candidate_bridge_schemas_refuse_legacy_ops_drift_and_false_sdk_claims() {
    for entry in fs::read_dir(root().join("examples")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        let kind = if name.ends_with(".command.json") {
            "command"
        } else if name.ends_with(".plan.json") {
            "plan"
        } else {
            "result"
        };
        let validator = jsonschema::validator_for(&read(&format!("{kind}.schema.json"))).unwrap();
        let value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            validator.is_valid(&value),
            name.contains(".valid."),
            "{name}"
        );
    }
}

#[test]
fn native_consumer_retains_exact_c1_facts_and_never_upgrades_sdk_checks() {
    let command: UnityCommand =
        serde_json::from_value(read("examples/assembly.valid.command.json")).unwrap();
    assert_eq!(command.operation, UnityOperation::AmfAssemble);
    assert_eq!(command.schema_version, 5);
    assert!(command.operation.is_mutating());
    let result: UnityResult =
        serde_json::from_value(read("examples/assembly.valid.result.json")).unwrap();
    assert_eq!(result.command_id, command.command_id);
    assert_eq!(result.data["editorVersion"], "2022.3.22f1c1");
    assert_eq!(result.data["officialSdkChecks"], "not_run");
    assert_eq!(result.data["instances"][0]["menuModified"], true);
}

#[test]
fn turntable_command_writes_into_the_project_and_reports_its_fixed_frame_set() {
    let command: UnityCommand =
        serde_json::from_value(read("examples/turntable.valid.command.json")).unwrap();
    assert_eq!(command.operation, UnityOperation::AmfTurntable);
    assert!(!command.dry_run);
    assert!(command.operation.is_mutating());
    let result: UnityResult =
        serde_json::from_value(read("examples/turntable.valid.result.json")).unwrap();
    assert_eq!(result.command_id, command.command_id);
    assert_eq!(result.data["turntable"]["frames"], 60);
    assert_eq!(result.data["turntable"]["size"], 512);
}

#[test]
fn full_inspection_result_carries_the_sdk_review_and_every_observed_field() {
    let raw = read("examples/inspection-full.valid.result.json");
    assert_eq!(raw["operation"], "amf_inspect");
    let result: UnityResult = serde_json::from_value(raw).unwrap();
    let data = &result.data;
    assert_eq!(data["officialSdkChecks"], "not_run");
    assert_eq!(data["sdkSignedIn"], "yes");
    assert_eq!(data["hasMissingScripts"], false);
    assert_eq!(data["sdkReview"]["state"], "checked");
    assert_eq!(data["sdkReview"]["platform"], "pc");
    assert_eq!(data["sdkReview"]["overall"], "Good");
    assert_eq!(data["sdkReview"]["issues"][0]["level"], "error");
    assert_eq!(data["sdkReview"]["issues"][1]["canFix"], true);
    assert_eq!(data["sdkReview"]["stats"][0]["rating"], "Excellent");
    assert_eq!(data["previews"][0]["view"], "front");
    assert_eq!(
        data["previews"][0]["path"],
        ".vua/previews/n3-vector-inspect-full/front.png"
    );
    assert_eq!(
        data["prefabs"][1]["preview"],
        ".vua/previews/n3-vector-inspect-full/candidate-001.png"
    );
    assert_eq!(data["instances"][0]["exists"], true);
    assert_eq!(data["instances"][1]["bone"], "LeftHand");
    assert_eq!(data["instances"][1]["menuModified"], false);
    assert_eq!(
        data["observedSelection"]["outfits"][1]["mode"],
        "attach_to_bone"
    );
    assert_eq!(data["avatars"][0]["globalObjectId"], data["avatarGlobalObjectId"]);
    assert_eq!(data["avatars"][0]["observedSelection"]["outfits"].as_array().unwrap().len(), 2);
    assert_eq!(data["usedSourceIds"][0], "source-avatar");
    assert_eq!(data["missingShaderMaterials"][0], "Assets/Synthetic/Broken.mat");
    assert_eq!(data["sdkFixesApplied"].as_array().unwrap().len(), 0);
}

#[test]
fn sdk_fix_result_names_the_items_the_sdk_actually_fixed() {
    let raw = read("examples/sdk-fix.valid.result.json");
    assert_eq!(raw["operation"], "amf_sdk_fix");
    let result: UnityResult = serde_json::from_value(raw).unwrap();
    let data = &result.data;
    let applied = data["sdkFixesApplied"].as_array().unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0], "This avatar has unused components.");
    // Every applied fix is one of the SDK check's own sentences, still offered with Auto Fix.
    assert_eq!(data["sdkReview"]["state"], "checked");
    let issues = data["sdkReview"]["issues"].as_array().unwrap();
    assert!(issues
        .iter()
        .any(|issue| issue["text"] == applied[0] && issue["canFix"] == true));
    assert_eq!(data["officialSdkChecks"], "not_run");
}
