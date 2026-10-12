use crate::{manifest::verify, transfer::Transfer, Error, Result};
use std::time::Duration;

pub const FEEDS: [&str; 2] = [
    "https://ph-r.com/vua/releases/latest/vua-release.json",
    "https://github.com/VUA-Project/VUA/releases/latest/download/vua-release.json",
];

/// Query both bounded sources so a reachable but stale mirror cannot hide a release.
/// Only authenticated complete inventories participate; no cached result is substituted.
pub(crate) async fn latest_manifest(
    transfer: &Transfer,
    sources: [&str; 2],
    public_key: &[u8; 32],
) -> Result<Vec<u8>> {
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
) -> Result<Vec<u8>> {
    let mut latest = None;
    for raw in results.into_iter().flatten() {
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
            latest = Some((version, raw));
        }
    }
    latest
        .map(|(_, raw)| raw)
        .ok_or_else(|| Error("feed_unavailable".into()))
}
