use super::provider_host::{
    app_error_category, application_error, application_error_with_params, application_success,
    uuid_v7_identity, FrameOutcome,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use vua_orchestrator::{amf_production::*, AppErrorV1, CommandAcceptedV1, SubmitRequest, TaskExit, TaskLane, TaskLanePolicy, TaskRuntime};

/// One AMF Unity job in the lane and what it is for, so the rooms can show and remove it.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaneEntry {
    pub task_id: String,
    pub run_id: String,
    pub operation: String,
}

pub(crate) struct AmfProductionServices {
    pub engine: AmfProductionEngine,
    pub runtime: TaskRuntime,
    /// Unity work is admitted in request order with capacity and per-run exclusion.
    pub lane: TaskLane,
    entries: Mutex<Vec<LaneEntry>>,
}

struct LaneView {
    capacity: usize,
    running: Vec<LaneEntry>,
    queue: Vec<LaneEntry>,
}

impl AmfProductionServices {
    /// Non-production Unity checks share admission without becoming production runs.
    pub(crate) fn submit_exclusive(&self, request: SubmitRequest, kind: &str, command_id: &str, fingerprint: &str) -> Result<CommandAcceptedV1, AppErrorV1> {
        let _entries = self.entries.lock().expect("lane entries poisoned");
        self.runtime.submit_idempotent_in_lane(request, &self.lane, TaskLanePolicy { key: None, exclusive: true }, kind, command_id, fingerprint)
    }
    pub(crate) fn new(engine: AmfProductionEngine, runtime: TaskRuntime) -> Self {
        let lane = TaskLane::with_capacity(engine.capacity()).expect("validated stored capacity");
        Self { engine, runtime, lane, entries: Mutex::new(Vec::new()) }
    }

    /// A Unity job runs now.
    pub(crate) fn busy(&self) -> bool {
        !self.lane.snapshot().running.is_empty()
    }

    /// Read the lane once while submissions cannot change its task metadata.
    fn lane_view(&self) -> LaneView {
        let entries = self.entries.lock().expect("lane entries poisoned");
        let snapshot = self.lane.snapshot();
        let resolve = |ids: Vec<String>| ids.iter()
            .filter_map(|id| entries.iter().find(|entry| &entry.task_id == id).cloned()).collect();
        LaneView { capacity: snapshot.capacity, running: resolve(snapshot.running), queue: resolve(snapshot.waiting) }
    }

    /// A job for this run runs or waits.
    pub(crate) fn run_busy(&self, run_id: &str) -> bool {
        let view = self.lane_view();
        view.running.iter().chain(&view.queue).any(|entry| entry.run_id == run_id)
    }

    fn set_capacity(&self, capacity: usize) -> Result<(), AppErrorV1> {
        let _entries = self.entries.lock().expect("lane entries poisoned");
        if !self.lane.is_idle() {
            return Err(production_error("busy", "Change capacity when no Unity work runs or waits."));
        }
        // No worker can enter an idle lane while the submission lock is held. Persist first;
        // an I/O failure keeps the current admission limit intact.
        self.engine.save_capacity(capacity)?;
        self.lane.set_capacity(capacity)
    }

    /// Submits Unity work into the lane and records what it is for.
    pub(crate) fn submit(&self, request: SubmitRequest, run_id: &str, operation: &str) -> Result<CommandAcceptedV1, AppErrorV1> {
        let mut entries = self.entries.lock().expect("lane entries poisoned");
        let policy = TaskLanePolicy { key: Some(run_id.to_owned()),
            exclusive: matches!(operation, "amf.production.upload" | "amf.production.open" | "amf.production.closeEditor" | "amf.production.delete") };
        let accepted = self.runtime.submit_in_lane_with_policy(request, &self.lane, policy)?;
        let snapshot = self.lane.snapshot();
        let live: Vec<String> = snapshot.running.into_iter().chain(snapshot.waiting).collect();
        entries.retain(|entry| live.contains(&entry.task_id));
        entries.push(LaneEntry { task_id: accepted.task_id.clone(), run_id: run_id.to_owned(), operation: operation.to_owned() });
        Ok(accepted)
    }
}

pub(crate) fn request(
    service: Option<Arc<AmfProductionServices>>,
    method: &str,
    request: &Value,
    request_id: &str,
    correlation_id: &str,
) -> FrameOutcome {
    let Some(service) = service else {
        return FrameOutcome::Response(application_error(
            request_id,
            correlation_id,
            "vua.amf_production.unavailable",
            "errors.amfProduction.unavailable",
            "unavailable",
        ));
    };
    let result = route(service, method, request);
    FrameOutcome::Response(match result {
        Ok(value) => application_success(request_id, value),
        Err(error) => application_error_with_params(
            request_id,
            correlation_id,
            &error.code,
            &error.message_key,
            app_error_category(error.category),
            &error
                .params
                .as_ref()
                .map(|params| {
                    params
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.clone()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        ),
    })
}
/// The run as the player sees it now. Work whose task no longer runs (VUA or its AMF process
/// ended mid-step) needs inspection, and its unfinished step reads as interrupted.
fn project_view(mut run: ProductionRun, service: &AmfProductionServices) -> ProductionRun {
    if service.run_busy(&run.run_id) {
        return run;
    }
    let active = service
        .runtime
        .snapshot(&run.task_id)
        .is_some_and(|task| !task.state.is_terminal());
    if !active {
        if ACTIVE_PHASES.contains(&run.phase.as_str()) {
            run.phase = "inspect_required".into();
        }
        if let Some(step) = run.steps.last_mut().filter(|step| step.state == "running") {
            step.state = "interrupted".into();
        }
    }
    run
}
fn wire_run(run: ProductionRun) -> Value {
    let mut value = serde_json::to_value(run).expect("run serialization");
    if let Some(sources) = value["sources"].as_array_mut() {
        for source in sources {
            source
                .as_object_mut()
                .expect("source object")
                .remove("assetPaths");
        }
    }
    // The materials kept with a restore point stay inside the record.
    let object = value.as_object_mut().expect("run object");
    object.remove("snapshotSources");
    // Fit provenance and observation bookkeeping stay in the record; the view carries the
    // digest-bound `fitChanges` and `availableFits` instead.
    object.remove("appliedFits");
    object.remove("observedAt");
    // A version list shows each record's summary; the full replay state stays in the record.
    if let Some(revisions) = value.get_mut("revisions").and_then(Value::as_array_mut) {
        for revision in revisions {
            revision
                .as_object_mut()
                .expect("revision object")
                .remove("state");
        }
    }
    value
}
/// Padded standard base64, as data URLs and MCP image content expect.
fn standard_base64(bytes: &[u8]) -> String {
    let mut text = vua_orchestrator::base64_url_encode(bytes).replace('-', "+").replace('_', "/");
    while !text.len().is_multiple_of(4) {
        text.push('=');
    }
    text
}
fn route(
    service: Arc<AmfProductionServices>,
    method: &str,
    request: &Value,
) -> Result<Value, vua_orchestrator::AppErrorV1> {
    if let Some(result) = super::amf_release_routes::route(&service, method, request) {
        return result;
    }
    let query = matches!(
        method,
        "amf.production.faceTrackingPlan" |
        "amf.production.list" | "amf.production.get" | "amf.production.plan" | "amf.production.rollbackPlan" | "amf.production.revisionPlan" | "amf.production.listRecipes" | "amf.production.getRecipe" | "amf.production.recipeSharePlan" | "amf.production.recipeMaterialPlan" | "amf.production.previewRecipeShare" | "amf.production.recipeApplyPlan" | "amf.production.previewImage" | "amf.production.planOutfitRemoval" | "amf.production.planLighting" | "amf.production.planTextureMemory" | "amf.production.listFits"
    );
    if request.get("kind").and_then(Value::as_str) != Some(if query { "query" } else { "command" })
    {
        return Err(production_error(
            "params_invalid",
            "Production method kind does not match its operation.",
        ));
    }
    let params = request
        .get("params")
        .and_then(Value::as_object)
        .ok_or_else(|| production_error("params_invalid", "Missing params."))?;
    let keys: &[&str] = match method {
        "amf.production.planOutfitRemoval" => &["schemaVersion", "runId", "sourceId"],
        "amf.production.removeOutfit" => &["schemaVersion", "runId", "sourceId", "planDigest", "confirmed"],
        "amf.production.faceTrackingPlan" => &["schemaVersion", "runId", "menuId"],
        "amf.production.faceTrackingPatch" => &["schemaVersion", "runId", "menuId", "planDigest", "confirmTrustedCode"],
        "amf.production.list" | "amf.production.listRecipes" | "amf.production.listFits" => &["schemaVersion"],
        "amf.production.saveFit" => &["schemaVersion", "runId", "sourceId", "name", "changesDigest"],
        "amf.production.renameFit" => &["schemaVersion", "fitId", "name"],
        "amf.production.deleteFit" => &["schemaVersion", "fitId"],
        "amf.production.closeEditor" => &["schemaVersion", "runId", "confirmed"],
        "amf.production.setCapacity" => &["schemaVersion", "capacity"],
        "amf.production.getRecipe" | "amf.production.recipeSharePlan" => &["schemaVersion", "recipeId"],
        "amf.production.previewRecipeShare" | "amf.production.importRecipe" => &["schemaVersion", "text"],
        "amf.production.exportRecipe" => &["schemaVersion", "recipeId", "planDigest", "sources"],
        "amf.production.saveRecipe" => &["schemaVersion", "runId", "title"],
        "amf.production.previewImage" => &["schemaVersion", "runId", "path"],
        "amf.production.prepareRecipe" => &["schemaVersion", "recipeId", "editorPath"],
        "amf.production.recipeMaterialPlan" => &["schemaVersion", "recipeId"],
        "amf.production.recipeApplyPlan" if params.contains_key("avatarGlobalObjectId") => &["schemaVersion", "runId", "recipeId", "avatarGlobalObjectId"],
        "amf.production.recipeApplyPlan" => &["schemaVersion", "runId", "recipeId"],
        "amf.production.applyRecipe" if params.contains_key("avatarGlobalObjectId") => &["schemaVersion", "runId", "recipeId", "planDigest", "choice", "avatarGlobalObjectId"],
        "amf.production.applyRecipe" => &["schemaVersion", "runId", "recipeId", "planDigest", "choice"],
        "amf.production.get" | "amf.production.reinspect" | "amf.production.open" | "amf.production.rollbackPlan" | "amf.production.planLighting" => {
            &["schemaVersion", "runId"]
        }
        "amf.production.prepare" => &["schemaVersion", "sources", "editorPath"],
        "amf.production.plan" => &["schemaVersion", "runId", "selection"],
        "amf.production.assemble" => &["schemaVersion", "runId", "planDigest", "selection"],
        "amf.production.rollback" if params.contains_key("target") => &["schemaVersion", "runId", "planDigest", "target"],
        "amf.production.rollback" => &["schemaVersion", "runId", "planDigest"],
        "amf.production.revisionPlan" => &["schemaVersion", "runId", "revisionId"],
        "amf.production.restoreRevision" => &["schemaVersion", "runId", "revisionId", "planDigest"],
        "amf.production.planTextureMemory" => &["schemaVersion", "runId", "platform", "targetRank"],
        "amf.production.applyTextureMemory" => &["schemaVersion", "runId", "platform", "targetRank", "planDigest", "textures"],
        "amf.production.applyLighting" => &["schemaVersion", "runId", "planDigest", "materials"],
        "amf.production.sdkFix" => &["schemaVersion", "runId", "fixes"],
        "amf.production.changeSources" => &["schemaVersion", "runId", "sources"],
        "amf.production.delete" => &["schemaVersion", "runId"],
        _ => {
            return Err(production_error(
                "method_invalid",
                "Unknown production method.",
            ))
        }
    };
    let optional_keys: &[&str] = match method {
        "amf.production.recipeMaterialPlan" | "amf.production.prepareRecipe" | "amf.production.recipeApplyPlan" => &["materialChoices"],
        "amf.production.applyRecipe" => &["materialChoices","recipeDifferencesConfirmed"],
        "amf.production.assemble" => &["recipeDifferencesConfirmed"],
        _ => &[],
    };
    if params.keys().any(|key| !keys.contains(&key.as_str()) && !optional_keys.contains(&key.as_str()))
        || keys.iter().any(|key| !params.contains_key(*key))
        || params["schemaVersion"] != "0.1"
    {
        return Err(production_error(
            "params_invalid",
            "Unknown keys or unsupported production version.",
        ));
    }
    let material_choices: vua_orchestrator::amf_recipe_materials::MaterialChoices = match params.get("materialChoices") {
        Some(value) => serde_json::from_value(value.clone()).map_err(|_| production_error("params_invalid", "Material choices map material IDs to source IDs."))?,
        None => Default::default(),
    };
    if material_choices.len() > 64 || material_choices.iter().any(|(id,source)| !vua_orchestrator::amf_recipe::valid_material_id(id) || source.is_empty() || source.len() > 256 || source.contains('\0')) {
        return Err(production_error("params_invalid", "Invalid Recipe material choices."));
    }
    let differences_confirmed = match params.get("recipeDifferencesConfirmed") {
        None => false,
        Some(Value::Bool(true)) => true,
        _ => return Err(production_error("params_invalid", "Confirm the reviewed Recipe differences explicitly.")),
    };
    let string = |key: &str| -> Result<String, vua_orchestrator::AppErrorV1> {
        params
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| production_error("params_invalid", format!("Missing {key}.")))
    };
    if method == "amf.production.closeEditor" && params.get("confirmed") != Some(&Value::Bool(true)) {
        return Err(production_error("confirmation_required", "Confirm saving the open scenes and assets and closing Unity."));
    }
    let avatar_id = if params.contains_key("avatarGlobalObjectId") {
        let id = string("avatarGlobalObjectId")?;
        if id.len() > 256 || id.contains('\0') { return Err(production_error("params_invalid", "Invalid Avatar identity.")); }
        Some(id)
    } else { None };
    let revision_id = if matches!(method, "amf.production.revisionPlan" | "amf.production.restoreRevision") {
        let id = string("revisionId")?;
        if id.len() > 128 || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')) {
            return Err(production_error("params_invalid", "Invalid version record identity."));
        }
        Some(id)
    } else { None };
    if matches!(method, "amf.production.assemble" | "amf.production.rollback" | "amf.production.applyRecipe" | "amf.production.applyLighting" | "amf.production.applyTextureMemory" | "amf.production.restoreRevision" | "amf.production.faceTrackingPatch" | "amf.production.removeOutfit") {
        let digest = string("planDigest")?;
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
            return Err(production_error("params_invalid", "A reviewed plan digest is required."));
        }
    }
    // The restore target names which reviewed copy the digest bound; absent keeps the default.
    let rollback_target = if method == "amf.production.rollback" {
        Some(RollbackTarget::parse(params.get("target").and_then(Value::as_str))?)
    } else { None };
    if method == "amf.production.saveFit" {
        let digest = string("changesDigest")?;
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
            return Err(production_error("params_invalid", "A reviewed fit changes digest is required."));
        }
    }
    if matches!(method, "amf.production.renameFit" | "amf.production.deleteFit") {
        let id = string("fitId")?;
        if id.strip_prefix("fit-").is_none_or(|rest| rest.is_empty() || rest.len() > 72)
            || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(production_error("params_invalid", "Invalid fit identity."));
        }
    }
    if matches!(method, "amf.production.saveFit" | "amf.production.renameFit") {
        let name = string("name")?;
        if name.chars().count() > 80 || !name.chars().any(|c| !c.is_whitespace()) {
            return Err(production_error("params_invalid", "A fit name is a single line of 1-80 characters."));
        }
    }
    if method == "amf.production.listFits" {
        let fits = service.engine.list_fits()?.iter().map(|fit| {
            json!({"fitId":fit.fit_id,"name":fit.name,"createdAt":fit.created_at,"updatedAt":fit.updated_at,
                "garment":fit.garment,"avatar":fit.avatar,"objectCount":fit.objects.len()})
        }).collect::<Vec<_>>();
        return Ok(json!({"schemaVersion":"0.1","fits":fits}));
    }
    if method == "amf.production.renameFit" {
        let fit = service.engine.rename_fit(&string("fitId")?, &string("name")?)?;
        return Ok(json!({"schemaVersion":"0.1","fitId":fit.fit_id,"name":fit.name}));
    }
    if method == "amf.production.deleteFit" {
        let fit_id = string("fitId")?;
        service.engine.delete_fit(&fit_id)?;
        return Ok(json!({"schemaVersion":"0.1","fitId":fit_id}));
    }
    if method == "amf.production.saveFit" {
        let run_id = string("runId")?;
        if service.run_busy(&run_id) {
            return Err(production_error("busy", "This line has work running or waiting."));
        }
        let fit = service.engine.save_fit(&run_id, &string("sourceId")?, &string("name")?, &string("changesDigest")?)?;
        return Ok(json!({"schemaVersion":"0.1","fitId":fit.fit_id,"name":fit.name,"createdAt":fit.created_at,"updatedAt":fit.updated_at}));
    }
    if method == "amf.production.list" {
        let view = service.lane_view();
        let runs = service.engine.list()?.into_iter().map(|run| {
            let run = project_view(run, &service);
            json!({"runId":run.run_id,"phase":run.phase,"sourceNames":run.sources.iter().map(|s| &s.item_name).take(3).collect::<Vec<_>>()})
        }).collect::<Vec<_>>();
        return Ok(
            json!({"schemaVersion":"0.1","sources":service.engine.sources()?,"editors":service.engine.editors()?,"runs":runs,"busy":!view.running.is_empty(),
                "capacity":view.capacity,"running":view.running,"queue":view.queue}),
        );
    }
    if method == "amf.production.setCapacity" {
        let capacity = params["capacity"].as_u64().filter(|capacity| (1..=3).contains(capacity))
            .ok_or_else(|| production_error("params_invalid", "Capacity must be 1, 2 or 3."))? as usize;
        service.set_capacity(capacity)?;
        return Ok(json!({"schemaVersion":"0.1","capacity":capacity}));
    }
    if method == "amf.production.previewImage" {
        let (run_id, path) = (string("runId")?, string("path")?);
        if !is_preview_path(&path) {
            return Err(production_error("params_invalid", "A preview path names a PNG under .vua/previews."));
        }
        let bytes = service.engine.preview_image(&run_id, &path)?;
        return Ok(json!({"schemaVersion":"0.1","runId":run_id,"path":path,"mediaType":"image/png","data":standard_base64(&bytes)}));
    }
    if method == "amf.production.get" {
        let run = project_view(service.engine.get(&string("runId")?)?, &service);
        let progress = ACTIVE_PHASES
            .contains(&run.phase.as_str())
            .then(|| service.engine.progress(&run))
            .flatten();
        // The run view's fit facts: the save hook's overlay counts when it is newer than the
        // last observation; an unreadable overlay simply counts as absent.
        let (fit_changes, available_fits) = service.engine.fit_view(&run);
        let run_id = run.run_id.clone();
        let mut value = wire_run(run);
        if let Some(object) = value.as_object_mut() {
            object.remove("fitChanges");
        }
        if !fit_changes.is_empty() {
            value["fitChanges"] = json!(fit_changes.iter().map(|entry| {
                json!({"sourceId":entry.source_id,"moved":entry.moved,"missing":entry.missing,
                    "changesDigest":changes_digest(&run_id, &entry.source_id, entry)})
            }).collect::<Vec<_>>());
        }
        if !available_fits.is_empty() {
            value["availableFits"] = json!(available_fits);
        }
        value["usualDurations"] = json!(service.engine.usual_durations());
        value["progress"] = progress.unwrap_or(Value::Null);
        return Ok(value);
    }
    if method == "amf.production.listRecipes" {
        return Ok(json!({"schemaVersion":"0.1", "recipes":service.engine.list_recipes()?}));
    }
    if method == "amf.production.getRecipe" {
        return Ok(json!({"schemaVersion":"0.1", "document":service.engine.get_recipe(&string("recipeId")?)?}));
    }
    if method == "amf.production.recipeMaterialPlan" {
        return Ok(json!({"schemaVersion":"0.1","plan":service.engine.recipe_material_plan(&string("recipeId")?, &material_choices)?}));
    }
    if method == "amf.production.recipeSharePlan" {
        return Ok(json!({"schemaVersion":"0.1", "plan":service.engine.recipe_share_plan(&string("recipeId")?)?}));
    }
    if method == "amf.production.previewRecipeShare" {
        return Ok(json!({"schemaVersion":"0.1", "document":vua_orchestrator::amf_recipe_share::SharedRecipe::decode(&string("text")?)?}));
    }
    // Plans read a run as it is; work for that run, running or waiting, changes it first.
    if method == "amf.production.rollbackPlan" {
        let run_id = string("runId")?;
        if service.run_busy(&run_id) {
            return Err(production_error("busy", "This line has work running or waiting."));
        }
        return service.engine.rollback_plan(&run_id);
    }
    if method == "amf.production.revisionPlan" {
        let run_id = string("runId")?;
        if service.run_busy(&run_id) {
            return Err(production_error("busy", "This line has work running or waiting."));
        }
        return service.engine.revision_plan(&run_id, &revision_id.expect("version record identity validated"));
    }
    if method == "amf.production.plan" {
        let run_id = string("runId")?;
        if service.run_busy(&run_id) {
            return Err(production_error("busy", "This line has work running or waiting."));
        }
        let selection: AssemblySelection = serde_json::from_value(params["selection"].clone())
            .map_err(|e| production_error("selection_invalid", e.to_string()))?;
        let run = project_view(service.engine.get(&run_id)?, &service);
        if run.phase == "inspect_required" {
            return Err(production_error(
                "inspect_required",
                "Reinspect interrupted work before planning.",
            ));
        }
        return service.engine.plan(&run_id, &selection);
    }
    let texture_options = if matches!(method, "amf.production.planTextureMemory" | "amf.production.applyTextureMemory") {
        let platform: TexturePlatform = serde_json::from_value(params["platform"].clone()).map_err(|_| production_error("params_invalid", "Choose PC or Android."))?;
        let target: TextureRank = serde_json::from_value(params["targetRank"].clone()).map_err(|_| production_error("params_invalid", "Choose a target texture rank."))?;
        if target.budget(platform).is_none() { return Err(production_error("params_invalid", "Choose Excellent, Good, Medium or Poor.")); }
        Some((platform, target))
    } else { None };
    if method == "amf.production.planTextureMemory" {
        let id = string("runId")?;
        if service.run_busy(&id) { return Err(production_error("busy", "This line has work running or waiting.")); }
        let (platform, target) = texture_options.expect("validated texture options");
        return service.engine.plan_texture_memory(&id, platform, target);
    }
    let removal_source = if matches!(method, "amf.production.planOutfitRemoval" | "amf.production.removeOutfit") {
        let id = string("sourceId")?;
        if id.chars().count() > 256 || id.contains('\0') { return Err(production_error("params_invalid", "Invalid outfit source identity.")); }
        if method == "amf.production.removeOutfit" && params["confirmed"] != true {
            return Err(production_error("confirmation_required", "Confirm the reviewed outfit removal."));
        }
        Some(id)
    } else { None };
    if method == "amf.production.planOutfitRemoval" {
        let run_id = string("runId")?;
        if service.run_busy(&run_id) { return Err(production_error("busy", "This line has work running or waiting.")); }
        return service.engine.plan_outfit_removal(&run_id, removal_source.as_deref().expect("validated outfit"));
    }
    if method == "amf.production.planLighting" {
        let run_id = string("runId")?;
        if service.run_busy(&run_id) {
            return Err(production_error("busy", "This line has work running or waiting."));
        }
        return service.engine.plan_lighting(&run_id);
    }
    if method == "amf.production.faceTrackingPlan" {
        let run_id = string("runId")?;
        if service.run_busy(&run_id) { return Err(production_error("busy", "This line has work running or waiting.")); }
        return service.engine.face_tracking_plan(&run_id, &string("menuId")?);
    }
    if method == "amf.production.recipeApplyPlan" {
        let run_id = string("runId")?;
        if service.run_busy(&run_id) {
            return Err(production_error("busy", "This line has work running or waiting."));
        }
        return Ok(json!({"schemaVersion":"0.1", "plan":service.engine.recipe_apply_plan_with_choices_for(&run_id, &string("recipeId")?, avatar_id.as_deref(), &material_choices)?}));
    }
    let recipe_file = matches!(method, "amf.production.exportRecipe" | "amf.production.importRecipe" | "amf.production.saveRecipe");
    if recipe_file && service.busy() {
        return Err(production_error("busy", "Wait for the current production operation to finish."));
    }
    // Checks against the work project run when the queued job acquires its slot.
    let queued = !service.lane.is_idle();
    if method == "amf.production.exportRecipe" {
        let sources: std::collections::BTreeMap<String,String> = serde_json::from_value(params["sources"].clone()).map_err(|_| production_error("recipe_sources_invalid", "Expected material source choices."))?;
        return Ok(json!({"schemaVersion":"0.1", "export":service.engine.export_recipe(&string("recipeId")?, &string("planDigest")?, &sources)?}));
    }
    if method == "amf.production.importRecipe" {
        return Ok(json!({"schemaVersion":"0.1", "document":service.engine.import_recipe(&string("text")?)?}));
    }
    if method == "amf.production.saveRecipe" {
        let id = format!("recipe-{}", uuid_v7_identity());
        return Ok(json!({"schemaVersion":"0.1", "document":service.engine.save_recipe(&string("runId")?, &id, &string("title")?)?}));
    }
    let run_id = if method == "amf.production.prepareRecipe" {
        let id = format!("n3-{}", uuid_v7_identity());
        service.engine.begin_recipe_with_choices(&id, &string("recipeId")?, &string("editorPath")?, &material_choices)?;
        id
    } else if method == "amf.production.prepare" {
        let choices: Vec<SourceChoice> = serde_json::from_value(params["sources"].clone())
            .map_err(|e| production_error("sources_invalid", e.to_string()))?;
        let id = format!("n3-{}", uuid_v7_identity());
        service.engine.begin(&id, choices, &string("editorPath")?)?;
        id
    } else {
        string("runId")?
    };
    if method == "amf.production.open" && !queued {
        service.engine.precheck_open(&run_id)?;
    }
    if matches!(method, "amf.production.reinspect" | "amf.production.assemble" | "amf.production.changeSources" | "amf.production.sdkFix"
        | "amf.production.rollback" | "amf.production.applyRecipe" | "amf.production.applyLighting" | "amf.production.applyTextureMemory" | "amf.production.delete"
        | "amf.production.restoreRevision" | "amf.production.faceTrackingPatch" | "amf.production.removeOutfit") {
        service.engine.precheck_closed(&run_id)?;
    }
    if method == "amf.production.delete" && service.run_busy(&run_id) {
        return Err(production_error("busy", "This line has work running or waiting."));
    }
    let changed_sources: Vec<SourceChoice> = if method == "amf.production.changeSources" {
        let choices: Vec<SourceChoice> = serde_json::from_value(params["sources"].clone())
            .map_err(|e| production_error("sources_invalid", e.to_string()))?;
        validate_choices(&choices)?;
        choices
    } else { Vec::new() };
    let selection: Option<AssemblySelection> = params
        .get("selection")
        .map(|s| serde_json::from_value(s.clone()))
        .transpose()
        .map_err(|e| production_error("selection_invalid", e.to_string()))?;
    let digest = params
        .get("planDigest")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let action = method.to_owned();
    let recipe_application = if method == "amf.production.applyRecipe" {
        let choice: vua_orchestrator::amf_recipe_apply::RecipeApplyChoice = serde_json::from_value(params["choice"].clone())
            .map_err(|_| production_error("params_invalid", "Choose how to apply the reviewed Recipe."))?;
        Some((string("recipeId")?, choice))
    } else { None };
    let sdk_fixes: Vec<String> = if method == "amf.production.sdkFix" {
        let fixes: Vec<String> = serde_json::from_value(params["fixes"].clone())
            .map_err(|_| production_error("params_invalid", "Name the SDK items to fix."))?;
        if fixes.is_empty() || fixes.len() > vua_orchestrator::amf_production::SDK_FIX_LIMIT
            || fixes.iter().any(|fix| fix.is_empty() || fix.chars().count() > 600 || fix.contains('\0')) {
            return Err(production_error("params_invalid", "Name between one and sixteen SDK items to fix."));
        }
        fixes
    } else { Vec::new() };
    let ft_menu = if method == "amf.production.faceTrackingPatch" {
        let menu = string("menuId")?;
        if menu.len() != 64 || !menu.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || params["confirmTrustedCode"] != true {
            return Err(production_error("confirmation_required", "Confirm the reviewed imported menu and its code/replay limits."));
        }
        Some(menu)
    } else { None };
    let lighting_materials: Vec<String> = if method == "amf.production.applyLighting" {
        let materials: Vec<String> = serde_json::from_value(params["materials"].clone())
            .map_err(|_| production_error("params_invalid", "Name the reviewed materials to change."))?;
        if materials.is_empty() || materials.len() > 512
            || materials.iter().any(|material| !vua_orchestrator::amf_production::safe_material_path(material))
            || materials.iter().collect::<std::collections::BTreeSet<_>>().len() != materials.len() {
            return Err(production_error("params_invalid", "Name distinct reviewed materials to change."));
        }
        materials
    } else { Vec::new() };
    let textures: Vec<String> = if method == "amf.production.applyTextureMemory" {
        let paths: Vec<String> = serde_json::from_value(params["textures"].clone()).map_err(|_| production_error("params_invalid", "Name reviewed textures."))?;
        if paths.is_empty() || paths.len() > 2048 || paths.iter().any(|p| !safe_texture_path(p))
            || paths.iter().collect::<std::collections::BTreeSet<_>>().len() != paths.len() {
            return Err(production_error("params_invalid", "Choose distinct reviewed texture paths."));
        }
        paths
    } else { Vec::new() };
    let operation_run = run_id.clone();
    let worker = service.clone();
    let accepted = service.submit(SubmitRequest {
        correlation_id: request
            .get("correlationId")
            .and_then(Value::as_str)
            .map(str::to_owned),
        timeout: None,
        job: Box::new(move |ctx| {
            match action.as_str() {
                "amf.production.prepare" | "amf.production.prepareRecipe" => worker.engine.prepare(&operation_run, ctx)?,
                "amf.production.assemble" => worker.engine.assemble_with_review(
                    &operation_run,
                    selection.expect("assemble selection validated"),
                    &digest,
                    differences_confirmed,
                    ctx,
                )?,
                "amf.production.reinspect" => worker.engine.reinspect(&operation_run, ctx)?,
                "amf.production.sdkFix" => worker.engine.sdk_fix(&operation_run, sdk_fixes, ctx)?,
                "amf.production.faceTrackingPatch" => worker.engine.face_tracking_patch(&operation_run,
                    ft_menu.as_deref().expect("confirmed menu"), &digest, true, ctx)?,
                "amf.production.applyTextureMemory" => {
                    let (platform, target) = texture_options.expect("validated texture options");
                    worker.engine.apply_texture_memory(&operation_run, platform, target, &digest, &textures, ctx)?;
                }
                "amf.production.removeOutfit" => worker.engine.remove_outfit(&operation_run, removal_source.as_deref().expect("confirmed outfit"), &digest, true, ctx)?,
                "amf.production.applyLighting" => worker.engine.apply_lighting(&operation_run, &digest, &lighting_materials, ctx)?,
                "amf.production.changeSources" => worker.engine.change_sources(&operation_run, changed_sources, ctx)?,
                "amf.production.delete" => worker.engine.delete_run(&operation_run)?,
                "amf.production.applyRecipe" => {
                    let (recipe, choice) = recipe_application.expect("Recipe application validated");
                    worker.engine.apply_recipe_with_review_to(&operation_run, &recipe, avatar_id.as_deref(), choice, &digest, &material_choices, differences_confirmed, ctx)?;
                }
                "amf.production.rollback" => worker.engine.rollback(&operation_run, &digest, rollback_target, ctx)?,
                "amf.production.restoreRevision" => worker.engine.restore_revision(
                    &operation_run,
                    revision_id.as_deref().expect("version record identity validated"),
                    &digest,
                    ctx,
                )?,
                "amf.production.closeEditor" => {
                    return worker.engine.close_editor(&operation_run, true, ctx).map(TaskExit::Done)
                }
                "amf.production.open" => {
                    return worker.engine.open(&operation_run, ctx).map(TaskExit::Done)
                }
                _ => {
                    return Err(production_error(
                        "method_invalid",
                        "Unknown production action.",
                    ))
                }
            }
            if ctx.check_cancel() {
                return Ok(TaskExit::Cancelled);
            }
            Ok(TaskExit::Done(
                json!({"schemaVersion":"0.1","operation":"amf.production","runId":operation_run}),
            ))
        }),
    }, &run_id, method)?;
    Ok(json!({"schemaVersion":"0.1","operation":method,"runId":run_id,"taskId":accepted.task_id}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use vua_orchestrator::{NanosTaskIdGenerator, SqliteTaskStore, SystemClock};

    #[test]
    fn wire_run_preserves_absent_revisions_and_hides_retained_replay_state() {
        let mut run: ProductionRun = serde_json::from_value(json!({
            "schemaVersion":"0.1", "runId":"n3-wire", "phase":"ready", "taskId":"task-wire",
            "editor":{"path":"Unity.exe","version":"2022.3.22f1"}, "projectRoot":"synthetic-project",
            "choices":[], "sources":[], "prefabs":[], "projectFingerprint":"", "unityFingerprint":"",
            "steps":[], "selection":null, "confirmedDigest":null, "result":null, "error":null, "recovery":null
        })).unwrap();
        assert!(wire_run(run.clone()).get("revisions").is_none(),
            "An empty optional version list must stay absent, not become null");
        run.revisions.push(serde_json::from_value(json!({
            "id":"rev-wire", "before":"assembly", "takenAt":"2026-10-25T00:00:00Z",
            "state":{}, "summary":{"avatar":null,"outfits":[],"sources":[],"lightingMaterials":0,"sdkFixes":[]},
            "editedInUnity":false
        })).unwrap());
        let view = wire_run(run);
        let revisions = view.get("revisions").and_then(Value::as_array).unwrap();
        assert_eq!(revisions.len(), 1);
        assert_eq!(revisions[0]["id"], "rev-wire");
        assert_eq!(revisions[0]["summary"]["lightingMaterials"], 0);
        assert!(revisions[0].get("state").is_none(), "Replay state stays inside the local record");
    }

    #[test]
    fn work_whose_task_ended_reads_interrupted_with_its_usual_durations() {
        let root = std::env::temp_dir().join(format!("vua-amf-routes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("amf/runs")).unwrap();
        let platform = crate::amf_production_adapter::NativeAmfProduction {
            bdl: Arc::new(vua_bdl_store::BdlStore::open(root.join("bdl.db")).unwrap()),
            root: root.join("amf"), editor_roots: vec![], explicit_editor: None, unity_hub: None,
        };
        let service = Arc::new(AmfProductionServices::new(
            AmfProductionEngine::new(root.join("amf"), Arc::new(platform)),
            TaskRuntime::with_sqlite(Arc::new(SqliteTaskStore::open(root.join("tasks.db")).unwrap()),
                Arc::new(SystemClock), Arc::new(NanosTaskIdGenerator::default())).unwrap(),
        ));
        let step = |key: &str, state: &str, start: &str, end: Option<&str>| json!({"key":key,"name":key,"state":state,"detail":"",
            "startedAt":format!("2026-10-17T{start}.000Z"),"endedAt":end.map(|end| format!("2026-10-17T{end}.000Z"))});
        let record = json!({"schemaVersion":"0.1","runId":"n3-routes","phase":"inspecting","taskId":"task-ended",
            "editor":{"path":"Unity.exe","version":"2022.3.22f1"},"projectRoot":root.join("amf/projects/n3-routes"),
            "choices":[],"sources":[],"prefabs":[],"projectFingerprint":"","unityFingerprint":"",
            "steps":[step("inspect","succeeded","08:00:00",Some("08:06:52")), step("reinspect","running","09:00:00",None)],
            "selection":null,"confirmedDigest":null,"result":null,"error":null,"recovery":null});
        std::fs::write(root.join("amf/runs/n3-routes.json"), serde_json::to_vec(&record).unwrap()).unwrap();

        let view = route(service, "amf.production.get", &json!({"kind":"query","params":{"schemaVersion":"0.1","runId":"n3-routes"}})).unwrap();
        assert_eq!(view["phase"], "inspect_required");
        assert_eq!(view["steps"][1]["state"], "interrupted");
        assert_eq!(view["steps"][1]["endedAt"], Value::Null);
        assert_eq!(view["usualDurations"], json!({"inspect":412}));
        assert_eq!(view["progress"], Value::Null, "work that no longer runs has no live progress");
        let schema = serde_json::from_slice(&std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/amf-production-flow/v0.1/status.schema.json")).unwrap()).unwrap();
        assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&view), "{view}");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn work_asked_for_meanwhile_waits_in_order_and_names_its_line() {
        let root = std::env::temp_dir().join(format!("vua-amf-lane-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("amf/runs")).unwrap();
        let platform = crate::amf_production_adapter::NativeAmfProduction {
            bdl: Arc::new(vua_bdl_store::BdlStore::open(root.join("bdl.db")).unwrap()),
            root: root.join("amf"), editor_roots: vec![], explicit_editor: None, unity_hub: None,
        };
        let service = AmfProductionServices::new(
            AmfProductionEngine::new(root.join("amf"), Arc::new(platform)),
            TaskRuntime::with_sqlite(Arc::new(SqliteTaskStore::open(root.join("tasks.db")).unwrap()),
                Arc::new(SystemClock), Arc::new(NanosTaskIdGenerator::default())).unwrap(),
        );
        let (release, gate) = std::sync::mpsc::channel::<()>();
        let held = SubmitRequest { correlation_id: None, timeout: None, job: Box::new(move |_ctx| {
            let _ = gate.recv_timeout(std::time::Duration::from_secs(10));
            Ok(TaskExit::Done(json!({})))
        }) };
        let running = service.submit(held, "n3-a", "amf.production.reinspect").unwrap().task_id;
        while service.lane.holder().is_none() { std::thread::sleep(std::time::Duration::from_millis(5)); }
        let quick = || SubmitRequest { correlation_id: None, timeout: None, job: Box::new(|_ctx| Ok(TaskExit::Done(json!({})))) };
        let upload = service.submit(quick(), "n3-b", "amf.production.upload").unwrap().task_id;
        let bake = service.submit(quick(), "n3-c", "amf.production.bakeTurntable").unwrap().task_id;
        assert!(service.busy());
        assert_eq!(service.lane_view().queue.iter().map(|entry| (entry.task_id.clone(), entry.run_id.as_str(), entry.operation.as_str())).collect::<Vec<_>>(),
            vec![(upload.clone(), "n3-b", "amf.production.upload"), (bake, "n3-c", "amf.production.bakeTurntable")]);
        assert!(service.run_busy("n3-a") && service.run_busy("n3-b") && !service.run_busy("n3-d"));
        assert_ne!(running, upload);
        release.send(()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !service.lane.is_idle() { assert!(std::time::Instant::now() < deadline); std::thread::sleep(std::time::Duration::from_millis(5)); }
        assert!(!service.busy() && service.lane_view().queue.is_empty() && !service.run_busy("n3-b"));
        drop(service);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn services_at(root: &std::path::Path) -> Arc<AmfProductionServices> {
        std::fs::create_dir_all(root.join("amf/runs")).unwrap();
        let platform = crate::amf_production_adapter::NativeAmfProduction {
            bdl: Arc::new(vua_bdl_store::BdlStore::open(root.join("bdl.db")).unwrap()),
            root: root.join("amf"), editor_roots: vec![], explicit_editor: None, unity_hub: None,
        };
        Arc::new(AmfProductionServices::new(
            AmfProductionEngine::new(root.join("amf"), Arc::new(platform)),
            TaskRuntime::with_sqlite(Arc::new(SqliteTaskStore::open(root.join("tasks.db")).unwrap()),
                Arc::new(SystemClock), Arc::new(NanosTaskIdGenerator::default())).unwrap(),
        ))
    }

    #[test]
    fn texture_routes_require_exact_platform_target_digest_and_explicit_paths() {
        let root = std::env::temp_dir().join(format!("vua-texture-routes-{}", uuid_v7_identity()));
        let service = services_at(&root);
        let params = json!({"schemaVersion":"0.1","runId":"n3-missing","platform":"pc","targetRank":"good"});
        let error = route(service.clone(), "amf.production.planTextureMemory", &json!({"kind":"query","params":params})).unwrap_err();
        assert_eq!(error.code, "vua.amf_production.run_not_found", "valid input reaches the owning engine");
        for (key, invalid) in [("platform", json!("ios")), ("targetRank", json!("unknown")), ("extra", json!(true))] {
            let mut bad = params.clone(); bad[key] = invalid;
            let error = route(service.clone(), "amf.production.planTextureMemory", &json!({"kind":"query","params":bad})).unwrap_err();
            assert_eq!(error.code, "vua.amf_production.params_invalid");
        }
        let error = route(service.clone(), "amf.production.applyTextureMemory", &json!({"kind":"command","params":params})).unwrap_err();
        assert_eq!(error.code, "vua.amf_production.params_invalid", "a mutation cannot omit the reviewed digest and path list");
        drop(service); std::fs::remove_dir_all(root).unwrap();
    }

    fn list(service: &Arc<AmfProductionServices>) -> Value {
        route(service.clone(), "amf.production.list", &json!({"kind":"query","params":{"schemaVersion":"0.1"}})).unwrap()
    }

    fn capacity(service: &Arc<AmfProductionServices>, value: Value) -> Result<Value, AppErrorV1> {
        route(service.clone(), "amf.production.setCapacity", &json!({"kind":"command","params":{"schemaVersion":"0.1","capacity":value}}))
    }

    fn wait_until(mut predicate: impl FnMut() -> bool) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !predicate() {
            assert!(std::time::Instant::now() < deadline, "task did not reach expected state");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    fn held(service: &AmfProductionServices, run: &str, operation: &str) -> (String, std::sync::mpsc::Sender<()>) {
        let (release, gate) = std::sync::mpsc::channel();
        let accepted = service.submit(SubmitRequest { correlation_id: None, timeout: None, job: Box::new(move |_| {
            gate.recv_timeout(std::time::Duration::from_secs(10)).expect("test releases running job");
            Ok(TaskExit::Done(Value::Null))
        }) }, run, operation).unwrap();
        (accepted.task_id, release)
    }

    #[test]
    fn capacity_is_persisted_read_back_and_validated_without_changing_on_failure() {
        let root = std::env::temp_dir().join(format!("vua-amf-capacity-{}", uuid_v7_identity()));
        let mut service = services_at(&root);
        assert_eq!(list(&service)["capacity"], 1);
        assert!(!root.join("amf/settings.json").exists());
        for value in [2, 3, 1] {
            assert_eq!(capacity(&service, json!(value)).unwrap(), json!({"schemaVersion":"0.1","capacity":value}));
            drop(service);
            service = services_at(&root);
            assert_eq!(list(&service)["capacity"], value);
        }
        let saved = std::fs::read(root.join("amf/settings.json")).unwrap();
        assert!(!saved.starts_with(&[0xef, 0xbb, 0xbf]));
        for invalid in [json!(0), json!(4), json!(1.5), json!("2"), json!(true), Value::Null] {
            assert_eq!(capacity(&service, invalid).unwrap_err().code, "vua.amf_production.params_invalid");
        }
        assert_eq!(std::fs::read(root.join("amf/settings.json")).unwrap(), saved);
        assert_eq!(list(&service)["capacity"], 1);
        // A failed publish must not alter the in-memory limit or the last saved settings.
        std::fs::create_dir(root.join("amf/settings.pending")).unwrap();
        assert_eq!(capacity(&service, json!(2)).unwrap_err().code, "vua.amf_production.store_failed");
        assert_eq!(list(&service)["capacity"], 1);
        assert_eq!(std::fs::read(root.join("amf/settings.json")).unwrap(), saved);
        drop(service);
        for bad in ["{", "{\"capacity\":4}", "{\"capacity\":\"2\"}", "{}"] {
            std::fs::write(root.join("amf/settings.json"), bad).unwrap();
            let fallback = services_at(&root);
            assert_eq!(list(&fallback)["capacity"], 1);
            assert_eq!(std::fs::read_to_string(root.join("amf/settings.json")).unwrap(), bad);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn list_reports_both_running_lines_and_the_waiting_queue_and_cancellation() {
        let root = std::env::temp_dir().join(format!("vua-amf-running-{}", uuid_v7_identity()));
        let service = services_at(&root);
        capacity(&service, json!(2)).unwrap();
        let (a, release_a) = held(&service, "n3-a", "amf.production.reinspect");
        let (b, release_b) = held(&service, "n3-b", "amf.production.reinspect");
        wait_until(|| [a.as_str(), b.as_str()].iter().all(|id| service.runtime.snapshot(id).unwrap().state == vua_orchestrator::TaskState::Running));
        let (c, _release_c) = held(&service, "n3-c", "amf.production.bakeTurntable");
        let snapshot = list(&service);
        assert_eq!(snapshot["capacity"], 2);
        assert_eq!(snapshot["busy"], true);
        assert_eq!(snapshot["running"], json!([
            {"taskId":a,"runId":"n3-a","operation":"amf.production.reinspect"},
            {"taskId":b,"runId":"n3-b","operation":"amf.production.reinspect"}]));
        assert_eq!(snapshot["queue"], json!([{"taskId":c,"runId":"n3-c","operation":"amf.production.bakeTurntable"}]));
        assert_eq!(capacity(&service, json!(1)).unwrap_err().code, "vua.amf_production.busy");
        assert_eq!(list(&service)["capacity"], 2);
        assert_eq!(service.engine.capacity(), 2);
        service.runtime.cancel(&c).unwrap();
        wait_until(|| service.runtime.snapshot(&c).unwrap().state == vua_orchestrator::TaskState::Cancelled);
        assert_eq!(list(&service)["queue"], json!([]));
        assert_eq!(list(&service)["busy"], true);
        release_a.send(()).unwrap();
        release_b.send(()).unwrap();
        wait_until(|| service.lane.is_idle());
        assert_eq!(list(&service)["busy"], false);
        assert_eq!(list(&service)["running"], json!([]));
        drop(service);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn revision_methods_validate_their_params_before_touching_a_run() {
        let root = std::env::temp_dir().join(format!("vua-amf-revisions-{}", uuid_v7_identity()));
        let service = services_at(&root);
        let err = |method: &str, kind: &str, params: Value| {
            route(service.clone(), method, &json!({"kind":kind,"params":params})).unwrap_err().code
        };
        assert_eq!(err("amf.production.closeEditor", "query", json!({"schemaVersion":"0.1","runId":"n3-x","confirmed":true})), "vua.amf_production.params_invalid");
        assert_eq!(err("amf.production.closeEditor", "command", json!({"schemaVersion":"0.1","runId":"n3-x","confirmed":false})), "vua.amf_production.confirmation_required");
        assert_eq!(err("amf.production.closeEditor", "command", json!({"schemaVersion":"0.1","runId":"n3-x"})), "vua.amf_production.params_invalid");
        // The kind must match the operation.
        assert_eq!(err("amf.production.revisionPlan", "command", json!({"schemaVersion":"0.1","runId":"n3-x","revisionId":"rev-1"})), "vua.amf_production.params_invalid");
        assert_eq!(err("amf.production.restoreRevision", "query", json!({"schemaVersion":"0.1","runId":"n3-x","revisionId":"rev-1","planDigest":"a".repeat(64)})), "vua.amf_production.params_invalid");
        // The version record identity is a bounded plain identifier.
        assert_eq!(err("amf.production.revisionPlan", "query", json!({"schemaVersion":"0.1","runId":"n3-x","revisionId":"rev/1"})), "vua.amf_production.params_invalid");
        // A restore needs a reviewed plan digest.
        assert_eq!(err("amf.production.restoreRevision", "command", json!({"schemaVersion":"0.1","runId":"n3-x","revisionId":"rev-1","planDigest":"xyz"})), "vua.amf_production.params_invalid");
        // Well-formed requests reach the engine.
        assert_eq!(err("amf.production.revisionPlan", "query", json!({"schemaVersion":"0.1","runId":"n3-x","revisionId":"rev-1"})), "vua.amf_production.run_not_found");
        assert_eq!(err("amf.production.restoreRevision", "command", json!({"schemaVersion":"0.1","runId":"n3-x","revisionId":"rev-1","planDigest":"a".repeat(64)})), "vua.amf_production.run_not_found");
        drop(service);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fit_methods_validate_their_params_before_touching_state() {
        let root = std::env::temp_dir().join(format!("vua-amf-fits-{}", uuid_v7_identity()));
        let service = services_at(&root);
        let err = |method: &str, kind: &str, params: Value| {
            route(service.clone(), method, &json!({"kind":kind,"params":params})).unwrap_err().code
        };
        // The kind must match the operation.
        assert_eq!(err("amf.production.saveFit", "query", json!({"schemaVersion":"0.1","runId":"n3-x","sourceId":"outfit","name":"Fit","changesDigest":"a".repeat(64)})), "vua.amf_production.params_invalid");
        assert_eq!(err("amf.production.listFits", "command", json!({"schemaVersion":"0.1"})), "vua.amf_production.params_invalid");
        assert_eq!(err("amf.production.renameFit", "query", json!({"schemaVersion":"0.1","fitId":"fit-1","name":"Fit"})), "vua.amf_production.params_invalid");
        assert_eq!(err("amf.production.deleteFit", "query", json!({"schemaVersion":"0.1","fitId":"fit-1"})), "vua.amf_production.params_invalid");
        // A save binds a reviewed changes digest.
        assert_eq!(err("amf.production.saveFit", "command", json!({"schemaVersion":"0.1","runId":"n3-x","sourceId":"outfit","name":"Fit","changesDigest":"xyz"})), "vua.amf_production.params_invalid");
        // The fit identity is a bounded plain identifier and the name a bounded single line.
        assert_eq!(err("amf.production.renameFit", "command", json!({"schemaVersion":"0.1","fitId":"fit/1","name":"Fit"})), "vua.amf_production.params_invalid");
        assert_eq!(err("amf.production.deleteFit", "command", json!({"schemaVersion":"0.1","fitId":"fit-1","extra":true})), "vua.amf_production.params_invalid");
        assert_eq!(err("amf.production.renameFit", "command", json!({"schemaVersion":"0.1","fitId":"fit-1","name":"   "})), "vua.amf_production.params_invalid");
        // Well-formed requests reach the engine.
        assert_eq!(err("amf.production.saveFit", "command", json!({"schemaVersion":"0.1","runId":"n3-x","sourceId":"outfit","name":"Fit","changesDigest":"a".repeat(64)})), "vua.amf_production.run_not_found");
        assert_eq!(err("amf.production.renameFit", "command", json!({"schemaVersion":"0.1","fitId":"fit-1","name":"Fit"})), "vua.amf_production.fit_not_found");
        assert_eq!(err("amf.production.deleteFit", "command", json!({"schemaVersion":"0.1","fitId":"fit-1"})), "vua.amf_production.fit_not_found");
        assert_eq!(route(service.clone(), "amf.production.listFits", &json!({"kind":"query","params":{"schemaVersion":"0.1"}})).unwrap(), json!({"schemaVersion":"0.1","fits":[]}));
        drop(service);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn upload_open_and_delete_are_exclusive_and_preserve_queue_order() {
        for operation in ["amf.production.upload", "amf.production.open", "amf.production.closeEditor", "amf.production.delete"] {
            let root = std::env::temp_dir().join(format!("vua-amf-exclusive-{}", uuid_v7_identity()));
            let service = services_at(&root);
            capacity(&service, json!(2)).unwrap();
            let (a, release_a) = held(&service, "n3-a", "amf.production.reinspect");
            wait_until(|| service.runtime.snapshot(&a).unwrap().state == vua_orchestrator::TaskState::Running);
            let (b, release_b) = held(&service, "n3-b", operation);
            let (c, release_c) = held(&service, "n3-c", "amf.production.reinspect");
            std::thread::sleep(std::time::Duration::from_millis(100));
            assert_eq!(list(&service)["running"].as_array().unwrap().len(), 1);
            assert_eq!(list(&service)["queue"][0]["taskId"], b);
            assert_eq!(list(&service)["queue"][1]["taskId"], c);
            release_a.send(()).unwrap();
            wait_until(|| service.runtime.snapshot(&b).unwrap().state == vua_orchestrator::TaskState::Running);
            assert_eq!(list(&service)["running"], json!([{"taskId":b,"runId":"n3-b","operation":operation}]));
            assert_eq!(service.runtime.snapshot(&c).unwrap().state, vua_orchestrator::TaskState::Queued);
            release_b.send(()).unwrap();
            wait_until(|| service.runtime.snapshot(&c).unwrap().state == vua_orchestrator::TaskState::Running);
            release_c.send(()).unwrap();
            wait_until(|| service.lane.is_idle());
            drop(service);
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}
