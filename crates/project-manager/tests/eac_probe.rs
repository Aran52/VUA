//! M6 EAC package (proposal 006 R1a) tests: the read-only EAC conflict
//! probe. Everything runs on synthetic process fixtures — zero real EAC
//! interaction in CI (R8); the real-machine probe is the explicitly
//! ignored manual test. Shapes are pinned by
//! `schemas/eac-probe/v0.1/probe.schema.json` and validated here.

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use vua_orchestrator::{FindingSeverity, FixedClock};
use vua_project_manager::{
    probe_eac, ProcessEntry, ProcessKind, ProcessSnapshotSource, ReadinessConclusion,
};

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-eac-probe-{label}-{nanos}"))
}

fn cleanup(base: &Path) {
    if base.exists() {
        fs::remove_dir_all(base).unwrap();
    }
}

fn clock() -> FixedClock {
    FixedClock::new(&["2026-09-09T04:30:00.000Z"])
}

/// A synthetic process table — the R8 discipline: structurally
/// representative fixtures, no real EAC interaction.
struct FixtureSource {
    entries: Vec<ProcessEntry>,
    fail: bool,
}

impl FixtureSource {
    fn of(entries: Vec<ProcessEntry>) -> Self {
        Self {
            entries,
            fail: false,
        }
    }

    fn failing() -> Self {
        Self {
            entries: Vec::new(),
            fail: true,
        }
    }
}

impl ProcessSnapshotSource for FixtureSource {
    fn snapshot(&self) -> std::io::Result<Vec<ProcessEntry>> {
        if self.fail {
            return Err(std::io::Error::other("snapshot refused (fixture)"));
        }
        Ok(self.entries.clone())
    }
}

fn entry(name: &str, pid: u32) -> ProcessEntry {
    ProcessEntry {
        image_name: name.to_owned(),
        pid,
        path: None,
    }
}

fn read_repo_json(relative: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn probe_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json("schemas/eac-probe/v0.1/probe.schema.json")).unwrap()
}

fn violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

#[test]
fn active_session_with_eac_concludes_close_game_first_and_termination_unavailable() {
    let source = FixtureSource::of(vec![
        entry("explorer.exe", 900),
        entry("vrchat.exe", 41_224),
        entry("EasyAntiCheat.exe", 39_012), // case-insensitive classification
    ]);
    let snapshot = probe_eac(&source, &clock());

    assert!(snapshot.vrchat_session_active);
    assert!(snapshot.eac_present);
    assert_eq!(
        snapshot.readiness.conclusion,
        ReadinessConclusion::EacActiveWithVrchatSession
    );
    assert_eq!(
        snapshot.readiness.guidance_code,
        "vua.eac_probe.guidance_close_game_first"
    );

    // Approval point 2: the termination face is unavailable, not merely
    // closed — the allowlist is empty and no real-machine evidence exists.
    assert_eq!(snapshot.termination_capability.state, "unavailable");
    assert_eq!(
        snapshot.termination_capability.reason,
        "allowlist_empty_no_machine_evidence"
    );

    // The unrelated process is never a finding; findings are classified.
    assert_eq!(snapshot.processes.len(), 2);
    assert!(snapshot
        .processes
        .iter()
        .all(|finding| finding.kind != ProcessKind::VrchatOverlay));

    let problems = violations(
        &probe_validator(),
        &serde_json::to_value(&snapshot).unwrap(),
    );
    assert!(problems.is_empty(), "violations: {problems:#?}");

    cleanup(&unique_dir("unused"));
}

#[test]
fn residual_eac_without_a_session_stays_an_honest_finding() {
    let source = FixtureSource::of(vec![entry("EasyAntiCheat.exe", 39_012)]);
    let snapshot = probe_eac(&source, &clock());

    assert!(!snapshot.vrchat_session_active);
    assert!(snapshot.eac_present);
    assert_eq!(
        snapshot.readiness.conclusion,
        ReadinessConclusion::EacResidualNoSession
    );
    assert_eq!(
        snapshot.readiness.guidance_code,
        "vua.eac_probe.guidance_allowlist_unavailable"
    );
    // Even in the residual state the v0.1 capability stays unavailable —
    // real-machine evidence for the allowlist (R2) does not exist yet.
    assert_eq!(snapshot.termination_capability.state, "unavailable");

    cleanup(&unique_dir("unused"));
}

#[test]
fn no_eac_found_is_the_designed_empty_state() {
    let source = FixtureSource::of(vec![entry("explorer.exe", 900), entry("notepad.exe", 901)]);
    let snapshot = probe_eac(&source, &clock());

    assert!(!snapshot.eac_present);
    assert!(!snapshot.vrchat_session_active);
    assert!(snapshot.processes.is_empty());
    assert_eq!(
        snapshot.readiness.conclusion,
        ReadinessConclusion::NoEacFound
    );
    assert_eq!(
        snapshot.readiness.guidance_code,
        "vua.eac_probe.guidance_no_eac_found"
    );

    let problems = violations(
        &probe_validator(),
        &serde_json::to_value(&snapshot).unwrap(),
    );
    assert!(problems.is_empty(), "violations: {problems:#?}");

    cleanup(&unique_dir("unused"));
}

#[test]
fn enumeration_failure_is_a_typed_error_never_a_clean_bill() {
    let snapshot = probe_eac(&FixtureSource::failing(), &clock());

    assert!(snapshot.processes.is_empty());
    assert_eq!(snapshot.diagnostics.len(), 1);
    assert_eq!(
        snapshot.diagnostics[0].code,
        "vua.eac_probe.enumeration_failed"
    );
    assert!(matches!(
        snapshot.diagnostics[0].severity,
        FindingSeverity::Error
    ));

    cleanup(&unique_dir("unused"));
}

#[test]
fn fixtures_and_snapshots_are_deterministic_and_schema_valid() {
    // The frozen fixtures validate against the pinned schema.
    for name in [
        "example.snapshot.session-active.json",
        "example.snapshot.residual.json",
    ] {
        let fixture = read_repo_json(&format!("schemas/eac-probe/v0.1/fixtures/{name}"));
        let problems = violations(&probe_validator(), &fixture);
        assert!(problems.is_empty(), "{name}: {problems:#?}");
    }

    // Two probes over the same snapshot differ only in capturedAt.
    let source = FixtureSource::of(vec![
        entry("vrchat.exe", 1),
        entry("easyanticheatservice.exe", 2),
    ]);
    let one = probe_eac(&source, &clock());
    let two = probe_eac(&source, &clock());
    let mut left = serde_json::to_value(&one).unwrap();
    let mut right = serde_json::to_value(&two).unwrap();
    left["capturedAt"] = json!("");
    right["capturedAt"] = json!("");
    assert_eq!(left, right);
    // Deterministic order: findings sorted by (kind, pid), stable.
    assert_eq!(
        one.processes
            .iter()
            .map(|finding| finding.pid)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );

    cleanup(&unique_dir("unused"));
}

/// Real-machine probe (R8): explicitly ignored; run locally with
/// `cargo test -p vua-project-manager --test eac_probe -- --ignored` on a
/// machine with VRChat/EAC present. Output goes to stdout for the evidence
/// record; nothing is written and nothing is terminated (R1a/R7).
#[test]
#[ignore = "real-machine probe: requires a Windows process table; CI runs synthetic fixtures only"]
fn real_machine_probe_records_the_actual_process_table() {
    let snapshot = probe_eac(
        &vua_project_manager::eac_probe::os_source::ToolhelpProcessSource,
        &clock(),
    );
    let value = serde_json::to_value(&snapshot).unwrap();
    let problems = violations(&probe_validator(), &value);
    assert!(problems.is_empty(), "violations: {problems:#?}");
    println!(
        "real-machine EAC probe: {} relevant processes, session_active={}, eac_present={}, conclusion={:?}",
        snapshot.processes.len(),
        snapshot.vrchat_session_active,
        snapshot.eac_present,
        snapshot.readiness.conclusion
    );
}
