//! M6 EAC package (proposal 006 R3): the candidate re-verification
//! primitive — the read-only half of the termination face.
//!
//! R3: before every termination, the candidate is re-verified against the
//! allowlist entry — name, PID, path, and signature; ANY element that
//! cannot be re-verified refuses the termination (R3: 核验失败不算发现，
//! 不得猜测补齐). This module performs the read-only checks that can run
//! without the termination privilege:
//!
//! - name match against the entry (from the process snapshot);
//! - executable path read (OpenProcess with
//!   PROCESS_QUERY_LIMITED_INFORMATION + QueryFullProcessImageNameW — a
//!   read-only query) and case-insensitive pattern match;
//! - signature: **unverified in v0.1** — the WinVerifyTrust binding lands
//!   with the termination slice; until then the overall verdict is
//!   honestly Refused (SignatureUnverified), because R3 refuses when any
//!   element cannot be verified. The refusal is the DESIGNED v0.1
//!   outcome, not a defect.
//!
//! v0.1 never opens a process with termination rights and never calls
//! TerminateProcess — that is the next slice, behind its own confirmation
//! chain (R5/R6).

use serde::Serialize;

use crate::eac_allowlist::{path_pattern_matches, AllowlistEntryV01};
use crate::eac_probe::ProcessSnapshotSource;

/// Verification-check codes (vua.eac_verify.*).
pub mod codes {
    pub const NAME_MISMATCH: &str = "vua.eac_verify.name_mismatch";
    pub const PATH_MISMATCH: &str = "vua.eac_verify.path_mismatch";
    pub const PATH_UNREADABLE: &str = "vua.eac_verify.path_unreadable";
    pub const PROCESS_NOT_FOUND: &str = "vua.eac_verify.process_not_found";
    pub const SIGNATURE_UNVERIFIED: &str = "vua.eac_verify.signature_unverified";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureState {
    /// WinVerifyTrust accepted the file (status 0).
    Verified,
    /// No signature present, or the trust provider refused/failed — R3
    /// treats this as "cannot be verified" and refuses.
    Unverified,
}

/// One check with its outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationCheck {
    pub code: &'static str,
    pub passed: bool,
    pub detail: String,
}

/// The overall verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Verified,
    Refused,
}

/// The full re-verification report for one candidate PID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateVerificationV01 {
    pub schema_version: &'static str,
    pub pid: u32,
    pub verdict: Verdict,
    pub signature_state: SignatureState,
    pub checks: Vec<VerificationCheck>,
}

pub const EAC_VERIFY_SCHEMA_VERSION: &str = "vua.eac_verify/v0.1";

/// Reads the executable path of `pid` (read-only query). Windows-only;
/// other targets report unreadable (the product is Windows-first). The
/// public wrapper serves the W25 window's B2b run: the first real
/// allowlist entry is drafted from this path (R2 — evidence, not fixture).
pub fn read_process_image_path_readonly(pid: u32) -> Option<String> {
    read_process_image_path(pid)
}

fn read_process_image_path(pid: u32) -> Option<String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return None;
        }
        let mut buffer = [0u16; 2048];
        let mut length = buffer.len() as u32;
        let ok = unsafe {
            windows_sys::Win32::System::Threading::QueryFullProcessImageNameW(
                handle,
                0,
                buffer.as_mut_ptr(),
                &mut length,
            )
        };
        unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
        if ok == 0 || length == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buffer[..length as usize]))
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        None
    }
}

/// Re-verifies the candidate `pid` against `entry` (R3). The candidate
/// must still be in the process snapshot (a vanished candidate is a
/// refusal, not a success). Every check lands in the report; the verdict
/// is Verified only when ALL checks pass — in v0.1 the signature check is
/// always unverified, so the designed verdict is Refused until the
/// termination slice wires WinVerifyTrust.
pub fn verify_candidate(
    pid: u32,
    entry: &AllowlistEntryV01,
    source: &dyn ProcessSnapshotSource,
) -> CandidateVerificationV01 {
    let mut checks = Vec::new();

    // Check 1: the candidate is still in the table under the entry's name.
    let snapshot_entry = source
        .snapshot()
        .ok()
        .and_then(|entries| entries.into_iter().find(|candidate| candidate.pid == pid));
    let name_matched = match &snapshot_entry {
        Some(candidate) if candidate.image_name.to_ascii_lowercase() == entry.process_name => {
            checks.push(VerificationCheck {
                code: codes::NAME_MISMATCH,
                passed: true,
                detail: format!("image name matches {}", entry.process_name),
            });
            true
        }
        Some(candidate) => {
            checks.push(VerificationCheck {
                code: codes::NAME_MISMATCH,
                passed: false,
                detail: format!(
                    "image name {} does not match {}",
                    candidate.image_name, entry.process_name
                ),
            });
            false
        }
        None => {
            checks.push(VerificationCheck {
                code: codes::PROCESS_NOT_FOUND,
                passed: false,
                detail: format!("pid {pid} is no longer in the process table"),
            });
            false
        }
    };

    // Check 2: the executable path reads and matches the entry's pattern.
    let image_path: Option<String> = snapshot_entry
        .as_ref()
        .and_then(|candidate| candidate.path.clone())
        .or_else(|| read_process_image_path(pid));
    let path_matched = match &image_path {
        Some(path) => {
            let matches = path_pattern_matches(&entry.expected_path_pattern, path).is_some();
            checks.push(VerificationCheck {
                code: codes::PATH_MISMATCH,
                passed: matches,
                detail: if matches {
                    format!("{path} matches the expected path pattern")
                } else {
                    format!(
                        "{path} does not match the expected path pattern {}",
                        entry.expected_path_pattern
                    )
                },
            });
            matches
        }
        None => {
            checks.push(VerificationCheck {
                code: codes::PATH_UNREADABLE,
                passed: false,
                detail: "the executable path could not be read — refusing (never guessed)"
                    .to_owned(),
            });
            false
        }
    };
    // Check 3: the signature, re-verified via WinVerifyTrust (all-or-
    // nothing, no UI, no revocation — the publisher evidence lives in the
    // allowlist entry; this is the re-verification, R3).
    let (signature_state, signature_detail) = match &image_path {
        Some(path) => verify_signature_windows(path),
        None => (
            SignatureState::Unverified,
            "no executable path to verify".to_owned(),
        ),
    };
    let signature_passed = signature_state == SignatureState::Verified;
    checks.push(VerificationCheck {
        code: codes::SIGNATURE_UNVERIFIED,
        passed: signature_passed,
        detail: signature_detail,
    });

    let verdict = if name_matched && path_matched && signature_passed {
        Verdict::Verified
    } else {
        Verdict::Refused
    };

    CandidateVerificationV01 {
        schema_version: EAC_VERIFY_SCHEMA_VERSION,
        pid,
        verdict,
        signature_state,
        checks,
    }
}

/// Test hook: exposes the Windows signature verdict for a path without
/// the full candidate flow (used by the allowlist/verify tests).
/// Windows-only (WinVerifyTrust) AND gated behind `test-hooks` (or
/// `cfg(test)`): a verification primitive must not sit on the default
/// public surface.
#[cfg(all(windows, any(test, feature = "test-hooks")))]
pub fn eac_verify_windows_signature_for_test(image_path: &str) -> (SignatureState, String) {
    verify_signature_windows(image_path)
}

#[cfg(windows)]
fn verify_signature_windows(image_path: &str) -> (SignatureState, String) {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::{
        CRYPT_E_SECURITY_SETTINGS, TRUST_E_NOSIGNATURE, TRUST_E_PROVIDER_UNKNOWN,
    };
    use windows_sys::Win32::Security::WinTrust::{
        WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0,
        WINTRUST_FILE_INFO, WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE,
        WTD_STATEACTION_VERIFY, WTD_UI_NONE,
    };

    let wide: Vec<u16> = image_path
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut file_info = WINTRUST_FILE_INFO {
        cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: wide.as_ptr(),
        hFile: std::ptr::null_mut(),
        pgKnownSubject: std::ptr::null_mut(),
    };
    let mut trust_data = WINTRUST_DATA {
        cbStruct: size_of::<WINTRUST_DATA>() as u32,
        pPolicyCallbackData: std::ptr::null_mut(),
        pSIPClientData: std::ptr::null_mut(),
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 {
            pFile: &mut file_info as *mut WINTRUST_FILE_INFO,
        },
        dwStateAction: WTD_STATEACTION_VERIFY,
        hWVTStateData: std::ptr::null_mut(),
        pwszURLReference: std::ptr::null_mut(),
        dwProvFlags: 0,
        dwUIContext: 0,
        pSignatureSettings: std::ptr::null_mut(),
    };
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;

    let status = unsafe {
        WinVerifyTrust(
            std::ptr::null_mut(),
            &mut action as *mut _,
            &mut trust_data as *mut WINTRUST_DATA as *mut core::ffi::c_void,
        )
    };
    // Release any state the VERIFY step opened (required by the contract
    // whenever dwStateAction is WTD_STATEACTION_VERIFY).
    trust_data.dwStateAction = WTD_STATEACTION_CLOSE;
    unsafe {
        WinVerifyTrust(
            std::ptr::null_mut(),
            &mut action as *mut _,
            &mut trust_data as *mut WINTRUST_DATA as *mut core::ffi::c_void,
        )
    };

    match status {
        0 => (
            SignatureState::Verified,
            format!("WinVerifyTrust accepted {image_path}"),
        ),
        TRUST_E_NOSIGNATURE => (
            SignatureState::Unverified,
            format!("no signature present: 0x{status:08X}"),
        ),
        TRUST_E_PROVIDER_UNKNOWN | CRYPT_E_SECURITY_SETTINGS => (
            SignatureState::Unverified,
            format!("trust provider/configuration refused: 0x{status:08X}"),
        ),
        other => (
            SignatureState::Unverified,
            format!("signature verification failed: 0x{other:08X}"),
        ),
    }
}

#[cfg(not(windows))]
fn verify_signature_windows(image_path: &str) -> (SignatureState, String) {
    (
        SignatureState::Unverified,
        format!("signature verification requires Windows: {image_path}"),
    )
}
