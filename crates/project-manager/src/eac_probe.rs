//! M6 EAC package (proposal 006 R1a, accepted by user ruling U1 with the
//! eight-point approval semantics): read-only EAC conflict detection — the
//! DEFAULT and, in this slice, the ONLY capability of the EAC face.
//!
//! Discipline (006 R1–R8 + approval points):
//! - one targeted pass over the process table, never a scan of user data,
//!   never a write (R1a "read-only detection: evidence + readiness +
//!   human path");
//! - an active VRChat process IS an active session — the conservative R4
//!   rule (from-wide refusal) is a *finding* here; the refusal itself
//!   belongs to the termination face, which does not exist in v0.1;
//! - approval point 2: with the allowlist empty and no real-machine
//!   evidence, the termination capability reports **unavailable** — the
//!   frozen open set has no other state in v0.1;
//! - R8: CI runs on synthetic process fixtures (structurally
//!   representative, zero real EAC interaction); the real-machine probe is
//!   the explicitly ignored manual test.
//!
//! Process enumeration is injectable ([`ProcessSnapshotSource`]) so every
//! unit test runs on synthetic fixtures; the OS-backed source lives in
//! [`os_source`] and is exercised only by the ignored manual test. Shapes
//! are pinned by `schemas/eac-probe/v0.1/probe.schema.json`.

use serde::Serialize;

use vua_orchestrator::{Clock, FindingSeverity, ManagerDiagnostic, PRODUCTION_TARGET};

pub const EAC_PROBE_SCHEMA_VERSION: &str = "vua.eac-probe/v0.1";

/// Read-face diagnostics (vua.eac_probe.*).
pub mod codes {
    pub const GUIDANCE_CLOSE_GAME_FIRST: &str = "vua.eac_probe.guidance_close_game_first";
    pub const GUIDANCE_ALLOWLIST_UNAVAILABLE: &str = "vua.eac_probe.guidance_allowlist_unavailable";
    pub const GUIDANCE_NO_EAC_FOUND: &str = "vua.eac_probe.guidance_no_eac_found";
    pub const ENUMERATION_FAILED: &str = "vua.eac_probe.enumeration_failed";
}

/// The process classes the probe distinguishes. Classification is by
/// lower-cased executable name; anything not in the table is simply not a
/// finding (the probe never speculates about unrelated processes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessKind {
    EacLoader,
    EacService,
    VrchatGame,
    VrchatOverlay,
}

impl ProcessKind {
    fn classify(image_name: &str) -> Option<Self> {
        let name = image_name.to_ascii_lowercase();
        match name.as_str() {
            "easyanticheat.exe" | "easyanticheat_eos.exe" => Some(ProcessKind::EacLoader),
            "easyanticheatservice.exe" => Some(ProcessKind::EacService),
            "vrchat.exe" => Some(ProcessKind::VrchatGame),
            _ => None,
        }
    }
}

/// One EAC/VRChat-relevant process finding. `path` is null when the probe
/// could not read it — an unreadable path is never guessed (006 R3 keeps
/// name/pid/path/identity verification for the termination face; the read
/// face reports exactly what it saw).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessFinding {
    pub name: String,
    pub pid: u32,
    pub kind: ProcessKind,
    pub path: Option<String>,
}

/// What the probe saw, with the R4 session rule applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Readiness {
    pub conclusion: ReadinessConclusion,
    pub guidance_code: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessConclusion {
    NoEacFound,
    EacActiveWithVrchatSession,
    EacResidualNoSession,
}

/// The honest termination-capability face. v0.1 freezes exactly one state:
/// unavailable while the allowlist is empty and no real-machine evidence
/// exists (approval point 2 — stronger than a closed switch; the interface
/// must present it as 不可用). The state open set grows only through R2
/// (real-machine evidence produces the first allowlist entry) and the R1b
/// termination face with its own confirmation chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminationCapability {
    pub state: &'static str,
    pub reason: &'static str,
}

/// Versioned probe snapshot pinned by
/// `schemas/eac-probe/v0.1/probe.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EacProbeSnapshotV01 {
    pub schema_version: &'static str,
    pub captured_at: String,
    pub production_target: &'static str,
    pub processes: Vec<ProcessFinding>,
    pub vrchat_session_active: bool,
    pub eac_present: bool,
    pub readiness: Readiness,
    pub termination_capability: TerminationCapability,
    pub diagnostics: Vec<ManagerDiagnostic>,
}

/// One process-table entry as the snapshot source reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEntry {
    pub image_name: String,
    pub pid: u32,
    pub path: Option<String>,
}

/// The injectable process-table source. Implementations MUST be read-only
/// and MUST NOT terminate, suspend, or inject into anything (R1a/R7).
pub trait ProcessSnapshotSource {
    /// The full process table snapshot. Names are image file names
    /// (e.g. "vrchat.exe").
    fn snapshot(&self) -> std::io::Result<Vec<ProcessEntry>>;
}

/// Assembles the read-only EAC conflict snapshot from a process snapshot.
/// Deterministic for a given snapshot: findings are sorted by (kind, pid).
pub fn probe_eac(source: &dyn ProcessSnapshotSource, clock: &dyn Clock) -> EacProbeSnapshotV01 {
    let mut diagnostics = Vec::new();
    let entries = match source.snapshot() {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(ManagerDiagnostic {
                code: codes::ENUMERATION_FAILED,
                severity: FindingSeverity::Error,
                detail: format!("process enumeration failed: {error}"),
            });
            return empty_snapshot(clock, diagnostics);
        }
    };

    let mut processes: Vec<ProcessFinding> = entries
        .into_iter()
        .filter_map(|entry| {
            ProcessKind::classify(&entry.image_name).map(|kind| ProcessFinding {
                name: entry.image_name,
                pid: entry.pid,
                kind,
                path: entry.path,
            })
        })
        .collect();
    // OS order is arbitrary; a snapshot must be deterministic.
    processes.sort_by_key(|finding| (finding.kind, finding.pid));

    let vrchat_session_active = processes
        .iter()
        .any(|finding| finding.kind == ProcessKind::VrchatGame);
    let eac_present = processes.iter().any(|finding| {
        matches!(
            finding.kind,
            ProcessKind::EacLoader | ProcessKind::EacService
        )
    });

    let readiness = if !eac_present {
        Readiness {
            conclusion: ReadinessConclusion::NoEacFound,
            guidance_code: codes::GUIDANCE_NO_EAC_FOUND,
        }
    } else if vrchat_session_active {
        Readiness {
            conclusion: ReadinessConclusion::EacActiveWithVrchatSession,
            guidance_code: codes::GUIDANCE_CLOSE_GAME_FIRST,
        }
    } else {
        Readiness {
            conclusion: ReadinessConclusion::EacResidualNoSession,
            guidance_code: codes::GUIDANCE_ALLOWLIST_UNAVAILABLE,
        }
    };

    EacProbeSnapshotV01 {
        schema_version: EAC_PROBE_SCHEMA_VERSION,
        captured_at: clock.now_rfc3339(),
        production_target: PRODUCTION_TARGET,
        processes,
        vrchat_session_active,
        eac_present,
        readiness,
        // v0.1 freezes the single honest state: the allowlist is empty and
        // no real-machine evidence exists, so termination is unavailable
        // (approval point 2). This is a fact about the capability, not a
        // toggle this crate could flip.
        termination_capability: TerminationCapability {
            state: "unavailable",
            reason: "allowlist_empty_no_machine_evidence",
        },
        diagnostics,
    }
}

fn empty_snapshot(clock: &dyn Clock, diagnostics: Vec<ManagerDiagnostic>) -> EacProbeSnapshotV01 {
    EacProbeSnapshotV01 {
        schema_version: EAC_PROBE_SCHEMA_VERSION,
        captured_at: clock.now_rfc3339(),
        production_target: PRODUCTION_TARGET,
        processes: Vec::new(),
        vrchat_session_active: false,
        eac_present: false,
        readiness: Readiness {
            conclusion: ReadinessConclusion::NoEacFound,
            guidance_code: codes::GUIDANCE_NO_EAC_FOUND,
        },
        termination_capability: TerminationCapability {
            state: "unavailable",
            reason: "allowlist_empty_no_machine_evidence",
        },
        diagnostics,
    }
}

// --- OS-backed source (Windows; exercised by the ignored manual test) ---

pub mod os_source {
    //! The real process table via the Windows Toolhelp snapshot. Read-only:
    //! creating a snapshot and reading entries touches nothing.

    use super::{ProcessEntry, ProcessSnapshotSource};

    /// Windows Toolhelp32-backed snapshot source.
    #[derive(Debug, Default, Clone, Copy)]
    pub struct ToolhelpProcessSource;

    impl ProcessSnapshotSource for ToolhelpProcessSource {
        fn snapshot(&self) -> std::io::Result<Vec<ProcessEntry>> {
            snapshot_toolhelp()
        }
    }

    #[cfg(windows)]
    fn snapshot_toolhelp() -> std::io::Result<Vec<ProcessEntry>> {
        use std::mem::zeroed;
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        };

        let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }

        let mut entries = Vec::new();
        let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        unsafe {
            if Process32FirstW(handle, &mut entry) != 0 {
                loop {
                    let len = entry
                        .szExeFile
                        .iter()
                        .position(|character| *character == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                    if !name.is_empty() {
                        entries.push(ProcessEntry {
                            image_name: name,
                            pid: entry.th32ProcessID,
                            // Path is intentionally not probed here: reading
                            // another process's image path requires opening
                            // its handle (QueryFullProcessImageNameW), which
                            // is the termination face's verification duty
                            // (006 R3), not the read face's. The read face
                            // reports null and stays minimal.
                            path: None,
                        });
                    }
                    if Process32NextW(handle, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(handle);
        }
        Ok(entries)
    }

    #[cfg(not(windows))]
    fn snapshot_toolhelp() -> std::io::Result<Vec<ProcessEntry>> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "the EAC probe requires Windows",
        ))
    }
}
