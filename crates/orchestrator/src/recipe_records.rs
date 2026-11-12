//! The AMF production-domain document store for Build Record v0.3
//! documents (schemas/recipe/v0.3/build-record.schema.json; W22 freeze).
//! Same document-store discipline as the evidence store: records are
//! immutable history — publish is exactly-once per buildId (hard link,
//! fails rather than replacing), reads are faithful, listing is
//! identity-only with kind/status aggregation left to consumers.

use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

pub const BUILD_RECORD_V03_SCHEMA_VERSION: &str = "0.3";

pub struct RecipeRecordStore {
    root: PathBuf,
}

impl RecipeRecordStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path_for(&self, build_id: &str) -> io::Result<PathBuf> {
        validate_id(build_id)?;
        Ok(self.root.join(format!("{build_id}.json")))
    }

    /// Publishes exactly once (hard link: fails rather than replacing an
    /// immutable prior record). The document must declare
    /// `schemaVersion: "0.3"` and a matching `buildId`.
    pub fn publish(&self, build_id: &str, record: &Value) -> io::Result<PathBuf> {
        if record.get("schemaVersion").and_then(Value::as_str)
            != Some(BUILD_RECORD_V03_SCHEMA_VERSION)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsupported build record schema version",
            ));
        }
        if record.get("buildId").and_then(Value::as_str) != Some(build_id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "buildId mismatch between identity and document",
            ));
        }
        if self.get(build_id)?.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "build record already exists; history is immutable",
            ));
        }
        fs::create_dir_all(&self.root)?;
        let destination = self.path_for(build_id)?;
        let temporary = self.root.join(format!(".{build_id}.tmp"));
        let bytes = serde_json::to_vec_pretty(record)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        {
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            output.write_all(&bytes)?;
            output.write_all(b"\n")?;
            output.sync_all()?;
        }
        match fs::hard_link(&temporary, &destination) {
            Ok(()) => {
                fs::remove_file(&temporary)?;
                Ok(destination)
            }
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                Err(error)
            }
        }
    }

    /// Reads one record (None when absent).
    pub fn get(&self, build_id: &str) -> io::Result<Option<Value>> {
        let path = self.path_for(build_id)?;
        match fs::read(&path) {
            Ok(bytes) => {
                Ok(Some(serde_json::from_slice(&bytes).map_err(|error| {
                    io::Error::new(io::ErrorKind::InvalidData, error)
                })?))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Identity listing (sorted by buildId).
    pub fn list_ids(&self) -> io::Result<Vec<String>> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(id) = name.strip_suffix(".json") {
                if !id.starts_with('.') {
                    ids.push(id.to_owned());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Full listing (documents), sorted by buildId. An absent root is the
    /// honest empty state (no records yet). Filtering/pagination belongs to
    /// the application face.
    pub fn list_documents(&self) -> io::Result<Vec<Value>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut documents = Vec::new();
        for id in self.list_ids()? {
            if let Some(document) = self.get(&id)? {
                documents.push(document);
            }
        }
        Ok(documents)
    }
}

fn validate_id(value: &str) -> io::Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid build record id",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn unique_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "vua-recipe-records-{tag}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn record_document(build_id: &str) -> Value {
        json!({
            "schemaVersion": "0.3",
            "buildId": build_id,
            "recipeId": "019e0000-0000-7000-8000-000000000001",
            "recipeRevision": 1,
            "planId": "019e0000-0000-7000-8000-000000000201",
            "planHash": "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "planSchemaVersion": "0.3",
            "environmentId": "019e0000-0000-7000-8000-000000000100",
            "startedAt": "2026-09-09T00:30:00.000Z",
            "finishedAt": "2026-09-09T00:31:00.000Z",
            "status": "succeeded",
            "inputs": {
                "recipeDigest": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
                "localResolutionDigest": "sha256:3333333333333333333333333333333333333333333333333333333333333333",
                "planHash": "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
            },
            "jobs": [],
            "recoveryPoints": [],
            "evidenceSummary": {"evidenceIds": []}
        })
    }

    #[test]
    fn publish_is_exactly_once_and_reads_back_faithfully() {
        let store = RecipeRecordStore::new(unique_root("once"));
        let build_id = "019e0000-0000-7000-8000-000000000401";
        let document = record_document(build_id);
        store.publish(build_id, &document).unwrap();
        assert!(store.publish(build_id, &document).is_err());
        let read = store.get(build_id).unwrap().unwrap();
        assert_eq!(read, document);
    }

    #[test]
    fn mismatched_identity_and_bad_versions_are_typed_errors() {
        let store = RecipeRecordStore::new(unique_root("guards"));
        let mut record = record_document("019e0000-0000-7000-8000-000000000402");
        record["schemaVersion"] = Value::String("0.2".into());
        assert!(store
            .publish("019e0000-0000-7000-8000-000000000402", &record)
            .is_err());
        let mut record = record_document("019e0000-0000-7000-8000-000000000402");
        record["buildId"] = Value::String("other-id".into());
        assert!(store
            .publish("019e0000-0000-7000-8000-000000000402", &record)
            .is_err());
    }

    #[test]
    fn list_ids_lists_sorted_and_get_absent_is_none() {
        let store = RecipeRecordStore::new(unique_root("list"));
        for id in [
            "019e0000-0000-7000-8000-000000000401",
            "019e0000-0000-7000-8000-000000000402",
        ] {
            let document = record_document(id);
            store.publish(id, &document).unwrap();
        }
        assert_eq!(
            store.list_ids().unwrap(),
            [
                "019e0000-0000-7000-8000-000000000401",
                "019e0000-0000-7000-8000-000000000402"
            ]
        );
        assert_eq!(
            store.get("019e0000-0000-7000-8000-000000000fff").unwrap(),
            None
        );
    }
}
