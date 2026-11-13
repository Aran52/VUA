//! M6 EAC package (proposal 006 R2): the termination allowlist data face.
//!
//! The list starts EMPTY and can only grow from real-machine verification
//! evidence (R2: entries come only from real-machine evidence — the probe
//! itself never self-attests an entry). Every entry carries the four
//! required elements plus the evidence reference and the release-notes
//! marker (R8). This module is deliberately boring: load, validate
//! completely (a half-valid entry is a load error, never a weakened
//! entry), and match path patterns. Creating an entry is a human+evidence
//! workflow outside this crate's authority.

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const EAC_ALLOWLIST_SCHEMA_VERSION: &str = "vua.eac-allowlist/v0.1";

/// One allowlist entry: the four R2 elements + provenance. Immutable by
/// construction — a changed list is a new file (and a new release note).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AllowlistEntryV01 {
    pub process_name: String,
    pub expected_path_pattern: String,
    pub publisher_evidence: String,
    pub reason: String,
    pub evidence_ref: String,
    pub added_at: String,
    pub release_notes: String,
}

/// The parsed allowlist document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowlistV01 {
    pub schema_version: String,
    pub entries: Vec<AllowlistEntryV01>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowlistLoadError {
    pub code: &'static str,
    pub detail: String,
}

impl AllowlistLoadError {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

/// Load-error codes (vua.eac_allowlist.*).
pub mod codes {
    pub const SCHEMA_INVALID: &str = "vua.eac_allowlist.schema_invalid";
    pub const SCHEMA_VERSION_MISMATCH: &str = "vua.eac_allowlist.schema_version_mismatch";
    pub const ELEMENT_INCOMPLETE: &str = "vua.eac_allowlist.element_incomplete";
    pub const NAME_NOT_LOWER_CASE: &str = "vua.eac_allowlist.name_not_lower_case";
    pub const PATTERN_INVALID: &str = "vua.eac_allowlist.pattern_invalid";
}

/// Validates one entry completely: all four R2 elements present and
/// non-empty, the provenance fields present, the name lower-case, and the
/// pattern well-formed. A half-valid entry is a load error, never a
/// weakened entry.
fn validate_entry(index: usize, entry: &AllowlistEntryV01) -> Result<(), AllowlistLoadError> {
    let at = |detail: String| {
        AllowlistLoadError::new(
            codes::ELEMENT_INCOMPLETE,
            format!("entries[{index}]: {detail}"),
        )
    };
    for (field, value) in [
        ("processName", &entry.process_name),
        ("expectedPathPattern", &entry.expected_path_pattern),
        ("publisherEvidence", &entry.publisher_evidence),
        ("reason", &entry.reason),
        ("evidenceRef", &entry.evidence_ref),
        ("addedAt", &entry.added_at),
        ("releaseNotes", &entry.release_notes),
    ] {
        if value.trim().is_empty() {
            return Err(at(format!(
                "{field} is required (R2 four elements + provenance)"
            )));
        }
    }
    if entry.process_name != entry.process_name.to_ascii_lowercase() {
        return Err(AllowlistLoadError::new(
            codes::NAME_NOT_LOWER_CASE,
            format!("entries[{index}]: processName must be lower-case"),
        ));
    }
    if entry.expected_path_pattern.contains('*') || entry.expected_path_pattern.contains('?') {
        return Err(AllowlistLoadError::new(
            codes::PATTERN_INVALID,
            format!("entries[{index}]: expectedPathPattern must be a non-wildcard literal or prefix pattern"),
        ));
    }
    Ok(())
}

/// Loads and fully validates the allowlist document. The empty list is the
/// designed initial state (R2).
pub fn load_allowlist(text: &str) -> Result<AllowlistV01, AllowlistLoadError> {
    let document: AllowlistV01 = serde_json::from_str(text).map_err(|error| {
        AllowlistLoadError::new(
            codes::SCHEMA_INVALID,
            format!("allowlist is not a valid v0.1 document: {error}"),
        )
    })?;
    if document.schema_version != EAC_ALLOWLIST_SCHEMA_VERSION {
        return Err(AllowlistLoadError::new(
            codes::SCHEMA_VERSION_MISMATCH,
            format!(
                "expected {}, found {}",
                EAC_ALLOWLIST_SCHEMA_VERSION, document.schema_version
            ),
        ));
    }
    for (index, entry) in document.entries.iter().enumerate() {
        validate_entry(index, entry)?;
    }
    Ok(document)
}

/// Finds the entry matching a lower-cased process name, if any.
pub fn find_entry<'a>(
    allowlist: &'a AllowlistV01,
    process_name: &str,
) -> Option<&'a AllowlistEntryV01> {
    let lower = process_name.to_ascii_lowercase();
    allowlist
        .entries
        .iter()
        .find(|entry| entry.process_name == lower)
}

/// Case-insensitive prefix-or-literal path matching: `Some(remaining)`
/// when `path` starts with `pattern` (the remainder is the file name
/// part), `None` otherwise. A pattern that contains wildcards or does not
/// end with a separator cannot be a prefix pattern — `None` for every
/// path (the loader refuses such patterns anyway).
pub fn path_pattern_matches(pattern: &str, path: &str) -> Option<()> {
    if pattern.contains('*') || pattern.contains('?') {
        return None;
    }
    let ends_with_separator = pattern.ends_with('\\') || pattern.ends_with('/');
    if !ends_with_separator {
        // A literal full-path pattern must match exactly (case-insensitive).
        return path.eq_ignore_ascii_case(pattern).then_some(());
    }
    let normalized_pattern = pattern.replace('/', "\\");
    let normalized_path = path.replace('/', "\\");
    if normalized_path.len() >= normalized_pattern.len()
        && normalized_path[..normalized_pattern.len()].eq_ignore_ascii_case(&normalized_pattern)
    {
        Some(())
    } else {
        None
    }
}

/// Convenience: does `path` sit under `dir` (case-insensitive)?
pub fn path_under_dir(path: &Path, dir: &str) -> bool {
    path_pattern_matches(dir, &path.to_string_lossy()).is_some()
}
