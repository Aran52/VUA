use serde_json::Value;
use std::{fs, path::PathBuf};

#[test]
fn unity_license_request_and_result_vectors_are_closed_and_independent() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/unity-license/v0.1");
    for (schema, folder) in [
        ("request.schema.json", "requests"),
        ("result.schema.json", "results"),
    ] {
        let schema: Value = serde_json::from_slice(&fs::read(root.join(schema)).unwrap()).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for entry in fs::read_dir(root.join(folder)).unwrap() {
            let path = entry.unwrap().path();
            let value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            assert_eq!(
                validator.is_valid(&value),
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(".valid.json"),
                "{}",
                path.display()
            );
        }
    }
    let scenario =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/scenario-environment/v0.1");
    let schema: Value =
        serde_json::from_slice(&fs::read(scenario.join("result.schema.json")).unwrap()).unwrap();
    let claim: Value = serde_json::from_slice(
        &fs::read(scenario.join("results/license-claim.invalid.json")).unwrap(),
    )
    .unwrap();
    assert!(!jsonschema::validator_for(&schema).unwrap().is_valid(&claim));
}
