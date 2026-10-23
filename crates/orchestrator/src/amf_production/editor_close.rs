//! Confirmed saving of player edits and cooperative Editor shutdown.
use super::*;

/// The Editor writes saving facts before exit. Only the native waiter sets `editorClosed`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorCloseResult {
    pub schema_version: String,
    pub run_id: String,
    pub close_id: String,
    pub outcome: String,
    pub scenes_saved: bool,
    pub assets_saved: bool,
    pub editor_closed: bool,
    pub reason: String,
    pub detail: String,
}

impl EditorCloseResult {
    pub fn succeeded(&self) -> bool {
        self.editor_closed
            && self.outcome == "saved"
            && self.scenes_saved
            && self.assets_saved
            && self.reason.is_empty()
    }
}

impl AmfProductionEngine {
    pub fn close_editor(
        &self,
        run_id: &str,
        confirmed: bool,
        ctx: &TaskContext,
    ) -> Result<serde_json::Value, AppErrorV1> {
        if !confirmed {
            return Err(production_error(
                "confirmation_required",
                "Confirm saving the open scenes and assets and closing Unity.",
            ));
        }
        let mut run = self.get(run_id)?;
        if let Some(error) = ctx.cancellation_error("amf-production") {
            return Err(error);
        }
        if !self.platform.editor_open(Path::new(&run.project_root)) {
            return Ok(serde_json::json!({"schemaVersion":"0.1", "runId":run_id,
                "outcome":"already_closed", "editorClosed":true}));
        }
        if run
            .recovery
            .as_deref()
            .is_some_and(|value| value.starts_with("restore_failed"))
            || (ACTIVE_PHASES.contains(&run.phase.as_str()) && run.snapshot_id.is_some())
        {
            return Err(production_error("inspect_required", "An unfinished change still needs its restore point. Save and close Unity manually, then resolve that recovery before requesting another save."));
        }
        let phase = run.phase.clone();
        // Saving player edits is allowed to start with drift. Do not bless those edits as an
        // inspected production result; a changed project must be inspected after it closes.
        let before = self.platform.fingerprint(Path::new(&run.project_root))?;
        let snapshot = self.platform.snapshot_open_editor(&run)?;
        if self.platform.fingerprint(Path::new(&run.project_root))? != before {
            return Err(production_error("project_drift", "Project files changed while the restore point was being taken. Save in Unity, then try again."));
        }
        run.task_id = ctx.task_id().into();
        run.phase = "assembling".into();
        run.error = None;
        run.recovery = None;
        run.editor_close = None;
        run.snapshot_id = Some(snapshot.clone());
        run.snapshot_selection = run.selection.clone();
        run.snapshot_sources = None;
        let revision = self.record_revision(&mut run, RevisionKind::EditorSave, true);
        run.confirmed_digest = None;
        self.save(&run)?;
        let result = (|| {
            self.step(&mut run, ctx, "editor_close", "Save and close Unity")?;
            let receipt = self.platform.close_editor(&run, ctx)?;
            let success = receipt.succeeded();
            let error = production_error(
                if receipt.reason == "timeout" {
                    "editor_close_timeout"
                } else {
                    "editor_save_failed"
                },
                &receipt.detail,
            );
            run.editor_close = Some(receipt);
            if !success {
                return Err(error);
            }
            let changed =
                self.platform.fingerprint(Path::new(&run.project_root))? != run.project_fingerprint;
            run.phase = if changed {
                "inspect_required".into()
            } else {
                phase
            };
            Ok(())
        })();
        if result.is_err() {
            let save_skipped = run.editor_close.as_ref().is_some_and(|receipt| {
                !receipt.scenes_saved
                    && !receipt.assets_saved
                    && matches!(
                        receipt.reason.as_str(),
                        "editor_busy" | "untitled_scene" | "expired"
                    )
            });
            // Never overwrite files while Unity holds the project. The native restore keeps
            // its usual lock guard; a failed restore retains the snapshot and version.
            if save_skipped {
                // These receipts reject before any save. A restore would only block on the
                // live Editor and incorrectly prevent retry after the player fixes the cause.
                run.revisions.retain(|entry| entry.id != revision);
            } else {
                run.recovery = Some(match self.platform.restore(&run, &snapshot) {
                    Ok(()) => {
                        run.revisions.retain(|entry| entry.id != revision);
                        "restored".into()
                    }
                    Err(error) => format!("restore_failed: {}", error.code),
                });
            }
        }
        let outcome = self.finish(&mut run, result);
        self.prune_snapshots(&run);
        outcome?;
        serde_json::to_value(run.editor_close).map_err(io_error)
    }
}
