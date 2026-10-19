use serde_json::Value;
use std::{fs, path::PathBuf};

#[test]
fn terms_request_and_result_vectors_consume_the_candidate_schemas() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/terms/v0.1");
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
                "{}: {:?}",
                path.display(),
                validator
                    .iter_errors(&value)
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
            );
        }
    }
}
