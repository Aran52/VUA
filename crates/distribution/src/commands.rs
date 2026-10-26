use crate::{
    feed::{latest_manifest, FEEDS},
    layout::Installation,
    manifest::verify,
    paths::owned_path,
    transfer::{Progress, Transfer},
    Error, Result,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{atomic::AtomicBool, Arc},
};

pub fn report(progress: Progress) {
    println!(
        "{}",
        json!({"schemaVersion":"0.1", "kind":"progress", "progress":progress})
    );
}
pub async fn execute(
    args: &[String],
    cancel: Arc<AtomicBool>,
    progress: impl Fn(Progress) + Send + Sync + 'static,
) -> Result<Value> {
    let command = args.first().map_or("launch", String::as_str);
    if command == "capabilities" {
        return Ok(
            json!({"componentUpdates":"0.2", "recovery":"0.2", "updatePlans":"0.1", "dataRecovery":"0.1"}),
        );
    }
    let root = option(args, "--root").map(PathBuf::from).unwrap_or(
        std::env::current_exe()?
            .parent()
            .ok_or_else(|| Error("invalid_install_root".into()))?
            .to_path_buf(),
    );
    let installation = Installation::open(&root, command == "install")?;
    if command.starts_with("component-") || command.starts_with("plan-") {
        installation.require_component_launcher()?;
    }
    if matches!(
        command,
        "component-request"
            | "component-restore"
            | "component-amf-begin"
            | "plan-request"
            | "plan-restore"
    ) {
        if let Some(profile) = option(args, "--profile") {
            installation.register_data_profile(Path::new(profile))?;
        }
    }
    let mut transfer = Transfer::new(cancel, progress)?;
    match command {
        "component-data-status" => installation.data_trial_status(Path::new(
            option(args, "--profile").ok_or_else(|| Error("invalid_data_profile".into()))?,
        )),
        "component-amf-preflight" => installation.amf_hot_preflight(
            option(args, "--manifest").unwrap_or(""),
            args.iter().any(|value| value == "--restore"),
        ),
        "component-amf-begin" => installation.begin_amf_hot(
            option(args, "--manifest").ok_or_else(|| Error("invalid_update_identity".into()))?,
            args.iter().any(|value| value == "--restore"),
        ),
        "component-amf-finish" => installation.finish_amf_hot(
            option(args, "--manifest").ok_or_else(|| Error("invalid_update_identity".into()))?,
        ),
        "component-amf-recover" => installation.recover_amf_hot(),
        "plan-snapshot" => installation.update_plan_snapshot(),
        "plan-propose" => installation.propose_update_plan(&transfer).await,
        "plan-stage" => {
            installation
                .stage_update_plan(
                    option(args, "--plan").ok_or_else(|| Error("update_plan_required".into()))?,
                    &mut transfer,
                    option(args, "--host-pack").map(Path::new),
                    option(args, "--amf-pack").map(Path::new),
                )
                .await
        }
        "plan-request" => {
            installation.request_update_plan(
                option(args, "--plan").ok_or_else(|| Error("update_plan_required".into()))?,
            )?;
            Ok(json!({"requested":true}))
        }
        "plan-apply" | "plan-restore" => {
            for attempt in 0..300 {
                let result = if command == "plan-apply" {
                    installation
                        .activate_update_plan(
                            option(args, "--plan")
                                .ok_or_else(|| Error("update_plan_required".into()))?,
                            &mut transfer,
                        )
                        .await
                } else {
                    installation.restore_update_plan()
                };
                match result {
                    Ok(()) => break,
                    Err(error) if error.0 == "installation_busy" && attempt < 299 => {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await
                    }
                    Err(error) => return Err(error),
                }
            }
            launch_trial(&installation, args)?;
            Ok(json!({"launched":true}))
        }
        "component-stage" | "component-request" | "component-apply" | "component-restore" => {
            let kind = component(args)?;
            match command {
                "component-stage" => {
                    installation
                        .stage_component(
                            kind,
                            option(args, "--manifest")
                                .ok_or_else(|| Error("missing_update_identity".into()))?,
                            &mut transfer,
                            option(args, "--pack").map(Path::new),
                        )
                        .await
                }
                "component-request" => {
                    installation.request_component_activation(kind)?;
                    Ok(json!({"requested":true}))
                }
                _ => {
                    for attempt in 0..300 {
                        let result = if command == "component-apply" {
                            installation.activate_component(kind, &mut transfer).await
                        } else {
                            installation.restore_component(kind)
                        };
                        match result {
                            Ok(()) => break,
                            Err(error) if error.0 == "installation_busy" && attempt < 299 => {
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await
                            }
                            Err(error) => return Err(error),
                        }
                    }
                    launch_trial(&installation, args)?;
                    Ok(json!({"launched":true}))
                }
            }
        }
        "acknowledge-launch" => {
            installation.data_trial_status(Path::new(option(args, "--profile").unwrap_or("")))?;
            installation.acknowledge_component_health(
                option(args, "--host-build")
                    .ok_or_else(|| Error("update_health_identity_mismatch".into()))?,
                option(args, "--amf-build")
                    .ok_or_else(|| Error("update_health_identity_mismatch".into()))?,
            )?;
            Ok(json!({"acknowledged":true}))
        }
        "component-snapshot" | "component-check" => {
            let component = match option(args, "--component") {
                Some("host") => crate::component_feed::ComponentKind::Host,
                Some("amf") => crate::component_feed::ComponentKind::Amf,
                _ => return Err(Error("invalid_update_component".into())),
            };
            if command == "component-check" {
                installation.check_component(component, &transfer).await
            } else {
                installation.component_snapshot(component)
            }
        }
        "install" => {
            let input = PathBuf::from(
                option(args, "--input").ok_or_else(|| Error("missing_install_input".into()))?,
            );
            installation
                .install(
                    &fs::read(input.join("vua-release.json"))?,
                    &input,
                    &mut transfer,
                )
                .await?;
            Ok(json!({"installed":true}))
        }
        "snapshot" => Ok(serde_json::to_value(installation.snapshot()?)?),
        "check" => {
            let selected = latest_manifest(&transfer, FEEDS, &crate::layout::PUBLIC_KEY).await?;
            let raw = &selected.raw;
            let target = verify(raw, &crate::layout::PUBLIC_KEY)?;
            let release_url = selected.release_page(&target);
            let active = installation.active()?;
            let host = installation.manifest(&active.host_manifest)?;
            let amf = installation.manifest(&active.amf_manifest)?;
            let compatible = target.compatible_amf(&host).is_ok()
                && target
                    .compatibility
                    .bdl_read
                    .contains(&amf.compatibility.bdl_write);
            let newer = semver::Version::parse(&target.amf.version)
                .map_err(|_| Error("invalid_version".into()))?
                > semver::Version::parse(&amf.amf.version)
                    .map_err(|_| Error("invalid_version".into()))?;
            let root = installation.amf_root(&amf)?;
            let download_bytes: u64 = target
                .amf
                .files
                .iter()
                .filter(|file| {
                    owned_path(&root, &file.path).is_err()
                        || !crate::transfer::verified_file(
                            &root.join(&file.path),
                            file.bytes,
                            &file.sha256,
                        )
                })
                .map(|file| file.bytes)
                .sum();
            let (manifest_id, _) = installation.store_manifest(raw)?;
            Ok(
                json!({"amfVersion":target.amf.version, "hostVersion":target.host.version, "compatible":compatible,
                "newer":newer, "downloadBytes":download_bytes, "manifestId":manifest_id, "releaseUrl":release_url}),
            )
        }
        "stage" => {
            let id = option(args, "--manifest")
                .ok_or_else(|| Error("missing_update_identity".into()))?;
            installation.manifest(id)?;
            let raw = fs::read(owned_path(&root, &format!("manifests/{id}.json"))?)?;
            let pack = option(args, "--pack").map(PathBuf::from);
            installation
                .stage_amf_with_pack(&raw, &mut transfer, pack.as_deref())
                .await?;
            Ok(json!({"staged":true}))
        }
        "request-activation" => {
            installation.request_activation()?;
            Ok(json!({"requested":true}))
        }
        "apply-update" => {
            for attempt in 0..300 {
                match installation.activate() {
                    Ok(()) => break,
                    Err(error) if error.0 == "installation_busy" && attempt < 299 => {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await
                    }
                    Err(error) => return Err(error),
                }
            }
            launch(&installation, args, false)?;
            Ok(json!({"launched":true}))
        }
        "rollback" => {
            installation.rollback()?;
            Ok(json!({"restored":true}))
        }
        "rollback-launch" => {
            for attempt in 0..300 {
                match installation.rollback() {
                    Ok(()) => break,
                    Err(error) if error.0 == "installation_busy" && attempt < 299 => {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await
                    }
                    Err(error) => return Err(error),
                }
            }
            launch(&installation, args, false)?;
            Ok(json!({"launched":true}))
        }
        "remove" => {
            installation.remove()?;
            Ok(json!({"removed":true}))
        }
        "launch" => {
            launch(&installation, args, false)?;
            Ok(json!({"launched":true}))
        }
        _ => Err(Error("unknown_distribution_command".into())),
    }
}
pub fn option<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|item| item == key)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}
fn component(args: &[String]) -> Result<crate::component_feed::ComponentKind> {
    match option(args, "--component") {
        Some("host") => Ok(crate::component_feed::ComponentKind::Host),
        Some("amf") => Ok(crate::component_feed::ComponentKind::Amf),
        _ => Err(Error("invalid_update_component".into())),
    }
}
fn launch_trial(installation: &Installation, args: &[String]) -> Result<()> {
    match launch(installation, args, true) {
        Ok(()) => Ok(()),
        Err(error) => {
            if installation.recover_component_activation()? {
                launch(installation, args, false)
            } else {
                Err(error)
            }
        }
    }
}
fn launch(installation: &Installation, args: &[String], trial: bool) -> Result<()> {
    if !trial {
        match installation.recover_component_activation() {
            Ok(_) => {}
            Err(error) if error.0 == "installation_busy" => {}
            Err(error) => return Err(error),
        }
    }
    let active = installation.active()?;
    let host = installation.manifest(&active.host_manifest)?;
    let amf = installation.manifest(&active.amf_manifest)?;
    amf.compatible_amf(&host)?;
    let host_root = installation.host_root(&host)?;
    let runtime_root = installation.runtime_root(&host)?;
    installation.verify_files(&host_root, &host.host.files)?;
    installation.verify_files(&runtime_root, &host.runtime.files)?;
    let bootstrap = host
        .host
        .files
        .iter()
        .find(|f| f.path == "bootstrap/app.asar")
        .ok_or_else(|| Error("missing_bootstrap".into()))?;
    if !crate::transfer::verified_file(
        &owned_path(&runtime_root, "resources/app.asar")?,
        bootstrap.bytes,
        &bootstrap.sha256,
    ) {
        return Err(Error("bootstrap_integrity_failed".into()));
    }
    let running = if trial {
        Some(installation.lock("running.lock")?)
    } else {
        installation.lock("running.lock").ok()
    };
    let executable = host
        .runtime
        .files
        .iter()
        .find(|file| file.path == "electron.exe")
        .ok_or_else(|| Error("incomplete_runtime".into()))?;
    let runtime_executable = owned_path(&runtime_root, "VUA.Runtime.exe")?;
    if !crate::transfer::verified_file(&runtime_executable, executable.bytes, &executable.sha256) {
        return Err(Error("runtime_launch_alias_needs_repair".into()));
    }
    let mut child = Command::new(runtime_executable);
    child.current_dir(&installation.root);
    for (name, _) in std::env::vars_os() {
        let name_text = name.to_string_lossy();
        if name_text.starts_with("ELECTRON_")
            || name_text == "NODE_OPTIONS"
            || name_text == "NODE_PATH"
            || name_text.starts_with("VUA_")
        {
            child.env_remove(name);
        }
    }
    for arg in args {
        if let Some(directory) = arg.strip_prefix("--vua-smoke-test=") {
            if !Path::new(directory).is_absolute() {
                return Err(Error("invalid_smoke_directory".into()));
            }
            child.arg(arg);
        }
    }
    let mut child = match child.spawn() {
        Ok(child) => child,
        Err(error) => {
            drop(running);
            if trial {
                installation.recover_component_activation()?;
                return launch(installation, args, false);
            }
            return Err(error.into());
        }
    };
    if trial {
        let started = std::time::Instant::now();
        loop {
            if installation
                .activation_journal()
                .ok()
                .flatten()
                .is_some_and(|journal| journal.state == "healthy" && journal.next == active)
            {
                break;
            }
            let child_finished = match child.try_wait() {
                Ok(status) => status.is_some(),
                Err(_) => true,
            };
            if child_finished || started.elapsed() > std::time::Duration::from_secs(45) {
                let _ = child.kill();
                let _ = child.wait();
                drop(running);
                installation.recover_component_activation()?;
                return launch(installation, args, false);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    let status = child.wait()?;
    drop(running);
    if !status.success() {
        return Err(Error("installed_application_failed".into()));
    }
    Ok(())
}
