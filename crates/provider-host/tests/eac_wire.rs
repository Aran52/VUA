//! Wire validation tests for `environment.inspectEac` on `vua-provider-host`.

use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fs, io::Cursor};

static ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn invoke(request: Value) -> Value {
    let root = std::env::temp_dir().join(format!(
        "vua-eac-wire-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed),
    ));
    fs::create_dir_all(&root).unwrap();
    let frame =
        json!({"frameVersion":"0.1", "frameId":"frame", "kind":"request", "payload":request});
    let mut output = Vec::new();
    vua_provider_host::run_provider_host_full(
        Cursor::new(format!("{frame}\n")),
        &mut output,
        root.join("tasks.sqlite"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    let _ = fs::remove_dir_all(&root);
    let response: Value =
        serde_json::from_str(String::from_utf8(output).unwrap().lines().next().unwrap()).unwrap();
    response["payload"].clone()
}

#[test]
fn inspect_eac_validates_envelope_and_reports_unavailable_without_environment() {
    let valid = json!({
        "contractVersion": "0.1",
        "requestId": "req-1",
        "correlationId": "corr-1",
        "kind": "query",
        "method": "environment.inspectEac",
        "params": { "schemaVersion": "0.2" }
    });
    let res = invoke(valid.clone());
    assert_eq!(res["ok"], false);
    assert_eq!(res["error"]["code"], "vua.eac_probe.unavailable");

    for invalid_override in [
        json!({ "params": { "schemaVersion": "0.1" } }),
        json!({ "params": { "schemaVersion": "0.2", "path": "C:/VRChat" } }),
        json!({ "kind": "command" }),
        json!({ "commandId": "unexpected" }),
    ] {
        let mut req = valid.clone();
        for (k, v) in invalid_override.as_object().unwrap() {
            req[k] = v.clone();
        }
        let err = invoke(req);
        assert_eq!(err["ok"], false);
        assert_eq!(err["error"]["code"], "vua.eac_probe.invalid_params");
    }
}
