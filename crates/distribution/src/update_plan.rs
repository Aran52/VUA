//! Explicit, authenticated pair plans break host/AMF compatibility cycles.
use crate::{
    component_feed::{self, ComponentKind, Release},
    component_update::fetch_inventory,
    layout::{Active, Installation},
    manifest::SignedManifest,
    paths::{digest, owned_path},
    transfer::Transfer,
    Error, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::Path};

const PLAN: &str = "updates-v02/pair-plan.json";
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PairPlan {
    schema_version: String,
    base: Active,
    next: Active,
    host_index: SignedManifest,
    amf_index: SignedManifest,
    state: String,
    requested: bool,
}
fn plan_id(plan: &PairPlan) -> Result<String> {
    Ok(digest(&serde_json::to_vec(
        &json!({"schemaVersion":"0.1", "base":plan.base, "next":plan.next}),
    )?))
}
fn newer(candidate: &str, installed: &str) -> bool {
    semver::Version::parse(candidate)
        .expect("validated candidate")
        .cmp_precedence(&semver::Version::parse(installed).expect("validated installed version"))
        .is_gt()
}

impl Installation {
    fn pair_plan(&self) -> Result<Option<(PairPlan, Release, Release)>> {
        if !owned_path(&self.root, PLAN)?.exists() {
            return Ok(None);
        }
        let plan: PairPlan = self.update_json(PLAN)?;
        if plan.schema_version != "0.1"
            || !["proposed", "interrupted", "staged"].contains(&plan.state.as_str())
            || plan.base.host_manifest == plan.next.host_manifest
            || plan.base.amf_manifest == plan.next.amf_manifest
        {
            return Err(Error("invalid_update_plan".into()));
        }
        let (_, old_amf) = self.validate_pair(&plan.base)?;
        let (_, new_amf) = self.validate_pair(&plan.next)?;
        if !new_amf
            .compatibility
            .bdl_read
            .contains(&old_amf.compatibility.bdl_write)
        {
            return Err(Error("unsupported_data_migration".into()));
        }
        let entry = |kind: ComponentKind, index: &SignedManifest, id: &str| -> Result<Release> {
            let feed =
                component_feed::verify(&serde_json::to_vec(index)?, self.trust_key(), kind, 0)?;
            let entry = feed
                .releases
                .into_iter()
                .find(|entry| entry.manifest.sha256 == id)
                .ok_or_else(|| Error("invalid_update_plan".into()))?;
            entry.verify_inventory(
                kind,
                &fs::read(owned_path(&self.root, &format!("manifests/{id}.json"))?)?,
                self.trust_key(),
            )?;
            Ok(entry)
        };
        let host = entry(
            ComponentKind::Host,
            &plan.host_index,
            &plan.next.host_manifest,
        )?;
        let amf = entry(ComponentKind::Amf, &plan.amf_index, &plan.next.amf_manifest)?;
        let (old_host, old_amf) = self.validate_pair(&plan.base)?;
        if !newer(&host.version, &old_host.host.version)
            || !newer(&amf.version, &old_amf.amf.version)
        {
            return Err(Error("update_not_newer".into()));
        }
        Ok(Some((plan, host, amf)))
    }
    pub fn update_plan_snapshot(&self) -> Result<Value> {
        let mut view = json!({"schemaVersion":"0.1", "available":true, "state":"idle", "rollbackAvailable":self.pair_restore_target().is_ok()});
        if let Some((plan, host, amf)) = self.pair_plan()? {
            let (old_host, old_amf) = self.validate_pair(&plan.base)?;
            let raw_host = fs::read(owned_path(
                &self.root,
                &format!("manifests/{}.json", plan.next.host_manifest),
            )?)?;
            let raw_amf = fs::read(owned_path(
                &self.root,
                &format!("manifests/{}.json", plan.next.amf_manifest),
            )?)?;
            let mut amf_target = self.component_target(ComponentKind::Amf, &amf, &raw_amf)?;
            amf_target["activation"] = json!("restart");
            view["plan"] = json!({"id":plan_id(&plan)?, "installed":{"host":old_host.host.version,"amf":old_amf.amf.version}, "activation":"restart", "targets":[
                {"component":"host","target":self.component_target(ComponentKind::Host, &host, &raw_host)?}, {"component":"amf","target":amf_target}]});
            view["state"] = json!(if self.active()? == plan.base {
                plan.state.as_str()
            } else {
                "interrupted"
            });
            if self.active()? != plan.base {
                view["error"] = json!("active_selection_changed");
            }
        }
        Ok(view)
    }
    pub async fn propose_update_plan(&self, transfer: &Transfer) -> Result<Value> {
        let _plan = self.lock("pair-plan.lock")?;
        let base = self.active()?;
        let (host, amf) = self.validate_pair(&base)?;
        let host_raw = transfer.manifest(ComponentKind::Host.feed()).await?;
        let host_feed = self.accept_component_index(ComponentKind::Host, &host_raw)?;
        let amf_raw = transfer.manifest(ComponentKind::Amf.feed()).await?;
        let amf_feed = self.accept_component_index(ComponentKind::Amf, &amf_raw)?;
        let mut candidates = Vec::new();
        for next_host in &host_feed.releases {
            if !newer(&next_host.version, &host.host.version) {
                continue;
            }
            for next_amf in &amf_feed.releases {
                if newer(&next_amf.version, &amf.amf.version)
                    && component_feed::compatible(
                        &next_amf.compatibility,
                        &next_host.version,
                        &next_host.compatibility,
                    )
                    && next_amf
                        .compatibility
                        .bdl_read
                        .contains(&amf.compatibility.bdl_write)
                {
                    candidates.push((next_host, next_amf));
                }
            }
        }
        candidates.sort_by(|(a_host, a_amf), (b_host, b_amf)| {
            let order = semver::Version::parse(&b_host.version)
                .expect("validated version")
                .cmp_precedence(
                    &semver::Version::parse(&a_host.version).expect("validated version"),
                );
            order.then_with(|| {
                semver::Version::parse(&b_amf.version)
                    .expect("validated version")
                    .cmp_precedence(
                        &semver::Version::parse(&a_amf.version).expect("validated version"),
                    )
            })
        });
        let (next_host, next_amf) = candidates
            .first()
            .ok_or_else(|| Error("no_compatible_update_plan".into()))?;
        let host_bytes =
            fetch_inventory(next_host, ComponentKind::Host, transfer, self.trust_key()).await?;
        let amf_bytes =
            fetch_inventory(next_amf, ComponentKind::Amf, transfer, self.trust_key()).await?;
        transfer.checkpoint()?;
        self.prepare_update_plan_objects(&base, &host_raw, &amf_raw, &host_bytes, &amf_bytes)
    }
    pub(crate) fn prepare_update_plan_objects(
        &self,
        base: &Active,
        host_raw: &[u8],
        amf_raw: &[u8],
        host_bytes: &[u8],
        amf_bytes: &[u8],
    ) -> Result<Value> {
        let host_feed = self.accept_component_index(ComponentKind::Host, host_raw)?;
        let amf_feed = self.accept_component_index(ComponentKind::Amf, amf_raw)?;
        let host_entry = host_feed
            .releases
            .iter()
            .find(|entry| entry.manifest.sha256 == digest(host_bytes))
            .ok_or_else(|| Error("invalid_update_plan".into()))?;
        let amf_entry = amf_feed
            .releases
            .iter()
            .find(|entry| entry.manifest.sha256 == digest(amf_bytes))
            .ok_or_else(|| Error("invalid_update_plan".into()))?;
        host_entry.verify_inventory(ComponentKind::Host, host_bytes, self.trust_key())?;
        amf_entry.verify_inventory(ComponentKind::Amf, amf_bytes, self.trust_key())?;
        let (current_host, current_amf) = self.validate_pair(base)?;
        if !newer(&host_entry.version, &current_host.host.version)
            || !newer(&amf_entry.version, &current_amf.amf.version)
        {
            return Err(Error("update_not_newer".into()));
        }
        if !amf_entry
            .compatibility
            .bdl_read
            .contains(&current_amf.compatibility.bdl_write)
        {
            return Err(Error("unsupported_data_migration".into()));
        }
        let host_id = self.store_manifest(host_bytes)?.0;
        let amf_id = self.store_manifest(amf_bytes)?.0;
        let plan = PairPlan {
            schema_version: "0.1".into(),
            base: base.clone(),
            next: Active {
                schema_version: "0.1".into(),
                host_manifest: host_id,
                amf_manifest: amf_id,
            },
            host_index: serde_json::from_slice(host_raw)?,
            amf_index: serde_json::from_slice(amf_raw)?,
            state: "proposed".into(),
            requested: false,
        };
        self.validate_pair(&plan.next)?;
        if &self.active()? != base {
            return Err(Error("active_selection_changed".into()));
        }
        self.save_update(PLAN, &plan)?;
        self.update_plan_snapshot()
    }
    pub async fn stage_update_plan(
        &self,
        id: &str,
        transfer: &mut Transfer,
        host_pack: Option<&Path>,
        amf_pack: Option<&Path>,
    ) -> Result<Value> {
        let _plan = self.lock("pair-plan.lock")?;
        let _host = self.lock("component-stage-host.lock")?;
        let _amf = self.lock("component-stage-amf.lock")?;
        let (mut plan, _, _) = self
            .pair_plan()?
            .ok_or_else(|| Error("update_plan_required".into()))?;
        if plan_id(&plan)? != id || plan.base != self.active()? {
            return Err(Error("update_plan_changed".into()));
        }
        let (host, amf) = self.validate_pair(&plan.next)?;
        let (old_host, old_amf) = self.validate_pair(&plan.base)?;
        let needed = host
            .host
            .pack
            .bytes
            .saturating_add(amf.amf.pack.bytes)
            .saturating_mul(2)
            .saturating_add(32 * 1024 * 1024)
            .saturating_add(
                if self
                    .verify_files(&self.runtime_root(&host)?, &host.runtime.files)
                    .is_err()
                {
                    host.runtime.archive.bytes.saturating_add(
                        host.runtime
                            .files
                            .iter()
                            .map(|file| file.bytes)
                            .sum::<u64>(),
                    )
                } else {
                    0
                },
            );
        if crate::layout::free_space(&self.root)? < needed {
            return Err(Error("insufficient_disk_space".into()));
        }
        plan.state = "interrupted".into();
        plan.requested = false;
        self.save_update(PLAN, &plan)?;
        fs::create_dir_all(owned_path(&self.root, "cache")?)?;
        for (kind, value, previous, output, local) in [
            (
                ComponentKind::Host,
                &host.host,
                self.host_root(&old_host)?,
                self.host_root(&host)?,
                host_pack,
            ),
            (
                ComponentKind::Amf,
                &amf.amf,
                self.amf_root(&old_amf)?,
                self.amf_root(&amf)?,
                amf_pack,
            ),
        ] {
            if output == previous && self.verify_files(&output, &value.files).is_err() {
                return Err(Error("active_files_need_repair".into()));
            }
            transfer.progress(
                if kind == ComponentKind::Host {
                    "plan-host"
                } else {
                    "plan-amf"
                },
                0,
                value.pack.bytes,
            );
            transfer
                .component(
                    &value.files,
                    &value.pack,
                    Some(&previous),
                    &output,
                    local,
                    &owned_path(&self.root, &format!("cache/{}.bin", value.pack.sha256))?,
                )
                .await?;
        }
        if self
            .verify_files(&self.runtime_root(&host)?, &host.runtime.files)
            .is_err()
        {
            transfer.progress("plan-host", 0, host.runtime.archive.bytes);
            transfer
                .archive(
                    &host.runtime.archive,
                    &owned_path(
                        &self.root,
                        &format!("cache/{}.zip", host.runtime.archive.sha256),
                    )?,
                )
                .await?;
            if host.runtime.archive.sha256 != old_host.runtime.archive.sha256 {
                self.runtime(
                    &host,
                    &owned_path(&self.root, "cache/no-install-input")?,
                    transfer,
                )
                .await?;
            }
        }
        transfer.checkpoint()?;
        if plan.base != self.active()? {
            return Err(Error("active_selection_changed".into()));
        }
        plan.state = "staged".into();
        self.save_update(PLAN, &plan)?;
        self.update_plan_snapshot()
    }
    pub fn request_update_plan(&self, id: &str) -> Result<()> {
        let _plan = self.lock("pair-plan.lock")?;
        let (mut plan, _, _) = self
            .pair_plan()?
            .ok_or_else(|| Error("update_plan_required".into()))?;
        if plan_id(&plan)? != id || plan.base != self.active()? || plan.state != "staged" {
            return Err(Error("update_plan_changed".into()));
        }
        self.admit_data_update(&plan.base, &plan.next)?;
        plan.requested = true;
        self.save_update(PLAN, &plan)
    }
    pub async fn activate_update_plan(&self, id: &str, transfer: &mut Transfer) -> Result<()> {
        let _operation = self.lock("operation.lock")?;
        let _running = self.lock("running.lock")?;
        let _plan = self.lock("pair-plan.lock")?;
        let (plan, _, _) = self
            .pair_plan()?
            .ok_or_else(|| Error("update_plan_required".into()))?;
        if plan_id(&plan)? != id
            || plan.base != self.active()?
            || plan.state != "staged"
            || !plan.requested
        {
            return Err(Error("activation_requires_choice".into()));
        }
        let (host, amf) = self.validate_pair(&plan.next)?;
        self.verify_files(&self.host_root(&host)?, &host.host.files)?;
        self.verify_files(&self.amf_root(&amf)?, &amf.amf.files)?;
        self.runtime(
            &host,
            &owned_path(&self.root, "cache/no-install-input")?,
            transfer,
        )
        .await?;
        self.commit_restart(&plan.base, &plan.next)?;
        fs::remove_file(owned_path(&self.root, PLAN)?)?;
        Ok(())
    }
    fn pair_restore_target(&self) -> Result<Active> {
        let journal = self
            .activation_journal()?
            .ok_or_else(|| Error("no_retained_version".into()))?;
        if journal.state != "healthy"
            || journal.next != self.active()?
            || journal.before.host_manifest == journal.next.host_manifest
            || journal.before.amf_manifest == journal.next.amf_manifest
        {
            return Err(Error("no_retained_version".into()));
        }
        let (host, amf) = self.validate_pair(&journal.before)?;
        let (_, current) = self.validate_pair(&journal.next)?;
        if !amf
            .compatibility
            .bdl_read
            .contains(&current.compatibility.bdl_write)
        {
            return Err(Error("rollback_requires_data_repair".into()));
        }
        self.verify_files(&self.host_root(&host)?, &host.host.files)?;
        self.verify_files(&self.runtime_root(&host)?, &host.runtime.files)?;
        self.verify_files(&self.amf_root(&amf)?, &amf.amf.files)?;
        Ok(journal.before)
    }
    pub fn restore_update_plan(&self) -> Result<()> {
        let _operation = self.lock("operation.lock")?;
        let _running = self.lock("running.lock")?;
        let next = self.pair_restore_target()?;
        self.commit_restart(&self.active()?, &next)
    }
    pub(crate) fn update_plan_retained_pairs(&self) -> Result<Vec<Active>> {
        Ok(self
            .pair_plan()?
            .map_or_else(Vec::new, |(plan, _, _)| vec![plan.base, plan.next]))
    }
}
