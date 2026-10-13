use crate::{
    manifest::{verify, Manifest},
    transfer::Transfer,
    Error, Result,
};
use std::time::Duration;

pub const FEEDS: [&str; 2] = [
    "https://ph-r.com/vua/releases/latest/vua-release.json",
    "https://github.com/VUA-Project/VUA/releases/latest/download/vua-release.json",
];

#[derive(Debug)]
pub(crate) struct SelectedManifest {
    pub raw: Vec<u8>,
    pub official: bool,
}
impl SelectedManifest {
    pub fn release_page(&self, manifest: &Manifest) -> String {
        if self.official {
            format!(
                "https://ph-r.com/vua/releases/v{}/",
                manifest.release_version
            )
        } else {
            manifest.release_url.clone()
        }
    }
}

/// Query both bounded sources so a reachable but stale mirror cannot hide a release.
/// Only authenticated complete inventories participate; no cached result is substituted.
pub(crate) async fn latest_manifest(
    transfer: &Transfer,
    sources: [&str; 2],
    public_key: &[u8; 32],
) -> Result<SelectedManifest> {
    transfer.checkpoint()?;
    let fetch = async |url: &str| {
        tokio::time::timeout(Duration::from_secs(10), transfer.manifest(url))
            .await
            .unwrap_or_else(|_| Err(Error("feed_unavailable".into())))
    };
    let (official, github) = tokio::join!(fetch(sources[0]), fetch(sources[1]));
    transfer.checkpoint()?;
    select_manifest([official, github], public_key)
}

pub(crate) fn select_manifest(
    results: [Result<Vec<u8>>; 2],
    public_key: &[u8; 32],
) -> Result<SelectedManifest> {
    let mut latest = None;
    for (source, result) in results.into_iter().enumerate() {
        let Ok(raw) = result else {
            continue;
        };
        let Ok(manifest) = verify(&raw, public_key) else {
            continue;
        };
        let version = (
            semver::Version::parse(&manifest.release_version)
                .map_err(|_| Error("invalid_version".into()))?,
            semver::Version::parse(&manifest.amf.version)
                .map_err(|_| Error("invalid_version".into()))?,
        );
        if latest
            .as_ref()
            .is_none_or(|(current, _)| &version > current)
        {
            latest = Some((
                version,
                SelectedManifest {
                    raw,
                    official: source == 0,
                },
            ));
        }
    }
    latest
        .map(|(_, raw)| raw)
        .ok_or_else(|| Error("feed_unavailable".into()))
}
