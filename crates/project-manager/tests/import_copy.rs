//! M6 T-A (proposal 014) tests: the import-as-VUA-copy write path. The
//! five guards, the double-summary discipline, the exclusion list, the
//! source link, and the re-inspection all run against synthetic trees; the
//! wire shapes are the frozen `schemas/project-ops/v0.2/` schemas, which
//! the positive/negative example vectors are validated against here.
//! (Followed the v0.2 elevation in place: the import-copy shapes it froze
//! are identical to v0.1, so these assertions read as before.)

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use vua_orchestrator::FixedClock;
use vua_project_manager::{
    apply_import_copy, plan_import_copy, ImportCopyRequest, ImportRejected, ManagerRoots,
    RejectionGuard,
};

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-import-copy-{label}-{nanos}"))
}

fn cleanup(base: &Path) {
    if base.exists() {
        fs::remove_dir_all(base).unwrap();
    }
}

fn roots(base: &Path) -> ManagerRoots {
    ManagerRoots {
        alcom_settings_candidates: vec![base.join("alcom/setting.json")],
    }
}

fn vcc_candidates(base: &Path) -> Vec<PathBuf> {
    vec![base.join("vcc/settings.json")]
}

fn clock() -> FixedClock {
    FixedClock::new(&["2026-09-09T03:40:00.000Z"])
}

fn request<'a>(
    source: &'a Path,
    target_parent: &'a Path,
    name: &'a str,
    vcc: &'a [PathBuf],
    manager_roots: &'a ManagerRoots,
) -> ImportCopyRequest<'a> {
    ImportCopyRequest {
        source,
        target_parent,
        name,
        vcc_settings_candidates: vcc,
        roots: manager_roots,
    }
}

/// Installs a real VPM project (manifest + Unity version + one excluded
/// regenerable directory) and registers it with BOTH managers.
fn install_source(base: &Path, name: &str) -> PathBuf {
    let project = base.join("projects").join(name);
    fs::create_dir_all(project.join("Assets")).unwrap();
    fs::create_dir_all(project.join("Packages")).unwrap();
    fs::create_dir_all(project.join("ProjectSettings")).unwrap();
    fs::create_dir_all(project.join("Library")).unwrap(); // regenerable
    fs::create_dir_all(project.join(".vua")).unwrap(); // old task state
    fs::write(project.join("Assets").join("scene.unity"), "dummy scene").unwrap();
    fs::write(
        project.join("Packages").join("vpm-manifest.json"),
        r#"{ "dependencies": { "com.vrchat.base": "3.7.x" } }"#,
    )
    .unwrap();
    fs::write(
        project.join("ProjectSettings").join("ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1\n",
    )
    .unwrap();
    fs::write(
        project
            .join("ProjectSettings")
            .join("ProjectSettings.asset"),
        "productName: Old\n",
    )
    .unwrap();
    fs::write(project.join("Library").join("big-cache.bin"), "regenerable").unwrap();
    register_vcc(base, &[&project]);
    register_alcom(base, &[&project]);
    project
}

fn write_settings(path: PathBuf, projects: &[&Path]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let listed: Vec<String> = projects
        .iter()
        .map(|project| project.to_string_lossy().into_owned())
        .collect();
    fs::write(
        &path,
        serde_json::to_string(&json!({ "userProjects": listed })).unwrap(),
    )
    .unwrap();
}

fn register_vcc(base: &Path, projects: &[&Path]) {
    write_settings(base.join("vcc/settings.json"), projects);
}

fn register_alcom(base: &Path, projects: &[&Path]) {
    write_settings(base.join("alcom/setting.json"), projects);
}

fn read_repo_json(relative: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("../..").join(relative);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/project-ops/v0.2/result.schema.json",
    ))
    .unwrap()
}

fn command_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_repo_json(
        "schemas/project-ops/v0.2/command.schema.json",
    ))
    .unwrap()
}

fn violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

#[test]
fn plan_and_apply_happy_path_copies_with_exclusions_and_records_the_source() {
    let base = unique_dir("happy");
    let source = install_source(&base, "My World");
    let target_parent = base.join("imports");
    fs::create_dir_all(&target_parent).unwrap();

    // PLAN: guards pass, the scope is measured, the exclusions are listed.
    let vcc = vcc_candidates(&base);
    let manager_roots = roots(&base);
    let plan = plan_import_copy(&request(
        &source,
        &target_parent,
        "My World Copy",
        &vcc,
        &manager_roots,
    ))
    .expect("plan must pass for a registered healthy project");
    assert_eq!(plan.kind, "plan");
    assert_eq!(
        plan.excluded_entries,
        vec![".vua", "Builds", "Library", "Logs", "Temp", "obj"]
    );
    assert!(plan.estimated_bytes > 0);
    assert!(plan.source_top_levels.contains(&"Assets".to_owned()));
    assert!(!plan.source_top_levels.contains(&"Library".to_owned()));

    let plan_json = serde_json::to_value(&plan).unwrap();
    let problems = violations(&result_validator(), &enveloped(plan_json.clone()));
    assert!(problems.is_empty(), "plan schema violations: {problems:#?}");

    // APPLY: the copy runs, the exclusions hold, the identity is new, the
    // source link is recorded, and the re-inspection reads the copy.
    let receipt = apply_import_copy(
        &request(
            &source,
            &target_parent,
            "My World Copy",
            &vcc,
            &manager_roots,
        ),
        &plan.plan_digest,
        "cmd-test-import-1",
        &clock(),
    )
    .expect("apply must succeed right after a passing plan");

    assert_eq!(receipt.kind, "receipt");
    let target = PathBuf::from(&receipt.target_path);
    assert!(target.is_dir());
    assert!(target.join("Assets").join("scene.unity").is_file());
    assert!(target.join("Packages").join("vpm-manifest.json").is_file());
    assert!(
        !target.join("Library").exists(),
        "regenerable dirs are excluded"
    );
    assert!(!target.join(".vua").join("pending-mutation.json").exists());

    // New Unity-facing identity: productName rewritten to the new name.
    let settings =
        fs::read_to_string(target.join("ProjectSettings").join("ProjectSettings.asset")).unwrap();
    assert!(
        settings.contains("productName: \"My World Copy\""),
        "settings: {settings}"
    );

    // Source link recorded inside the new project's .vua.
    let source_doc = fs::read_to_string(target.join(".vua").join("source.json")).unwrap();
    let source_value: Value = serde_json::from_str(&source_doc).unwrap();
    assert_eq!(
        source_value["sourceLink"]["sourcePath"],
        json!(source.to_string_lossy().into_owned())
    );
    assert_eq!(
        source_value["sourceLink"]["taskCorrelation"],
        json!("cmd-test-import-1")
    );
    assert_eq!(
        source_value["sourceLink"]["sourceAssociations"],
        json!(["vcc_registered", "alcom_registered"])
    );

    // Re-inspection reads the COPY, not the original.
    assert_eq!(
        receipt.re_inspection.unity_version.as_deref(),
        Some("2022.3.22f1")
    );
    assert!(receipt.re_inspection.manifest_present && receipt.re_inspection.manifest_schema_ok);
    assert!(receipt.bytes_copied > 0);
    assert!(receipt.copied_top_levels.contains(&"Assets".to_owned()));

    // VUA-native identity (rulings items 7/9/12): the copy is first-marked
    // on apply; the original stays unmarked.
    assert!(matches!(
        vua_project_manager::read_identity(&target),
        vua_project_manager::VuaIdentity::Present(_)
    ));
    assert_eq!(
        vua_project_manager::read_identity(&source),
        vua_project_manager::VuaIdentity::Absent,
        "the original project is never marked"
    );

    // The ORIGINAL project is untouched (1.2.0 read-only ruling): its .vua
    // still holds exactly what the fixture put there — no source.json.
    assert!(!source.join(".vua").join("source.json").exists());
    assert!(source.join("Library").exists());

    let receipt_json = serde_json::to_value(&receipt).unwrap();
    let problems = violations(&result_validator(), &enveloped(receipt_json));
    assert!(
        problems.is_empty(),
        "receipt schema violations: {problems:#?}"
    );

    cleanup(&base);
}

/// Wraps a result document (which carries its own `kind`) into the
/// project-ops result envelope, exactly as the provider face will.
fn enveloped(result: Value) -> Value {
    json!({
        "schemaVersion": "0.2",
        "operation": "project.import-copy",
        "result": result,
    })
}

#[test]
fn apply_refuses_plan_drift_instead_of_executing_a_stale_confirmation() {
    let base = unique_dir("drift");
    let source = install_source(&base, "Drifty");
    let target_parent = base.join("imports");
    fs::create_dir_all(&target_parent).unwrap();

    let vcc = vcc_candidates(&base);
    let manager_roots = roots(&base);
    let plan = plan_import_copy(&request(
        &source,
        &target_parent,
        "Drifty Copy",
        &vcc,
        &manager_roots,
    ))
    .unwrap();

    // The source changes after the user confirmed the plan.
    fs::write(source.join("Assets").join("new-file.txt"), "changed").unwrap();

    let rejected = apply_import_copy(
        &request(&source, &target_parent, "Drifty Copy", &vcc, &manager_roots),
        &plan.plan_digest,
        "cmd-drift",
        &clock(),
    )
    .expect_err("drifted plan must be refused");
    assert_eq!(rejected.guard, RejectionGuard::PlanDrift);
    assert_eq!(rejected.code, "vua.project.plan_drift");
    assert!(
        !target_parent.join("Drifty Copy").exists(),
        "a refused import writes nothing"
    );

    cleanup(&base);
}

#[test]
fn the_five_guards_refuse_typecally() {
    let base = unique_dir("guards");
    let source = install_source(&base, "Guarded");
    let target_parent = base.join("imports");
    fs::create_dir_all(&target_parent).unwrap();

    // Guard: source not registered (no settings mention it).
    let stranger = base.join("projects").join("Stranger");
    fs::create_dir_all(stranger.join("Packages")).unwrap();
    fs::create_dir_all(stranger.join("ProjectSettings")).unwrap();
    fs::write(stranger.join("Packages").join("vpm-manifest.json"), "{}").unwrap();
    fs::write(
        stranger.join("ProjectSettings").join("ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1\n",
    )
    .unwrap();
    let vcc = vcc_candidates(&base);
    let manager_roots = roots(&base);
    let rejected = plan_import_copy(&request(
        &stranger,
        &target_parent,
        "Stranger Copy",
        &vcc,
        &manager_roots,
    ))
    .expect_err("an unregistered source must be refused");
    assert_eq!(rejected.guard, RejectionGuard::SourceNotRegistered);

    // Guard: source invalid (registered but no VPM markers).
    let incomplete = base.join("projects").join("Incomplete");
    fs::create_dir_all(&incomplete).unwrap();
    register_vcc(&base, &[&source, &incomplete]);
    let rejected = plan_import_copy(&request(
        &incomplete,
        &target_parent,
        "Incomplete Copy",
        &vcc,
        &manager_roots,
    ))
    .expect_err("a marker-less source must be refused");
    assert_eq!(rejected.guard, RejectionGuard::SourceInvalid);

    // Guard: target exists.
    let existing_target = target_parent.join("Taken");
    fs::create_dir_all(&existing_target).unwrap();
    let rejected = plan_import_copy(&request(
        &source,
        &target_parent,
        "Taken",
        &vcc,
        &manager_roots,
    ))
    .expect_err("an existing target must be refused");
    assert_eq!(rejected.guard, RejectionGuard::TargetExists);

    // Guard: target inside the source tree.
    let inside = source.join("nested");
    fs::create_dir_all(&inside).unwrap();
    let rejected = plan_import_copy(&request(
        &source,
        &inside,
        "Nested Copy",
        &vcc,
        &manager_roots,
    ))
    .expect_err("a target inside the source must be refused");
    assert_eq!(rejected.guard, RejectionGuard::TargetInsideSource);

    // Guard: invalid project name (validated before anything else).
    let rejected = plan_import_copy(&request(
        &source,
        &target_parent,
        "bad/name",
        &vcc,
        &manager_roots,
    ))
    .expect_err("an invalid name must be refused");
    assert_eq!(rejected.guard, RejectionGuard::SourceInvalid);

    cleanup(&base);
}

#[test]
fn example_vectors_match_the_frozen_schemas() {
    // Positive vectors validate.
    let command =
        read_repo_json("schemas/project-ops/v0.2/examples/project-import-copy-plan.request.json");
    assert!(violations(&command_validator(), &command).is_empty());
    let command =
        read_repo_json("schemas/project-ops/v0.2/examples/project-import-copy-apply.request.json");
    assert!(violations(&command_validator(), &command).is_empty());

    let plan =
        read_repo_json("schemas/project-ops/v0.2/examples/project-import-copy-plan.result.json");
    assert!(violations(&result_validator(), &plan).is_empty());
    let receipt =
        read_repo_json("schemas/project-ops/v0.2/examples/project-import-copy-apply.result.json");
    assert!(violations(&result_validator(), &receipt).is_empty());

    // Negative vectors are refused by the command schema.
    let bad_phase = read_repo_json("schemas/project-ops/v0.2/examples/invalid-phase.json");
    assert!(!violations(&command_validator(), &bad_phase).is_empty());
    // The "apply without confirmedPlanDigest" negative was not re-issued
    // with the v0.2 vectors; construct it here from the v0.2 apply vector
    // so the required-field refusal stays pinned against the current
    // frozen command schema.
    let mut missing_digest =
        read_repo_json("schemas/project-ops/v0.2/examples/project-import-copy-apply.request.json");
    missing_digest["params"]
        .as_object_mut()
        .expect("apply params object")
        .remove("confirmedPlanDigest");
    assert!(!violations(&command_validator(), &missing_digest).is_empty());
    let bad_operation = read_repo_json("schemas/project-ops/v0.2/examples/invalid-operation.json");
    assert!(!violations(&command_validator(), &bad_operation).is_empty());
}

/// BG-18 regression (CI rust 34630656044 / schema-vectors 34630656005):
/// the GitHub Windows runner's `temp_dir()` is the 8.3 short form
/// (`C:\Users\RUNNER~1\...`). The source (which exists on disk)
/// canonicalizes to the long name, while the not-yet-existing target
/// fell back to the caller's literal spelling — so the prefix comparison
/// compared `RUNNER~1` against `runneradmin` and the TargetInsideSource
/// guard let the plan through. The guard must refuse the target regardless
/// of how the caller spells the path (drive-letter case, verbatim `\\?\`
/// prefix, 8.3 short names).
#[test]
fn target_inside_source_guard_survives_runner_path_spellings() {
    let base = unique_dir("guards-spellings");
    let source = install_source(&base, "GuardedSpelled");
    let target_parent = base.join("imports");
    fs::create_dir_all(&target_parent).unwrap();
    let vcc = vcc_candidates(&base);
    let manager_roots = roots(&base);
    let inside = source.join("nested");
    fs::create_dir_all(&inside).unwrap();

    // Control: the plain spelling is refused.
    let rejected = plan_import_copy(&request(
        &source,
        &inside,
        "Nested Copy",
        &vcc,
        &manager_roots,
    ))
    .expect_err("a target inside the source must be refused");
    assert_eq!(rejected.guard, RejectionGuard::TargetInsideSource);

    #[cfg(windows)]
    {
        // Drive-letter case drift must not defeat the guard.
        let lowered = with_drive_letter_case(&inside, false);
        if lowered != inside.as_os_str() {
            let rejected = plan_import_copy(&request(
                &source,
                Path::new(&lowered),
                "Nested Copy",
                &vcc,
                &manager_roots,
            ))
            .expect_err("a lower-case drive spelling must still be refused");
            assert_eq!(rejected.guard, RejectionGuard::TargetInsideSource);
        }

        // The verbatim `\\?\` prefix must not defeat the guard.
        let verbatim = PathBuf::from(format!(r"\\?\{}", inside.display()));
        let rejected = plan_import_copy(&request(
            &source,
            &verbatim,
            "Nested Copy",
            &vcc,
            &manager_roots,
        ))
        .expect_err("a verbatim-prefix spelling must still be refused");
        assert_eq!(rejected.guard, RejectionGuard::TargetInsideSource);

        // The 8.3 short-name form — the runner's actual TEMP shape — must
        // not defeat the guard. Short names only exist when the volume
        // tracks them: GetShortPathNameW then returns the input unchanged
        // and this spelling is not applicable here (capability detection,
        // not a skipped guard — the case-drift and verbatim spellings above
        // still exercise the guard on such volumes).
        if let Some(short) = short_path(&inside) {
            if short != *inside.as_os_str() {
                let rejected = plan_import_copy(&request(
                    &source,
                    Path::new(&short),
                    "Nested Copy",
                    &vcc,
                    &manager_roots,
                ))
                .expect_err("an 8.3 short-name spelling must still be refused");
                assert_eq!(rejected.guard, RejectionGuard::TargetInsideSource);
            }
        }
    }

    cleanup(&base);
}

#[cfg(windows)]
fn with_drive_letter_case(path: &Path, upper: bool) -> std::ffi::OsString {
    let mut text = path.as_os_str().to_string_lossy().into_owned();
    let bytes = text.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        let first = bytes[0];
        let replaced = if upper {
            first.to_ascii_uppercase()
        } else {
            first.to_ascii_lowercase()
        };
        if replaced != first {
            text.replace_range(0..1, &(replaced as char).to_string());
        }
    }
    text.into()
}

/// Returns the 8.3 short form of `path`, or `None` when the Win32 call
/// fails (the path does not exist or the call is unavailable).
#[cfg(windows)]
fn short_path(path: &Path) -> Option<std::ffi::OsString> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let needed = unsafe { GetShortPathNameW(wide.as_ptr(), std::ptr::null_mut(), 0) };
    if needed == 0 {
        return None;
    }
    let mut buffer = vec![0u16; needed as usize];
    let written = unsafe { GetShortPathNameW(wide.as_ptr(), buffer.as_mut_ptr(), needed) };
    if written == 0 {
        return None;
    }
    Some(std::ffi::OsString::from_wide(&buffer[..written as usize]))
}

#[test]
fn rejected_documents_validate_with_the_frozen_guard_closed_set() {
    let rejection = ImportRejected {
        kind: "rejected",
        guard: RejectionGuard::InsufficientDiskSpace,
        code: "vua.project.insufficient_disk_space",
        detail: "demo".to_owned(),
    };
    let enveloped = json!({
        "schemaVersion": "0.2",
        "operation": "project.import-copy",
        "result": rejection,
    });
    let problems = violations(&result_validator(), &enveloped);
    assert!(problems.is_empty(), "violations: {problems:#?}");
    assert_eq!(
        enveloped["result"]["code"],
        json!("vua.project.insufficient_disk_space")
    );
}

/// BG-12 家族环境域成员（wt-6 反向审查批 2026-09-24）：副本自身
/// productName 写失败不再被 `let _ =` 吞掉——身份步是 1.2.0 规格项 1，
/// 失败必须以 ExecutionFailed 类型化拒绝申报，而不是收据宣称完整成功、
/// 新项目静默沿用旧名。半成品副本照词面留盘作证据（无隐式清理）。
/// 只读位经 fs::copy 权限位继承落到副本上，写腿必然失败而读腿照常。
#[test]
fn failed_product_name_write_refuses_instead_of_silent_success() {
    fn force_writable(path: &Path) {
        if let Ok(metadata) = fs::metadata(path) {
            let mut permissions = metadata.permissions();
            // Clearing the readonly bit is exactly this helper's job (the
            // test just asserted against it); the clippy alternative
            // (`Permissions::new(mode)`) needs the platform-specific mode
            // word this test deliberately avoids.
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            let _ = fs::set_permissions(path, permissions);
        }
    }

    let base = unique_dir("readonly-identity");
    let source = install_source(&base, "Source Project");
    let vcc = vcc_candidates(&base);
    let manager_roots = roots(&base);

    let settings = source.join("ProjectSettings").join("ProjectSettings.asset");
    {
        let mut permissions = fs::metadata(&settings).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&settings, permissions).unwrap();
    }

    let workspace = base.join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let req = request(&source, &workspace, "New Name", &vcc, &manager_roots);
    let plan = plan_import_copy(&req).expect("plan guards pass");

    let error = apply_import_copy(&req, &plan.plan_digest, "corr-readonly", &clock())
        .expect_err("a failed identity write must refuse, not claim success");
    assert_eq!(error.guard, RejectionGuard::ExecutionFailed);
    assert_eq!(error.code, "vua.project.execution_failed");
    assert!(
        error.detail.contains("productName"),
        "the detail names the failed step: {}",
        error.detail
    );

    // 半成品留存：已复制的副本在盘上作证据，不隐式清理、不隐式重试。
    let target = base.join("workspace").join("New Name");
    assert!(target.is_dir(), "the half-copy stays as evidence");

    // 清理：解除只读位再删（Windows remove_dir_all 不越只读位）。
    force_writable(&settings);
    force_writable(&target.join("ProjectSettings").join("ProjectSettings.asset"));
    cleanup(&base);
}
