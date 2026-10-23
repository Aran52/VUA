//! Explicit deletion of one owned garment. Omission from assembly is still not deletion.
use super::*;
use std::collections::BTreeSet;

pub const REMOVAL_OBJECT_LIMIT: usize = 8192;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutfitRemovalRoot {
    pub kind: String,
    pub global_object_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutfitRemovalObject {
    pub global_object_id: String,
    /// Display only. Execution resolves ownership and GlobalObjectIds, never this path.
    pub path: String,
    pub type_name: String,
    pub manual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutfitRemoval {
    pub source_id: String,
    pub avatar_global_object_id: String,
    pub roots: Vec<OutfitRemovalRoot>,
    pub objects: Vec<OutfitRemovalObject>,
    pub blocked_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutfitRemovalApplied {
    pub source_id: String,
    pub avatar_global_object_id: String,
    pub deleted_object_ids: Vec<String>,
}

fn bounded(value: &str, max: usize, empty: bool) -> bool {
    (empty || !value.is_empty()) && value.chars().count() <= max && !value.contains('\0')
}

pub fn valid_outfit_removal(removal: &OutfitRemoval) -> bool {
    let roots: BTreeSet<_> = removal.roots.iter().map(|r| &r.global_object_id).collect();
    let objects: BTreeSet<_> = removal.objects.iter().map(|o| &o.global_object_id).collect();
    bounded(&removal.source_id, 256, false)
        && bounded(&removal.avatar_global_object_id, 256, false)
        && ["", "ownership_invalid", "target_unverified", "contains_owned_object", "inventory_limit", "identity_invalid"].contains(&removal.blocked_reason.as_str())
        && removal.roots.len() <= 2 && roots.len() == removal.roots.len()
        && removal.roots.iter().all(|r| ["outfit", "menu"].contains(&r.kind.as_str()) && bounded(&r.global_object_id, 256, false))
        && removal.objects.len() <= REMOVAL_OBJECT_LIMIT && objects.len() == removal.objects.len()
        && removal.objects.iter().all(|o| bounded(&o.global_object_id, 256, false) && bounded(&o.path, 4096, true) && bounded(&o.type_name, 256, false))
        && (!removal.blocked_reason.is_empty() || (
            removal.roots.iter().filter(|r| r.kind == "outfit").count() == 1
                && removal.roots.iter().filter(|r| r.kind == "menu").count() <= 1
                && roots.is_subset(&objects) && !objects.contains(&removal.avatar_global_object_id)
                && removal.roots.iter().all(|r| removal.objects.iter().any(|o| o.global_object_id == r.global_object_id && o.type_name == "UnityEngine.GameObject"))
        ))
}

/// Additive receipt fields are optional for old projects; present fields are always checked.
pub fn validate_outfit_removal_facts(facts: &serde_json::Value) -> Result<(), AppErrorV1> {
    let invalid = || production_error("receipt_invalid", "Outfit removal inventory exceeds its contract.");
    if let Some(value) = facts.get("outfitRemoval") {
        let entries: Vec<OutfitRemoval> = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
        if entries.len() > 64 || entries.iter().any(|e| !valid_outfit_removal(e))
            || entries.iter().map(|e| &e.source_id).collect::<BTreeSet<_>>().len() != entries.len() {
            return Err(invalid());
        }
    }
    if let Some(value) = facts.get("outfitRemovalApplied") {
        let entries: Vec<OutfitRemovalApplied> = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
        if entries.len() > 1 { return Err(invalid()); }
        for applied in entries {
            if !bounded(&applied.source_id, 256, false) || !bounded(&applied.avatar_global_object_id, 256, false)
            || applied.deleted_object_ids.is_empty() || applied.deleted_object_ids.len() > REMOVAL_OBJECT_LIMIT
            || applied.deleted_object_ids.iter().any(|id| !bounded(id, 256, false))
                || applied.deleted_object_ids.iter().collect::<BTreeSet<_>>().len() != applied.deleted_object_ids.len() { return Err(invalid()); }
        }
    }
    Ok(())
}

fn verify_removal(before: &ProductionRun, after: &ProductionRun, source: &str) -> Result<(), AppErrorV1> {
    let invalid = || production_error("outfit_removal_unverified", "Unity did not retain exactly the other outfits and owned Avatars.");
    let expected = before.selection.as_ref().ok_or_else(invalid)?;
    let actual: AssemblySelection = serde_json::from_value(after.result["facts"]["observedSelection"].clone()).map_err(|_| invalid())?;
    let remaining: Vec<_> = expected.outfits.iter().filter(|o| o.source_id != source).collect();
    if actual.avatar_prefab != expected.avatar_prefab || actual.outfits.len() != remaining.len()
        || remaining.iter().any(|o| !actual.outfits.contains(o))
        || after.result["facts"]["avatarGlobalObjectId"] != before.result["facts"]["avatarGlobalObjectId"] { return Err(invalid()); }
    let old = before.result["facts"]["instances"].as_array().ok_or_else(invalid)?;
    let new = after.result["facts"]["instances"].as_array().ok_or_else(invalid)?;
    if new.len() != remaining.len() || new.iter().any(|o| o["sourceId"] == source)
        || remaining.iter().any(|o| {
            let previous = old.iter().find(|v| v["sourceId"] == o.source_id);
            previous.is_none() || previous != new.iter().find(|v| v["sourceId"] == o.source_id)
        }) { return Err(invalid()); }
    if let Some(avatars) = before.result["facts"]["avatars"].as_array() {
        let current = after.result["facts"]["avatars"].as_array().ok_or_else(invalid)?;
        if current.len() != avatars.len() || avatars.iter().filter(|a| a["activeTarget"] != true)
            .any(|a| !current.contains(a)) { return Err(invalid()); }
    }
    Ok(())
}

impl AmfProductionEngine {
    pub fn plan_outfit_removal(&self, run_id: &str, source_id: &str) -> Result<serde_json::Value, AppErrorV1> {
        let mut run = self.get(run_id)?;
        face_tracking::require_replayable_face_tracking(&run)?;
        if run.phase != "succeeded" || !still_assembled(&run) {
            return Err(production_error("inspect_required", "Inspect the completed line before removing an outfit."));
        }
        self.ensure_project_matches(&mut run)?;
        let selection = run.selection.as_ref().expect("assembled selection");
        if !selection.outfits.iter().any(|o| o.source_id == source_id) {
            return Err(production_error("outfit_invalid", "Select one outfit the active Avatar currently wears."));
        }
        if selection.outfits.len() < 2 {
            return Err(production_error("last_outfit", "This production face requires at least one outfit to remain. Use a separate line to prepare a bare Avatar."));
        }
        validate_outfit_removal_facts(&run.result["facts"])?;
        let inventories: Vec<OutfitRemoval> = serde_json::from_value(run.result["facts"]["outfitRemoval"].clone())
            .map_err(|_| production_error("inspect_required", "Reinspect to obtain the owned-object removal inventory."))?;
        let removal = inventories.into_iter().find(|e| e.source_id == source_id)
            .ok_or_else(|| production_error("inspect_required", "Reinspect the outfit's owned-object inventory."))?;
        if !removal.blocked_reason.is_empty() {
            return Err(production_error("outfit_removal_blocked", format!("Removal is blocked: {}. Reinspect or resolve it in Unity.", removal.blocked_reason)));
        }
        if run.result["facts"]["avatarGlobalObjectId"] != removal.avatar_global_object_id {
            return Err(production_error("inspect_required", "The active Avatar's identity changed."));
        }
        let binding = serde_json::json!({"runId":run.run_id,"revision":run.revision,
            "projectFingerprint":run.project_fingerprint,"unityFingerprint":run.unity_fingerprint,
            "selection":selection,"removal":removal});
        let digest = Sha256::digest(serde_json::to_vec(&binding).map_err(io_error)?).iter().map(|b| format!("{b:02x}")).collect::<String>();
        Ok(serde_json::json!({"schemaVersion":"0.1","runId":run.run_id,"planDigest":digest,"removal":removal}))
    }

    pub fn remove_outfit(&self, run_id: &str, source_id: &str, digest: &str, confirmed: bool, ctx: &TaskContext) -> Result<(), AppErrorV1> {
        if !confirmed { return Err(production_error("confirmation_required", "Confirm the outfit, its menu and all listed scene additions before removal.")); }
        self.precheck_closed(run_id)?;
        let plan = self.plan_outfit_removal(run_id, source_id)?;
        if plan["planDigest"] != digest { return Err(production_error("plan_drift", "Review the current removal inventory again.")); }
        let removal: OutfitRemoval = serde_json::from_value(plan["removal"].clone()).map_err(io_error)?;
        let mut run = self.get(run_id)?;
        let before = run.clone();
        self.ensure_project_matches(&mut run)?;
        run.task_id = ctx.task_id().into(); run.phase = "assembling".into(); run.error = None; run.recovery = None;
        let mut snapshot = None;
        let mut revision_id = None;
        let result = (|| {
            self.step(&mut run, ctx, "snapshot", "Protect the work project")?;
            snapshot = Some(self.platform.snapshot(&run)?);
            run.snapshot_id = snapshot.clone(); run.snapshot_selection = run.selection.clone(); run.snapshot_sources = None;
            revision_id = Some(self.record_revision(&mut run, RevisionKind::OutfitRemoval, false));
            self.save(&run)?;
            self.step(&mut run, ctx, "outfit_removal", "Remove the confirmed owned outfit and its switch")?;
            let receipt = self.platform.remove_outfit(&run, &removal, &self.watch(&run, ctx))?;
            validate_outfit_removal_facts(&receipt.result["facts"])?;
            let mut proof: Vec<OutfitRemovalApplied> = serde_json::from_value(receipt.result["facts"]["outfitRemovalApplied"].clone())
                .map_err(|_| production_error("outfit_removal_unverified", "Unity did not report the deleted object identities."))?;
            let applied = proof.pop().ok_or_else(|| production_error("outfit_removal_unverified", "Unity did not report the deleted object identities."))?;
            let expected: BTreeSet<_> = removal.objects.iter().map(|o| &o.global_object_id).collect();
            if receipt.editor_version != run.editor.version || applied.source_id != source_id
                || applied.avatar_global_object_id != removal.avatar_global_object_id
                || applied.deleted_object_ids.iter().collect::<BTreeSet<_>>() != expected {
                return Err(production_error("outfit_removal_unverified", "Unity deleted a different object set."));
            }
            if let Some(error) = ctx.cancellation_error("amf-production") { return Err(error); }
            self.step(&mut run, ctx, "inspect", "Recheck the remaining outfits in Unity")?;
            self.update_observations(&mut run, ctx)?;
            if let Some(error) = ctx.cancellation_error("amf-production") { return Err(error); }
            verify_removal(&before, &run, source_id)?;
            run.selection.as_mut().expect("assembled selection").outfits.retain(|o| o.source_id != source_id);
            run.applied_fits.retain(|f| f.source_id != source_id);
            run.phase = "succeeded".into(); run.confirmed_digest = None;
            Ok(())
        })();
        if result.is_err() {
            if let Some(snapshot) = snapshot {
                run.recovery = Some(match self.platform.restore(&run, &snapshot) {
                    Ok(()) => {
                        run.selection = before.selection; run.prefabs = before.prefabs; run.result = before.result;
                        run.project_fingerprint = before.project_fingerprint; run.unity_fingerprint = before.unity_fingerprint;
                        run.revision = before.revision; run.observed_at = before.observed_at;
                        run.fit_changes = before.fit_changes; run.applied_fits = before.applied_fits;
                        run.confirmed_digest = before.confirmed_digest;
                        if let Some(id) = revision_id { run.revisions.retain(|r| r.id != id); }
                        "restored".into()
                    }
                    Err(error) => format!("restore_failed: {}", error.code),
                });
            }
        }
        let outcome = self.finish(&mut run, result);
        self.prune_snapshots(&run);
        outcome
    }
}
