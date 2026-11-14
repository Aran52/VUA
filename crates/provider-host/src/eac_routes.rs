//! Wire adapter for `eac-probe v0.2` read-only inspection (`environment.inspectEac`).

use crate::provider_host::{application_error, application_success, FrameOutcome};
use serde_json::{json, Value};
use vua_project_manager::eac_probe::v02::EacProbeService;

pub(crate) fn request(
    service: Option<&EacProbeService>,
    request: &Value,
    id: &str,
    correlation: &str,
) -> FrameOutcome {
    let method = request["method"].as_str().unwrap_or("");
    let error = |code, category| {
        FrameOutcome::Response(application_error(
            id,
            correlation,
            code,
            "errors.eac.unavailable",
            category,
        ))
    };

    match method {
        "environment.inspectEac" => {
            let keys = [
                "contractVersion",
                "requestId",
                "correlationId",
                "kind",
                "method",
                "params",
            ];
            let valid = request
                .as_object()
                .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
                && request["contractVersion"] == "0.1"
                && request["kind"] == "query"
                && ["requestId", "correlationId"]
                    .iter()
                    .all(|key| request[*key].as_str().is_some_and(|v| !v.is_empty()))
                && request["params"]
                    .as_object()
                    .is_some_and(|p| p.len() == 1 && p.get("schemaVersion") == Some(&json!("0.2")));
            if !valid {
                return error("vua.eac_probe.invalid_params", "validation");
            }
            let Some(service) = service else {
                return error("vua.eac_probe.unavailable", "unavailable");
            };
            let snapshot = service.inspect();
            FrameOutcome::Response(application_success(id, json!({ "eacSnapshot": snapshot })))
        }
        _ => error("vua.eac_probe.invalid_params", "validation"),
    }
}
