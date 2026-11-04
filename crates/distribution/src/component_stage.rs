//! Component staging and restart transactions. No live executable is overwritten.
use crate::{
    component_feed::{self, Activation, ComponentKind, Release},
    layout::{Active, Installation},
    manifest::{Manifest, SignedManifest},
    paths::{atomic_json, owned_path},
    transfer::Transfer,
    Error, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ComponentPending {
    pub schema_version: String,
    pub component: ComponentKind,
    pub base: Active,
    pub manifest: String,
    pub index: SignedManifest,
    pub state: String,
    pub requested: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Retained {
    schema_version: String,
    component: ComponentKind,
    before: String,
    after: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivationJournal {
    pub schema_version: String,
    pub before: Active,
    pub next: Active,
    pub state: String,
}
fn name(kind: ComponentKind, suffix: &str) -> String {
    format!("updates-v02/{}-{suffix}.json", kind.name())
}
fn selected(pair: &Active, kind: ComponentKind) -> &str {
    match kind {
        ComponentKind::Host => &pair.host_manifest,
        ComponentKind::Amf => &pair.amf_manifest,
    }
}
fn next_pair(base: &Active, kind: ComponentKind, id: &str) -> Active {
    let mut next = base.clone();
    match kind {
        ComponentKind::Host => next.host_manifest = id.into(),
        ComponentKind::Amf => next.amf_manifest = id.into(),
    };
    next
}

impl Installation {
    pub fn amf_hot_preflight(&self, id: &str, restore: bool) -> Result<Value> {
        let active = self.active()?;
        let (_, old) = self.validate_pair(&active)?;
        let mut advertised_reload = restore;
        let next = if restore {
            self.component_restore_target(ComponentKind::Amf)?
        } else {
            let (pending, entry) = self
                .component_pending(ComponentKind::Amf)?
                .ok_or_else(|| Error("update_not_staged".into()))?;
            if pending.state != "staged" || pending.base != active {
                return Err(Error("update_not_staged".into()));
            }
            advertised_reload = entry.activation == Activation::Reload;
            next_pair(&active, ComponentKind::Amf, &pending.manifest)
        };
        if (id.is_empty() && !restore) || (!id.is_empty() && next.amf_manifest != id) {
            return Err(Error("invalid_update_identity".into()));
        }
        let id = next.amf_manifest.as_str();
        let target = self.manifest(id)?;
        self.verify_files(&self.amf_root(&target)?, &target.amf.files)?;
        let metadata_file = target
            .amf
            .files
            .iter()
            .find(|file| file.path == "ui/component.json");
        let mut activation = "restart";
        if let Some(file) = metadata_file.filter(|file| file.bytes <= 65536) {
            let metadata: Value = serde_json::from_slice(&fs::read(owned_path(
                &self.amf_root(&target)?,
                &file.path,
            )?)?)?;
            if metadata["schemaVersion"] != "0.1"
                || metadata["component"] != "amf"
                || metadata["version"] != target.amf.version
                || metadata["build"] != target.amf.build
                || metadata["uiAbi"] != target.compatibility.ui_abi
                || metadata["bdlWrite"] != target.compatibility.bdl_write
            {
                return Err(Error("amf_generation_identity_mismatch".into()));
            }
            if advertised_reload
                && metadata["hotReloadAbi"] == "amf-hot/0.1"
                && target.compatibility.bdl_write == old.compatibility.bdl_write
            {
                let mut probe = std::process::Command::new(owned_path(
                    &self.amf_root(&target)?,
                    "provider/vua-amf-provider.exe",
                )?);
                probe.arg("--component-identity");
                for (name, _) in std::env::vars_os() {
                    if name
                        .to_string_lossy()
                        .to_ascii_uppercase()
                        .starts_with("VUA_")
                    {
                        probe.env_remove(name);
                    }
                }
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    probe.creation_flags(0x08000000);
                }
                let result = probe.output()?;
                let identity: Value = serde_json::from_slice(&result.stdout)
                    .map_err(|_| Error("amf_generation_identity_mismatch".into()))?;
                if !result.status.success()
                    || identity["schemaVersion"] != "0.1"
                    || identity["component"] != "amf"
                    || identity["version"] != target.amf.version
                    || identity["build"] != target.amf.build
                    || identity["bdlWrite"] != target.compatibility.bdl_write
                    || identity["serviceBuildId"]
                        != format!("amf/{}@{}", target.amf.version, target.amf.build)
                {
                    return Err(Error("amf_generation_identity_mismatch".into()));
                }
                activation = "reload";
            }
        }
        Ok(
            json!({"schemaVersion":"0.1", "manifestId":id, "version":target.amf.version, "build":target.amf.build, "bdlWrite":target.compatibility.bdl_write, "activation":activation}),
        )
    }
    pub fn begin_amf_hot(&self, id: &str, restore: bool) -> Result<Value> {
        let _operation = self.lock("operation.lock")?;
        let _stage = self.lock("component-stage-amf.lock")?;
        let target = self.amf_hot_preflight(id, restore)?;
        if target["activation"] != "reload" {
            return Err(Error("restart_required".into()));
        }
        let active = self.active()?;
        if !restore
            && !self
                .component_pending(ComponentKind::Amf)?
                .is_some_and(|(pending, _)| pending.requested)
        {
            return Err(Error("activation_requires_choice".into()));
        }
        let next = next_pair(&active, ComponentKind::Amf, id);
        let mut journal = ActivationJournal {
            schema_version: "0.2".into(),
            before: active.clone(),
            next: next.clone(),
            state: "prepared".into(),
        };
        self.save_update("updates-v02/activation.json", &journal)?;
        self.save_update("updates-v02/hot-intent.json", &journal)?;
        self.prepare_data_recovery(&active, &next)?;
        self.save_update(
            &name(ComponentKind::Amf, "previous"),
            &Retained {
                schema_version: "0.2".into(),
                component: ComponentKind::Amf,
                before: active.amf_manifest.clone(),
                after: id.into(),
            },
        )?;
        atomic_json(&owned_path(&self.root, "previous.json")?, &active)?;
        atomic_json(&owned_path(&self.root, "active.json")?, &next)?;
        journal.state = "verifying".into();
        self.save_update("updates-v02/activation.json", &journal)?;
        Ok(target)
    }
    pub fn finish_amf_hot(&self, id: &str) -> Result<Value> {
        let _operation = self.lock("operation.lock")?;
        let mut journal = self
            .activation_journal()?
            .ok_or_else(|| Error("invalid_activation_journal".into()))?;
        if journal.next != self.active()?
            || journal.next.amf_manifest != id
            || journal.before.host_manifest != journal.next.host_manifest
            || journal.state != "verifying"
        {
            return Err(Error("update_health_identity_mismatch".into()));
        }
        let target = self.manifest(id)?;
        self.verify_files(&self.amf_root(&target)?, &target.amf.files)?;
        journal.state = "healthy".into();
        self.save_update("updates-v02/activation.json", &journal)?;
        // Health is the commit point. Cleanup cannot turn a committed generation into failure.
        self.commit_data_recovery(&journal.before, &journal.next)?;
        if self
            .component_pending(ComponentKind::Amf)
            .ok()
            .flatten()
            .is_some_and(|(pending, _)| pending.manifest == id)
        {
            let _ = fs::remove_file(owned_path(
                &self.root,
                &name(ComponentKind::Amf, "pending"),
            )?);
        }
        let _ = fs::remove_file(owned_path(&self.root, "updates-v02/hot-intent.json")?);
        Ok(json!({"schemaVersion":"0.1", "manifestId":id, "committed":true}))
    }
    pub fn recover_amf_hot(&self) -> Result<Value> {
        let _operation = self.lock("operation.lock")?;
        let snapshot = |healthy: bool| -> Result<Value> {
            let active = self.active()?;
            let amf = self.manifest(&active.amf_manifest)?;
            Ok(
                json!({"schemaVersion":"0.1", "manifestId":active.amf_manifest, "version":amf.amf.version, "build":amf.amf.build, "healthy":healthy}),
            )
        };
        let Some(mut journal) = self.activation_journal()? else {
            return snapshot(true);
        };
        if journal.state == "healthy" || journal.state == "recovered" {
            if journal.state == "healthy" {
                self.commit_data_recovery(&journal.before, &journal.next)?;
            }
            return snapshot(journal.state == "healthy");
        }
        let active = self.active()?;
        if journal.before.host_manifest != journal.next.host_manifest
            || active != journal.next && active != journal.before
        {
            return Err(Error("restart_required".into()));
        }
        let old = self.manifest(&journal.before.amf_manifest)?;
        let next = self.manifest(&journal.next.amf_manifest)?;
        if old.compatibility.bdl_write != next.compatibility.bdl_write {
            return Err(Error("rollback_requires_data_repair".into()));
        }
        self.verify_files(&self.amf_root(&old)?, &old.amf.files)?;
        self.recover_data_recovery(&journal.before, &journal.next)?;
        atomic_json(&owned_path(&self.root, "active.json")?, &journal.before)?;
        journal.state = "recovered".into();
        self.save_update("updates-v02/activation.json", &journal)?;
        snapshot(false)
    }
    pub fn require_component_launcher(&self) -> Result<()> {
        self.launcher_capabilities().map(|_| ())
    }
    pub(crate) fn require_data_launcher(&self) -> Result<()> {
        if self.launcher_capabilities()?["dataRecovery"] != "0.1" {
            return Err(Error("installer_required".into()));
        }
        Ok(())
    }
    fn launcher_capabilities(&self) -> Result<Value> {
        let launcher = owned_path(&self.root, "VUA.exe")?;
        if !launcher.is_file() {
            return Err(Error("installer_required".into()));
        }
        let hash = crate::paths::file_digest(&launcher)?;
        let mut authenticated = false;
        for entry in fs::read_dir(owned_path(&self.root, "manifests")?)
            .map_err(|_| Error("installer_required".into()))?
        {
            let name = entry?.file_name().to_string_lossy().into_owned();
            let Some(id) = name.strip_suffix(".json") else {
                continue;
            };
            if let Ok(manifest) = self.manifest(id) {
                authenticated |= manifest.host.files.iter().any(|file| {
                    file.path == "launcher/VUA.exe"
                        && file.sha256 == hash
                        && fs::metadata(&launcher)
                            .is_ok_and(|metadata| metadata.len() == file.bytes)
                });
            }
        }
        if !authenticated {
            return Err(Error("installer_required".into()));
        }
        let mut probe = std::process::Command::new(launcher);
        probe.args(["capabilities", "--headless"]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            probe.creation_flags(0x08000000);
        }
        let result = probe.output()?;
        let value: Value = serde_json::from_slice(&result.stdout)
            .map_err(|_| Error("installer_required".into()))?;
        if !result.status.success()
            || value["schemaVersion"] != "0.1"
            || value["kind"] != "result"
            || value["value"]["componentUpdates"] != "0.2"
            || value["value"]["recovery"] != "0.2"
            || value["value"]["updatePlans"] != "0.1"
        {
            return Err(Error("installer_required".into()));
        }
        Ok(value["value"].clone())
    }
    pub(crate) fn commit_restart(&self, before: &Active, next: &Active) -> Result<()> {
        self.admit_data_update(before, next)?;
        let (host, amf) = self.validate_pair(next)?;
        let (_, old_amf) = self.validate_pair(before)?;
        if !amf
            .compatibility
            .bdl_read
            .contains(&old_amf.compatibility.bdl_write)
        {
            return Err(Error("unsupported_data_migration".into()));
        }
        let mut journal = ActivationJournal {
            schema_version: "0.2".into(),
            before: before.clone(),
            next: next.clone(),
            state: "prepared".into(),
        };
        self.save_update("updates-v02/activation.json", &journal)?;
        self.prepare_data_recovery(before, next)?;
        for kind in [ComponentKind::Host, ComponentKind::Amf] {
            if selected(before, kind) != selected(next, kind) {
                self.save_update(
                    &name(kind, "previous"),
                    &Retained {
                        schema_version: "0.2".into(),
                        component: kind,
                        before: selected(before, kind).into(),
                        after: selected(next, kind).into(),
                    },
                )?;
            }
        }
        self.install_bootstrap(&host)?;
        atomic_json(&owned_path(&self.root, "previous.json")?, before)?;
        atomic_json(&owned_path(&self.root, "active.json")?, next)?;
        journal.state = "verifying".into();
        self.save_update("updates-v02/activation.json", &journal)
    }
    pub(crate) fn activation_journal(&self) -> Result<Option<ActivationJournal>> {
        if !owned_path(&self.root, "updates-v02/activation.json")?.exists() {
            return Ok(None);
        }
        let journal: ActivationJournal = self.update_json("updates-v02/activation.json")?;
        if journal.schema_version != "0.2"
            || !["prepared", "verifying", "healthy", "recovered"].contains(&journal.state.as_str())
        {
            return Err(Error("invalid_activation_journal".into()));
        }
        self.validate_pair(&journal.before)?;
        self.validate_pair(&journal.next)?;
        Ok(Some(journal))
    }
    pub(crate) fn admit_data_update(&self, before: &Active, next: &Active) -> Result<()> {
        self.admit_data_recovery(before, next)
    }
    pub fn acknowledge_component_health(&self, host_build: &str, amf_build: &str) -> Result<()> {
        let _operation = self.lock("operation.lock")?;
        let Some(mut journal) = self.activation_journal()? else {
            return Ok(());
        };
        if journal.state == "healthy" {
            return self.commit_data_recovery(&journal.before, &journal.next);
        }
        if journal.state != "verifying" {
            return Ok(());
        }
        if owned_path(&self.root, "updates-v02/hot-intent.json")?.exists() {
            let intent: ActivationJournal = self.update_json("updates-v02/hot-intent.json")?;
            if intent.before == journal.before && intent.next == journal.next {
                return Err(Error("module_health_pending".into()));
            }
        }
        let active = self.active()?;
        let (host, amf) = self.validate_pair(&active)?;
        if active != journal.next || host.host.build != host_build || amf.amf.build != amf_build {
            return Err(Error("update_health_identity_mismatch".into()));
        }
        journal.state = "healthy".into();
        self.save_update("updates-v02/activation.json", &journal)?;
        self.commit_data_recovery(&journal.before, &journal.next)
    }
    pub fn recover_component_activation(&self) -> Result<bool> {
        let _operation = self.lock("operation.lock")?;
        let _running = self.lock("running.lock")?;
        let Some(mut journal) = self.activation_journal()? else {
            return Ok(false);
        };
        if journal.state == "healthy" || journal.state == "recovered" {
            return Ok(false);
        }
        let active = self.active()?;
        if active != journal.next && active != journal.before {
            return Err(Error("active_selection_changed".into()));
        }
        let (host, amf) = self.validate_pair(&journal.before)?;
        let (_, current_amf) = self.validate_pair(&active)?;
        let data_restored = self.recover_data_recovery(&journal.before, &journal.next)?;
        if !data_restored
            && !amf
                .compatibility
                .bdl_read
                .contains(&current_amf.compatibility.bdl_write)
        {
            return Err(Error("rollback_requires_data_repair".into()));
        }
        self.verify_files(&self.host_root(&host)?, &host.host.files)?;
        self.verify_files(&self.runtime_root(&host)?, &host.runtime.files)?;
        self.verify_files(&self.amf_root(&amf)?, &amf.amf.files)?;
        self.install_bootstrap(&host)?;
        atomic_json(&owned_path(&self.root, "active.json")?, &journal.before)?;
        journal.state = "recovered".into();
        self.save_update("updates-v02/activation.json", &journal)?;
        Ok(true)
    }
    fn component_restore_target(&self, kind: ComponentKind) -> Result<Active> {
        let retained: Retained = self.update_json(&name(kind, "previous"))?;
        let active = self.active()?;
        if retained.schema_version != "0.2"
            || retained.component != kind
            || selected(&active, kind) != retained.after
            || retained.before == retained.after
        {
            return Err(Error("no_retained_version".into()));
        }
        let next = next_pair(&active, kind, &retained.before);
        let (host, amf) = self.validate_pair(&next)?;
        let (_, current_amf) = self.validate_pair(&active)?;
        if !amf
            .compatibility
            .bdl_read
            .contains(&current_amf.compatibility.bdl_write)
        {
            return Err(Error("rollback_requires_data_repair".into()));
        }
        self.verify_files(&self.host_root(&host)?, &host.host.files)?;
        self.verify_files(&self.runtime_root(&host)?, &host.runtime.files)?;
        self.verify_files(&self.amf_root(&amf)?, &amf.amf.files)?;
        Ok(next)
    }
    pub fn restore_component(&self, kind: ComponentKind) -> Result<()> {
        let _operation = self.lock("operation.lock")?;
        let _running = self.lock("running.lock")?;
        let before = self.active()?;
        let next = self.component_restore_target(kind)?;
        self.commit_restart(&before, &next)
    }
    pub(crate) fn component_pending_view(&self, kind: ComponentKind) -> Result<Value> {
        let mut view = json!({"rollbackAvailable":self.component_restore_target(kind).is_ok()});
        if let Some(journal) = self.activation_journal()? {
            if journal.state == "recovered"
                && journal.before == self.active()?
                && selected(&journal.before, kind) != selected(&journal.next, kind)
            {
                view["error"] = json!("update_recovered");
            }
        }
        if let Some((pending, entry)) = self.component_pending(kind)? {
            let raw = fs::read(owned_path(
                &self.root,
                &format!("manifests/{}.json", pending.manifest),
            )?)?;
            view["target"] = self.component_target(kind, &entry, &raw)?;
            view["state"] = json!(if pending.base == self.active()? {
                pending.state.as_str()
            } else {
                "interrupted"
            });
            if pending.base != self.active()? {
                view["error"] = json!("active_selection_changed");
            }
        }
        Ok(view)
    }
    pub(crate) fn component_retained_pairs(&self) -> Result<Vec<Active>> {
        let mut pairs = Vec::new();
        for kind in [ComponentKind::Host, ComponentKind::Amf] {
            if let Some((pending, _)) = self.component_pending(kind)? {
                pairs.push(next_pair(&pending.base, kind, &pending.manifest));
                pairs.push(pending.base);
            }
            if let Ok(retained) = self.update_json::<Retained>(&name(kind, "previous")) {
                let mut pair = self.active()?;
                match kind {
                    ComponentKind::Host => pair.host_manifest = retained.before,
                    ComponentKind::Amf => pair.amf_manifest = retained.before,
                };
                pairs.push(pair);
            }
        }
        if let Some(journal) = self.activation_journal()? {
            pairs.push(journal.before);
            pairs.push(journal.next);
        }
        Ok(pairs)
    }
}

impl Installation {
    pub(crate) fn update_json<T: serde::de::DeserializeOwned>(&self, relative: &str) -> Result<T> {
        let file = owned_path(&self.root, relative)?;
        if fs::metadata(&file)?.len() > 16 * 1024 * 1024 {
            return Err(Error("invalid_update_record".into()));
        }
        Ok(serde_json::from_slice(&fs::read(file)?)?)
    }
    pub(crate) fn save_update(&self, relative: &str, value: &impl Serialize) -> Result<()> {
        let file = owned_path(&self.root, relative)?;
        fs::create_dir_all(
            file.parent()
                .ok_or_else(|| Error("invalid_release_path".into()))?,
        )?;
        atomic_json(&file, value)
    }
    pub(crate) fn component_pending(
        &self,
        kind: ComponentKind,
    ) -> Result<Option<(ComponentPending, Release)>> {
        if !owned_path(&self.root, &name(kind, "pending"))?.exists() {
            return Ok(None);
        }
        let pending: ComponentPending = self.update_json(&name(kind, "pending"))?;
        if pending.schema_version != "0.2"
            || pending.component != kind
            || !["interrupted", "staged"].contains(&pending.state.as_str())
        {
            return Err(Error("invalid_pending_update".into()));
        }
        let (host, amf) = self.validate_pair(&pending.base)?;
        let feed = component_feed::verify(
            &serde_json::to_vec(&pending.index)?,
            self.trust_key(),
            kind,
            0,
        )?;
        let entry = feed
            .select(&host, &amf)?
            .target
            .filter(|entry| entry.manifest.sha256 == pending.manifest)
            .cloned()
            .ok_or_else(|| Error("invalid_update_identity".into()))?;
        let raw = fs::read(owned_path(
            &self.root,
            &format!("manifests/{}.json", pending.manifest),
        )?)?;
        entry.verify_inventory(kind, &raw, self.trust_key())?;
        Ok(Some((pending, entry)))
    }
    pub(crate) fn invalidate_stale_component_pending(
        &self,
        kind: ComponentKind,
        active: &Active,
    ) -> Result<()> {
        let _stage = self.lock(&format!("component-stage-{}.lock", kind.name()))?;
        let failed = self
            .activation_journal()?
            .filter(|journal| journal.state == "recovered" && &journal.before == active);
        if self.component_pending(kind)?.is_some_and(|(pending, _)| {
            &pending.base != active
                || failed
                    .as_ref()
                    .is_some_and(|journal| selected(&journal.next, kind) == pending.manifest)
        }) {
            // Only obsolete intent is cleared; downloaded immutable files remain reusable.
            fs::remove_file(owned_path(&self.root, &name(kind, "pending"))?)?;
        }
        Ok(())
    }
    pub(crate) fn validate_pair(&self, pair: &Active) -> Result<(Manifest, Manifest)> {
        if pair.schema_version != "0.1" {
            return Err(Error("unsupported_installed_layout".into()));
        }
        let host = self.manifest(&pair.host_manifest)?;
        let amf = self.manifest(&pair.amf_manifest)?;
        amf.compatible_amf(&host)?;
        Ok((host, amf))
    }
    fn checked_component(
        &self,
        kind: ComponentKind,
        id: &str,
    ) -> Result<(SignedManifest, Release)> {
        let (feed, index) = self
            .cached_component_index(kind)?
            .ok_or_else(|| Error("update_check_required".into()))?;
        let active = self.active()?;
        let (host, amf) = self.validate_pair(&active)?;
        let selection = feed.select(&host, &amf)?;
        let entry = selection
            .target
            .filter(|entry| entry.manifest.sha256 == id)
            .cloned()
            .ok_or_else(|| Error("update_target_changed".into()))?;
        Ok((index, entry))
    }
    pub async fn stage_component(
        &self,
        kind: ComponentKind,
        id: &str,
        transfer: &mut Transfer,
        local_pack: Option<&Path>,
    ) -> Result<Value> {
        let _stage = self.lock(&format!("component-stage-{}.lock", kind.name()))?;
        let base = self.active()?;
        let (host, old_amf) = self.validate_pair(&base)?;
        let (index, entry) = if let Some((pending, entry)) = self
            .component_pending(kind)?
            .filter(|(pending, _)| pending.manifest == id && pending.base == base)
        {
            (pending.index, entry)
        } else {
            self.checked_component(kind, id)?
        };
        let raw = fs::read(owned_path(&self.root, &format!("manifests/{id}.json"))?)?;
        let target = entry.verify_inventory(kind, &raw, self.trust_key())?;
        let next = next_pair(&base, kind, id);
        let (_, next_amf) = self.validate_pair(&next)?;
        if !next_amf
            .compatibility
            .bdl_read
            .contains(&old_amf.compatibility.bdl_write)
        {
            return Err(Error("unsupported_data_migration".into()));
        }
        let (value, old_root, output) = match kind {
            ComponentKind::Host => (
                &target.host,
                self.host_root(&host)?,
                self.host_root(&target)?,
            ),
            ComponentKind::Amf => (
                &target.amf,
                self.amf_root(&old_amf)?,
                self.amf_root(&target)?,
            ),
        };
        if output == old_root && self.verify_files(&output, &value.files).is_err() {
            return Err(Error("active_files_need_repair".into()));
        }
        let runtime_root = self.runtime_root(&target)?;
        let runtime_missing = kind == ComponentKind::Host
            && self
                .verify_files(&runtime_root, &target.runtime.files)
                .is_err();
        let needed = value
            .pack
            .bytes
            .saturating_mul(2)
            .saturating_add(32 * 1024 * 1024)
            .saturating_add(if runtime_missing {
                target.runtime.archive.bytes.saturating_add(
                    target
                        .runtime
                        .files
                        .iter()
                        .map(|file| file.bytes)
                        .sum::<u64>(),
                )
            } else {
                0
            });
        if crate::layout::free_space(&self.root)? < needed {
            return Err(Error("insufficient_disk_space".into()));
        }
        let mut pending = ComponentPending {
            schema_version: "0.2".into(),
            component: kind,
            base: base.clone(),
            manifest: id.into(),
            index,
            state: "interrupted".into(),
            requested: false,
        };
        self.save_update(&name(kind, "pending"), &pending)?;
        fs::create_dir_all(owned_path(&self.root, "cache")?)?;
        transfer
            .component(
                &value.files,
                &value.pack,
                Some(&old_root),
                &output,
                local_pack,
                &owned_path(&self.root, &format!("cache/{}.bin", value.pack.sha256))?,
            )
            .await?;
        if runtime_missing {
            let cache = owned_path(
                &self.root,
                &format!("cache/{}.zip", target.runtime.archive.sha256),
            )?;
            transfer.archive(&target.runtime.archive, &cache).await?;
            // Repair a damaged shared runtime only after the running app exits.
            if target.runtime.archive.sha256 != host.runtime.archive.sha256 {
                self.runtime(
                    &target,
                    &owned_path(&self.root, "cache/no-install-input")?,
                    transfer,
                )
                .await?;
            }
        }
        transfer.checkpoint()?;
        if self.active()? != base {
            return Err(Error("active_selection_changed".into()));
        }
        pending.state = "staged".into();
        self.save_update(&name(kind, "pending"), &pending)?;
        self.component_snapshot(kind)
    }
    pub fn request_component_activation(&self, kind: ComponentKind) -> Result<()> {
        let _operation = self.lock("operation.lock")?;
        let (mut pending, _) = self
            .component_pending(kind)?
            .ok_or_else(|| Error("update_not_staged".into()))?;
        if pending.state != "staged" || pending.base != self.active()? {
            return Err(Error("update_not_staged".into()));
        }
        self.admit_data_update(
            &pending.base,
            &next_pair(&pending.base, kind, &pending.manifest),
        )?;
        pending.requested = true;
        self.save_update(&name(kind, "pending"), &pending)
    }
    pub async fn activate_component(
        &self,
        kind: ComponentKind,
        transfer: &mut Transfer,
    ) -> Result<()> {
        let _operation = self.lock("operation.lock")?;
        let _running = self.lock("running.lock")?;
        let (pending, _) = self
            .component_pending(kind)?
            .ok_or_else(|| Error("update_not_staged".into()))?;
        if !pending.requested || pending.state != "staged" {
            return Err(Error("activation_requires_choice".into()));
        }
        if self.active()? != pending.base {
            return Err(Error("active_selection_changed".into()));
        }
        let next = next_pair(&pending.base, kind, &pending.manifest);
        let (host, amf) = self.validate_pair(&next)?;
        self.verify_files(&self.host_root(&host)?, &host.host.files)?;
        self.verify_files(&self.amf_root(&amf)?, &amf.amf.files)?;
        self.runtime(
            &host,
            &owned_path(&self.root, "cache/no-install-input")?,
            transfer,
        )
        .await?;
        self.commit_restart(&pending.base, &next)?;
        fs::remove_file(owned_path(&self.root, &name(kind, "pending"))?)?;
        Ok(())
    }
}
