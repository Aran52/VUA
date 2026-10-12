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
    let root = option(args, "--root").map(PathBuf::from).unwrap_or(
        std::env::current_exe()?
            .parent()
            .ok_or_else(|| Error("invalid_install_root".into()))?
            .to_path_buf(),
    );
    let installation = Installation::open(&root, command == "install")?;
    let mut transfer = Transfer::new(cancel, progress)?;
    match command {
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
            let raw = latest_manifest(&transfer, FEEDS, &crate::layout::PUBLIC_KEY).await?;
            let target = verify(&raw, &crate::layout::PUBLIC_KEY)?;
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
            let (manifest_id, _) = installation.store_manifest(&raw)?;
            Ok(
                json!({"amfVersion":target.amf.version, "hostVersion":target.host.version, "compatible":compatible,
                "newer":newer, "downloadBytes":download_bytes, "manifestId":manifest_id, "releaseUrl":target.release_url}),
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
            launch(&installation, args)?;
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
            launch(&installation, args)?;
            Ok(json!({"launched":true}))
        }
        "remove" => {
            installation.remove()?;
            Ok(json!({"removed":true}))
        }
        "launch" => {
            launch(&installation, args)?;
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
fn launch(installation: &Installation, args: &[String]) -> Result<()> {
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
    let running = installation.lock("running.lock").ok();
    let executable = host
        .runtime
        .files
        .iter()
        .find(|file| file.path == "electron.exe")
        .ok_or_else(|| Error("incomplete_runtime".into()))?;
    let launch = owned_path(&runtime_root, "VUA.Runtime.exe")?;
    if !crate::transfer::verified_file(&launch, executable.bytes, &executable.sha256) {
        return Err(Error("runtime_launch_alias_needs_repair".into()));
    }
    let mut child = Command::new(launch);
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
    let status = child.spawn()?.wait()?;
    drop(running);
    if !status.success() {
        return Err(Error("installed_application_failed".into()));
    }
    Ok(())
}
