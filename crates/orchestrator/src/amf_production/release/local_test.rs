//! Local SDK builds have their own history and never replace a production result.
use super::*;

pub const LOCAL_TEST_STAGES: [&str; 4] = ["sdk_panel", "sign_in", "building", "finishing"];
pub const LOCAL_TEST_REASONS: [&str; 13] = [
    "sign_in_required",
    "validation",
    "build",
    "build_blocked",
    "avatar_missing",
    "scene_not_saved",
    "interrupted",
    "editor_unavailable",
    "timeout",
    "cancelled",
    "platform_unsupported",
    "bridge_outdated",
    "unknown",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalTestRecord {
    pub test_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub state: String,
    pub stage: String,
    pub status: String,
    pub reason: Option<String>,
    pub detail: String,
    pub errors: Vec<String>,
    pub sdk_sign_in: Option<String>,
    /// Fresh SDK observations belong to this test, not to the assembled result.
    pub sdk_review: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTestRequest {
    pub test_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalTestReceipt {
    pub outcome: String,
    pub reason: Option<String>,
    pub detail: String,
    pub errors: Vec<String>,
    pub can_publish_avatars: Option<bool>,
    pub sdk_sign_in: Option<String>,
    pub sdk_review: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UploadEligibility {
    schema_version: String,
    pub can_publish: bool,
    pub observed_at: String,
}

impl AmfProductionEngine {
    pub(super) fn record_upload_eligibility(
        &self,
        observed: Option<bool>,
        ctx: &TaskContext,
        errors: &mut Vec<String>,
    ) {
        if self.observe_upload_eligibility(observed).is_err() {
            // The SDK outcome remains authoritative even if this separate observation cannot be saved.
            ctx.warn();
            errors.truncate(MAX_ERRORS - 1);
            errors.push("The latest upload eligibility could not be saved. Any earlier observation remains dated.".into());
        }
    }

    pub(super) fn upload_eligibility(&self) -> Option<serde_json::Value> {
        let path = self.root.join("upload-eligibility.json");
        if fs::metadata(&path).ok()?.len() > 4096 {
            return None;
        }
        let value: UploadEligibility = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
        if value.schema_version != "0.1" || crate::rfc3339_millis(&value.observed_at).is_none() {
            return None;
        }
        Some(serde_json::json!({"canPublish":value.can_publish,"observedAt":value.observed_at}))
    }

    pub(super) fn observe_upload_eligibility(
        &self,
        observed: Option<bool>,
    ) -> Result<(), AppErrorV1> {
        let Some(can_publish) = observed else {
            return Ok(());
        };
        let value = UploadEligibility {
            schema_version: "0.1".into(),
            can_publish,
            observed_at: SystemClock.now_rfc3339(),
        };
        fs::create_dir_all(&self.root).map_err(io_error)?;
        let pending = self.root.join("upload-eligibility.pending");
        fs::write(&pending, serde_json::to_vec(&value).map_err(io_error)?).map_err(io_error)?;
        fs::rename(pending, self.root.join("upload-eligibility.json")).map_err(io_error)
    }

    pub fn local_test(&self, run_id: &str, ctx: &TaskContext) -> Result<(), AppErrorV1> {
        self.precheck_local_test(run_id)?;
        let mut run = self.get(run_id)?;
        let mut record = self.release_record(run_id)?;
        let now = SystemClock.now_rfc3339();
        for stale in record
            .local_tests
            .iter_mut()
            .filter(|test| test.state == "running")
        {
            stale.state = "failed".into();
            stale.reason = Some("interrupted".into());
            stale.ended_at = Some(now.clone());
        }
        let test_id = format!(
            "test-{}",
            ctx.task_id()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .take(80)
                .collect::<String>()
        );
        record.local_tests.push(LocalTestRecord {
            test_id: test_id.clone(),
            started_at: now,
            ended_at: None,
            state: "running".into(),
            stage: "sdk_panel".into(),
            status: String::new(),
            reason: None,
            detail: String::new(),
            errors: Vec::new(),
            sdk_sign_in: None,
            sdk_review: None,
        });
        let excess = record.local_tests.len().saturating_sub(20);
        record.local_tests.drain(..excess);
        self.save_release(&record)?;
        run.task_id = ctx.task_id().into();
        self.step(
            &mut run,
            ctx,
            "local_test",
            "Build and test with the VRChat SDK",
        )?;
        let mut last: Option<UploadProgress> = None;
        let mut progress = |line: UploadProgress| {
            if !LOCAL_TEST_STAGES.contains(&line.stage.as_str()) || last.as_ref() == Some(&line) {
                return;
            }
            if let Ok(mut record) = self.release_record(run_id) {
                if let Some(test) = record
                    .local_tests
                    .iter_mut()
                    .find(|test| test.test_id == test_id)
                {
                    test.stage = line.stage.clone();
                    test.status = bounded_text(&line.status, 200);
                    let _ = self.save_release(&record);
                }
            }
            ctx.emit_progress(
                serde_json::json!({"operation":"amf.production","runId":run_id,"stage":"local_test",
                "name":"Build and test with the VRChat SDK","localTest":{"stage":line.stage}}),
            );
            last = Some(line);
        };
        let outcome = self.platform.local_test(
            &run,
            &LocalTestRequest {
                test_id: test_id.clone(),
            },
            ctx,
            &mut progress,
        );
        let mut record = self.release_record(run_id)?;
        let test = record
            .local_tests
            .iter_mut()
            .find(|test| test.test_id == test_id)
            .ok_or_else(|| {
                production_error("release_invalid", "The local test record is missing.")
            })?;
        test.ended_at = Some(SystemClock.now_rfc3339());
        let result = match outcome {
            Ok(receipt) => {
                test.detail = bounded_text(&receipt.detail, MAX_DETAIL_CHARS);
                test.errors = receipt
                    .errors
                    .iter()
                    .take(MAX_ERRORS)
                    .map(|error| bounded_text(error, 500))
                    .collect();
                self.record_upload_eligibility(receipt.can_publish_avatars, ctx, &mut test.errors);
                test.sdk_sign_in = receipt
                    .sdk_sign_in
                    .filter(|source| source == "panel" || source == "vua_session");
                test.sdk_review = receipt.sdk_review.filter(|review| {
                    matches!(review["state"].as_str(), Some("checked" | "partial"))
                });
                match receipt.outcome.as_str() {
                    "succeeded" => {
                        test.state = "succeeded".into();
                        test.stage = "finishing".into();
                        Ok(())
                    }
                    "cancelled" => {
                        test.state = "cancelled".into();
                        test.reason = Some("cancelled".into());
                        Err(ctx.cancellation_error("amf-production").unwrap_or_else(|| {
                            production_error("local_test_cancelled", "The local test was stopped.")
                        }))
                    }
                    _ => {
                        test.state = "failed".into();
                        test.reason = Some(
                            receipt
                                .reason
                                .filter(|reason| LOCAL_TEST_REASONS.contains(&reason.as_str()))
                                .unwrap_or_else(|| "unknown".into()),
                        );
                        Err(production_error(
                            "local_test_failed",
                            format!(
                                "{}: {}",
                                test.reason.as_deref().unwrap_or("unknown"),
                                test.detail
                            ),
                        ))
                    }
                }
            }
            Err(error) => {
                let reason = match error.code.as_str() {
                    "vua.amf_production.local_test_timeout" => "timeout",
                    "vua.amf_production.platform_unsupported" => "platform_unsupported",
                    "vua.amf_production.bridge_outdated" => "bridge_outdated",
                    _ => reason_of(&error),
                };
                test.state = if reason == "cancelled" {
                    "cancelled"
                } else {
                    "failed"
                }
                .into();
                test.reason = Some(reason.into());
                test.detail = bounded_text(&error_detail(&error), MAX_DETAIL_CHARS);
                Err(error)
            }
        };
        self.save_release(&record)?;
        match &result {
            Ok(()) => Self::end_step(&mut run, "succeeded", String::new()),
            Err(error) => Self::end_step(
                &mut run,
                "failed",
                format!("{} — {}", error.code, error_detail(error)),
            ),
        }
        self.save(&run)?;
        result
    }
}
