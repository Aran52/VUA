//! Durable, independent discovery. Fixed feeds and opaque inventory identities stay native.
use crate::{
    component_feed::{
        self, ComponentFeed, ComponentKind, CompressedTransfer, Release, SCHEMA, SCHEMA_V03,
    },
    layout::{Active, Installation},
    manifest::{Component, Manifest, SignedManifest, MAX_MANIFEST_BYTES},
    paths::{atomic_json, owned_path},
    transfer::{verified_file, Transfer},
    Error, Result,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

fn index_name_v02(kind: ComponentKind) -> String {
    format!("updates-v02/{}-index.json", kind.name())
}

fn index_name_v03(kind: ComponentKind) -> String {
    format!("updates-v03/{}-index.json", kind.name())
}

fn transfer_manifest_name(sha256: &str) -> String {
    format!("updates-v03/transfers/{sha256}.json")
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

    fn cached_component_index_at(
        &self,
        kind: ComponentKind,
        relative: &str,
        expected_schema: &str,
    ) -> Result<Option<(ComponentFeed, SignedManifest)>> {
        let file = owned_path(&self.root, relative)?;
        if !file.exists() {
            return Ok(None);
        }
        if fs::metadata(&file)?.len() > MAX_MANIFEST_BYTES as u64 {
            return Err(Error("component_feed_too_large".into()));
        }
        let raw = fs::read(file)?;
        let feed = component_feed::verify(&raw, self.trust_key(), kind, 0)?;
        if feed.schema_version != expected_schema {
            return Err(Error("unsupported_component_feed".into()));
        }
        let envelope: SignedManifest = serde_json::from_slice(&raw)?;
        Ok(Some((feed, envelope)))
    }

    pub(crate) fn cached_component_index(
        &self,
        kind: ComponentKind,
    ) -> Result<Option<(ComponentFeed, SignedManifest)>> {
        if let Some(v03) =
            self.cached_component_index_at(kind, &index_name_v03(kind), SCHEMA_V03)?
        {
            return Ok(Some(v03));
        }
        self.cached_component_index_at(kind, &index_name_v02(kind), SCHEMA)
    }

    /// Persist the authenticated high-water mark even when a later object fetch fails.
    /// Equal sequence numbers within the same schema must describe the exact same signed payload.
    pub(crate) fn accept_component_index(
        &self,
        kind: ComponentKind,
        raw: &[u8],
    ) -> Result<ComponentFeed> {
        let _index = self.lock(&format!("component-index-{}.lock", kind.name()))?;
        let cached_v02 = self.cached_component_index_at(kind, &index_name_v02(kind), SCHEMA)?;
        let cached_v03 = self.cached_component_index_at(kind, &index_name_v03(kind), SCHEMA_V03)?;
        let minimum_sequence = cached_v02
            .as_ref()
            .map_or(0, |(feed, _)| feed.sequence)
            .max(cached_v03.as_ref().map_or(0, |(feed, _)| feed.sequence));
        let feed = component_feed::verify(raw, self.trust_key(), kind, minimum_sequence)?;
        if feed.schema_version == SCHEMA && cached_v03.is_some() {
            return Err(Error("component_feed_downgraded".into()));
        }
        let envelope: SignedManifest = serde_json::from_slice(raw)?;
        let same_schema_cached = if feed.schema_version == SCHEMA_V03 {
            &cached_v03
        } else {
            &cached_v02
        };
        if same_schema_cached
            .as_ref()
            .is_some_and(|(previous, previous_envelope)| {
                previous.sequence == feed.sequence && previous_envelope.payload != envelope.payload
            })
        {
            return Err(Error("component_feed_equivocation".into()));
        }
        let target_name = if feed.schema_version == SCHEMA_V03 {
            index_name_v03(kind)
        } else {
            index_name_v02(kind)
        };
        let file = owned_path(&self.root, &target_name)?;
        fs::create_dir_all(
            file.parent()
                .ok_or_else(|| Error("invalid_release_path".into()))?,
        )?;
        atomic_json(&file, &envelope)?;
        Ok(feed)
    }

    pub(crate) async fn fetch_component_feed_raw(
        &self,
        kind: ComponentKind,
        transfer: &Transfer,
    ) -> Result<Vec<u8>> {
        let has_cached_v03 = self
            .cached_component_index_at(kind, &index_name_v03(kind), SCHEMA_V03)?
            .is_some();
        match transfer.manifest(kind.feed_v03()).await {
            Ok(raw)
                if has_cached_v03
                    || raw.iter().copied().find(|b| !b.is_ascii_whitespace()) == Some(b'{') =>
            {
                Ok(raw)
            }
            Ok(_) => transfer.manifest(kind.feed()).await,
            Err(error) if error.0 == "feed_unavailable" && !has_cached_v03 => {
                transfer.manifest(kind.feed()).await
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn load_cached_compressed_transfer(
        &self,
        kind: ComponentKind,
        entry: &Release,
        inventory: &Manifest,
    ) -> Result<Option<CompressedTransfer>> {
        let Some(reference) = &entry.compressed_transfer else {
            return Ok(None);
        };
        let file = owned_path(&self.root, &transfer_manifest_name(&reference.sha256))?;
        if !file.exists() {
            return Ok(None);
        }
        let raw = fs::read(file)?;
        entry
            .verify_transfer_manifest(kind, inventory, &raw, self.trust_key())
            .map(Some)
    }

    pub(crate) async fn ensure_compressed_transfer(
        &self,
        kind: ComponentKind,
        entry: &Release,
        inventory: &Manifest,
        transfer: &Transfer,
    ) -> Result<Option<CompressedTransfer>> {
        let Some(reference) = &entry.compressed_transfer else {
            return Ok(None);
        };
        if let Ok(Some(cached)) = self.load_cached_compressed_transfer(kind, entry, inventory) {
            return Ok(Some(cached));
        }
        for url in &reference.sources {
            transfer.checkpoint()?;
            if let Ok(raw) = transfer.manifest(url).await {
                if let Ok(verified) =
                    entry.verify_transfer_manifest(kind, inventory, &raw, self.trust_key())
                {
                    let file = owned_path(&self.root, &transfer_manifest_name(&reference.sha256))?;
                    fs::create_dir_all(
                        file.parent()
                            .ok_or_else(|| Error("invalid_release_path".into()))?,
                    )?;
                    let envelope: SignedManifest = serde_json::from_slice(&raw)?;
                    atomic_json(&file, &envelope)?;
                    return Ok(Some(verified));
                }
            }
        }
        transfer.checkpoint()?;
        Err(Error("component_transfer_unavailable".into()))
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
        let roots = [&root as &Path, &destination as &Path];
        let reused_bytes = reused(&roots, value)?;
        let total: u64 = value.files.iter().map(|file| file.bytes).sum();
        let component_download_bytes =
            if let Some(compressed) = self.load_cached_compressed_transfer(kind, entry, &target)? {
                compressed
                    .files
                    .iter()
                    .filter(|file| {
                        !roots.iter().any(|candidate| {
                            owned_path(candidate, &file.path)
                                .is_ok_and(|path| verified_file(&path, file.bytes, &file.sha256))
                        })
                    })
                    .map(|file| file.compressed_bytes)
                    .sum()
            } else {
                total - reused_bytes
            };
        let runtime_bytes = if kind == ComponentKind::Host {
            let runtime_root = self.runtime_root(&target)?;
            if self
                .verify_files(&runtime_root, &target.runtime.files)
                .is_ok()
                || verified_file(
                    &owned_path(
                        &self.root,
                        &format!("cache/{}.zip", target.runtime.archive.sha256),
                    )?,
                    target.runtime.archive.bytes,
                    &target.runtime.archive.sha256,
                )
            {
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
            "activation":entry.activation_for(kind, &amf), "downloadBytes":component_download_bytes + runtime_bytes,
            "reusedBytes":reused_bytes, "runtimeBytes":runtime_bytes}),
        )
    }

    pub(crate) fn accept_component_check(
        &self,
        kind: ComponentKind,
        raw: &[u8],
        active: &Active,
    ) -> Result<(ComponentFeed, Value)> {
        let feed = self.accept_component_index(kind, raw)?;
        let withdrawn = self.invalidate_stale_component_pending(kind, active, &feed)?;
        let mut view = self.component_snapshot(kind)?;
        if withdrawn {
            view["error"] = json!("update_withdrawn");
        }
        Ok((feed, view))
    }

    pub async fn check_component(&self, kind: ComponentKind, transfer: &Transfer) -> Result<Value> {
        let _check = self.lock(&format!("component-check-{}.lock", kind.name()))?;
        let active = self.active()?;
        let host = self.manifest(&active.host_manifest)?;
        let amf = self.manifest(&active.amf_manifest)?;
        let raw = self.fetch_component_feed_raw(kind, transfer).await?;
        transfer.checkpoint()?;
        let (feed, mut view) = self.accept_component_check(kind, &raw, &active)?;
        let selection = feed.select(&host, &amf)?;
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
        if let Some(entry) = selection
            .target
            .filter(|_| view["state"] == "idle" && view["error"] != "update_withdrawn")
        {
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
            let verified_inventory = entry.verify_inventory(kind, &raw, self.trust_key())?;
            self.ensure_compressed_transfer(kind, entry, &verified_inventory, transfer)
                .await?;
            view["target"] = self.component_target(kind, entry, &raw)?;
        }
        if self.active()? != active {
            return Err(Error("active_selection_changed".into()));
        }
        Ok(view)
    }
}
