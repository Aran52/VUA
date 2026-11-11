use crate::provider_host::{application_error, application_success, FrameOutcome};
use serde_json::{json, Value};
use vua_orchestrator::{
    deployment::{DeploymentIntent, DeploymentService},
    scenario_environment::{
        check_scenario, list_play_setups, scenario_sop, scenario_sop_v02, PlaySetupSelection,
        ScenarioSopParams,
    },
};
pub(crate) fn request(
    service: Option<&DeploymentService>,
    method: &str,
    request: &Value,
    id: &str,
    correlation: &str,
) -> FrameOutcome {
    let reject = |code: &str| {
        FrameOutcome::Response(application_error(
            id,
            correlation,
            code,
            "errors.scenario.failed",
            "validation",
        ))
    };
    if request["kind"] != "query" {
        return reject("vua.scenario.invalid_params");
    }
    let Some(params) = request["params"].as_object() else {
        return reject("vua.scenario.invalid_params");
    };
    let result = match method {
        "environment.listPlaySetups" => {
            if params.len() != 1 || params.get("schemaVersion") != Some(&json!("0.2")) {
                return reject("vua.scenario.invalid_params");
            }
            Ok(list_play_setups())
        }
        "environment.getScenarioSop" => match params.get("schemaVersion").and_then(Value::as_str) {
            Some("0.1") => {
                if params.len() != 2 || !params.contains_key("scenario") {
                    return reject("vua.scenario.invalid_params");
                }
                serde_json::from_value::<ScenarioSopParams>(params["scenario"].clone())
                    .map_err(|_| "vua.scenario.invalid_params")
                    .and_then(|p| scenario_sop(&p))
            }
            Some("0.2") => {
                if params.len() != 2 || !params.contains_key("selection") {
                    return reject("vua.scenario.invalid_params");
                }
                serde_json::from_value::<PlaySetupSelection>(params["selection"].clone())
                    .map_err(|_| "vua.scenario.invalid_params")
                    .and_then(|s| scenario_sop_v02(&s))
            }
            _ => return reject("vua.scenario.invalid_params"),
        },
        "environment.checkScenarioEnvironment" => {
            if params.len() != 2
                || params.get("schemaVersion") != Some(&json!("0.1"))
                || !params.contains_key("intent")
            {
                return reject("vua.scenario.invalid_params");
            }
            let intent = match serde_json::from_value::<DeploymentIntent>(params["intent"].clone())
            {
                Ok(value) => value,
                Err(_) => return reject("vua.scenario.invalid_params"),
            };
            if let Err(code) = intent.validate() {
                return reject(code);
            }
            let Some(service) = service else {
                return FrameOutcome::Response(application_error(
                    id,
                    correlation,
                    "vua.scenario.unavailable",
                    "errors.scenario.failed",
                    "unavailable",
                ));
            };
            check_scenario(service, &intent)
        }
        _ => return reject("vua.scenario.invalid_params"),
    };
    match result {
        Ok(value) => FrameOutcome::Response(application_success(id, value)),
        Err(code) => reject(code),
    }
}
