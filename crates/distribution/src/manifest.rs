use crate::{Error, Result};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SCHEMA: &str = "0.1";
pub const UI_ABI: &str = "amf-ui/0.1";
pub const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileObject {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub offset: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pack {
    pub bytes: u64,
    pub sha256: String,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Component {
    pub version: String,
    pub build: String,
    pub files: Vec<FileObject>,
    pub pack: Pack,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Runtime {
    pub version: String,
    pub archive: Pack,
    pub files: Vec<FileObject>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Compatibility {
    pub host: String,
    pub ui_abi: String,
    pub react: String,
    pub application: String,
    pub bridge: String,
    pub bdl_write: String,
    pub bdl_read: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: String,
    pub product: String,
    pub platform: String,
    pub arch: String,
    pub release_version: String,
    pub release_url: String,
    pub runtime: Runtime,
    pub host: Component,
    pub amf: Component,
    pub compatibility: Compatibility,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignedManifest {
    pub schema_version: String,
    pub algorithm: String,
    pub payload: String,
    pub signature: String,
}

fn hex<const N: usize>(value: &str) -> Result<[u8; N]> {
    if value.len() != N * 2 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error("invalid_hex".into()));
    }
    let mut result = [0u8; N];
    for (index, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| Error("invalid_hex".into()))?;
    }
    Ok(result)
}

pub fn verify(bytes: &[u8], public_key: &[u8; 32]) -> Result<Manifest> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(Error("manifest_too_large".into()));
    }
    let signed: SignedManifest = serde_json::from_slice(bytes)?;
    if signed.schema_version != SCHEMA || signed.algorithm != "ed25519" {
        return Err(Error("unsupported_manifest".into()));
    }
    let key =
        VerifyingKey::from_bytes(public_key).map_err(|_| Error("invalid_trust_key".into()))?;
    key.verify_strict(
        signed.payload.as_bytes(),
        &Signature::from_bytes(&hex::<64>(&signed.signature)?),
    )
    .map_err(|_| Error("invalid_release_signature".into()))?;
    let manifest: Manifest = serde_json::from_str(&signed.payload)?;
    manifest.validate()?;
    Ok(manifest)
}

pub fn https_source(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
    })
}

impl Manifest {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SCHEMA
            || self.product != "VUA"
            || self.platform != "windows"
            || self.arch != "x64"
            || self.compatibility.ui_abi != UI_ABI
            || self.compatibility.application != "0.1"
            || !https_source(&self.release_url)
        {
            return Err(Error("unsupported_release".into()));
        }
        for version in [
            &self.release_version,
            &self.runtime.version,
            &self.host.version,
            &self.amf.version,
            &self.compatibility.react,
        ] {
            semver::Version::parse(version).map_err(|_| Error("invalid_version".into()))?;
        }
        let host_range = semver::VersionReq::parse(&self.compatibility.host)
            .map_err(|_| Error("invalid_host_range".into()))?;
        if !host_range.matches(
            &semver::Version::parse(&self.host.version)
                .map_err(|_| Error("invalid_host_version".into()))?,
        ) || self.host.version != self.release_version
        {
            return Err(Error("incompatible_host".into()));
        }
        if self.compatibility.bdl_read.is_empty()
            || self.compatibility.bdl_read.len() > 32
            || !self
                .compatibility
                .bdl_read
                .contains(&self.compatibility.bdl_write)
            || self.compatibility.bridge.is_empty()
        {
            return Err(Error("invalid_data_compatibility".into()));
        }
        validate_files(&self.runtime.files, None)?;
        validate_pack(&self.runtime.archive)?;
        for component in [&self.host, &self.amf] {
            if component.build.len() != 40
                || !component.build.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(Error("invalid_build_identity".into()));
            }
            validate_pack(&component.pack)?;
            validate_files(&component.files, Some(component.pack.bytes))?;
        }
        for (files, required) in [
            (
                &self.runtime.files,
                &[
                    "electron.exe",
                    "resources/default_app.asar",
                    "LICENSE",
                    "LICENSES.chromium.html",
                ][..],
            ),
            (
                &self.host.files,
                &[
                    "dist/electron/main.js",
                    "dist/electron/preload.js",
                    "dist/renderer/index.html",
                    "provider/vua-orchestrator-provider.exe",
                    "bootstrap/app.asar",
                ][..],
            ),
            (
                &self.amf.files,
                &[
                    "ui/amf-entry.js",
                    "ui/style.css",
                    "provider/vua-amf-provider.exe",
                ][..],
            ),
        ] {
            if required
                .iter()
                .any(|path| !files.iter().any(|file| file.path == *path))
            {
                return Err(Error("incomplete_release_layout".into()));
            }
        }
        Ok(())
    }
    pub fn compatible_amf(&self, active_host: &Manifest) -> Result<()> {
        let range = semver::VersionReq::parse(&self.compatibility.host)
            .map_err(|_| Error("invalid_host_range".into()))?;
        if !range.matches(
            &semver::Version::parse(&active_host.host.version)
                .map_err(|_| Error("invalid_host_version".into()))?,
        ) || self.compatibility.ui_abi != active_host.compatibility.ui_abi
            || self.compatibility.react != active_host.compatibility.react
            || self.compatibility.application != active_host.compatibility.application
            || self.compatibility.bridge != active_host.compatibility.bridge
        {
            return Err(Error("amf_requires_host_update".into()));
        }
        Ok(())
    }
}

fn validate_pack(pack: &Pack) -> Result<()> {
    if pack.bytes == 0
        || pack.bytes > 2 * 1024 * 1024 * 1024
        || pack.sha256.len() != 64
        || hex::<32>(&pack.sha256).is_err()
        || pack.sources.is_empty()
        || pack.sources.len() > 4
        || !pack.sources.iter().all(|source| https_source(source))
    {
        return Err(Error("invalid_release_source".into()));
    }
    Ok(())
}
fn validate_files(files: &[FileObject], pack_bytes: Option<u64>) -> Result<()> {
    if files.is_empty() || files.len() > 20_000 {
        return Err(Error("invalid_release_inventory".into()));
    }
    let mut names = HashSet::new();
    let mut end = 0;
    for file in files {
        if !crate::paths::relative_name(&file.path)
            || !names.insert(file.path.to_ascii_lowercase())
            || hex::<32>(&file.sha256).is_err()
            || file.bytes > 512 * 1024 * 1024
        {
            return Err(Error("invalid_release_file".into()));
        }
        if let Some(size) = pack_bytes {
            let next = file
                .offset
                .checked_add(file.bytes)
                .ok_or_else(|| Error("invalid_release_range".into()))?;
            if file.offset != end || next > size {
                return Err(Error("invalid_release_range".into()));
            }
            end = next;
        }
    }
    if pack_bytes.is_some_and(|size| end != size) {
        return Err(Error("incomplete_release_pack".into()));
    }
    Ok(())
}
