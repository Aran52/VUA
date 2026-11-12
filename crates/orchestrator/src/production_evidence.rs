//! Immutable production-evidence documents (W23 freeze:
//! `schemas/production-evidence/v0.1/evidence.schema.json`) and their AMF
//! production-domain store (011 convergence decision: a document store per
//! the BuildRecordStore precedent — never BDL).
//!
//! Evidence facts are append-only: publish is exactly-once per evidenceId
//! (a repeated publication cannot rewrite an observed fact), and resolution
//! is a separate, later fact that references the evidence without rewriting
//! it. The Local Resolution document carries the evidence id in its
//! `assetResolution.evidenceIds[]`; the Build Record evidence summary may
//! carry it too — reference, never copy.

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

pub const PRODUCTION_EVIDENCE_SCHEMA_VERSION: &str = "0.1";

/// Closed vocabulary (v0.1): `missing_asset` = a Recipe-referenced asset is
/// absent from the warehouse and the filesystem; `missing_package` = a
/// declared dependency package is absent; `version_mismatch` = an observed
/// Unity/package version contradicts the recipe's constraint or lock;
/// `guard_denied` = a server-side guard refused an action and the refusal is
/// the evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    MissingAsset,
    MissingPackage,
    VersionMismatch,
    GuardDenied,
}

/// Namespaced identity of the subject the evidence is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceSubject {
    /// e.g. `booth:<id>`, `warehouse:<itemId>`, `pkg:<id>`, or an explicit
    /// path when no identity exists yet.
    pub ref_: String,
    /// Display label when one was observed; never invented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Where the evidence came from: exactly one of the two references is
/// required (Local Resolution document or producing task correlation).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceSourceRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_resolution_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_correlation: Option<String>,
}

impl EvidenceSourceRef {
    pub fn from_local_resolution(id: impl Into<String>) -> Self {
        Self {
            local_resolution_id: Some(id.into()),
            task_correlation: None,
        }
    }

    pub fn from_task_correlation(id: impl Into<String>) -> Self {
        Self {
            local_resolution_id: None,
            task_correlation: Some(id.into()),
        }
    }
}

/// `null` = unresolved (the evidence stands); an object = the fact was later
/// satisfied, with the reference to what satisfied it. Resolving never
/// deletes or rewrites the observed evidence — it is a separate fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceResolution {
    pub resolved_at: String,
    pub resolution_ref: String,
}

/// The frozen production-evidence document (v0.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionEvidenceV01 {
    pub schema_version: String,
    pub evidence_id: String,
    pub kind: EvidenceKind,
    pub subject: EvidenceSubject,
    pub observed_at: String,
    pub detail: String,
    #[serde(default)]
    pub source_ref: EvidenceSourceRef,
    /// `None` serializes as `null` (unresolved) — the field is always
    /// present on the wire per the frozen schema.
    pub resolution: Option<EvidenceResolution>,
}

impl ProductionEvidenceV01 {
    pub fn new_unresolved(
        evidence_id: impl Into<String>,
        kind: EvidenceKind,
        subject: EvidenceSubject,
        observed_at: impl Into<String>,
        detail: impl Into<String>,
        source_ref: EvidenceSourceRef,
    ) -> Self {
        Self {
            schema_version: PRODUCTION_EVIDENCE_SCHEMA_VERSION.to_owned(),
            evidence_id: evidence_id.into(),
            kind,
            subject,
            observed_at: observed_at.into(),
            detail: detail.into(),
            source_ref,
            resolution: None,
        }
    }
}

/// The AMF production-domain document store for evidence (BuildRecordStore
/// precedent: `{evidenceId}.json` under a root, atomic create-new publish,
/// exactly-once per identity).
pub struct EvidenceStore {
    root: PathBuf,
}

impl EvidenceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn path_for(&self, evidence_id: &str) -> io::Result<PathBuf> {
        validate_id(evidence_id)?;
        Ok(self.root.join(format!("{evidence_id}.json")))
    }

    /// Publishes exactly once. Evidence is an observed fact: a repeated
    /// production run must read and return the existing document; it can
    /// never overwrite it.
    pub fn publish(&self, evidence: &ProductionEvidenceV01) -> io::Result<PathBuf> {
        if evidence.schema_version != PRODUCTION_EVIDENCE_SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsupported production evidence version",
            ));
        }
        let destination = self.path_for(&evidence.evidence_id)?;
        fs::create_dir_all(&self.root)?;
        let temporary = self.root.join(format!(".{}.tmp", evidence.evidence_id));
        let bytes = serde_json::to_vec_pretty(evidence)
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
        // A hard link publishes the fully synced document and fails rather
        // than replacing an immutable prior fact — exactly-once holds on
        // every platform (fs::rename would silently replace on Windows).
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

    /// Reads back an existing evidence document; `None` when absent. A
    /// publication that hits an existing id returns the stored fact instead
    /// of an error at this layer (the exactly-once decision belongs to the
    /// caller that compares what it wanted to publish with what is stored).
    pub fn read(&self, evidence_id: &str) -> io::Result<Option<ProductionEvidenceV01>> {
        let path = self.path_for(evidence_id)?;
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

    /// Lists every evidence id currently stored (identity listing only —
    /// bodies are read individually; kind/status aggregation belongs to the
    /// consumer, per the 012 data stance on minimal summaries). A store
    /// that never published anything lists as empty — the absent directory
    /// is the honest empty state, not an error.
    pub fn list_ids(&self) -> io::Result<Vec<String>> {
        let mut ids = Vec::new();
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(ids),
            Err(error) => return Err(error),
        };
        for entry in entries {
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

    /// Lists the evidence ids published by one Local Resolution run (the
    /// `sourceRef.localResolutionId` back-reference). The Build Record
    /// evidence summary consumes this at execution time — the resolve Done
    /// payload is long gone by then, while the evidence facts persist.
    pub fn list_by_local_resolution(&self, local_resolution_id: &str) -> io::Result<Vec<String>> {
        let mut ids = Vec::new();
        for id in self.list_ids()? {
            if let Some(document) = self.read(&id)? {
                if document.source_ref.local_resolution_id.as_deref() == Some(local_resolution_id) {
                    ids.push(id);
                }
            }
        }
        Ok(ids)
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
            "invalid production evidence id",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("vua-evidence-{tag}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample(id: &str) -> ProductionEvidenceV01 {
        ProductionEvidenceV01::new_unresolved(
            id,
            EvidenceKind::MissingAsset,
            EvidenceSubject {
                ref_: "booth:1234567".into(),
                label: Some("Sailor Uniform Set".into()),
            },
            "2026-09-08T06:30:00.000Z",
            "no entry with role=original exists and no filesystem path resolves",
            EvidenceSourceRef::from_local_resolution("0198a7b3-9f8e-7d6c-5b4a-321098765432"),
        )
    }

    #[test]
    fn publish_is_exactly_once_and_read_back_is_faithful() {
        let root = unique_root("once");
        let store = EvidenceStore::new(&root);
        let evidence = sample("0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8b");
        let path = store.publish(&evidence).unwrap();
        assert!(path.exists());
        // Exactly once: the stored fact cannot be rewritten.
        let rewritten = sample("0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8b");
        assert!(store.publish(&rewritten).is_err());
        // Faithful read-back.
        let read = store.read(&evidence.evidence_id).unwrap().unwrap();
        assert_eq!(read, evidence);
        assert_eq!(read.resolution, None, "unresolved serializes as null");
        assert_eq!(
            read.source_ref.local_resolution_id.as_deref(),
            Some("0198a7b3-9f8e-7d6c-5b4a-321098765432")
        );
    }

    #[test]
    fn read_of_absent_evidence_is_none_and_invalid_ids_are_typed() {
        let store = EvidenceStore::new(unique_root("absent"));
        assert_eq!(
            store.read("0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8b").unwrap(),
            None
        );
        assert!(store.path_for("../escape").is_err());
        assert!(store.path_for("").is_err());
    }

    #[test]
    fn list_ids_lists_every_published_identity_sorted() {
        let store = EvidenceStore::new(unique_root("list"));
        for id in [
            "0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8c",
            "0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8b",
        ] {
            store.publish(&sample(id)).unwrap();
        }
        assert_eq!(
            store.list_ids().unwrap(),
            [
                "0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8b",
                "0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8c"
            ]
        );
    }

    #[test]
    fn list_by_local_resolution_filters_on_the_source_back_reference() {
        let store = EvidenceStore::new(unique_root("by-resolution"));
        let resolution = "0198a7b3-9f8e-7d6c-5b4a-321098765432";
        let mut mine = sample("0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8d");
        mine.source_ref = EvidenceSourceRef::from_local_resolution(resolution);
        let mut other = sample("0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8e");
        other.source_ref =
            EvidenceSourceRef::from_local_resolution("0198a7b3-0000-7d6c-5b4a-321098765432");
        let mut task_born = sample("0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8f");
        task_born.source_ref = EvidenceSourceRef::from_task_correlation("corr-1");
        for evidence in [mine, other, task_born] {
            store.publish(&evidence).unwrap();
        }
        assert_eq!(
            store.list_by_local_resolution(resolution).unwrap(),
            ["0198a7b3-1c2d-7e4f-8a5b-3c2d1e0f9a8d"]
        );
        assert!(store
            .list_by_local_resolution("0198a7b3-aaaa-7d6c-5b4a-321098765432")
            .unwrap()
            .is_empty());
    }
}
