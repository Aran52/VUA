//! Synthetic use-case tests only; not real Unity or N3 material acceptance.
#![allow(clippy::result_large_err)]
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use vua_orchestrator::{
    amf_production::*, AppErrorV1, NanosTaskIdGenerator, SqliteTaskStore, SubmitRequest,
    SystemClock, TaskContext, TaskExit, TaskRuntime,
};

const PIPELINE_FIX: &str = "This avatar has no pipeline manager component.";

struct TestPlatform {
    texture_facts: Mutex<Option<serde_json::Value>>,
    texture_before: Mutex<Option<serde_json::Value>>,
    texture_outcome: AtomicUsize,
    removal_mode: AtomicUsize,
    ft_facts: Mutex<Option<serde_json::Value>>,
    ft_outcome: AtomicUsize,
    ft_snapshots: Mutex<std::collections::BTreeMap<String, (String, bool)>>,
    fail: AtomicBool,
    fail_restore: AtomicBool,
    fail_inspect: AtomicBool,
    change_recipe_input: AtomicBool,
    /// The next Unity step reports progress and runs until the player stops it.
    wait_for_stop: AtomicBool,
    /// The next material change writes part of the project, then fails.
    fail_replace: AtomicBool,
    /// Unity holds the work project.
    editor_open: AtomicBool,
    close_outcome: AtomicUsize,
    /// The Avatar package carries a second Avatar prefab.
    alt_avatar: AtomicBool,
    /// The Avatar source reports a different content hash.
    alt_avatar_hash: AtomicBool,
    /// How often the work scene was cleared for a rebuild.
    cleared: AtomicUsize,
    /// Every selection Unity assembled, in order.
    assembled_with: Mutex<Vec<AssemblySelection>>,
    /// The fits each assembly received, in order.
    assembled_fits: Mutex<Vec<Vec<OutfitFit>>>,
    snapshots: AtomicUsize,
    root: PathBuf,
    /// Every keep list the engine asked to prune to, newest last.
    prune_keeps: Mutex<Vec<Vec<String>>>,
    /// The material use facts the next Unity check reports, when set.
    material_use: std::sync::Mutex<serde_json::Value>,
    /// The fitChanges facts every Unity check reports, when set.
    fit_changes: std::sync::Mutex<Option<serde_json::Value>>,
    /// Original file contents a restore writes back, as a real snapshot would.
    restore_files: std::sync::Mutex<Vec<(PathBuf, Vec<u8>)>>,
}
/// A Unity step that imports until the player asks to stop, then ends as Unity would.
fn unity_until_stopped(watch: &UnityWatch) -> AppErrorV1 {
    let started = std::time::Instant::now();
    let mut current = 0;
    while !watch.stop_requested() {
        assert!(started.elapsed() < Duration::from_secs(10), "the stop request never reached the Unity step");
        current = (current + 1).min(379);
        watch.report(UnityProgress { stage: UnityStage::Importing, current: Some(current), total: Some(379),
            last_activity_at: "2026-10-17T08:00:00.000Z".into() });
        std::thread::sleep(Duration::from_millis(10));
    }
    watch.stopped()
}
fn candidates() -> Vec<PrefabCandidate> {
    vec![
        PrefabCandidate {
            source_id: "avatar".into(),
            path: "Assets/Avatar.prefab".into(),
            name: "Avatar".into(),
            is_avatar: true,
            has_skinned_mesh: true,
            has_merge_armature: false,
            has_missing_scripts: false,
            humanoid_bones: vec!["Head".into()],
            preview: Some(".vua/previews/cmd/candidate-000.png".into()),
            parameter_budget: Some(ParameterBudget {
                expression_parameters_observed: true,
                expression_parameters: vec![
                    ParameterEntry { name: "VRCEmote".into(), bits: 8, unique: false },
                    ParameterEntry { name: "VRMode".into(), bits: 1, unique: false },
                ],
                ma_components: vec![ParameterComponent {
                    path: "Default Clothing".into(),
                    parameters: vec![ParameterEntry { name: "DefaultDress".into(), bits: 1, unique: false }],
                }],
                unresolved_prefix_configs: 0,
                truncated: false,
            }),
        },
        PrefabCandidate {
            source_id: "outfit".into(),
            path: "Assets/Outfit.prefab".into(),
            name: "Coat".into(),
            is_avatar: false,
            has_skinned_mesh: true,
            has_merge_armature: false,
            has_missing_scripts: false,
            humanoid_bones: vec![],
            preview: None,
            parameter_budget: Some(ParameterBudget {
                expression_parameters_observed: true,
                expression_parameters: vec![],
                ma_components: vec![ParameterComponent {
                    path: String::new(),
                    parameters: vec![ParameterEntry { name: "CoatOn".into(), bits: 1, unique: false }],
                }],
                unresolved_prefix_configs: 0,
                truncated: false,
            }),
        },
        PrefabCandidate {
            source_id: "outfit2".into(),
            path: "Assets/Outfit2.prefab".into(),
            name: "Dress".into(),
            is_avatar: false,
            has_skinned_mesh: true,
            has_merge_armature: false,
            has_missing_scripts: false,
            humanoid_bones: vec![],
            preview: None,
            parameter_budget: None,
        },
    ]
}
impl AmfProductionPlatform for TestPlatform {
    fn resolve_recipe_materials(&self, materials: &[vua_orchestrator::amf_recipe::RecipeMaterial]) -> Result<std::collections::BTreeMap<String,String>, AppErrorV1> {
        materials.iter().map(|material| {
            let source = if material.sha256 == "a".repeat(64) { "avatar" } else if material.sha256 == "b".repeat(64) { "outfit" }
                else { return Err(production_error("recipe_material_missing", "Synthetic missing original.")); };
            Ok((material.id.clone(), source.into()))
        }).collect()
    }
    fn sources(&self) -> Result<Vec<ProductionSource>, AppErrorV1> {
        Ok(["avatar", "outfit", "outfit2"]
            .into_iter()
            .map(|id| ProductionSource {
                source_id: id.into(),
                item_id: id.into(),
                item_name: id.into(),
                name: format!("{id}.unitypackage"),
                product_ids: Vec::new(),
                path: self.root.join(id),
                archive_member: None,
            })
            .collect())
    }
    fn editors(&self) -> Result<Vec<ProductionEditor>, AppErrorV1> {
        Ok(vec![self.verify_editor("synthetic-editor")?])
    }
    fn verify_editor(&self, path: &str) -> Result<ProductionEditor, AppErrorV1> {
        Ok(ProductionEditor {
            path: path.into(),
            version: "2022.3.22f1c1".into(),
        })
    }
    fn create_project(&self, run: &ProductionRun, _: &TaskContext) -> Result<(), AppErrorV1> {
        fs::create_dir_all(Path::new(&run.project_root).join("Assets")).unwrap();
        fs::write(Path::new(&run.project_root).join("Assets/state"), "before").unwrap();
        Ok(())
    }
    fn import_sources(
        &self,
        run: &ProductionRun,
        _: &TaskContext,
    ) -> Result<Vec<PreparedSource>, AppErrorV1> {
        Ok(run
            .choices
            .iter()
            .map(|s| PreparedSource {
                source_id: s.source_id.clone(),
                role: s.role,
                name: format!("{}.unitypackage", s.source_id),
                item_name: s.source_id.clone(),
                sha256: format!("sha256:{}", if s.source_id == "avatar" { if self.alt_avatar_hash.load(Ordering::SeqCst) { "d".repeat(64) } else { "a".repeat(64) } } else if self.change_recipe_input.load(Ordering::SeqCst) { "c".repeat(64) } else { "b".repeat(64) }),
                asset_paths: vec![format!("Assets/{}.prefab", s.source_id)],
            })
            .collect())
    }
    fn replace_sources(&self, run: &ProductionRun, _: &[PreparedSource], ctx: &TaskContext) -> Result<Vec<PreparedSource>, AppErrorV1> {
        let ids = run.choices.iter().map(|choice| choice.source_id.as_str()).collect::<Vec<_>>().join("+");
        let state = Path::new(&run.project_root).join("Assets/state");
        if self.fail_replace.swap(false, Ordering::SeqCst) {
            fs::write(&state, "half-changed").unwrap();
            return Err(production_error("test_replace", "Synthetic failure while changing materials."));
        }
        fs::write(&state, format!("materials:{ids}")).unwrap();
        self.import_sources(run, ctx)
    }
    fn editor_open(&self, _: &Path) -> bool {
        self.editor_open.load(Ordering::SeqCst)
    }
    fn inspect(&self, run: &ProductionRun, watch: &UnityWatch) -> Result<ObservedProject, AppErrorV1> {
        if self.wait_for_stop.swap(false, Ordering::SeqCst) {
            return Err(unity_until_stopped(watch));
        }
        if self.fail_inspect.swap(false, Ordering::SeqCst) {
            return Err(production_error("test_inspect", "Synthetic inspection failed."));
        }
        let previews = Path::new(&run.project_root).join(".vua/previews/cmd");
        fs::create_dir_all(&previews).unwrap();
        for file in ["front.png", "candidate-000.png", "unreported.png"] {
            fs::write(previews.join(file), b"\x89PNG\r\n\x1a\nsynthetic").unwrap();
        }
        let mut prefabs = candidates();
        if self.alt_avatar.load(Ordering::SeqCst) {
            prefabs.push(PrefabCandidate { path: "Assets/AvatarAlt.prefab".into(), name: "Avatar Alt".into(), preview: None, ..candidates().remove(0) });
        }
        let mut assembled = self.assembled_with.lock().unwrap().last().cloned().unwrap_or_else(selection);
        let state = fs::read_to_string(Path::new(&run.project_root).join("Assets/state")).unwrap();
        if let Some(removed) = state.strip_prefix("removed:") { assembled.outfits.retain(|o| o.source_id != removed); }
        if state.starts_with("removed:") && self.removal_mode.load(Ordering::SeqCst) == 3 { assembled.outfits[0].active = !assembled.outfits[0].active; }
        let mut result = serde_json::json!({"synthetic":true,
            "facts":{"avatarName":"Synthetic Avatar", "avatarGlobalObjectId":"synthetic-avatar", "usedSourceIds":["avatar","outfit"],
                "previews":[{"view":"front","path":".vua/previews/cmd/front.png"}], "observedSelection":(state == "after" || state.starts_with("removed:")).then_some(assembled.clone()),
                "sdkReview":{"state":"checked","platform":"pc","overall":"Poor","detail":"","stats":[],"issues":[
                    {"level":"error","text":PIPELINE_FIX,"canFix":true},{"level":"warning","text":"This avatar is shorter than average.","canFix":false}]}},
            "packageVersions":{"com.vrchat.avatars":"3.10.5","nadena.dev.modular-avatar":"1.18.7"}});
        if let Some(textures) = self.texture_facts.lock().unwrap().clone() { result["facts"]["textureMemory"] = textures; }
        if self.removal_mode.load(Ordering::SeqCst) > 0 {
            let outfits = if result["facts"]["observedSelection"].is_null() { &[][..] } else { assembled.outfits.as_slice() };
            result["facts"]["instances"] = serde_json::json!(outfits.iter().map(|o| serde_json::json!({
                "sourceId":o.source_id,"globalObjectId":format!("id-{}",o.source_id),"bindingVerified":true,"menuVerified":true,"active":o.active})).collect::<Vec<_>>());
            result["facts"]["outfitRemoval"] = serde_json::json!(outfits.iter().map(|o| synthetic_removal(&o.source_id)).collect::<Vec<_>>());
            result["facts"]["avatars"] = serde_json::json!([
                {"globalObjectId":"synthetic-avatar","activeTarget":true,"observedSelection":result["facts"]["observedSelection"]},
                {"globalObjectId":"other-avatar","activeTarget":false,"observedSelection":selection2()}]);
        }
        let material_use = self.material_use.lock().unwrap().clone();
        if let Some(mut facts) = self.ft_facts.lock().unwrap().clone() {
            let patched = Path::new(&run.project_root).join("ProjectSettings/VUA.FaceTracking.json").exists();
            facts["manualReplayRequired"] = serde_json::json!(patched);
            result["facts"]["faceTracking"] = facts;
            if patched && self.ft_outcome.load(Ordering::SeqCst) == 5 {
                result["facts"]["avatarGlobalObjectId"] = serde_json::json!("wrong-avatar");
            }
        }
        if !material_use.is_null() {
            result["facts"]["materialUse"] = material_use;
        }
        if let Some(fit_changes) = self.fit_changes.lock().unwrap().clone() {
            result["facts"]["fitChanges"] = fit_changes;
        }
        Ok(ObservedProject {
            prefabs,
            unity_fingerprint: "scene-before".into(),
            editor_version: "2022.3.22f1c1".into(),
            result,
        })
    }
    fn assemble(
        &self,
        run: &ProductionRun,
        selection: &AssemblySelection,
        fits: &[OutfitFit],
        watch: &UnityWatch,
    ) -> Result<ObservedProject, AppErrorV1> {
        fs::write(Path::new(&run.project_root).join("Assets/state"), "after").unwrap();
        if self.fail.load(Ordering::SeqCst) {
            return Err(production_error(
                "test_failure",
                "Synthetic failure after a mutation.",
            ));
        }
        self.assembled_with.lock().unwrap().push(selection.clone());
        self.assembled_fits.lock().unwrap().push(fits.to_vec());
        self.inspect(run, watch)
    }
    fn clear_work_scene(&self, run: &ProductionRun) -> Result<(), AppErrorV1> {
        self.cleared.fetch_add(1, Ordering::SeqCst);
        fs::write(Path::new(&run.project_root).join("Assets/state"), "fresh").unwrap();
        Ok(())
    }
    fn fingerprint(&self, project: &Path) -> Result<String, AppErrorV1> {
        fs::read_to_string(project.join("Assets/state"))
            .map_err(|e| production_error("test_io", e.to_string()))
    }
    /// The v1 analog of this platform's fingerprint: the same state under the old prefix,
    /// as a record written before the cached value would carry it.
    fn fingerprint_legacy(&self, project: &Path) -> Result<String, AppErrorV1> {
        Ok(format!("sha256:{}", self.fingerprint(project)?))
    }
    fn apply_recipe(&self, run: &ProductionRun, selection: &AssemblySelection, target: &vua_orchestrator::amf_recipe_apply::RecipeApplyTarget, fits: &[OutfitFit], watch: &UnityWatch) -> Result<ObservedProject, AppErrorV1> {
        let mut observed = self.assemble(run, selection, fits, watch)?;
        observed.result["facts"]["observedSelection"] = serde_json::to_value(selection).unwrap();
        observed.result["facts"]["avatarGlobalObjectId"] = serde_json::json!(if target.new_avatar_key.is_some() { "synthetic-new-avatar" } else { &target.avatar_global_object_id });
        Ok(observed)
    }
    fn texture_import(&self, run: &ProductionRun, changes: &[TextureImportChange], watch: &UnityWatch) -> Result<ObservedProject, AppErrorV1> {
        assert!(run.snapshot_id.is_some());
        let saved: ProductionRun = serde_json::from_slice(&fs::read(self.root.join("production/runs/n3-test.json")).unwrap()).unwrap();
        assert_eq!(saved.revisions[0].before, RevisionKind::TextureMemory, "history is durable before writes");
        let mut reports = validate_texture_memory(self.texture_facts.lock().unwrap().as_ref().unwrap()).unwrap();
        for change in changes {
            let report = reports.iter_mut().find(|r| r.platform == change.platform).unwrap();
            let texture = report.textures.iter_mut().find(|t| t.path == change.path).unwrap();
            let scale = (texture.width.max(texture.height) / change.max_size).max(1);
            texture.width = (texture.width / scale).max(1); texture.height = (texture.height / scale).max(1);
            texture.max_size = change.max_size; texture.format = change.format.clone();
            texture.estimated_bytes = estimate_texture_bytes(texture.width, texture.height, texture.mipmap, &texture.format, 1).unwrap();
            report.total_bytes = report.textures.iter().map(|t| t.estimated_bytes).sum(); report.rank = texture_rank(report.total_bytes, report.platform);
        }
        if self.texture_outcome.load(Ordering::SeqCst) == 5 {
            let report = reports.iter_mut().find(|r| r.platform == changes[0].platform).unwrap();
            let mut unknown = report.textures[0].clone();
            unknown.id = "generated-unknown".into(); unknown.path.clear(); unknown.known = false;
            unknown.editable = false; unknown.format = "Unknown".into(); unknown.estimated_bytes = 0;
            report.textures.push(unknown); report.complete = false; report.rank = TextureRank::Unknown;
        }
        *self.texture_facts.lock().unwrap() = Some(serde_json::json!(reports));
        match self.texture_outcome.load(Ordering::SeqCst) {
            1 => return Err(production_error("texture_test_failed", "Importer failed after a partial write.")),
            2 => { *self.texture_facts.lock().unwrap() = self.texture_before.lock().unwrap().clone(); },
            3 => return Err(watch.stopped()),
            _ => (),
        }
        let observed = self.inspect(run, watch)?;
        if self.texture_outcome.load(Ordering::SeqCst) == 4 { self.fail_inspect.store(true, Ordering::SeqCst); }
        Ok(observed)
    }
    fn remove_outfit(&self, run: &ProductionRun, removal: &OutfitRemoval, watch: &UnityWatch) -> Result<ObservedProject, AppErrorV1> {
        assert!(run.snapshot_id.is_some());
        let durable: ProductionRun = serde_json::from_slice(&fs::read(self.root.join("production/runs/n3-test.json")).unwrap()).unwrap();
        assert_eq!(durable.revisions[0].before, RevisionKind::OutfitRemoval);
        fs::write(Path::new(&run.project_root).join("Assets/state"), format!("removed:{}", removal.source_id)).unwrap();
        match self.removal_mode.load(Ordering::SeqCst) {
            2 => return Err(production_error("test_removal_failed", "Synthetic deletion failed after writing.")),
            4 => return Err(watch.stopped()),
            _ => {}
        }
        let mut receipt = self.inspect(run, watch)?;
        receipt.result["facts"]["outfitRemovalApplied"] = serde_json::json!([{
            "sourceId":removal.source_id,"avatarGlobalObjectId":removal.avatar_global_object_id,
            "deletedObjectIds":removal.objects.iter().map(|o| &o.global_object_id).collect::<Vec<_>>() }]);
        if self.removal_mode.load(Ordering::SeqCst) == 5 { receipt.result["facts"]["outfitRemovalApplied"] = serde_json::json!([]); }
        Ok(receipt)
    }
    fn sdk_fix(&self, run: &ProductionRun, fixes: &[String], watch: &UnityWatch) -> Result<ObservedProject, AppErrorV1> {
        if self.fail.load(Ordering::SeqCst) {
            fs::write(Path::new(&run.project_root).join("Assets/state"), "half-fixed").unwrap();
            return Err(production_error("test_failure", "Synthetic failure inside the SDK fix."));
        }
        let mut observed = self.inspect(run, watch)?;
        observed.result["facts"]["sdkFixesApplied"] = serde_json::json!(fixes);
        Ok(observed)
    }
    fn snapshot(&self, run: &ProductionRun) -> Result<String, AppErrorV1> {
        *self.texture_before.lock().unwrap() = self.texture_facts.lock().unwrap().clone();
        let index = self.snapshots.fetch_add(1, Ordering::SeqCst);
        let state = self.fingerprint(Path::new(&run.project_root))?;
        if self.ft_facts.lock().unwrap().is_some() {
            let id = format!("ft-snapshot-{index}");
            self.ft_snapshots.lock().unwrap().insert(id.clone(), (state,
                Path::new(&run.project_root).join("ProjectSettings/VUA.FaceTracking.json").exists()));
            Ok(id)
        } else { Ok(state) }
    }
    fn face_tracking_patch(&self, run: &ProductionRun, patch: &FaceTrackingPatch, watch: &UnityWatch) -> Result<ObservedProject, AppErrorV1> {
        assert!(patch.confirm_trusted_code);
        assert_eq!(patch.avatar_global_object_id, "synthetic-avatar");
        assert!(run.snapshot_id.is_some());
        let marker = Path::new(&run.project_root).join("ProjectSettings/VUA.FaceTracking.json");
        fs::create_dir_all(marker.parent().unwrap()).unwrap();
        fs::write(marker, "{}").unwrap();
        match self.ft_outcome.load(Ordering::SeqCst) {
            2 => return Err(production_error("test_patch_failed", "Synthetic patch changed a mesh then failed.")),
            6 => return Err(watch.stopped()),
            7 => return Err(production_error("process_cleanup_unverified", "Synthetic helper is still running.")),
            _ => (),
        }
        let mut observed = self.inspect(run, watch)?;
        match self.ft_outcome.load(Ordering::SeqCst) {
            3 => observed.result["facts"]["faceTracking"]["manualReplayRequired"] = serde_json::json!(false),
            4 => self.fail_inspect.store(true, Ordering::SeqCst),
            _ => (),
        }
        Ok(observed)
    }
    fn source_content_hash(&self, source_id: &str) -> Result<Option<String>, AppErrorV1> {
        if !["avatar", "outfit", "outfit2"].contains(&source_id) {
            return Ok(None);
        }
        Ok(Some(format!("sha256:{}", if source_id == "avatar" { if self.alt_avatar_hash.load(Ordering::SeqCst) { "d".repeat(64) } else { "a".repeat(64) } }
            else if self.change_recipe_input.load(Ordering::SeqCst) { "c".repeat(64) } else { "b".repeat(64) })))
    }
    fn prune_snapshots(&self, run: &ProductionRun, keep: &[&str]) -> Vec<String> {
        self.prune_keeps.lock().unwrap().push(keep.iter().map(|id| id.to_string()).collect());
        vua_orchestrator::FileSystemSnapshotStore.prune_unreferenced(Path::new(&run.project_root), keep)
    }
    fn restore(&self, run: &ProductionRun, snapshot: &str) -> Result<(), AppErrorV1> {
        if self.editor_open.load(Ordering::SeqCst) { return Err(production_error("project_open", "Synthetic live Editor lock.")); }
        if run.phase == "restoring" {
            let saved: ProductionRun = serde_json::from_slice(&fs::read(self.root.join("production/runs/n3-test.json")).unwrap()).unwrap();
            assert_eq!(saved.phase, "restoring");
            assert_eq!(saved.snapshot_id, run.snapshot_id, "Current-state recovery must be durable before any restore writes");
        }
        if self.fail_restore.swap(false, Ordering::SeqCst) {
            fs::write(Path::new(&run.project_root).join("Assets/state"), "partial restore").unwrap();
            return Err(production_error("test_restore", "Synthetic partial restore."));
        }
        *self.texture_facts.lock().unwrap() = self.texture_before.lock().unwrap().clone();
        for (path, bytes) in self.restore_files.lock().unwrap().drain(..) {
            fs::write(path, bytes).unwrap();
        }
        if let Some((state, marker_exists)) = self.ft_snapshots.lock().unwrap().get(snapshot) {
            let marker = Path::new(&run.project_root).join("ProjectSettings/VUA.FaceTracking.json");
            if *marker_exists { fs::write(marker, "{}").unwrap(); }
            else if marker.exists() { fs::remove_file(marker).unwrap(); }
            fs::write(Path::new(&run.project_root).join("Assets/state"), state).unwrap();
        } else {
            fs::write(Path::new(&run.project_root).join("Assets/state"), snapshot).unwrap();
        }
        Ok(())
    }
    fn snapshot_open_editor(&self, run: &ProductionRun) -> Result<String, AppErrorV1> { self.snapshot(run) }
    fn close_editor(&self, run: &ProductionRun, _: &TaskContext) -> Result<EditorCloseResult, AppErrorV1> {
        let saved: ProductionRun = serde_json::from_slice(&fs::read(self.root.join("production/runs/n3-test.json")).unwrap()).unwrap();
        assert!(saved.snapshot_id.is_some());
        assert_eq!(saved.revisions[0].before, RevisionKind::EditorSave);
        assert_eq!(saved.steps.last().unwrap().key, "editor_close");
        let mode = self.close_outcome.load(Ordering::SeqCst);
        if mode != 5 { fs::write(Path::new(&run.project_root).join("Assets/state"), "saved-player-edits").unwrap(); }
        if mode != 2 && mode != 5 { self.editor_open.store(false, Ordering::SeqCst); }
        Ok(EditorCloseResult { schema_version:"0.1".into(), run_id:run.run_id.clone(), close_id:"close-1".into(),
            outcome:if mode == 1 { "saved" } else { "failed" }.into(), scenes_saved:mode == 1, assets_saved:mode == 1,
            editor_closed:mode != 2 && mode != 5, reason:if mode == 1 { "" } else if mode == 5 { "editor_busy" } else { "scene_save_failed" }.into(), detail:"Synthetic save receipt.".into() })
    }
    fn open(&self, _: &ProductionRun, _: &TaskContext) -> Result<serde_json::Value, AppErrorV1> {
        Ok(serde_json::json!({"synthetic":true}))
    }
}
fn selection() -> AssemblySelection {
    AssemblySelection {
        avatar_prefab: "Assets/Avatar.prefab".into(),
        outfits: vec![OutfitSelection {
            source_id: "outfit".into(),
            prefab_path: "Assets/Outfit.prefab".into(),
            mode: BindingMode::MergeArmature,
            bone: "".into(),
            toggle_name: "Coat".into(),
            active: true,
        }],
    }
}
struct Harness {
    root: PathBuf,
    engine: Arc<AmfProductionEngine>,
    platform: Arc<TestPlatform>,
    runtime: TaskRuntime,
}
impl Harness {
    fn new() -> Self {
        // Parallel tests can read the same clock value; the counter keeps each root unique.
        static NEXT_ROOT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "vua-n3-test-{}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let platform = Arc::new(TestPlatform {
            texture_facts: Mutex::new(None), texture_before: Mutex::new(None), texture_outcome: AtomicUsize::new(0),
            removal_mode: AtomicUsize::new(0),
            ft_facts: Mutex::new(None),
            ft_outcome: AtomicUsize::new(1),
            ft_snapshots: Mutex::new(std::collections::BTreeMap::new()),
            fail: AtomicBool::new(false),
            fail_restore: AtomicBool::new(false),
            fail_inspect: AtomicBool::new(false),
            change_recipe_input: AtomicBool::new(false),
            wait_for_stop: AtomicBool::new(false),
            fail_replace: AtomicBool::new(false),
            editor_open: AtomicBool::new(false),
            close_outcome: AtomicUsize::new(1),
            alt_avatar: AtomicBool::new(false),
            alt_avatar_hash: AtomicBool::new(false),
            cleared: AtomicUsize::new(0),
            assembled_with: Mutex::new(Vec::new()),
            assembled_fits: Mutex::new(Vec::new()),
            snapshots: AtomicUsize::new(0),
            root: root.clone(),
            prune_keeps: Mutex::new(Vec::new()),
            material_use: std::sync::Mutex::new(serde_json::Value::Null),
            fit_changes: std::sync::Mutex::new(None),
            restore_files: std::sync::Mutex::new(Vec::new()),
        });
        let engine = Arc::new(AmfProductionEngine::new(
            root.join("production"),
            platform.clone(),
        ));
        let runtime = TaskRuntime::with_sqlite(
            Arc::new(SqliteTaskStore::open(root.join("tasks.db")).unwrap()),
            Arc::new(SystemClock),
            Arc::new(NanosTaskIdGenerator::default()),
        )
        .unwrap();
        Self {
            root,
            engine,
            platform,
            runtime,
        }
    }
    fn job(
        &self,
        action: impl FnOnce(&AmfProductionEngine, &TaskContext) -> Result<(), AppErrorV1>
            + Send
            + 'static,
    ) -> bool {
        self.run_job(action).is_ok()
    }
    /// A job whose failure code the test inspects.
    fn run_job(
        &self,
        action: impl FnOnce(&AmfProductionEngine, &TaskContext) -> Result<(), AppErrorV1>
            + Send
            + 'static,
    ) -> Result<(), String> {
        let engine = self.engine.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.runtime
            .submit(SubmitRequest {
                correlation_id: None,
                timeout: None,
                job: Box::new(move |ctx| {
                    let result = action(&engine, ctx);
                    tx.send(result.as_ref().map(|_| ()).map_err(|error| error.code.clone())).unwrap();
                    result.map(|()| TaskExit::Done(serde_json::json!({"synthetic":true})))
                }),
            })
            .unwrap();
        // CI runs these tests in parallel, each with its own task store; a synthetic job that
        // finishes in milliseconds locally can take seconds there. A hung job still fails.
        rx.recv_timeout(Duration::from_secs(60)).unwrap()
    }
    fn prepared(&self) {
        self.engine
            .begin(
                "n3-test",
                vec![
                    SourceChoice {
                        source_id: "avatar".into(),
                        role: SourceRole::Auto,
                    },
                    SourceChoice {
                        source_id: "outfit".into(),
                        role: SourceRole::Auto,
                    },
                ],
                "synthetic-editor",
            )
            .unwrap();
        assert!(self.job(|engine, ctx| engine.prepare("n3-test", ctx)));
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        assert!(
            self.root.starts_with(std::env::temp_dir())
                && self
                    .root
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("vua-n3-test-")
        );
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn confirmation_binds_source_target_editor_and_reinspection_revision() {
    let h = Harness::new();
    h.prepared();
    let run = h.engine.get("n3-test").unwrap();
    let digest = plan_digest(&run, &selection()).unwrap();
    let mut changed = run.clone();
    changed.editor.version = "2022.3.22f1".into();
    assert_ne!(digest, plan_digest(&changed, &selection()).unwrap());
    changed = run.clone();
    changed.sources[0].sha256 = "b".repeat(64);
    assert_ne!(digest, plan_digest(&changed, &selection()).unwrap());
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    assert_ne!(
        digest,
        plan_digest(&h.engine.get("n3-test").unwrap(), &selection()).unwrap()
    );
}
#[test]
fn only_previews_the_run_reports_are_read() {
    let h = Harness::new();
    h.prepared();
    let front = h.engine.preview_image("n3-test", ".vua/previews/cmd/front.png").unwrap();
    assert!(front.starts_with(b"\x89PNG"));
    assert!(h.engine.preview_image("n3-test", ".vua/previews/cmd/candidate-000.png").is_ok());
    for unreported in [".vua/previews/cmd/unreported.png", ".vua/previews/cmd/../cmd/front.png", "Assets/state"] {
        assert_eq!(h.engine.preview_image("n3-test", unreported).unwrap_err().code, "vua.amf_production.preview_unknown");
    }
    let run = h.engine.get("n3-test").unwrap();
    fs::write(Path::new(&run.project_root).join(".vua/previews/cmd/front.png"), b"not an image").unwrap();
    assert_eq!(h.engine.preview_image("n3-test", ".vua/previews/cmd/front.png").unwrap_err().code, "vua.amf_production.preview_invalid");
}
#[test]
fn project_drift_refuses_before_snapshot_or_mutation() {
    let h = Harness::new();
    h.prepared();
    let run = h.engine.get("n3-test").unwrap();
    let digest = plan_digest(&run, &selection()).unwrap();
    fs::write(
        Path::new(&run.project_root).join("Assets/state"),
        "player-change",
    )
    .unwrap();
    assert!(!h.job(move |engine, ctx| engine.assemble("n3-test", selection(), &digest, ctx)));
    assert_eq!(
        fs::read_to_string(Path::new(&run.project_root).join("Assets/state")).unwrap(),
        "player-change"
    );
}
#[test]
fn a_reinspected_finished_avatar_stays_finished_until_its_scene_changes() {
    let h = Harness::new();
    let run = assembled(&h);
    assert_eq!(h.engine.get("n3-test").unwrap().phase, "succeeded");
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    let checked = h.engine.get("n3-test").unwrap();
    assert_eq!(checked.phase, "succeeded", "Unity still observes the assembled Avatar");
    assert!(checked.confirmed_digest.is_none());
    fs::write(Path::new(&run.project_root).join("Assets/state"), "player-change").unwrap();
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    assert_eq!(h.engine.get("n3-test").unwrap().phase, "ready", "a changed scene goes back to planning");
}
#[test]
fn an_sdk_fix_runs_only_offered_items_behind_a_restore_point_and_keeps_the_phase() {
    let h = Harness::new();
    let run = assembled(&h);
    let snapshots = h.platform.snapshots.load(Ordering::SeqCst);
    for refused in [vec![], vec!["This avatar is shorter than average.".to_owned()], vec!["Unknown item".to_owned()],
        vec![PIPELINE_FIX.to_owned(), PIPELINE_FIX.to_owned()], vec![PIPELINE_FIX.to_owned(); 17]] {
        assert!(!h.job(move |engine, ctx| engine.sdk_fix("n3-test", refused, ctx)));
    }
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots, "a refused request takes no restore point");
    assert!(h.job(|engine, ctx| engine.sdk_fix("n3-test", vec![PIPELINE_FIX.to_owned()], ctx)));
    let fixed = h.engine.get("n3-test").unwrap();
    assert_eq!(fixed.phase, "succeeded", "the finished Avatar stays finished");
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots + 1);
    assert_eq!(fixed.result["facts"]["sdkFixesApplied"], serde_json::json!([PIPELINE_FIX]));
    let keys: Vec<&str> = fixed.steps.iter().rev().take(2).map(|step| step.key.as_str()).collect();
    assert_eq!(keys, ["sdk_fix", "snapshot"]);
    h.platform.fail.store(true, Ordering::SeqCst);
    assert!(!h.job(|engine, ctx| engine.sdk_fix("n3-test", vec![PIPELINE_FIX.to_owned()], ctx)));
    let failed = h.engine.get("n3-test").unwrap();
    assert_eq!(failed.recovery.as_deref(), Some("restored"));
    assert_eq!(fs::read_to_string(Path::new(&run.project_root).join("Assets/state")).unwrap(), "after");
}
#[test]
fn an_assembly_a_forced_end_interrupted_restores_in_one_step() {
    let h = Harness::new();
    h.prepared();
    let run = h.engine.get("n3-test").unwrap();
    let digest = plan_digest(&run, &selection()).unwrap();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-test", selection(), &digest, ctx)));
    // VUA ended in the middle of a second assembly: the run file still reads as assembling,
    // its assembly step is still running and the work project is half written.
    let file = h.root.join("production/runs/n3-test.json");
    let mut stored: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    stored["phase"] = serde_json::json!("assembling");
    let mut step = stored["steps"].as_array().unwrap().last().unwrap().clone();
    step["key"] = serde_json::json!("assembly");
    step["state"] = serde_json::json!("running");
    step["endedAt"] = serde_json::Value::Null;
    stored["steps"].as_array_mut().unwrap().push(step);
    fs::write(&file, serde_json::to_vec_pretty(&stored).unwrap()).unwrap();
    let state = Path::new(&run.project_root).join("Assets/state");
    fs::write(&state, "half-assembled").unwrap();

    let plan = h.engine.rollback_plan("n3-test").unwrap();
    assert_eq!(plan["currentChanged"], true);
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |engine, ctx| engine.rollback("n3-test", &digest, None, ctx)));
    let restored = h.engine.get("n3-test").unwrap();
    assert_eq!(fs::read_to_string(&state).unwrap(), "before", "the restore point taken before the assembly");
    assert_eq!(restored.phase, "ready");
    let interrupted = restored.steps.iter().rev().find(|step| step.key == "assembly").unwrap();
    assert_eq!(interrupted.state, "interrupted", "the stopped assembly is not recorded as succeeded");
}

fn face_tracking_ready(h: &Harness) -> String {
    let mut run = assembled(h);
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/unity-bridge/v5/examples/face-tracking.valid.result.json")).unwrap();
    let mut facts = fixture["data"]["faceTracking"].clone();
    facts["patchMenus"][0]["sourceId"] = serde_json::json!("avatar");
    facts["patchMenus"][0]["scriptPath"] = serde_json::json!("Assets/FtSyntheticMenu.cs");
    run.sources[0].asset_paths.push("Assets/FtSyntheticMenu.cs".into());
    run.result["facts"]["faceTracking"] = facts.clone();
    *h.platform.ft_facts.lock().unwrap() = Some(facts);
    fs::write(h.root.join("production/runs/n3-test.json"), serde_json::to_vec(&run).unwrap()).unwrap();
    h.engine.face_tracking_plan("n3-test", &"a".repeat(64)).unwrap()["planDigest"].as_str().unwrap().into()
}

#[test]
fn creator_patch_requires_exact_confirmation_and_observed_source_before_snapshot() {
    let h = Harness::new();
    let digest = face_tracking_ready(&h);
    let snapshots = h.platform.snapshots.load(Ordering::SeqCst);
    let review = digest.clone();
    assert_eq!(h.run_job(move |e, ctx| e.face_tracking_patch("n3-test", &"a".repeat(64), &review, false, ctx)).unwrap_err(),
        "vua.amf_production.confirmation_required");
    assert_eq!(h.run_job(|e, ctx| e.face_tracking_patch("n3-test", &"a".repeat(64), &"b".repeat(64), true, ctx)).unwrap_err(),
        "vua.amf_production.plan_drift");
    assert!(h.engine.face_tracking_plan("n3-test", "Tools/Unobserved").is_err());
    h.platform.editor_open.store(true, Ordering::SeqCst);
    assert!(h.run_job(move |e, ctx| e.face_tracking_patch("n3-test", &"a".repeat(64), &digest, true, ctx)).is_err());
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots);
}

#[test]
fn completed_creator_patch_rechecks_and_blocks_incomplete_replay_but_full_rollback_works() {
    let h = Harness::new();
    let digest = face_tracking_ready(&h);
    assert!(h.job(move |e, ctx| e.face_tracking_patch("n3-test", &"a".repeat(64), &digest, true, ctx)));
    let patched = h.engine.get("n3-test").unwrap();
    assert_eq!(patched.phase, "succeeded");
    assert_eq!(patched.result["facts"]["faceTracking"]["manualReplayRequired"], true);
    assert_eq!(patched.revisions[0].before, RevisionKind::FaceTracking);
    assert_eq!(h.engine.save_recipe("n3-test", "recipe-ft", "Incomplete").unwrap_err().code, "vua.amf_production.face_tracking_manual_replay");
    assert_eq!(h.engine.revision_plan("n3-test", &patched.revisions[0].id).unwrap_err().code, "vua.amf_production.face_tracking_manual_replay");
    assert!(h.engine.plan("n3-test", &selection()).is_err());
    assert!(h.engine.recipe_apply_plan("n3-test", "missing-recipe").is_err());
    let plan = h.engine.rollback_plan("n3-test").unwrap();
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |e, ctx| e.rollback("n3-test", &digest, None, ctx)));
    assert_eq!(h.engine.get("n3-test").unwrap().result["facts"]["faceTracking"]["manualReplayRequired"], false);
}

#[test]
fn pre_patch_copy_survives_later_fixes_and_a_reviewed_restore_returns_to_it() {
    let h = Harness::new();
    let digest = face_tracking_ready(&h);
    assert!(h.job(move |e, ctx| e.face_tracking_patch("n3-test", &"a".repeat(64), &digest, true, ctx)));
    let patched = h.engine.get("n3-test").unwrap();
    let pre_patch = patched.ft_pre_patch_snapshot.clone().expect("the patch records its pre-patch copy");
    assert_eq!(patched.snapshot_id.as_deref(), Some(pre_patch.as_str()),
        "a fresh patch keeps the pre-patch copy as the restore point");
    assert!(h.engine.rollback_plan("n3-test").unwrap()["prePatch"].is_null(),
        "no second target while the restore point already reaches the pre-patch state");

    // A later protected fix takes its own restore point; the pre-patch copy stays reachable.
    assert!(h.job(|e, ctx| e.sdk_fix("n3-test", vec![PIPELINE_FIX.to_owned()], ctx)));
    let fixed = h.engine.get("n3-test").unwrap();
    let fix_point = fixed.snapshot_id.clone().expect("the fix took its own restore point");
    assert_ne!(fix_point, pre_patch);
    assert_eq!(fixed.ft_pre_patch_snapshot.as_deref(), Some(pre_patch.as_str()));
    let keeps = h.platform.prune_keeps.lock().unwrap().last().cloned().unwrap();
    assert!(keeps.contains(&fix_point) && keeps.contains(&pre_patch),
        "pruning keeps both the fix point and the pre-patch copy: {keeps:?}");

    let plan = h.engine.rollback_plan("n3-test").unwrap();
    assert_eq!(plan["prePatch"]["snapshotId"].as_str(), Some(pre_patch.as_str()));
    let pre_digest = plan["prePatch"]["planDigest"].as_str().unwrap().to_owned();
    let default_digest = plan["planDigest"].as_str().unwrap().to_owned();
    assert!(!h.job(move |e, ctx| e.rollback("n3-test", &default_digest, Some(RollbackTarget::PreCreatorPatch), ctx)),
        "the default review's digest must not authorize the pre-patch target");
    assert!(h.job(move |e, ctx| e.rollback("n3-test", &pre_digest, Some(RollbackTarget::PreCreatorPatch), ctx)));
    let restored = h.engine.get("n3-test").unwrap();
    assert_eq!(restored.phase, "ready");
    assert!(restored.ft_pre_patch_snapshot.is_none(), "returning to the pre-patch state ends the retention");
    assert!(!Path::new(&restored.project_root).join("ProjectSettings/VUA.FaceTracking.json").exists());
    assert_eq!(restored.result["facts"]["faceTracking"]["manualReplayRequired"], false);

    // The patched state the return left behind stays the restore point, so the return is reversible.
    let again = h.engine.rollback_plan("n3-test").unwrap();
    assert!(again["prePatch"].is_null());
    let digest = again["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |e, ctx| e.rollback("n3-test", &digest, None, ctx)));
    let forward = h.engine.get("n3-test").unwrap();
    assert!(Path::new(&forward.project_root).join("ProjectSettings/VUA.FaceTracking.json").exists(),
        "the protected patched state returns with its marker");
}

#[test]
fn a_patch_after_the_marker_left_records_its_own_pre_patch_copy() {
    let h = Harness::new();
    let digest = face_tracking_ready(&h);
    assert!(h.job(move |e, ctx| e.face_tracking_patch("n3-test", &"a".repeat(64), &digest, true, ctx)));
    let mut run = h.engine.get("n3-test").unwrap();
    let stale = run.ft_pre_patch_snapshot.clone().expect("the first patch records its pre-patch copy");
    // A restore took the line back before the patch while the record still names the old copy.
    fs::remove_file(Path::new(&run.project_root).join("ProjectSettings/VUA.FaceTracking.json")).unwrap();
    run.result["facts"]["faceTracking"]["manualReplayRequired"] = serde_json::json!(false);
    fs::write(h.root.join("production/runs/n3-test.json"), serde_json::to_vec(&run).unwrap()).unwrap();
    assert!(h.engine.rollback_plan("n3-test").unwrap()["prePatch"].is_null());
    let digest: String = h.engine.face_tracking_plan("n3-test", &"a".repeat(64)).unwrap()["planDigest"].as_str().unwrap().into();
    assert!(h.job(move |e, ctx| e.face_tracking_patch("n3-test", &"a".repeat(64), &digest, true, ctx)));
    let patched = h.engine.get("n3-test").unwrap();
    assert_ne!(patched.ft_pre_patch_snapshot.as_deref(), Some(stale.as_str()), "the stale copy is not reused");
    assert_eq!(patched.ft_pre_patch_snapshot, patched.snapshot_id, "the new patch keeps the state right before it");
    let keeps = h.platform.prune_keeps.lock().unwrap().last().cloned().unwrap();
    assert!(!keeps.contains(&stale), "pruning drops the stale copy: {keeps:?}");
}

#[test]
fn patch_failure_missing_receipt_recheck_failure_target_drift_and_cancel_restore_marker_and_facts() {
    for outcome in [2, 3, 4, 5, 6] {
        let h = Harness::new();
        let digest = face_tracking_ready(&h);
        let before = h.engine.get("n3-test").unwrap();
        h.platform.ft_outcome.store(outcome, Ordering::SeqCst);
        assert!(!h.job(move |e, ctx| e.face_tracking_patch("n3-test", &"a".repeat(64), &digest, true, ctx)), "outcome {outcome}");
        let failed = h.engine.get("n3-test").unwrap();
        assert_eq!(failed.recovery.as_deref(), Some("restored"), "outcome {outcome}");
        assert_eq!(failed.result, before.result);
        assert_eq!(failed.revisions, before.revisions);
        assert!(!Path::new(&failed.project_root).join("ProjectSettings/VUA.FaceTracking.json").exists());
    }
}
#[test]
fn creator_helper_cleanup_must_be_verified_before_automatic_restore() {
    let h = Harness::new();
    let digest = face_tracking_ready(&h);
    h.platform.ft_outcome.store(7, Ordering::SeqCst);
    assert!(!h.job(move |e, ctx| e.face_tracking_patch("n3-test", &"a".repeat(64), &digest, true, ctx)));
    let failed = h.engine.get("n3-test").unwrap();
    assert_eq!(failed.recovery.as_deref(), Some("restore_failed: vua.amf_production.process_cleanup_unverified"));
    assert!(failed.snapshot_id.is_some());
    assert!(Path::new(&failed.project_root).join("ProjectSettings/VUA.FaceTracking.json").exists(),
        "automatic restore must not race a creator helper");
}
#[test]
fn interruption_during_the_post_patch_check_retains_the_explicit_restore_path() {
    let h = Harness::new();
    let digest = face_tracking_ready(&h);
    assert!(h.job(move |e, ctx| e.face_tracking_patch("n3-test", &"a".repeat(64), &digest, true, ctx)));
    let mut interrupted = h.engine.get("n3-test").unwrap();
    interrupted.phase = "assembling".into();
    let step = interrupted.steps.last_mut().unwrap();
    assert_eq!(step.key, "inspect");
    step.state = "running".into();
    step.ended_at = None;
    fs::write(h.root.join("production/runs/n3-test.json"), serde_json::to_vec(&interrupted).unwrap()).unwrap();
    assert_eq!(h.engine.get("n3-test").unwrap().phase, "assembling",
        "the stored state is preserved; the provider projects it as interrupted");
    assert!(Path::new(&interrupted.project_root).join("ProjectSettings/VUA.FaceTracking.json").exists(),
        "reading an interrupted run must neither rerun the menu nor restore files");
    let plan = h.engine.rollback_plan("n3-test").unwrap();
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |e, ctx| e.rollback("n3-test", &digest, None, ctx)));
    assert_eq!(h.engine.get("n3-test").unwrap().result["facts"]["faceTracking"]["manualReplayRequired"], false);
}
#[test]
fn a_run_that_never_changed_in_unity_keeps_asking_for_a_new_check() {
    let h = Harness::new();
    h.prepared();
    let file = h.root.join("production/runs/n3-test.json");
    let mut stored: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    stored["phase"] = serde_json::json!("inspecting");
    fs::write(&file, serde_json::to_vec_pretty(&stored).unwrap()).unwrap();
    assert_eq!(h.engine.rollback_plan("n3-test").unwrap_err().code, "vua.amf_production.inspect_required");
}
#[test]
fn failure_restores_and_requires_explicit_reinspection() {
    let h = Harness::new();
    h.prepared();
    let run = h.engine.get("n3-test").unwrap();
    let digest = plan_digest(&run, &selection()).unwrap();
    h.platform.fail.store(true, Ordering::SeqCst);
    assert!(!h.job(move |engine, ctx| engine.assemble("n3-test", selection(), &digest, ctx)));
    let failed = h.engine.get("n3-test").unwrap();
    assert_eq!(failed.phase, "failed");
    assert_eq!(failed.recovery.as_deref(), Some("restored"));
    assert_eq!(
        fs::read_to_string(Path::new(&run.project_root).join("Assets/state")).unwrap(),
        "before"
    );
    assert!(h.engine.plan("n3-test", &selection()).is_err());
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    assert_eq!(h.engine.get("n3-test").unwrap().phase, "ready");
}
#[test]
fn targets_cannot_be_substituted_by_an_unobserved_prefab_or_an_avatar_as_clothing() {
    let h = Harness::new();
    h.prepared();
    let run = h.engine.get("n3-test").unwrap();
    let mut invalid = selection();
    invalid.outfits[0].prefab_path = "Assets/Avatar.prefab".into();
    assert!(validate_selection(&run, &invalid).is_err());
    invalid = selection();
    invalid.avatar_prefab = "Assets/Unobserved.prefab".into();
    assert!(validate_selection(&run, &invalid).is_err());
    invalid = selection();
    invalid.outfits[0].mode = BindingMode::AttachToBone;
    invalid.outfits[0].bone = "Head".into();
    assert!(validate_selection(&run, &invalid).is_err());
}
#[test]
fn terminal_record_does_not_keep_an_inflight_step() {
    let h = Harness::new();
    h.prepared();
    let digest = plan_digest(&h.engine.get("n3-test").unwrap(), &selection()).unwrap();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-test", selection(), &digest, ctx)));
    let records = fs::read_dir(h.root.join("production/records"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records.len(), 1);
    let record: ProductionRun =
        serde_json::from_slice(&fs::read(records[0].path()).unwrap()).unwrap();
    assert_eq!(record.phase, "succeeded");
    assert!(record.steps.iter().all(|s| s.state == "succeeded"));
}
#[test]
fn traversal_and_duplicate_inputs_never_create_work_projects() {
    let h = Harness::new();
    assert!(h.engine.get("../outside").is_err());
    assert!(validate_choices(&[
        SourceChoice {
            source_id: "same".into(),
            role: SourceRole::Auto
        },
        SourceChoice {
            source_id: "same".into(),
            role: SourceRole::Auto
        }
    ])
    .is_err());
}

fn assembled(h: &Harness) -> ProductionRun {
    h.prepared();
    let digest = plan_digest(&h.engine.get("n3-test").unwrap(), &selection()).unwrap();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-test", selection(), &digest, ctx)));
    h.engine.get("n3-test").unwrap()
}

fn restore_reviewed(h: &Harness) -> bool {
    let plan = h.engine.rollback_plan("n3-test").unwrap();
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    h.job(move |engine, ctx| engine.rollback("n3-test", &digest, None, ctx))
}

#[test]
fn user_restore_retains_current_edits_and_can_itself_be_undone() {
    let h = Harness::new();
    let run = assembled(&h);
    let state = Path::new(&run.project_root).join("Assets/state");
    fs::write(&state, "player-adjustments-after-assembly").unwrap();
    assert_eq!(h.engine.rollback_plan("n3-test").unwrap()["currentChanged"], true);
    assert!(restore_reviewed(&h));
    let restored = h.engine.get("n3-test").unwrap();
    assert_eq!(fs::read_to_string(&state).unwrap(), "before");
    assert_eq!(restored.phase, "ready");
    assert_eq!(restored.snapshot_id.as_deref(), Some("player-adjustments-after-assembly"));
    assert_eq!(restored.selection, None);
    assert_eq!(restored.snapshot_selection, Some(selection()));
    assert!(restored.confirmed_digest.is_none());
    assert!(restored.revision > run.revision);
    // A new reviewed restore returns to the actual edited state, not merely
    // the last VUA-produced result. The original materials remain unrelated.
    assert!(restore_reviewed(&h));
    assert_eq!(fs::read_to_string(&state).unwrap(), "player-adjustments-after-assembly");
    assert_eq!(h.engine.get("n3-test").unwrap().selection, Some(selection()));
}

#[test]
fn restore_rejects_drift_and_old_confirmations_before_snapshot_or_mutation() {
    let h = Harness::new();
    let run = assembled(&h);
    let plan = h.engine.rollback_plan("n3-test").unwrap();
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    let count = h.platform.snapshots.load(Ordering::SeqCst);
    let state = Path::new(&run.project_root).join("Assets/state");
    fs::write(&state, "newer-unreviewed-change").unwrap();
    assert!(!h.job(move |engine, ctx| engine.rollback("n3-test", &digest, None, ctx)));
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), count);
    assert_eq!(fs::read_to_string(&state).unwrap(), "newer-unreviewed-change");
    let digest = h.engine.rollback_plan("n3-test").unwrap()["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    assert!(!h.job(move |engine, ctx| engine.rollback("n3-test", &digest, None, ctx)));
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), count);
}

#[test]
fn partial_restore_and_failed_verification_recover_the_protected_current_state() {
    for fail_inspection in [false, true] {
        let h = Harness::new();
        let run = assembled(&h);
        let state = Path::new(&run.project_root).join("Assets/state");
        fs::write(&state, "latest-player-work").unwrap();
        h.platform.fail_inspect.store(fail_inspection, Ordering::SeqCst);
        h.platform.fail_restore.store(!fail_inspection, Ordering::SeqCst);
        assert!(!restore_reviewed(&h));
        let failed = h.engine.get("n3-test").unwrap();
        assert_eq!(fs::read_to_string(&state).unwrap(), "latest-player-work");
        assert_eq!(failed.phase, "failed");
        assert_eq!(failed.recovery.as_deref(), Some("restored"));
        assert_eq!(failed.snapshot_id.as_deref(), Some("latest-player-work"));
        assert!(failed.result.is_null());
        assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    }
}

#[test]
fn preparation_without_a_snapshot_does_not_offer_a_restore() {
    let h = Harness::new();
    h.prepared();
    assert!(h.engine.rollback_plan("n3-test").is_err());
}

#[test]
fn recipes_require_current_observations_and_reopen_with_pinned_inputs() {
    let h = Harness::new(); h.prepared();
    assert!(h.engine.save_recipe("n3-test", "recipe-test", "My Avatar").is_err());
    let plan = h.engine.plan("n3-test", &selection()).unwrap(); let digest = plan["planDigest"].as_str().unwrap().to_owned();
    // The fixture Avatar carries 8+1 asset bits plus one embedded MA bit; the Coat adds one more.
    assert_eq!(plan["parameterBudget"]["state"], "fits");
    assert_eq!(plan["parameterBudget"]["estimateBits"], 11);
    assert_eq!(plan["parameterBudget"]["limitBits"], 256);
    assert!(h.job(move |engine, ctx| engine.assemble("n3-test", selection(), &digest, ctx)));
    let saved = h.engine.save_recipe("n3-test", "recipe-test", "My Avatar").unwrap();
    assert_eq!(h.engine.get_recipe("recipe-test").unwrap(), saved);
    assert_eq!(saved.recipe["materials"][0]["sha256"].as_str().unwrap().len(), 64);
    assert!(!saved.recipe.to_string().contains(&h.root.to_string_lossy().to_string()));
    h.engine.begin_recipe("n3-recipe", "recipe-test", "synthetic-editor").unwrap();
    assert!(h.job(|engine, ctx| engine.prepare("n3-recipe", ctx)));
    let reproduced = h.engine.get("n3-recipe").unwrap();
    assert_eq!(reproduced.phase, "ready"); assert_eq!(reproduced.selection, Some(selection()));
    assert_eq!(reproduced.package_pins["nadena.dev.modular-avatar"], "1.18.7");
    h.engine.begin_recipe("n3-recipe-drift", "recipe-test", "synthetic-editor").unwrap();
    h.platform.change_recipe_input.store(true, Ordering::SeqCst);
    assert!(!h.job(|engine, ctx| engine.prepare("n3-recipe-drift", ctx)));
    let drift = h.engine.get("n3-recipe-drift").unwrap();
    assert!(drift.error.unwrap().contains("recipe_material_changed"));
    assert!(!drift.steps.iter().any(|step| step.key == "inspect"), "Changed Recipe originals must be refused before Unity inspection");
    fs::write(h.root.join("production/projects/n3-test/Assets/state"), "external edit").unwrap();
    assert_eq!(h.engine.save_recipe("n3-test", "recipe-stale", "Stale").unwrap_err().code, "vua.amf_production.inspect_required");
    let restarted = AmfProductionEngine::new(h.root.join("production"), h.platform.clone());
    assert_eq!(restarted.get_recipe("recipe-test").unwrap(), saved);
}

#[test]
fn sharing_supplements_only_referenced_inputs_and_remembers_confirmed_choices() {
    use std::collections::BTreeMap;
    use vua_orchestrator::amf_recipe_share::SharedRecipe;
    let h = Harness::new(); assembled(&h);
    h.engine.save_recipe("n3-test", "recipe-share", "A shareable Avatar").unwrap();
    let plan = h.engine.recipe_share_plan("recipe-share").unwrap();
    assert_eq!(plan.materials.len(), 2); assert!(plan.materials.iter().all(|m| m.selected_product_id.is_none()));
    assert!(h.engine.export_recipe("recipe-share", &plan.plan_digest, &BTreeMap::new()).is_err());
    let choices = plan.materials.iter().map(|m| (m.material_id.clone(), "1000001".into())).collect::<BTreeMap<_,_>>();
    let mut foreign = choices.clone(); foreign.insert(format!("m-{}", "c".repeat(64)), "1000002".into());
    assert!(h.engine.export_recipe("recipe-share", &plan.plan_digest, &foreign).is_err());
    assert!(h.engine.recipe_share_plan("recipe-share").unwrap().materials.iter().all(|m| m.selected_product_id.is_none()));
    let export = h.engine.export_recipe("recipe-share", &plan.plan_digest, &choices).unwrap();
    assert_eq!(SharedRecipe::decode(&export.share_code).unwrap(), SharedRecipe::decode(&export.file_text).unwrap());
    assert!(!export.file_text.contains(&h.root.to_string_lossy().to_string()));
    assert!(!export.file_text.contains("GlobalObjectId")); assert!(!export.file_text.contains("sourceId"));
    let restarted = AmfProductionEngine::new(h.root.join("production"), h.platform.clone());
    let remembered = restarted.recipe_share_plan("recipe-share").unwrap();
    assert!(remembered.materials.iter().all(|m| m.selected_product_id.as_deref() == Some("1000001")));
    assert!(restarted.export_recipe("recipe-share", &plan.plan_digest, &BTreeMap::new()).is_err(), "An obsolete source review cannot be replayed");
    restarted.export_recipe("recipe-share", &remembered.plan_digest, &BTreeMap::new()).unwrap();
    restarted.save_recipe("n3-test", "recipe-another", "Another combination").unwrap();
    assert!(restarted.recipe_share_plan("recipe-another").unwrap().materials.iter().all(|m| m.selected_product_id.is_some()));
    let recipient = Harness::new();
    let imported = recipient.engine.import_recipe(&export.share_code).unwrap();
    assert_eq!(recipient.engine.import_recipe(&export.file_text).unwrap(), imported);
    assert!(recipient.engine.list().unwrap().is_empty(), "Importing a declaration never prepares or edits a Unity project");
    assert_eq!(imported.recipe["outfits"], export.document.recipe.outfits.into_iter().map(|o| serde_json::to_value(o).unwrap()).collect::<serde_json::Value>());
    let id = imported.recipe["recipeId"].as_str().unwrap();
    recipient.engine.begin_recipe("n3-imported", id, "synthetic-editor").unwrap();
    assert!(recipient.job(|engine, ctx| engine.prepare("n3-imported", ctx)));
    assert_eq!(recipient.engine.get("n3-imported").unwrap().selection, Some(selection()));
}

#[test]
fn recipe_application_binds_current_facts_and_recovers_failed_mutations() {
    use vua_orchestrator::amf_recipe_apply::RecipeApplyChoice;
    let h=Harness::new();h.prepared();
    let digest=plan_digest(&h.engine.get("n3-test").unwrap(),&selection()).unwrap();
    assert!(h.job(move |engine,ctx| engine.assemble("n3-test",selection(),&digest,ctx)));
    h.engine.save_recipe("n3-test","recipe-apply","Retained recipe").unwrap();
    let plan=h.engine.recipe_apply_plan("n3-test","recipe-apply").unwrap();
    let snapshots=h.platform.snapshots.load(Ordering::SeqCst);
    assert_eq!(h.engine.get("n3-test").unwrap().phase,"succeeded","Review does not mutate the run");
    assert!(h.job(|engine,ctx|engine.reinspect("n3-test",ctx)));
    assert!(!h.job(move|engine,ctx|engine.apply_recipe("n3-test","recipe-apply",RecipeApplyChoice::RecipeWins,&plan.plan_digest,ctx)));
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst),snapshots,"Stale comparisons fail before a snapshot or any write");
    let plan=h.engine.recipe_apply_plan("n3-test","recipe-apply").unwrap();
    assert!(h.job(move|engine,ctx|engine.apply_recipe("n3-test","recipe-apply",RecipeApplyChoice::AvatarWins,&plan.plan_digest,ctx)));
    assert_eq!(h.engine.get("n3-test").unwrap().result["facts"]["avatarGlobalObjectId"],"synthetic-avatar");
    let plan=h.engine.recipe_apply_plan("n3-test","recipe-apply").unwrap();
    h.platform.fail.store(true,Ordering::SeqCst);
    assert!(!h.job(move|engine,ctx|engine.apply_recipe("n3-test","recipe-apply",RecipeApplyChoice::NewAvatar,&plan.plan_digest,ctx)));
    let run=h.engine.get("n3-test").unwrap();assert_eq!(run.recovery.as_deref(),Some("restored"));
    assert_eq!(h.platform.fingerprint(Path::new(&run.project_root)).unwrap(),"after");
}

fn flow_schema(name: &str) -> jsonschema::Validator {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/amf-production-flow/v0.1").join(name);
    jsonschema::validator_for(&serde_json::from_slice(&fs::read(path).unwrap()).unwrap()).unwrap()
}

#[test]
fn progress_and_status_vectors_accept_stage_facts_and_refuse_log_text() {
    for (schema, folder) in [("progress.schema.json", "progress-examples"), ("status.schema.json", "status-examples")] {
        let validator = flow_schema(schema);
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/amf-production-flow/v0.1").join(folder);
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            let value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            let valid = path.file_name().unwrap().to_string_lossy().contains(".valid.");
            assert_eq!(validator.is_valid(&value), valid, "{}", path.display());
        }
    }
}

/// Starts a job and returns its task identity and a receiver for its outcome.
fn started(
    h: &Harness,
    action: impl FnOnce(&AmfProductionEngine, &TaskContext) -> Result<(), AppErrorV1> + Send + 'static,
) -> (String, std::sync::mpsc::Receiver<Result<(), AppErrorV1>>) {
    let engine = h.engine.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let accepted = h.runtime.submit(SubmitRequest {
        correlation_id: None,
        timeout: None,
        job: Box::new(move |ctx| {
            let result = action(&engine, ctx);
            tx.send(result.clone()).unwrap();
            result.map(|()| TaskExit::Done(serde_json::json!({"synthetic":true})))
        }),
    }).unwrap();
    (accepted.task_id, rx)
}

/// Waits for the job's first Unity progress event and returns its payload.
fn unity_event(events: &std::sync::mpsc::Receiver<vua_orchestrator::TaskEventV1>) -> serde_json::Value {
    loop {
        let event = events.recv_timeout(Duration::from_secs(5)).expect("Unity progress");
        if event.kind == vua_orchestrator::TaskEventKind::Progress && event.payload.get("unity").is_some() {
            return event.payload;
        }
    }
}

#[test]
fn a_stopped_unity_step_keeps_the_project_and_continue_reinspects_it() {
    let h = Harness::new();
    h.prepared();
    let prepared = h.engine.get("n3-test").unwrap();
    let state = Path::new(&prepared.project_root).join("Assets/state");
    let events = h.runtime.subscribe();
    h.platform.wait_for_stop.store(true, Ordering::SeqCst);
    let (task, outcome) = started(&h, |engine, ctx| engine.reinspect("n3-test", ctx));

    let payload = unity_event(&events);
    assert!(flow_schema("progress.schema.json").is_valid(&payload), "{payload}");
    assert_eq!((payload["step"].as_str(), payload["unity"]["stage"].as_str()), (Some("reinspect"), Some("importing")));
    let running = h.engine.get("n3-test").unwrap();
    assert_eq!(running.phase, "inspecting");
    let progress = h.engine.progress(&running).expect("the running step's progress");
    assert_eq!(progress["step"], "reinspect");
    assert_eq!(progress["unity"]["total"], 379);
    assert!(progress["elapsedSeconds"].as_u64().is_some());
    assert!(progress["usualSeconds"].is_null(), "no earlier reinspection finished on this computer");

    h.runtime.cancel(&task).unwrap();
    let error = outcome.recv_timeout(Duration::from_secs(5)).unwrap().unwrap_err();
    assert_eq!(error.code, "vua.task.cancelled");
    let stopped = h.engine.get("n3-test").unwrap();
    assert_eq!(stopped.phase, "stopped");
    assert_eq!(stopped.error, None);
    let step = stopped.steps.last().unwrap();
    assert_eq!((step.key.as_str(), step.state.as_str()), ("reinspect", "stopped"));
    assert!(step.ended_at.is_some());
    assert!(h.engine.progress(&stopped).is_none(), "a stopped run has no running step");
    assert_eq!(fs::read_to_string(&state).unwrap(), "before", "the work project stays");
    assert_eq!(stopped.sources.len(), 2, "the imported materials stay");
    assert!(h.engine.rollback_plan("n3-test").is_err(), "preparation alone has no restore point");

    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)), "continue reinspects the retained project");
    let continued = h.engine.get("n3-test").unwrap();
    assert_eq!(continued.phase, "ready");
    assert_eq!(continued.steps.last().unwrap().state, "succeeded");
    assert!(h.engine.plan("n3-test", &selection()).is_ok());
}

#[test]
fn stopping_assembly_restores_the_protected_project() {
    let h = Harness::new();
    h.prepared();
    let run = h.engine.get("n3-test").unwrap();
    let digest = plan_digest(&run, &selection()).unwrap();
    let events = h.runtime.subscribe();
    h.platform.wait_for_stop.store(true, Ordering::SeqCst);
    let (task, outcome) = started(&h, move |engine, ctx| engine.assemble("n3-test", selection(), &digest, ctx));
    assert_eq!(unity_event(&events)["step"], "assembly");
    h.runtime.cancel(&task).unwrap();
    assert!(outcome.recv_timeout(Duration::from_secs(5)).unwrap().is_err());
    let stopped = h.engine.get("n3-test").unwrap();
    assert_eq!(stopped.phase, "stopped");
    assert_eq!(stopped.recovery.as_deref(), Some("restored"));
    assert_eq!(fs::read_to_string(Path::new(&run.project_root).join("Assets/state")).unwrap(), "before");
    assert!(h.engine.plan("n3-test", &selection()).is_err(), "a stopped assembly needs reinspection before a new plan");
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    assert_eq!(h.engine.get("n3-test").unwrap().phase, "ready");
}

#[test]
fn steps_record_their_times_and_usual_durations_take_the_median_of_finished_steps() {
    let h = Harness::new();
    h.prepared();
    let run = h.engine.get("n3-test").unwrap();
    assert_eq!(run.steps.iter().map(|s| s.key.as_str()).collect::<Vec<_>>(), ["project", "materials", "inspect"]);
    let millis = |text: &Option<String>| text.as_deref().and_then(vua_orchestrator::rfc3339_millis).unwrap();
    for step in &run.steps {
        assert!(millis(&step.started_at) <= millis(&step.ended_at));
    }
    let mut view = serde_json::to_value(&run).unwrap();
    view["usualDurations"] = serde_json::to_value(h.engine.usual_durations()).unwrap();
    view["progress"] = serde_json::Value::Null;
    assert!(flow_schema("status.schema.json").is_valid(&view), "{view}");
    let usual = h.engine.usual_durations();
    assert_eq!(usual.keys().map(String::as_str).collect::<Vec<_>>(), ["inspect", "materials", "project"]);

    // Earlier runs on this computer: inspections of 60, 300 and 120 seconds, and steps that do
    // not count — a failed one, a stopped one and one recorded before step times existed.
    let record = |id: &str, steps: serde_json::Value| {
        let mut other = serde_json::to_value(&run).unwrap();
        other["runId"] = id.into();
        other["projectRoot"] = h.root.join("production/projects").join(id).to_string_lossy().into_owned().into();
        other["steps"] = steps;
        fs::write(h.root.join("production/runs").join(format!("{id}.json")), serde_json::to_vec(&other).unwrap()).unwrap();
    };
    let step = |state: &str, start: &str, end: &str| serde_json::json!({"key":"inspect","name":"Inspect","state":state,"detail":"",
        "startedAt":format!("2026-10-17T{start}.000Z"),"endedAt":format!("2026-10-17T{end}.000Z")});
    record("n3-earlier-1", serde_json::json!([step("succeeded", "08:00:00", "08:01:00"), step("failed", "09:00:00", "10:00:00")]));
    record("n3-earlier-2", serde_json::json!([step("succeeded", "08:00:00", "08:05:00"), step("stopped", "09:00:00", "09:30:00")]));
    record("n3-earlier-3", serde_json::json!([step("succeeded", "08:00:00", "08:02:00"), {"key":"inspect","name":"Inspect","state":"succeeded","detail":""}]));
    // This run's own inspection took well under a minute, so it is the shortest of four.
    assert_eq!(h.engine.usual_durations()["inspect"], 90, "the median of four finished inspections");
    record("n3-earlier-4", serde_json::json!([step("succeeded", "08:00:00", "08:10:00")]));
    assert_eq!(h.engine.usual_durations()["inspect"], 120, "a new record replaces the cached medians");
    assert!(h.engine.get("n3-earlier-3").unwrap().steps[1].started_at.is_none(), "records without step times still read");
}

#[test]
fn every_step_the_engine_records_is_in_the_published_status_schema() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let schema: serde_json::Value = serde_json::from_str(&fs::read_to_string(root.join("../../schemas/amf-production-flow/v0.1/status.schema.json")).unwrap()).unwrap();
    let text = schema.to_string();
    let mut sources = vec![root.join("src/amf_production.rs")];
    sources.extend(fs::read_dir(root.join("src/amf_production")).unwrap().map(|entry| entry.unwrap().path()));
    let marker = "self.step(&mut run, ctx, \"";
    let mut keys = Vec::new();
    for source in sources {
        let code = fs::read_to_string(source).unwrap();
        for part in code.split(marker).skip(1) {
            keys.push(part.split('"').next().unwrap().to_owned());
        }
    }
    assert!(keys.len() >= 8, "found the engine's steps: {keys:?}");
    for key in keys {
        assert!(text.contains(&format!("\"{key}\"")), "status.schema.json lists the step key {key}");
    }
}

#[test]
fn a_v1_record_compares_by_computing_v1_once_and_then_carries_the_current_value() {
    let h = Harness::new();
    h.prepared();
    let record = h.root.join("production/runs/n3-test.json");
    let carry = |value: &str| {
        let mut run: serde_json::Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
        run["projectFingerprint"] = value.into();
        run["snapshotId"] = "n3-snap".into();
        fs::write(&record, serde_json::to_vec_pretty(&run).unwrap()).unwrap();
    };
    // A record from before the cached fingerprint, on a project nobody touched.
    carry("sha256:before");
    let plan = h.engine.rollback_plan("n3-test").unwrap();
    assert_eq!(plan["currentChanged"], false, "a new algorithm must not read as a changed project");
    // The gate passes and upgrades the record; this synthetic run has not been assembled,
    // so building the Recipe itself declines for its own reason.
    let refused = h.engine.save_recipe("n3-test", "recipe-migrated", "Migrated").unwrap_err();
    assert_ne!(refused.code, "vua.amf_production.inspect_required", "the fingerprint gate accepted the legacy value");
    assert_eq!(h.engine.get("n3-test").unwrap().project_fingerprint, "before", "the record carries the current value after the match");
    // A record whose project really changed still reports the change, in either algorithm.
    carry("sha256:before");
    fs::write(h.root.join("production/projects/n3-test/Assets/state"), "external edit").unwrap();
    assert_eq!(h.engine.rollback_plan("n3-test").unwrap()["currentChanged"], true);
    assert_eq!(h.engine.save_recipe("n3-test", "recipe-stale", "Stale").unwrap_err().code, "vua.amf_production.inspect_required");
}

#[test]
fn the_after_unity_project_check_shows_as_its_own_stage() {
    let h = Harness::new();
    h.prepared();
    let events = h.runtime.subscribe();
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    let mut stages = Vec::new();
    while let Ok(event) = events.try_recv() {
        if event.kind == vua_orchestrator::TaskEventKind::Progress
            && event.payload.get("unity").is_some()
        {
            stages.push(event.payload["unity"]["stage"].clone());
        }
    }
    assert_eq!(stages, vec![serde_json::json!("fingerprint")], "the check is a named stage, and the only Unity stage of a synthetic reinspection");
}

fn choices(ids: &[&str]) -> Vec<SourceChoice> {
    ids.iter().map(|id| SourceChoice { source_id: (*id).into(), role: SourceRole::Auto }).collect()
}
fn ids(sources: &[PreparedSource]) -> Vec<&str> {
    sources.iter().map(|source| source.source_id.as_str()).collect()
}

#[test]
fn a_line_changes_its_materials_in_place_and_its_restore_point_brings_them_back() {
    let h = Harness::new();
    let run = assembled(&h);
    assert_eq!(run.phase, "succeeded");
    let snapshots = h.platform.snapshots.load(Ordering::SeqCst);
    // The same materials, or one no longer in the library, change nothing.
    assert!(!h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["outfit", "avatar"]), ctx)));
    assert!(!h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["avatar", "gone"]), ctx)));
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots, "a refused change takes no restore point");

    assert!(h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["avatar", "outfit2"]), ctx)));
    let changed = h.engine.get("n3-test").unwrap();
    assert_eq!(changed.project_root, run.project_root, "the line keeps its work project");
    assert_eq!(changed.phase, "ready", "the line plans again");
    assert!(changed.completed_at.is_none());
    assert_eq!(ids(&changed.sources), ["avatar", "outfit2"]);
    assert_eq!(fs::read_to_string(Path::new(&run.project_root).join("Assets/state")).unwrap(), "materials:avatar+outfit2");
    let kept = changed.selection.clone().expect("the Avatar choice stays");
    assert_eq!(kept.avatar_prefab, "Assets/Avatar.prefab");
    assert!(kept.outfits.is_empty(), "an outfit of a removed material leaves the plan");
    let keys: Vec<&str> = changed.steps.iter().rev().take(3).map(|step| step.key.as_str()).collect();
    assert_eq!(keys, ["inspect", "materials", "snapshot"]);
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots + 1);

    // Restoring brings the earlier project and its materials back together.
    assert!(restore_reviewed(&h));
    let restored = h.engine.get("n3-test").unwrap();
    assert_eq!(ids(&restored.sources), ["avatar", "outfit"]);
    assert_eq!(restored.choices, run.choices);
    assert_eq!(fs::read_to_string(Path::new(&run.project_root).join("Assets/state")).unwrap(), "after");
    // And the restore is itself reversible, materials included.
    assert!(restore_reviewed(&h));
    assert_eq!(ids(&h.engine.get("n3-test").unwrap().sources), ["avatar", "outfit2"]);
}

#[test]
fn a_failed_material_change_restores_the_project_and_the_materials() {
    let h = Harness::new();
    let run = assembled(&h);
    h.platform.fail_replace.store(true, Ordering::SeqCst);
    assert!(!h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["avatar", "outfit2"]), ctx)));
    let failed = h.engine.get("n3-test").unwrap();
    assert_eq!(failed.recovery.as_deref(), Some("restored"));
    assert_eq!(failed.phase, "failed");
    assert_eq!(ids(&failed.sources), ["avatar", "outfit"]);
    assert_eq!(failed.choices, run.choices);
    assert_eq!(fs::read_to_string(Path::new(&run.project_root).join("Assets/state")).unwrap(), "after");
}

#[test]
fn deleting_a_line_removes_its_work_project_and_records() {
    let h = Harness::new();
    let run = assembled(&h);
    let release = h.root.join("production/releases/n3-test.json");
    fs::create_dir_all(release.parent().unwrap()).unwrap();
    fs::write(&release, "{}").unwrap();
    h.platform.editor_open.store(true, Ordering::SeqCst);
    assert!(h.engine.delete_run("n3-test").unwrap_err().code.ends_with("project_open"));
    assert!(Path::new(&run.project_root).exists(), "a project Unity holds stays");
    h.platform.editor_open.store(false, Ordering::SeqCst);
    let records = h.root.join("production/records");
    let own: Vec<_> = fs::read_dir(&records).unwrap().flatten().map(|entry| entry.path()).collect();
    assert!(!own.is_empty(), "the assembly left its record");
    let other = records.join("n3-other-0123456789abcdef.json");
    fs::write(&other, "{}").unwrap();
    h.engine.delete_run("n3-test").unwrap();
    assert!(!Path::new(&run.project_root).exists());
    assert!(!release.exists());
    assert!(own.iter().all(|record| !record.exists()), "the line's assembly records leave with it");
    assert!(other.exists(), "another line's record stays");
    assert!(h.engine.get("n3-test").unwrap_err().code.ends_with("run_not_found"));
    assert!(h.engine.list().unwrap().is_empty());
}

#[test]
fn work_on_a_project_unity_holds_is_refused_when_asked() {
    let h = Harness::new();
    assembled(&h);
    h.engine.precheck_closed("n3-test").unwrap();
    h.platform.editor_open.store(true, Ordering::SeqCst);
    assert!(h.engine.precheck_closed("n3-test").unwrap_err().code.ends_with("project_open"));
    assert!(h.engine.precheck_closed("n3-missing").unwrap_err().code.ends_with("run_not_found"));
}

// --- lilToon brightness limit alignment ---

/// A synthetic lilToon material with the two brightness limit lines.
fn liltoon_mat(name: &str, min: &str, max: &str, newline: &str) -> Vec<u8> {
    [
        "%YAML 1.1".to_owned(),
        "%TAG !u! tag:unity3d.com,2011:".to_owned(),
        "--- !u!21 &2100000".to_owned(),
        "Material:".to_owned(),
        "  serializedVersion: 8".to_owned(),
        format!("  m_Name: {name}"),
        "  m_Shader: {fileID: 4800000, guid: DF12117ECD77C31469C224178886498E, type: 3}".to_owned(),
        "  m_SavedProperties:".to_owned(),
        "    serializedVersion: 3".to_owned(),
        "    m_Floats:".to_owned(),
        format!("    - _LightMaxLimit: {max}"),
        format!("    - _LightMinLimit: {min}"),
        "    - _lilToonVersion: 44".to_owned(),
        "    m_TexEnvs: []".to_owned(),
        "    m_Colors: []".to_owned(),
    ]
    .join(newline)
    .into_bytes()
}

/// A synthetic non-lilToon material: no version marker, no lilToon limit lines.
fn standard_mat(name: &str) -> Vec<u8> {
    format!(
        "%YAML 1.1\n--- !u!21 &2100000\nMaterial:\n  m_Name: {name}\n  m_Shader: {{fileID: 46, guid: 0000000000000000f000000000000000, type: 0}}\n  m_SavedProperties:\n    m_Floats: []\n"
    )
    .into_bytes()
}

fn write_mat(project_root: &str, relative: &str, bytes: &[u8]) {
    let path = Path::new(project_root).join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// Assembles the run, then lets the platform report this material use and refreshes the facts.
fn assembled_with_material_use(h: &Harness, uses: serde_json::Value) -> ProductionRun {
    let run = assembled(h);
    assert_eq!(run.phase, "succeeded");
    *h.platform.material_use.lock().unwrap() = uses;
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    h.engine.get("n3-test").unwrap()
}

fn coat_entry() -> serde_json::Value {
    serde_json::json!({"material":"Assets/Outfit/Coat.mat","sourceId":"outfit",
        "current":{"min":"0.05","max":"1"},"proposed":{"min":"0.1","max":"1"}})
}

#[test]
fn lighting_plan_uses_the_body_renderers_pair_and_apply_changes_only_the_two_lines() {
    for newline in ["\n", "\r\n"] {
        let h = Harness::new();
        let run = assembled_with_material_use(&h, serde_json::json!([
            {"owner":"avatar","renderer":"Body","material":"Assets/Avatar/Body.mat"},
            {"owner":"avatar","renderer":"Hair","material":"Assets/Avatar/Hair.mat"},
            {"owner":"outfit","renderer":"Outfit/Coat","material":"Assets/Outfit/Coat.mat"},
        ]));
        let project = run.project_root.clone();
        write_mat(&project, "Assets/Avatar/Body.mat", &liltoon_mat("Body", "0.1", "1", newline));
        write_mat(&project, "Assets/Avatar/Hair.mat", &liltoon_mat("Hair", "0.1", "1", newline));
        let coat = liltoon_mat("Coat", "0.05", "1", newline);
        write_mat(&project, "Assets/Outfit/Coat.mat", &coat);

        let plan = h.engine.plan_lighting("n3-test").unwrap();
        assert_eq!(plan["reference"], serde_json::json!({"min":"0.1","max":"1"}));
        assert_eq!(plan["referenceReason"], serde_json::Value::Null);
        assert_eq!(plan["entries"], serde_json::json!([coat_entry()]));
        assert_eq!(plan["skipped"], serde_json::json!([]));
        let digest = plan["planDigest"].as_str().unwrap().to_owned();
        assert!(h.job(move |engine, ctx| engine.apply_lighting("n3-test", &digest, &["Assets/Outfit/Coat.mat".to_owned()], ctx)));

        let updated = fs::read(Path::new(&project).join("Assets/Outfit/Coat.mat")).unwrap();
        assert_eq!(updated, liltoon_mat("Coat", "0.1", "1", newline), "only the lower-limit line changed, {newline:?} endings intact");
        let run = h.engine.get("n3-test").unwrap();
        assert_eq!(run.phase, "succeeded");
        assert!(run.snapshot_id.is_some(), "a restore point protects the change");
        let keys: Vec<&str> = run.steps.iter().rev().take(3).map(|step| step.key.as_str()).collect();
        assert_eq!(keys, ["inspect", "lighting", "snapshot"]);
        assert_eq!(run.result["facts"]["materialUse"][2]["material"], "Assets/Outfit/Coat.mat", "the recheck refreshed the facts");
    }
}

#[test]
fn lighting_plan_prefers_body_over_the_majority_and_falls_back_to_it() {
    // A Body pair different from every other Avatar material wins over the majority.
    let h = Harness::new();
    let run = assembled_with_material_use(&h, serde_json::json!([
        {"owner":"avatar","renderer":"Body","material":"Assets/Avatar/Body.mat"},
        {"owner":"avatar","renderer":"Hair","material":"Assets/Avatar/Hair.mat"},
        {"owner":"avatar","renderer":"Face","material":"Assets/Avatar/Face.mat"},
        {"owner":"outfit","renderer":"Outfit/Coat","material":"Assets/Outfit/Coat.mat"},
    ]));
    write_mat(&run.project_root, "Assets/Avatar/Body.mat", &liltoon_mat("Body", "0.15", "0.9", "\n"));
    write_mat(&run.project_root, "Assets/Avatar/Hair.mat", &liltoon_mat("Hair", "0.1", "1", "\n"));
    write_mat(&run.project_root, "Assets/Avatar/Face.mat", &liltoon_mat("Face", "0.1", "1", "\n"));
    write_mat(&run.project_root, "Assets/Outfit/Coat.mat", &liltoon_mat("Coat", "0.05", "1", "\n"));
    let plan = h.engine.plan_lighting("n3-test").unwrap();
    assert_eq!(plan["reference"], serde_json::json!({"min":"0.15","max":"0.9"}));
    assert_eq!(plan["entries"][0]["proposed"], serde_json::json!({"min":"0.15","max":"0.9"}));

    // Without a Body renderer the most common pair wins, one vote per Avatar material.
    let h = Harness::new();
    let run = assembled_with_material_use(&h, serde_json::json!([
        {"owner":"avatar","renderer":"Hair","material":"Assets/Avatar/Hair.mat"},
        {"owner":"avatar","renderer":"Face","material":"Assets/Avatar/Face.mat"},
        {"owner":"avatar","renderer":"Eye","material":"Assets/Avatar/Eye.mat"},
        {"owner":"outfit","renderer":"Outfit/Coat","material":"Assets/Outfit/Coat.mat"},
    ]));
    write_mat(&run.project_root, "Assets/Avatar/Hair.mat", &liltoon_mat("Hair", "0.1", "1", "\n"));
    write_mat(&run.project_root, "Assets/Avatar/Face.mat", &liltoon_mat("Face", "0.1", "1", "\n"));
    write_mat(&run.project_root, "Assets/Avatar/Eye.mat", &liltoon_mat("Eye", "0.2", "1", "\n"));
    write_mat(&run.project_root, "Assets/Outfit/Coat.mat", &liltoon_mat("Coat", "0.05", "1", "\n"));
    let plan = h.engine.plan_lighting("n3-test").unwrap();
    assert_eq!(plan["reference"], serde_json::json!({"min":"0.1","max":"1"}));
    assert_eq!(plan["entries"], serde_json::json!([coat_entry()]));
}

#[test]
fn lighting_plan_has_no_entries_without_a_unique_reference() {
    // A tied vote.
    let h = Harness::new();
    let run = assembled_with_material_use(&h, serde_json::json!([
        {"owner":"avatar","renderer":"Hair","material":"Assets/Avatar/Hair.mat"},
        {"owner":"avatar","renderer":"Face","material":"Assets/Avatar/Face.mat"},
        {"owner":"outfit","renderer":"Outfit/Coat","material":"Assets/Outfit/Coat.mat"},
    ]));
    write_mat(&run.project_root, "Assets/Avatar/Hair.mat", &liltoon_mat("Hair", "0.1", "1", "\n"));
    write_mat(&run.project_root, "Assets/Avatar/Face.mat", &liltoon_mat("Face", "0.2", "1", "\n"));
    write_mat(&run.project_root, "Assets/Outfit/Coat.mat", &liltoon_mat("Coat", "0.05", "1", "\n"));
    let plan = h.engine.plan_lighting("n3-test").unwrap();
    assert_eq!(plan["reference"], serde_json::Value::Null);
    assert_eq!(plan["referenceReason"], "tie");
    assert_eq!(plan["entries"], serde_json::json!([]));

    // An Avatar without lilToon materials.
    let h = Harness::new();
    let run = assembled_with_material_use(&h, serde_json::json!([
        {"owner":"avatar","renderer":"Body","material":"Assets/Avatar/Body.mat"},
        {"owner":"outfit","renderer":"Outfit/Coat","material":"Assets/Outfit/Coat.mat"},
    ]));
    write_mat(&run.project_root, "Assets/Avatar/Body.mat", &standard_mat("Body"));
    write_mat(&run.project_root, "Assets/Outfit/Coat.mat", &liltoon_mat("Coat", "0.05", "1", "\n"));
    let plan = h.engine.plan_lighting("n3-test").unwrap();
    assert_eq!(plan["reference"], serde_json::Value::Null);
    assert_eq!(plan["referenceReason"], "no_avatar_liltoon");
    assert_eq!(plan["entries"], serde_json::json!([]));
}

#[test]
fn lighting_plan_skips_shared_and_ignores_non_liltoon_materials() {
    let h = Harness::new();
    let run = assembled_with_material_use(&h, serde_json::json!([
        {"owner":"avatar","renderer":"Body","material":"Assets/Avatar/Body.mat"},
        {"owner":"avatar","renderer":"Hair","material":"Assets/Shared/Shared.mat"},
        {"owner":"outfit","renderer":"Outfit/Trim","material":"Assets/Shared/Shared.mat"},
        {"owner":"outfit","renderer":"Outfit/Coat","material":"Assets/Outfit/Coat.mat"},
        {"owner":"outfit","renderer":"Outfit/Cap","material":"Assets/Outfit/Cap.mat"},
    ]));
    write_mat(&run.project_root, "Assets/Avatar/Body.mat", &liltoon_mat("Body", "0.1", "1", "\n"));
    // The shared material still informs the Avatar's reference through its Body-less renderers.
    write_mat(&run.project_root, "Assets/Shared/Shared.mat", &liltoon_mat("Shared", "0.05", "0.9", "\n"));
    write_mat(&run.project_root, "Assets/Outfit/Coat.mat", &liltoon_mat("Coat", "0.05", "1", "\n"));
    write_mat(&run.project_root, "Assets/Outfit/Cap.mat", &standard_mat("Cap"));
    let plan = h.engine.plan_lighting("n3-test").unwrap();
    assert_eq!(plan["reference"], serde_json::json!({"min":"0.1","max":"1"}));
    assert_eq!(plan["entries"], serde_json::json!([coat_entry()]));
    assert_eq!(
        plan["skipped"],
        serde_json::json!([{"material":"Assets/Shared/Shared.mat","reason":"shared_with_avatar"}]),
        "a material the Avatar and an outfit share is never changed"
    );
    assert!(!plan.to_string().contains("Cap.mat"), "a non-lilToon material never appears");
}

#[test]
fn lighting_needs_an_assembled_undrifted_line_with_material_use_facts() {
    let h = Harness::new();
    h.prepared();
    assert_eq!(h.engine.plan_lighting("n3-test").unwrap_err().code, "vua.amf_production.result_not_ready");
    let digest = plan_digest(&h.engine.get("n3-test").unwrap(), &selection()).unwrap();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-test", selection(), &digest, ctx)));
    let run = h.engine.get("n3-test").unwrap();
    assert_eq!(
        h.engine.plan_lighting("n3-test").unwrap_err().code,
        "vua.amf_production.inspect_required",
        "a run checked before the Bridge reported material use asks for reinspection"
    );
    // A fresh check carries the facts; a project changed since then replans instead.
    *h.platform.material_use.lock().unwrap() = serde_json::json!([]);
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    fs::write(Path::new(&run.project_root).join("Assets/state"), "external edit").unwrap();
    assert_eq!(h.engine.plan_lighting("n3-test").unwrap_err().code, "vua.amf_production.project_drift");
}

#[test]
fn lighting_apply_refuses_a_stale_digest_and_unknown_materials_before_any_change() {
    let h = Harness::new();
    let run = assembled_with_material_use(&h, serde_json::json!([
        {"owner":"avatar","renderer":"Body","material":"Assets/Avatar/Body.mat"},
        {"owner":"outfit","renderer":"Outfit/Coat","material":"Assets/Outfit/Coat.mat"},
    ]));
    write_mat(&run.project_root, "Assets/Avatar/Body.mat", &liltoon_mat("Body", "0.1", "1", "\n"));
    let coat = liltoon_mat("Coat", "0.05", "1", "\n");
    write_mat(&run.project_root, "Assets/Outfit/Coat.mat", &coat);
    let plan = h.engine.plan_lighting("n3-test").unwrap();
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    let snapshots = h.platform.snapshots.load(Ordering::SeqCst);

    assert!(!h.job(|engine, ctx| engine.apply_lighting("n3-test", &"f".repeat(64), &["Assets/Outfit/Coat.mat".to_owned()], ctx)));
    // Materials outside the reviewed entries, duplicates and an empty choice are refused too.
    let reviewed = digest.clone();
    assert!(!h.job(move |engine, ctx| engine.apply_lighting("n3-test", &reviewed, &["Assets/Avatar/Body.mat".to_owned()], ctx)));
    let reviewed = digest.clone();
    assert!(!h.job(move |engine, ctx| engine.apply_lighting("n3-test", &reviewed, &[], ctx)));
    let reviewed = digest.clone();
    assert!(!h.job(move |engine, ctx| engine.apply_lighting("n3-test", &reviewed, &["Assets/Outfit/Coat.mat".to_owned(), "Assets/Outfit/Coat.mat".to_owned()], ctx)));
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots, "refusals happen before any restore point");
    assert_eq!(fs::read(Path::new(&run.project_root).join("Assets/Outfit/Coat.mat")).unwrap(), coat);
    assert_eq!(h.engine.get("n3-test").unwrap().phase, "succeeded");

    // A project change between review and execution replans instead of executing.
    fs::write(Path::new(&run.project_root).join("Assets/state"), "player-change").unwrap();
    assert!(!h.job(move |engine, ctx| engine.apply_lighting("n3-test", &digest, &["Assets/Outfit/Coat.mat".to_owned()], ctx)));
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots);
    assert_eq!(fs::read(Path::new(&run.project_root).join("Assets/Outfit/Coat.mat")).unwrap(), coat);
}

#[test]
fn a_failed_recheck_restores_the_written_materials() {
    let h = Harness::new();
    let run = assembled_with_material_use(&h, serde_json::json!([
        {"owner":"avatar","renderer":"Body","material":"Assets/Avatar/Body.mat"},
        {"owner":"outfit","renderer":"Outfit/Coat","material":"Assets/Outfit/Coat.mat"},
    ]));
    write_mat(&run.project_root, "Assets/Avatar/Body.mat", &liltoon_mat("Body", "0.1", "1", "\n"));
    let coat = liltoon_mat("Coat", "0.05", "1", "\n");
    let coat_path = Path::new(&run.project_root).join("Assets/Outfit/Coat.mat");
    write_mat(&run.project_root, "Assets/Outfit/Coat.mat", &coat);
    // The synthetic restore point brings the original file bytes back, as the real one does.
    h.platform.restore_files.lock().unwrap().push((coat_path.clone(), coat.clone()));
    let digest = h.engine.plan_lighting("n3-test").unwrap()["planDigest"].as_str().unwrap().to_owned();
    h.platform.fail_inspect.store(true, Ordering::SeqCst);
    assert!(!h.job(move |engine, ctx| engine.apply_lighting("n3-test", &digest, &["Assets/Outfit/Coat.mat".to_owned()], ctx)));
    let failed = h.engine.get("n3-test").unwrap();
    assert_eq!(failed.phase, "failed");
    assert_eq!(failed.recovery.as_deref(), Some("restored"));
    assert_eq!(fs::read(&coat_path).unwrap(), coat, "the failure restored the written material");
    // The run keeps the observations that match the restored project.
    assert_eq!(failed.result["facts"], run.result["facts"]);
    assert_eq!(failed.unity_fingerprint, run.unity_fingerprint);
}

#[test]
fn lighting_apply_accepts_a_reviewed_subset_and_the_plan_stays_reproducible() {
    let h = Harness::new();
    let run = assembled_with_material_use(&h, serde_json::json!([
        {"owner":"avatar","renderer":"Body","material":"Assets/Avatar/Body.mat"},
        {"owner":"outfit","renderer":"Outfit/Coat","material":"Assets/Outfit/Coat.mat"},
        {"owner":"outfit","renderer":"Outfit/Skirt","material":"Assets/Outfit/Skirt.mat"},
    ]));
    write_mat(&run.project_root, "Assets/Avatar/Body.mat", &liltoon_mat("Body", "0.1", "1", "\n"));
    write_mat(&run.project_root, "Assets/Outfit/Coat.mat", &liltoon_mat("Coat", "0.05", "1", "\n"));
    write_mat(&run.project_root, "Assets/Outfit/Skirt.mat", &liltoon_mat("Skirt", "0.05", "0.8", "\n"));
    let plan = h.engine.plan_lighting("n3-test").unwrap();
    assert_eq!(plan["entries"].as_array().unwrap().len(), 2);
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |engine, ctx| engine.apply_lighting("n3-test", &digest, &["Assets/Outfit/Coat.mat".to_owned()], ctx)));
    // The chosen material changed; the other entry keeps its limits, and the next review
    // proposes only what still differs.
    assert_eq!(fs::read(Path::new(&run.project_root).join("Assets/Outfit/Coat.mat")).unwrap(), liltoon_mat("Coat", "0.1", "1", "\n"));
    assert_eq!(fs::read(Path::new(&run.project_root).join("Assets/Outfit/Skirt.mat")).unwrap(), liltoon_mat("Skirt", "0.05", "0.8", "\n"));
    let next = h.engine.plan_lighting("n3-test").unwrap();
    assert_eq!(next["entries"].as_array().unwrap().len(), 1);
    assert_eq!(next["entries"][0]["material"], "Assets/Outfit/Skirt.mat");
    assert_eq!(next["entries"][0]["proposed"], serde_json::json!({"min":"0.1","max":"1"}));
    assert!(h.engine.rollback_plan("n3-test").is_ok(), "the restore point stays compatible with the workshop's restore");
}

fn alt_selection() -> AssemblySelection {
    AssemblySelection { avatar_prefab: "Assets/AvatarAlt.prefab".into(), ..selection() }
}

#[test]
fn another_base_on_a_finished_line_rebuilds_it_behind_a_restore_point() {
    let h = Harness::new();
    h.platform.alt_avatar.store(true, Ordering::SeqCst);
    let before = assembled(&h);
    assert_eq!(before.phase, "succeeded");
    assert_eq!(h.engine.plan("n3-test", &selection()).unwrap()["rebuild"], false);
    let plan = h.engine.plan("n3-test", &alt_selection()).unwrap();
    assert_eq!(plan["rebuild"], true);
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    let snapshots = h.platform.snapshots.load(Ordering::SeqCst);
    assert!(h.job(move |engine, ctx| engine.assemble("n3-test", alt_selection(), &digest, ctx)));
    let rebuilt = h.engine.get("n3-test").unwrap();
    assert_eq!(rebuilt.phase, "succeeded");
    assert_eq!(h.platform.cleared.load(Ordering::SeqCst), 1);
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots + 1);
    assert_eq!(h.platform.assembled_with.lock().unwrap().last(), Some(&alt_selection()));
    assert_eq!(rebuilt.selection, Some(alt_selection()));
    assert_eq!(rebuilt.snapshot_selection, Some(selection()));
    assert_eq!(rebuilt.result["facts"]["observedSelection"]["avatarPrefab"], "Assets/AvatarAlt.prefab");
    let keys: Vec<_> = rebuilt.steps.iter().rev().take(3).map(|step| step.key.as_str()).collect();
    assert_eq!(keys, ["assembly", "inspect", "snapshot"]);
    // The restore point brings the earlier base back.
    assert!(restore_reviewed(&h));
    let restored = h.engine.get("n3-test").unwrap();
    assert_eq!(restored.selection, Some(selection()));
    assert_eq!(fs::read_to_string(Path::new(&restored.project_root).join("Assets/state")).unwrap(), "after");
}

#[test]
fn a_failed_rebuild_restores_the_earlier_base_and_its_facts() {
    let h = Harness::new();
    h.platform.alt_avatar.store(true, Ordering::SeqCst);
    let before = assembled(&h);
    let digest = h.engine.plan("n3-test", &alt_selection()).unwrap()["planDigest"].as_str().unwrap().to_owned();
    h.platform.fail.store(true, Ordering::SeqCst);
    assert!(!h.job(move |engine, ctx| engine.assemble("n3-test", alt_selection(), &digest, ctx)));
    let failed = h.engine.get("n3-test").unwrap();
    assert_eq!(failed.recovery.as_deref(), Some("restored"));
    assert_eq!(fs::read_to_string(Path::new(&failed.project_root).join("Assets/state")).unwrap(), "after");
    assert_eq!(failed.result["facts"]["observedSelection"], before.result["facts"]["observedSelection"]);
    assert_eq!(failed.unity_fingerprint, before.unity_fingerprint);
    assert_eq!(failed.project_fingerprint, before.project_fingerprint);
}

// --- Line version records and restores by replay ---

fn selection2() -> AssemblySelection {
    AssemblySelection {
        avatar_prefab: "Assets/Avatar.prefab".into(),
        outfits: vec![OutfitSelection {
            source_id: "outfit2".into(),
            prefab_path: "Assets/Outfit2.prefab".into(),
            mode: BindingMode::MergeArmature,
            bone: "".into(),
            toggle_name: "Dress".into(),
            active: true,
        }],
    }
}

fn restore_reviewed_revision(h: &Harness, revision_id: &str) -> bool {
    let plan = h.engine.revision_plan("n3-test", revision_id).unwrap();
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    let revision_id = revision_id.to_owned();
    h.job(move |engine, ctx| engine.restore_revision("n3-test", &revision_id, &digest, ctx))
}

#[test]
fn version_records_replay_the_materials_and_selection_of_a_finished_line() {
    let h = Harness::new();
    let run = assembled(&h);
    assert_eq!(run.revisions.len(), 1);
    assert_eq!(run.revisions[0].before, RevisionKind::Assembly);
    // The version records the prepared line before its first assembly.
    assert_eq!(run.revisions[0].state.selection, None);
    assert_eq!(run.revisions[0].summary.sources, vec!["avatar", "outfit"]);
    assert_eq!(run.revisions[0].summary.avatar, None);
    assert!(!run.revisions[0].edited_in_unity);

    // The line changes its outfit and assembles again — the real-machine failure scenario.
    assert!(h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["avatar", "outfit2"]), ctx)));
    let digest = plan_digest(&h.engine.get("n3-test").unwrap(), &selection2()).unwrap();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-test", selection2(), &digest, ctx)));
    let changed = h.engine.get("n3-test").unwrap();
    assert_eq!(ids(&changed.sources), ["avatar", "outfit2"]);
    assert_eq!(changed.revisions.len(), 3);
    assert_eq!(changed.revisions[0].before, RevisionKind::Assembly);
    assert_eq!(changed.revisions[1].before, RevisionKind::Materials);
    assert_eq!(changed.revisions[1].state.choices, choices(&["avatar", "outfit"]));
    assert_eq!(changed.revisions[1].state.sources[1].sha256, format!("sha256:{}", "b".repeat(64)));

    // Restoring the version taken before the material change replays it: the earlier
    // materials and the earlier assembly come back.
    let plan = h.engine.revision_plan("n3-test", &changed.revisions[1].id).unwrap();
    assert_eq!(plan["editedInUnity"], false);
    assert_eq!(plan["steps"][0], serde_json::json!({"key":"materials","skipped":false}));
    assert_eq!(plan["steps"][1], serde_json::json!({"key":"assembly","skipped":false,"rebuild":false}));
    assert!(restore_reviewed_revision(&h, &changed.revisions[1].id.clone()));
    let restored = h.engine.get("n3-test").unwrap();
    assert_eq!(ids(&restored.sources), ["avatar", "outfit"]);
    assert_eq!(restored.choices, run.choices);
    assert_eq!(restored.selection, Some(selection()));
    assert_eq!(restored.phase, "succeeded");
    assert_eq!(h.platform.assembled_with.lock().unwrap().last(), Some(&selection()));

    // The restore recorded its own version first, so it is itself reversible.
    assert_eq!(restored.revisions[0].before, RevisionKind::Restore);
    assert!(restore_reviewed_revision(&h, &restored.revisions[0].id.clone()));
    let back = h.engine.get("n3-test").unwrap();
    assert_eq!(ids(&back.sources), ["avatar", "outfit2"]);
    assert_eq!(back.selection, Some(selection2()));
}

#[test]
fn a_restore_warns_when_unity_edited_the_project_and_still_replays() {
    let h = Harness::new();
    let run = assembled(&h);
    assert!(h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["avatar", "outfit2"]), ctx)));
    let target = h.engine.get("n3-test").unwrap().revisions[0].id.clone();
    // The player edited the project in Unity since VUA last checked it.
    fs::write(Path::new(&run.project_root).join("Assets/state"), "player-edit").unwrap();
    let plan = h.engine.revision_plan("n3-test", &target).unwrap();
    assert_eq!(plan["editedInUnity"], true);
    assert!(plan["warning"].as_str().is_some());
    assert!(restore_reviewed_revision(&h, &target));
    let restored = h.engine.get("n3-test").unwrap();
    assert_eq!(ids(&restored.sources), ["avatar", "outfit"]);
    assert!(restored.revisions[0].edited_in_unity, "the undo version records the foreign edits");
}

#[test]
fn a_restore_writes_back_the_recorded_lighting_values() {
    let h = Harness::new();
    let run = assembled_with_material_use(&h, serde_json::json!([
        {"owner":"avatar","renderer":"Body","material":"Assets/Avatar/Body.mat"},
        {"owner":"outfit","renderer":"Outfit/Coat","material":"Assets/Outfit/Coat.mat"},
    ]));
    write_mat(&run.project_root, "Assets/Avatar/Body.mat", &liltoon_mat("Body", "0.1", "1", "\n"));
    write_mat(&run.project_root, "Assets/Outfit/Coat.mat", &liltoon_mat("Coat", "0.05", "1", "\n"));
    // Align the outfit's lighting, then change to another outfit and assemble it: the version
    // taken before the material change carries the aligned values.
    let digest = h.engine.plan_lighting("n3-test").unwrap()["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |engine, ctx| engine.apply_lighting("n3-test", &digest, &["Assets/Outfit/Coat.mat".to_owned()], ctx)));
    assert!(h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["avatar", "outfit2"]), ctx)));
    let digest = h.engine.plan("n3-test", &selection2()).unwrap()["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-test", selection2(), &digest, ctx)));
    let changed = h.engine.get("n3-test").unwrap();
    let target = changed.revisions.iter().find(|revision| revision.before == RevisionKind::Materials).unwrap().clone();
    let coat = target.state.lighting.iter().find(|entry| entry.material == "Assets/Outfit/Coat.mat").unwrap();
    assert_eq!((coat.min.as_str(), coat.max.as_str()), ("0.1", "1"), "the aligned values are recorded");
    assert_eq!(target.summary.lighting_materials, 2);

    // The material drifts afterwards; the restore writes the recorded values back.
    write_mat(&run.project_root, "Assets/Outfit/Coat.mat", &liltoon_mat("Coat", "0.05", "1", "\n"));
    let plan = h.engine.revision_plan("n3-test", &target.id).unwrap();
    assert_eq!(plan["steps"][2]["skipped"], false);
    assert!(restore_reviewed_revision(&h, &target.id.clone()));
    let restored = h.engine.get("n3-test").unwrap();
    assert_eq!(fs::read(Path::new(&run.project_root).join("Assets/Outfit/Coat.mat")).unwrap(), liltoon_mat("Coat", "0.1", "1", "\n"));
    assert_eq!(restored.selection, Some(selection()));
    assert_eq!(restored.phase, "succeeded");
}

#[test]
fn version_records_are_bounded_and_the_oldest_leave() {
    let h = Harness::new();
    assembled(&h);
    for round in 0..REVISION_LIMIT {
        let next = if round % 2 == 0 { choices(&["avatar", "outfit2"]) } else { choices(&["avatar", "outfit"]) };
        assert!(h.job(move |engine, ctx| engine.change_sources("n3-test", next, ctx)));
    }
    let run = h.engine.get("n3-test").unwrap();
    assert_eq!(run.revisions.len(), REVISION_LIMIT);
    assert!(run.revisions.iter().all(|revision| revision.before == RevisionKind::Materials),
        "the first assembly's version was the oldest and left");
    let unique: std::collections::BTreeSet<&str> = run.revisions.iter().map(|revision| revision.id.as_str()).collect();
    assert_eq!(unique.len(), REVISION_LIMIT);
}

#[test]
fn a_failed_change_restored_automatically_leaves_no_version() {
    let h = Harness::new();
    assembled(&h);
    let before = h.engine.get("n3-test").unwrap().revisions.len();
    h.platform.fail_replace.store(true, Ordering::SeqCst);
    assert!(!h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["avatar", "outfit2"]), ctx)));
    let failed = h.engine.get("n3-test").unwrap();
    assert_eq!(failed.recovery.as_deref(), Some("restored"));
    assert_eq!(failed.revisions.len(), before, "the restored change recorded nothing");
}

#[test]
fn only_the_referenced_full_copy_survives_a_protected_change() {
    let h = Harness::new();
    let run = assembled(&h);
    let snapshots = Path::new(&run.project_root).join(".vua/snapshots");
    // The restore point the next change takes is named "after" by this platform.
    for name in ["before", "after", "n3-stale"] {
        fs::create_dir_all(snapshots.join(name)).unwrap();
    }
    fs::write(snapshots.join("note.txt"), b"stray").unwrap();
    assert!(h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["avatar", "outfit2"]), ctx)));
    assert!(!snapshots.join("before").exists(), "an unreferenced copy is pruned");
    assert!(!snapshots.join("n3-stale").exists());
    assert!(snapshots.join("after").exists(), "the referenced copy stays");
    assert!(snapshots.join("note.txt").exists(), "a foreign file is left in place");
    let run = h.engine.get("n3-test").unwrap();
    assert_eq!(run.phase, "ready", "a pruning problem never fails the change");
    let log: serde_json::Value = serde_json::from_slice(&fs::read(h.root.join("production/snapshot-prune.json")).unwrap()).unwrap();
    assert!(log.as_array().unwrap().iter().any(|entry| entry["diagnostic"].as_str().unwrap().contains("note.txt")));
}

#[test]
fn a_restore_whose_review_went_stale_is_refused_before_any_change() {
    let h = Harness::new();
    let run = assembled(&h);
    assert!(h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["avatar", "outfit2"]), ctx)));
    let target = h.engine.get("n3-test").unwrap().revisions[0].id.clone();
    let digest = h.engine.revision_plan("n3-test", &target).unwrap()["planDigest"].as_str().unwrap().to_owned();
    // The line changed after the review.
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    let id = target.clone();
    let stale = digest.clone();
    assert_eq!(h.run_job(move |engine, ctx| engine.restore_revision("n3-test", &id, &stale, ctx)).unwrap_err(),
        "vua.amf_production.plan_drift");
    assert_eq!(fs::read_to_string(Path::new(&run.project_root).join("Assets/state")).unwrap(), "materials:avatar+outfit2");
    // A version discarded between review and execution is refused as missing.
    let digest = h.engine.revision_plan("n3-test", &target).unwrap()["planDigest"].as_str().unwrap().to_owned();
    let file = h.root.join("production/runs/n3-test.json");
    let mut stored: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    stored.as_object_mut().unwrap().remove("revisions");
    fs::write(&file, serde_json::to_vec_pretty(&stored).unwrap()).unwrap();
    assert_eq!(h.run_job(move |engine, ctx| engine.restore_revision("n3-test", &target, &digest, ctx)).unwrap_err(),
        "vua.amf_production.revision_missing");
}

#[test]
fn a_restore_refuses_materials_whose_content_changed_in_the_library() {
    let h = Harness::new();
    assembled(&h);
    assert!(h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["avatar", "outfit2"]), ctx)));
    let target = h.engine.get("n3-test").unwrap().revisions[0].id.clone();
    // The library's copy of a recorded original changed.
    h.platform.change_recipe_input.store(true, Ordering::SeqCst);
    let error = h.engine.revision_plan("n3-test", &target).unwrap_err();
    assert_eq!(error.code, "vua.amf_production.revision_source_missing");
    let detail = error.params.as_ref().and_then(|params| params.get("detail")).map(|value| format!("{value:?}")).unwrap_or_default();
    assert!(detail.contains("outfit.unitypackage"), "{detail}");
    // And one the library no longer holds at all.
    h.platform.change_recipe_input.store(false, Ordering::SeqCst);
    let file = h.root.join("production/runs/n3-test.json");
    let mut stored: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    stored["revisions"][0]["state"]["sources"][1]["sourceId"] = serde_json::json!("gone");
    fs::write(&file, serde_json::to_vec_pretty(&stored).unwrap()).unwrap();
    assert_eq!(h.engine.revision_plan("n3-test", &target).unwrap_err().code, "vua.amf_production.revision_source_missing");
}

#[test]
fn a_record_from_before_version_records_reads_with_one_synthesized_version() {
    let h = Harness::new();
    let run = assembled(&h);
    let file = h.root.join("production/runs/n3-test.json");
    let mut stored: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    stored.as_object_mut().unwrap().remove("revisions");
    fs::write(&file, serde_json::to_vec_pretty(&stored).unwrap()).unwrap();
    let read = h.engine.get("n3-test").unwrap();
    assert_eq!(read.revisions.len(), 1);
    let legacy = &read.revisions[0];
    assert_eq!(legacy.before, RevisionKind::Assembly);
    assert_eq!(legacy.id, format!("rev-legacy-{}", run.snapshot_id.as_deref().unwrap()));
    assert_eq!(legacy.state.selection, run.snapshot_selection);
    assert!(legacy.state.sources.is_empty(), "a restore point without materials leaves them empty");
    assert!(legacy.state.lighting.is_empty() && legacy.state.sdk_fixes.is_empty());
    assert!(!legacy.edited_in_unity);
}

#[test]
fn a_recipe_records_adjustments_and_a_prepared_line_writes_them_after_its_first_assembly() {
    let h = Harness::new();
    let run = assembled_with_material_use(&h, serde_json::json!([
        {"owner":"avatar","renderer":"Body","material":"Assets/Avatar/Body.mat"},
        {"owner":"outfit","renderer":"Outfit/Coat","material":"Assets/Outfit/Coat.mat"},
    ]));
    write_mat(&run.project_root, "Assets/Avatar/Body.mat", &liltoon_mat("Body", "0.1", "1", "\n"));
    write_mat(&run.project_root, "Assets/Outfit/Coat.mat", &liltoon_mat("Coat", "0.1", "1", "\n"));
    assert!(h.job(|engine, ctx| engine.sdk_fix("n3-test", vec![PIPELINE_FIX.to_owned()], ctx)));
    let saved = h.engine.save_recipe("n3-test", "recipe-adjusted", "Adjusted").unwrap();
    assert_eq!(saved.recipe["adjustments"]["sdkFixes"], serde_json::json!([PIPELINE_FIX]));
    let lighting = saved.recipe["adjustments"]["lighting"].as_array().unwrap();
    assert_eq!(lighting.len(), 2);
    assert!(lighting.iter().all(|entry| entry["min"] == "0.1" && entry["max"] == "1"));

    // A line prepared from the Recipe lists the adjustments in its assembly plan and writes
    // them after the first successful assembly.
    h.engine.begin_recipe("n3-adjusted", "recipe-adjusted", "synthetic-editor").unwrap();
    assert!(h.job(|engine, ctx| engine.prepare("n3-adjusted", ctx)));
    let prepared = h.engine.get("n3-adjusted").unwrap();
    assert!(prepared.recipe_adjustments.is_some());
    // The new project's materials still carry their original values.
    write_mat(&prepared.project_root, "Assets/Avatar/Body.mat", &liltoon_mat("Body", "0.05", "0.9", "\n"));
    write_mat(&prepared.project_root, "Assets/Outfit/Coat.mat", &liltoon_mat("Coat", "0.05", "0.9", "\n"));
    let plan = h.engine.plan("n3-adjusted", &selection()).unwrap();
    assert_eq!(plan["adjustments"]["sdkFixes"], serde_json::json!([PIPELINE_FIX]));
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    let project = prepared.project_root.clone();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-adjusted", selection(), &digest, ctx)));
    let assembled = h.engine.get("n3-adjusted").unwrap();
    assert!(assembled.recipe_adjustments.is_none(), "the adjustments landed and are spent");
    assert_eq!(assembled.sdk_fixes_applied, vec![PIPELINE_FIX.to_owned()]);
    assert_eq!(fs::read(Path::new(&project).join("Assets/Outfit/Coat.mat")).unwrap(), liltoon_mat("Coat", "0.1", "1", "\n"));
    assert_eq!(fs::read(Path::new(&project).join("Assets/Avatar/Body.mat")).unwrap(), liltoon_mat("Body", "0.1", "1", "\n"));
    let keys: Vec<&str> = assembled.steps.iter().map(|step| step.key.as_str()).collect();
    assert!(keys.contains(&"lighting") && keys.contains(&"sdk_fix"), "the adjustments ran as their own steps: {keys:?}");
}

#[test]
fn a_restore_to_fewer_outfits_on_the_same_materials_starts_a_fresh_scene() {
    let h = Harness::new();
    assembled(&h);
    // The same materials carry a second outfit: assemble one, then add the other in place.
    assert!(h.job(|engine, ctx| engine.change_sources("n3-test", choices(&["avatar", "outfit", "outfit2"]), ctx)));
    let digest = plan_digest(&h.engine.get("n3-test").unwrap(), &selection()).unwrap();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-test", selection(), &digest, ctx)));
    let mut both = selection();
    both.outfits.extend(selection2().outfits);
    let digest = plan_digest(&h.engine.get("n3-test").unwrap(), &both).unwrap();
    let assemble_both = both.clone();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-test", assemble_both, &digest, ctx)));
    let run = h.engine.get("n3-test").unwrap();
    assert_eq!(run.selection, Some(both));
    let target = run.revisions[0].clone();
    assert_eq!(target.state.selection, Some(selection()));

    // An in-place assembly cannot drop the second outfit; the replay starts a fresh scene.
    let cleared = h.platform.cleared.load(Ordering::SeqCst);
    let plan = h.engine.revision_plan("n3-test", &target.id).unwrap();
    assert_eq!(plan["steps"][0]["skipped"], true);
    assert_eq!(plan["steps"][1], serde_json::json!({"key":"assembly","skipped":false,"rebuild":true}));
    assert!(restore_reviewed_revision(&h, &target.id), "{:?}", h.engine.get("n3-test").unwrap().error);
    let restored = h.engine.get("n3-test").unwrap();
    assert_eq!(restored.selection, Some(selection()));
    assert_eq!(restored.phase, "succeeded");
    assert_eq!(h.platform.cleared.load(Ordering::SeqCst), cleared + 1);
    assert_eq!(h.platform.assembled_with.lock().unwrap().last(), Some(&selection()));
}

#[test]
fn a_restore_to_the_version_before_the_first_assembly_leaves_a_fresh_scene() {
    let h = Harness::new();
    let run = assembled(&h);
    let target = run.revisions[0].clone();
    assert_eq!(target.state.selection, None);
    let plan = h.engine.revision_plan("n3-test", &target.id).unwrap();
    assert_eq!(plan["steps"][1], serde_json::json!({"key":"assembly","skipped":false,"rebuild":true}));
    let cleared = h.platform.cleared.load(Ordering::SeqCst);
    let assemblies = h.platform.assembled_with.lock().unwrap().len();
    assert!(restore_reviewed_revision(&h, &target.id));
    let restored = h.engine.get("n3-test").unwrap();
    assert_eq!(restored.phase, "ready");
    assert_eq!(restored.selection, None);
    assert_eq!(restored.completed_at, None);
    assert_eq!(h.platform.cleared.load(Ordering::SeqCst), cleared + 1);
    assert_eq!(h.platform.assembled_with.lock().unwrap().len(), assemblies, "nothing is assembled");
    // The restore's own version brings the assembly back.
    assert_eq!(restored.revisions[0].state.selection, Some(selection()));
    assert!(restore_reviewed_revision(&h, &restored.revisions[0].id.clone()));
    let back = h.engine.get("n3-test").unwrap();
    assert_eq!((back.phase.as_str(), back.selection), ("succeeded", Some(selection())));
}

// --- Saved fits: named local-Transform adjustments of a garment on one Avatar ---

fn fit_object(path: &str, x: f64) -> FitObject {
    FitObject {
        path: path.into(),
        local_position: FitVec3 { x, y: 0.0, z: 0.0 },
        local_rotation: FitQuat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 },
        local_scale: FitVec3 { x: 1.0, y: 1.0, z: 1.0 },
    }
}

fn write_baseline(h: &Harness, run_id: &str, objects: Vec<FitObject>) {
    let baseline = FitBaseline {
        schema_version: "0.1".into(),
        run_id: run_id.into(),
        garments: vec![BaselineGarment {
            source_id: "outfit".into(),
            owner_key: "outfit-0".into(),
            objects,
        }],
    };
    let dir = h.root.join("production/projects").join(run_id).join(".vua");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("amf-fit-baseline.json"), serde_json::to_vec(&baseline).unwrap()).unwrap();
}

fn report_changes(h: &Harness, moved: Vec<FitObject>, missing: Vec<String>) {
    *h.platform.fit_changes.lock().unwrap() =
        Some(serde_json::json!([{"sourceId":"outfit","moved":moved,"missing":missing}]));
}

fn write_changes_overlay(h: &Harness, run_id: &str, written_at: &str, moved: Vec<FitObject>, missing: Vec<String>) {
    let file = FitChangesFile {
        schema_version: "0.1".into(),
        run_id: run_id.into(),
        written_at: written_at.into(),
        fit_changes: vec![GarmentFitChanges { source_id: "outfit".into(), moved, missing }],
    };
    let dir = h.root.join("production/projects").join(run_id).join(".vua/bridge");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("amf-fit-changes.json"), serde_json::to_vec(&file).unwrap()).unwrap();
}

fn changes_digest_of(h: &Harness, run_id: &str) -> String {
    let run = h.engine.get(run_id).unwrap();
    let (changes, _) = h.engine.fit_view(&run);
    changes_digest(run_id, "outfit", changes.iter().find(|entry| entry.source_id == "outfit").expect("reported changes"))
}

/// The baseline, a changes report and a saved fit on an assembled line.
fn fitted(h: &Harness) -> StoredFit {
    assembled(h);
    write_baseline(h, "n3-test", vec![fit_object("Clip", 0.0)]);
    report_changes(h, vec![fit_object("Clip", 0.5)], vec![]);
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    let digest = changes_digest_of(h, "n3-test");
    h.engine.save_fit("n3-test", "outfit", "Clip fit", &digest).unwrap()
}

#[test]
fn fit_changes_overlay_and_save_fit_update_the_baseline_and_clear_the_report() {
    let h = Harness::new();
    assembled(&h);
    write_baseline(&h, "n3-test", vec![fit_object("Clip", 0.0), fit_object("Hook", 0.0)]);
    report_changes(&h, vec![fit_object("Clip", 0.5)], vec!["Hook".into()]);
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    let run = h.engine.get("n3-test").unwrap();
    assert_eq!(run.fit_changes.len(), 1);
    assert_eq!(run.fit_changes[0].missing, vec!["Hook"]);
    assert!(run.observed_at.is_some());
    let (changes, available) = h.engine.fit_view(&run);
    assert_eq!(changes[0].moved[0].local_position.x, 0.5);
    assert!(available.is_empty(), "no fit is stored yet");
    // The save hook's file overlays the recorded facts only when it is newer than the
    // last observation of the same line.
    write_changes_overlay(&h, "n3-test", "2000-01-01T00:00:00.0000000Z", vec![fit_object("Clip", 9.0)], vec![]);
    let (changes, _) = h.engine.fit_view(&run);
    assert_eq!(changes[0].moved[0].local_position.x, 0.5, "an older overlay loses to the observation");
    write_changes_overlay(&h, "n3-test", "2999-01-01T00:00:00.0000000Z", vec![fit_object("Clip", 0.25)], vec!["Hook".into()]);
    let (changes, _) = h.engine.fit_view(&run);
    assert_eq!(changes[0].moved[0].local_position.x, 0.25, "a newer overlay wins");
    let digest = changes_digest("n3-test", "outfit", &changes[0]);
    let fit = h.engine.save_fit("n3-test", "outfit", "Clip fit", &digest).unwrap();
    assert_eq!(fit.objects, vec![fit_object("Clip", 0.25)], "moved transforms folded into the baseline, missing dropped");
    assert_eq!(fit.garment.sha256, "b".repeat(64));
    assert_eq!(fit.garment.prefab_path, "Assets/Outfit.prefab");
    assert_eq!(fit.avatar.sha256, "a".repeat(64));
    assert_eq!(fit.avatar.prefab_path, "Assets/Avatar.prefab");
    // The baseline now carries the overall state, the report is cleared for the garment and
    // the save hook's file left empty is deleted.
    let baseline: FitBaseline = serde_json::from_slice(
        &fs::read(Path::new(&run.project_root).join(".vua/amf-fit-baseline.json")).unwrap(),
    ).unwrap();
    assert_eq!(baseline.garments[0].objects, vec![fit_object("Clip", 0.25)]);
    assert!(!Path::new(&run.project_root).join(".vua/bridge/amf-fit-changes.json").exists());
    let run = h.engine.get("n3-test").unwrap();
    assert!(run.fit_changes.is_empty());
    let (changes, available) = h.engine.fit_view(&run);
    assert!(changes.is_empty());
    assert_eq!(available, vec![FitRef { source_id: "outfit".into(), fit_id: Some(fit.fit_id.clone()), name: Some("Clip fit".into()) }]);
    // The garment reports no further changes, so it cannot be saved twice.
    let error = h.engine.save_fit("n3-test", "outfit", "Again", &digest).unwrap_err();
    assert_eq!(error.code, "vua.amf_production.fit_changes_missing");
    // Names: whitespace removed, empty falls back, capped at 80.
    assert_eq!(default_fit_name("Hair Clip", "Meiyun"), "HairClipForMeiyun");
    assert_eq!(default_fit_name("  ", ""), "For", "only a wholly empty result falls back");
    assert_eq!(default_fit_name("", ""), "For");
    assert_eq!(default_fit_name(&"ab ".repeat(40), "").chars().count(), 80);
    assert!(valid_fit_name("Clip fit") && !valid_fit_name("") && !valid_fit_name(" \t") && !valid_fit_name("a\nb") && !valid_fit_name(&"a".repeat(81)));
}

#[test]
fn a_saved_fit_applies_to_the_same_avatar_and_garment_only() {
    let h = Harness::new();
    let fit = fitted(&h);
    // A second line with the same Avatar and garment content lists and applies the fit.
    h.engine.begin("n3-second", choices(&["avatar", "outfit"]), "synthetic-editor").unwrap();
    assert!(h.job(|engine, ctx| engine.prepare("n3-second", ctx)));
    let plan = h.engine.plan("n3-second", &selection()).unwrap();
    assert_eq!(plan["fits"], serde_json::json!([{"sourceId":"outfit","fitId":fit.fit_id,"name":"Clip fit"}]));
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-second", selection(), &digest, ctx)));
    let applied = h.platform.assembled_fits.lock().unwrap().last().cloned().unwrap();
    assert_eq!(applied, vec![OutfitFit { source_id: "outfit".into(), objects: vec![fit_object("Clip", 0.5)] }]);
    let run = h.engine.get("n3-second").unwrap();
    assert_eq!(run.applied_fits.len(), 1);
    assert_eq!(run.applied_fits[0].fit_id.as_deref(), Some(fit.fit_id.as_str()));
    // A different Avatar content hash does not match.
    h.platform.alt_avatar_hash.store(true, Ordering::SeqCst);
    h.engine.begin("n3-other", choices(&["avatar", "outfit"]), "synthetic-editor").unwrap();
    assert!(h.job(|engine, ctx| engine.prepare("n3-other", ctx)));
    assert_eq!(h.engine.plan("n3-other", &selection()).unwrap()["fits"], serde_json::json!([]));
    // Neither does a different garment content hash.
    h.platform.alt_avatar_hash.store(false, Ordering::SeqCst);
    h.platform.change_recipe_input.store(true, Ordering::SeqCst);
    h.engine.begin("n3-changed", choices(&["avatar", "outfit"]), "synthetic-editor").unwrap();
    assert!(h.job(|engine, ctx| engine.prepare("n3-changed", ctx)));
    assert_eq!(h.engine.plan("n3-changed", &selection()).unwrap()["fits"], serde_json::json!([]));
}

#[test]
fn save_fit_refuses_stale_digests_bad_names_and_missing_pieces() {
    let h = Harness::new();
    assembled(&h);
    write_baseline(&h, "n3-test", vec![fit_object("Clip", 0.0)]);
    report_changes(&h, vec![fit_object("Clip", 0.5)], vec![]);
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    let digest = changes_digest_of(&h, "n3-test");
    let error = h.engine.save_fit("n3-test", "outfit", "two\nlines", &digest).unwrap_err();
    assert_eq!(error.code, "vua.amf_production.fit_invalid");
    let error = h.engine.save_fit("n3-test", "outfit", "Fit", &"a".repeat(64)).unwrap_err();
    assert_eq!(error.code, "vua.amf_production.plan_drift");
    let error = h.engine.save_fit("n3-test", "outfit2", "Fit", &digest).unwrap_err();
    assert_eq!(error.code, "vua.amf_production.fit_changes_missing");
    let error = h.engine.save_fit("n3-none", "outfit", "Fit", &digest).unwrap_err();
    assert_eq!(error.code, "vua.amf_production.run_not_found");
    fs::remove_file(h.root.join("production/projects/n3-test/.vua/amf-fit-baseline.json")).unwrap();
    let error = h.engine.save_fit("n3-test", "outfit", "Fit", &digest).unwrap_err();
    assert_eq!(error.code, "vua.amf_production.fit_baseline_missing");
}

#[test]
fn fits_rename_delete_and_refuse_beyond_their_limits() {
    let h = Harness::new();
    let fit = fitted(&h);
    let renamed = h.engine.rename_fit(&fit.fit_id, "Renamed clip").unwrap();
    assert_eq!(renamed.name, "Renamed clip");
    assert!(renamed.updated_at >= renamed.created_at);
    assert_eq!(h.engine.list_fits().unwrap()[0].name, "Renamed clip");
    for error in [
        h.engine.rename_fit("fit-none", "Fit").unwrap_err(),
        h.engine.delete_fit("fit-none").unwrap_err(),
    ] {
        assert_eq!(error.code, "vua.amf_production.fit_not_found");
    }
    assert_eq!(h.engine.rename_fit(&fit.fit_id, "  ").unwrap_err().code, "vua.amf_production.fit_invalid");
    h.engine.delete_fit(&fit.fit_id).unwrap();
    assert!(h.engine.list_fits().unwrap().is_empty());
    // The store keeps at most 500 fits; a new identity beyond that is refused.
    let store = h.engine.fit_store();
    for index in 0..500 {
        store
            .save(&StoredFit {
                schema_version: "0.1".into(),
                fit_id: format!("fit-d{index}"),
                name: format!("Fit {index}"),
                created_at: "2026-01-01T00:00:00.000Z".into(),
                updated_at: "2026-01-01T00:00:00.000Z".into(),
                garment: FitIdentity { sha256: "0".repeat(64), prefab_path: "Assets/X.prefab".into(), name: "X".into() },
                avatar: FitIdentity { sha256: "0".repeat(64), prefab_path: "Assets/Y.prefab".into(), name: "Y".into() },
                objects: vec![],
            })
            .unwrap();
    }
    report_changes(&h, vec![fit_object("Clip", 0.5)], vec![]);
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    let digest = changes_digest_of(&h, "n3-test");
    let error = h.engine.save_fit("n3-test", "outfit", "Clip fit", &digest).unwrap_err();
    assert_eq!(error.code, "vua.amf_production.fit_limit");
    h.engine.delete_fit("fit-d0").unwrap();
    h.engine.save_fit("n3-test", "outfit", "Clip fit", &digest).unwrap();
    // A fit carries at most 2048 objects.
    h.engine.delete_fit("fit-d1").unwrap();
    write_baseline(&h, "n3-test", (0..2049).map(|index| fit_object(&format!("Part{index}"), 0.0)).collect());
    report_changes(&h, vec![fit_object("Part0", 0.5)], vec![]);
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    let digest = changes_digest_of(&h, "n3-test");
    let error = h.engine.save_fit("n3-test", "outfit", "Clip fit", &digest).unwrap_err();
    assert_eq!(error.code, "vua.amf_production.fit_too_large");
}

#[test]
fn a_version_replay_applies_the_recorded_fit_not_the_store() {
    let h = Harness::new();
    let fit = fitted(&h);
    // A second line assembles with the fit applied, then changes its materials; the version
    // recorded before that change carries the applied fit.
    h.engine.begin("n3-second", choices(&["avatar", "outfit"]), "synthetic-editor").unwrap();
    assert!(h.job(|engine, ctx| engine.prepare("n3-second", ctx)));
    let plan = h.engine.plan("n3-second", &selection()).unwrap();
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-second", selection(), &digest, ctx)));
    assert!(h.job(|engine, ctx| engine.change_sources("n3-second", choices(&["avatar", "outfit2"]), ctx)));
    let changed = h.engine.get("n3-second").unwrap();
    let recorded = changed.revisions.iter().find(|revision| revision.before == RevisionKind::Materials).unwrap().clone();
    assert_eq!(recorded.state.fits.len(), 1);
    assert_eq!(recorded.state.fits[0].objects, vec![fit_object("Clip", 0.5)]);
    // The store losing the fit does not lose the replay.
    h.engine.delete_fit(&fit.fit_id).unwrap();
    let plan = h.engine.revision_plan("n3-second", &recorded.id).unwrap();
    assert_eq!(plan["fits"], serde_json::json!([{"sourceId":"outfit","fitId":fit.fit_id,"name":"Clip fit"}]));
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    let revision_id = recorded.id.clone();
    assert!(h.job(move |engine, ctx| engine.restore_revision("n3-second", &revision_id, &digest, ctx)));
    let applied = h.platform.assembled_fits.lock().unwrap().last().cloned().unwrap();
    assert_eq!(applied, vec![OutfitFit { source_id: "outfit".into(), objects: vec![fit_object("Clip", 0.5)] }]);
    let restored = h.engine.get("n3-second").unwrap();
    assert_eq!(restored.applied_fits.len(), 1);
    assert_eq!(restored.applied_fits[0].fit_id.as_deref(), Some(fit.fit_id.as_str()));
}

#[test]
fn recipes_carry_and_replay_the_applied_fits() {
    let h = Harness::new();
    let fit = fitted(&h);
    h.engine.begin("n3-second", choices(&["avatar", "outfit"]), "synthetic-editor").unwrap();
    assert!(h.job(|engine, ctx| engine.prepare("n3-second", ctx)));
    let plan = h.engine.plan("n3-second", &selection()).unwrap();
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-second", selection(), &digest, ctx)));
    // A Recipe saved from the fitted line records the fit under the material identity.
    h.engine.save_recipe("n3-second", "recipe-fits", "Fitted").unwrap();
    let stored = h.engine.get_recipe("recipe-fits").unwrap();
    let fits = &stored.recipe["adjustments"]["fits"];
    assert_eq!(fits[0]["materialId"], format!("m-{}", "b".repeat(64)));
    assert_eq!(fits[0]["objects"][0]["path"], "Clip");
    // Saving again for the same garment and Avatar updates the stored fit in place, leaving
    // the Recipe's recorded objects behind.
    write_baseline(&h, "n3-second", vec![fit_object("Clip", 0.5)]);
    report_changes(&h, vec![fit_object("Clip", 0.75)], vec![]);
    assert!(h.job(|engine, ctx| engine.reinspect("n3-second", ctx)));
    let digest = changes_digest_of(&h, "n3-second");
    let updated = h.engine.save_fit("n3-second", "outfit", "Clip fit", &digest).unwrap();
    assert_eq!(updated.fit_id, fit.fit_id, "the same identity updates in place");
    assert_eq!(updated.created_at, fit.created_at);
    assert_eq!(updated.objects, vec![fit_object("Clip", 0.75)]);
    // A line prepared from the Recipe applies the Recipe's fit, not the store's newer one.
    h.engine.begin_recipe("n3-third", "recipe-fits", "synthetic-editor").unwrap();
    assert!(h.job(|engine, ctx| engine.prepare("n3-third", ctx)));
    let third = h.engine.get("n3-third").unwrap();
    let selection = third.selection.clone().unwrap();
    let plan = h.engine.plan("n3-third", &selection).unwrap();
    assert_eq!(plan["fits"], serde_json::json!([{"sourceId":"outfit","fitId":null,"name":null}]));
    let digest = plan["planDigest"].as_str().unwrap().to_owned();
    assert!(h.job(move |engine, ctx| engine.assemble("n3-third", selection, &digest, ctx)));
    let applied = h.platform.assembled_fits.lock().unwrap().last().cloned().unwrap();
    assert_eq!(applied[0].objects, vec![fit_object("Clip", 0.5)], "the Recipe's recorded fit wins over the store");
    assert_eq!(h.engine.get("n3-third").unwrap().applied_fits[0].fit_id, None);
}

#[test]
fn runs_recorded_before_fits_read_with_empty_defaults() {
    let record = serde_json::json!({"schemaVersion":"0.1","runId":"n3-old","phase":"ready","taskId":"task",
        "editor":{"path":"synthetic","version":"2022.3.22f1c1"},"projectRoot":"synthetic",
        "choices":[],"sources":[],"prefabs":[],"projectFingerprint":"","unityFingerprint":"",
        "steps":[],"selection":null,"confirmedDigest":null,"result":null,"error":null,"recovery":null});
    let run: ProductionRun = serde_json::from_value(record).unwrap();
    assert!(run.fit_changes.is_empty() && run.applied_fits.is_empty() && run.observed_at.is_none());
}

#[test]
fn close_editor_requires_confirmation_and_protects_player_saves_before_dispatch() {
    let h = Harness::new(); h.prepared();
    h.platform.editor_open.store(true, Ordering::SeqCst);
    let count = h.platform.snapshots.load(Ordering::SeqCst);
    h.run_job(|engine,ctx| {
        assert_eq!(engine.close_editor("n3-test",false,ctx).unwrap_err().code,"vua.amf_production.confirmation_required");
        Ok(())
    }).unwrap();
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst),count);
    h.run_job(|engine,ctx| { let value=engine.close_editor("n3-test",true,ctx)?; assert_eq!(value["editorClosed"],true); Ok(()) }).unwrap();
    let run=h.engine.get("n3-test").unwrap();
    assert_eq!(run.phase,"inspect_required");
    assert_eq!(run.revisions[0].before,RevisionKind::EditorSave);
    assert!(run.editor_close.unwrap().succeeded());
    assert!(run.confirmed_digest.is_none());
    assert_eq!(fs::read_to_string(Path::new(&run.project_root).join("Assets/state")).unwrap(),"saved-player-edits");
}

#[test]
fn a_failed_save_preserves_the_live_editor_and_reports_blocked_automatic_recovery() {
    let h = Harness::new(); h.prepared();
    h.platform.editor_open.store(true, Ordering::SeqCst);
    h.platform.close_outcome.store(2, Ordering::SeqCst);
    assert_eq!(h.run_job(|e,c|e.close_editor("n3-test",true,c).map(|_|())).unwrap_err(),"vua.amf_production.editor_save_failed");
    let run=h.engine.get("n3-test").unwrap();
    assert_eq!(run.recovery.as_deref(),Some("restore_failed: vua.amf_production.project_open"));
    assert!(run.snapshot_id.is_some()); assert_eq!(run.revisions[0].before,RevisionKind::EditorSave);
    assert!(h.platform.editor_open.load(Ordering::SeqCst));
    assert!(!run.editor_close.unwrap().scenes_saved);
    let snapshots = h.platform.snapshots.load(Ordering::SeqCst);
    assert_eq!(h.run_job(|e,c|e.close_editor("n3-test",true,c).map(|_|())).unwrap_err(), "vua.amf_production.inspect_required");
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), snapshots, "a retry cannot replace unfinished recovery");
}

#[test]
fn a_failed_save_restores_automatically_once_unity_released_the_project() {
    let h = Harness::new(); h.prepared();
    h.platform.editor_open.store(true, Ordering::SeqCst);
    h.platform.close_outcome.store(3, Ordering::SeqCst);
    assert!(h.run_job(|e,c|e.close_editor("n3-test",true,c).map(|_|())).is_err());
    let run=h.engine.get("n3-test").unwrap();
    assert_eq!(run.recovery.as_deref(),Some("restored"));
    assert!(!run.revisions.iter().any(|r|r.before==RevisionKind::EditorSave));
    assert_ne!(fs::read_to_string(Path::new(&run.project_root).join("Assets/state")).unwrap(),"saved-player-edits");
}

#[test]
fn closing_an_already_closed_project_is_a_noop_and_open_errors_offer_a_cooperative_action() {
    let h = Harness::new(); h.prepared();
    let count=h.platform.snapshots.load(Ordering::SeqCst);
    h.run_job(|e,c| { assert_eq!(e.close_editor("n3-test",true,c)?["outcome"],"already_closed");Ok(()) }).unwrap();
    assert_eq!(count,h.platform.snapshots.load(Ordering::SeqCst));
    h.platform.editor_open.store(true,Ordering::SeqCst);
    let value=serde_json::to_value(h.engine.precheck_closed("n3-test").unwrap_err()).unwrap();
    assert_eq!(value["params"]["suggestedAction"],"amf.production.closeEditor");
}

#[test]
fn a_receipt_that_refused_before_saving_allows_retry_without_live_restore() {
    let h=Harness::new();h.prepared();
    h.platform.editor_open.store(true,Ordering::SeqCst);
    h.platform.close_outcome.store(5,Ordering::SeqCst);
    assert_eq!(h.run_job(|e,c|e.close_editor("n3-test",true,c).map(|_|())).unwrap_err(),"vua.amf_production.editor_save_failed");
    let run=h.engine.get("n3-test").unwrap();
    assert_eq!(run.recovery,None);
    assert!(!run.revisions.iter().any(|r|r.before==RevisionKind::EditorSave));
    h.platform.close_outcome.store(1,Ordering::SeqCst);
    assert!(h.job(|e,c|e.close_editor("n3-test",true,c).map(|_|())));
}

fn assembled_with_textures(h: &Harness) -> ProductionRun {
    let raw: serde_json::Value = serde_json::from_slice(include_bytes!("../../../schemas/unity-bridge/v5/examples/texture-memory.valid.result.json")).unwrap();
    assembled(h);
    *h.platform.texture_facts.lock().unwrap() = Some(raw["data"]["textureMemory"].clone());
    assert!(h.job(|engine, ctx| engine.reinspect("n3-test", ctx)));
    h.engine.get("n3-test").unwrap()
}
fn texture_digest(h: &Harness) -> String {
    h.engine.plan_texture_memory("n3-test", TexturePlatform::Pc, TextureRank::Good).unwrap()["planDigest"].as_str().unwrap().into()
}
#[test]
fn texture_apply_requires_a_current_review_and_explicit_subset_before_snapshot() {
    let h = Harness::new(); assembled_with_textures(&h);
    let digest = texture_digest(&h);
    let count = h.platform.snapshots.load(Ordering::SeqCst);
    for (review, paths) in [("f".repeat(64), vec!["Assets/Color.png".into()]),
        (digest.clone(), vec![]), (digest.clone(), vec!["Assets/Unknown.png".into()]),
        (digest.clone(), vec!["Assets/Color.png".into(), "Assets/Color.png".into()])] {
        assert!(!h.job(move |e,c| e.apply_texture_memory("n3-test", TexturePlatform::Pc, TextureRank::Good, &review, &paths, c)));
    }
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), count);
    assert!(h.job(move |e,c| e.apply_texture_memory("n3-test", TexturePlatform::Pc, TextureRank::Good, &digest, &["Assets/Color.png".into()], c)));
    let run = h.engine.get("n3-test").unwrap();
    assert_eq!(run.result["facts"]["textureMemory"][0]["rank"], "excellent");
    assert_eq!(run.result["facts"]["textureMemory"][1]["rank"], "very_poor", "PC changes leave the Android override alone");
    assert_eq!(run.revisions[0].before, RevisionKind::TextureMemory);
    assert_eq!(run.phase, "succeeded");
    assert_eq!(h.platform.snapshots.load(Ordering::SeqCst), count + 1);
    assert_eq!(h.engine.revision_plan("n3-test", &run.revisions[0].id).unwrap_err().code, "vua.amf_production.texture_manual_replay");
}
#[test]
fn texture_partial_writes_verification_failure_cancel_and_inspection_failure_restore_observations() {
    for outcome in [1,2,3,4,5] {
        let h = Harness::new(); let before = assembled_with_textures(&h);
        let digest = texture_digest(&h);
        h.platform.texture_outcome.store(outcome, Ordering::SeqCst);
        assert!(!h.job(move |e,c| e.apply_texture_memory("n3-test", TexturePlatform::Pc, TextureRank::Good, &digest, &["Assets/Color.png".into()], c)));
        let after = h.engine.get("n3-test").unwrap();
        assert_eq!(after.recovery.as_deref(), Some("restored"));
        assert_eq!(after.result, before.result);
        assert_eq!(after.revisions, before.revisions, "automatically restored changes leave no fictitious history");
    }
    let h = Harness::new(); assembled_with_textures(&h); let digest = texture_digest(&h);
    h.platform.texture_outcome.store(1, Ordering::SeqCst); h.platform.fail_restore.store(true, Ordering::SeqCst);
    assert!(!h.job(move |e,c| e.apply_texture_memory("n3-test", TexturePlatform::Pc, TextureRank::Good, &digest, &["Assets/Color.png".into()], c)));
    let run = h.engine.get("n3-test").unwrap();
    assert!(run.recovery.unwrap().starts_with("restore_failed"));
    assert_eq!(run.revisions[0].before, RevisionKind::TextureMemory);
}
#[test]
fn texture_planning_requires_facts_and_refuses_project_drift() {
    let h = Harness::new(); assembled(&h);
    assert_eq!(h.engine.plan_texture_memory("n3-test", TexturePlatform::Pc, TextureRank::Good).unwrap_err().code, "vua.amf_production.inspect_required");
    let h = Harness::new(); let run = assembled_with_textures(&h);
    fs::write(Path::new(&run.project_root).join("Assets/state"), "external edit").unwrap();
    assert_eq!(h.engine.plan_texture_memory("n3-test", TexturePlatform::Pc, TextureRank::Good).unwrap_err().code, "vua.amf_production.project_drift");
}

fn synthetic_removal(source: &str) -> OutfitRemoval {
    let outfit = format!("id-{source}"); let menu = format!("menu-{source}");
    OutfitRemoval { source_id: source.into(), avatar_global_object_id: "synthetic-avatar".into(), blocked_reason: String::new(),
        roots: vec![OutfitRemovalRoot { kind:"outfit".into(), global_object_id:outfit.clone() }, OutfitRemovalRoot { kind:"menu".into(), global_object_id:menu.clone() }],
        objects: vec![OutfitRemovalObject { global_object_id:outfit,path:"Coat".into(),type_name:"UnityEngine.GameObject".into(),manual:false },
            OutfitRemovalObject { global_object_id:menu,path:"Switch".into(),type_name:"UnityEngine.GameObject".into(),manual:false },
            OutfitRemovalObject { global_object_id:format!("added-{source}"),path:"Coat/Added child".into(),type_name:"UnityEngine.GameObject".into(),manual:true }] }
}
#[path = "support/outfit_removal.rs"]
mod outfit_removal_tests;
