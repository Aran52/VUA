//! Independently signed host/AMF channels. Existing v0.1 inventories stay immutable.
use crate::{
    manifest::{https_source, Compatibility, Manifest, SignedManifest, MAX_MANIFEST_BYTES, UI_ABI},
    Error, Result,
};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashSet};

pub const SCHEMA: &str = "0.2";
pub const HOST_FEED: &str = "https://ph-r.com/vua/updates/beta/host.json";
pub const AMF_FEED: &str = "https://ph-r.com/vua/updates/beta/amf.json";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComponentKind {
    Host,
    Amf,
}
impl ComponentKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Amf => "amf",
        }
    }
    pub fn feed(self) -> &'static str {
        match self {
            Self::Host => HOST_FEED,
            Self::Amf => AMF_FEED,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Activation {
    Restart,
    Reload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestObject {
    pub sha256: String,
    pub bytes: u64,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Release {
    pub version: String,
    pub build: String,
    pub release_url: String,
    pub manifest: ManifestObject,
    pub compatibility: Compatibility,
    pub activation: Activation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentFeed {
    pub schema_version: String,
    pub product: String,
    pub platform: String,
    pub arch: String,
    pub channel: String,
    pub component: ComponentKind,
    pub sequence: u64,
    pub releases: Vec<Release>,
}

#[derive(Debug)]
pub struct Selection<'a> {
    pub latest: Option<&'a Release>,
    pub target: Option<&'a Release>,
}

fn hex(value: &str, bytes: usize) -> bool {
    value.len() == bytes * 2
        && value
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
}
fn version(value: &str) -> Result<semver::Version> {
    semver::Version::parse(value).map_err(|_| Error("invalid_component_version".into()))
}
fn compatible(amf: &Compatibility, host_version: &str, host: &Compatibility) -> bool {
    semver::VersionReq::parse(&amf.host)
        .ok()
        .zip(version(host_version).ok())
        .is_some_and(|(range, version)| range.matches(&version))
        && amf.ui_abi == host.ui_abi
        && amf.react == host.react
        && amf.application == host.application
        && amf.bridge == host.bridge
}

impl ComponentFeed {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SCHEMA
            || self.product != "VUA"
            || self.platform != "windows"
            || self.arch != "x64"
            || self.channel != "beta"
            || self.sequence == 0
            || self.sequence > 9_007_199_254_740_991
            || self.releases.len() > 64
        {
            return Err(Error("unsupported_component_feed".into()));
        }
        let mut versions = HashSet::new();
        for release in &self.releases {
            let mut release_version = version(&release.version)?;
            // Build metadata must not make a second artifact with equal SemVer precedence.
            release_version.build = semver::BuildMetadata::EMPTY;
            if !versions.insert(release_version)
                || !hex(&release.build, 20)
                || !https_source(&release.release_url)
                || !hex(&release.manifest.sha256, 32)
                || release.manifest.bytes == 0
                || release.manifest.bytes > MAX_MANIFEST_BYTES as u64
                || release.manifest.sources.is_empty()
                || release.manifest.sources.len() > 4
                || !release
                    .manifest
                    .sources
                    .iter()
                    .all(|source| https_source(source))
            {
                return Err(Error("invalid_component_release".into()));
            }
            let c = &release.compatibility;
            if c.ui_abi != UI_ABI
                || c.application != "0.1"
                || c.bridge.is_empty()
                || c.bdl_write.is_empty()
                || c.bdl_read.is_empty()
                || c.bdl_read.len() > 32
                || c.bdl_read.iter().any(String::is_empty)
                || !c.bdl_read.contains(&c.bdl_write)
                || c.bdl_read.iter().collect::<HashSet<_>>().len() != c.bdl_read.len()
                || semver::VersionReq::parse(&c.host).is_err()
                || version(&c.react).is_err()
            {
                return Err(Error("invalid_component_compatibility".into()));
            }
            match self.component {
                ComponentKind::Host => {
                    if release.activation != Activation::Restart
                        || release
                            .runtime_version
                            .as_deref()
                            .and_then(|v| version(v).ok())
                            .is_none()
                        || !semver::VersionReq::parse(&c.host).is_ok_and(|r| {
                            r.matches(&version(&release.version).expect("validated version"))
                        })
                    {
                        return Err(Error("invalid_host_release".into()));
                    }
                }
                ComponentKind::Amf if release.runtime_version.is_some() => {
                    return Err(Error("amf_must_not_select_runtime".into()))
                }
                ComponentKind::Amf => {}
            }
        }
        Ok(())
    }

    /// Select the latest compatible improvement, even when a newer release needs a newer host.
    pub fn select<'a>(&'a self, host: &Manifest, amf: &Manifest) -> Result<Selection<'a>> {
        self.validate()?;
        host.validate()?;
        amf.validate()?;
        let installed = version(match self.component {
            ComponentKind::Host => &host.host.version,
            ComponentKind::Amf => &amf.amf.version,
        })?;
        let mut ordered = self.releases.iter().collect::<Vec<_>>();
        ordered.sort_by(|a, b| {
            version(&b.version)
                .expect("validated version")
                .cmp_precedence(&version(&a.version).expect("validated version"))
        });
        let target = ordered.iter().copied().find(|release| {
            version(&release.version)
                .expect("validated version")
                .cmp_precedence(&installed)
                == Ordering::Greater
                && match self.component {
                    ComponentKind::Host => {
                        compatible(&amf.compatibility, &release.version, &release.compatibility)
                    }
                    ComponentKind::Amf => {
                        compatible(
                            &release.compatibility,
                            &host.host.version,
                            &host.compatibility,
                        ) && release
                            .compatibility
                            .bdl_read
                            .contains(&amf.compatibility.bdl_write)
                    }
                }
        });
        Ok(Selection {
            latest: ordered.first().copied(),
            target,
        })
    }
}

impl Release {
    /// Bind every offered fact back to the separately authenticated inventory, before staging.
    pub fn verify_inventory(
        &self,
        kind: ComponentKind,
        raw: &[u8],
        key: &[u8; 32],
    ) -> Result<Manifest> {
        if raw.len() as u64 != self.manifest.bytes
            || crate::paths::digest(raw) != self.manifest.sha256
        {
            return Err(Error("component_manifest_integrity_failed".into()));
        }
        let manifest = crate::manifest::verify(raw, key)?;
        let component = match kind {
            ComponentKind::Host => &manifest.host,
            ComponentKind::Amf => &manifest.amf,
        };
        if component.version != self.version
            || component.build != self.build
            || manifest.compatibility != self.compatibility
            || (kind == ComponentKind::Host
                && self.runtime_version.as_deref() != Some(manifest.runtime.version.as_str()))
        {
            return Err(Error("component_manifest_identity_mismatch".into()));
        }
        Ok(manifest)
    }
    pub fn activation_for(&self, kind: ComponentKind, current_amf: &Manifest) -> Activation {
        if kind == ComponentKind::Host
            || self.compatibility.bdl_write != current_amf.compatibility.bdl_write
        {
            Activation::Restart
        } else {
            self.activation
        }
    }
}

/// The expected component and durable sequence are supplied by Main/native code, never a renderer URL.
pub fn verify(
    raw: &[u8],
    key: &[u8; 32],
    expected: ComponentKind,
    minimum_sequence: u64,
) -> Result<ComponentFeed> {
    if raw.len() > MAX_MANIFEST_BYTES {
        return Err(Error("component_feed_too_large".into()));
    }
    let envelope: SignedManifest = serde_json::from_slice(raw)?;
    if envelope.schema_version != SCHEMA
        || envelope.algorithm != "ed25519"
        || !hex(&envelope.signature, 64)
    {
        return Err(Error("unsupported_component_feed".into()));
    }
    let mut signature = [0u8; 64];
    for (i, byte) in signature.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&envelope.signature[i * 2..i * 2 + 2], 16)
            .map_err(|_| Error("invalid_release_signature".into()))?;
    }
    VerifyingKey::from_bytes(key)
        .map_err(|_| Error("invalid_trust_key".into()))?
        .verify_strict(
            envelope.payload.as_bytes(),
            &Signature::from_bytes(&signature),
        )
        .map_err(|_| Error("invalid_release_signature".into()))?;
    let feed: ComponentFeed = serde_json::from_str(&envelope.payload)?;
    feed.validate()?;
    if feed.component != expected {
        return Err(Error("wrong_component_feed".into()));
    }
    if feed.sequence < minimum_sequence {
        return Err(Error("component_feed_replayed".into()));
    }
    Ok(feed)
}

#[cfg(test)]
mod tests;
