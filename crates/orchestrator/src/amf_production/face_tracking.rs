//! Source facts and a confirmed, opaque creator-menu session. No runtime-readiness claim.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FaceTrackingMenu {
    pub id: String,
    pub source_id: String,
    pub script_path: String,
    pub menu_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FaceTrackingPatch {
    pub menu_id: String,
    pub avatar_global_object_id: String,
    pub confirm_trusted_code: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Profile {
    id: String,
    reference_count: usize,
    matched: Vec<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Mesh {
    renderer: String,
    blend_shape_count: u32,
    missing_mesh: bool,
    profiles: Vec<Profile>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Facts {
    schema_version: String,
    evidence: String,
    observed_at: String,
    state: String,
    runtime: String,
    built_parameter_budget: String,
    truncated: bool,
    manual_replay_required: bool,
    expression_parameters_observed: bool,
    source_synced_parameter_bits: u32,
    meshes: Vec<Mesh>,
    parameters: Vec<String>,
    components: Vec<String>,
    patch_menus: Vec<FaceTrackingMenu>,
}

fn plain(text: &str, max: usize) -> bool {
    text.chars().count() <= max && !text.chars().any(char::is_control)
}
fn digest(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn distinct_text(values: &[String], max: usize, count: usize) -> bool {
    values.len() <= count && values.iter().all(|s| !s.is_empty() && plain(s, max))
        && values.iter().collect::<std::collections::BTreeSet<_>>().len() == values.len()
}
fn creator_menu(path: &str) -> bool {
    let root = path.split('/').next().unwrap_or("");
    path.contains('/') && path.split('/').all(|s| !s.is_empty()) && plain(path, 400)
        && !["File", "Edit", "Assets", "GameObject", "Component", "Window", "Help", "CONTEXT", "VRChat SDK"].contains(&root)
        && !path.contains(['%', '#', '&']) && !path.contains(" _")
}
/// Validate new evidence when present; an older Bridge may omit it completely.
pub fn validate_face_tracking(value: &serde_json::Value) -> Result<(), AppErrorV1> {
    let facts: Facts = serde_json::from_value(value.clone())
        .map_err(|_| production_error("receipt_invalid", "Invalid face tracking facts."))?;
    let valid = facts.schema_version == "0.1" && facts.evidence == "unity_source"
        && !facts.observed_at.is_empty() && plain(&facts.observed_at, 64)
        && matches!(facts.state.as_str(), "no_avatar" | "observed" | "partial")
        && facts.runtime == "unknown" && facts.built_parameter_budget == "unknown"
        && (!facts.truncated || facts.state != "observed")
        && (facts.expression_parameters_observed || facts.source_synced_parameter_bits == 0)
        && facts.source_synced_parameter_bits <= 65536
        && facts.meshes.len() <= 128 && facts.meshes.iter().all(|mesh| {
            plain(&mesh.renderer, 1024) && mesh.blend_shape_count <= 65536
                && (!mesh.missing_mesh || mesh.blend_shape_count == 0)
                && mesh.profiles.len() == 3
                && mesh.profiles.iter().map(|p| &p.id).collect::<std::collections::BTreeSet<_>>().len() == 3
                && mesh.profiles.iter().all(|profile| {
                    let count = match profile.id.as_str() {
                        "arkit52-v1" => 52, "unified-core12-v1" | "sranipal-core12-v1" => 12, _ => 0,
                    };
                    count != 0 && count == profile.reference_count && profile.matched.len() <= mesh.blend_shape_count as usize
                        && distinct_text(&profile.matched, 100, count)
                })
        })
        && distinct_text(&facts.parameters, 256, 1024) && distinct_text(&facts.components, 256, 64)
        && facts.patch_menus.len() <= 64 && facts.patch_menus.iter().all(|m| {
            digest(&m.id) && !m.source_id.is_empty() && plain(&m.source_id, 100)
                && m.script_path.starts_with("Assets/") && m.script_path.to_ascii_lowercase().ends_with(".cs") && plain(&m.script_path, 1024)
                && !m.script_path.contains(['\\', ':']) && m.script_path.split('/').all(|s| !matches!(s, "" | "." | ".."))
                && creator_menu(&m.menu_path)
        })
        && facts.patch_menus.iter().map(|m| &m.id).collect::<std::collections::BTreeSet<_>>().len() == facts.patch_menus.len();
    // Both values are meaningful even when no recognizable FT shapes are present.
    let _manual_replay = facts.manual_replay_required;
    if valid { Ok(()) } else { Err(production_error("receipt_invalid", "Face tracking evidence exceeds its contract.")) }
}

pub(crate) fn manual_face_tracking(run: &ProductionRun) -> bool {
    run.result["facts"]["faceTracking"]["manualReplayRequired"] == true
        || Path::new(&run.project_root).join("ProjectSettings/VUA.FaceTracking.json").exists()
}
pub(crate) fn require_replayable_face_tracking(run: &ProductionRun) -> Result<(), AppErrorV1> {
    if manual_face_tracking(run) { Err(manual_replay_error()) } else { Ok(()) }
}
fn manual_replay_error() -> AppErrorV1 {
    production_error("face_tracking_manual_replay", "This line contains an opaque creator patch. Recipe saving, assembly, material replacement and version replay cannot reproduce it. Use the retained full restore point or prepare a separate line from original materials and reapply the patch.")
}

impl AmfProductionEngine {
    pub fn face_tracking_plan(&self, run_id: &str, menu_id: &str) -> Result<serde_json::Value, AppErrorV1> {
        let mut run = self.get(run_id)?;
        if run.phase != "succeeded" || !still_assembled(&run) {
            return Err(production_error("inspect_required", "Inspect a completed line before adding face tracking."));
        }
        self.ensure_project_matches(&mut run)?;
        validate_face_tracking(&run.result["facts"]["faceTracking"])?;
        let menus: Vec<FaceTrackingMenu> = serde_json::from_value(run.result["facts"]["faceTracking"]["patchMenus"].clone()).map_err(io_error)?;
        let menu = menus.into_iter().find(|menu| menu.id == menu_id && run.sources.iter().any(|source|
            source.source_id == menu.source_id && source.asset_paths.contains(&menu.script_path)))
            .ok_or_else(|| production_error("face_tracking_menu_unavailable", "Select an unambiguous imported menu from the latest inspection."))?;
        let avatar = run.result["facts"]["avatarGlobalObjectId"].as_str().filter(|id| !id.is_empty())
            .ok_or_else(|| production_error("inspect_required", "The owned Avatar identity is missing."))?;
        let review = serde_json::json!({
            "schemaVersion":"0.1", "runId":run.run_id, "revision":run.revision,
            "projectFingerprint":run.project_fingerprint, "unityFingerprint":run.unity_fingerprint,
            "menu":menu, "avatarGlobalObjectId":avatar, "execution":"interactive_editor",
            "replay":"manual", "requiresTrustedCodeConfirmation":true,
            "warning":"Creator code runs with Editor privileges. The restore point covers only the work project. Finish on the same Avatar. Recipe and version replay will require manual reapplication."
        });
        let hash = Sha256::digest(serde_json::to_vec(&review).map_err(io_error)?);
        let mut plan = review;
        plan["planDigest"] = serde_json::json!(hash.iter().map(|b| format!("{b:02x}")).collect::<String>());
        Ok(plan)
    }

    pub fn face_tracking_patch(&self, run_id: &str, menu_id: &str, plan_digest: &str, confirmed: bool, ctx: &TaskContext) -> Result<(), AppErrorV1> {
        if !confirmed {
            return Err(production_error("confirmation_required", "The player must confirm this creator code and its replay limits."));
        }
        self.precheck_closed(run_id)?;
        let plan = self.face_tracking_plan(run_id, menu_id)?;
        if plan["planDigest"] != plan_digest {
            return Err(production_error("plan_drift", "Review the current creator patch again."));
        }
        let patch = FaceTrackingPatch { menu_id: menu_id.into(),
            avatar_global_object_id: plan["avatarGlobalObjectId"].as_str().expect("validated Avatar").into(),
            confirm_trusted_code: true };
        let mut run = self.get(run_id)?;
        let before = run.clone();
        run.task_id = ctx.task_id().into();
        run.phase = "assembling".into(); run.error = None; run.recovery = None;
        let mut snapshot = None;
        let mut revision_id = None;
        let result = (|| {
            self.step(&mut run, ctx, "snapshot", "Protect the work project")?;
            snapshot = Some(self.platform.snapshot(&run)?);
            run.snapshot_id = snapshot.clone(); run.snapshot_selection = run.selection.clone(); run.snapshot_sources = None;
            revision_id = Some(self.record_revision(&mut run, RevisionKind::FaceTracking, false));
            self.save(&run)?;
            self.step(&mut run, ctx, "face_tracking", "Complete the creator patch in Unity")?;
            let receipt = self.platform.face_tracking_patch(&run, &patch, &self.watch(&run, ctx))?;
            if receipt.result["facts"]["faceTracking"]["manualReplayRequired"] != true {
                return Err(production_error("receipt_invalid", "The creator session did not record its manual replay boundary."));
            }
            self.step(&mut run, ctx, "inspect", "Check the patched Avatar")?;
            self.update_observations(&mut run, ctx)?;
            if let Some(error) = ctx.cancellation_error("amf-production") { return Err(error); }
            if !still_assembled(&run) || run.result["facts"]["avatarGlobalObjectId"] != patch.avatar_global_object_id
                || run.result["facts"]["faceTracking"]["manualReplayRequired"] != true {
                return Err(production_error("face_tracking_target_changed", "The patch did not retain the owned Avatar and its outfits."));
            }
            run.phase = "succeeded".into(); run.confirmed_digest = None;
            Ok(())
        })();
        if result.is_err() {
            if let Some(snapshot) = snapshot {
                run.recovery = Some(if result.as_ref().is_err_and(|error| error.code == "vua.amf_production.process_cleanup_unverified") {
                    // Do not replace files while an owned creator helper might still be writing.
                    "restore_failed: vua.amf_production.process_cleanup_unverified".into()
                } else { match self.platform.restore(&run, &snapshot) {
                    Ok(()) => {
                        run.result = before.result; run.prefabs = before.prefabs;
                        run.unity_fingerprint = before.unity_fingerprint; run.project_fingerprint = before.project_fingerprint;
                        run.revision = before.revision; run.fit_changes = before.fit_changes; run.observed_at = before.observed_at;
                        if let Some(id) = revision_id { run.revisions.retain(|r| r.id != id); }
                        "restored".into()
                    }
                    Err(error) => format!("restore_failed: {}", error.code),
                } });
            }
        }
        let outcome = self.finish(&mut run, result);
        self.prune_snapshots(&run);
        outcome
    }
}
