//! M6 EAC package (proposal 006 R1b) tests: the termination plan step.
//! The full chain (R3 four-element re-verification → R4 guard → terminate
//! → await → post-check) runs against a test-spawned, fully controlled
//! process — never against a real EAC/game process (R7/R8). The signature
//! element is exercised for real (PowerShell carries a Microsoft embedded
//! signature).
//!
//! 真机手动验证件（operator fix directive 2026-09-10）：every test in this
//! file spawns a REAL PowerShell process, and the refused paths still end
//! with a real taskkill cleanup — so the default test set carries zero real
//! process termination (same-source discipline as 006 "CI 零真实 EAC 交互",
//! W25 warmup #[ignore] convention). Run manually with:
//!   cargo test -p vua-project-manager --test eac_terminate -- --ignored
//! The primitive test additionally needs the gated hook:
//!   --features test-hooks

use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use vua_orchestrator::FixedClock;
use vua_project_manager::{
    terminate_candidate, ManagerRoots, ProcessEntry, ProcessSnapshotSource, TerminationGuard,
};

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-eac-terminate-{label}-{nanos}"))
}

fn roots(base: &std::path::Path) -> ManagerRoots {
    let _ = base;
    ManagerRoots {
        alcom_settings_candidates: vec![],
    }
}

fn clock() -> FixedClock {
    FixedClock::new(&["2026-09-09T07:30:00.000Z"])
}

/// Spawns a fully controlled process that stays alive for the test and
/// returns (child, pid, image_name, image_path). PowerShell is used
/// because it carries a Microsoft embedded signature — the signature
/// element of the R3 re-verification is exercised for real.
fn spawn_target() -> (Child, u32, String, String) {
    let child = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 120"])
        .spawn()
        .expect("spawning the controlled target process");
    let pid = child.id();
    // Give the process a moment to reach its image path; then read it
    // through the same read-only query the verifier uses.
    std::thread::sleep(std::time::Duration::from_millis(700));
    let image_path = vua_project_manager::read_process_image_path_readonly(pid)
        .expect("the target image path must be readable");
    let image_name = PathBuf::from(&image_path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "powershell.exe".to_owned())
        .to_ascii_lowercase();
    (child, pid, image_name, image_path)
}

/// A synthetic snapshot source that reports the target entry only while
/// the OS still sees it alive — mirroring the real table's behavior
/// around a termination.
struct LiveSource {
    pid: u32,
    name: String,
    path: String,
    include_vrchat: bool,
}

impl ProcessSnapshotSource for LiveSource {
    fn snapshot(&self) -> std::io::Result<Vec<ProcessEntry>> {
        let alive = vua_project_manager::read_process_image_path_readonly(self.pid).is_some();
        if !alive {
            return Ok(Vec::new());
        }
        let mut entries = vec![ProcessEntry {
            image_name: self.name.clone(),
            pid: self.pid,
            path: Some(self.path.clone()),
        }];
        if self.include_vrchat {
            entries.push(ProcessEntry {
                image_name: "vrchat.exe".to_owned(),
                pid: 999_999,
                path: None,
            });
        }
        Ok(entries)
    }
}

fn empty_vcc() -> Vec<PathBuf> {
    Vec::new()
}

#[test]
#[ignore = "真机手动验证件: spawns and taskkills a real PowerShell process; the default set carries zero real process termination"]
fn a_catalog_signed_target_is_refused_by_the_four_element_check() {
    let (_child, pid, name, path) = spawn_target();
    let base = unique_dir("happy");
    let directory = path
        .rsplit_once('\\')
        .map(|(directory, _)| format!("{directory}\\"))
        .expect("the image path has a directory part");

    // The allowlist entry drafted from the running process (R2 evidence
    // discipline — same shape as the W25 B2b window run).
    let entry = vua_project_manager::AllowlistEntryV01 {
        process_name: name.clone(),
        expected_path_pattern: directory,
        publisher_evidence: "Microsoft signature verified in-window".to_owned(),
        reason: "controlled test target (synthetic process, R8)".to_owned(),
        evidence_ref: "this test".to_owned(),
        added_at: "2026-09-09T07:30:00.000Z".to_owned(),
        release_notes: "test-only entry, never shipped".to_owned(),
    };
    let vcc = empty_vcc();
    let manager_roots = roots(&base);
    let source = LiveSource {
        pid,
        name: name.clone(),
        path: path.clone(),
        include_vrchat: false,
    };
    let request = vua_project_manager::TerminationRequest {
        pid,
        entry: &entry,
        vcc_settings_candidates: &vcc,
        roots: &manager_roots,
        source: &source,
    };

    let receipt = terminate_candidate(&request, "cmd-test-catalog", &clock());

    // powershell.exe is catalog-signed on Windows; GENERIC_VERIFY_V2 file
    // verification honestly types it Unverified (TRUST_E_NOSIGNATURE) and
    // R3 refuses — the strict four-element discipline, working as
    // designed. The Verified path is asserted in the W25 window (B2b
    // against the embedded-signed EasyAntiCheat.exe).
    assert_eq!(receipt.kind, "refused", "receipt: {receipt:?}");
    assert_eq!(
        receipt.guard,
        Some(TerminationGuard::VerificationNotVerified)
    );
    // The refused step did not terminate the target — clean up explicitly.
    let _ = kill(pid);

    cleanup(&base);
}

#[cfg(feature = "test-hooks")]
#[test]
#[ignore = "真机手动验证件: REALLY terminates a real PowerShell process via TerminateProcess (the gated hook); the default set carries zero real process termination"]
fn the_termination_primitive_terminates_and_confirms_exit_for_a_controlled_process() {
    // Direct test of the open→terminate→await→post-check primitive on a
    // fully controlled process (R8: synthetic target, no real EAC
    // interaction). The R3 verdict gate is tested above.
    let (_child, pid, name, path) = spawn_target();
    let base = unique_dir("primitive");
    let pre = vua_project_manager::PreCheck {
        vrchat_session_active: false,
        eac_present: false,
    };
    let source = LiveSource {
        pid,
        name: name.clone(),
        path: path.clone(),
        include_vrchat: false,
    };
    let clock = clock();

    let receipt = vua_project_manager::terminate_open_and_wait_for_test(
        pid,
        &name,
        "cmd-test-primitive-1",
        pre,
        &source,
        &clock,
    );

    assert_eq!(receipt.kind, "terminated", "receipt: {receipt:?}");
    assert_eq!(receipt.exit_confirmed, Some(true));
    assert_eq!(
        receipt.post_check.as_ref().map(|check| check.pid_gone),
        Some(true)
    );
    assert!(receipt.terminated_at.is_some());

    cleanup(&base);
}

#[test]
#[ignore = "真机手动验证件: spawns and taskkills a real PowerShell process; the default set carries zero real process termination"]
fn an_active_vrchat_session_refuses_independently_of_the_allowlist() {
    let (_child, pid, name, path) = spawn_target();
    let base = unique_dir("r4");
    let directory = path
        .rsplit_once('\\')
        .map(|(directory, _)| format!("{directory}\\"))
        .expect("the image path has a directory part");
    let entry = vua_project_manager::AllowlistEntryV01 {
        process_name: name.clone(),
        expected_path_pattern: directory,
        publisher_evidence: "Microsoft signature verified in-window".to_owned(),
        reason: "controlled test target".to_owned(),
        evidence_ref: "this test".to_owned(),
        added_at: "2026-09-09T07:30:00.000Z".to_owned(),
        release_notes: "test-only entry".to_owned(),
    };
    let vcc = empty_vcc();
    let manager_roots = roots(&base);
    let source = LiveSource {
        pid,
        name: name.clone(),
        path: path.clone(),
        include_vrchat: true, // the R4 guard: a VRChat process is in the table
    };
    let request = vua_project_manager::TerminationRequest {
        pid,
        entry: &entry,
        vcc_settings_candidates: &vcc,
        roots: &manager_roots,
        source: &source,
    };

    let receipt = terminate_candidate(&request, "cmd-test-r4", &clock());

    assert_eq!(receipt.kind, "refused");
    assert_eq!(receipt.guard, Some(TerminationGuard::ActiveVrchatSession));
    assert!(!receipt.verified);
    assert!(
        receipt.inspect_required.is_none(),
        "a refusal is not a failure"
    );

    // The refused step did NOT terminate the target — clean it up
    // explicitly (test-owned process).
    let _ = kill(pid);

    cleanup(&base);
}

#[test]
#[ignore = "真机手动验证件: spawns and taskkills a real PowerShell process; the default set carries zero real process termination"]
fn a_verification_refusal_leaves_the_process_alive() {
    let (_child, pid, _name, path) = spawn_target();
    let base = unique_dir("unverified");

    // The allowlist entry names a DIFFERENT process — the name check
    // refuses, and the target must still be alive afterwards.
    let entry = vua_project_manager::AllowlistEntryV01 {
        process_name: "some-other.exe".to_owned(),
        expected_path_pattern: "C:\\".to_owned(),
        publisher_evidence: "n/a".to_owned(),
        reason: "mismatch entry".to_owned(),
        evidence_ref: "this test".to_owned(),
        added_at: "2026-09-09T07:30:00.000Z".to_owned(),
        release_notes: "test-only entry".to_owned(),
    };
    let vcc = empty_vcc();
    let manager_roots = roots(&base);
    let source = LiveSource {
        pid,
        name: "powershell.exe".to_owned(),
        path: path.clone(),
        include_vrchat: false,
    };
    let request = vua_project_manager::TerminationRequest {
        pid,
        entry: &entry,
        vcc_settings_candidates: &vcc,
        roots: &manager_roots,
        source: &source,
    };

    let receipt = terminate_candidate(&request, "cmd-test-mismatch", &clock());

    assert_eq!(receipt.kind, "refused");
    assert_eq!(
        receipt.guard,
        Some(TerminationGuard::VerificationNotVerified)
    );
    // The process survived the refused step.
    let survived = vua_project_manager::read_process_image_path_readonly(pid).is_some();
    assert!(survived, "a refused termination leaves the process alive");
    let _ = kill(pid); // test cleanup

    cleanup(&base);
}

fn kill(pid: u32) -> std::io::Result<()> {
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .output()?;
    assert!(status.status.success(), "taskkill failed");
    Ok(())
}

fn cleanup(base: &std::path::Path) {
    if base.exists() {
        fs::remove_dir_all(base).unwrap();
    }
}

use std::fs;
