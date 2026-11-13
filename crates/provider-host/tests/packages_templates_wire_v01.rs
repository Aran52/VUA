//! packages-templates v0.1 wire tests (proposal 027 F5 wiring, 2026-09-20):
//! the `packages.listTemplates` read route rides the real frame loop over
//! the frozen `schemas/packages-templates/v0.1/` word list — one method,
//! ZERO parameters (the `packages.listRepos` zero-parameter precedent; the
//! template face is environment-level configuration, not per-project, so
//! the closed EMPTY params set answers `vua.packages.invalid_params` on any
//! key, at the route layer, BEFORE the gate), the NEW defaulted accessor
//! `template_capabilities` (default declared-none) gates the route BEFORE
//! the port call (absence answers the generic `vua.vpm.capability_missing`;
//! unlike the A5 face the port method HAS a default body, so a declared-
//! but-unimplemented backend CAN exist at the type level — both layers
//! answer `capability_missing`, the route gate first), the port's typed
//! refusals travel VERBATIM (the read-face pass-through discipline), an
//! EMPTY templates array is the honest zero-templates answer (a missing
//! root is a fact, never an error — the R4 precedent), and the projection
//! is pinned by the frozen schemas (the contract-side consumer loop lives
//! in `packages_templates_consumer_v01`). The served row
//! `packages.templatesOps` flips ONLY with the accessor declaration —
//! default declared-none keeps it honestly unavailable until the
//! environment implementation-verification slice flips it with the
//! VrcGetLib override (the F2 accessor law; the CLI backend has no
//! directory-root scan face and stays honestly false). The REAL backend
//! consumption (the two-root directory scan) is the environment
//! implementation slice. Everything runs against synthetic values — no
//! machine-specific facts, no network, no user VCC home.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use vua_orchestrator::{
    AppErrorV1, ErrorCategory, PackageRequestV1, ProjectRef, TemplateCapabilities,
    TemplateEntryV01, VpmBackend, VpmCapabilities,
};
use vua_provider_host::{
    provider_host::{
        PACKAGES_TEMPLATES_ENVELOPE_SCHEMA_VERSION_V01, PACKAGES_TEMPLATES_SCHEMA_VERSION_V01,
    },
    run_provider_host_full,
};

fn read_templates_schema(name: &str) -> Value {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .join("../..")
        .join("schemas/packages-templates/v0.1")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn templates_result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_templates_schema("result.schema.json")).unwrap()
}

fn schema_violations(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

fn unique_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "vua-packages-templates-wire-v01-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("root creates");
    dir
}

/// A deterministic fake engine implementing the F5 read face behind the
/// frozen capability declaration. `template_cap: false` models the default
/// declared-none backend (the trait-default absence arm — the route must
/// answer the gate BEFORE the port, so the port body panics if it is ever
/// reached with the bit false). Per-case failure injection pins the verbatim
/// pass-through. The synthetic facts cover the frozen word face: an entry
/// that exists under BOTH roots enumerates ONCE (the duplicate-name rule —
/// collapsed to the root the creation resolution order would pick),
/// id-ascending, `name` the same-value display projection of `id`.
struct TemplatesVpm {
    template_cap: bool,
    templates: Vec<TemplateEntryV01>,
    failure: Option<AppErrorV1>,
}

impl TemplatesVpm {
    fn declaring() -> Self {
        Self {
            template_cap: true,
            templates: vec![
                TemplateEntryV01 {
                    id: "Avatar".into(),
                    name: "Avatar".into(),
                },
                TemplateEntryV01 {
                    id: "Base".into(),
                    name: "Base".into(),
                },
                TemplateEntryV01 {
                    id: "World".into(),
                    name: "World".into(),
                },
            ],
            failure: None,
        }
    }

    fn undeclaring() -> Self {
        let mut fake = Self::declaring();
        fake.template_cap = false;
        fake
    }

    fn failing(code: &'static str) -> Self {
        let mut fake = Self::declaring();
        fake.failure = Some(AppErrorV1::new(
            code,
            ErrorCategory::ExternalFailure,
            "errors.vpm.templateScanFailed",
            "corr-fake-templates",
        ));
        fake
    }

    fn empty() -> Self {
        let mut fake = Self::declaring();
        fake.templates = vec![];
        fake
    }
}

impl VpmBackend for TemplatesVpm {
    fn name(&self) -> &'static str {
        "fake-templates-wire"
    }
    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: true,
            preview_install: false,
            list_packages: false,
            remove_packages: false,
            project_registry: false,
            resolve_project: false,
        }
    }
    fn template_capabilities(&self) -> TemplateCapabilities {
        TemplateCapabilities {
            list_templates: self.template_cap,
        }
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
    ) -> Result<vua_orchestrator::ChangePreviewV1, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        _confirmed_digest: &str,
    ) -> Result<Value, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn create_project(
        &self,
        _parent: &Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
        unreachable!("not exercised in this suite")
    }
    fn list_templates(&self) -> Result<Vec<TemplateEntryV01>, AppErrorV1> {
        assert!(
            self.template_cap,
            "the route gate must answer capability_missing BEFORE the port"
        );
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(self.templates.clone())
    }
}

/// Sends one query frame through the real host loop and returns the
/// response payload.
fn run_query_frame(
    database: &Path,
    vpm: Option<Arc<dyn VpmBackend>>,
    method: &str,
    params: Value,
) -> Value {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-packages-templates-v01",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-packages-templates-v01",
            "correlationId": "corr-packages-templates-v01",
            "kind": "query",
            "method": method,
            "params": params,
        },
    });
    let mut output = Vec::new();
    run_provider_host_full(
        Cursor::new(format!("{frame}\n")),
        &mut output,
        database,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        vpm,
    )
    .expect("frame loop runs");
    let frames: Vec<Value> = String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect();
    frames[0]["payload"].clone()
}

/// Sends `application.getSnapshot` and extracts the `packages.templatesOps`
/// capability row.
fn templates_capability_row(database: &Path, vpm: Option<Arc<dyn VpmBackend>>) -> Value {
    let frame = json!({
        "frameVersion": "0.1",
        "frameId": "frame-snapshot",
        "kind": "request",
        "payload": {
            "contractVersion": "0.1",
            "requestId": "req-snapshot",
            "correlationId": "corr-snapshot",
            "kind": "query",
            "method": "application.getSnapshot",
            "params": {},
        },
    });
    let mut output = Vec::new();
    run_provider_host_full(
        Cursor::new(format!("{frame}\n")),
        &mut output,
        database,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        vpm,
    )
    .expect("frame loop runs");
    let frames: Vec<Value> = String::from_utf8(output)
        .expect("output is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("output lines are frames"))
        .collect();
    let payload = &frames[0]["payload"]["value"];
    payload["capabilities"]["operations"]
        .as_array()
        .expect("operations array")
        .iter()
        .find(|row| row["operationId"] == "packages.templatesOps")
        .cloned()
        .expect("the packages.templatesOps capability row exists")
}

#[test]
fn absent_vpm_wiring_answers_the_typed_honest_absence_and_unavailable_row() {
    let root = unique_root("absent");
    let database = root.join("tasks.sqlite");

    let payload = run_query_frame(&database, None, "packages.listTemplates", json!({}));
    assert_eq!(payload["ok"], false, "absence is a typed failure");
    assert_eq!(payload["error"]["code"], "vua.packages.unavailable");
    assert_eq!(payload["error"]["category"], "unavailable");
    assert!(
        payload.get("result").is_none(),
        "absence never carries a fabricated result"
    );

    let row = templates_capability_row(&database, None);
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wired_route_projects_the_frozen_word_face_over_the_real_frame_loop() {
    let validator = templates_result_validator();
    let root = unique_root("wired");
    let database = root.join("tasks.sqlite");

    let payload = run_query_frame(
        &database,
        Some(Arc::new(TemplatesVpm::declaring())),
        "packages.listTemplates",
        json!({}),
    );
    assert_eq!(
        payload["ok"], true,
        "wired + declared must succeed: {payload}"
    );
    let value = &payload["value"];
    assert_eq!(
        value["schemaVersion"],
        PACKAGES_TEMPLATES_ENVELOPE_SCHEMA_VERSION_V01
    );
    assert_eq!(value["operation"], "packages.listTemplates");
    assert!(
        validator.is_valid(value),
        "the frozen v0.1 result schema must admit the response: {:?} {value}",
        schema_violations(&validator, value)
    );
    let result = &value["result"];
    assert_eq!(
        result["schemaVersion"],
        PACKAGES_TEMPLATES_SCHEMA_VERSION_V01
    );
    let rows = result["templates"].as_array().expect("template rows");
    assert_eq!(rows.len(), 3);
    // id-ascending — the frozen presentation fact (the duplicate-name rule
    // collapsed the both-roots entry to ONE row, resolved to the root the
    // creation resolution order would pick).
    let ids: Vec<&str> = rows.iter().map(|row| row["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["Avatar", "Base", "World"]);
    // The same-value display projection: name === id for every row (the
    // producer contract the JSON Schema cannot express across keys — the
    // consumers never fabricate a friendlier label).
    for row in rows {
        assert_eq!(
            row["name"], row["id"],
            "name is the same-value projection of id"
        );
    }
    // The deliberate ceiling: the v0.1 word list carries EXACTLY id + name —
    // no metadata fact, no per-row root disclosure, no cacheSourced (the
    // face is zero-network; a constant informational field is not a fact).
    for row in rows {
        let keys: Vec<&str> = row
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            ["id", "name"],
            "no invented fields at the v0.1 word face"
        );
    }

    let row = templates_capability_row(&database, Some(Arc::new(TemplatesVpm::declaring())));
    assert_eq!(row["availability"], "available");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn trait_default_backend_answers_capability_missing_before_the_port_and_row_stays_unavailable() {
    // The F5 gate reads the NEW defaulted accessor BEFORE the port call —
    // default declared-none keeps the route on the generic capability-
    // missing arm. The port method HAS a default body, so the honest
    // structural difference holds: a declared-but-unimplemented backend CAN
    // exist at the type level (the consumer suite pins the trait default
    // answering capability_missing too) — the route gate simply answers
    // first and the port is never reached (the fake's port body panics if
    // it ever is).
    let root = unique_root("undeclared");
    let database = root.join("tasks.sqlite");
    let vpm = Arc::new(TemplatesVpm::undeclaring());

    let payload = run_query_frame(
        &database,
        Some(vpm.clone()),
        "packages.listTemplates",
        json!({}),
    );
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"]["code"], "vua.vpm.capability_missing");
    assert_eq!(payload["error"]["category"], "unavailable");
    assert!(
        payload.get("result").is_none(),
        "a gated face never carries a fabricated listing"
    );

    let row = templates_capability_row(&database, Some(vpm));
    assert_eq!(row["availability"], "unavailable");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn port_refusals_travel_verbatim() {
    // The read-face pass-through discipline: no read-face fold exists — the
    // port's typed refusal travels verbatim as code + message key +
    // category, and a refusal never carries a fabricated listing.
    let root = unique_root("verbatim");
    let database = root.join("tasks.sqlite");
    let vpm = TemplatesVpm::failing("vua.vpm.template_scan_failed");

    let payload = run_query_frame(
        &database,
        Some(Arc::new(vpm)),
        "packages.listTemplates",
        json!({}),
    );
    assert_eq!(
        payload["ok"], false,
        "the refusal travels as a typed failure"
    );
    assert_eq!(payload["error"]["code"], "vua.vpm.template_scan_failed");
    assert_eq!(
        payload["error"]["messageKey"],
        "errors.vpm.templateScanFailed"
    );
    assert_eq!(payload["error"]["category"], "external_failure");
    assert!(
        payload.get("result").is_none(),
        "a refusal never carries a fabricated result"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn empty_listing_is_the_honest_zero_templates_answer_on_the_wire() {
    let validator = templates_result_validator();
    let root = unique_root("empty");
    let database = root.join("tasks.sqlite");

    // A missing root or an empty pair of roots is a FACT, never an error
    // (the R4 precedent): the empty listing rides the wire as a SUCCESS
    // carrying the empty array, never an error code.
    let payload = run_query_frame(
        &database,
        Some(Arc::new(TemplatesVpm::empty())),
        "packages.listTemplates",
        json!({}),
    );
    assert_eq!(payload["ok"], true, "zero templates is a real backend fact");
    assert!(
        validator.is_valid(&payload["value"]),
        "{:?}",
        schema_violations(&validator, &payload["value"])
    );
    assert_eq!(payload["value"]["result"]["templates"], json!([]));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn param_violations_answer_invalid_params_before_the_gate() {
    let root = unique_root("params");
    let database = root.join("tasks.sqlite");
    // The shape check precedes the gate: even a declared-none backend gets
    // the params verdict first (the closed EMPTY set has no key to inspect,
    // so the verdict is pure shape).
    let vpm = Some(Arc::new(TemplatesVpm::undeclaring()) as Arc<dyn VpmBackend>);

    let violations: Vec<Value> = vec![
        // Any key breaks the closed empty set (the template face is
        // environment-level configuration, not per-project — no projectPath,
        // no filter, no pagination).
        json!({ "projectPath": "C:/some/project" }),
        json!({ "unexpected": true }),
        // A non-object params is a violation.
        json!(null),
        json!(""),
        json!([1, 2]),
    ];
    for params in violations {
        let payload = run_query_frame(
            &database,
            vpm.clone(),
            "packages.listTemplates",
            params.clone(),
        );
        assert_eq!(payload["ok"], false, "violations are typed failures");
        assert_eq!(
            payload["error"]["code"], "vua.packages.invalid_params",
            "{params}: {payload}"
        );
        assert_eq!(payload["error"]["category"], "validation");
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn envelope_consts_are_detectable_and_match_the_frozen_schema_consts() {
    // The A3/A4/A5/F2/F3 precedent closed at the wiring batch: the envelope
    // const and the family const are named HERE and pinned against the
    // frozen schema consts — a consumer can detect the row version from the
    // wire alone (the route stamps both consts verbatim, never literals).
    let command = read_templates_schema("command.schema.json");
    let result = read_templates_schema("result.schema.json");
    assert_eq!(
        command["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_TEMPLATES_ENVELOPE_SCHEMA_VERSION_V01),
        "the envelope const equals the frozen command schema's const"
    );
    assert_eq!(PACKAGES_TEMPLATES_ENVELOPE_SCHEMA_VERSION_V01, "0.1");
    assert_eq!(
        result["$defs"]["listTemplatesResult"]["properties"]["schemaVersion"]["const"],
        json!(PACKAGES_TEMPLATES_SCHEMA_VERSION_V01),
        "the family const equals the frozen result def's const"
    );
    assert_eq!(
        PACKAGES_TEMPLATES_SCHEMA_VERSION_V01,
        "vua.packages-templates/v0.1"
    );

    // The live stamp: the one response carries both consts (pinned against
    // the constants, not against string literals).
    let root = unique_root("envelope");
    let database = root.join("tasks.sqlite");
    let payload = run_query_frame(
        &database,
        Some(Arc::new(TemplatesVpm::declaring())),
        "packages.listTemplates",
        json!({}),
    );
    assert_eq!(
        payload["value"]["schemaVersion"],
        PACKAGES_TEMPLATES_ENVELOPE_SCHEMA_VERSION_V01
    );
    assert_eq!(
        payload["value"]["result"]["schemaVersion"],
        PACKAGES_TEMPLATES_SCHEMA_VERSION_V01
    );
    fs::remove_dir_all(&root).ok();
}
