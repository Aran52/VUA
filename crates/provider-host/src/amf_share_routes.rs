//! Derived image artifacts stay in AMF's private cache, outside the one-MiB wire frames.
use super::amf_production_routes::AmfProductionServices;
use super::provider_host::uuid_v7_identity;
use super::share_image;
use image::imageops::FilterType;
use serde_json::{json, Value};
use std::{fs, io::Write, path::PathBuf, sync::Arc};
use vua_orchestrator::{amf_production::production_error, AppErrorV1};

fn fail(code: &str) -> AppErrorV1 {
    production_error(code, "The shared image could not be processed.")
}
fn root(service: &AmfProductionServices) -> Result<PathBuf, AppErrorV1> {
    let root = service.engine.share_image_root();
    fs::create_dir_all(&root).map_err(|_| fail("share_image_invalid"))?;
    let actual = fs::canonicalize(&root).map_err(|_| fail("share_image_invalid"))?;
    let parent = fs::canonicalize(service.engine.share_storage_boundary())
        .map_err(|_| fail("share_image_invalid"))?;
    if !actual.starts_with(&parent) {
        return Err(fail("share_image_invalid"));
    }
    Ok(actual)
}
fn store(
    service: &AmfProductionServices,
    bytes: &[u8],
    extension: &str,
) -> Result<String, AppErrorV1> {
    let id = format!("share-{}.{}", uuid_v7_identity(), extension);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = root(service)?.join(&id);
    let mut output = options
        .open(&file)
        .map_err(|_| fail("share_image_encode"))?;
    if output
        .write_all(bytes)
        .and_then(|_| output.sync_all())
        .is_err()
    {
        drop(output);
        let _ = fs::remove_file(&file);
        return Err(fail("share_image_encode"));
    }
    Ok(id)
}
pub(crate) fn route(
    service: &Arc<AmfProductionServices>,
    method: &str,
    request: &Value,
) -> Option<Result<Value, AppErrorV1>> {
    if !matches!(
        method,
        "amf.production.exportShareImage" | "amf.production.readShareImage"
    ) {
        return None;
    }
    Some(handle(service, method, request))
}
fn handle(
    service: &AmfProductionServices,
    method: &str,
    request: &Value,
) -> Result<Value, AppErrorV1> {
    let export = method == "amf.production.exportShareImage";
    if request["kind"] != if export { "command" } else { "query" } {
        return Err(fail("params_invalid"));
    }
    let params = request["params"]
        .as_object()
        .ok_or_else(|| fail("params_invalid"))?;
    let keys: &[&str] = if export {
        &["schemaVersion", "runId", "format", "index", "code"]
    } else {
        &["schemaVersion", "inputId"]
    };
    if params.len() != keys.len()
        || keys.iter().any(|key| !params.contains_key(*key))
        || params["schemaVersion"] != "0.1"
    {
        return Err(fail("params_invalid"));
    }
    if export {
        let run = params["runId"]
            .as_str()
            .ok_or_else(|| fail("params_invalid"))?;
        let index = params["index"]
            .as_u64()
            .filter(|v| *v < 60)
            .ok_or_else(|| fail("params_invalid"))? as u32;
        let format = params["format"]
            .as_str()
            .filter(|v| matches!(*v, "png" | "gif"))
            .ok_or_else(|| fail("params_invalid"))?;
        let code = if params["code"].is_null() {
            None
        } else {
            Some(
                params["code"]
                    .as_str()
                    .filter(|v| share_image::normalize_code(v).as_deref() == Some(*v))
                    .ok_or_else(|| fail("params_invalid"))?,
            )
        };
        if service.run_busy(run) {
            return Err(fail("busy"));
        }
        if service.engine.release_state(run)?["turntable"]["current"] != true {
            return Err(fail("turntable_missing"));
        }
        let (bytes, side, frames) = if format == "png" {
            let image = share_image::bounded_image(&service.engine.turntable_frame(run, index)?)
                .map_err(|e| fail(&e))?;
            let image = image::imageops::resize(&image, 1024, 1024, FilterType::Lanczos3);
            (
                share_image::png(
                    &share_image::decorate(&image, code, share_image::Position::BottomRight)
                        .map_err(|e| fail(&e))?,
                )
                .map_err(|e| fail(&e))?,
                1024,
                1,
            )
        } else {
            share_image::animation(
                |index| {
                    share_image::bounded_image(
                        &service
                            .engine
                            .turntable_frame(run, index)
                            .map_err(|_| "share_image_invalid")?,
                    )
                },
                code,
            )
            .map_err(|e| fail(&e))?
        };
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(fail("share_image_too_large"));
        }
        let artifact = store(service, &bytes, format)?;
        return Ok(
            json!({"schemaVersion":"0.1","artifactId":artifact,"byteLength":bytes.len(),"mediaType":format!("image/{format}"),"width":side,"height":side,"frames":frames}),
        );
    }
    let input = params["inputId"]
        .as_str()
        .filter(|v| {
            v.len() == 46
                && v.is_ascii()
                && v.starts_with("input-")
                && v.ends_with(".bin")
                && v[6..42].bytes().all(|c| c.is_ascii_hexdigit() || c == b'-')
        })
        .ok_or_else(|| fail("params_invalid"))?;
    let path = root(service)?.join(input);
    let metadata = fs::symlink_metadata(&path).map_err(|_| fail("share_image_invalid"))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > share_image::MAX_IMAGE_BYTES as u64
    {
        return Err(fail("share_image_too_large"));
    }
    let bytes = fs::read(path).map_err(|_| fail("share_image_invalid"))?;
    let (code, image) = share_image::read_image(&bytes).map_err(|e| fail(&e))?;
    let image = image::imageops::thumbnail(&image, 480, 480);
    let preview = share_image::png(&image).map_err(|e| fail(&e))?;
    if preview.len() > 1024 * 1024 {
        return Err(fail("share_image_too_large"));
    }
    let artifact = store(service, &preview, "png")?;
    Ok(
        json!({"schemaVersion":"0.1","code":code,"artifactId":artifact,"byteLength":preview.len(),"mediaType":"image/png"}),
    )
}
