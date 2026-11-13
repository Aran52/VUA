//! The VUA-native project identity file (user rulings 2026-09-09, items
//! 7 and 12): a migrated/imported project carries `.vua/project.json` as
//! the folder's VUA-unique marker, and that same file holds the user note
//! (displayed only in the project list).
//!
//! Three artifacts already live in a project's `.vua/` directory; this is
//! the identity/metadata one:
//!
//! - `project.lock` — runtime exclusion gate ([`crate::project_lock`]);
//! - `pending-mutation.json` — crash-evidence marker ([`crate::project_lock`]);
//! - `source.json` — import-copy source link (proposal 014);
//! - `project.json` — **this file**: the VUA-native declaration plus the
//!   user note.
//!
//! Semantics the detection face reports:
//!
//! - absent — no identity file: not a VUA-native project (its `.vua/` may
//!   still exist holding only lock artifacts, which is why "has `.vua/`"
//!   alone is not the native marker);
//! - present — a parseable identity at the current version;
//! - unreadable — the file exists but cannot be parsed (or carries an
//!   unknown version): itself evidence, never silently reported as clean.
//!
//! Note scope (ruling 12): the note is stored here and shown only in the
//! project list; setting one on a project without an identity file is
//! refused — the note feature presupposes the VUA-native declaration.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Schema version of the `project.json` identity document.
pub const IDENTITY_SCHEMA_VERSION: u32 = 1;
/// The identity document lives beside the lock artifacts.
pub const IDENTITY_FILE_NAME: &str = "project.json";

/// The VUA-native identity of one project: when it became VUA-native and
/// the user note attached to it (ruling 12: list-display only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VuaProjectIdentityV1 {
    pub identity_version: u32,
    /// RFC 3339 at the moment the project was marked VUA-native
    /// (import-copy completion; a future migration command reuses this).
    pub marked_at: String,
    /// User note. `None` = no note set (the field stays present so the
    /// document shape is stable).
    #[serde(default)]
    pub note: Option<String>,
}

/// Reading an identity yields one of three findings; an unreadable
/// identity file is itself evidence and must not be reported as "absent".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VuaIdentity {
    /// No identity file — not a VUA-native project.
    Absent,
    /// A parseable identity at the current schema version.
    Present(VuaProjectIdentityV1),
    /// The file exists but cannot be parsed, or carries an unknown
    /// identity version.
    Unreadable,
}

impl VuaIdentity {
    /// The user note when one is readable; `None` otherwise. The read
    /// face shows the note only for a `present` identity.
    pub fn note(&self) -> Option<&str> {
        match self {
            VuaIdentity::Present(identity) => identity.note.as_deref(),
            VuaIdentity::Absent | VuaIdentity::Unreadable => None,
        }
    }
}

fn identity_path(project_root: &Path) -> PathBuf {
    project_root.join(".vua").join(IDENTITY_FILE_NAME)
}

/// Reads the VUA-native identity for `project_root`. Absent on a missing
/// file; every other read failure (permission, directory, garbage,
/// unknown version) is `Unreadable` — never silently "absent".
pub fn read_identity(project_root: &Path) -> VuaIdentity {
    let raw = match fs::read_to_string(identity_path(project_root)) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return VuaIdentity::Absent,
        Err(_) => return VuaIdentity::Unreadable,
        Ok(raw) => raw,
    };
    match serde_json::from_str::<VuaProjectIdentityV1>(&raw) {
        Ok(identity) if identity.identity_version == IDENTITY_SCHEMA_VERSION => {
            VuaIdentity::Present(identity)
        }
        _ => VuaIdentity::Unreadable,
    }
}

/// Writes (or overwrites) the identity document, creating `.vua/` when
/// needed. Callers own the semantics — `mark` below is the import-copy
/// first-marking; a future migration command reuses this primitive.
pub fn write_identity(project_root: &Path, identity: &VuaProjectIdentityV1) -> io::Result<()> {
    let dir = project_root.join(".vua");
    fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(identity)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(identity_path(project_root), json)
}

/// First-marks a project VUA-native (import-copy completion). If an
/// identity already exists its note is preserved; otherwise the note is
/// empty — marking never invents a note.
pub fn mark_vua_native(project_root: &Path, marked_at: &str) -> io::Result<VuaProjectIdentityV1> {
    let note = match read_identity(project_root) {
        VuaIdentity::Present(existing) => existing.note,
        VuaIdentity::Absent | VuaIdentity::Unreadable => None,
    };
    let identity = VuaProjectIdentityV1 {
        identity_version: IDENTITY_SCHEMA_VERSION,
        marked_at: marked_at.to_owned(),
        note,
    };
    write_identity(project_root, &identity)?;
    Ok(identity)
}

/// Why a note update was refused.
#[derive(Debug)]
pub enum SetNoteError {
    /// No identity file: the note presupposes the VUA-native declaration
    /// (a non-native project has nothing to attach the note to).
    NotVuaNative,
    /// The identity file exists but cannot be parsed — refusing to
    /// overwrite unreadable evidence with a blind rewrite.
    Unreadable,
    Io(io::Error),
}

impl std::fmt::Display for SetNoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetNoteError::NotVuaNative => {
                write!(
                    f,
                    "project has no VUA identity file; notes attach to VUA-native projects only"
                )
            }
            SetNoteError::Unreadable => {
                write!(
                    f,
                    "VUA identity file is unreadable; resolve it before editing the note"
                )
            }
            SetNoteError::Io(error) => write!(f, "VUA identity io error: {error}"),
        }
    }
}

impl From<io::Error> for SetNoteError {
    fn from(error: io::Error) -> Self {
        SetNoteError::Io(error)
    }
}

/// Sets (or clears, with `None`) the user note on a VUA-native project.
/// The identity document is the only note store (ruling 12).
pub fn set_note(
    project_root: &Path,
    note: Option<&str>,
) -> Result<VuaProjectIdentityV1, SetNoteError> {
    let identity = match read_identity(project_root) {
        VuaIdentity::Present(identity) => identity,
        VuaIdentity::Absent => return Err(SetNoteError::NotVuaNative),
        VuaIdentity::Unreadable => return Err(SetNoteError::Unreadable),
    };
    let updated = VuaProjectIdentityV1 {
        identity_version: IDENTITY_SCHEMA_VERSION,
        marked_at: identity.marked_at,
        note: note.map(str::to_owned),
    };
    write_identity(project_root, &updated)?;
    Ok(updated)
}
