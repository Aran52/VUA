use crate::{
    provider_host::{application_error, application_success, FrameOutcome},
    unity_license::UnityLicenseService,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub(crate) fn request(
    service: Option<&Arc<UnityLicenseService>>,
    method: &str,
    request: &Value,
    id: &str,
    correlation: &str,
) -> FrameOutcome {
    let query = method == "environment.unityLicense";
    let params = request.get("params").and_then(Value::as_object);
    let valid = params.is_some_and(|params| {
        params.get("schemaVersion") == Some(&json!("0.1"))
            && params.len() == if query { 1 } else { 2 }
            && (query
                || params
                    .get("editorPath")
                    .and_then(Value::as_str)
                    .is_some_and(|path| {
                        !path.is_empty()
                            && path.len() <= 4096
                            && !path.chars().any(char::is_control)
                    }))
    });
    let command_id = request
        .get("commandId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 256 && !id.chars().any(char::is_control));
    if request["kind"] != if query { "query" } else { "command" }
        || !valid
        || (!query && command_id.is_none())
    {
        return FrameOutcome::Response(application_error(
            id,
            correlation,
            "vua.unity_license.invalid_params",
            "errors.unityLicense.failed",
            "validation",
        ));
    }
    let Some(service) = service else {
        return FrameOutcome::Response(application_error(
            id,
            correlation,
            "vua.unity_license.unavailable",
            "errors.unityLicense.failed",
            "unavailable",
        ));
    };
    let result = if query {
        Ok(serde_json::to_value(service.snapshot()).expect("license snapshot"))
    } else {
        service.submit(
            request["params"]["editorPath"]
                .as_str()
                .expect("validated path"),
            correlation,
            command_id.expect("validated command"),
        )
    };
    FrameOutcome::Response(match result {
        Ok(value) => application_success(id, value),
        Err(error) => application_error(
            id,
            correlation,
            &error.code,
            &error.message_key,
            crate::provider_host::app_error_category(error.category),
        ),
    })
}
