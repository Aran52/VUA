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
fn face_tracking_is_optional_source_evidence_and_patch_requires_a_bound_write() {
    let old: UnityResult = serde_json::from_value(read("examples/assembly.valid.result.json")).unwrap();
    assert!(old.data.get("faceTracking").is_none());
    let value = read("examples/face-tracking.valid.result.json");
    vua_orchestrator::amf_production::validate_face_tracking(&value["data"]["faceTracking"]).unwrap();
    for file in ["invalid-face-tracking-runtime.result.json", "invalid-face-tracking-cost.result.json"] {
        assert!(vua_orchestrator::amf_production::validate_face_tracking(&read(&format!("examples/{file}"))["data"]["faceTracking"]).is_err());
    }
    let command = read("examples/face-tracking.valid.command.json");
    let operation: UnityOperation = serde_json::from_value(command["operation"].clone()).unwrap();
    assert_eq!(operation, UnityOperation::AmfFtPatch);
    assert!(operation.is_mutating() && command["dryRun"] == false);
    assert!(command["expectedProjectFingerprint"].is_string());
    let patch: vua_orchestrator::amf_production::FaceTrackingPatch =
        serde_json::from_value(read("examples/face-tracking.valid.plan.json")["faceTrackingPatch"].clone()).unwrap();
    assert!(patch.confirm_trusted_code);
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
    let material_use = data["materialUse"].as_array().unwrap();
    assert_eq!(material_use.len(), 4);
    assert_eq!(material_use[0]["owner"], "avatar");
    assert_eq!(material_use[0]["renderer"], "Body");
    assert_eq!(material_use[0]["material"], "Assets/Synthetic/Body.mat");
    assert_eq!(material_use[2]["owner"], "source-outfit");
}

/// Results carry optional fit facts; results written before fits existed stay valid, and
/// the Unity side may always emit both lists (empty when nothing moved).
#[test]
fn fit_facts_are_optional_and_bounded() {
    let validator = jsonschema::validator_for(&read("result.schema.json")).unwrap();
    let full = read("examples/inspection-full.valid.result.json");
    let result: UnityResult = serde_json::from_value(full.clone()).unwrap();
    let changes = result.data["fitChanges"].as_array().unwrap();
    assert_eq!(changes[0]["sourceId"], "source-outfit");
    assert_eq!(changes[0]["moved"][0]["path"], "HairClip");
    assert_eq!(changes[0]["moved"][0]["localScale"]["x"], 1.0);
    assert_eq!(changes[0]["missing"][0], "OldHook");
    assert_eq!(result.data["fitSkipped"][0]["sourceId"], "source-accessory");
    assert_eq!(result.data["fitSkipped"][0]["paths"][0], "Clips#2");
    let mut without = full.clone();
    without["data"].as_object_mut().unwrap().remove("fitChanges");
    without["data"].as_object_mut().unwrap().remove("fitSkipped");
    assert!(validator.is_valid(&without));
    let mut empty = full.clone();
    empty["data"]["fitChanges"] = Value::Array(vec![]);
    empty["data"]["fitSkipped"] = Value::Array(vec![]);
    assert!(validator.is_valid(&empty), "JsonUtility always emits both lists");
    let mut broken = full;
    broken["data"]["fitChanges"][0]["moved"][0]
        .as_object_mut()
        .unwrap()
        .remove("localScale");
    assert!(!validator.is_valid(&broken));
}

/// Results written before material use existed stay valid; the list is capped at 512 entries.
#[test]
fn material_use_is_optional_and_bounded() {
    let validator = jsonschema::validator_for(&read("result.schema.json")).unwrap();
    let full = read("examples/inspection-full.valid.result.json");
    let mut without = full.clone();
    without["data"].as_object_mut().unwrap().remove("materialUse");
    assert!(validator.is_valid(&without));
    let mut foreign = full.clone();
    foreign["data"]["materialUse"][0]["material"] = Value::String("Resources/unity_builtin_extra".into());
    assert!(!validator.is_valid(&foreign));
    let mut over = full;
    let entries = vec![over["data"]["materialUse"][0].clone(); 513];
    over["data"]["materialUse"] = Value::Array(entries);
    assert!(!validator.is_valid(&over));
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

#[test]
fn creator_menu_schema_accepts_custom_roots_without_native_commands_or_shortcuts() {
    let validator = jsonschema::validator_for(&read("result.schema.json")).unwrap();
    let mut result = read("examples/face-tracking.valid.result.json");
    for path in ["Tools/Creator/Apply", "Creator FT/Apply"] {
        result["data"]["faceTracking"]["patchMenus"][0]["menuPath"] = path.into();
        assert!(validator.is_valid(&result), "{path}");
    }
    for path in ["Assets/Apply", "Window/Apply", "Tools//Apply", "Tools/Apply %g", "Tools/Apply _g", "Tools/"] {
        result["data"]["faceTracking"]["patchMenus"][0]["menuPath"] = path.into();
        assert!(!validator.is_valid(&result), "{path}");
    }
}
