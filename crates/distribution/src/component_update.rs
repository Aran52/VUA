//! Durable, independent discovery. Fixed feeds and opaque inventory identities stay native.
use crate::{
    component_feed::{self, ComponentFeed, ComponentKind, Release},
    layout::Installation,
    manifest::{Component, Manifest, SignedManifest, MAX_MANIFEST_BYTES},
    paths::{atomic_json, owned_path},
    transfer::{verified_file, Transfer},
    Error, Result,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

fn index_name(kind: ComponentKind) -> String {
    format!("updates-v02/{}-index.json", kind.name())
}

pub(crate) async fn fetch_inventory(
    entry: &Release,
    kind: ComponentKind,
    transfer: &Transfer,
    key: &[u8; 32],
) -> Result<Vec<u8>> {
    for url in &entry.manifest.sources {
        transfer.checkpoint()?;
        if let Ok(raw) = transfer.manifest(url).await {
            if entry.verify_inventory(kind, &raw, key).is_ok() {
                return Ok(raw);
            }
        }
    }
    transfer.checkpoint()?;
    Err(Error("component_manifest_unavailable".into()))
}
fn component(manifest: &Manifest, kind: ComponentKind) -> &Component {
    match kind {
        ComponentKind::Host => &manifest.host,
        ComponentKind::Amf => &manifest.amf,
    }
}
fn reused(roots: &[&Path], value: &Component) -> Result<u64> {
    let mut bytes = 0;
    for file in &value.files {
        if roots.iter().any(|root| {
            owned_path(root, &file.path)
                .is_ok_and(|path| verified_file(&path, file.bytes, &file.sha256))
        }) {
            bytes += file.bytes;
        }
    }
    Ok(bytes)
}

impl Installation {
    pub fn component_snapshot(&self, kind: ComponentKind) -> Result<Value> {
        let active = self.active()?;
        let manifest = self.manifest(match kind {
            ComponentKind::Host => &active.host_manifest,
            ComponentKind::Amf => &active.amf_manifest,
        })?;
        let installed = component(&manifest, kind);
        let mut view = json!({"schemaVersion":"0.2", "component":kind, "available":true,
            "state":"idle", "installedVersion":installed.version, "installedBuild":installed.build,
            "rollbackAvailable":false});
        view.as_object_mut().expect("snapshot object").extend(
            self.component_pending_view(kind)?
                .as_object()
                .expect("pending object")
                .clone(),
        );
        Ok(view)
    }

    pub(crate) fn cached_component_index(
        &self,
        kind: ComponentKind,
    ) -> Result<Option<(ComponentFeed, SignedManifest)>> {
        let file = owned_path(&self.root, &index_name(kind))?;
        if !file.exists() {
            return Ok(None);
        }
        if fs::metadata(&file)?.len() > MAX_MANIFEST_BYTES as u64 {
            return Err(Error("component_feed_too_large".into()));
        }
        let raw = fs::read(file)?;
        let feed = component_feed::verify(&raw, self.trust_key(), kind, 0)?;
        let envelope: SignedManifest = serde_json::from_slice(&raw)?;
        Ok(Some((feed, envelope)))
    }

    /// Persist the authenticated high-water mark even when a later object fetch fails.
    /// Equal sequence numbers must describe the exact same signed payload.
    pub(crate) fn accept_component_index(
        &self,
        kind: ComponentKind,
        raw: &[u8],
    ) -> Result<ComponentFeed> {
        let _index = self.lock(&format!("component-index-{}.lock", kind.name()))?;
        let cached = self.cached_component_index(kind)?;
        let feed = component_feed::verify(
            raw,
            self.trust_key(),
            kind,
            cached.as_ref().map_or(0, |(feed, _)| feed.sequence),
        )?;
        let envelope: SignedManifest = serde_json::from_slice(raw)?;
        if cached
            .as_ref()
            .is_some_and(|(previous, previous_envelope)| {
                previous.sequence == feed.sequence && previous_envelope.payload != envelope.payload
            })
        {
            return Err(Error("component_feed_equivocation".into()));
        }
        let file = owned_path(&self.root, &index_name(kind))?;
        fs::create_dir_all(
            file.parent()
                .ok_or_else(|| Error("invalid_release_path".into()))?,
        )?;
        atomic_json(&file, &envelope)?;
        Ok(feed)
    }

    pub(crate) fn component_target(
        &self,
        kind: ComponentKind,
        entry: &Release,
        raw: &[u8],
    ) -> Result<Value> {
        let target = entry.verify_inventory(kind, raw, self.trust_key())?;
        let active = self.active()?;
        let host = self.manifest(&active.host_manifest)?;
        let amf = self.manifest(&active.amf_manifest)?;
        let (root, value) = match kind {
            ComponentKind::Host => (self.host_root(&host)?, &target.host),
            ComponentKind::Amf => (self.amf_root(&amf)?, &target.amf),
        };
        let destination = match kind {
            ComponentKind::Host => self.host_root(&target)?,
            ComponentKind::Amf => self.amf_root(&target)?,
        };
        let reused_bytes = reused(&[&root, &destination], value)?;
        let total: u64 = value.files.iter().map(|file| file.bytes).sum();
        let runtime_bytes = if kind == ComponentKind::Host {
            let runtime_root = self.runtime_root(&target)?;
            if target.runtime.files.iter().all(|file| {
                owned_path(&runtime_root, &file.path)
                    .is_ok_and(|path| verified_file(&path, file.bytes, &file.sha256))
            }) || verified_file(
                &owned_path(
                    &self.root,
                    &format!("cache/{}.zip", target.runtime.archive.sha256),
                )?,
                target.runtime.archive.bytes,
                &target.runtime.archive.sha256,
            ) {
                0
            } else {
                target.runtime.archive.bytes
            }
        } else {
            0
        };
        let (id, _) = self.store_manifest(raw)?;
        if self.active()? != active {
            return Err(Error("active_selection_changed".into()));
        }
        Ok(
            json!({"manifestId":id, "version":value.version, "build":value.build, "releaseUrl":entry.release_url,
            "activation":entry.activation_for(kind, &amf), "downloadBytes":total - reused_bytes + runtime_bytes,
            "reusedBytes":reused_bytes, "runtimeBytes":runtime_bytes}),
        )
    }

    pub async fn check_component(&self, kind: ComponentKind, transfer: &Transfer) -> Result<Value> {
        let _check = self.lock(&format!("component-check-{}.lock", kind.name()))?;
        let active = self.active()?;
        let host = self.manifest(&active.host_manifest)?;
        let amf = self.manifest(&active.amf_manifest)?;
        let raw = transfer.manifest(kind.feed()).await?;
        transfer.checkpoint()?;
        let feed = self.accept_component_index(kind, &raw)?;
        self.invalidate_stale_component_pending(kind, &active)?;
        let selection = feed.select(&host, &amf)?;
        let mut view = self.component_snapshot(kind)?;
        if let Some(latest) = selection.latest {
            view["latestVersion"] = json!(latest.version);
            if kind == ComponentKind::Amf
                && selection.target.is_none()
                && !semver::VersionReq::parse(&latest.compatibility.host)
                    .expect("validated host requirement")
                    .matches(
                        &semver::Version::parse(&host.host.version)
                            .expect("validated host version"),
                    )
                && semver::Version::parse(&latest.version)
                    .expect("validated version")
                    .cmp_precedence(
                        &semver::Version::parse(&amf.amf.version)
                            .expect("validated installed version"),
                    )
                    .is_gt()
            {
                view["requiresHost"] = json!(latest.compatibility.host);
            }
        }
        if let Some(entry) = selection.target.filter(|_| view["state"] == "idle") {
            let mut authenticated = None;
            for url in &entry.manifest.sources {
                transfer.checkpoint()?;
                if let Ok(raw) = transfer.manifest(url).await {
                    if entry.verify_inventory(kind, &raw, self.trust_key()).is_ok() {
                        authenticated = Some(raw);
                        break;
                    }
                }
            }
            transfer.checkpoint()?;
            let raw =
                authenticated.ok_or_else(|| Error("component_manifest_unavailable".into()))?;
            view["target"] = self.component_target(kind, entry, &raw)?;
        }
        if self.active()? != active {
            return Err(Error("active_selection_changed".into()));
        }
        Ok(view)
    }
}
