//! Cross-profile exclusive project mutation lock and the versioned
//! unfinished-mutation marker (instance-identity ADR, decision 7).
//!
//! Two independent artifacts live in the project's `.vua/` directory:
//!
//! - `project.lock` — the runtime exclusion gate. Exclusion comes from an
//!   OS byte-range lock (`LockFileEx`, fail-immediately), NOT from file
//!   presence: the OS releases the lock when the holder process dies, so a
//!   crash can never wedge the project. Windows byte-range locks are
//!   mandatory, so the lock sits at an offset far beyond the diagnostic
//!   envelope — the envelope at bytes 0..N stays readable, the lock byte
//!   does not.
//! - `pending-mutation.json` — the crash-evidence marker. Written when a
//!   mutating task starts, removed when it finishes; a leftover marker
//!   means the previous mutation never confirmed completion, so the next
//!   channel must Inspect the project before mutating it.
//!
//! SQLite leases remain the per-profile source of truth for task
//! generations and recovery; this module closes the cross-profile gap the
//! leases cannot see.

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Schema version of the `project.lock` diagnostic envelope.
pub const PROJECT_LOCK_SCHEMA_VERSION: u32 = 1;
/// Schema version of the `pending-mutation.json` marker.
pub const MUTATION_MARKER_SCHEMA_VERSION: u32 = 1;

pub const LOCK_FILE_NAME: &str = "project.lock";
pub const MARKER_FILE_NAME: &str = "pending-mutation.json";

/// Who holds a project mutation lock. Purely diagnostic payload —
/// exclusion is enforced by the OS byte-range lock, never by this file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockHolder {
    pub channel: String,
    pub profile: String,
    pub pid: u32,
    pub instance_id: String,
    /// RFC 3339 at acquisition, for humans reading the file.
    pub acquired_at: String,
}

/// Versioned diagnostic envelope stored in `project.lock`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockEnvelopeV1 {
    pub lock_version: u32,
    pub holder: LockHolder,
}

/// What kind of mutation a pending marker stands for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationMarkerV1 {
    pub marker_version: u32,
    /// e.g. `material_intake`, `recipe_apply` — free-form task family.
    pub mutation_kind: String,
    pub holder: LockHolder,
}

/// Reading a marker yields one of three findings; an unreadable marker is
/// itself evidence of a half-written mutation, so it must not be reported
/// as "clean".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingMutation {
    /// No marker file.
    None,
    /// A parseable leftover marker.
    Leftover(MutationMarkerV1),
    /// A marker file exists but cannot be parsed (e.g. crash mid-write).
    Unreadable,
}

#[derive(Debug)]
pub enum ProjectLockError {
    /// Another live instance holds the lock; `previous` is the best-effort
    /// parse of the diagnostic envelope the other holder left.
    Held {
        previous: Option<LockHolder>,
    },
    Io(io::Error),
}

impl std::fmt::Display for ProjectLockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectLockError::Held { previous } => match previous {
                Some(holder) => write!(
                    f,
                    "project mutation lock held by instance {} (pid {})",
                    holder.instance_id, holder.pid
                ),
                None => write!(f, "project mutation lock held by another instance"),
            },
            ProjectLockError::Io(error) => write!(f, "project lock io error: {error}"),
        }
    }
}

impl From<io::Error> for ProjectLockError {
    fn from(error: io::Error) -> Self {
        ProjectLockError::Io(error)
    }
}

fn vua_dir(project_root: &Path) -> PathBuf {
    project_root.join(".vua")
}

fn read_holder(lock_path: &Path) -> Option<LockHolder> {
    let raw = fs::read_to_string(lock_path).ok()?;
    let envelope: LockEnvelopeV1 = serde_json::from_str(&raw).ok()?;
    (envelope.lock_version == PROJECT_LOCK_SCHEMA_VERSION).then_some(envelope.holder)
}

/// An acquired project mutation lock. The guard owns the file handle whose
/// byte-range lock provides the exclusion; dropping it (explicit
/// `release` or unwind) clears the diagnostics and releases the lock.
#[derive(Debug)]
pub struct ProjectLockGuard {
    file: Option<File>,
    lock_path: PathBuf,
    released: bool,
}

impl ProjectLockGuard {
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }

    /// Releases the lock. Idempotent; the Drop path calls the same core.
    pub fn release(mut self) -> io::Result<()> {
        let result = self.release_core();
        self.released = true;
        result
    }

    fn release_core(&mut self) -> io::Result<()> {
        if let Some(mut file) = self.file.take() {
            // Clear diagnostics first: an empty file reads as "nobody
            // holds" even before the OS lock is observed as released.
            file.set_len(0)?;
            release_os_lock(&mut file)?;
            // Dropping `file` here closes the handle; the empty file is
            // deliberately left in place to avoid a delete-vs-open race
            // with a contender that already opened it.
        }
        Ok(())
    }
}

impl Drop for ProjectLockGuard {
    fn drop(&mut self) {
        if !self.released {
            let _ = self.release_core();
        }
    }
}

/// Acquires the cross-profile project mutation lock for `project_root`.
///
/// Fails with [`ProjectLockError::Held`] when another live instance holds
/// the lock. Crash semantics are delegated to the OS: when the holder
/// process dies, its byte-range lock is released and the next acquirer
/// wins — no staleness heuristics.
pub fn acquire_project_lock(
    project_root: &Path,
    holder: LockHolder,
) -> Result<ProjectLockGuard, ProjectLockError> {
    let dir = vua_dir(project_root);
    fs::create_dir_all(&dir)?;
    let lock_path = dir.join(LOCK_FILE_NAME);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;

    match acquire_os_lock(&file) {
        Ok(()) => {}
        Err(LockRefused::Held) => {
            // The OS refused the byte-range lock: someone else holds it.
            // Best-effort diagnostics from whatever the holder wrote.
            return Err(ProjectLockError::Held {
                previous: read_holder(&lock_path),
            });
        }
        Err(LockRefused::Io(error)) => return Err(ProjectLockError::Io(error)),
    }

    // Seek+truncate+rewrite under the lock so contenders never observe a
    // torn envelope (they fail on the lock anyway; this is for readers).
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    let envelope = LockEnvelopeV1 {
        lock_version: PROJECT_LOCK_SCHEMA_VERSION,
        holder,
    };
    serde_json::to_writer_pretty(&mut file, &envelope)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    file.flush()?;

    Ok(ProjectLockGuard {
        file: Some(file),
        lock_path,
        released: false,
    })
}

/// Reads the pending-mutation marker for the inspect-first gate: any
/// channel that finds [`PendingMutation::Leftover`] or
/// [`PendingMutation::Unreadable`] must Inspect the project before its own
/// mutation.
pub fn read_pending_mutation(project_root: &Path) -> PendingMutation {
    let path = vua_dir(project_root).join(MARKER_FILE_NAME);
    let raw = match fs::read_to_string(&path) {
        // Absent means clean; any OTHER read failure (permission, the path
        // being a directory, ...) is evidence we cannot read — the
        // inspect-first gate must treat it as Unreadable, never as clean.
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return PendingMutation::None;
        }
        Err(_) => return PendingMutation::Unreadable,
        Ok(raw) => raw,
    };
    match serde_json::from_str::<MutationMarkerV1>(&raw) {
        Ok(marker) if marker.marker_version == MUTATION_MARKER_SCHEMA_VERSION => {
            PendingMutation::Leftover(marker)
        }
        _ => PendingMutation::Unreadable,
    }
}

/// A begun mutation marker. Removed on `release` or Drop; a crash leaves
/// it behind on purpose — that is the entire point.
pub struct MutationMarkerGuard {
    path: PathBuf,
    removed: bool,
}

impl MutationMarkerGuard {
    pub fn marker_path(&self) -> &Path {
        &self.path
    }

    pub fn release(mut self) -> io::Result<()> {
        let result = fs::remove_file(&self.path);
        self.removed = true;
        result
    }
}

impl Drop for MutationMarkerGuard {
    fn drop(&mut self) {
        if !self.removed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Writes the unfinished-mutation marker. Overwrites any leftover: the
/// caller has already observed it through [`read_pending_mutation`] and
/// decided to proceed (after its Inspect).
pub fn begin_mutation(
    project_root: &Path,
    mutation_kind: &str,
    holder: &LockHolder,
) -> io::Result<MutationMarkerGuard> {
    let dir = vua_dir(project_root);
    fs::create_dir_all(&dir)?;
    let path = dir.join(MARKER_FILE_NAME);
    let marker = MutationMarkerV1 {
        marker_version: MUTATION_MARKER_SCHEMA_VERSION,
        mutation_kind: mutation_kind.to_owned(),
        holder: holder.clone(),
    };
    let json = serde_json::to_string_pretty(&marker)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(&path, json)?;
    Ok(MutationMarkerGuard {
        path,
        removed: false,
    })
}

// --- OS byte-range lock (Windows: LockFileEx; elsewhere unsupported) ---

/// Why an acquire attempt failed: the range was locked by a live holder,
/// or a genuine IO error occurred.
enum LockRefused {
    Held,
    Io(io::Error),
}

/// Windows byte-range locks are mandatory: a lock covering byte 0 would
/// block every read of the diagnostic envelope, including our own. The
/// lock therefore lives at an offset far beyond any envelope content; the
/// envelope stays human-readable at bytes 0..N while the OS lock at
/// `LOCK_OFFSET` provides the exclusion.
#[cfg(windows)]
const LOCK_OFFSET: u32 = 65536;

#[cfg(windows)]
fn acquire_os_lock(file: &File) -> Result<(), LockRefused> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::ERROR_LOCK_VIOLATION;
    use windows_sys::Win32::Storage::FileSystem::{
        LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
    };
    use windows_sys::Win32::System::IO::OVERLAPPED;

    let mut overlapped = OVERLAPPED::default();
    overlapped.Anonymous.Anonymous.Offset = LOCK_OFFSET;
    let status = unsafe {
        LockFileEx(
            file.as_raw_handle(),
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            1,
            0,
            &mut overlapped,
        )
    };
    if status != 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(ERROR_LOCK_VIOLATION as i32) {
        // Documented refusal when the range is already locked.
        Err(LockRefused::Held)
    } else {
        Err(LockRefused::Io(error))
    }
}

#[cfg(windows)]
fn release_os_lock(file: &mut File) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::UnlockFileEx;
    use windows_sys::Win32::System::IO::OVERLAPPED;

    let mut overlapped = OVERLAPPED::default();
    overlapped.Anonymous.Anonymous.Offset = LOCK_OFFSET;
    let status = unsafe { UnlockFileEx(file.as_raw_handle(), 0, 1, 0, &mut overlapped) };
    if status != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(windows))]
fn acquire_os_lock(_file: &File) -> Result<(), LockRefused> {
    Err(LockRefused::Io(io::Error::new(
        io::ErrorKind::Unsupported,
        "project mutation lock requires Windows",
    )))
}

#[cfg(not(windows))]
fn release_os_lock(_file: &mut File) -> io::Result<()> {
    Ok(())
}
