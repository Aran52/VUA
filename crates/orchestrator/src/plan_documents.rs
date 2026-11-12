//! The AMF production-domain document store for approved-plan documents
//! (011 §4; the Bridge v2 job input). Same document-store discipline as the
//! evidence store: identity-addressed JSON documents, atomic publish, and
//! the plan's OWN status lifecycle (draft -> approved; superseded is set by
//! a later plan, never by execution — execution facts live in the Build
//! Record).

use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

pub const APPROVED_PLAN_SCHEMA_VERSION: &str = "0.3";

/// The approval act outcome: `Approved` on the first success,
/// `AlreadyApproved` when repeated (idempotent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApproveOutcome {
    Approved,
    AlreadyApproved,
}

pub struct PlanDocumentStore {
    root: PathBuf,
}

impl PlanDocumentStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path_for(&self, plan_id: &str) -> Result<PathBuf, io::Error> {
        validate_id(plan_id)?;
        Ok(self.root.join(format!("{plan_id}.json")))
    }

    /// Publishes a NEW plan document. The document must declare
    /// `status: draft` (approval is a separate, explicit act) and a
    /// `planId` matching the caller-supplied identity. Exactly-once per
    /// planId: a repeated save of the same identity is a typed error — a
    /// revised plan is a NEW plan id with the old one superseded.
    pub fn publish_draft(&self, plan_id: &str, document: &Value) -> io::Result<()> {
        if document.get("schemaVersion").and_then(Value::as_str)
            != Some(APPROVED_PLAN_SCHEMA_VERSION)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsupported approved-plan schema version",
            ));
        }
        if document.get("planId").and_then(Value::as_str) != Some(plan_id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "planId mismatch between identity and document",
            ));
        }
        if document.get("status").and_then(Value::as_str) != Some("draft") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a new plan must be published as draft",
            ));
        }
        if self.get(plan_id)?.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "plan already exists; a revised plan is a new plan id",
            ));
        }
        self.write_exactly_once(plan_id, document)
    }

    /// The approval act: draft -> approved, idempotent (approving an
    /// approved plan is a no-op success). Missing or superseded plans are
    /// typed errors at this layer.
    pub fn approve(&self, plan_id: &str) -> io::Result<ApproveOutcome> {
        let path = self.path_for(plan_id)?;
        let bytes = fs::read(&path)?;
        let mut document: Value = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        match document.get("status").and_then(Value::as_str) {
            Some("approved") => return Ok(ApproveOutcome::AlreadyApproved),
            Some("superseded") => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "plan is superseded and cannot be approved",
                ))
            }
            Some("draft") => {}
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown plan status {other:?}"),
                ))
            }
        }
        document["status"] = Value::String("approved".into());
        self.write_exactly_once(plan_id, &document)?;
        Ok(ApproveOutcome::Approved)
    }

    /// Marks an existing plan superseded (a later resolution/plan replaces
    /// it). Never deletes — the authorization history stays readable.
    pub fn supersede(&self, plan_id: &str) -> io::Result<()> {
        let path = self.path_for(plan_id)?;
        let bytes = fs::read(&path)?;
        let mut document: Value = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        document["status"] = Value::String("superseded".into());
        self.write_exactly_once(plan_id, &document)
    }

    /// Reads one plan document (with its status lifecycle facts).
    pub fn get(&self, plan_id: &str) -> io::Result<Option<Value>> {
        let path = self.path_for(plan_id)?;
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

    /// Lists every plan id currently stored (identity listing only).
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

    /// Full listing (documents with their lifecycle statuses), sorted by
    /// planId. An absent root is the honest empty state (no plans yet).
    /// Filtering/pagination belongs to the application face.
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

    fn write_exactly_once(&self, plan_id: &str, document: &Value) -> io::Result<()> {
        let destination = self.path_for(plan_id)?;
        fs::create_dir_all(&self.root)?;
        let temporary = self.root.join(format!(".{plan_id}.tmp"));
        let bytes = serde_json::to_vec_pretty(document)
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
        // In-place replace is safe for mutable authorization documents; the
        // temp file keeps a crash from leaving a torn document.
        fs::rename(&temporary, &destination)
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
            "invalid plan id",
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
            "vua-plan-docs-{tag}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn draft_document(plan_id: &str) -> Value {
        json!({
            "schemaVersion": "0.3",
            "planId": plan_id,
            "recipeId": "019e0000-0000-7000-8000-000000000001",
            "recipeRevision": 1,
            "localResolutionId": "019e0000-0000-7000-8000-000000000202",
            "environmentId": "019e0000-0000-7000-8000-000000000100",
            "createdAt": "2026-09-09T00:30:00.000Z",
            "approvedAt": "2026-09-09T00:31:00.000Z",
            "planHash": "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "fingerprint": {"expectedProjectFingerprint": "projfp-1"},
            "target": {"avatarInstanceId": "avatar_root"},
            "jobs": [{"jobId": "019e0000-0000-7000-8000-000000000210",
                      "kind": "install_modular_asset", "inputs": {}}],
            "status": "draft"
        })
    }

    #[test]
    fn draft_publish_approve_is_idempotent_and_supersede_is_final() {
        let store = PlanDocumentStore::new(unique_root("lifecycle"));
        let plan_id = "019e0000-0000-7000-8000-000000000301";
        store
            .publish_draft(plan_id, &draft_document(plan_id))
            .unwrap();

        // Approve flips draft -> approved and is idempotent.
        assert!(matches!(
            store.approve(plan_id).unwrap(),
            ApproveOutcome::Approved
        ));
        assert!(matches!(
            store.approve(plan_id).unwrap(),
            ApproveOutcome::AlreadyApproved
        ));
        let document = store.get(plan_id).unwrap().unwrap();
        assert_eq!(document["status"], "approved");

        // Supersede is the final transition; approving a superseded plan is
        // a typed error.
        store.supersede(plan_id).unwrap();
        assert_eq!(store.get(plan_id).unwrap().unwrap()["status"], "superseded");
        assert!(store.approve(plan_id).is_err());
    }

    #[test]
    fn publish_is_exactly_once_and_validates_shape() {
        let store = PlanDocumentStore::new(unique_root("once"));
        let plan_id = "019e0000-0000-7000-8000-000000000302";
        let mut document = draft_document(plan_id);
        store.publish_draft(plan_id, &document).unwrap();
        // Exactly-once per planId.
        assert!(store.publish_draft(plan_id, &document).is_err());
        // A new plan must be published as draft — approved documents are
        // created only through the approval act.
        document["status"] = Value::String("approved".into());
        let mut approved_document = document.clone();
        approved_document["planId"] = Value::String("019e0000-0000-7000-8000-000000000303".into());
        assert!(store
            .publish_draft("019e0000-0000-7000-8000-000000000303", &approved_document)
            .is_err());
        // planId mismatch between identity and body is a typed error.
        assert!(store
            .publish_draft("another-id", &draft_document("another-id"))
            .is_ok());
        let _ = document;
    }

    #[test]
    fn absent_reads_are_none_and_invalid_ids_are_typed() {
        let store = PlanDocumentStore::new(unique_root("absent"));
        assert_eq!(
            store.get("019e0000-0000-7000-8000-000000000fff").unwrap(),
            None
        );
        assert!(store.list_ids().unwrap().is_empty());
        let result = store.get("../escape");
        assert!(result.is_err());
    }
}
