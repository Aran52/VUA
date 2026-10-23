//! Material-first N3 use case. Platform facts and Unity stay behind this port;
//! confirmations, retained runs and recovery are AMF-owned.

use crate::{AppErrorV1, Clock, ErrorCategory, ParamValue, SystemClock, TaskContext};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

mod editor_close;
pub use editor_close::*;
mod release;
pub use release::*;
mod lighting;
pub use lighting::*;
mod texture_memory;
pub use texture_memory::*;
mod revisions;
pub use revisions::*;
mod fits;
pub use fits::*;
mod face_tracking;
pub use face_tracking::*;
mod parameter_budget;
pub use parameter_budget::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChoice {
    pub source_id: String,
    pub role: SourceRole,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    Auto,
    Avatar,
    Outfit,
    Dependency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSource {
    pub source_id: String,
    pub item_id: String,
    pub item_name: String,
    pub name: String,
    /// The BOOTH products the library entry maps to, for terms checks; empty when none is known.
    #[serde(default)]
    pub product_ids: Vec<String>,
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(skip)]
    pub archive_member: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionEditor {
    pub path: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutfitSelection {
    pub source_id: String,
    pub prefab_path: String,
    pub mode: BindingMode,
    pub bone: String,
    pub toggle_name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BindingMode {
    MergeArmature,
    AttachToBone,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssemblySelection {
    pub avatar_prefab: String,
    pub outfits: Vec<OutfitSelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefabCandidate {
    pub source_id: String,
    pub path: String,
    pub name: String,
    pub is_avatar: bool,
    pub has_skinned_mesh: bool,
    pub has_merge_armature: bool,
    pub has_missing_scripts: bool,
    pub humanoid_bones: Vec<String>,
    /// Work-project-relative PNG the Bridge rendered for this candidate during inspection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    /// Source evidence for the synced parameter budget; an older Bridge omits it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter_budget: Option<ParameterBudget>,
}

/// Bridge previews live in this work-project folder, one subfolder per Bridge command.
pub const PREVIEW_ROOT: &str = ".vua/previews";
/// One preview PNG, base64-encoded, fits in a single Gateway frame.
pub const MAX_PREVIEW_BYTES: u64 = 640 * 1024;
pub const PREVIEW_VIEWS: [&str; 3] = ["front", "side", "back"];

/// A preview path has the form `.vua/previews/<command>/<name>.png` with plain ASCII names.
pub fn is_preview_path(relative: &str) -> bool {
    let plain = |part: &str| part.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        && part.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
    let Some(rest) = relative.strip_prefix(PREVIEW_ROOT).and_then(|rest| rest.strip_prefix('/')) else { return false; };
    let parts = rest.split('/').collect::<Vec<_>>();
    relative.len() <= 512 && parts.len() == 2 && parts.iter().all(|part| plain(part)) && parts[1].ends_with(".png")
}

/// Resolves a reported preview path to its PNG file, which stays inside the work project.
pub fn preview_file(project_root: &Path, relative: &str) -> Result<PathBuf, AppErrorV1> {
    let invalid = || production_error("preview_invalid", format!("A preview path is outside the work project's preview folder: {relative}"));
    if !is_preview_path(relative) {
        return Err(invalid());
    }
    let root = fs::canonicalize(project_root).map_err(|e| production_error("project_missing", e.to_string()))?;
    let file = relative.split('/').fold(project_root.to_path_buf(), |path, part| path.join(part));
    let metadata = fs::symlink_metadata(&file).map_err(|e| production_error("preview_missing", format!("{relative}: {e}")))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_PREVIEW_BYTES {
        return Err(production_error("preview_unusable", format!("A preview is not a bounded image file: {relative}")));
    }
    let actual = fs::canonicalize(&file).map_err(|e| production_error("preview_missing", format!("{relative}: {e}")))?;
    if !actual.starts_with(&root) { return Err(invalid()); }
    Ok(actual)
}

/// Preview paths a run reports: the active Avatar's views and each candidate's image.
pub fn preview_paths(run: &ProductionRun) -> Vec<String> {
    let views = run.result["facts"]["previews"].as_array().into_iter().flatten()
        .filter_map(|view| view["path"].as_str().map(str::to_owned));
    views.chain(run.prefabs.iter().filter_map(|p| p.preview.clone())).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedSource {
    pub source_id: String,
    pub role: SourceRole,
    pub name: String,
    pub item_name: String,
    pub sha256: String,
    pub asset_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionStep {
    pub key: String,
    pub name: String,
    /// `running`, `succeeded`, `failed` or `stopped`.
    pub state: String,
    pub detail: String,
    /// When the step started and ended. Records written before step times carry neither.
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub ended_at: Option<String>,
}

/// SDK items one fix request may name.
pub const SDK_FIX_LIMIT: usize = 16;

/// A stage of a Unity batch run, read from the Editor's own log. The set is closed.
/// `Fingerprint` is VUA's own final stage: the project-file check after the Editor exits.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum UnityStage {
    Starting,
    License,
    Packages,
    Importing,
    Compiling,
    Bridge,
    Previews,
    Finishing,
    Fingerprint,
}

/// What a running Unity batch run shows: its stage, how many of the run's assets Unity has
/// imported, and when Unity last wrote to its log. It carries no log text and no paths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnityProgress {
    pub stage: UnityStage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    /// When Unity last wrote to its log; its start time until it writes.
    pub last_activity_at: String,
}

/// The running task as a platform's Unity step sees it: whether the player asked to stop, and
/// where the step's live progress goes.
pub struct UnityWatch<'a> {
    ctx: &'a TaskContext,
    engine: &'a AmfProductionEngine,
    run_id: String,
    step: ProductionStep,
    emitted: Mutex<Option<(UnityProgress, Instant)>>,
}

impl UnityWatch<'_> {
    /// True once the player asked to stop this task.
    pub fn stop_requested(&self) -> bool {
        self.ctx.check_cancel()
    }
    /// The typed result of a stop, for a Unity run that ended because the player asked.
    pub fn stopped(&self) -> AppErrorV1 {
        self.ctx.cancellation_error("amf-production").unwrap_or_else(|| {
            AppErrorV1::new(crate::runtime::error_codes::CANCELLED, ErrorCategory::Cancelled, "errors.task.cancelled", "amf-production")
                .with_recoverable(true)
        })
    }
    /// Keeps the Unity run's progress for `amf.production.get` and emits a task progress event
    /// when the stage changes, the import count moves (at most once a second) or Unity's last
    /// activity moves on (at most every 15 seconds).
    pub fn report(&self, progress: UnityProgress) {
        self.engine.live.lock().expect("AMF progress poisoned").insert(self.run_id.clone(), progress.clone());
        let mut emitted = self.emitted.lock().expect("AMF progress poisoned");
        let due = match emitted.as_ref() {
            None => true,
            Some((previous, at)) => previous.stage != progress.stage
                || ((previous.current, previous.total) != (progress.current, progress.total) && at.elapsed() >= Duration::from_secs(1))
                || (previous.last_activity_at != progress.last_activity_at && at.elapsed() >= Duration::from_secs(15)),
        };
        if due {
            let mut payload = step_progress(&self.run_id, &self.step);
            payload["unity"] = serde_json::to_value(&progress).expect("Unity progress serialization");
            self.ctx.emit_progress(payload);
            *emitted = Some((progress, Instant::now()));
        }
    }
    /// The key of the step this Unity run belongs to.
    pub fn step_key(&self) -> &str {
        &self.step.key
    }
}

impl Drop for UnityWatch<'_> {
    fn drop(&mut self) {
        if let Ok(mut live) = self.engine.live.lock() {
            live.remove(&self.run_id);
        }
    }
}

/// While VUA verifies the project files after a Unity run, that check shows as its own
/// stage: one task progress event and a live entry, which ends with the check.
struct FingerprintCheck<'a> {
    engine: &'a AmfProductionEngine,
    run_id: String,
}

impl<'a> FingerprintCheck<'a> {
    fn begin(engine: &'a AmfProductionEngine, run: &ProductionRun, ctx: &TaskContext) -> Self {
        let progress = UnityProgress {
            stage: UnityStage::Fingerprint,
            current: None,
            total: None,
            last_activity_at: SystemClock.now_rfc3339(),
        };
        engine
            .live
            .lock()
            .expect("AMF progress poisoned")
            .insert(run.run_id.clone(), progress.clone());
        if let Some(step) = run.steps.last().filter(|step| step.state == "running") {
            let mut payload = step_progress(&run.run_id, step);
            payload["unity"] =
                serde_json::to_value(&progress).expect("Unity progress serialization");
            ctx.emit_progress(payload);
        }
        Self {
            engine,
            run_id: run.run_id.clone(),
        }
    }
}

impl Drop for FingerprintCheck<'_> {
    fn drop(&mut self) {
        if let Ok(mut live) = self.engine.live.lock() {
            live.remove(&self.run_id);
        }
    }
}

/// A Unity change that a forced end interrupted: the run still reads as assembling, its last step
/// (assembly or an SDK fix) never finished, and the restore point taken right before it is kept.
/// The provider runs one task at a time and refuses restores while one runs, so a run in this
/// state when a restore is asked for is one VUA stopped mid-change.
fn interrupted_change(run: &ProductionRun) -> bool {
    run.phase == "assembling"
        && run.snapshot_id.is_some()
        && (run.steps.last().is_some_and(|step| matches!(step.key.as_str(), "assembly" | "sdk_fix" | "face_tracking" | "texture_memory") && step.state == "running")
            || run.steps.iter().rev().take_while(|step| step.key != "snapshot").any(|step| matches!(step.key.as_str(), "face_tracking" | "texture_memory")))
}

/// Whether the run assembled successfully before and Unity now observes the same Avatar and
/// outfits it assembled.
fn still_assembled(run: &ProductionRun) -> bool {
    let assembled = run.completed_at.is_some()
        || run.steps.iter().any(|step| step.key == "assembly" && step.state == "succeeded");
    let observed: Option<AssemblySelection> =
        serde_json::from_value(run.result["facts"]["observedSelection"].clone()).ok();
    match (assembled, run.selection.as_ref(), observed) {
        (true, Some(selection), Some(observed)) => {
            observed.avatar_prefab == selection.avatar_prefab
                && observed.outfits.len() == selection.outfits.len()
                && selection.outfits.iter().all(|outfit| observed.outfits.contains(outfit))
        }
        _ => false,
    }
}

/// The task progress payload of a step: `step` is the step key, `name` what it works on.
pub fn step_progress(run_id: &str, step: &ProductionStep) -> serde_json::Value {
    serde_json::json!({"operation":"amf.production","runId":run_id,"step":step.key,"name":step.name,
        "startedAt":step.started_at.clone().unwrap_or_default()})
}

/** A line's materials, kept with a restore point taken before they changed. */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSnapshot {
    pub choices: Vec<SourceChoice>,
    pub sources: Vec<PreparedSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionRun {
    pub schema_version: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub revision: u64,
    pub run_id: String,
    pub phase: String,
    pub task_id: String,
    pub editor: ProductionEditor,
    pub project_root: String,
    pub choices: Vec<SourceChoice>,
    pub sources: Vec<PreparedSource>,
    pub prefabs: Vec<PrefabCandidate>,
    pub project_fingerprint: String,
    pub unity_fingerprint: String,
    pub steps: Vec<ProductionStep>,
    pub selection: Option<AssemblySelection>,
    pub confirmed_digest: Option<String>,
    #[serde(default)]
    pub snapshot_id: Option<String>,
    #[serde(default)]
    pub snapshot_selection: Option<AssemblySelection>,
    /// The line's materials at the restore point, when that point was taken before they changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_sources: Option<SourceSnapshot>,
    /// The full copy taken before the creator patch, kept while the line carries its manual-replay
    /// marker so later restore points cannot make the pre-patch state unreachable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ft_pre_patch_snapshot: Option<String>,
    /// The line's version records, newest first, taken before each protected change.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revisions: Vec<ProductionRevision>,
    /// The SDK fixes applied to this line, named by the SDK's exact words, in application order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sdk_fixes_applied: Vec<String>,
    /// Adjustments a Recipe declares, written into the project after its first assembly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe_adjustments: Option<crate::amf_recipe::RecipeAdjustments>,
    /// The garment adjustments the last observation reported against the assembly baseline.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fit_changes: Vec<GarmentFitChanges>,
    /// The fits that produced the current assembled state, per garment source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied_fits: Vec<AppliedFit>,
    /// When the last observation was taken, with seven fractional digits so the editor save
    /// hook's round-trip stamps string-compare against it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
    #[serde(default)]
    pub recipe_id: Option<String>,
    #[serde(default)]
    pub package_pins: BTreeMap<String, String>,
    #[serde(default)]
    pub recipe_source_hashes: BTreeMap<String, String>,
    pub result: serde_json::Value,
    pub error: Option<String>,
    pub recovery: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_close: Option<EditorCloseResult>,
}

#[derive(Debug, Clone)]
pub struct ObservedProject {
    pub prefabs: Vec<PrefabCandidate>,
    pub unity_fingerprint: String,
    pub editor_version: String,
    pub result: serde_json::Value,
}

/// Implemented by the project-manager/Bridge adapter in the AMF composition.
/// No method may modify an external player project or the library input.
pub trait AmfProductionPlatform: Send + Sync {
    fn sources(&self) -> Result<Vec<ProductionSource>, AppErrorV1>;
    fn resolve_recipe_materials(&self, _: &[crate::amf_recipe::RecipeMaterial]) -> Result<BTreeMap<String, String>, AppErrorV1> {
        Err(production_error("recipe_unavailable", "Recipe input resolution is unavailable."))
    }
    fn recipe_source_candidates(&self, _: &[crate::amf_recipe::RecipeMaterial], _: &BTreeMap<String, Vec<String>>) -> Result<BTreeMap<String, Vec<String>>, AppErrorV1> { Ok(BTreeMap::new()) }
    fn editors(&self) -> Result<Vec<ProductionEditor>, AppErrorV1>;
    fn verify_editor(&self, path: &str) -> Result<ProductionEditor, AppErrorV1>;
    fn create_project(&self, run: &ProductionRun, ctx: &TaskContext) -> Result<(), AppErrorV1>;
    fn import_sources(
        &self,
        run: &ProductionRun,
        ctx: &TaskContext,
    ) -> Result<Vec<PreparedSource>, AppErrorV1>;
    /// Brings a prepared work project to the run's current choices: the files of materials no
    /// longer chosen leave, changed files are replaced, new ones arrive, and the AMF work scene
    /// starts fresh. `previous` are the materials the project holds now.
    fn replace_sources(
        &self,
        _run: &ProductionRun,
        _previous: &[PreparedSource],
        _ctx: &TaskContext,
    ) -> Result<Vec<PreparedSource>, AppErrorV1> {
        Err(production_error("change_unavailable", "Changing a line's materials is unavailable on this platform."))
    }
    /// Removes AMF's work scene and its object ownership, so the next inspection starts a
    /// fresh scene. Imported materials and packages stay.
    fn clear_work_scene(&self, _run: &ProductionRun) -> Result<(), AppErrorV1> {
        Err(production_error("rebuild_unavailable", "Rebuilding a line on another base is unavailable on this platform."))
    }
    /// Unity steps report their progress through `watch` and end Unity when it asks to stop.
    fn inspect(&self, run: &ProductionRun, watch: &UnityWatch) -> Result<ObservedProject, AppErrorV1>;
    /// Assembles the selection, applying each outfit's resolved fit objects when it has any.
    fn assemble(
        &self,
        run: &ProductionRun,
        selection: &AssemblySelection,
        fits: &[OutfitFit],
        watch: &UnityWatch,
    ) -> Result<ObservedProject, AppErrorV1>;
    fn apply_recipe(
        &self,
        _: &ProductionRun,
        _: &AssemblySelection,
        _: &crate::amf_recipe_apply::RecipeApplyTarget,
        _: &[OutfitFit],
        _: &UnityWatch,
    ) -> Result<ObservedProject, AppErrorV1> {
        Err(production_error("recipe_apply_unavailable", "Recipe application is unavailable on this platform."))
    }
    /// The work project's fingerprint as the AMF flow computes it (cached v2 algorithm).
    fn fingerprint(&self, project: &Path) -> Result<String, AppErrorV1>;
    /// The same project under the uncached v1 algorithm, kept only to compare run
    /// records that were written before the cached value existed.
    fn fingerprint_legacy(&self, project: &Path) -> Result<String, AppErrorV1>;
    /// Runs the VRChat SDK's own Auto Fix for the chosen items, named by the SDK's exact words.
    fn sdk_fix(
        &self,
        _run: &ProductionRun,
        _fixes: &[String],
        _watch: &UnityWatch,
    ) -> Result<ObservedProject, AppErrorV1> {
        Err(production_error("sdk_fix_unavailable", "SDK fixes are unavailable on this platform."))
    }
    fn snapshot(&self, run: &ProductionRun) -> Result<String, AppErrorV1>;
    fn texture_import(&self, _: &ProductionRun, _: &[TextureImportChange], _: &UnityWatch) -> Result<ObservedProject, AppErrorV1> {
        Err(production_error("texture_import_unavailable", "Texture import changes are unavailable on this platform."))
    }
    fn face_tracking_patch(&self, _: &ProductionRun, _: &FaceTrackingPatch, _: &UnityWatch) -> Result<ObservedProject, AppErrorV1> {
        Err(production_error("face_tracking_unavailable", "Creator patch sessions are unavailable on this platform."))
    }
    fn restore(&self, run: &ProductionRun, snapshot: &str) -> Result<(), AppErrorV1>;
    /// The current content identity (`sha256:<hex>`) of a library source, or None when the
    /// library no longer holds it. Restoring a version refuses sources it cannot verify.
    fn source_content_hash(&self, _source_id: &str) -> Result<Option<String>, AppErrorV1> {
        Ok(None)
    }
    /// Deletes every retained full-copy snapshot of the run's project except those named in
    /// `keep`. Best effort: problems come back as diagnostics and never fail the calling
    /// operation.
    fn prune_snapshots(&self, _run: &ProductionRun, _keep: &[&str]) -> Vec<String> {
        Vec::new()
    }
    fn open(&self, run: &ProductionRun, ctx: &TaskContext)
        -> Result<serde_json::Value, AppErrorV1>;
    /// Bakes the 60-frame turntable of the assembled Avatar into the work project.
    fn turntable(&self, _: &ProductionRun, _: &UnityWatch) -> Result<ObservedProject, AppErrorV1> {
        Err(production_error("turntable_unavailable", "Turntable baking is unavailable on this platform."))
    }
    /// Hands one build-and-upload to the official VRChat SDK in the open Editor and waits for its receipt.
    fn upload(&self, _: &ProductionRun, _: &UploadRequest, _: &TaskContext, _: &mut dyn FnMut(UploadProgress)) -> Result<UploadReceipt, AppErrorV1> {
        Err(production_error("upload_unavailable", "Managed upload is unavailable on this platform."))
    }
    /// Captures the on-disk state before a confirmed Editor save, without writing project files.
    fn snapshot_open_editor(&self, _: &ProductionRun) -> Result<String, AppErrorV1> {
        Err(production_error("editor_close_unavailable", "Saving and closing Unity is unavailable on this platform."))
    }
    fn close_editor(&self, _: &ProductionRun, _: &TaskContext) -> Result<EditorCloseResult, AppErrorV1> {
        Err(production_error("editor_close_unavailable", "Saving and closing Unity is unavailable on this platform."))
    }
    /// Whether a Unity Editor holds the work project now.
    fn editor_open(&self, _: &Path) -> bool { false }
}

pub struct AmfProductionEngine {
    pub(crate) root: PathBuf,
    pub(crate) platform: Arc<dyn AmfProductionPlatform>,
    /// Live progress of each run's Unity step, while it runs.
    live: Mutex<BTreeMap<String, UnityProgress>>,
    /// Usual step durations, with the signature of the run records they were read from.
    usual: Mutex<Option<(RecordSignature, BTreeMap<String, u64>)>>,
}

/// Name, length and modification time of each run record, in name order.
type RecordSignature = Vec<(std::ffi::OsString, u64, Option<SystemTime>)>;

/// Run phases during which a task works on the run.
pub const ACTIVE_PHASES: [&str; 4] = ["preparing", "inspecting", "assembling", "restoring"];

use crate::filesystem::remove_tree;

pub fn production_error(code: &str, detail: impl Into<String>) -> AppErrorV1 {
    let detail = detail.into();
    let mut error = AppErrorV1::new(
        format!("vua.amf_production.{code}"),
        ErrorCategory::Validation,
        "errors.amfProduction.failed",
        "amf-production",
    )
    .with_recoverable(true)
    .with_param("detail", ParamValue::Text(if code == "project_open" {
        format!("{detail} You can save and close Unity first.")
    } else { detail }));
    if code == "project_open" {
        error = error.with_param("suggestedAction", ParamValue::Text("amf.production.closeEditor".into()));
    }
    error
}

fn io_error(error: impl std::fmt::Display) -> AppErrorV1 {
    production_error("store_failed", error.to_string())
}

pub fn validate_choices(choices: &[SourceChoice]) -> Result<(), AppErrorV1> {
    if choices.is_empty()
        || choices.len() > 64
        || choices
            .iter()
            .filter(|s| s.role == SourceRole::Avatar)
            .count()
            > 1
        || (!choices.iter().any(|s| s.role == SourceRole::Auto)
            && (choices
                .iter()
                .filter(|s| s.role == SourceRole::Avatar)
                .count()
                != 1
                || !choices.iter().any(|s| s.role == SourceRole::Outfit)))
    {
        return Err(production_error(
            "sources_invalid",
            "Select one Avatar package and at least one outfit package.",
        ));
    }
    let mut ids = std::collections::HashSet::new();
    if choices
        .iter()
        .any(|s| s.source_id.is_empty() || !ids.insert(&s.source_id))
    {
        return Err(production_error(
            "sources_invalid",
            "Duplicate or empty source identity.",
        ));
    }
    Ok(())
}

/// One reviewed assembly execution: the selection, the reviewed digest, an optional Recipe
/// overlay target, whether the base is rebuilt, the Recipe adjustments to write afterwards
/// and the fits (with their provenance) the assembly applies per garment.
pub(crate) struct AssemblyExecution<'a> {
    pub selection: AssemblySelection,
    pub digest: &'a str,
    pub target: Option<crate::amf_recipe_apply::RecipeApplyTarget>,
    pub rebuild: bool,
    pub adjustments: Option<crate::amf_recipe::RecipeAdjustments>,
    pub fits: Vec<AppliedFit>,
}

impl AmfProductionEngine {
    pub fn new(root: PathBuf, platform: Arc<dyn AmfProductionPlatform>) -> Self {
        Self { root, platform, live: Mutex::new(BTreeMap::new()), usual: Mutex::new(None) }
    }

    /// Missing or unreadable settings retain the single-task default without rewriting the file.
    pub fn capacity(&self) -> usize {
        fs::read(self.root.join("settings.json")).ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .and_then(|value| value["capacity"].as_u64())
            .filter(|capacity| (1..=3).contains(capacity))
            .map_or(1, |capacity| capacity as usize)
    }

    /// The serving layer serializes settings changes with admission while its lane is idle.
    pub fn save_capacity(&self, capacity: usize) -> Result<(), AppErrorV1> {
        if !(1..=3).contains(&capacity) {
            return Err(production_error("params_invalid", "Capacity must be 1, 2 or 3."));
        }
        fs::create_dir_all(&self.root).map_err(io_error)?;
        let path = self.root.join("settings.json");
        let pending = path.with_extension("pending");
        let bytes = serde_json::to_vec_pretty(&serde_json::json!({"capacity":capacity})).map_err(io_error)?;
        use std::io::Write;
        let mut file = fs::File::create(&pending).map_err(io_error)?;
        file.write_all(&bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        drop(file);
        fs::rename(pending, path).map_err(io_error)
    }
    fn watch<'a>(&'a self, run: &ProductionRun, ctx: &'a TaskContext) -> UnityWatch<'a> {
        UnityWatch {
            ctx,
            engine: self,
            run_id: run.run_id.clone(),
            step: run.steps.last().cloned().expect("a Unity step runs inside a recorded step"),
            emitted: Mutex::new(None),
        }
    }
    /// The median duration in whole seconds of each step kind that finished on this computer,
    /// read from the retained runs.
    pub fn usual_durations(&self) -> BTreeMap<String, u64> {
        let signature = self.record_signature();
        let mut cache = self.usual.lock().expect("AMF durations poisoned");
        if let Some((read_from, durations)) = cache.as_ref() {
            if *read_from == signature {
                return durations.clone();
            }
        }
        let mut samples: BTreeMap<String, Vec<i64>> = BTreeMap::new();
        for run in self.retained_runs() {
            for step in run.steps.iter().filter(|step| step.state == "succeeded") {
                let times = step.started_at.as_deref().and_then(crate::rfc3339_millis)
                    .zip(step.ended_at.as_deref().and_then(crate::rfc3339_millis));
                if let Some((start, end)) = times.filter(|(start, end)| end >= start) {
                    samples.entry(step.key.clone()).or_default().push(end - start);
                }
            }
        }
        let durations = samples.into_iter().map(|(key, mut values)| {
            values.sort_unstable();
            let middle = values.len() / 2;
            let median = if values.len().is_multiple_of(2) { (values[middle - 1] + values[middle]) / 2 } else { values[middle] };
            (key, ((median + 500) / 1000) as u64)
        }).collect::<BTreeMap<_, _>>();
        *cache = Some((signature, durations.clone()));
        durations
    }
    /// Each run record's name, length and modification time. A record written within the same
    /// timestamp tick still changes its name or length.
    fn record_signature(&self) -> RecordSignature {
        let Ok(entries) = fs::read_dir(self.root.join("runs")) else { return Vec::new(); };
        let mut signature = entries.flatten()
            .filter(|entry| entry.path().extension().and_then(|p| p.to_str()) == Some("json"))
            .filter_map(|entry| entry.metadata().ok().map(|m| (entry.file_name(), m.len(), m.modified().ok())))
            .collect::<RecordSignature>();
        signature.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        signature
    }
    /// The most recent retained runs; unreadable records are skipped.
    fn retained_runs(&self) -> Vec<ProductionRun> {
        let Ok(entries) = fs::read_dir(self.root.join("runs")) else { return Vec::new(); };
        let mut ids = entries.flatten().filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|p| p.to_str()) == Some("json"))
                .then(|| path.file_stem().and_then(|p| p.to_str()).map(str::to_owned))?
        }).collect::<Vec<_>>();
        ids.sort_unstable_by(|a, b| b.cmp(a));
        ids.into_iter().take(30).filter_map(|id| self.get(&id).ok()).collect()
    }
    /// The running step of an active run: when it started, how long it has run, its usual
    /// duration on this computer and, inside Unity, the live Unity progress.
    pub fn progress(&self, run: &ProductionRun) -> Option<serde_json::Value> {
        let step = run.steps.last().filter(|step| step.state == "running")?;
        let started = step.started_at.as_deref().and_then(crate::rfc3339_millis)?;
        let now = crate::rfc3339_millis(&SystemClock.now_rfc3339())?;
        let mut view = step_progress(&run.run_id, step);
        let object = view.as_object_mut().expect("progress object");
        object.remove("operation");
        object.remove("runId");
        object.insert("elapsedSeconds".into(), ((now - started).max(0) / 1000).into());
        object.insert("usualSeconds".into(), self.usual_durations().get(&step.key).copied().into());
        if let Some(unity) = self.live.lock().expect("AMF progress poisoned").get(&run.run_id) {
            object.insert("unity".into(), serde_json::to_value(unity).expect("Unity progress serialization"));
        }
        Some(view)
    }
    pub fn sources(&self) -> Result<Vec<ProductionSource>, AppErrorV1> {
        self.platform.sources()
    }
    pub fn editors(&self) -> Result<Vec<ProductionEditor>, AppErrorV1> {
        self.platform.editors()
    }

    fn run_file(&self, run_id: &str) -> Result<PathBuf, AppErrorV1> {
        if run_id.is_empty()
            || run_id.len() > 100
            || !run_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err(production_error(
                "run_invalid",
                "Invalid production run identity.",
            ));
        }
        Ok(self.root.join("runs").join(format!("{run_id}.json")))
    }
    fn save(&self, run: &ProductionRun) -> Result<(), AppErrorV1> {
        let path = self.run_file(&run.run_id)?;
        fs::create_dir_all(path.parent().expect("run parent")).map_err(io_error)?;
        let pending = path.with_extension("pending");
        fs::write(&pending, serde_json::to_vec_pretty(run).map_err(io_error)?).map_err(io_error)?;
        fs::rename(&pending, path).map_err(io_error)?;
        // A saved step can change the medians; the next read computes them again.
        *self.usual.lock().expect("AMF durations poisoned") = None;
        Ok(())
    }
    pub fn get(&self, run_id: &str) -> Result<ProductionRun, AppErrorV1> {
        let path = self.run_file(run_id)?;
        let bytes = fs::read(path).map_err(|e| if e.kind() == std::io::ErrorKind::NotFound {
            production_error("run_not_found", "There is no production line with this ID. List the current lines and use one of their IDs.")
        } else { io_error(e) })?;
        let run: ProductionRun = serde_json::from_slice(&bytes).map_err(io_error)?;
        let expected = self.root.join("projects").join(run_id);
        if run.run_id != run_id || Path::new(&run.project_root) != expected {
            return Err(production_error(
                "run_invalid",
                "Run project binding changed.",
            ));
        }
        // Records written before version records existed read with one synthesized `assembly`
        // version built from their retained restore point. Nothing is discarded.
        let mut run = run;
        if let Some(legacy) = legacy_revision(&run) {
            run.revisions.push(legacy);
        }
        Ok(run)
    }
    pub fn list(&self) -> Result<Vec<ProductionRun>, AppErrorV1> {
        let directory = self.root.join("runs");
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let mut runs = Vec::new();
        for entry in fs::read_dir(directory).map_err(io_error)? {
            let path = entry.map_err(io_error)?.path();
            if path.extension().and_then(|p| p.to_str()) != Some("json") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|p| p.to_str()) else {
                continue;
            };
            runs.push(self.get(id)?);
        }
        runs.sort_by(|a, b| b.run_id.cmp(&a.run_id));
        runs.truncate(30);
        Ok(runs)
    }
    pub(crate) fn recipe_store(&self) -> crate::RecipeDocumentStore {
        crate::RecipeDocumentStore::new_with_system_clock(self.root.join("recipes-v04"))
    }
    pub fn save_recipe(&self, run_id: &str, recipe_id: &str, title: &str) -> Result<crate::StoredRecipeDocument, AppErrorV1> {
        let mut run = self.get(run_id)?;
        face_tracking::require_replayable_face_tracking(&run)?;
        if !matches!(run.phase.as_str(), "ready" | "succeeded") {
            return Err(production_error("inspect_required", "Reinspect current project changes before saving a Recipe."));
        }
        let (unchanged, current) = self.project_state(&run)?;
        if !unchanged {
            return Err(production_error("inspect_required", "Reinspect current project changes before saving a Recipe."));
        }
        if run.project_fingerprint != current {
            // A record that still carries a v1 value upgrades to the current value on this match.
            run.project_fingerprint = current;
            self.save(&run)?;
        }
        let recipe = crate::amf_recipe::ProductionRecipe::from_run(&run, recipe_id, title, current_adjustments(&run))?;
        if self.recipe_store().get(recipe_id).map_err(io_error)?.is_some() { return Err(production_error("recipe_exists", "Save a revised Recipe under a new identity.")); }
        self.retain_recipe_origin(&run, &recipe)?;
        self.recipe_store().save(recipe_id, &serde_json::to_value(recipe).map_err(io_error)?, 0).map_err(io_error)
    }
    pub fn list_recipes(&self) -> Result<Vec<crate::RecipeListEntry>, AppErrorV1> { self.recipe_store().list().map_err(io_error) }
    pub fn get_recipe(&self, recipe_id: &str) -> Result<crate::StoredRecipeDocument, AppErrorV1> {
        let stored = self.recipe_store().get(recipe_id).map_err(io_error)?
            .ok_or_else(|| production_error("recipe_missing", "The saved Recipe is no longer available."))?;
        let recipe: crate::amf_recipe::ProductionRecipe = serde_json::from_value(stored.recipe.clone()).map_err(io_error)?;
        recipe.validate()?;
        if recipe.recipe_id != recipe_id { return Err(production_error("recipe_invalid", "Recipe identity changed.")); }
        Ok(stored)
    }
    pub fn begin_recipe(&self, run_id: &str, recipe_id: &str, editor_path: &str) -> Result<(), AppErrorV1> {
        let recipe: crate::amf_recipe::ProductionRecipe = serde_json::from_value(self.get_recipe(recipe_id)?.recipe).map_err(io_error)?;
        let editor = self.platform.verify_editor(editor_path)?;
        if editor.version != recipe.editor_version { return Err(production_error("recipe_editor_mismatch", format!("This Recipe was made with Unity {}. Select that exact Editor to reproduce it.", recipe.editor_version))); }
        let resolved = self.platform.resolve_recipe_materials(&recipe.materials)?;
        let selection = recipe.selection(&resolved)?;
        let choices = recipe.materials.iter().map(|material| SourceChoice {
            source_id: resolved[&material.id].clone(), role: if recipe.avatar.material_id == material.id { SourceRole::Avatar }
                else if recipe.outfits.iter().any(|outfit| outfit.material_id == material.id) { SourceRole::Outfit } else { SourceRole::Dependency },
        }).collect();
        self.begin(run_id, choices, editor_path)?;
        let mut run = self.get(run_id)?;
        run.recipe_source_hashes = recipe.materials.iter().map(|material| (resolved[&material.id].clone(), material.sha256.clone())).collect();
        run.recipe_id = Some(recipe_id.into()); run.package_pins = recipe.packages; run.selection = Some(selection);
        run.recipe_adjustments = recipe.adjustments.clone();
        self.save(&run)
    }
    pub fn begin(
        &self,
        run_id: &str,
        choices: Vec<SourceChoice>,
        editor_path: &str,
    ) -> Result<(), AppErrorV1> {
        validate_choices(&choices)?;
        if self.run_file(run_id)?.exists() {
            return Err(production_error("run_exists", "Run already exists."));
        }
        let sources = self.sources()?;
        for choice in &choices {
            if !sources.iter().any(|s| s.source_id == choice.source_id) {
                return Err(production_error(
                    "source_missing",
                    format!("Source {} is no longer present.", choice.source_id),
                ));
            }
        }
        let editor = self.platform.verify_editor(editor_path)?;
        self.save(&ProductionRun {
            schema_version: "0.1".into(),
            created_at: SystemClock.now_rfc3339(),
            completed_at: None,
            revision: 0,
            run_id: run_id.into(),
            phase: "preparing".into(),
            task_id: String::new(),
            editor,
            project_root: self
                .root
                .join("projects")
                .join(run_id)
                .to_string_lossy()
                .into_owned(),
            choices,
            sources: Vec::new(),
            prefabs: Vec::new(),
            project_fingerprint: String::new(),
            unity_fingerprint: String::new(),
            steps: Vec::new(),
            selection: None,
            confirmed_digest: None,
            snapshot_id: None,
            snapshot_selection: None,
            snapshot_sources: None,
            ft_pre_patch_snapshot: None,
            revisions: Vec::new(),
            sdk_fixes_applied: Vec::new(),
            recipe_adjustments: None,
            fit_changes: Vec::new(),
            applied_fits: Vec::new(),
            observed_at: None,
            recipe_id: None,
            package_pins: BTreeMap::new(),
            recipe_source_hashes: BTreeMap::new(),
            result: serde_json::Value::Null,
            error: None,
            recovery: None,
            editor_close: None,
        })
    }
    fn step(
        &self,
        run: &mut ProductionRun,
        ctx: &TaskContext,
        key: &str,
        name: &str,
    ) -> Result<(), AppErrorV1> {
        if let Some(error) = ctx.cancellation_error("amf-production") {
            return Err(error);
        }
        let now = SystemClock.now_rfc3339();
        if let Some(previous) = run.steps.last_mut() {
            if previous.state == "running" {
                previous.state = "succeeded".into();
                previous.ended_at = Some(now.clone());
            }
        }
        run.steps.push(ProductionStep {
            key: key.into(),
            name: name.into(),
            state: "running".into(),
            detail: String::new(),
            started_at: Some(now),
            ended_at: None,
        });
        self.save(run)?;
        ctx.emit_progress(step_progress(&run.run_id, run.steps.last().expect("step just recorded")));
        Ok(())
    }
    /// Ends the running step with `state`.
    fn close_step(run: &mut ProductionRun, state: &str) {
        if let Some(step) = run.steps.last_mut().filter(|step| step.state == "running") {
            step.state = state.into();
            step.ended_at = Some(SystemClock.now_rfc3339());
        }
    }
    fn finish(
        &self,
        run: &mut ProductionRun,
        result: Result<(), AppErrorV1>,
    ) -> Result<(), AppErrorV1> {
        match result {
            Ok(()) => {
                Self::close_step(run, "succeeded");
                self.save(run)
            }
            // A stop keeps the work project and its imported materials; reinspection continues
            // from there. Before the materials are in place, preparation starts again instead.
            Err(error) if error.category == ErrorCategory::Cancelled => {
                Self::close_step(run, "stopped");
                if run.sources.is_empty() {
                    run.phase = "cancelled".into();
                    run.error = Some(error.code.clone());
                } else {
                    run.phase = "stopped".into();
                    run.error = None;
                }
                self.save(run)?;
                Err(error)
            }
            Err(error) => {
                let detail = error
                    .params
                    .as_ref()
                    .map(|params| {
                        params
                            .iter()
                            .filter_map(|(key, value)| match value {
                                ParamValue::Text(text) => Some(format!("{key}: {text}")),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("; ")
                    })
                    .filter(|s| !s.is_empty())
                    .map(|detail| format!("{} — {detail}", error.code))
                    .unwrap_or_else(|| error.code.clone());
                Self::close_step(run, "failed");
                if let Some(step) = run.steps.last_mut() {
                    step.state = "failed".into();
                    step.detail = detail.clone();
                }
                run.phase = "failed".into();
                run.error = Some(detail);
                self.save(run)?;
                Err(error)
            }
        }
    }
    /// Keeps the run's named full-copy snapshots per line: the current restore point and, while
    /// the line carries the creator-patch marker, the pre-patch copy. Once the run record naming
    /// them is persisted, every unreferenced snapshot directory under `.vua/snapshots/` is
    /// deleted. Pruning is best-effort; problems are recorded in a bounded diagnostics file and
    /// never fail the operation they follow.
    fn prune_snapshots(&self, run: &ProductionRun) {
        let mut keep: Vec<&str> = Vec::new();
        if let Some(id) = run.snapshot_id.as_deref() {
            keep.push(id);
        }
        // The pre-patch copy is kept only while the line still carries the creator-patch marker.
        if let Some(id) = run.ft_pre_patch_snapshot.as_deref().filter(|_| face_tracking::manual_face_tracking(run)) {
            keep.push(id);
        }
        if keep.is_empty() {
            return;
        }
        let diagnostics = self.platform.prune_snapshots(run, &keep);
        if diagnostics.is_empty() {
            return;
        }
        let path = self.root.join("snapshot-prune.json");
        let _ = (|| -> Result<(), AppErrorV1> {
            let mut entries: Vec<serde_json::Value> = fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or_default();
            for diagnostic in diagnostics {
                entries.push(serde_json::json!({
                    "at": SystemClock.now_rfc3339(),
                    "runId": run.run_id,
                    "diagnostic": diagnostic,
                }));
            }
            let len = entries.len();
            if len > 50 {
                entries.drain(..len - 50);
            }
            let pending = path.with_extension("pending");
            fs::write(&pending, serde_json::to_vec_pretty(&entries).map_err(io_error)?).map_err(io_error)?;
            fs::rename(&pending, &path).map_err(io_error)
        })();
    }
    pub fn prepare(&self, run_id: &str, ctx: &TaskContext) -> Result<(), AppErrorV1> {
        let mut run = self.get(run_id)?;
        run.task_id = ctx.task_id().into();
        let result = (|| {
            self.step(&mut run, ctx, "project", "Prepare a managed work project")?;
            self.platform.create_project(&run, ctx)?;
            self.step(&mut run, ctx, "materials", "Import the selected materials")?;
            run.sources = self.platform.import_sources(&run, ctx)?;
            for (source_id, expected) in &run.recipe_source_hashes {
                if !run.sources.iter().any(|source| source.source_id == *source_id && source.sha256.strip_prefix("sha256:").unwrap_or(&source.sha256) == expected) {
                    return Err(production_error("recipe_material_changed", "A resolved Recipe input changed before import. Restore the required original and retry."));
                }
            }
            self.save(&run)?;
            self.step(
                &mut run,
                ctx,
                "inspect",
                "Identify actual Avatar and outfit prefabs",
            )?;
            self.update_observations(&mut run, ctx)?;
            if let Some(error) = ctx.cancellation_error("amf-production") {
                return Err(error);
            }
            run.phase = "ready".into();
            if run.recipe_id.is_some() {
                for (id, version) in &run.package_pins {
                    if run.result["packageVersions"][id].as_str() != Some(version.as_str()) {
                        return Err(production_error("recipe_dependency_mismatch", format!("Recipe requires {id} {version}; the prepared dependency differs.")));
                    }
                }
                if let Some(selection) = &run.selection { validate_selection(&run, selection)?; }
            }
            Ok(())
        })();
        self.finish(&mut run, result)
    }
    fn update_observations(&self, run: &mut ProductionRun, ctx: &TaskContext) -> Result<(), AppErrorV1> {
        let observed = self.platform.inspect(run, &self.watch(run, ctx))?;
        if observed.editor_version != run.editor.version {
            return Err(production_error(
                "editor_drift",
                "Running Editor identity differs from the selected Editor.",
            ));
        }
        run.prefabs = observed.prefabs;
        run.unity_fingerprint = observed.unity_fingerprint;
        run.result = observed.result;
        run.fit_changes = serde_json::from_value(run.result["facts"]["fitChanges"].clone()).unwrap_or_default();
        run.observed_at = Some(fit_now());
        let _check = FingerprintCheck::begin(self, run, ctx);
        run.project_fingerprint = self.platform.fingerprint(Path::new(&run.project_root))?;
        run.revision = run
            .revision
            .checked_add(1)
            .ok_or_else(|| production_error("revision_invalid", "Run revision exhausted."))?;
        Ok(())
    }
    /// Whether the work project still matches the fingerprint a run record carries, and
    /// the current (cached v2) value. A record written before the cached algorithm holds
    /// a `sha256:` v1 value; that compares by computing v1 once, so a new algorithm never
    /// reads as a changed project.
    pub(crate) fn project_state(&self, run: &ProductionRun) -> Result<(bool, String), AppErrorV1> {
        let project = Path::new(&run.project_root);
        let current = self.platform.fingerprint(project)?;
        if run.project_fingerprint == current {
            return Ok((true, current));
        }
        if run.project_fingerprint.starts_with(crate::FINGERPRINT_V1_PREFIX) {
            let legacy = self.platform.fingerprint_legacy(project)?;
            if legacy == run.project_fingerprint {
                return Ok((true, current));
            }
        }
        Ok((false, current))
    }
    /// The drift gate of every mutation and save: the project must match the record.
    /// A matching legacy record upgrades to the current value in place.
    fn ensure_project_matches(&self, run: &mut ProductionRun) -> Result<(), AppErrorV1> {
        let (matches, current) = self.project_state(run)?;
        if !matches {
            return Err(production_error(
                "project_drift",
                "The project changed. Reinspect it before assembly.",
            ));
        }
        run.project_fingerprint = current;
        Ok(())
    }
    /// The line's own materials change: VUA protects the work project, brings it to the new
    /// materials and builds a fresh work scene, and the line returns to planning. The restore
    /// point keeps the previous project and materials.
    pub fn change_sources(&self, run_id: &str, choices: Vec<SourceChoice>, ctx: &TaskContext) -> Result<(), AppErrorV1> {
        validate_choices(&choices)?;
        let mut run = self.get(run_id)?;
        face_tracking::require_replayable_face_tracking(&run)?;
        if run.sources.is_empty() || !matches!(run.phase.as_str(), "ready" | "succeeded" | "failed" | "stopped") {
            return Err(production_error("inspect_required", "Check the work project again before changing its materials."));
        }
        let sources = self.sources()?;
        if let Some(missing) = choices.iter().find(|choice| !sources.iter().any(|s| s.source_id == choice.source_id)) {
            return Err(production_error("source_missing", format!("Source {} is no longer present.", missing.source_id)));
        }
        let key = |choices: &[SourceChoice]| { let mut ids = choices.iter().map(|c| format!("{}:{:?}", c.source_id, c.role)).collect::<Vec<_>>(); ids.sort(); ids };
        if key(&choices) == key(&run.choices) {
            return Err(production_error("sources_unchanged", "Choose a different set of materials for this line."));
        }
        self.ensure_project_matches(&mut run)?;
        let previous = SourceSnapshot { choices: run.choices.clone(), sources: run.sources.clone() };
        run.task_id = ctx.task_id().into();
        run.phase = "preparing".into();
        run.error = None;
        run.recovery = None;
        run.confirmed_digest = None;
        let mut snapshot = None;
        let mut revision_id = None;
        let result = (|| {
            self.step(&mut run, ctx, "snapshot", "Protect the work project")?;
            snapshot = Some(self.platform.snapshot(&run)?);
            run.snapshot_id = snapshot.clone();
            run.snapshot_selection = run.selection.clone();
            run.snapshot_sources = Some(previous.clone());
            revision_id = Some(self.record_revision(&mut run, RevisionKind::Materials, false));
            self.save(&run)?;
            self.step(&mut run, ctx, "materials", "Change the line's materials")?;
            run.choices = choices.clone();
            run.sources = self.platform.replace_sources(&run, &previous.sources, ctx)?;
            self.save(&run)?;
            self.step(&mut run, ctx, "inspect", "Identify actual Avatar and outfit prefabs")?;
            self.update_observations(&mut run, ctx)?;
            if let Some(error) = ctx.cancellation_error("amf-production") {
                return Err(error);
            }
            // The line plans again; what still names a present material stays chosen.
            run.selection = run.selection.take().map(|mut selection| {
                selection.outfits.retain(|outfit| run.sources.iter().any(|s| s.source_id == outfit.source_id));
                if !run.prefabs.iter().any(|p| p.is_avatar && p.path == selection.avatar_prefab) {
                    selection.avatar_prefab.clear();
                }
                selection
            }).filter(|selection| !selection.avatar_prefab.is_empty() || !selection.outfits.is_empty());
            run.recipe_id = None;
            run.recipe_source_hashes.clear();
            // The fresh work scene holds no applied fits anymore.
            run.applied_fits.clear();
            run.completed_at = None;
            run.phase = "ready".into();
            Ok(())
        })();
        if result.is_err() {
            if let Some(snapshot) = snapshot {
                run.recovery = Some(match self.platform.restore(&run, &snapshot) {
                    Ok(()) => {
                        run.choices = previous.choices.clone();
                        run.sources = previous.sources.clone();
                        "restored".into()
                    }
                    Err(error) => format!("restore_failed: {}", error.code),
                });
            }
            // A change whose automatic restore brought the project back leaves no state
            // behind worth a version record.
            if !matches!(run.recovery.as_deref(), Some(recovery) if recovery.starts_with("restore_failed")) {
                if let Some(id) = &revision_id {
                    run.revisions.retain(|revision| revision.id != *id);
                }
            }
        }
        let outcome = self.finish(&mut run, result);
        self.prune_snapshots(&run);
        outcome
    }
    /// Deletes a line: its work project with the restore points, previews and turntable inside it,
    /// its release record and its run record. Recipes saved from it and Avatars uploaded from it stay.
    pub fn delete_run(&self, run_id: &str) -> Result<(), AppErrorV1> {
        let run = self.get(run_id)?;
        let project = PathBuf::from(&run.project_root);
        if self.platform.editor_open(&project) {
            return Err(production_error("project_open", "Close Unity for this line, then delete it."));
        }
        if project.exists() {
            remove_tree(&project).map_err(|e| production_error("delete_failed", e.to_string()))?;
        }
        let release = self.root.join("releases").join(format!("{run_id}.json"));
        if release.exists() {
            fs::remove_file(release).map_err(io_error)?;
        }
        // Assembly records are named `<runId>-<digest>.json`; the line's own leave with it.
        if let Ok(entries) = fs::read_dir(self.root.join("records")) {
            let prefix = format!("{run_id}-");
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_str().is_some_and(|name| name.starts_with(&prefix)) && entry.path().is_file() {
                    fs::remove_file(entry.path()).map_err(|e| production_error("delete_failed", e.to_string()))?;
                }
            }
        }
        fs::remove_file(self.run_file(run_id)?).map_err(io_error)?;
        *self.usual.lock().expect("AMF durations poisoned") = None;
        Ok(())
    }
    pub fn reinspect(&self, run_id: &str, ctx: &TaskContext) -> Result<(), AppErrorV1> {
        let mut run = self.get(run_id)?;
        if run.sources.is_empty() {
            return Err(production_error(
                "prepare_incomplete",
                "Preparation did not finish. Start a new preparation with the retained selections.",
            ));
        }
        run.task_id = ctx.task_id().into();
        run.error = None;
        // A stopped run continues here: Unity reinspects the retained project and its materials.
        run.phase = "inspecting".into();
        let result = (|| {
            self.step(
                &mut run,
                ctx,
                "reinspect",
                "Reinspect the retained work project",
            )?;
            self.update_observations(&mut run, ctx)?;
            // A finished Avatar stays finished while Unity still observes what was assembled;
            // a changed scene goes back to planning.
            run.phase = if still_assembled(&run) { "succeeded" } else { "ready" }.into();
            run.confirmed_digest = None;
            Ok(())
        })();
        self.finish(&mut run, result)
    }
    /// Runs the SDK's own Auto Fix for items its Avatar check offers one for. The fix changes
    /// the work scene and assets, so a restore point comes first and is restored on failure;
    /// the run keeps its phase and reads the SDK's check again.
    pub fn sdk_fix(&self, run_id: &str, fixes: Vec<String>, ctx: &TaskContext) -> Result<(), AppErrorV1> {
        let mut run = self.get(run_id)?;
        if !matches!(run.phase.as_str(), "ready" | "succeeded") {
            return Err(production_error("inspect_required", "Check the work project again before fixing it with the SDK."));
        }
        let offered: Vec<&str> = run.result["facts"]["sdkReview"]["issues"].as_array().into_iter().flatten()
            .filter(|issue| issue["canFix"].as_bool() == Some(true))
            .filter_map(|issue| issue["text"].as_str())
            .collect();
        let unique: std::collections::BTreeSet<&String> = fixes.iter().collect();
        if fixes.is_empty() || fixes.len() > SDK_FIX_LIMIT || unique.len() != fixes.len()
            || fixes.iter().any(|fix| !offered.contains(&fix.as_str())) {
            return Err(production_error("sdk_fix_unavailable", "Choose items the SDK's last check offers a fix for. Check the work project again if it changed."));
        }
        self.ensure_project_matches(&mut run)?;
        let phase = run.phase.clone();
        run.task_id = ctx.task_id().into();
        run.phase = "assembling".into();
        run.error = None;
        run.recovery = None;
        let mut snapshot = None;
        let mut revision_id = None;
        let result = (|| {
            self.step(&mut run, ctx, "snapshot", "Protect the work project")?;
            snapshot = Some(self.platform.snapshot(&run)?);
            run.snapshot_id = snapshot.clone();
            run.snapshot_selection = run.selection.clone();
            run.snapshot_sources = None;
            revision_id = Some(self.record_revision(&mut run, RevisionKind::SdkFix, false));
            self.save(&run)?;
            self.step(&mut run, ctx, "sdk_fix", "Run the VRChat SDK's Auto Fix")?;
            self.run_sdk_fixes(&mut run, &fixes, ctx)?;
            run.phase = phase.clone();
            run.confirmed_digest = None;
            Ok(())
        })();
        if result.is_err() {
            if let Some(snapshot) = snapshot {
                run.recovery = Some(match self.platform.restore(&run, &snapshot) {
                    Ok(()) => "restored".into(),
                    Err(error) => format!("restore_failed: {}", error.code),
                });
            }
            // A change whose automatic restore brought the project back leaves no state
            // behind worth a version record.
            if !matches!(run.recovery.as_deref(), Some(recovery) if recovery.starts_with("restore_failed")) {
                if let Some(id) = &revision_id {
                    run.revisions.retain(|revision| revision.id != *id);
                }
            }
        }
        let outcome = self.finish(&mut run, result);
        self.prune_snapshots(&run);
        outcome
    }
    /// Runs the SDK's Auto Fix inside an already-protected step and takes over the observed
    /// facts. Applied items accumulate on the run so version records can replay them.
    pub(crate) fn run_sdk_fixes(
        &self,
        run: &mut ProductionRun,
        fixes: &[String],
        ctx: &TaskContext,
    ) -> Result<(), AppErrorV1> {
        let observed = self.platform.sdk_fix(run, fixes, &self.watch(run, ctx))?;
        if let Some(error) = ctx.cancellation_error("amf-production") {
            return Err(error);
        }
        if observed.editor_version != run.editor.version {
            return Err(production_error("editor_drift", "The actual Editor version changed."));
        }
        let applied: Vec<String> =
            serde_json::from_value(observed.result["facts"]["sdkFixesApplied"].clone())
                .unwrap_or_default();
        run.prefabs = observed.prefabs;
        run.result = observed.result;
        run.unity_fingerprint = observed.unity_fingerprint;
        for fix in applied {
            if !run.sdk_fixes_applied.contains(&fix) {
                run.sdk_fixes_applied.push(fix);
            }
        }
        let _check = FingerprintCheck::begin(self, run, ctx);
        run.project_fingerprint = self.platform.fingerprint(Path::new(&run.project_root))?;
        run.revision = run.revision.checked_add(1)
            .ok_or_else(|| production_error("revision_invalid", "Run revision exhausted."))?;
        Ok(())
    }
    pub fn plan(
        &self,
        run_id: &str,
        selection: &AssemblySelection,
    ) -> Result<serde_json::Value, AppErrorV1> {
        let run = self.get(run_id)?;
        face_tracking::require_replayable_face_tracking(&run)?;
        let mut normalized = selection.clone();
        for outfit in &mut normalized.outfits {
            if outfit.toggle_name.trim().is_empty() { outfit.toggle_name.clear(); }
        }
        let selection = &normalized;
        validate_selection(&run, selection)?;
        let recipe_fits = run.recipe_adjustments.as_ref().map(|adjustments| adjustments.fits.as_slice()).unwrap_or(&[]);
        let fits = self.planned_fits(&run, selection, recipe_fits);
        let digest = plan_digest_with_fits(&run, selection, &fits)?;
        let inputs = run
            .sources
            .iter()
            .map(|source| {
                let role =
                    if run.prefabs.iter().any(|p| {
                        p.source_id == source.source_id && p.path == selection.avatar_prefab
                    }) {
                        "avatar"
                    } else if selection
                        .outfits
                        .iter()
                        .any(|o| o.source_id == source.source_id)
                    {
                        "outfit"
                    } else {
                        "import_only"
                    };
                serde_json::json!({"sourceId":source.source_id,"name":source.item_name,"role":role})
            })
            .collect::<Vec<_>>();
        Ok(
            serde_json::json!({"schemaVersion":"0.1", "runId":run.run_id, "planDigest":digest,"inputs":inputs,
            "selection":selection,"editor":run.editor,"projectRoot":run.project_root,
            "rebuild":rebuilds_base(&run, selection),"preserveOriginals":true,"officialSdkChecks":"not_run",
            "adjustments":run.recipe_adjustments.as_ref().map(|adjustments| serde_json::json!({
                "lighting":adjustments.lighting,"sdkFixes":adjustments.sdk_fixes})),
            "fits":fits.iter().map(|fit| serde_json::json!({"sourceId":fit.source_id,"fitId":fit.fit_id,"name":fit.name})).collect::<Vec<_>>(),
            "parameterBudget":plan_parameter_budget(&run, selection)}),
        )
    }
    pub fn assemble(
        &self,
        run_id: &str,
        selection: AssemblySelection,
        digest: &str,
        ctx: &TaskContext,
    ) -> Result<(), AppErrorV1> {
        let mut run = self.get(run_id)?;
        validate_selection(&run, &selection)?;
        // The reviewed digest bound the fits resolved at plan time; resolve them again and
        // let the digest comparison prove they still match.
        let recipe_fits = run.recipe_adjustments.as_ref().map(|adjustments| adjustments.fits.as_slice()).unwrap_or(&[]);
        let fits = self.planned_fits(&run, &selection, recipe_fits);
        if plan_digest_with_fits(&run, &selection, &fits)? != digest {
            return Err(production_error(
                "plan_drift",
                "The reviewed plan changed. Review it again.",
            ));
        }
        self.ensure_project_matches(&mut run)?;
        let rebuild = rebuilds_base(&run, &selection);
        let observed: Option<AssemblySelection> = serde_json::from_value(run.result["facts"]["observedSelection"].clone()).ok();
        let target = if let Some(current) = observed.filter(|_| !rebuild) {
            let id = run.result["facts"]["avatarGlobalObjectId"].as_str().filter(|id| !id.is_empty())
                .ok_or_else(|| production_error("inspect_required", "Reinspect the current Avatar identity before editing."))?;
            Some(crate::amf_recipe_apply::RecipeApplyTarget {
                avatar_global_object_id: id.into(), new_avatar_key: None,
                write_source_ids: selection.outfits.iter().filter(|outfit| !current.outfits.contains(outfit))
                    .map(|outfit| outfit.source_id.clone()).collect(),
            })
        } else { None };
        let adjustments = run.recipe_adjustments.clone();
        self.execute_assembly(run, AssemblyExecution { selection, digest, target, rebuild, adjustments, fits }, ctx)
    }
    pub(crate) fn execute_assembly(
        &self,
        mut run: ProductionRun,
        execution: AssemblyExecution<'_>,
        ctx: &TaskContext,
    ) -> Result<(), AppErrorV1> {
        let AssemblyExecution { selection, digest, target, rebuild, adjustments, fits } = execution;
        face_tracking::require_replayable_face_tracking(&run)?;
        // The comparison and execution are separate reads. Check again at the
        // mutation boundary, including for Recipe application.
        let (matches_boundary, current) = self.project_state(&run)?;
        if !matches_boundary {
            return Err(production_error("project_drift", "The project changed before assembly. Reinspect it."));
        }
        run.project_fingerprint = current;
        run.task_id = ctx.task_id().into();
        run.phase = "assembling".into();
        run.error = None;
        run.recovery = None;
        let previous_selection = run.selection.clone();
        run.selection = Some(selection.clone());
        run.confirmed_digest = Some(digest.into());
        let mut snapshot = None;
        let mut revision_id = None;
        // A rebuild replaces the observed facts before assembly; a restore brings these back with the files.
        let observed_before = rebuild.then(|| (run.prefabs.clone(), run.result.clone(), run.unity_fingerprint.clone(), run.project_fingerprint.clone()));
        let result = (|| {
            self.step(&mut run, ctx, "snapshot", "Protect the prepared project")?;
            snapshot = Some(self.platform.snapshot(&run)?);
            run.snapshot_id = snapshot.clone();
            run.snapshot_selection = previous_selection.clone();
            run.snapshot_sources = None;
            // The version records the line as it was before this assembly.
            run.selection = previous_selection.clone();
            revision_id = Some(self.record_revision(
                &mut run,
                if rebuild { RevisionKind::Rebuild } else { RevisionKind::Assembly },
                false,
            ));
            run.selection = Some(selection.clone());
            self.save(&run)?;
            if rebuild {
                // Another base starts from a fresh work scene; the base and every chosen outfit are assembled on it.
                self.step(&mut run, ctx, "inspect", "Start a fresh work scene for the new base")?;
                self.platform.clear_work_scene(&run)?;
                self.update_observations(&mut run, ctx)?;
            }
            self.step(
                &mut run,
                ctx,
                "assembly",
                "Bind selected materials and create their switches",
            )?;
            let outfit_fits: Vec<OutfitFit> = fits
                .iter()
                .map(|fit| OutfitFit { source_id: fit.source_id.clone(), objects: fit.objects.clone() })
                .collect();
            let observed = match &target {
                Some(target) => self.platform.apply_recipe(&run, &selection, target, &outfit_fits, &self.watch(&run, ctx))?,
                None => self.platform.assemble(&run, &selection, &outfit_fits, &self.watch(&run, ctx))?,
            };
            if let Some(error) = ctx.cancellation_error("amf-production") {
                return Err(error);
            }
            if observed.editor_version != run.editor.version {
                return Err(production_error(
                    "editor_drift",
                    "The actual Editor version changed.",
                ));
            }
            if let Some(target) = &target {
                let actual: AssemblySelection = serde_json::from_value(observed.result["facts"]["observedSelection"].clone())
                    .map_err(|_| production_error("recipe_result_unverified", "Unity did not verify the resulting Recipe settings."))?;
                let same_outfits = actual.outfits.len() == selection.outfits.len()
                    && selection.outfits.iter().all(|wanted| actual.outfits.contains(wanted));
                let actual_avatar = observed.result["facts"]["avatarGlobalObjectId"].as_str().unwrap_or("");
                let correct_target = !actual_avatar.is_empty() && if target.new_avatar_key.is_some() {
                    actual_avatar != target.avatar_global_object_id
                } else { actual_avatar == target.avatar_global_object_id };
                if actual.avatar_prefab != selection.avatar_prefab || !same_outfits || !correct_target {
                    return Err(production_error("recipe_result_unverified", "The observed Avatar does not match the reviewed Recipe outcome."));
                }
            }
            run.result = observed.result;
            run.unity_fingerprint = observed.unity_fingerprint;
            let _check = FingerprintCheck::begin(self, &run, ctx);
            run.project_fingerprint = self.platform.fingerprint(Path::new(&run.project_root))?;
            run.phase = "succeeded".into();
            run.completed_at = Some(SystemClock.now_rfc3339());
            // A Recipe's recorded adjustments land on its first successful assembly: the
            // lighting values are written and the SDK fixes run with the same replay
            // functions a version-record restore uses, then Unity rechecks the project.
            if let Some(adjustments) = &adjustments {
                if !adjustments.lighting.is_empty() {
                    self.step(&mut run, ctx, "lighting", "Write the Recipe's recorded lighting values")?;
                    self.apply_lighting_values(&mut run, &adjustments.lighting, ctx)?;
                }
                if !adjustments.sdk_fixes.is_empty() {
                    self.step(&mut run, ctx, "sdk_fix", "Run the Recipe's recorded SDK fixes")?;
                    self.run_sdk_fixes(&mut run, &adjustments.sdk_fixes, ctx)?;
                }
                if !adjustments.lighting.is_empty() || !adjustments.sdk_fixes.is_empty() {
                    self.step(&mut run, ctx, "inspect", "Recheck the assembled project in Unity")?;
                    self.update_observations(&mut run, ctx)?;
                }
                run.recipe_adjustments = None;
            }
            Self::close_step(&mut run, "succeeded");
            // The fits that produced this state: rewritten garments take the fit they just
            // received (or drop their record when none applied), garments a partial assembly
            // kept are untouched, and garments no longer selected drop out.
            let written: std::collections::BTreeSet<&str> = match &target {
                Some(target) => target.write_source_ids.iter().map(String::as_str).collect(),
                None => selection.outfits.iter().map(|outfit| outfit.source_id.as_str()).collect(),
            };
            run.applied_fits.retain(|fit| !written.contains(fit.source_id.as_str()));
            run.applied_fits.extend(fits.iter().filter(|fit| written.contains(fit.source_id.as_str())).cloned());
            run.applied_fits.retain(|fit| selection.outfits.iter().any(|outfit| outfit.source_id == fit.source_id));
            let record =
                self.root
                    .join("records")
                    .join(format!("{}-{}.json", run.run_id, &digest[..16]));
            fs::create_dir_all(record.parent().expect("record parent")).map_err(io_error)?;
            if !record.exists() {
                let pending = record.with_extension("pending");
                fs::write(&pending, serde_json::to_vec_pretty(&run).map_err(io_error)?)
                    .map_err(io_error)?;
                fs::rename(pending, record).map_err(io_error)?;
            }
            Ok(())
        })();
        if result.is_err() {
            if let Some(snapshot) = snapshot {
                run.recovery = Some(match self.platform.restore(&run, &snapshot) {
                    Ok(()) => {
                        if let Some((prefabs, facts, unity, project)) = observed_before {
                            (run.prefabs, run.result, run.unity_fingerprint, run.project_fingerprint) = (prefabs, facts, unity, project);
                        }
                        "restored".into()
                    }
                    Err(error) => format!("restore_failed: {}", error.code),
                });
            }
            // A change whose automatic restore brought the project back leaves no state
            // behind worth a version record.
            if !matches!(run.recovery.as_deref(), Some(recovery) if recovery.starts_with("restore_failed")) {
                if let Some(id) = &revision_id {
                    run.revisions.retain(|revision| revision.id != *id);
                }
            }
        }
        let outcome = self.finish(&mut run, result);
        self.prune_snapshots(&run);
        outcome
    }
    pub fn rollback_plan(&self, run_id: &str) -> Result<serde_json::Value, AppErrorV1> {
        let run = self.get(run_id)?;
        if !matches!(run.phase.as_str(), "ready" | "succeeded" | "failed" | "cancelled" | "stopped") && !interrupted_change(&run) {
            return Err(production_error(
                "inspect_required",
                "Reinspect interrupted work before reviewing a restore.",
            ));
        }
        let snapshot = run.snapshot_id.as_ref().ok_or_else(|| {
            production_error("snapshot_missing", "This project has no retained restore point.")
        })?;
        let (unchanged, current) = self.project_state(&run)?;
        let digest = rollback_digest(&run, &current, RollbackTarget::RestorePoint)?;
        let mut plan = serde_json::json!({"schemaVersion":"0.1", "runId":run.run_id,
            "planDigest":digest, "snapshotId":snapshot, "projectRoot":run.project_root,
            "currentChanged":!unchanged, "preserveCurrent":true});
        // While the line carries the creator-patch marker, its pre-patch copy stays a second
        // reviewed target; when it equals the restore point the default restore already reaches it.
        if run.ft_pre_patch_snapshot.as_deref().is_some_and(|id| id != snapshot)
            && face_tracking::manual_face_tracking(&run) {
            plan["prePatch"] = serde_json::json!({
                "snapshotId":run.ft_pre_patch_snapshot,
                "planDigest":rollback_digest(&run, &current, RollbackTarget::PreCreatorPatch)?});
        }
        Ok(plan)
    }
    /// A restore is itself reversible. Persist the current-state snapshot before
    /// touching the project so a crash, failed inspection or cancellation cannot
    /// discard the player's latest edits.
    pub fn rollback(&self, run_id: &str, digest: &str, target: Option<RollbackTarget>, ctx: &TaskContext) -> Result<(), AppErrorV1> {
        let target = target.unwrap_or(RollbackTarget::RestorePoint);
        let plan = self.rollback_plan(run_id)?;
        let reviewed = match target {
            RollbackTarget::RestorePoint => plan["planDigest"].as_str(),
            RollbackTarget::PreCreatorPatch => Some(plan["prePatch"]["planDigest"].as_str().ok_or_else(|| {
                production_error("snapshot_missing", "This line no longer keeps a state from before the creator patch.")
            })?),
        };
        if reviewed != Some(digest) {
            return Err(production_error("plan_drift", "The project or restore point changed. Review the restore again."));
        }
        let mut run = self.get(run_id)?;
        // A change a forced end interrupted never finished; it is recorded as interrupted, not
        // as succeeded by the next step.
        if interrupted_change(&run) {
            if let Some(step) = run.steps.last_mut() {
                step.state = "interrupted".into();
                step.ended_at = Some(SystemClock.now_rfc3339());
            }
        }
        let target_kind = target;
        let (target, target_selection, target_sources) = match target {
            RollbackTarget::RestorePoint => (
                run.snapshot_id.clone().expect("reviewed restore point"),
                run.snapshot_selection.clone(),
                run.snapshot_sources.clone(),
            ),
            RollbackTarget::PreCreatorPatch => (
                run.ft_pre_patch_snapshot.clone().expect("reviewed pre-patch copy"),
                run.selection.clone(),
                None,
            ),
        };
        let current_selection = run.selection.clone();
        let current_sources = SourceSnapshot { choices: run.choices.clone(), sources: run.sources.clone() };
        run.task_id = ctx.task_id().into();
        run.phase = "restoring".into();
        run.error = None;
        run.recovery = None;
        run.confirmed_digest = None;
        let mut safety = None;
        let mut mutated = false;
        let edited = self.edited_in_unity(&run);
        let undo = self.record_revision(&mut run, RevisionKind::Restore, edited);
        let result = (|| {
            self.step(&mut run, ctx, "protect_current", "Protect the current project before restoring")?;
            let current = self.platform.fingerprint(Path::new(&run.project_root))?;
            if rollback_digest(&run, &current, target_kind)? != digest {
                return Err(production_error("plan_drift", "The project changed before restoration. Review it again."));
            }
            safety = Some(self.platform.snapshot(&run)?);
            run.snapshot_id = safety.clone();
            run.snapshot_selection = current_selection.clone();
            // Restoring across a material change brings the earlier materials back with the files.
            run.snapshot_sources = target_sources.as_ref().map(|_| current_sources.clone());
            self.step(&mut run, ctx, "restore", "Restore the reviewed project state")?;
            if self.platform.fingerprint(Path::new(&run.project_root))? != current {
                return Err(production_error("project_drift", "The project changed while protecting it. Review the restore again."));
            }
            mutated = true;
            self.platform.restore(&run, &target)?;
            if let Some(earlier) = &target_sources {
                run.choices = earlier.choices.clone();
                run.sources = earlier.sources.clone();
            }
            self.step(&mut run, ctx, "reinspect", "Verify the restored project in Unity")?;
            self.update_observations(&mut run, ctx)?;
            if let Some(error) = ctx.cancellation_error("amf-production") { return Err(error); }
            run.selection = target_selection;
            run.phase = "ready".into();
            run.completed_at = None;
            // A successful return to the pre-patch state ends the marker's retention: the
            // copy is spent and the protected current (patched) state is the restore point.
            if target_kind == RollbackTarget::PreCreatorPatch {
                run.ft_pre_patch_snapshot = None;
            }
            Ok(())
        })();
        if result.is_err() && mutated {
            if let Some(safety) = safety {
                run.recovery = Some(match self.platform.restore(&run, &safety) {
                    Ok(()) => "restored".into(),
                    Err(error) => format!("restore_failed: {}", error.code),
                });
            }
            run.selection = current_selection;
            // Neither the old result nor a partial restored inspection proves
            // the current state. Explicit reinspection must establish it again.
            run.result = serde_json::Value::Null;
        }
        // A restore that never touched the project, or whose automatic recovery brought
        // it back, leaves no state behind worth a version record.
        if result.is_err()
            && !matches!(run.recovery.as_deref(), Some(recovery) if recovery.starts_with("restore_failed"))
        {
            run.revisions.retain(|revision| revision.id != undo);
        }
        let outcome = self.finish(&mut run, result);
        self.prune_snapshots(&run);
        outcome
    }
    /// Work that changes a line's project needs Unity to let go of it; asking while Unity holds
    /// the project is refused at once instead of failing later as a task.
    pub fn precheck_closed(&self, run_id: &str) -> Result<(), AppErrorV1> {
        let run = self.get(run_id)?;
        if self.platform.editor_open(Path::new(&run.project_root)) {
            return Err(production_error("project_open", "Unity has this line's project open. Close that Unity window, then try again."));
        }
        Ok(())
    }
    pub fn precheck_open(&self, run_id: &str) -> Result<(), AppErrorV1> {
        let run = self.get(run_id)?;
        if run.phase != "succeeded" {
            return Err(production_error(
                "result_not_ready",
                "Assemble a result before opening the SDK handoff.",
            ));
        }
        let observed = self.platform.verify_editor(&run.editor.path)?;
        if observed.version != run.editor.version {
            return Err(production_error(
                "editor_drift",
                "The selected Editor identity changed.",
            ));
        }
        Ok(())
    }
    pub fn open(&self, run_id: &str, ctx: &TaskContext) -> Result<serde_json::Value, AppErrorV1> {
        self.precheck_open(run_id)?;
        self.platform.open(&self.get(run_id)?, ctx)
    }
    /// Reads one preview PNG that the run reports, for the Avatar page and the AI tools.
    pub fn preview_image(&self, run_id: &str, path: &str) -> Result<Vec<u8>, AppErrorV1> {
        let run = self.get(run_id)?;
        if !preview_paths(&run).iter().any(|known| known == path) {
            return Err(production_error("preview_unknown", "This run reports another set of previews. Read the run again."));
        }
        let bytes = fs::read(preview_file(Path::new(&run.project_root), path)?)
            .map_err(|e| production_error("preview_missing", e.to_string()))?;
        if bytes.len() as u64 > MAX_PREVIEW_BYTES || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(production_error("preview_invalid", "The preview file is not a bounded PNG image."));
        }
        Ok(bytes)
    }
}

/// A line's current VUA adjustments as a Recipe records them: the brightness limits each
/// in-use lilToon material carries, the SDK fixes applied so far and the fits each garment
/// received. Lines without any of these produce no `adjustments` section.
pub(crate) fn current_adjustments(run: &ProductionRun) -> Option<crate::amf_recipe::RecipeAdjustments> {
    let lighting = lighting::current_lighting(run);
    let fits: Vec<crate::amf_recipe::RecipeFit> = run
        .applied_fits
        .iter()
        .filter(|fit| !fit.objects.is_empty())
        .filter_map(|fit| {
            let source = run.sources.iter().find(|source| source.source_id == fit.source_id)?;
            Some(crate::amf_recipe::RecipeFit {
                material_id: format!("m-{}", source.sha256.strip_prefix("sha256:").unwrap_or(&source.sha256)),
                objects: fit.objects.clone(),
            })
        })
        .collect();
    if lighting.is_empty() && run.sdk_fixes_applied.is_empty() && fits.is_empty() {
        return None;
    }
    Some(crate::amf_recipe::RecipeAdjustments {
        lighting,
        sdk_fixes: run.sdk_fixes_applied.clone(),
        fits,
    })
}

pub fn validate_selection(
    run: &ProductionRun,
    selection: &AssemblySelection,
) -> Result<(), AppErrorV1> {    let observed: Option<AssemblySelection> = if run.result["facts"]["observedSelection"].is_null() { None }
        else { Some(serde_json::from_value(run.result["facts"]["observedSelection"].clone())
            .map_err(|_| production_error("inspect_required", "Current Avatar relationships need inspection."))?) };
    let rebuild = observed.as_ref().is_some_and(|current| current.avatar_prefab != selection.avatar_prefab);
    validate_selection_inner(run, selection, observed.is_none() || rebuild)?;
    if let Some(current) = observed.filter(|_| !rebuild) {
        check_retained_outfits(&current, selection)?;
    }
    Ok(())
}

/// An in-place assembly keeps every outfit the Avatar already wears, with the same prefab and
/// switch; leaving one out needs an explicit removal.
pub(crate) fn check_retained_outfits(current: &AssemblySelection, selection: &AssemblySelection) -> Result<(), AppErrorV1> {
    for retained in &current.outfits {
        let chosen = selection.outfits.iter().find(|outfit| outfit.source_id == retained.source_id)
            .ok_or_else(|| production_error("explicit_removal_required", "An existing outfit cannot be removed by omitting it from the selection."))?;
        if chosen.prefab_path != retained.prefab_path {
            return Err(production_error("outfit_target_changed", "An existing outfit has a different object identity. Choose a new Avatar to change its prefab."));
        }
        if chosen.toggle_name.trim().is_empty() && !retained.toggle_name.trim().is_empty() {
            return Err(production_error("explicit_removal_required", "An existing switch cannot be removed by clearing its name."));
        }
    }
    Ok(())
}

/// Whether assembling `selection` rebuilds an assembled line on another Avatar prefab: the
/// work scene starts fresh and the base and every chosen outfit are assembled again.
pub fn rebuilds_base(run: &ProductionRun, selection: &AssemblySelection) -> bool {
    serde_json::from_value::<AssemblySelection>(run.result["facts"]["observedSelection"].clone())
        .is_ok_and(|current| current.avatar_prefab != selection.avatar_prefab)
}

pub(crate) fn validate_selection_subset(run: &ProductionRun, selection: &AssemblySelection) -> Result<(), AppErrorV1> {
    validate_selection_inner(run, selection, false)
}

fn validate_selection_inner(run: &ProductionRun, selection: &AssemblySelection, require_all: bool) -> Result<(), AppErrorV1> {
    if run.phase != "ready" && run.phase != "succeeded" {
        return Err(production_error(
            "inspect_required",
            "Reinspect before planning or assembly.",
        ));
    }
    let avatar_sources = run
        .sources
        .iter()
        .filter(|s| matches!(s.role, SourceRole::Avatar | SourceRole::Auto))
        .map(|s| &s.source_id)
        .collect::<Vec<_>>();
    let avatar = run
        .prefabs
        .iter()
        .find(|p| {
            p.path == selection.avatar_prefab
                && p.is_avatar
                && avatar_sources.contains(&&p.source_id)
        })
        .ok_or_else(|| {
            production_error(
                "avatar_invalid",
                "Select an observed Avatar prefab from the Avatar input.",
            )
        })?;
    if avatar.has_missing_scripts {
        return Err(production_error(
            "missing_scripts",
            "The Avatar has missing scripts. Supply its actual dependencies.",
        ));
    }
    let required = run
        .sources
        .iter()
        .filter(|s| s.role == SourceRole::Outfit)
        .map(|s| s.source_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let allowed = run
        .sources
        .iter()
        .filter(|s| matches!(s.role, SourceRole::Outfit | SourceRole::Auto))
        .map(|s| s.source_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut selected = std::collections::BTreeSet::new();
    if selection.outfits.is_empty() || selection.outfits.len() > 64 {
        return Err(production_error(
            "outfits_invalid",
            "Select the outfit prefabs.",
        ));
    }
    for outfit in &selection.outfits {
        if !allowed.contains(outfit.source_id.as_str())
            || !selected.insert(outfit.source_id.as_str())
        {
            return Err(production_error(
                "outfits_invalid",
                "Each selected outfit input needs exactly one explicit prefab.",
            ));
        }
        let candidate = run
            .prefabs
            .iter()
            .find(|p| {
                p.source_id == outfit.source_id && p.path == outfit.prefab_path && !p.is_avatar
            })
            .ok_or_else(|| {
                production_error(
                    "outfit_invalid",
                    "The outfit prefab is not an observed candidate for this source.",
                )
            })?;
        if candidate.has_missing_scripts {
            return Err(production_error(
                "missing_scripts",
                "An outfit has missing scripts. Supply its dependencies.",
            ));
        }
        if outfit.toggle_name.chars().count() > 80
            || outfit.toggle_name.chars().any(char::is_control)
        {
            return Err(production_error(
                "menu_invalid",
                "Switch labels must be short, single-line text.",
            ));
        }
        match outfit.mode {
            BindingMode::MergeArmature
                if !candidate.has_skinned_mesh && !candidate.has_merge_armature =>
            {
                return Err(production_error(
                    "binding_invalid",
                    "A static accessory needs an explicit bone target.",
                ))
            }
            BindingMode::AttachToBone
                if candidate.has_skinned_mesh || !avatar.humanoid_bones.contains(&outfit.bone) =>
            {
                return Err(production_error(
                    "binding_invalid",
                    "Bone attachment needs a static accessory and an observed humanoid bone.",
                ))
            }
            _ => {}
        }
    }
    if require_all && !required.is_subset(&selected) {
        return Err(production_error(
            "outfits_invalid",
            "Some selected outfit inputs have no confirmed prefab.",
        ));
    }
    Ok(())
}

pub fn plan_digest(
    run: &ProductionRun,
    selection: &AssemblySelection,
) -> Result<String, AppErrorV1> {
    plan_digest_with_fits(run, selection, &[])
}

/// The plan digest with the resolved fits bound in: any fit content change between the
/// review and the assembly refuses the execution as `plan_drift`.
pub fn plan_digest_with_fits(
    run: &ProductionRun,
    selection: &AssemblySelection,
    fits: &[AppliedFit],
) -> Result<String, AppErrorV1> {
    let fits: Vec<serde_json::Value> = fits
        .iter()
        .map(|fit| {
            let objects_sha256 = Sha256::digest(serde_json::to_string(&fit.objects).map_err(io_error)?.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            Ok(serde_json::json!({"sourceId":fit.source_id,"fitId":fit.fit_id,"name":fit.name,"objectsSha256":objects_sha256}))
        })
        .collect::<Result<_, AppErrorV1>>()?;
    let value = serde_json::json!({"runId":run.run_id,"revision":run.revision,"editor":run.editor,"projectRoot":run.project_root,
        "sources":run.sources,"prefabs":run.prefabs,"projectFingerprint":run.project_fingerprint,
        "unityFingerprint":run.unity_fingerprint,"selection":selection,"recipeId":run.recipe_id,
        "recipeSourceHashes":run.recipe_source_hashes,"packagePins":run.package_pins,
        "recipeAdjustments":run.recipe_adjustments,"fits":fits});
    Ok(
        Sha256::digest(serde_json::to_vec(&value).map_err(io_error)?)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

fn rollback_digest(
    run: &ProductionRun,
    current_fingerprint: &str,
    target: RollbackTarget,
) -> Result<String, AppErrorV1> {
    // A marked line's selection and materials cannot change, so the pre-patch copy restores to
    // the selection the line still wears; the digest names which retained copy a review bound.
    let (target_name, snapshot_id, snapshot_selection) = match target {
        RollbackTarget::RestorePoint => (
            "restore_point",
            run.snapshot_id.clone(),
            run.snapshot_selection.clone(),
        ),
        RollbackTarget::PreCreatorPatch => (
            "pre_creator_patch",
            run.ft_pre_patch_snapshot.clone(),
            run.selection.clone(),
        ),
    };
    let value = serde_json::json!({"operation":"restore", "target":target_name, "runId":run.run_id,
        "revision":run.revision, "editor":run.editor, "projectRoot":run.project_root,
        "snapshotId":snapshot_id, "snapshotSelection":snapshot_selection,
        "currentFingerprint":current_fingerprint});
    Ok(Sha256::digest(serde_json::to_vec(&value).map_err(io_error)?)
        .iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Which retained full copy a reviewed restore returns the line to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackTarget {
    /// The line's current restore point.
    RestorePoint,
    /// The copy kept from before the creator patch, present while the line carries its marker.
    PreCreatorPatch,
}

impl RollbackTarget {
    /// The wire name of the target; `None` names the default restore point for older callers.
    pub fn parse(value: Option<&str>) -> Result<Self, AppErrorV1> {
        match value {
            None | Some("restorePoint") => Ok(Self::RestorePoint),
            Some("prePatch") => Ok(Self::PreCreatorPatch),
            Some(_) => Err(production_error(
                "params_invalid",
                "Choose the restore point or the state before the creator patch.",
            )),
        }
    }
}

