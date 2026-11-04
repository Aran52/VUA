//! Offline AMF-owned SQLite snapshots. Never opens BdlStore or runs a migration.
use rusqlite::{
    backup::{Backup, StepResult},
    Connection, OpenFlags,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

pub const DATA_RECOVERY_ABI: &str = "amf-data/0.1";
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    schema_version: String,
    token: String,
    binding: String,
    layout: String,
    state: String,
    files: Vec<Database>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Database {
    name: String,
    present: bool,
    bytes: u64,
    sha256: String,
}

fn identity(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn no_links(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err("invalid_data_profile".into());
    }
    for ancestor in path.ancestors() {
        if ancestor
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err("invalid_data_profile".into());
        }
        if let Ok(metadata) = fs::symlink_metadata(ancestor) {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err("data_path_is_link".into());
                }
            }
            if metadata.file_type().is_symlink() {
                return Err("data_path_is_link".into());
            }
        }
    }
    Ok(())
}
fn layout(profile: &Path) -> Result<String> {
    let registration = profile.join("modules/amf.json");
    no_links(&registration)?;
    if registration.exists() {
        let value: serde_json::Value = serde_json::from_slice(&fs::read(registration)?)?;
        if value["schemaVersion"] != "0.1" || !value["enabled"].is_boolean() {
            return Err("invalid_amf_registration".into());
        }
        return match value["dataLayout"].as_str() {
            Some("legacy") => Ok("legacy".into()),
            Some("isolated") => Ok("isolated".into()),
            _ => Err("invalid_amf_registration".into()),
        };
    }
    Ok(if profile.join("bdl/bdl.db").exists() {
        "legacy"
    } else {
        "isolated"
    }
    .into())
}
fn database_path(profile: &Path, layout: &str, name: &str) -> Result<PathBuf> {
    let relative = match (layout, name) {
        ("legacy", "bdl") => "bdl/bdl.db",
        ("isolated", "bdl") => "modules/amf/data/bdl/bdl.db",
        ("legacy" | "isolated", "tasks") => "modules/amf/tasks.db",
        _ => return Err("invalid_backup_database".into()),
    };
    let value = profile.join(relative);
    no_links(&value)?;
    Ok(value)
}
fn digest(path: &Path) -> Result<(u64, String)> {
    no_links(path)?;
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut bytes = 0;
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        hash.update(&buffer[..count]);
    }
    Ok((
        bytes,
        hash.finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    ))
}
fn sqlite_copy(source: &Path, destination: &Path) -> Result<()> {
    no_links(source)?;
    no_links(destination)?;
    // Refuse to truncate or repurpose an existing file, including a failed prior snapshot.
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)?;
    let source = Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    source.busy_timeout(Duration::from_secs(3))?;
    let mut target = Connection::open(destination)?;
    {
        let backup = Backup::new(&source, &mut target)?;
        let started = std::time::Instant::now();
        loop {
            if started.elapsed() > Duration::from_secs(120) {
                return Err("data_backup_timeout".into());
            }
            match backup.step(1024)? {
                StepResult::Done => break,
                StepResult::More => {}
                _ => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    }
    let check: String = target.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if check != "ok" {
        return Err("backup_database_corrupt".into());
    }
    drop(target);
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(destination)?
        .sync_all()?;
    Ok(())
}
fn save(directory: &Path, record: &Record) -> Result<()> {
    let pending = directory.join("snapshot.pending");
    no_links(&pending)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&pending)?;
    file.write_all(&serde_json::to_vec(record)?)?;
    file.sync_all()?;
    drop(file);
    fs::rename(pending, directory.join("snapshot.json"))?;
    Ok(())
}
fn load(directory: &Path, token: &str, binding: &str) -> Result<Record> {
    let file = directory.join("snapshot.json");
    no_links(&file)?;
    if fs::metadata(&file)?.len() > 65536 {
        return Err("invalid_data_backup".into());
    }
    let value: Record = serde_json::from_slice(&fs::read(file)?)?;
    if value.schema_version != "0.1"
        || value.token != token
        || value.binding != binding
        || !["prepared", "restoring", "restored", "committed"].contains(&value.state.as_str())
        || value.files.len() != 2
        || value.files[0].name != "bdl"
        || value.files[1].name != "tasks"
        || !["legacy", "isolated"].contains(&value.layout.as_str())
    {
        return Err("invalid_data_backup".into());
    }
    Ok(value)
}
/// Call only after AMF and all of its writers have stopped. Inputs come from trusted Main/native code.
pub fn maintain(
    profile: &Path,
    token: &str,
    binding: &str,
    action: &str,
    expected_receipt: Option<&str>,
) -> Result<String> {
    if !identity(token) || !identity(binding) {
        return Err("invalid_data_backup_identity".into());
    }
    no_links(profile)?;
    let backups = profile.join("modules/amf/update-backups");
    no_links(&backups)?;
    fs::create_dir_all(&backups)?;
    let lock_path = backups.join("maintenance.lock");
    no_links(&lock_path)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.try_lock().map_err(|_| "data_backup_busy")?;
    let directory = backups.join(token);
    no_links(&directory)?;
    if action == "snapshot" {
        fs::create_dir(&directory)?;
        let layout = layout(profile)?;
        let mut files = Vec::new();
        for name in ["bdl", "tasks"] {
            let source = database_path(profile, &layout, name)?;
            let present = source.exists();
            let (bytes, sha256) = if present {
                let target = directory.join(format!("{name}.db"));
                sqlite_copy(&source, &target)?;
                digest(&target)?
            } else {
                (0, String::new())
            };
            files.push(Database {
                name: name.into(),
                present,
                bytes,
                sha256,
            });
        }
        save(
            &directory,
            &Record {
                schema_version: "0.1".into(),
                token: token.into(),
                binding: binding.into(),
                layout,
                state: "prepared".into(),
                files,
            },
        )?;
        return Ok("prepared".into());
    }
    let mut record = load(&directory, token, binding)?;
    if expected_receipt != Some(inventory_receipt(&record)?.as_str()) {
        return Err("data_backup_receipt_mismatch".into());
    }
    if action == "migrate" {
        if record.state != "prepared" {
            return Err("invalid_data_backup_state".into());
        }
        if record.files[0].present {
            drop(crate::BdlStore::open(database_path(
                profile,
                &record.layout,
                "bdl",
            )?)?);
        }
        return Ok(record.state);
    }
    if action == "commit" {
        if record.state != "prepared" && record.state != "committed" {
            return Err("invalid_data_backup_state".into());
        }
        record.state = "committed".into();
        save(&directory, &record)?;
        return Ok(record.state);
    }
    if action != "restore" || record.state == "committed" {
        return Err("invalid_data_backup_state".into());
    }
    if record.state == "restored" {
        return Ok(record.state);
    }
    // Check every backup before changing either live database.
    for file in &record.files {
        if file.present
            && digest(&directory.join(format!("{}.db", file.name)))?
                != (file.bytes, file.sha256.clone())
        {
            return Err("data_backup_corrupt".into());
        }
    }
    record.state = "restoring".into();
    save(&directory, &record)?;
    for file in &record.files {
        let target = database_path(profile, &record.layout, &file.name)?;
        let pending = target.with_extension("update-restore");
        no_links(&pending)?;
        if file.present {
            fs::create_dir_all(target.parent().ok_or("invalid_data_profile")?)?;
            if pending.exists() {
                fs::remove_file(&pending)?;
            }
            sqlite_copy(&directory.join(format!("{}.db", file.name)), &pending)?;
        }
        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{suffix}", target.to_string_lossy()));
            no_links(&sidecar)?;
            if sidecar.exists() {
                fs::remove_file(sidecar)?;
            }
        }
        if file.present {
            fs::rename(pending, target)?;
        } else if target.exists() {
            fs::remove_file(target)?;
        }
    }
    record.state = "restored".into();
    save(&directory, &record)?;
    Ok(record.state)
}

fn inventory_receipt(record: &Record) -> Result<String> {
    let bytes = serde_json::to_vec(&(
        &record.schema_version,
        &record.token,
        &record.binding,
        &record.layout,
        &record.files,
    ))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
pub fn backup_receipt(profile: &Path, token: &str, binding: &str) -> Result<String> {
    if !identity(token) || !identity(binding) {
        return Err("invalid_data_backup_identity".into());
    }
    let directory = profile.join("modules/amf/update-backups").join(token);
    no_links(&directory)?;
    inventory_receipt(&load(&directory, token, binding)?)
}

#[cfg(test)]
#[path = "recovery_tests.rs"]
mod tests;
