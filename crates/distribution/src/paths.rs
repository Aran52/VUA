use crate::{Error, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Release paths are portable, relative names. They never contain user data.
pub fn relative_name(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 240
        || value.contains(['\\', ':', '\0', '?', '*', '<', '>', '|', '"'])
    {
        return false;
    }
    value.split('/').all(|part| {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with([' ', '.'])
            || part.chars().any(char::is_control)
        {
            return false;
        }
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        ![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ]
        .contains(&stem.as_str())
    })
}

pub fn owned_path(root: &Path, relative: &str) -> Result<PathBuf> {
    if !relative_name(relative) {
        return Err(Error("unsafe_release_path".into()));
    }
    if std::fs::symlink_metadata(root).is_ok_and(|meta| is_link(&meta)) {
        return Err(Error("release_root_is_link".into()));
    }
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        if let Ok(metadata) = std::fs::symlink_metadata(&path) {
            if is_link(&metadata) {
                return Err(Error("release_path_is_link".into()));
            }
        }
    }
    Ok(path)
}

#[cfg(windows)]
fn reparse(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    meta.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
fn reparse(_: &std::fs::Metadata) -> bool {
    false
}
pub fn is_link(meta: &std::fs::Metadata) -> bool {
    meta.file_type().is_symlink() || reparse(meta)
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
pub fn file_digest(path: &Path) -> Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex(&hasher.finalize()))
}

pub fn atomic_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    if std::fs::symlink_metadata(path).is_ok_and(|meta| is_link(&meta)) {
        return Err(Error("release_path_is_link".into()));
    }
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error("invalid_system_time".into()))?
        .as_nanos();
    let pending = path.with_extension(format!("pending-{}-{unique}", std::process::id()));
    let bytes = serde_json::to_vec(value)?;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&pending)?;
    use std::io::Write;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(pending, path)?;
    Ok(())
}
