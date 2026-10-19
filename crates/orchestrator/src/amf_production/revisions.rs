//! Line version records: every protected change records the line's state beforehand, and a
//! restore replays the recorded state with the existing steps (change materials, assemble or
//! rebuild, write lighting values, run SDK fixes) instead of copying the whole project back.
//! The single retained full-copy snapshot stays the hard guarantee for failed or interrupted
//! changes; version records are the reviewable, bounded history.

use super::*;
use std::collections::BTreeMap;

/// Version records retained per line; the oldest are discarded beyond this.
pub const REVISION_LIMIT: usize = 50;

/// The protected change a version record was taken before.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RevisionKind {
    Materials,
    Assembly,
    Rebuild,
    SdkFix,
    Lighting,
    Restore,
}

/// One material's content identity in a version record: the library source and the original
/// package's file name and content hash, as `PreparedSource` carries them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionSource {
    pub source_id: String,
    pub role: SourceRole,
    pub name: String,
    pub sha256: String,
}

/// Everything a replay needs to bring the line back to the recorded state.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RevisionState {
    #[serde(default)]
    pub choices: Vec<SourceChoice>,
    #[serde(default)]
    pub sources: Vec<RevisionSource>,
    #[serde(default)]
    pub selection: Option<AssemblySelection>,
    /// The brightness limits written into each in-use lilToon material at record time.
    #[serde(default)]
    pub lighting: Vec<MaterialLighting>,
    /// The SDK fixes applied to the line up to record time, named by the SDK's exact words.
    #[serde(default)]
    pub sdk_fixes: Vec<String>,
    /// The VPM dependency versions observed at record time.
    #[serde(default)]
    pub package_versions: BTreeMap<String, String>,
}

/// What a version list shows and a later A/B comparison reads.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RevisionSummary {
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub outfits: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub lighting_materials: u32,
    #[serde(default)]
    pub sdk_fixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProductionRevision {
    pub id: String,
    pub before: RevisionKind,
    pub taken_at: String,
    pub state: RevisionState,
    pub summary: RevisionSummary,
    /// The project's files no longer matched the run record when this version was taken:
    /// the player edited the project in Unity since VUA last checked it.
    #[serde(default)]
    pub edited_in_unity: bool,
}

fn choices_key(choices: &[SourceChoice]) -> Vec<String> {
    let mut ids = choices
        .iter()
        .map(|choice| format!("{}:{:?}", choice.source_id, choice.role))
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

/// A fresh version-record identity: timestamp-ordered and unique within the run.
fn fresh_revision_id(run: &ProductionRun) -> String {
    let nanos = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or(0);
    let mut id = format!("rev-{nanos}");
    while run.revisions.iter().any(|revision| revision.id == id) {
        id.push('_');
    }
    id
}

fn summarize(run: &ProductionRun, state: &RevisionState) -> RevisionSummary {
    let prefab_name = |path: &str| {
        run.prefabs
            .iter()
            .find(|candidate| candidate.path == path)
            .map(|candidate| candidate.name.clone())
            .unwrap_or_else(|| {
                path.rsplit('/')
                    .next()
                    .unwrap_or(path)
                    .trim_end_matches(".prefab")
                    .to_string()
            })
    };
    RevisionSummary {
        avatar: state.selection.as_ref().map(|selection| {
            run.result["facts"]["avatarName"]
                .as_str()
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| prefab_name(&selection.avatar_prefab))
        }),
        outfits: state
            .selection
            .as_ref()
            .map(|selection| {
                selection
                    .outfits
                    .iter()
                    .map(|outfit| prefab_name(&outfit.prefab_path))
                    .collect()
            })
            .unwrap_or_default(),
        sources: run
            .sources
            .iter()
            .map(|source| source.item_name.clone())
            .collect(),
        lighting_materials: state.lighting.len() as u32,
        sdk_fixes: state.sdk_fixes.clone(),
    }
}

/// Runs recorded before version records existed carry only a full-copy restore point. Reading
/// one synthesizes a single `assembly` version from the restore point's selection and
/// materials; what the restore point never recorded stays empty. Nothing is discarded.
pub(crate) fn legacy_revision(run: &ProductionRun) -> Option<ProductionRevision> {
    if !run.revisions.is_empty() {
        return None;
    }
    let snapshot_id = run.snapshot_id.as_ref()?;
    let state = RevisionState {
        choices: run
            .snapshot_sources
            .as_ref()
            .map(|sources| sources.choices.clone())
            .unwrap_or_default(),
        sources: run
            .snapshot_sources
            .as_ref()
            .map(|sources| {
                sources
                    .sources
                    .iter()
                    .map(|source| RevisionSource {
                        source_id: source.source_id.clone(),
                        role: source.role,
                        name: source.name.clone(),
                        sha256: source.sha256.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        selection: run.snapshot_selection.clone(),
        ..Default::default()
    };
    Some(ProductionRevision {
        id: format!("rev-legacy-{snapshot_id}"),
        before: RevisionKind::Assembly,
        taken_at: run.created_at.clone(),
        summary: summarize(run, &state),
        state,
        edited_in_unity: false,
    })
}

/// The digest binds the line, its revision, the current project fingerprint and the target
/// version's state. A discarded version, a changed line or a changed project all refuse the
/// execution as `plan_drift` and ask for a fresh review.
fn revision_digest(
    run: &ProductionRun,
    revision: &ProductionRevision,
    current_fingerprint: &str,
) -> Result<String, AppErrorV1> {
    let state_hash = Sha256::digest(&serde_json::to_vec(&revision.state).map_err(io_error)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let value = serde_json::json!({
        "operation": "restore_revision",
        "runId": run.run_id,
        "revision": run.revision,
        "projectRoot": run.project_root,
        "currentFingerprint": current_fingerprint,
        "revisionId": revision.id,
        "state": state_hash,
    });
    Ok(Sha256::digest(&serde_json::to_vec(&value).map_err(io_error)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Marks a replay error with the step that failed, so the report names where the restore stopped.
fn at_step(step: &str, error: AppErrorV1) -> AppErrorV1 {
    let detail = error
        .params
        .as_ref()
        .and_then(|params| params.get("detail"))
        .and_then(|value| match value {
            ParamValue::Text(text) => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_else(|| error.code.clone());
    production_error(
        error.code.strip_prefix("vua.amf_production.").unwrap_or(&error.code),
        format!("The restore's {step} step failed: {detail}"),
    )
}

/// The steps a replay of `revision` on `run` needs, as plan and execution agree on them.
struct ReplaySteps {
    materials: bool,
    assembly: bool,
    rebuild: bool,
    lighting: Vec<MaterialLighting>,
    sdk_fixes: Vec<String>,
}

fn replay_steps(run: &ProductionRun, revision: &ProductionRevision) -> ReplaySteps {
    let state = &revision.state;
    let materials =
        !state.choices.is_empty() && choices_key(&state.choices) != choices_key(&run.choices);
    let assembly = state.selection.as_ref().is_some_and(|selection| {
        // A selection whose outfits all left with their materials mirrors a line planning
        // again: the replay changes the materials and assembles nothing.
        !selection.outfits.is_empty()
            && (materials || run.selection.as_ref() != state.selection.as_ref())
    });
    let rebuild = assembly
        && !materials
        && serde_json::from_value::<AssemblySelection>(
            run.result["facts"]["observedSelection"].clone(),
        )
        .is_ok_and(|observed| {
            observed.avatar_prefab
                != state
                    .selection
                    .as_ref()
                    .expect("assembly step has a selection")
                    .avatar_prefab
        });
    ReplaySteps {
        materials,
        assembly,
        rebuild,
        lighting: state.lighting.clone(),
        sdk_fixes: state
            .sdk_fixes
            .iter()
            .filter(|fix| !run.sdk_fixes_applied.contains(*fix))
            .cloned()
            .collect(),
    }
}

impl AmfProductionEngine {
    /// Records the line's current state as a version before a protected change. Newest first;
    /// beyond `REVISION_LIMIT` the oldest records are discarded. Returns the record's identity
    /// so a failed change whose automatic restore succeeded can drop it again.
    pub(crate) fn record_revision(
        &self,
        run: &mut ProductionRun,
        before: RevisionKind,
        edited_in_unity: bool,
    ) -> String {
        let state = RevisionState {
            choices: run.choices.clone(),
            sources: run
                .sources
                .iter()
                .map(|source| RevisionSource {
                    source_id: source.source_id.clone(),
                    role: source.role,
                    name: source.name.clone(),
                    sha256: source.sha256.clone(),
                })
                .collect(),
            selection: run.selection.clone(),
            lighting: current_lighting(run),
            sdk_fixes: run.sdk_fixes_applied.clone(),
            package_versions: serde_json::from_value(run.result["packageVersions"].clone())
                .unwrap_or_default(),
        };
        let revision = ProductionRevision {
            id: fresh_revision_id(run),
            before,
            taken_at: SystemClock.now_rfc3339(),
            summary: summarize(run, &state),
            state,
            edited_in_unity,
        };
        let id = revision.id.clone();
        run.revisions.insert(0, revision);
        run.revisions.truncate(REVISION_LIMIT);
        id
    }

    /// Whether the project's files differ from the run record right now; unreadable state
    /// counts as changed, so the version list rather warns once too often.
    pub(crate) fn edited_in_unity(&self, run: &ProductionRun) -> bool {
        self.project_state(run).map(|(same, _)| !same).unwrap_or(true)
    }

    /// Every recorded original must still be in the library with the recorded content;
    /// otherwise the replay cannot bring the recorded materials back.
    fn verify_revision_sources(&self, revision: &ProductionRevision) -> Result<(), AppErrorV1> {
        let mut problems = Vec::new();
        for source in &revision.state.sources {
            match self.platform.source_content_hash(&source.source_id)? {
                Some(actual) if actual == source.sha256 => {}
                Some(_) => problems.push(format!("{} changed in the library", source.name)),
                None => problems.push(format!("{} is no longer in the library", source.name)),
            }
        }
        if !problems.is_empty() {
            return Err(production_error(
                "revision_source_missing",
                format!(
                    "This version's original packages are unavailable: {}. Supply these exact original packages in the library, then retry.",
                    problems.join(", ")
                ),
            ));
        }
        Ok(())
    }

    /// Reviews a restore to a recorded version: the steps the replay takes, which it skips,
    /// and a digest binding the line, the current project and the version's state.
    pub fn revision_plan(
        &self,
        run_id: &str,
        revision_id: &str,
    ) -> Result<serde_json::Value, AppErrorV1> {
        let run = self.get(run_id)?;
        if !matches!(
            run.phase.as_str(),
            "ready" | "succeeded" | "failed" | "cancelled" | "stopped"
        ) {
            return Err(production_error(
                "inspect_required",
                "Reinspect interrupted work before reviewing a restore.",
            ));
        }
        let revision = run
            .revisions
            .iter()
            .find(|revision| revision.id == revision_id)
            .ok_or_else(|| {
                production_error(
                    "revision_missing",
                    "This version record is no longer retained. Review the line's current versions.",
                )
            })?;
        self.verify_revision_sources(revision)?;
        let edited = self.edited_in_unity(&run);
        let (_, current) = self.project_state(&run)?;
        let steps = replay_steps(&run, revision);
        let digest = revision_digest(&run, revision, &current)?;
        Ok(serde_json::json!({
            "schemaVersion": "0.1",
            "runId": run.run_id,
            "revisionId": revision.id,
            "planDigest": digest,
            "before": revision.before,
            "takenAt": revision.taken_at,
            "summary": revision.summary,
            "editedInUnity": edited,
            "warning": edited.then_some(
                "The project changed in Unity since the last check. Restoring this version replaces those edits."
            ),
            "steps": [
                {"key": "materials", "skipped": !steps.materials},
                {"key": "assembly", "skipped": !steps.assembly, "rebuild": steps.rebuild},
                {"key": "lighting", "skipped": steps.lighting.is_empty(), "materials": steps.lighting},
                {"key": "sdk_fix", "skipped": steps.sdk_fixes.is_empty(), "fixes": steps.sdk_fixes},
            ],
        }))
    }

    /// Restores a reviewed version by replaying the recorded state with the existing steps.
    /// A version of the current state is recorded first, so the restore is itself reversible;
    /// any failed step restores the retained full-copy snapshot taken before the replay.
    pub fn restore_revision(
        &self,
        run_id: &str,
        revision_id: &str,
        digest: &str,
        ctx: &TaskContext,
    ) -> Result<(), AppErrorV1> {
        let plan = self.revision_plan(run_id, revision_id)?;
        if plan["planDigest"].as_str() != Some(digest) {
            return Err(production_error(
                "plan_drift",
                "The project or version record changed. Review the restore again.",
            ));
        }
        let mut run = self.get(run_id)?;
        let revision = run
            .revisions
            .iter()
            .find(|revision| revision.id == revision_id)
            .cloned()
            .expect("a reviewed version record");
        // The library is re-verified at execution: a source removed or replaced between the
        // review and the restore refuses here, before the project is touched.
        self.verify_revision_sources(&revision)?;
        let steps = replay_steps(&run, &revision);
        let entry_phase = run.phase.clone();
        let current_selection = run.selection.clone();
        let current_fixes = run.sdk_fixes_applied.clone();
        let current_sources = SourceSnapshot {
            choices: run.choices.clone(),
            sources: run.sources.clone(),
        };
        let edited = plan["editedInUnity"].as_bool().unwrap_or(false);
        run.task_id = ctx.task_id().into();
        run.phase = "restoring".into();
        run.error = None;
        run.recovery = None;
        run.confirmed_digest = None;
        let undo = self.record_revision(&mut run, RevisionKind::Restore, edited);
        let mut safety = None;
        let mut mutated = false;
        let result = (|| {
            self.step(
                &mut run,
                ctx,
                "protect_current",
                "Protect the current project before restoring",
            )?;
            let current = self.platform.fingerprint(Path::new(&run.project_root))?;
            if revision_digest(&run, &revision, &current)? != digest {
                return Err(production_error(
                    "plan_drift",
                    "The project changed before restoration. Review it again.",
                ));
            }
            safety = Some(self.platform.snapshot(&run)?);
            run.snapshot_id = safety.clone();
            run.snapshot_selection = current_selection.clone();
            run.snapshot_sources = Some(current_sources.clone());
            if steps.materials {
                self.step(&mut run, ctx, "materials", "Change the line's materials")
                    .map_err(|error| at_step("materials", error))?;
                validate_choices(&revision.state.choices).map_err(|error| at_step("materials", error))?;
                run.choices = revision.state.choices.clone();
                run.sources = self
                    .platform
                    .replace_sources(&run, &current_sources.sources, ctx)
                    .map_err(|error| at_step("materials", error))?;
                mutated = true;
                self.save(&run)?;
                self.step(&mut run, ctx, "inspect", "Identify actual Avatar and outfit prefabs")
                    .map_err(|error| at_step("inspect", error))?;
                self.update_observations(&mut run, ctx)
                    .map_err(|error| at_step("inspect", error))?;
            }
            if steps.assembly {
                let selection = revision
                    .state
                    .selection
                    .clone()
                    .expect("assembly step has a selection");
                if steps.rebuild {
                    self.step(&mut run, ctx, "inspect", "Start a fresh work scene for the new base")
                        .map_err(|error| at_step("inspect", error))?;
                    self.platform
                        .clear_work_scene(&run)
                        .map_err(|error| at_step("inspect", error))?;
                    self.update_observations(&mut run, ctx)
                        .map_err(|error| at_step("inspect", error))?;
                }
                let mut check = run.clone();
                check.phase = "ready".into();
                validate_selection(&check, &selection).map_err(|error| at_step("assembly", error))?;
                let observed: Option<AssemblySelection> =
                    serde_json::from_value(run.result["facts"]["observedSelection"].clone()).ok();
                let target = if let Some(current) = observed.filter(|_| !steps.rebuild) {
                    let id = run.result["facts"]["avatarGlobalObjectId"]
                        .as_str()
                        .filter(|id| !id.is_empty())
                        .ok_or_else(|| {
                            at_step(
                                "assembly",
                                production_error(
                                    "inspect_required",
                                    "Reinspect the current Avatar identity before editing.",
                                ),
                            )
                        })?;
                    Some(crate::amf_recipe_apply::RecipeApplyTarget {
                        avatar_global_object_id: id.into(),
                        new_avatar_key: None,
                        write_source_ids: selection
                            .outfits
                            .iter()
                            .filter(|outfit| !current.outfits.contains(outfit))
                            .map(|outfit| outfit.source_id.clone())
                            .collect(),
                    })
                } else {
                    None
                };
                self.step(&mut run, ctx, "assembly", "Assemble the recorded selection")
                    .map_err(|error| at_step("assembly", error))?;
                let observed = match &target {
                    Some(target) => self
                        .platform
                        .apply_recipe(&run, &selection, target, &self.watch(&run, ctx))
                        .map_err(|error| at_step("assembly", error))?,
                    None => self
                        .platform
                        .assemble(&run, &selection, &self.watch(&run, ctx))
                        .map_err(|error| at_step("assembly", error))?,
                };
                if let Some(error) = ctx.cancellation_error("amf-production") {
                    return Err(error);
                }
                if observed.editor_version != run.editor.version {
                    return Err(at_step(
                        "assembly",
                        production_error("editor_drift", "The actual Editor version changed."),
                    ));
                }
                mutated = true;
                run.result = observed.result;
                run.unity_fingerprint = observed.unity_fingerprint;
                let _check = FingerprintCheck::begin(self, &run, ctx);
                run.project_fingerprint = self
                    .platform
                    .fingerprint(Path::new(&run.project_root))
                    .map_err(|error| at_step("assembly", error))?;
                run.completed_at = Some(SystemClock.now_rfc3339());
            }
            if !steps.lighting.is_empty() {
                self.step(&mut run, ctx, "lighting", "Write back the recorded lighting values")
                    .map_err(|error| at_step("lighting", error))?;
                self.apply_lighting_values(&mut run, &steps.lighting, ctx)
                    .map_err(|error| at_step("lighting", error))?;
                mutated = true;
            }
            if !steps.sdk_fixes.is_empty() {
                self.step(&mut run, ctx, "sdk_fix", "Run the recorded SDK fixes")
                    .map_err(|error| at_step("sdk_fix", error))?;
                self.run_sdk_fixes(&mut run, &steps.sdk_fixes, ctx)
                    .map_err(|error| at_step("sdk_fix", error))?;
                mutated = true;
            }
            self.step(&mut run, ctx, "reinspect", "Verify the restored project in Unity")
                .map_err(|error| at_step("reinspect", error))?;
            self.update_observations(&mut run, ctx)
                .map_err(|error| at_step("reinspect", error))?;
            if let Some(error) = ctx.cancellation_error("amf-production") {
                return Err(error);
            }
            // Materials the replay changed are already on the run; a version that never
            // recorded materials (a legacy one) leaves the line's materials as they are.
            run.selection = revision.state.selection.clone();
            // SDK fixes physically applied to a project cannot be unrun; a replay can only
            // add the recorded ones. The applied set after a restore is the union of both.
            let mut applied = revision.state.sdk_fixes.clone();
            for fix in &current_fixes {
                if !applied.contains(fix) {
                    applied.push(fix.clone());
                }
            }
            run.sdk_fixes_applied = applied;
            if steps.materials && !steps.assembly {
                run.completed_at = None;
            }
            run.phase = if steps.assembly {
                "succeeded".into()
            } else if steps.materials {
                // The replay changed the materials without a recorded assembly: the line
                // plans again, as a material change leaves it.
                "ready".into()
            } else if matches!(entry_phase.as_str(), "ready" | "succeeded") {
                entry_phase.clone()
            } else {
                "ready".into()
            };
            Ok(())
        })();
        if result.is_err() {
            if mutated {
                if let Some(safety) = safety {
                    run.recovery = Some(match self.platform.restore(&run, &safety) {
                        Ok(()) => "restored".into(),
                        Err(error) => format!("restore_failed: {}", error.code),
                    });
                }
                run.choices = current_sources.choices.clone();
                run.sources = current_sources.sources.clone();
                run.selection = current_selection;
                // Neither the old result nor a partial replay proves the current state.
                // Explicit reinspection must establish it again.
                run.result = serde_json::Value::Null;
            }
            // A restore that never touched the project, or whose automatic recovery brought
            // it back, leaves no state behind worth a version record.
            if !matches!(run.recovery.as_deref(), Some(recovery) if recovery.starts_with("restore_failed")) {
                run.revisions.retain(|revision| revision.id != undo);
            }
        }
        let outcome = self.finish(&mut run, result);
        self.prune_snapshots(&run);
        outcome
    }
}
