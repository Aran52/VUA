//! BuildAndTest cannot be cancelled after building starts. Keep waiting for its actual receipt.
use super::*;

const TEST_REQUEST: &str = "amf-test.json";
const TEST_PROGRESS: &str = "amf-test.progress.json";
const TEST_RECEIPT: &str = "amf-test.result.json";
const TEST_CANCEL: &str = "amf-test.cancel";
const TEST_TIMEOUT: Duration = Duration::from_secs(60 * 60);

pub(crate) fn local_test(
    platform: &NativeAmfProduction,
    run: &ProductionRun,
    request: &LocalTestRequest,
    ctx: &TaskContext,
    progress: &mut dyn FnMut(UploadProgress),
) -> Result<LocalTestReceipt, AppErrorV1> {
    platform.verify_editor(&run.editor.path)?;
    let avatar = run.result["facts"]["avatarGlobalObjectId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| err("result_invalid", "Result has no Avatar identity."))?;
    let project = Path::new(&run.project_root);
    ensure_sdk_idle(project)?;
    let open = editor_holds_project(project);
    if open {
        ensure_current_bridge(project)?;
    } else {
        refresh_bridge_package(project)?;
    }
    let folder = project.join(".vua/bridge");
    fs::create_dir_all(&folder).map_err(|error| err("local_test_failed", error))?;
    for name in [TEST_RECEIPT, TEST_PROGRESS, TEST_CANCEL] {
        remove(&folder.join(name))?;
    }
    write_atomic(
        &folder.join(TEST_REQUEST),
        &request_json(run, request, avatar),
    )?;
    if !open {
        platform.start_unity_hub();
        if std::env::var("VUA_AMF_HANDOFF_FROM_DESKTOP").as_deref() != Ok("1") {
            if let Err(error) = open_editor(run) {
                if !folder.join("amf-test.started").exists() {
                    let _ = remove(&folder.join(TEST_REQUEST));
                }
                return Err(error);
            }
        }
    }
    wait_test(
        &folder,
        run,
        request,
        ctx,
        progress,
        PICKUP_TIMEOUT,
        TEST_TIMEOUT,
        CANCEL_GRACE,
    )
}

fn request_json(
    run: &ProductionRun,
    request: &LocalTestRequest,
    avatar: &str,
) -> serde_json::Value {
    serde_json::json!({"schemaVersion":"0.1","runId":run.run_id,"testId":request.test_id,"avatarGlobalObjectId":avatar})
}

fn read_test(
    path: &Path,
    run: &ProductionRun,
    request: &LocalTestRequest,
) -> Option<serde_json::Value> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    (value["schemaVersion"] == "0.1"
        && value["runId"] == run.run_id.as_str()
        && value["testId"] == request.test_id.as_str())
    .then_some(value)
}

fn read_test_receipt(
    path: &Path,
    run: &ProductionRun,
    request: &LocalTestRequest,
) -> Result<Option<LocalTestReceipt>, AppErrorV1> {
    let Some(value) = read_test(path, run, request) else {
        return Ok(None);
    };
    let outcome = value["outcome"]
        .as_str()
        .filter(|outcome| ["succeeded", "failed", "cancelled"].contains(outcome))
        .ok_or_else(|| {
            err(
                "receipt_invalid",
                "The Editor reported an unknown local test outcome.",
            )
        })?;
    let text = |key: &str| {
        value[key]
            .as_str()
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    Ok(Some(LocalTestReceipt {
        outcome: outcome.into(),
        reason: text("reason"),
        detail: text("detail").unwrap_or_default(),
        errors: value["errors"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|error| error.as_str().map(str::to_owned))
            .take(32)
            .collect(),
        can_publish_avatars: value["canPublishAvatars"]
            .as_bool()
            .filter(|_| value["canPublishAvatarsObserved"] == true),
        sdk_sign_in: text("sdkSignIn")
            .filter(|source| source == "panel" || source == "vua_session"),
        sdk_review: Some(value["sdkReview"].clone()).filter(read_review),
    }))
}

#[allow(clippy::too_many_arguments)]
fn wait_test(
    folder: &Path,
    run: &ProductionRun,
    request: &LocalTestRequest,
    ctx: &TaskContext,
    progress: &mut dyn FnMut(UploadProgress),
    pickup_timeout: Duration,
    timeout: Duration,
    cancel_grace: Duration,
) -> Result<LocalTestReceipt, AppErrorV1> {
    let start = Instant::now();
    let mut picked_up = false;
    let mut building = false;
    let mut stopping: Option<(Instant, AppErrorV1)> = None;
    loop {
        if let Some(receipt) = read_test_receipt(&folder.join(TEST_RECEIPT), run, request)? {
            // A failed blueprint restoration keeps the journal/request for the observer to retry.
            if !folder.join("amf-test.started").exists() {
                let _ = remove(&folder.join(TEST_REQUEST));
            }
            return Ok(receipt);
        }
        // The durable journal is written at the last cancellation boundary, before building.
        // It remains authoritative even if a progress write was delayed or lost during reload.
        building |= read_test(&folder.join("amf-test.started"), run, request).is_some();
        picked_up |= building;
        if let Some(value) = read_test(&folder.join(TEST_PROGRESS), run, request) {
            if let Some(stage) = value["stage"]
                .as_str()
                .filter(|stage| LOCAL_TEST_STAGES.contains(stage))
            {
                picked_up = true;
                building |= matches!(stage, "building" | "finishing");
                progress(UploadProgress {
                    stage: stage.into(),
                    progress: None,
                    status: bounded_text(value["status"].as_str().unwrap_or(""), 200),
                });
            }
        }
        if !building && stopping.is_none() {
            if let Some(error) = ctx.cancellation_error("amf-production") {
                write_atomic(
                    &folder.join(TEST_CANCEL),
                    &serde_json::json!({"schemaVersion":"0.1","runId":run.run_id,"testId":request.test_id}),
                )?;
                stopping = Some((Instant::now(), error));
            }
        }
        if !building {
            if let Some((since, error)) = &stopping {
                if since.elapsed() >= cancel_grace {
                    remove(&folder.join(TEST_REQUEST))?;
                    return Err(error.clone());
                }
            }
        }
        if !picked_up && start.elapsed() >= pickup_timeout {
            write_atomic(
                &folder.join(TEST_CANCEL),
                &serde_json::json!({"schemaVersion":"0.1","runId":run.run_id,"testId":request.test_id}),
            )?;
            remove(&folder.join(TEST_REQUEST))?;
            return Err(err(
                "editor_unavailable",
                "Unity did not pick up the local test within 10 minutes.",
            ));
        }
        if start.elapsed() >= timeout {
            // The SDK may still be building. Preserve its request and recovery journal, and refuse
            // another SDK task until the observer has restored the original blueprint identity.
            if !building {
                write_atomic(
                    &folder.join(TEST_CANCEL),
                    &serde_json::json!({"schemaVersion":"0.1","runId":run.run_id,"testId":request.test_id}),
                )?;
                remove(&folder.join(TEST_REQUEST))?;
            }
            return Err(err("local_test_timeout", "The SDK did not finish the local test within 60 minutes. Inspect Unity before retrying."));
        }
        std::thread::sleep(POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use vua_orchestrator::{
        NanosTaskIdGenerator, SqliteTaskStore, SubmitRequest, SystemClock, TaskExit, TaskRuntime,
    };

    fn folder() -> PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "vua-local-handoff-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&folder).unwrap();
        folder
    }
    fn run(project: &Path) -> ProductionRun {
        serde_json::from_value(serde_json::json!({"schemaVersion":"0.1","runId":"n3-local","phase":"succeeded","taskId":"task",
            "editor":{"path":"Unity.exe","version":"2022.3.22f1"},"projectRoot":project.to_string_lossy(),"choices":[],"sources":[],
            "prefabs":[],"projectFingerprint":"p","unityFingerprint":"u","steps":[],"selection":null,"confirmedDigest":null,
            "result":{"facts":{"avatarGlobalObjectId":"owned-avatar"}},"error":null,"recovery":null})).unwrap()
    }
    fn request() -> LocalTestRequest {
        LocalTestRequest {
            test_id: "test-1".into(),
        }
    }
    fn task(
        root: &Path,
        job: impl FnOnce(&TaskContext, &TaskRuntime) -> Result<LocalTestReceipt, AppErrorV1>
            + Send
            + 'static,
    ) -> Result<LocalTestReceipt, AppErrorV1> {
        let runtime = TaskRuntime::with_sqlite(
            Arc::new(SqliteTaskStore::open(root.join("tasks.db")).unwrap()),
            Arc::new(SystemClock),
            Arc::new(NanosTaskIdGenerator::default()),
        )
        .unwrap();
        let worker = runtime.clone();
        let (send, receive) = std::sync::mpsc::channel();
        runtime
            .submit(SubmitRequest {
                correlation_id: None,
                timeout: None,
                job: Box::new(move |ctx| {
                    let result = job(ctx, &worker);
                    send.send(result).unwrap();
                    Ok(TaskExit::Done(serde_json::json!({"synthetic":true})))
                }),
            })
            .unwrap();
        receive.recv_timeout(Duration::from_secs(5)).unwrap()
    }
    fn receipt() -> serde_json::Value {
        serde_json::json!({"schemaVersion":"0.1","runId":"n3-local","testId":"test-1","outcome":"succeeded",
            "canPublishAvatars":false,"canPublishAvatarsObserved":true,"sdkSignIn":"panel"})
    }
    #[test]
    fn local_test_schema_vectors_and_the_producer_request_agree() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../schemas/unity-bridge/v5/local-test.schema.json"
        )))
        .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let vectors: Vec<serde_json::Value> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../schemas/unity-bridge/v5/local-test-vectors.json"
        )))
        .unwrap();
        for vector in vectors {
            assert_eq!(
                validator.is_valid(&vector["value"]),
                vector["valid"].as_bool().unwrap(),
                "{}",
                vector["name"]
            );
        }
        assert!(validator.is_valid(&request_json(
            &run(Path::new("synthetic-project")),
            &request(),
            "owned-avatar"
        )));
    }
    #[test]
    fn local_test_request_and_receipt_are_bound_and_do_not_carry_upload_inputs() {
        let folder = folder();
        let run = run(&folder);
        let request = request();
        let value = request_json(&run, &request, "owned-avatar");
        assert_eq!(value.as_object().unwrap().len(), 4);
        assert!(value.get("rightsConfirmed").is_none() && value.get("avatarName").is_none());
        let path = folder.join(TEST_RECEIPT);
        let mut value = receipt();
        value["testId"] = "earlier-test".into();
        write_atomic(&path, &value).unwrap();
        assert!(read_test_receipt(&path, &run, &request).unwrap().is_none());
        write_atomic(&path, &receipt()).unwrap();
        assert_eq!(
            read_test_receipt(&path, &run, &request)
                .unwrap()
                .unwrap()
                .can_publish_avatars,
            Some(false)
        );
        let mut value = receipt();
        value
            .as_object_mut()
            .unwrap()
            .remove("canPublishAvatarsObserved");
        write_atomic(&path, &value).unwrap();
        assert_eq!(
            read_test_receipt(&path, &run, &request)
                .unwrap()
                .unwrap()
                .can_publish_avatars,
            None
        );
        value["outcome"] = "done".into();
        write_atomic(&path, &value).unwrap();
        assert_eq!(
            read_test_receipt(&path, &run, &request).unwrap_err().code,
            "vua.amf_production.receipt_invalid"
        );
        fs::write(&path, vec![b' '; MAX_FILE as usize + 1]).unwrap();
        assert!(read_test_receipt(&path, &run, &request).unwrap().is_none());
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn cancellation_before_pickup_removes_intent_and_leaves_a_bound_stop() {
        let folder = folder();
        let work = folder.clone();
        let result = task(&folder, move |ctx, runtime| {
            write_atomic(
                &work.join(TEST_REQUEST),
                &request_json(&run(&work), &request(), "owned-avatar"),
            )?;
            runtime.cancel(ctx.task_id())?;
            wait_test(
                &work,
                &run(&work),
                &request(),
                ctx,
                &mut |_| {},
                Duration::from_secs(1),
                Duration::from_secs(1),
                Duration::ZERO,
            )
        });
        assert!(result.is_err());
        assert!(!folder.join(TEST_REQUEST).exists());
        assert_eq!(
            read_test(&folder.join(TEST_CANCEL), &run(&folder), &request()).unwrap()["testId"],
            "test-1"
        );
        let _ = fs::remove_dir_all(folder);
    }
    #[test]
    fn cancellation_after_building_waits_for_the_sdk_success() {
        let folder = folder();
        let work = folder.clone();
        let result = task(&folder, move |ctx, runtime| {
            write_atomic(
                &work.join(TEST_PROGRESS),
                &serde_json::json!({"schemaVersion":"0.1","runId":"n3-local","testId":"test-1","stage":"building"}),
            )?;
            runtime.cancel(ctx.task_id())?;
            let writer = work.clone();
            let thread = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(50));
                write_atomic(&writer.join(TEST_RECEIPT), &receipt()).unwrap();
            });
            let result = wait_test(
                &work,
                &run(&work),
                &request(),
                ctx,
                &mut |_| {},
                Duration::from_secs(2),
                Duration::from_secs(2),
                Duration::ZERO,
            );
            thread.join().unwrap();
            result
        });
        assert_eq!(result.unwrap().outcome, "succeeded");
        assert!(!folder.join(TEST_CANCEL).exists());
        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn a_bound_recovery_journal_prevents_false_cancellation_without_progress() {
        let folder = folder();
        let work = folder.clone();
        let result = task(&folder, move |ctx, runtime| {
            write_atomic(
                &work.join("amf-test.started"),
                &serde_json::json!({"schemaVersion":"0.1","runId":"n3-local",
                "testId":"test-1","avatarGlobalObjectId":"owned-avatar","blueprintId":"","hadPipeline":true}),
            )?;
            runtime.cancel(ctx.task_id())?;
            wait_test(
                &work,
                &run(&work),
                &request(),
                ctx,
                &mut |_| {},
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
            )
        });
        assert_eq!(
            result.unwrap_err().code,
            "vua.amf_production.local_test_timeout"
        );
        assert!(!folder.join(TEST_CANCEL).exists());
        assert!(folder.join("amf-test.started").exists());
        let _ = fs::remove_dir_all(folder);
    }
    #[test]
    fn pickup_timeout_clears_intent_but_build_timeout_keeps_recovery() {
        for building in [false, true] {
            let folder = folder();
            let work = folder.clone();
            let result = task(&folder, move |ctx, _| {
                write_atomic(
                    &work.join(TEST_REQUEST),
                    &request_json(&run(&work), &request(), "owned-avatar"),
                )?;
                if building {
                    fs::write(work.join("amf-test.started"), "synthetic recovery journal").unwrap();
                    write_atomic(
                        &work.join(TEST_PROGRESS),
                        &serde_json::json!({"schemaVersion":"0.1","runId":"n3-local","testId":"test-1","stage":"building"}),
                    )?;
                }
                wait_test(
                    &work,
                    &run(&work),
                    &request(),
                    ctx,
                    &mut |_| {},
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                )
            });
            assert_eq!(
                result.unwrap_err().code,
                if building {
                    "vua.amf_production.local_test_timeout"
                } else {
                    "vua.amf_production.editor_unavailable"
                }
            );
            assert_eq!(folder.join(TEST_REQUEST).exists(), building);
            assert_eq!(folder.join("amf-test.started").exists(), building);
            let _ = fs::remove_dir_all(folder);
        }
    }
    #[test]
    fn an_open_editor_must_have_current_bridge_files_and_no_pending_sdk_work() {
        let folder = folder();
        assert_eq!(
            ensure_current_bridge(&folder).unwrap_err().code,
            "vua.amf_production.bridge_outdated"
        );
        refresh_bridge_package(&folder).unwrap();
        assert!(ensure_current_bridge(&folder).is_ok());
        fs::write(
            folder.join("Packages/com.ph-r.vua/Editor/Amf/AmfProductionEntryPoint.LocalTest.cs"),
            "old bridge",
        )
        .unwrap();
        assert_eq!(
            ensure_current_bridge(&folder).unwrap_err().code,
            "vua.amf_production.bridge_outdated"
        );
        fs::create_dir_all(folder.join(".vua/bridge")).unwrap();
        for name in [
            "amf-upload.json",
            "amf-upload.started",
            "amf-test.json",
            "amf-test.started",
        ] {
            let path = folder.join(".vua/bridge").join(name);
            fs::write(&path, "pending").unwrap();
            assert_eq!(
                ensure_sdk_idle(&folder).unwrap_err().code,
                "vua.amf_production.sdk_busy"
            );
            fs::remove_file(path).unwrap();
        }
        assert!(ensure_sdk_idle(&folder).is_ok());
        fs::remove_dir_all(folder).unwrap();
    }
}
