//! Cooperative save/close handoff. A receipt alone never proves that Unity let go.
use super::*;
use std::io::Read;

const REQUEST: &str = "amf-close.json";
const RECEIPT: &str = "amf-close.result.json";
const CANCEL: &str = "amf-close.cancel";
const TIMEOUT: Duration = Duration::from_secs(120);
const POLL: Duration = Duration::from_millis(250);
const MAX_RECEIPT: u64 = 16 * 1024;

pub(super) fn close(
    run: &ProductionRun,
    ctx: &TaskContext,
) -> Result<EditorCloseResult, AppErrorV1> {
    let project = Path::new(&run.project_root);
    let folder = project.join(".vua/bridge");
    let marker = fs::read_to_string(project.join(".vua/amf-run.txt"))
        .map_err(|e| err("editor_close_failed", e))?;
    if marker.trim() != run.run_id {
        return Err(err(
            "editor_close_failed",
            "The work project's run marker changed.",
        ));
    }
    fs::create_dir_all(&folder).map_err(|e| err("editor_close_failed", e))?;
    remove(&folder.join(REQUEST))?;
    remove(&folder.join(RECEIPT))?;
    remove(&folder.join(CANCEL))?;
    let close_id = super::super::provider_host::uuid_v7_identity();
    let expires = vua_orchestrator::rfc3339(std::time::SystemTime::now() + TIMEOUT);
    let request = serde_json::json!({"schemaVersion":"0.1", "runId":run.run_id, "closeId":close_id,
        "confirmed":true, "expiresAt":expires});
    let pending = folder.join("amf-close.json.pending");
    fs::write(
        &pending,
        serde_json::to_vec(&request).map_err(|e| err("editor_close_failed", e))?,
    )
    .map_err(|e| err("editor_close_failed", e))?;
    fs::rename(&pending, folder.join(REQUEST)).map_err(|e| err("editor_close_failed", e))?;
    let result = wait(
        &folder,
        &run.run_id,
        &close_id,
        TIMEOUT,
        POLL,
        || editor_holds_project(project),
        || ctx.cancellation_error("amf-production"),
    );
    if !result.as_ref().is_ok_and(|receipt| receipt.succeeded()) {
        // The Editor checks this again after synchronous saving, before exit. A request
        // already past that final check may still finish; cancellation never kills it.
        let _ = fs::write(folder.join(CANCEL), &close_id);
    }
    // Remove an unconsumed request on every exit; expiry also prevents an old request from
    // closing an Editor that only finishes importing after the task has timed out.
    let cleanup = remove(&folder.join(REQUEST));
    match result {
        Ok(value) => {
            cleanup?;
            Ok(value)
        }
        Err(error) => Err(error),
    }
}

fn remove(path: &Path) -> Result<(), AppErrorV1> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(err("editor_close_failed", error)),
    }
}

fn read_receipt(
    path: &Path,
    run_id: &str,
    close_id: &str,
) -> Result<Option<EditorCloseResult>, AppErrorV1> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(err("editor_close_failed", error)),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_RECEIPT {
        return Err(err(
            "receipt_invalid",
            "Unity's save receipt is not a bounded regular file.",
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| file.take(MAX_RECEIPT + 1).read_to_end(&mut bytes))
        .map_err(|e| err("editor_close_failed", e))?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| err("receipt_invalid", e))?;
    if value["runId"] != run_id || value["closeId"] != close_id {
        return Ok(None);
    }
    let result: EditorCloseResult =
        serde_json::from_value(value).map_err(|e| err("receipt_invalid", e))?;
    if bytes.len() as u64 > MAX_RECEIPT
        || result.schema_version != "0.1"
        || result.editor_closed
        || !matches!(result.outcome.as_str(), "saved" | "failed")
        || result.detail.chars().count() > 1000
        || result.detail.contains('\0')
        || !matches!(
            result.reason.as_str(),
            "" | "scene_save_failed"
                | "asset_save_failed"
                | "untitled_scene"
                | "editor_busy"
                | "expired"
                | "cancelled"
                | "save_failed"
        )
        || (result.outcome == "saved"
            && (!result.scenes_saved || !result.assets_saved || !result.reason.is_empty()))
        || (result.outcome == "failed" && result.reason.is_empty())
    {
        return Err(err(
            "receipt_invalid",
            "Unity's save receipt has invalid or contradictory saving facts.",
        ));
    }
    Ok(Some(result))
}

fn wait(
    folder: &Path,
    run_id: &str,
    close_id: &str,
    timeout: Duration,
    poll: Duration,
    mut held: impl FnMut() -> bool,
    mut cancelled: impl FnMut() -> Option<AppErrorV1>,
) -> Result<EditorCloseResult, AppErrorV1> {
    let start = Instant::now();
    let mut saved = None;
    loop {
        if saved.is_none() {
            saved = read_receipt(&folder.join(RECEIPT), run_id, close_id)?;
        }
        let open = held();
        if let Some(receipt) = saved.as_mut() {
            receipt.editor_closed = !open;
            if receipt.outcome == "failed" || !open {
                return Ok(receipt.clone());
            }
        }
        if let Some(error) = cancelled() {
            return Err(error);
        }
        if start.elapsed() >= timeout {
            let mut receipt = saved.unwrap_or_else(|| EditorCloseResult {
                schema_version: "0.1".into(),
                run_id: run_id.into(),
                close_id: close_id.into(),
                outcome: "failed".into(),
                scenes_saved: false,
                assets_saved: false,
                editor_closed: !open,
                reason: String::new(),
                detail: String::new(),
            });
            receipt.reason = "timeout".into();
            receipt.detail = if receipt.scenes_saved && receipt.assets_saved {
                "Unity saved the scenes and assets but still holds the project. Finish any dialog in Unity, then try again."
            } else { "Unity did not report saving within two minutes. Keep Unity open and check its scenes and save errors; reopen the project to update an older Bridge." }.into();
            return Ok(receipt);
        }
        std::thread::sleep(poll);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vua-editor-close-{}",
            crate::provider_host::uuid_v7_identity()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
    fn receipt() -> serde_json::Value {
        serde_json::json!({"schemaVersion":"0.1", "runId":"n3-test", "closeId":"close-1", "outcome":"saved",
            "scenesSaved":true,"assetsSaved":true,"editorClosed":false,"reason":"","detail":""})
    }
    fn write(folder: &Path, value: &serde_json::Value) {
        fs::write(folder.join(RECEIPT), serde_json::to_vec(value).unwrap()).unwrap();
    }
    #[test]
    fn only_a_bound_bounded_and_consistent_save_receipt_counts() {
        let folder = folder();
        for key in ["runId", "closeId"] {
            let mut value = receipt();
            value[key] = "old".into();
            write(&folder, &value);
            assert_eq!(
                read_receipt(&folder.join(RECEIPT), "n3-test", "close-1").unwrap(),
                None
            );
        }
        for (key, value) in [
            ("schemaVersion", serde_json::json!("1")),
            ("outcome", serde_json::json!("closed")),
            ("scenesSaved", serde_json::json!(false)),
            ("assetsSaved", serde_json::json!(false)),
            ("editorClosed", serde_json::json!(true)),
            ("reason", serde_json::json!("timeout")),
            ("detail", serde_json::json!("x".repeat(1001))),
            ("extra", serde_json::json!(true)),
        ] {
            let mut v = receipt();
            v[key] = value;
            write(&folder, &v);
            assert_eq!(
                read_receipt(&folder.join(RECEIPT), "n3-test", "close-1")
                    .unwrap_err()
                    .code,
                "vua.amf_production.receipt_invalid",
                "{key}"
            );
        }
        write(&folder, &receipt());
        assert!(
            read_receipt(&folder.join(RECEIPT), "n3-test", "close-1")
                .unwrap()
                .unwrap()
                .assets_saved
        );
        fs::write(folder.join(RECEIPT), vec![b' '; MAX_RECEIPT as usize + 1]).unwrap();
        assert!(read_receipt(&folder.join(RECEIPT), "n3-test", "close-1").is_err());
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn a_save_receipt_waits_until_the_editor_releases_the_project() {
        let folder = folder();
        write(&folder, &receipt());
        let mut probes = 0;
        let result = wait(
            &folder,
            "n3-test",
            "close-1",
            Duration::from_secs(1),
            Duration::ZERO,
            || {
                probes += 1;
                probes < 3
            },
            || None,
        )
        .unwrap();
        assert_eq!(probes, 3);
        assert!(result.succeeded());
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn timeout_never_treats_saved_or_a_disappeared_lock_as_a_completed_close() {
        let folder = folder();
        write(&folder, &receipt());
        let result = wait(
            &folder,
            "n3-test",
            "close-1",
            Duration::ZERO,
            Duration::ZERO,
            || true,
            || None,
        )
        .unwrap();
        assert!(!result.succeeded());
        assert!(result.scenes_saved);
        assert_eq!(result.reason, "timeout");
        fs::remove_file(folder.join(RECEIPT)).unwrap();
        let result = wait(
            &folder,
            "n3-test",
            "close-1",
            Duration::ZERO,
            Duration::ZERO,
            || false,
            || None,
        )
        .unwrap();
        assert!(!result.succeeded());
        assert_eq!(result.reason, "timeout");
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn a_failed_save_and_cancellation_end_without_requesting_process_termination() {
        let folder = folder();
        let mut value = receipt();
        value["outcome"] = "failed".into();
        value["scenesSaved"] = false.into();
        value["assetsSaved"] = false.into();
        value["reason"] = "scene_save_failed".into();
        write(&folder, &value);
        let result = wait(
            &folder,
            "n3-test",
            "close-1",
            Duration::from_secs(1),
            Duration::ZERO,
            || true,
            || None,
        )
        .unwrap();
        assert!(!result.editor_closed);
        assert!(!result.succeeded());
        fs::remove_file(folder.join(RECEIPT)).unwrap();
        assert_eq!(
            wait(
                &folder,
                "n3-test",
                "close-1",
                Duration::from_secs(1),
                Duration::ZERO,
                || true,
                || Some(err("cancelled", "Synthetic cancellation."))
            )
            .unwrap_err()
            .code,
            "vua.amf_production.cancelled"
        );
        fs::remove_dir_all(folder).unwrap();
    }
}
