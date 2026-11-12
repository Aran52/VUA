//! Envelope contract tests (E-S0, ORC-TYP-005, ORC-IPC-010).
//!
//! The Rust side and the TypeScript side (packages/contracts) both consume the
//! same fixtures from `schemas/orchestrator/envelope-v1/fixtures`. Changing an
//! envelope shape without updating schema, fixtures and both consumers fails
//! here first.

use std::fs;
use std::path::PathBuf;
use vua_orchestrator::{
    AppErrorV1, CommandAcceptedV1, ErrorCategory, ParamValue, TaskEventKind, TaskEventV1,
    TaskState, UnityOperation, ENVELOPE_SCHEMA_VERSION,
};

fn envelope_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/orchestrator/envelope-v1")
}

fn read_json(relative: &str) -> serde_json::Value {
    let bytes = fs::read(envelope_dir().join(relative)).expect("fixture/schema must exist");
    serde_json::from_slice(&bytes).expect("fixture/schema must be valid JSON")
}

fn unity_bridge_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/unity-bridge/v1")
}

fn download_events_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/download-events/v0.1")
}

fn bdl_queries_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/bdl-queries/v0.3")
}

#[test]
fn orc_typ_005_schema_documents_exist_and_pin_schema_version_one() {
    for name in [
        "app-error.schema.json",
        "command-accepted.schema.json",
        "task-event.schema.json",
    ] {
        let schema = read_json(name);
        assert_eq!(schema["properties"]["schemaVersion"]["const"], 1, "{name}");
    }
    let states = read_json("task-state.schema.json")["enum"]
        .as_array()
        .expect("task states enum")
        .clone();
    assert_eq!(states.len(), 9);
}

#[test]
fn orc_typ_005_valid_app_error_fixture_deserializes_and_roundtrips() {
    let fixture = read_json("fixtures/app-error.valid.json");
    let error: AppErrorV1 = serde_json::from_value(fixture.clone()).unwrap();
    assert_eq!(error.schema_version, ENVELOPE_SCHEMA_VERSION);
    assert_eq!(error.category, ErrorCategory::ExternalFailure);
    assert!(error.recoverable);
    assert_eq!(
        error.params.as_ref().unwrap()["package"],
        ParamValue::Text("com.example.toolkit".into())
    );
    assert_eq!(
        error.params.as_ref().unwrap()["exitCode"],
        ParamValue::Number(2.0)
    );

    let roundtrip: AppErrorV1 =
        serde_json::from_value(serde_json::to_value(&error).unwrap()).unwrap();
    assert_eq!(error, roundtrip);
}

#[test]
fn orc_ipc_010_unknown_error_category_is_rejected() {
    let fixture = read_json("fixtures/app-error.invalid.json");
    let result = serde_json::from_value::<AppErrorV1>(fixture);
    assert!(
        result.is_err(),
        "an unknown category must fail deserialization"
    );
}

#[test]
fn orc_typ_005_valid_command_accepted_fixture_deserializes() {
    let fixture = read_json("fixtures/command-accepted.valid.json");
    let accepted: CommandAcceptedV1 = serde_json::from_value(fixture).unwrap();
    assert_eq!(accepted.accepted_revision, 1);
    assert_eq!(accepted.initial_state, TaskState::Queued);
}

#[test]
fn orc_typ_005_valid_task_event_fixture_deserializes() {
    let fixture = read_json("fixtures/task-event.valid.json");
    let event: TaskEventV1 = serde_json::from_value(fixture).unwrap();
    assert_eq!(event.revision, 4);
    assert_eq!(event.kind, TaskEventKind::StateChanged);
    assert_eq!(event.state, TaskState::Running);
    assert_eq!(event.payload["step"], "install_package");
}

/// 真 JSON Schema 校验（外部审阅 Fix 7 的补强）：fixtures 与提案示例
/// 不只过 serde 反序列化，还要真的过一遍 schema 文档。H-IPC 把 bindings
/// 也接上同一来源。
#[test]
fn orc_typ_005_fixtures_validate_against_the_actual_json_schemas(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = envelope_dir();
    let error_schema = serde_json::from_value::<serde_json::Value>(
        serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(dir.join("app-error.schema.json")).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let accepted_schema = serde_json::from_value::<serde_json::Value>(
        serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(dir.join("command-accepted.schema.json")).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let event_schema = serde_json::from_value::<serde_json::Value>(
        serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(dir.join("task-event.schema.json")).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();

    // $id 是绝对 URI，$ref 指向同目录的 task-state.schema.json——注册进
    // 本地 registry 解析，绝不允许校验器联网（CI 纪律）。
    let state_schema = read_json("task-state.schema.json");
    let registry = jsonschema::Registry::new()
        .add(state_schema["$id"].as_str().unwrap(), state_schema.clone())?
        .prepare()?;
    let error_validator = jsonschema::options()
        .with_registry(&registry)
        .build(&error_schema)?;
    let accepted_validator = jsonschema::options()
        .with_registry(&registry)
        .build(&accepted_schema)?;
    let event_validator = jsonschema::options()
        .with_registry(&registry)
        .build(&event_schema)?;
    let _ = (&error_validator, &accepted_validator, &event_validator);

    let app_error = read_json("fixtures/app-error.valid.json");
    assert!(error_validator.is_valid(&app_error));
    let accepted = read_json("fixtures/command-accepted.valid.json");
    assert!(accepted_validator.is_valid(&accepted));
    let event = read_json("fixtures/task-event.valid.json");
    assert!(event_validator.is_valid(&event));

    // 无效 fixture 同样要被 schema 拒绝（与 serde 拒绝互相印证）。
    let invalid = read_json("fixtures/app-error.invalid.json");
    assert!(!error_validator.is_valid(&invalid));
    Ok(())
}

/// Recipe v0.2 测试格式的示例文档要通过同目录 schema（E-RECIPE 补强）。
#[test]
fn orc_typ_005_recipe_v0_2_example_validates_against_its_schema() {
    let recipe_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/recipe/v0.2");
    let schema_value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(recipe_dir.join("recipe.schema.json")).unwrap())
            .unwrap();
    let validator = jsonschema::validator_for(&schema_value).unwrap();
    let example: serde_json::Value =
        serde_json::from_slice(&std::fs::read(recipe_dir.join("example.recipe.json")).unwrap())
            .unwrap();
    let errors: Vec<String> = validator
        .iter_errors(&example)
        .map(|error| format!("{}: {}", error.instance_path(), error))
        .collect();
    assert!(errors.is_empty(), "schema violations: {errors:?}");

    let mut former_v2 = example;
    let object = former_v2.as_object_mut().unwrap();
    object.remove("formatVersion");
    object.insert("schemaVersion".into(), 2.into());
    assert!(
        !validator.is_valid(&former_v2),
        "the former integer Recipe v2 marker must not be accepted as v0.2"
    );
}

#[test]
fn orc_typ_005_unity_bridge_examples_and_operation_enum_stay_in_sync() {
    let dir = unity_bridge_dir();
    for (schema_name, example_name) in [
        ("command.schema.json", "examples/inspect.request.json"),
        (
            "command.schema.json",
            "examples/import-unitypackage.request.json",
        ),
        (
            "command.schema.json",
            "examples/create-local-vpm.request.json",
        ),
        ("result.schema.json", "examples/inspect.result.json"),
    ] {
        let schema: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join(schema_name)).unwrap()).unwrap();
        let example: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join(example_name)).unwrap()).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&example)
            .map(|error| format!("{}: {}", error.instance_path(), error))
            .collect();
        assert!(errors.is_empty(), "{example_name}: {errors:?}");
    }

    let command_schema: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("command.schema.json")).unwrap()).unwrap();
    let schema_operations = command_schema["properties"]["operation"]["enum"]
        .as_array()
        .unwrap();
    let rust_operations = [
        UnityOperation::InspectProject,
        UnityOperation::ImportUnityPackage,
        UnityOperation::MaterializeExtractedPackage,
        UnityOperation::CreateLocalVpmPackage,
        UnityOperation::ValidateAssetPaths,
        UnityOperation::IdentifyAssets,
        UnityOperation::InstallOutfit,
        UnityOperation::CreateToggle,
        UnityOperation::ValidateAvatar,
        UnityOperation::AnalyzePerformance,
    ]
    .map(|operation| serde_json::to_value(operation).unwrap());
    assert_eq!(schema_operations, &rust_operations);
}

/// 冻结的下载事件词表（v0.1）：七个正例必须过 schema，两个负例（端口不可
/// 如实归因的 network 失败、旁路用户路径字段）必须被拒绝。serde 侧的同一
/// 批 fixtures 在 `download_events.rs` 单元测试里互相印证。
#[test]
fn orc_typ_005_download_event_examples_validate_against_the_frozen_schema() {
    let dir = download_events_dir();
    let schema: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("event.schema.json")).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    for name in [
        "started",
        "progress",
        "interrupted",
        "completed",
        "cancelled",
        "failed-policy",
        "failed-unknown",
    ] {
        let example: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join(format!("examples/{name}.json"))).unwrap())
                .unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&example)
            .map(|error| format!("{}: {}", error.instance_path(), error))
            .collect();
        assert!(errors.is_empty(), "examples/{name}.json: {errors:?}");
    }

    let invalid_network: serde_json::Value = serde_json::from_slice(
        &fs::read(dir.join("examples/invalid-failed-network.json")).unwrap(),
    )
    .unwrap();
    assert!(
        !validator.is_valid(&invalid_network),
        "the port cannot honestly attribute network/disk/server failures"
    );
    let invalid_user_path: serde_json::Value = serde_json::from_slice(
        &fs::read(dir.join("examples/invalid-user-chosen-path.json")).unwrap(),
    )
    .unwrap();
    assert!(
        !validator.is_valid(&invalid_user_path),
        "storedPath is the only path field; extra path semantics are rejected"
    );
}

/// 冻结的 BDL 读取面（v0.2，五只读方法）：请求/响应示例必须过对应 schema；
/// 负例（实体过滤参数、非法 availabilityStatus 枚举值）必须被拒——v0.2 无
/// 实体存储、筛选只吃派生稳定枚举，违规即契约错误而非静默空答案；操作词表
/// 与 Rust 枚举互相钉死。
#[test]
fn orc_typ_005_bdl_query_examples_validate_against_the_frozen_schemas() {
    use vua_bdl_store::BdlQueryOperation;

    let dir = bdl_queries_dir();
    let query_schema: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("query.schema.json")).unwrap()).unwrap();
    assert_eq!(query_schema["properties"]["schemaVersion"]["const"], "0.3");
    let result_schema: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("result.schema.json")).unwrap()).unwrap();
    let query_validator = jsonschema::validator_for(&query_schema).unwrap();
    let result_validator = jsonschema::validator_for(&result_schema).unwrap();

    for name in [
        "catalog-list.request",
        "catalog-detail.request",
        "catalog-status.request",
        "warehouse-list-entries.request",
        "warehouse-entry-detail.request",
    ] {
        let example: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join(format!("examples/{name}.json"))).unwrap())
                .unwrap();
        let errors: Vec<String> = query_validator
            .iter_errors(&example)
            .map(|error| format!("{}: {}", error.instance_path(), error))
            .collect();
        assert!(errors.is_empty(), "examples/{name}.json: {errors:?}");
    }
    for name in [
        "catalog-list.result",
        "catalog-detail.result",
        "catalog-status.result",
        "warehouse-list-entries.result",
        "warehouse-entry-detail.result",
    ] {
        let example: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join(format!("examples/{name}.json"))).unwrap())
                .unwrap();
        let errors: Vec<String> = result_validator
            .iter_errors(&example)
            .map(|error| format!("{}: {}", error.instance_path(), error))
            .collect();
        assert!(errors.is_empty(), "examples/{name}.json: {errors:?}");
    }

    let invalid_kind: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("examples/invalid-entry-kind.json")).unwrap())
            .unwrap();
    assert!(
        !result_validator.is_valid(&invalid_kind),
        "the entry-kind vocabulary is closed in v0.3"
    );

    let invalid: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("examples/invalid-entity-filter.json")).unwrap())
            .unwrap();
    assert!(
        !query_validator.is_valid(&invalid),
        "entityType filtering has no v0.2 backing; it is a contract error"
    );
    let invalid: serde_json::Value = serde_json::from_slice(
        &fs::read(dir.join("examples/invalid-availability-filter.json")).unwrap(),
    )
    .unwrap();
    assert!(
        !query_validator.is_valid(&invalid),
        "filtering consumes the derived stable enum, never free-form raw words"
    );

    let schema_operations = query_schema["properties"]["operation"]["enum"]
        .as_array()
        .unwrap();
    let rust_operations = [
        BdlQueryOperation::CatalogList,
        BdlQueryOperation::CatalogDetail,
        BdlQueryOperation::CatalogStatus,
        BdlQueryOperation::WarehouseListEntries,
        BdlQueryOperation::WarehouseEntryDetail,
    ]
    .map(|operation| serde_json::to_value(operation).unwrap());
    assert_eq!(schema_operations, &rust_operations);
}

#[test]
fn orc_ipc_010_task_states_serialize_to_the_nine_stable_snake_case_names() {
    let pairs = [
        (TaskState::Queued, "queued"),
        (TaskState::Preparing, "preparing"),
        (TaskState::Running, "running"),
        (TaskState::WaitingForInput, "waiting_for_input"),
        (TaskState::Paused, "paused"),
        (TaskState::Succeeded, "succeeded"),
        (TaskState::SucceededWithWarnings, "succeeded_with_warnings"),
        (TaskState::Failed, "failed"),
        (TaskState::Cancelled, "cancelled"),
    ];
    for (state, name) in pairs {
        let encoded = serde_json::to_value(state).unwrap();
        assert_eq!(encoded, name);
    }
    assert!(TaskState::Failed.is_terminal());
    assert!(!TaskState::Running.is_terminal());
}
