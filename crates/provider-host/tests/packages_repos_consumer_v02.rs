//! packages-repos v0.2 consumer tests (proposal 027 freeze batch, slice
//! F4 = repository lifecycle, 2026-09-20): the core-side consumer of the
//! subscription-list state increment. The frozen
//! `schemas/packages-repos/v0.2/` schemas pin the wire command/result
//! shapes; the example vectors drive JSON-Schema validation directly,
//! and a fake backend's port facts (`list_repos_v02` answering
//! `Vec<RepoInfoV02>` — the frozen v0.1 five-key projection plus the
//! REQUIRED VUA-owned `enabled` state bit) are projected onto the v0.2
//! result document and validated against the same schema. The
//! port→wire consumer loop is nailed here first. The REAL backend
//! consumption (`VrcGetLibBackend` projecting the state from VUA's own
//! storage under the environment root) is the environment
//! implementation-verification slice and lands with its own tests; this
//! file pins the contract side: the additive dual-version negotiation
//! law (the defaulted `repos_v02` accessor — default false, the frozen
//! v0.1 word face keeps being served; the stamped family const tells
//! the consumer which word face answered, never a guess — the
//! `catalog_v02` / `query_v02` precedent), the ruling-(c) word face
//! (the W25 read-only evidence record settled that VCC carries NO
//! enable/disable state anywhere — the bit projects VUA-owned storage,
//! never a settings.json key), the id-absent-row law (a null-repoId row
//! projects `enabled: true` ALWAYS — it is outside the toggle faces'
//! reach, so true is its honest permanent fact), the disabled-row
//! semantics (subscribed and listed, its packages excluded from the
//! collection world), and the false-assertion guard (no health, no
//! status, no lastRefreshed, no disabledAt — facts with no port
//! carrier are INVALID by schema). The command face is byte-for-byte
//! the frozen v0.1 command face (the F3 increment precedent): the wire
//! routes do not change shape, so nothing here exercises a running
//! engine. Everything runs against synthetic data — no machine-specific
//! facts, no network.

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use vua_orchestrator::{AppErrorV1, PackageRequestV1, ProjectRef, RepoInfoV02, VpmBackend};

fn read_repos_json(relative: &str) -> Value {
    // Tests run from the crate directory; the schemas live at the repo root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn repos_command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repos_json(
        "schemas/packages-repos/v0.2/command.schema.json",
    ))
    .unwrap()
}

fn repos_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repos_json(
        "schemas/packages-repos/v0.2/result.schema.json",
    ))
    .unwrap()
}

fn violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

fn example(name: &str) -> Value {
    read_repos_json(&format!("schemas/packages-repos/v0.2/examples/{name}"))
}

/// A backend that serves the subscription-list face at the v0.2 word
/// face: the negotiation accessor true, the method answering rows that
/// cover the three state laws (an enabled row, a disabled row, and the
/// id-absent row that is always enabled).
struct FakeReposV02Backend;

impl VpmBackend for FakeReposV02Backend {
    fn name(&self) -> &'static str {
        "fake-repos-v02"
    }
    fn capabilities(&self) -> vua_orchestrator::VpmCapabilities {
        vua_orchestrator::VpmCapabilities {
            create_project: false,
            preview_install: false,
            list_packages: false,
            remove_packages: false,
            project_registry: false,
            resolve_project: false,
        }
    }
    fn repos_v02(&self) -> bool {
        true
    }
    fn list_repos_v02(&self) -> Result<Vec<RepoInfoV02>, AppErrorV1> {
        Ok(vec![
            RepoInfoV02 {
                repo_id: Some("repo.example.official".to_string()),
                name: Some("Official Repo".to_string()),
                url: Some("https://example.invalid/vrc-official.json".to_string()),
                local_path: Some("C:/anywhere/Repos/official-cache.json".to_string()),
                cached: true,
                enabled: true,
            },
            RepoInfoV02 {
                repo_id: Some("repo.example.community".to_string()),
                name: Some("Community Repo".to_string()),
                url: Some("https://example.invalid/community.json".to_string()),
                local_path: Some("C:/anywhere/Repos/community-cache.json".to_string()),
                cached: true,
                enabled: false,
            },
            RepoInfoV02 {
                repo_id: None,
                name: Some("Local Dir Repo".to_string()),
                url: None,
                local_path: None,
                cached: false,
                // The id-absent-row law: outside the toggle faces' reach,
                // so enabled is this row's honest PERMANENT fact.
                enabled: true,
            },
        ])
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn create_project(
        &self,
        _parent: &std::path::Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        _confirmed_digest: &str,
    ) -> Result<serde_json::Value, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
}

#[test]
fn repos_v02_schema_admits_positive_vectors_and_rejects_negative_ones() {
    let command = repos_command_validator();
    let result = repos_result_validator();

    // --- the positive vectors pass their respective schema ---
    let request = example("packages-list-repos.request.json");
    assert!(
        command.is_valid(&request),
        "{:?}",
        violations(&command, &request)
    );
    let listing = example("packages-list-repos.result.json");
    assert!(
        result.is_valid(&listing),
        "{:?}",
        violations(&result, &listing)
    );
    let empty = example("packages-list-repos-empty.result.json");
    assert!(result.is_valid(&empty), "{:?}", violations(&result, &empty));

    // --- the negative vectors all fail ---
    let negative_request = example("invalid-extra-params.request.json");
    assert!(
        !command.is_valid(&negative_request),
        "the closed-empty params law holds at v0.2: {:?}",
        violations(&command, &negative_request)
    );
    for name in [
        "invalid-row-missing-enabled.result.json",
        "invalid-row-invented-state.result.json",
        "invalid-row-stale-family.result.json",
    ] {
        let negative = example(name);
        assert!(
            !result.is_valid(&negative),
            "{name} must be an invalid result: {:?}",
            violations(&result, &negative)
        );
    }

    // --- the v0.1 command face is byte-for-byte the frozen v0.1 face:
    // the COMMAND schema keeps the 0.1 envelope const while the RESULT
    // document carries the v0.2 family const (the F3 increment law, the
    // c914cf2 two-independent-versions rule) ---
    assert_eq!(listing["schemaVersion"], "0.1");
    assert_eq!(
        listing["result"]["schemaVersion"],
        "vua.packages-repos/v0.2"
    );
    let v02_row_invalid_under_a_v01_family_stamp = json!({
        "schemaVersion": "0.1",
        "operation": "packages.listRepos",
        "result": {
            "schemaVersion": "vua.packages-repos/v0.1",
            "repos": [
                {
                    "repoId": "repo.example.official",
                    "name": "Official Repo",
                    "url": "https://example.invalid/vrc-official.json",
                    "localPath": "C:/anywhere/Repos/official-cache.json",
                    "cached": true,
                    "enabled": true
                }
            ]
        },
    });
    assert!(
        !result.is_valid(&v02_row_invalid_under_a_v01_family_stamp),
        "a v0.2-shaped row under a v0.1 family stamp is the version-increment detectability pin: {:?}",
        violations(&result, &v02_row_invalid_under_a_v01_family_stamp)
    );
}

#[test]
fn repos_v02_negotiation_defaults_declared_and_v01_keeps_serving() {
    // The additive dual-version negotiation law: the defaulted accessor
    // defaults false and the frozen v0.1 word face keeps being served
    // until a backend overrides it; a backend that has not adopted v0.2
    // keeps answering v0.1 through `list_repos`. The stamped family const
    // tells the consumer which word face answered, never a guess.
    let backend = FakeReposV02Backend;
    assert!(backend.repos_v02(), "the adopting backend declares v0.2");
    assert_eq!(backend.name(), "fake-repos-v02");

    // The trait-default absence arm on a bare backend: repos_v02 false,
    // the defaulted method answering the standing capability_missing.
    struct BareBackend;
    impl VpmBackend for BareBackend {
        fn name(&self) -> &'static str {
            "bare"
        }
        fn capabilities(&self) -> vua_orchestrator::VpmCapabilities {
            vua_orchestrator::VpmCapabilities {
                create_project: false,
                preview_install: false,
                list_packages: false,
                remove_packages: false,
                project_registry: false,
                resolve_project: false,
            }
        }
        fn preview_install(
            &self,
            _project: &ProjectRef,
            _packages: &[PackageRequestV1],
        ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
            unreachable!("not exercised in this suite")
        }
        fn apply_install(
            &self,
            _project: &ProjectRef,
            _packages: &[PackageRequestV1],
            _confirmed_digest: &str,
        ) -> Result<serde_json::Value, AppErrorV1> {
            unreachable!("not exercised in this suite")
        }
        fn create_project(
            &self,
            _parent: &std::path::Path,
            _name: &str,
            _template: Option<&str>,
        ) -> Result<ProjectRef, AppErrorV1> {
            unreachable!("not exercised in this suite")
        }
    }
    let bare = BareBackend;
    assert!(
        !bare.repos_v02(),
        "the default is declared-false, never reserved"
    );
    let refusal = bare
        .list_repos_v02()
        .expect_err("the defaulted method refuses");
    assert_eq!(refusal.code, "vua.vpm.capability_missing");
}

#[test]
fn fake_backend_rows_project_onto_the_v02_wire_shape() {
    let command = repos_command_validator();
    let result = repos_result_validator();

    let request = json!({
        "schemaVersion": "0.1",
        "operation": "packages.listRepos",
        "params": {},
    });
    assert!(
        command.is_valid(&request),
        "{:?}",
        violations(&command, &request)
    );

    let rows = FakeReposV02Backend
        .list_repos_v02()
        .expect("the fake backend must list");
    assert_eq!(rows.len(), 3);
    assert!(rows[0].enabled);
    assert!(!rows[1].enabled, "the disabled row is listed, not hidden");
    assert!(
        rows[2].repo_id.is_none() && rows[2].enabled,
        "the id-absent row is always enabled"
    );

    let projected: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "repoId": row.repo_id,
                "name": row.name,
                "url": row.url,
                "localPath": row.local_path,
                "cached": row.cached,
                "enabled": row.enabled,
            })
        })
        .collect();
    let listing = json!({
        "schemaVersion": "0.1",
        "operation": "packages.listRepos",
        "result": {
            "schemaVersion": "vua.packages-repos/v0.2",
            "repos": projected,
        },
    });
    assert!(
        result.is_valid(&listing),
        "{:?}",
        violations(&result, &listing)
    );

    // The row key set is exactly the declared six — the frozen v0.1 five
    // facts plus the ONE new state bit, nothing invented (no health, no
    // status, no lastRefreshed, no disabledAt).
    let row_keys: Vec<String> = listing["result"]["repos"][0]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        row_keys,
        vec![
            "repoId".to_string(),
            "name".to_string(),
            "url".to_string(),
            "localPath".to_string(),
            "cached".to_string(),
            "enabled".to_string(),
        ]
    );
}
