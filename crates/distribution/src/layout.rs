use crate::{
    manifest::{verify, Manifest},
    paths::{atomic_json, digest, owned_path},
    transfer::{verified_file, Transfer},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const PUBLIC_KEY: [u8; 32] = [
    0x49, 0xe6, 0xcf, 0x56, 0xa3, 0x3e, 0xd2, 0x22, 0x95, 0x98, 0x97, 0xb6, 0x8d, 0x5d, 0xbb, 0xac,
    0xce, 0x39, 0xae, 0x2a, 0xa6, 0x49, 0xf4, 0x9d, 0x53, 0x2e, 0x52, 0xf0, 0x96, 0xe9, 0x1d, 0xb8,
];
pub const FEED: &str =
    "https://github.com/VUA-Project/VUA/releases/latest/download/vua-release.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Active {
    pub schema_version: String,
    pub host_manifest: String,
    pub amf_manifest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pending {
    pub schema_version: String,
    pub manifest: String,
    pub state: String,
    pub requested: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub schema_version: String,
    pub host_version: String,
    pub amf_version: String,
    pub runtime_version: String,
    pub pending: Option<String>,
    pub target_amf_version: Option<String>,
    pub rollback_available: bool,
    pub pending_manifest_id: Option<String>,
}
pub struct Installation {
    pub root: PathBuf,
    key: [u8; 32],
}
impl Installation {
    pub fn open(root: &Path, create: bool) -> Result<Self> {
        Self::with_key(root, create, PUBLIC_KEY)
    }
    pub fn with_key(root: &Path, create: bool, key: [u8; 32]) -> Result<Self> {
        if !root.is_absolute() || root.parent().is_none() || root.components().count() < 3 {
            return Err(Error("invalid_install_root".into()));
        }
        let mut parent = Some(root);
        while let Some(path) = parent {
            if fs::symlink_metadata(path).is_ok_and(|meta| crate::paths::is_link(&meta)) {
                return Err(Error("install_root_is_link".into()));
            }
            parent = path.parent();
        }
        let marker = root.join("vua-installation.json");
        if !marker.exists() {
            if !create || (root.exists() && fs::read_dir(root)?.next().is_some()) {
                return Err(Error("unowned_install_root".into()));
            }
            fs::create_dir_all(root)?;
            atomic_json(
                &marker,
                &serde_json::json!({"schemaVersion":"0.1","product":"VUA"}),
            )?;
        } else {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase", deny_unknown_fields)]
            struct Marker {
                schema_version: String,
                product: String,
            }
            let marker: Marker = serde_json::from_slice(&fs::read(&marker)?)
                .map_err(|_| Error("invalid_install_marker".into()))?;
            if marker.schema_version != "0.1" || marker.product != "VUA" {
                return Err(Error("invalid_install_marker".into()));
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            key,
        })
    }
    pub fn lock(&self, name: &str) -> Result<fs::File> {
        let path = owned_path(&self.root, name)?;
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        file.try_lock()
            .map_err(|_| Error("installation_busy".into()))?;
        Ok(file)
    }
    fn json<T: serde::de::DeserializeOwned>(&self, name: &str) -> Result<T> {
        Ok(serde_json::from_slice(&fs::read(owned_path(
            &self.root, name,
        )?)?)?)
    }
    pub fn active(&self) -> Result<Active> {
        let active: Active = self.json("active.json")?;
        if active.schema_version != "0.1" {
            return Err(Error("unsupported_installed_layout".into()));
        }
        self.manifest(&active.host_manifest)?;
        self.manifest(&active.amf_manifest)?;
        Ok(active)
    }
    pub fn manifest(&self, id: &str) -> Result<Manifest> {
        if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error("invalid_manifest_identity".into()));
        }
        let bytes = fs::read(owned_path(&self.root, &format!("manifests/{id}.json"))?)?;
        if digest(&bytes) != id {
            return Err(Error("manifest_integrity_failed".into()));
        }
        verify(&bytes, &self.key)
    }
    pub fn store_manifest(&self, bytes: &[u8]) -> Result<(String, Manifest)> {
        let manifest = verify(bytes, &self.key)?;
        let id = digest(bytes);
        let path = owned_path(&self.root, &format!("manifests/{id}.json"))?;
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| Error("invalid_release_path".into()))?,
        )?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(bytes)?;
                file.sync_all()?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if fs::read(&path)? != bytes {
                    return Err(Error("manifest_integrity_failed".into()));
                }
            }
            Err(error) => return Err(error.into()),
        }
        Ok((id, manifest))
    }
    pub fn host_root(&self, manifest: &Manifest) -> Result<PathBuf> {
        owned_path(
            &self.root,
            &format!("payloads/host/{}", manifest.host.pack.sha256),
        )
    }
    pub fn amf_root(&self, manifest: &Manifest) -> Result<PathBuf> {
        owned_path(
            &self.root,
            &format!("payloads/amf/{}", manifest.amf.pack.sha256),
        )
    }
    pub fn runtime_root(&self, manifest: &Manifest) -> Result<PathBuf> {
        owned_path(
            &self.root,
            &format!("runtime/{}", manifest.runtime.archive.sha256),
        )
    }
    pub fn verify_files(&self, root: &Path, files: &[crate::manifest::FileObject]) -> Result<()> {
        for file in files {
            if !verified_file(&owned_path(root, &file.path)?, file.bytes, &file.sha256) {
                return Err(Error("installed_files_need_repair".into()));
            }
        }
        Ok(())
    }
    pub fn pending(&self) -> Result<Option<Pending>> {
        if !self.root.join("pending.json").exists() {
            return Ok(None);
        }
        let pending: Pending = self.json("pending.json")?;
        if pending.schema_version != "0.1"
            || !["interrupted", "staged"].contains(&pending.state.as_str())
        {
            return Err(Error("invalid_pending_update".into()));
        }
        self.manifest(&pending.manifest)?;
        Ok(Some(pending))
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let active = self.active()?;
        let host = self.manifest(&active.host_manifest)?;
        let amf = self.manifest(&active.amf_manifest)?;
        amf.compatible_amf(&host)?;
        let pending = self.pending()?;
        Ok(Snapshot {
            schema_version: "0.1".into(),
            host_version: host.host.version.clone(),
            amf_version: amf.amf.version.clone(),
            runtime_version: host.runtime.version.clone(),
            target_amf_version: pending
                .as_ref()
                .map(|p| self.manifest(&p.manifest).map(|m| m.amf.version))
                .transpose()?,
            pending_manifest_id: pending.as_ref().map(|p| p.manifest.clone()),
            pending: pending.map(|p| p.state),
            rollback_available: self.rollback_target(&host, &amf).is_ok(),
        })
    }
    pub async fn install(&self, raw: &[u8], input: &Path, transfer: &mut Transfer) -> Result<()> {
        let _lock = self.lock("operation.lock")?;
        let _running = self.lock("running.lock")?;
        let (id, manifest) = self.store_manifest(raw)?;
        let previous = if self.root.join("active.json").exists() {
            Some(self.active()?)
        } else {
            None
        };
        let previous_host = previous
            .as_ref()
            .map(|p| self.manifest(&p.host_manifest))
            .transpose()?;
        let previous_amf = previous
            .as_ref()
            .map(|p| self.manifest(&p.amf_manifest))
            .transpose()?;
        // Installation never silently downgrades AMF or opens data with an incompatible reader.
        let selected_amf = if let Some(old) = &previous_amf {
            let newer = version(&old.amf.version)? > version(&manifest.amf.version)?;
            if newer && old.compatible_amf(&manifest).is_ok() {
                old.clone()
            } else {
                if !manifest
                    .compatibility
                    .bdl_read
                    .contains(&old.compatibility.bdl_write)
                {
                    return Err(Error("data_requires_newer_amf".into()));
                }
                manifest.clone()
            }
        } else {
            manifest.clone()
        };
        self.space_required(&manifest, true)?;
        fs::create_dir_all(owned_path(&self.root, "cache")?)?;
        let host_root = self.host_root(&manifest)?;
        let amf_root = self.amf_root(&selected_amf)?;
        transfer
            .component(
                &manifest.host.files,
                &manifest.host.pack,
                previous_host
                    .as_ref()
                    .map(|m| self.host_root(m))
                    .transpose()?
                    .as_deref(),
                &host_root,
                Some(&input.join("host.bin")),
                &self.root.join("cache/host.bin"),
            )
            .await?;
        if selected_amf.amf.pack.sha256 == manifest.amf.pack.sha256 {
            transfer
                .component(
                    &manifest.amf.files,
                    &manifest.amf.pack,
                    previous_amf
                        .as_ref()
                        .map(|m| self.amf_root(m))
                        .transpose()?
                        .as_deref(),
                    &amf_root,
                    Some(&input.join("amf.bin")),
                    &self.root.join("cache/amf.bin"),
                )
                .await?;
        } else {
            self.verify_files(&amf_root, &selected_amf.amf.files)?;
        }
        self.runtime(&manifest, input, transfer).await?;
        self.install_bootstrap(&manifest)?;
        let amf_id = if selected_amf.amf.pack.sha256 == manifest.amf.pack.sha256 {
            id.clone()
        } else {
            self.active()?.amf_manifest
        };
        let next = Active {
            schema_version: "0.1".into(),
            host_manifest: id,
            amf_manifest: amf_id,
        };
        if let Some(old) = previous.filter(|old| old != &next) {
            atomic_json(&self.root.join("previous.json"), &old)?;
        }
        atomic_json(&self.root.join("active.json"), &next)?;
        if self.prune_unreferenced().is_err() {
            transfer.progress("cleanup-later", 0, 1);
        }
        transfer.progress("ready", 1, 1);
        Ok(())
    }
    async fn runtime(
        &self,
        manifest: &Manifest,
        input: &Path,
        transfer: &mut Transfer,
    ) -> Result<()> {
        let root = self.runtime_root(manifest)?;
        if self.verify_files(&root, &manifest.runtime.files).is_ok() {
            transfer.progress("runtime-reused", 1, 1);
            return Ok(());
        }
        let cache = owned_path(
            &self.root,
            &format!("cache/{}.zip", manifest.runtime.archive.sha256),
        )?;
        let local = input.join("runtime.zip");
        if verified_file(
            &local,
            manifest.runtime.archive.bytes,
            &manifest.runtime.archive.sha256,
        ) {
            fs::copy(&local, &cache)?;
        } else {
            transfer.archive(&manifest.runtime.archive, &cache).await?;
        }
        fs::create_dir_all(&root)?;
        let mut zip = zip::ZipArchive::new(fs::File::open(cache)?)
            .map_err(|_| Error("invalid_runtime_archive".into()))?;
        let count = zip.len();
        for index in 0..count {
            transfer.checkpoint()?;
            let mut file = zip
                .by_index(index)
                .map_err(|_| Error("invalid_runtime_archive".into()))?;
            let name = file.name().to_string();
            if file
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err(Error("runtime_archive_link".into()));
            }
            let target = owned_path(&root, name.trim_end_matches('/'))?;
            if file.is_dir() {
                fs::create_dir_all(target)?;
                continue;
            }
            let expected = manifest
                .runtime
                .files
                .iter()
                .find(|entry| entry.path == name)
                .ok_or_else(|| Error("unexpected_runtime_file".into()))?;
            if file.size() != expected.bytes {
                return Err(Error("runtime_integrity_failed".into()));
            }
            fs::create_dir_all(
                target
                    .parent()
                    .ok_or_else(|| Error("invalid_release_path".into()))?,
            )?;
            let mut dest = fs::File::create(&target)?;
            std::io::copy(&mut file, &mut dest)?;
            dest.sync_all()?;
            transfer.progress("extracting", index as u64 + 1, count as u64);
        }
        self.verify_files(&root, &manifest.runtime.files)
    }
    fn install_bootstrap(&self, manifest: &Manifest) -> Result<()> {
        let file = manifest
            .host
            .files
            .iter()
            .find(|f| f.path == "bootstrap/app.asar")
            .ok_or_else(|| Error("missing_bootstrap".into()))?;
        let source = owned_path(&self.host_root(manifest)?, &file.path)?;
        if !verified_file(&source, file.bytes, &file.sha256) {
            return Err(Error("bootstrap_integrity_failed".into()));
        }
        let runtime = self.runtime_root(manifest)?;
        fs::copy(source, owned_path(&runtime, "resources/app.asar")?)?;
        // Electron classifies electron.exe as the development runner. The owned
        // launch alias has exactly the official binary bytes, with a product name.
        // Keep electron.exe intact as part of the complete upstream inventory.
        let executable = manifest
            .runtime
            .files
            .iter()
            .find(|file| file.path == "electron.exe")
            .ok_or_else(|| Error("incomplete_runtime".into()))?;
        let launch = owned_path(&runtime, "VUA.Runtime.exe")?;
        if !verified_file(&launch, executable.bytes, &executable.sha256) {
            fs::copy(owned_path(&runtime, "electron.exe")?, launch)?;
        }
        Ok(())
    }
    pub async fn stage_amf(&self, raw: &[u8], transfer: &mut Transfer) -> Result<()> {
        self.stage_amf_with_pack(raw, transfer, None).await
    }
    pub async fn stage_amf_with_pack(
        &self,
        raw: &[u8],
        transfer: &mut Transfer,
        local_pack: Option<&Path>,
    ) -> Result<()> {
        let _lock = self.lock("operation.lock")?;
        let active = self.active()?;
        let host = self.manifest(&active.host_manifest)?;
        let old = self.manifest(&active.amf_manifest)?;
        let (id, target) = self.store_manifest(raw)?;
        target.compatible_amf(&host)?;
        if version(&target.amf.version)? <= version(&old.amf.version)?
            && target.amf.pack.sha256 != old.amf.pack.sha256
        {
            return Err(Error("update_not_newer".into()));
        }
        if !target
            .compatibility
            .bdl_read
            .contains(&old.compatibility.bdl_write)
        {
            return Err(Error("unsupported_data_migration".into()));
        }
        self.space_required(&target, false)?;
        fs::create_dir_all(owned_path(&self.root, "cache")?)?;
        let mut pending = Pending {
            schema_version: "0.1".into(),
            manifest: id,
            state: "interrupted".into(),
            requested: false,
        };
        atomic_json(&self.root.join("pending.json"), &pending)?;
        transfer
            .component(
                &target.amf.files,
                &target.amf.pack,
                Some(&self.amf_root(&old)?),
                &self.amf_root(&target)?,
                local_pack,
                &owned_path(&self.root, &format!("cache/{}.bin", target.amf.pack.sha256))?,
            )
            .await?;
        pending.state = "staged".into();
        atomic_json(&self.root.join("pending.json"), &pending)?;
        transfer.progress("ready", 1, 1);
        Ok(())
    }
    pub fn request_activation(&self) -> Result<()> {
        let _lock = self.lock("operation.lock")?;
        let mut pending = self
            .pending()?
            .ok_or_else(|| Error("update_not_staged".into()))?;
        if pending.state != "staged" {
            return Err(Error("update_not_staged".into()));
        }
        pending.requested = true;
        atomic_json(&self.root.join("pending.json"), &pending)
    }
    pub fn activate(&self) -> Result<()> {
        let _lock = self.lock("operation.lock")?;
        let _running = self.lock("running.lock")?;
        let active = self.active()?;
        let pending = self
            .pending()?
            .ok_or_else(|| Error("update_not_staged".into()))?;
        if !pending.requested || pending.state != "staged" {
            return Err(Error("activation_requires_choice".into()));
        }
        let target = self.manifest(&pending.manifest)?;
        let host = self.manifest(&active.host_manifest)?;
        target.compatible_amf(&host)?;
        self.verify_files(&self.amf_root(&target)?, &target.amf.files)?;
        let next = Active {
            amf_manifest: pending.manifest,
            ..active.clone()
        };
        if next != active {
            atomic_json(&self.root.join("previous.json"), &active)?;
        }
        atomic_json(&self.root.join("active.json"), &next)?;
        fs::remove_file(owned_path(&self.root, "pending.json")?)?;
        let _ = self.prune_unreferenced();
        Ok(())
    }
    fn rollback_target(&self, host: &Manifest, amf: &Manifest) -> Result<Active> {
        let old: Active = self.json("previous.json")?;
        if old == self.active()? {
            return Err(Error("no_retained_version".into()));
        }
        if old.schema_version != "0.1" {
            return Err(Error("unsupported_installed_layout".into()));
        }
        let previous_host = self.manifest(&old.host_manifest)?;
        let previous_amf = self.manifest(&old.amf_manifest)?;
        previous_amf.compatible_amf(&previous_host)?;
        if !previous_amf
            .compatibility
            .bdl_read
            .contains(&amf.compatibility.bdl_write)
            || previous_host.compatibility.application != host.compatibility.application
        {
            return Err(Error("rollback_requires_data_repair".into()));
        }
        self.verify_files(&self.host_root(&previous_host)?, &previous_host.host.files)?;
        self.verify_files(&self.amf_root(&previous_amf)?, &previous_amf.amf.files)?;
        Ok(old)
    }
    pub fn rollback(&self) -> Result<()> {
        let _lock = self.lock("operation.lock")?;
        let _running = self.lock("running.lock")?;
        let active = self.active()?;
        let host = self.manifest(&active.host_manifest)?;
        let amf = self.manifest(&active.amf_manifest)?;
        let old = self.rollback_target(&host, &amf)?;
        self.install_bootstrap(&self.manifest(&old.host_manifest)?)?;
        atomic_json(&self.root.join("active.json"), &old)?;
        atomic_json(&self.root.join("previous.json"), &active)?;
        Ok(())
    }
    fn prune_unreferenced(&self) -> Result<()> {
        // Only immutable directories identified by previously verified signed
        // manifests are eligible. Retain active, previous and interrupted targets.
        let active = self.active()?;
        let mut keep = vec![
            self.manifest(&active.host_manifest)?,
            self.manifest(&active.amf_manifest)?,
        ];
        if let Ok(previous) = self.json::<Active>("previous.json") {
            keep.push(self.manifest(&previous.host_manifest)?);
            keep.push(self.manifest(&previous.amf_manifest)?);
        }
        if let Some(pending) = self.pending()? {
            keep.push(self.manifest(&pending.manifest)?);
        }
        for entry in fs::read_dir(owned_path(&self.root, "manifests")?)? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            let Some(id) = name.strip_suffix(".json") else {
                continue;
            };
            let Ok(manifest) = self.manifest(id) else {
                continue;
            };
            for (relative, retained) in [
                (
                    format!("payloads/host/{}", manifest.host.pack.sha256),
                    keep.iter()
                        .any(|m| m.host.pack.sha256 == manifest.host.pack.sha256),
                ),
                (
                    format!("payloads/amf/{}", manifest.amf.pack.sha256),
                    keep.iter()
                        .any(|m| m.amf.pack.sha256 == manifest.amf.pack.sha256),
                ),
                (
                    format!("runtime/{}", manifest.runtime.archive.sha256),
                    keep.iter()
                        .any(|m| m.runtime.archive.sha256 == manifest.runtime.archive.sha256),
                ),
            ] {
                if !retained {
                    let path = owned_path(&self.root, &relative)?;
                    if path.exists() {
                        reject_links(&path)?;
                        fs::remove_dir_all(path)?;
                    }
                }
            }
        }
        Ok(())
    }
    fn space_required(&self, manifest: &Manifest, runtime: bool) -> Result<()> {
        let mut needed = manifest.amf.pack.bytes * 2 + 32 * 1024 * 1024;
        if runtime {
            needed += manifest.host.pack.bytes * 2;
            if self
                .verify_files(&self.runtime_root(manifest)?, &manifest.runtime.files)
                .is_err()
            {
                needed += manifest.runtime.archive.bytes
                    + manifest.runtime.files.iter().map(|f| f.bytes).sum::<u64>()
                    + manifest
                        .runtime
                        .files
                        .iter()
                        .find(|f| f.path == "electron.exe")
                        .map_or(0, |f| f.bytes);
            }
        }
        if free_space(&self.root)? < needed {
            return Err(Error("insufficient_disk_space".into()));
        }
        Ok(())
    }
    pub fn remove(&self) -> Result<()> {
        let _lock = self.lock("operation.lock")?;
        let _running = self.lock("running.lock")?;
        for name in ["payloads", "runtime", "manifests", "cache"] {
            let path = owned_path(&self.root, name)?;
            if path.exists() {
                reject_links(&path)?;
                fs::remove_dir_all(path)?;
            }
        }
        for name in ["active.json", "pending.json", "previous.json"] {
            let path = owned_path(&self.root, name)?;
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }
}
fn reject_links(path: &Path) -> Result<()> {
    if crate::paths::is_link(&fs::symlink_metadata(path)?) {
        return Err(Error("owned_tree_is_link".into()));
    }
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            reject_links(&entry?.path())?;
        }
    }
    Ok(())
}
fn version(value: &str) -> Result<semver::Version> {
    semver::Version::parse(value).map_err(|_| Error("invalid_version".into()))
}
#[cfg(windows)]
fn free_space(root: &Path) -> Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = root.as_os_str().encode_wide().chain([0]).collect();
    let mut available = 0;
    let result = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if result == 0 {
        return Err(Error("cannot_check_disk_space".into()));
    }
    Ok(available)
}
#[cfg(not(windows))]
fn free_space(_: &Path) -> Result<u64> {
    Ok(u64::MAX)
}
