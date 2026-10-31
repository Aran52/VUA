//! E-VPM-DUAL direct tests for the VpmBackend port: digest completeness,
//! VCC CLI arg shapes, and template-based creation. Each test cites its ORC
//! requirement (ORC-TST-006).

#![allow(clippy::result_large_err)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn interactive_repo_catalog_bounds_a_stalled_refresh_and_keeps_cached_packages() {
    use std::io::Read as _;
    let base = unique_dir("catalog-read-budget");
    let root = base.join("environment");
    let cache = root.join("Repos/slow.json");
    fs::create_dir_all(cache.parent().unwrap()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/index.json", listener.local_addr().unwrap());
    let (release, held) = std::sync::mpsc::channel();
    let (started, accepted) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = [0; 4096];
        stream.read(&mut bytes).unwrap();
        started.send(()).unwrap();
        let _ = held.recv_timeout(std::time::Duration::from_secs(6));
    });
    fs::write(&cache, serde_json::json!({"repo": {
        "id":"com.vua.test.slow", "name":"Slow feed", "url":url,
        "packages":{"com.vua.test.cached":{"versions":{"1.0.0":{
            "name":"com.vua.test.cached", "version":"1.0.0", "unity":"2022.3", "vpmDependencies":{}
        }}}}
    }}).to_string()).unwrap();
    fs::write(root.join("settings.json"), serde_json::json!({
        "ignoreOfficialRepository":true, "ignoreCuratedRepository":true,
        "userRepos":[{"id":"com.vua.test.slow", "name":"Slow feed", "url":url, "localPath":cache}]
    }).to_string()).unwrap();
    let backend = VrcGetLibBackend::with_environment_root(root, false).unwrap();
    let before = std::time::Instant::now();
    let result = backend.repo_catalog(None, &[]);
    let elapsed = before.elapsed();
    release.send(()).unwrap();
    accepted.recv_timeout(std::time::Duration::from_secs(1)).expect("online refresh was attempted");
    server.join().unwrap();
    let catalog = result.unwrap();
    assert!(elapsed < std::time::Duration::from_secs(4), "interactive read held for {elapsed:?}");
    assert!(catalog.repos.iter().any(|repo| repo.cached && repo.packages.iter().any(|package| package.package_id == "com.vua.test.cached")), "a slow refresh must retain cached packages");
    fs::remove_dir_all(base).unwrap();
}

use vua_orchestrator::{
    FakeProcessRunner, FixedClock, PackageRequestV1, PackageSourceV01, ProcessOutcome,
    ProjectRef, VpmBackend, CREDENTIAL_ENV_REMOVALS,
};
use vua_project_manager::{VccCliBackend, VrcGetLibBackend};

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-vpmbe-{label}-{nanos}"))
}

fn backend_with(runner: Arc<FakeProcessRunner>) -> VccCliBackend {
    VccCliBackend::new(
        runner,
        Arc::new(FixedClock::new(&["2026-08-31T09:00:00.000Z"])),
        "vpm",
    )
}

fn success(outcome_stdout: &str) -> ProcessOutcome {
    ProcessOutcome {
        exit_code: Some(0),
        timed_out: false,
        cancelled: false,
        process_tree_clean: true,
        stdout: outcome_stdout.to_owned(),
        stderr: String::new(),
        truncated: false,
    }
}

fn minimal_vpm_project(root: &std::path::Path) -> ProjectRef {
    minimal_vpm_project_at(root, "2022.3.22f1")
}

fn minimal_vpm_project_at(root: &std::path::Path, editor_version: &str) -> ProjectRef {
    fs::create_dir_all(root.join("Packages")).unwrap();
    fs::create_dir_all(root.join("ProjectSettings")).unwrap();
    fs::write(
        root.join("Packages/manifest.json"),
        r#"{"dependencies":{}}"#,
    )
    .unwrap();
    fs::write(
        root.join("Packages/vpm-manifest.json"),
        r#"{"dependencies":{},"locked":{}}"#,
    )
    .unwrap();
    fs::write(
        root.join("ProjectSettings/ProjectVersion.txt"),
        format!("m_EditorVersion: {editor_version}\n"),
    )
    .unwrap();
    ProjectRef {
        id: "vpm-local-spike".to_owned(),
        root: root.to_owned(),
    }
}

// --- R2-2: env 剥离基线 ---

#[test]
fn orc_adp_003_credential_env_removals_strip_tokens_from_children() {
    let base = unique_dir("env");
    fs::create_dir_all(base.join("proj")).unwrap();
    let runner = Arc::new(FakeProcessRunner::new());
    runner.push(Ok(success("vpm 0.1.28")));
    let backend = backend_with(runner.clone());

    let _ = backend.create_project(&base, "proj", Some("Avatar"));

    let spec = runner.calls()[0].clone();
    assert_eq!(spec.args.first().map(String::as_str), Some("new"));
    for credential in CREDENTIAL_ENV_REMOVALS {
        assert!(
            spec.removals.iter().any(|key| key == credential),
            "{credential} must be stripped (R2-2)"
        );
    }
    // 代理变量保留：vrc-get/VCC 下载可能依赖用户代理。
    assert!(!spec.removals.iter().any(|key| key.contains("PROXY")));
    fs::remove_dir_all(&base).ok();
}

// --- VCC CLI 后端：创建参数形状（官方 vpm/cli.md 语法） ---

#[test]
fn orc_adp_004_vcc_cli_create_project_uses_documented_vpm_new_syntax() {
    let base = unique_dir("vcc-create");
    fs::create_dir_all(&base).unwrap();
    let runner = Arc::new(FakeProcessRunner::new());
    runner.push(Ok(success("vpm 0.1.28")));
    // 模拟 `vpm new` 的真实落盘：模板目录复制出项目骨架
    runner.on_run(|spec: &vua_orchestrator::ProcessSpec| {
        if spec.args.first().map(String::as_str) == Some("new") {
            let name = &spec.args[1];
            let parent = PathBuf::from(&spec.args[spec.args.len() - 1]);
            let root = parent.join(name);
            fs::create_dir_all(root.join("ProjectSettings")).unwrap();
            fs::write(
                root.join("ProjectSettings/ProjectVersion.txt"),
                "m_EditorVersion: 2022.3.22f1",
            )
            .unwrap();
        }
    });
    let backend = backend_with(runner.clone());

    let project = backend
        .create_project(&base, "My World", Some("World"))
        .unwrap();
    assert!(project.root.join(".vua/project.json").is_file());

    let spec = runner.calls()[0].clone();
    assert_eq!(
        spec.args,
        vec![
            "new".to_owned(),
            "My World".to_owned(),
            "World".to_owned(),
            "-p".to_owned(),
            base.to_string_lossy().into_owned(),
        ]
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_adp_004_vcc_cli_rejects_option_like_project_names_before_spawn() {
    let base = unique_dir("vcc-invalid-name");
    fs::create_dir_all(&base).unwrap();
    let runner = Arc::new(FakeProcessRunner::new());
    let backend = backend_with(runner.clone());

    let error = backend
        .create_project(&base, "--help", Some("Avatar"))
        .expect_err("a project name must not be reinterpreted as a CLI option");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Validation);
    assert!(runner.calls().is_empty());
    fs::remove_dir_all(&base).ok();
}

// --- 模板创建（ADR-0006 §4：复制 → productName → 校验 → 登记） ---

#[test]
fn orc_adp_004_template_creation_copies_sets_product_name_and_registers() {
    let base = unique_dir("template");
    let environment_root = base.join("VRChatCreatorCompanion");
    let template_dir = environment_root.join("VRCTemplates").join("Avatar");
    fs::create_dir_all(template_dir.join("ProjectSettings")).unwrap();
    fs::create_dir_all(template_dir.join("Packages")).unwrap();
    fs::write(
        template_dir.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1",
    )
    .unwrap();
    fs::write(
        template_dir.join("ProjectSettings/ProjectSettings.asset"),
        "someKey: 1\n  productName: TemplateProject\notherKey: 2\n",
    )
    .unwrap();
    fs::write(template_dir.join("Packages/manifest.json"), "{}").unwrap();

    let project = vua_project_manager::create_from_template(
        &environment_root,
        &base.join("workspace"),
        "My Avatar",
        Some("Avatar"),
    )
    .unwrap();

    // 模板内容已复制
    assert!(project.root.join("Packages/manifest.json").is_file());
    // productName 已替换为项目名
    let settings =
        fs::read_to_string(project.root.join("ProjectSettings/ProjectSettings.asset")).unwrap();
    assert!(settings.contains("  productName: \"My Avatar\""));
    assert!(!settings.contains("TemplateProject"));
    // 引擎登记文件存在（FileSystemProjectStore::initialize）
    assert!(project.root.join(".vua/project.json").is_file());
    fs::remove_dir_all(&base).ok();
}

#[test]
fn orc_adp_004_template_missing_is_a_typed_dependency_error() {
    let base = unique_dir("no-template");
    fs::create_dir_all(&base).unwrap();
    let error = vua_project_manager::create_from_template(
        &base,
        &base.join("workspace"),
        "Nope",
        Some("Ghost"),
    )
    .expect_err("missing template must be a typed error");
    assert_eq!(error.code, "vua.vpm.template_missing");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Dependency);
    fs::remove_dir_all(&base).ok();
}

// --- 026 A5 实现核对切片（冻结批 0c77273 ＋接线批 8abb638 落地面直读钉例） ---
// 逐码映射申报：端口错误闭集恰三既有码零新码——template_missing（库路径
// 四 i18n 键共享载体：projectExists/projectNameInvalid/templateMissing/
// templateCopyFailed）、apply_failed（CLI 超时/非零携 exitCode ＋登记腿
// 携 reason）、backend_unavailable（仅 CLI spawn 故障，库路径永不答此码）。
// 接线批把三码全折进 execution_failed guard、原端口码入 detail——本节把
// 端口侧的每一腿钉死，使折算面零缺口可证。

/// 最小 Unity 模板骨架（复制→productName→校验→登记四步全过）。
fn unity_template(template_dir: &std::path::Path, marker: &str) {
    fs::create_dir_all(template_dir.join("ProjectSettings")).unwrap();
    fs::create_dir_all(template_dir.join("Packages")).unwrap();
    fs::write(
        template_dir.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1",
    )
    .unwrap();
    fs::write(
        template_dir.join("ProjectSettings/ProjectSettings.asset"),
        format!("someKey: 1\n  productName: {marker}\notherKey: 2\n"),
    )
    .unwrap();
    fs::write(
        template_dir.join("Packages/manifest.json"),
        format!("{{\"marker\":\"{marker}\"}}"),
    )
    .unwrap();
}

/// `template: null`（接线批 REQUIRED-nullable 语义的端口侧事实）＝后端
/// 默认模板名 "Avatar"，且候选顺序第一候选 `VRCTemplates/` 优先：两级
/// 同名并存时复制的是 VCC 安装位模板，不是用户位模板。
#[test]
fn orc_adp_005_null_template_resolves_the_default_avatar_first_candidate() {
    let base = unique_dir("a5-null-default");
    let environment_root = base.join("VRChatCreatorCompanion");
    unity_template(&environment_root.join("VRCTemplates").join("Avatar"), "vrc");
    unity_template(&environment_root.join("Templates").join("Avatar"), "user");

    let project = vua_project_manager::create_from_template(
        &environment_root,
        &base.join("workspace"),
        "My Avatar",
        None,
    )
    .unwrap();

    let manifest =
        fs::read_to_string(project.root.join("Packages/manifest.json")).unwrap();
    assert!(
        manifest.contains("\"vrc\""),
        "null template must resolve through the FIRST candidate \
         (VRCTemplates/Avatar), not the user Templates/ fallback"
    );
    fs::remove_dir_all(&base).ok();
}

/// 候选顺序逐级兜底：`VRCTemplates/` 无此模板时落到用户 `Templates/`；
/// 两处皆无时第三候选＝显式路径原样直读（模板三候选默认 Avatar 三级
/// 解析冻结事实的后两级）。
#[test]
fn orc_adp_005_template_resolution_falls_through_candidates_in_order() {
    let base = unique_dir("a5-fallthrough");
    // 第二候选：仅用户位有 World 模板。
    let user_root = base.join("env-user");
    unity_template(&user_root.join("Templates").join("World"), "user-world");
    let from_user = vua_project_manager::create_from_template(
        &user_root,
        &base.join("ws-user"),
        "User World",
        Some("World"),
    )
    .unwrap();
    assert!(fs::read_to_string(from_user.root.join("Packages/manifest.json"))
        .unwrap()
        .contains("\"user-world\""));

    // 第三候选：显式路径在两级环境目录之外，原样直读。
    let explicit = base.join("elsewhere").join("MyTemplate");
    unity_template(&explicit, "explicit-path");
    let from_explicit = vua_project_manager::create_from_template(
        &base.join("env-empty"),
        &base.join("ws-explicit"),
        "Explicit",
        Some(explicit.to_string_lossy().as_ref()),
    )
    .unwrap();
    assert!(fs::read_to_string(from_explicit.root.join("Packages/manifest.json"))
        .unwrap()
        .contains("\"explicit-path\""));
    fs::remove_dir_all(&base).ok();
}

/// 不幂等（诚实边界）：目标路径已存在时 exists() 前置守卫拒绝（库路径
/// `projectExists` 键、Validation、携 name 参数），且拒绝先于模板解析——
/// 连 `template: null` 也照拒；首建工程原样无损（接线批 NO-idempotence
/// 钉例的端口侧孪生：wire 面 duplicate＝execution_failed rejected）。
#[test]
fn orc_adp_005_duplicate_target_refusal_is_honest_and_not_idempotent() {
    let base = unique_dir("a5-duplicate");
    let environment_root = base.join("VRChatCreatorCompanion");
    unity_template(&environment_root.join("VRCTemplates").join("Avatar"), "vrc");

    let first = vua_project_manager::create_from_template(
        &environment_root,
        &base.join("workspace"),
        "My Avatar",
        None,
    )
    .unwrap();

    // 同名二建（显式模板名与 null 皆然）＝同型拒绝，绝不发明幂等成功。
    for template in [Some("Avatar"), None] {
        let error = vua_project_manager::create_from_template(
            &environment_root,
            &base.join("workspace"),
            "My Avatar",
            template,
        )
        .expect_err("an existing target must be refused, never idempotent");
        assert_eq!(error.code, "vua.vpm.template_missing");
        assert_eq!(error.category, vua_orchestrator::ErrorCategory::Validation);
        assert_eq!(error.message_key, "errors.vpm.projectExists");
        assert_eq!(
            error.params.as_ref().unwrap().get("name"),
            Some(&vua_orchestrator::ParamValue::Text("My Avatar".to_owned()))
        );
    }

    // 首建工程无损：登记文件与 productName 原样。
    assert!(first.root.join(".vua/project.json").is_file());
    let settings =
        fs::read_to_string(first.root.join("ProjectSettings/ProjectSettings.asset"))
            .unwrap();
    assert!(settings.contains("  productName: \"My Avatar\""));
    fs::remove_dir_all(&base).ok();
}

/// 半成品无回滚（诚实边界）：模板非 Unity 工程（缺
/// ProjectSettings/ProjectVersion.txt）时校验腿拒绝
/// （templateCopyFailed/ExternalFailure 携 reason），已复制的目标目录
/// 原样留存——copy_tree 半成品照词面不清理、不回滚；登记文件不落
/// （绝无虚假登记）。缺失 ProjectSettings.asset 时 set_product_name
/// 的 best-effort 静默 no-op 顺带被该腿覆盖（读不到文件＝静默跳过）。
#[test]
fn orc_adp_005_half_created_target_is_never_rolled_back() {
    let base = unique_dir("a5-no-rollback");
    let environment_root = base.join("VRChatCreatorCompanion");
    let broken = environment_root.join("VRCTemplates").join("Broken");
    fs::create_dir_all(broken.join("Packages")).unwrap();
    fs::write(broken.join("Packages/manifest.json"), "{}").unwrap();

    let error = vua_project_manager::create_from_template(
        &environment_root,
        &base.join("workspace"),
        "Half Done",
        Some("Broken"),
    )
    .expect_err("a non-Unity template must be refused at the validation leg");
    assert_eq!(error.code, "vua.vpm.template_missing");
    assert_eq!(
        error.category,
        vua_orchestrator::ErrorCategory::ExternalFailure
    );
    assert_eq!(error.message_key, "errors.vpm.templateCopyFailed");
    assert_eq!(
        error.params.as_ref().unwrap().get("reason"),
        Some(&vua_orchestrator::ParamValue::Text(
            "template is not a Unity project".to_owned()
        ))
    );

    // 半成品留存：已复制内容在、无回滚、无登记。
    let target = base.join("workspace").join("Half Done");
    assert!(target.is_dir(), "the copied target must NOT be rolled back");
    assert!(target.join("Packages/manifest.json").is_file());
    assert!(
        !target.join(".vua").exists(),
        "a failed creation must never leave a registration file"
    );
    fs::remove_dir_all(&base).ok();
}

/// 四键共享载体最后一键（逐码映射闭合）：非法工程名在库路径同样拒绝
/// （validate_vpm_project_name :1511-1532 → projectNameInvalid/Validation
/// 携 name），且校验先于 exists() 守卫与模板解析（:1435 先于 :1437/:1447）
/// ——模板缺席与否不影响该腿（磁盘零触碰）。
#[test]
fn orc_adp_005_invalid_project_name_answers_project_name_invalid_before_disk() {
    let base = unique_dir("a5-invalid-name");
    let environment_root = base.join("VRChatCreatorCompanion");
    // 模板在位：拒绝只能来自名字校验，绝非模板解析。
    unity_template(&environment_root.join("VRCTemplates").join("Avatar"), "vrc");

    let error = vua_project_manager::create_from_template(
        &environment_root,
        &base.join("workspace"),
        "-leading-dash",
        Some("Avatar"),
    )
    .expect_err("an option-like name must be refused before any disk work");
    assert_eq!(error.code, "vua.vpm.template_missing");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Validation);
    assert_eq!(error.message_key, "errors.vpm.projectNameInvalid");
    assert_eq!(
        error.params.as_ref().unwrap().get("name"),
        Some(&vua_orchestrator::ParamValue::Text(
            "-leading-dash".to_owned()
        ))
    );
    assert!(
        !base.join("workspace").exists(),
        "the name guard must precede exists() and every disk touch"
    );
    fs::remove_dir_all(&base).ok();
}

/// set_product_name 第二条静默 no-op 腿（best-effort 诚实申报闭合）：
/// settings 文件在但无 productName 行时 replaced=false＝不写盘、照常
/// 成功（:1542-1582 两腿——缺文件腿由 no-rollback 例顺带覆盖，本例钉
/// 无行腿），登记不受影响（模板内容逐字节保真）。
#[test]
fn orc_adp_005_settings_without_product_name_line_is_a_silent_noop() {
    let base = unique_dir("a5-no-line-noop");
    let environment_root = base.join("VRChatCreatorCompanion");
    let template = environment_root.join("VRCTemplates").join("Avatar");
    fs::create_dir_all(template.join("ProjectSettings")).unwrap();
    fs::create_dir_all(template.join("Packages")).unwrap();
    fs::write(
        template.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1",
    )
    .unwrap();
    let settings_without_name = "someKey: 1\notherKey: 2\n";
    fs::write(
        template.join("ProjectSettings/ProjectSettings.asset"),
        settings_without_name,
    )
    .unwrap();
    fs::write(template.join("Packages/manifest.json"), "{}").unwrap();

    let project = vua_project_manager::create_from_template(
        &environment_root,
        &base.join("workspace"),
        "My Avatar",
        Some("Avatar"),
    )
    .unwrap();

    let settings =
        fs::read_to_string(project.root.join("ProjectSettings/ProjectSettings.asset"))
            .unwrap();
    assert_eq!(
        settings, settings_without_name,
        "no productName line must stay a byte-identical silent no-op"
    );
    assert!(project.root.join(".vua/project.json").is_file());
    fs::remove_dir_all(&base).ok();
}

/// CLI 后端 apply_failed 双腿（逐码映射）：超时（exit_code 缺席＝透传
/// -1）与非零退出（携 exitCode）同型应答 apply_failed/ExternalFailure
/// （:1392-1402）。
#[test]
fn orc_adp_005_vcc_cli_timeout_and_nonzero_exit_answer_apply_failed() {
    let base = unique_dir("a5-cli-apply");
    fs::create_dir_all(&base).unwrap();

    let timed_out = Arc::new(FakeProcessRunner::new());
    timed_out.push(Ok(ProcessOutcome {
        exit_code: None,
        timed_out: true,
        cancelled: false,
        process_tree_clean: true,
        stdout: String::new(),
        stderr: String::new(),
        truncated: false,
    }));
    let error = backend_with(timed_out)
        .create_project(&base, "Timed", Some("Avatar"))
        .expect_err("a timed-out vpm new must be a typed failure");
    assert_eq!(error.code, vua_orchestrator::vpm_backend_error_codes::APPLY_FAILED);
    assert_eq!(
        error.category,
        vua_orchestrator::ErrorCategory::ExternalFailure
    );
    assert_eq!(error.message_key, "errors.vpm.applyFailed");
    assert_eq!(
        error.params.as_ref().unwrap().get("exitCode"),
        Some(&vua_orchestrator::ParamValue::Number(-1.0))
    );

    let nonzero = Arc::new(FakeProcessRunner::new());
    nonzero.push(Ok(ProcessOutcome {
        exit_code: Some(3),
        timed_out: false,
        cancelled: false,
        process_tree_clean: true,
        stdout: String::new(),
        stderr: "boom".to_owned(),
        truncated: false,
    }));
    let error = backend_with(nonzero)
        .create_project(&base, "Failed", Some("Avatar"))
        .expect_err("a non-zero vpm new must be a typed failure");
    assert_eq!(error.code, vua_orchestrator::vpm_backend_error_codes::APPLY_FAILED);
    assert_eq!(
        error.params.as_ref().unwrap().get("exitCode"),
        Some(&vua_orchestrator::ParamValue::Number(3.0))
    );
    fs::remove_dir_all(&base).ok();
}

/// CLI 后端 backend_unavailable 唯一腿（逐码映射）：runner spawn 故障
/// 应答 backend_unavailable/Unavailable 携 reason（:1383-1391）——库
/// 路径永不答此码（无进程段）。
#[test]
fn orc_adp_005_vcc_cli_spawn_failure_answers_backend_unavailable() {
    let base = unique_dir("a5-cli-spawn");
    fs::create_dir_all(&base).unwrap();
    let runner = Arc::new(FakeProcessRunner::new());
    runner.push(Err("simulated spawn failure".to_owned()));

    let error = backend_with(runner)
        .create_project(&base, "Unreachable", Some("Avatar"))
        .expect_err("a spawn failure must be a typed unavailability");
    assert_eq!(
        error.code,
        vua_orchestrator::vpm_backend_error_codes::BACKEND_UNAVAILABLE
    );
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Unavailable);
    assert_eq!(error.message_key, "errors.vpm.backendUnavailable");
    assert!(matches!(
        error.params.as_ref().unwrap().get("reason"),
        Some(vua_orchestrator::ParamValue::Text(_))
    ));
    fs::remove_dir_all(&base).ok();
}

/// CLI 后端登记腿（逐码映射，接线批登记的 apply_failed 第三腿）：进程
/// 成功返回但落盘缺 Unity 工程标记时，FileSystemProjectStore::initialize
/// 失败折算 apply_failed 携 reason（:1409-1417）。
#[test]
fn orc_adp_005_vcc_cli_initialize_failure_answers_apply_failed() {
    let base = unique_dir("a5-cli-init");
    fs::create_dir_all(&base).unwrap();
    let runner = Arc::new(FakeProcessRunner::new());
    // spawn「成功」但磁盘上没有工程骨架（无 on_run 落盘模拟）。
    runner.push(Ok(success("vpm 0.1.28")));

    let error = backend_with(runner)
        .create_project(&base, "Ghost", Some("Avatar"))
        .expect_err("a successful spawn without a real project must be a typed failure");
    assert_eq!(error.code, vua_orchestrator::vpm_backend_error_codes::APPLY_FAILED);
    assert_eq!(
        error.category,
        vua_orchestrator::ErrorCategory::ExternalFailure
    );
    assert_eq!(error.message_key, "errors.vpm.applyFailed");
    assert!(matches!(
        error.params.as_ref().unwrap().get("reason"),
        Some(vua_orchestrator::ParamValue::Text(_))
    ));
    assert!(!base.join("Ghost").join(".vua").exists());
    fs::remove_dir_all(&base).ok();
}

/// 能力位核对（A5 裁定：维持既有五联位零新 accessor，库真 CLI 真）：
/// 双在库后端 create_project 位均 true，其余四值与既有声明逐位相等——
/// 结构体恰五位、零新增（新增第六位即编译破坏此钉）。served 行
/// packages.createOps 骑此位即 available（接线批落地面直读核实），
/// 环境侧零覆写动作（与 A4 repo_write_capabilities 覆写不同构）。
#[test]
fn orc_adp_005_capabilities_declare_the_create_bit_on_both_backends() {
    use vua_orchestrator::VpmCapabilities;
    let base = unique_dir("a5-caps");
    let library = VrcGetLibBackend::with_environment_root(
        base.join("isolated-vpm-environment"),
        true,
    )
    .unwrap();
    assert_eq!(
        library.capabilities(),
        VpmCapabilities {
            create_project: true,
            preview_install: true,
            list_packages: true,
            remove_packages: true,
            project_registry: true,
            resolve_project: true,
        },
        "the library backend creates in-process; exactly the existing five bits"
    );

    let cli = backend_with(Arc::new(FakeProcessRunner::new()));
    assert_eq!(
        cli.capabilities(),
        VpmCapabilities {
            create_project: true,
            preview_install: false,
            list_packages: false,
            remove_packages: false,
            project_registry: false,
            resolve_project: false,
        },
        "the CLI backend owns only vpm new; exactly the existing five bits"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn b3_spike_local_package_is_registered_previewed_and_installed_by_vrc_get() {
    let base = unique_dir("local-package");
    let environment_root = base.join("isolated-vpm-environment");
    let package_root = base.join("generated-package");
    fs::create_dir_all(package_root.join("Runtime")).unwrap();
    fs::write(package_root.join("Runtime/hello.txt"), "hello\n").unwrap();
    fs::write(
        package_root.join("package.json"),
        r#"{
  "name": "com.ph-r.vua.local.synthetic",
  "displayName": "Synthetic",
  "version": "0.0.1",
  "unity": "2022.3",
  "vpmDependencies": {}
}"#,
    )
    .unwrap();
    let project = minimal_vpm_project(&base.join("validation-project"));
    let backend = VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    backend.register_local_package(&package_root).unwrap();
    // Registration is idempotent within the private environment.
    backend.register_local_package(&package_root).unwrap();
    let request = PackageRequestV1 {
        package_id: "com.ph-r.vua.local.synthetic".to_owned(),
        version: Some("0.0.1".to_owned()),
    };
    let preview = backend
        .preview_install(&project, std::slice::from_ref(&request))
        .unwrap();
    assert_eq!(preview.items.len(), 1);
    assert_eq!(preview.items[0].package_id, request.package_id);
    assert!(!preview.destructive);

    backend
        .apply_install(&project, &[request], &preview.digest)
        .unwrap();

    let vpm_manifest = fs::read_to_string(project.root.join("Packages/vpm-manifest.json")).unwrap();
    assert!(vpm_manifest.contains("com.ph-r.vua.local.synthetic"));
    assert!(
        project
            .root
            .join("Packages/com.ph-r.vua.local.synthetic/package.json")
            .is_file(),
        "the package must be installed by vrc-get, not copied by the test"
    );
    assert!(environment_root.join("settings.json").is_file());
    fs::remove_dir_all(&base).ok();
}

#[test]
fn b3_spike_rejects_an_invalid_local_package_before_preview() {
    let base = unique_dir("invalid-local-package");
    let package_root = base.join("not-a-package");
    fs::create_dir_all(&package_root).unwrap();
    let backend =
        VrcGetLibBackend::with_environment_root(base.join("isolated-vpm-environment"), true)
            .unwrap();

    let error = backend.register_local_package(&package_root).unwrap_err();
    assert_eq!(error.code, "vua.vpm.local_package_invalid");
    fs::remove_dir_all(&base).ok();
}

#[test]
fn b3_install_roundtrip_is_digest_bound_and_honest_in_preview() {
    let base = unique_dir("b3-install-drift");
    let environment_root = base.join("isolated-vpm-environment");
    let package_root = base.join("generated-package");
    fs::create_dir_all(package_root.join("Runtime")).unwrap();
    fs::write(package_root.join("Runtime/hello.txt"), "hello\n").unwrap();
    fs::write(
        package_root.join("package.json"),
        r#"{
  "name": "com.ph-r.vua.local.synthetic",
  "displayName": "Synthetic",
  "version": "0.0.1",
  "unity": "2022.3",
  "vpmDependencies": {}
}"#,
    )
    .unwrap();
    let project = minimal_vpm_project(&base.join("install-project"));
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();
    backend.register_local_package(&package_root).unwrap();
    let request = PackageRequestV1 {
        package_id: "com.ph-r.vua.local.synthetic".to_owned(),
        version: Some("0.0.1".to_owned()),
    };
    let preview = backend
        .preview_install(&project, std::slice::from_ref(&request))
        .unwrap();
    assert_eq!(preview.items.len(), 1);
    assert_eq!(preview.items[0].kind, vua_orchestrator::ChangeKindV1::Install);

    // The double-digest discipline on the install path (026 A2 frozen word
    // face, same port-level anchor as the A1 removal test): a stale
    // confirmation is refused as a typed RECOVERABLE conflict — re-preview
    // and re-confirm, never a silent overwrite (honesty rule 3).
    let forged = format!("0{}", &preview.digest[1..]);
    assert_ne!(forged, preview.digest);
    let error = backend
        .apply_install(&project, std::slice::from_ref(&request), &forged)
        .unwrap_err();
    assert_eq!(error.code, "vua.vpm.preview_drift");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Conflict);
    assert!(
        error.recoverable,
        "digest drift is a recoverable conflict, never a terminal failure"
    );
    assert!(
        !project
            .root
            .join("Packages/com.ph-r.vua.local.synthetic/package.json")
            .is_file(),
        "a drifted apply must not install anything"
    );

    // The honest confirmation passes and really installs via vrc-get.
    backend
        .apply_install(&project, &[request], &preview.digest)
        .unwrap();
    let manifest = fs::read_to_string(project.root.join("Packages/vpm-manifest.json")).unwrap();
    assert!(
        manifest.contains("com.ph-r.vua.local.synthetic"),
        "the confirmed install is applied by vrc-get"
    );
    fs::remove_dir_all(&base).ok();
}

/// The port install receipt contract (026 A2 wiring, 2026-09-19): the
/// backend answers `{"applied": items}` — the item array the wire layer
/// lifts verbatim into the frozen audit receipt (`appliedItems`, with the
/// confirmedDigest echo and the requestedPackages rows). A result without
/// that array is a port-contract violation the wire refuses; this test
/// pins the shape the audit receipt's third part is sourced from.
#[test]
fn b3_apply_install_receipt_carries_the_applied_items() {
    let base = unique_dir("b3-install-receipt");
    let environment_root = base.join("isolated-vpm-environment");
    let package_root = base.join("generated-package");
    fs::create_dir_all(package_root.join("Runtime")).unwrap();
    fs::write(package_root.join("Runtime/hello.txt"), "hello\n").unwrap();
    fs::write(
        package_root.join("package.json"),
        r#"{
  "name": "com.ph-r.vua.local.synthetic",
  "displayName": "Synthetic",
  "version": "0.0.1",
  "unity": "2022.3",
  "vpmDependencies": {}
}"#,
    )
    .unwrap();
    let project = minimal_vpm_project(&base.join("receipt-project"));
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();
    backend.register_local_package(&package_root).unwrap();
    let request = PackageRequestV1 {
        package_id: "com.ph-r.vua.local.synthetic".to_owned(),
        version: Some("0.0.1".to_owned()),
    };
    let preview = backend
        .preview_install(&project, std::slice::from_ref(&request))
        .unwrap();

    let receipt = backend
        .apply_install(&project, &[request], &preview.digest)
        .unwrap();
    let applied = receipt
        .get("applied")
        .and_then(|value| value.as_array())
        .expect("the port receipt carries the applied item array");
    assert_eq!(applied.len(), preview.items.len());
    let item = &applied[0];
    assert_eq!(item["packageId"], "com.ph-r.vua.local.synthetic");
    assert_eq!(item["kind"], "install");
    assert_eq!(
        item["version"], "0.0.1",
        "install items carry the resolved target version"
    );
    assert!(
        item["reason"].is_null(),
        "install items carry no machine reason (removal-only field)"
    );
    fs::remove_dir_all(&base).ok();
}

/// The frozen A2 version-selection semantics at the port (026 A2 freeze
/// batch): a null request row resolves to the resolver-picked LATEST
/// STABLE (`VersionSelector::latest_for`, prereleases never auto-selected)
/// and an exact pin rides `specific_version` verbatim — the wire receipt's
/// `requestedPackages` rows carry these semantics, this test pins the port
/// behavior they project.
#[test]
fn b3_preview_install_null_version_selects_the_latest_stable() {
    let base = unique_dir("b3-install-null-version");
    let environment_root = base.join("isolated-vpm-environment");
    let package_root = base.join("generated-package");
    fs::create_dir_all(package_root.join("Runtime")).unwrap();
    fs::write(package_root.join("Runtime/hello.txt"), "hello\n").unwrap();
    fs::write(
        package_root.join("package.json"),
        r#"{
  "name": "com.ph-r.vua.local.synthetic",
  "displayName": "Synthetic",
  "version": "0.0.1",
  "unity": "2022.3",
  "vpmDependencies": {}
}"#,
    )
    .unwrap();
    let project = minimal_vpm_project(&base.join("null-version-project"));
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();
    backend.register_local_package(&package_root).unwrap();

    // null row: the resolver picks the latest stable (the only version
    // registered here), the plan item carries the resolved version.
    let null_version = PackageRequestV1 {
        package_id: "com.ph-r.vua.local.synthetic".to_owned(),
        version: None,
    };
    let preview = backend
        .preview_install(&project, std::slice::from_ref(&null_version))
        .unwrap();
    assert_eq!(preview.items.len(), 1);
    assert_eq!(
        preview.items[0].version.as_deref(),
        Some("0.0.1"),
        "a null row resolves to the latest stable version"
    );

    // exact pin: the requested string rides through verbatim.
    let pinned = PackageRequestV1 {
        package_id: "com.ph-r.vua.local.synthetic".to_owned(),
        version: Some("0.0.1".to_owned()),
    };
    let preview = backend
        .preview_install(&project, std::slice::from_ref(&pinned))
        .unwrap();
    assert_eq!(preview.items[0].version.as_deref(), Some("0.0.1"));
    fs::remove_dir_all(&base).ok();
}

/// A3 port facts (proposal 026 A3 wiring batch, 2026-09-19): the served
/// wire row `packages.registerOps` is gated on
/// `VpmBackend::register_capabilities().register_local_package` BEFORE
/// submit, so the port-level capability declaration IS the flip switch —
/// the frozen word face keeps the row honestly unavailable until the
/// environment override lands, and this test pins the flip. Same law as
/// the 025 catalog declaration: the implementing backend declares exactly
/// its implemented face; the non-implementing backend stays declared-none
/// (ORC-DEV-004: no implementation, no reservation).
#[test]
fn b3_register_capabilities_declare_exactly_the_local_package_face() {
    let base = unique_dir("b3-register-caps");
    let backend =
        VrcGetLibBackend::with_environment_root(base.join("isolated-vpm-environment"), true)
            .unwrap();

    let caps = backend.register_capabilities();
    assert!(
        caps.register_local_package,
        "the library implements registration in-process (vrc-get 0.0.16 \
         Settings::add_user_package), so the override declares it"
    );

    // VccCliBackend implements no registration arm — it stays declared-none
    // (honest absence: an undeclared face can never be requested through
    // the wire gate, and the trait-default port arm stays unreachable).
    let cli = backend_with(Arc::new(FakeProcessRunner::new()));
    assert_eq!(
        cli.register_capabilities(),
        vua_orchestrator::RegisterCapabilities::NONE,
        "no implementation, no declaration"
    );
    fs::remove_dir_all(&base).ok();
}

/// The isolated-settings I/O leg (026 A3 freeze word face): a failure
/// loading the backend environment's settings.json answers
/// `vua.vpm.local_package_register_failed` (ExternalFailure) through
/// `map_local_package_io` — never a guessed success, never a validation
/// code (the settings leg is the environment's, not the package's; the
/// package-shaped refusals own `local_package_invalid`). The wire folds
/// this code into the rejected `execution_failed` arm carrying the
/// original code in detail (per-code mapping, mapping table entry 2).
#[test]
fn b3_register_io_failure_answers_local_package_register_failed() {
    let base = unique_dir("b3-register-io");
    let environment_root = base.join("isolated-vpm-environment");
    // A package-shaped world passes the two validation legs
    // (canonicalize + package.json presence) and reaches the settings leg.
    let package_root = base.join("generated-package");
    fs::create_dir_all(package_root.join("Runtime")).unwrap();
    fs::write(
        package_root.join("package.json"),
        r#"{
  "name": "com.ph-r.vua.local.synthetic",
  "displayName": "Synthetic",
  "version": "0.0.1",
  "unity": "2022.3",
  "vpmDependencies": {}
}"#,
    )
    .unwrap();
    // The environment's settings.json as a DIRECTORY makes the settings
    // load fail with a non-NotFound io error (try_load_json answers None
    // only for a missing file — a directory open propagates), which rides
    // map_local_package_io into the typed register-failed code.
    fs::create_dir_all(environment_root.join("settings.json")).unwrap();
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();

    let error = backend.register_local_package(&package_root).unwrap_err();
    assert_eq!(error.code, "vua.vpm.local_package_register_failed");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::ExternalFailure);
    fs::remove_dir_all(&base).ok();
}

/// Idempotent set-add, environment-side evidence (026 A3 freeze word
/// face): first registration (Success) and the repeat (AlreadyAdded)
/// collapse into ONE success fact — the environment keeps EXACTLY ONE
/// userPackageFolders entry for the package root, no duplicate row is
/// ever written. This is the port-side fact the wire's idempotence test
/// (two rounds, identical registered receipts) projects.
#[test]
fn b3_register_idempotence_keeps_exactly_one_settings_entry() {
    let base = unique_dir("b3-register-idempotent");
    let environment_root = base.join("isolated-vpm-environment");
    let package_root = base.join("generated-package");
    fs::create_dir_all(package_root.join("Runtime")).unwrap();
    fs::write(
        package_root.join("package.json"),
        r#"{
  "name": "com.ph-r.vua.local.synthetic",
  "displayName": "Synthetic",
  "version": "0.0.1",
  "unity": "2022.3",
  "vpmDependencies": {}
}"#,
    )
    .unwrap();
    let backend = VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    backend.register_local_package(&package_root).unwrap();
    backend.register_local_package(&package_root).unwrap();

    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(environment_root.join("settings.json")).unwrap())
            .unwrap();
    let folders = settings["userPackageFolders"]
        .as_array()
        .expect("settings.json carries the userPackageFolders array");
    assert_eq!(
        folders.len(),
        1,
        "Success and AlreadyAdded collapse into one success fact: exactly one entry"
    );
    let registered = folders[0].as_str().unwrap();
    let canonical = std::fs::canonicalize(&package_root).unwrap();
    assert_eq!(
        std::path::PathBuf::from(registered),
        canonical,
        "the single entry is the canonicalized package root"
    );
    fs::remove_dir_all(&base).ok();
}

// --- 026 A4: repository add/remove implementation verification ---

/// Port-level capability declaration IS the served-row flip switch (the A3
/// law carried to A4): the library backend implements all three repo write
/// methods in-process (vrc-get 0.0.16 `Settings::add_remote_repo` /
/// `add_local_repo` / `remove_repo`), so the override declares the THREE
/// INDEPENDENT bits; `VccCliBackend` implements none and stays declared-none
/// (ORC-DEV-004: no implementation, no reservation). Until this override the
/// wire's `packages.repoOps` row answered honestly unavailable.
#[test]
fn b4_repo_write_capabilities_declare_exactly_the_three_bits() {
    let base = unique_dir("b4-repo-caps");
    let backend =
        VrcGetLibBackend::with_environment_root(base.join("isolated-vpm-environment"), true)
            .unwrap();

    let caps = backend.repo_write_capabilities();
    assert!(
        caps.add_remote_repo && caps.add_local_repo && caps.remove_repo,
        "the library implements all three repo write methods in-process, so \
         the override declares all three INDEPENDENT bits"
    );

    let cli = backend_with(Arc::new(FakeProcessRunner::new()));
    assert_eq!(
        cli.repo_write_capabilities(),
        vua_orchestrator::RepoWriteCapabilities::NONE,
        "no implementation, no declaration"
    );
    fs::remove_dir_all(&base).ok();
}

/// Local-directory subscription round trip through the isolated settings
/// (026 A4 word face: the directory maps onto its `repo.json`, which the
/// library persists as the row's local_path and later reads AS the manifest
/// json). The second identical add is the duplicate-path guard — the add
/// face claims NO idempotence (the A3 AlreadyAdded collapse deliberately not
/// copied): the refusal travels as `vua.vpm.repo_invalid`.
#[test]
fn b4_add_local_repo_roundtrip_and_duplicate_refusal() {
    let base = unique_dir("b4-add-local");
    let environment_root = base.join("isolated-vpm-environment");
    let repo_dir = base.join("local-repo");
    fs::create_dir_all(&repo_dir).unwrap();
    fs::write(
        repo_dir.join("repo.json"),
        r#"{"name":"synthetic-local","packages":{}}"#,
    )
    .unwrap();
    let backend = VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    backend.add_local_repo(&repo_dir, "Synthetic Local").unwrap();

    let repos = backend.list_repos().unwrap();
    assert_eq!(repos.len(), 1, "exactly one subscription row");
    let canonical_manifest = std::fs::canonicalize(repo_dir.join("repo.json")).unwrap();
    assert_eq!(
        std::path::PathBuf::from(repos[0].local_path.as_deref().unwrap()),
        canonical_manifest,
        "the row's local_path is the directory's repo.json (the library's \
         cache-path-equals-local_path law)"
    );
    assert_eq!(repos[0].name.as_deref(), Some("Synthetic Local"));
    assert!(repos[0].url.is_none(), "a local row carries no url");
    assert!(repos[0].repo_id.is_none(), "a local row carries no id — it \
        sits OUTSIDE the remove face's reach (frozen protocol boundary)");
    assert!(repos[0].cached, "repo.json exists and parses — cached=true");

    let error = backend.add_local_repo(&repo_dir, "Synthetic Local").unwrap_err();
    assert_eq!(error.code, "vua.vpm.repo_invalid");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Validation);
    fs::remove_dir_all(&base).ok();
}

/// The malformed-shape refusal (026 A4 word face: guard refusals answer
/// `vua.vpm.repo_invalid`): a directory without repo.json is exactly that —
/// the library would silently persist an unreadable row, the environment
/// refuses it before any settings write.
#[test]
fn b4_add_local_repo_rejects_directory_without_repo_json() {
    let base = unique_dir("b4-add-local-empty");
    let environment_root = base.join("isolated-vpm-environment");
    let repo_dir = base.join("empty-repo");
    fs::create_dir_all(&repo_dir).unwrap();
    let backend = VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    let error = backend.add_local_repo(&repo_dir, "Empty").unwrap_err();
    assert_eq!(error.code, "vua.vpm.repo_invalid");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Validation);
    assert!(
        !environment_root.join("settings.json").exists(),
        "a refused subscription writes no settings at all"
    );
    fs::remove_dir_all(&base).ok();
}

/// Unknown repoId answers `vua.vpm.repo_not_found` — the empty removed-row
/// list is the honest not-found, never a silent success (the frozen word
/// face's honest failure mode for remove).
#[test]
fn b4_remove_repo_unknown_id_answers_repo_not_found() {
    let base = unique_dir("b4-remove-missing");
    let backend =
        VrcGetLibBackend::with_environment_root(base.join("isolated-vpm-environment"), true)
            .unwrap();

    let error = backend.remove_repo("com.example.absent").unwrap_err();
    assert_eq!(error.code, "vua.vpm.repo_not_found");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Validation);
    fs::remove_dir_all(&base).ok();
}

/// Remove is id-addressed and reaches ONLY rows carrying that id (the
/// frozen boundary: id-absent rows are outside the face's remove reach).
/// A preset isolated settings.json carries one id-bearing remote row and
/// one id-less local row; removing by id deletes exactly the former and
/// leaves the latter — then the repeat answers repo_not_found (no
/// idempotence claimed on the remove face either: the second removal is an
/// honest not-found, not an invented success).
#[test]
fn b4_remove_repo_deletes_exactly_the_id_bearing_row() {
    let base = unique_dir("b4-remove-id");
    let environment_root = base.join("isolated-vpm-environment");
    fs::create_dir_all(&environment_root).unwrap();
    let idless_manifest = base.join("idless/repo.json");
    fs::create_dir_all(idless_manifest.parent().unwrap()).unwrap();
    fs::write(&idless_manifest, r#"{"name":"idless","packages":{}}"#).unwrap();
    let idless = std::fs::canonicalize(&idless_manifest).unwrap();
    fs::write(
        environment_root.join("settings.json"),
        format!(
            r#"{{"userRepos":[
                {{"localPath":"{}","name":"preset-remote","url":"https://example.invalid/repo.json","id":"com.example.remote"}},
                {{"localPath":"{}","name":"preset-local"}}
            ]}}"#,
            base.join("remote-cache.json").to_string_lossy().replace('\\', "\\\\"),
            idless.to_string_lossy().replace('\\', "\\\\"),
        ),
    )
    .unwrap();
    let backend = VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    backend.remove_repo("com.example.remote").unwrap();

    let repos = backend.list_repos().unwrap();
    assert_eq!(repos.len(), 1, "exactly the id-less local row survives");
    assert_eq!(repos[0].name.as_deref(), Some("preset-local"));

    let error = backend.remove_repo("com.example.remote").unwrap_err();
    assert_eq!(error.code, "vua.vpm.repo_not_found");

    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(environment_root.join("settings.json")).unwrap())
            .unwrap();
    assert_eq!(
        settings["userRepos"].as_array().unwrap().len(),
        1,
        "the persisted settings carry exactly the surviving row"
    );
    fs::remove_dir_all(&base).ok();
}

/// The fetch network segment (add_remote_repo): an unresolvable host
/// answers `vua.vpm.repo_fetch_failed` (ExternalFailure) — the honest
/// failure mode of the face's inherent network work, never a guessed
/// success and never a validation code. (The official/curated guard is the
/// library's own can_add_remote_repo, exercised by the duplicate/official
/// legs of the same repo_invalid code; no real remote is contacted by any
/// test here.)
#[test]
fn b4_add_remote_repo_unreachable_host_answers_repo_fetch_failed() {
    let base = unique_dir("b4-add-remote-fetch");
    let backend =
        VrcGetLibBackend::with_environment_root(base.join("isolated-vpm-environment"), false)
            .unwrap();

    let error = backend
        .add_remote_repo("http://127.0.0.1:1/repo.json", "Unreachable")
        .unwrap_err();
    assert_eq!(error.code, "vua.vpm.repo_fetch_failed");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::ExternalFailure);
    fs::remove_dir_all(&base).ok();
}

/// An unparseable url is the Validation leg BEFORE any network work: the
/// answer is `vua.vpm.repo_invalid` and nothing is fetched, nothing saved.
#[test]
fn b4_add_remote_repo_unparseable_url_answers_repo_invalid_without_fetch() {
    let base = unique_dir("b4-add-remote-url");
    let backend =
        VrcGetLibBackend::with_environment_root(base.join("isolated-vpm-environment"), false)
            .unwrap();

    let error = backend.add_remote_repo("not a url at all", "Broken").unwrap_err();
    assert_eq!(error.code, "vua.vpm.repo_invalid");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Validation);
    fs::remove_dir_all(&base).ok();
}


// --- B6: general project/package management path ---

/// Builds the B3 local-package world (isolated environment + generated
/// package + project) and installs the package through the digest-bound
/// path, so removal and listing tests start from a real installed state.
fn installed_world(label: &str) -> (VrcGetLibBackend, ProjectRef, PathBuf) {
    let base = unique_dir(label);
    let environment_root = base.join("isolated-vpm-environment");
    let package_root = base.join("generated-package");
    fs::create_dir_all(package_root.join("Runtime")).unwrap();
    fs::write(package_root.join("Runtime/hello.txt"), "hello\n").unwrap();
    fs::write(
        package_root.join("package.json"),
        r#"{
  "name": "com.ph-r.vua.local.synthetic",
  "displayName": "Synthetic",
  "version": "0.0.1",
  "unity": "2022.3",
  "vpmDependencies": {}
}"#,
    )
    .unwrap();
    let project = minimal_vpm_project(&base.join("managed-project"));
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();
    backend.register_local_package(&package_root).unwrap();
    let request = PackageRequestV1 {
        package_id: "com.ph-r.vua.local.synthetic".to_owned(),
        version: Some("0.0.1".to_owned()),
    };
    let preview = backend
        .preview_install(&project, std::slice::from_ref(&request))
        .unwrap();
    backend
        .apply_install(&project, &[request], &preview.digest)
        .unwrap();
    (backend, project, base)
}

#[test]
fn b6_list_packages_reports_the_installed_content_of_one_project() {
    let (backend, project, base) = installed_world("b6-list");
    let packages = backend.list_packages(&project).unwrap();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].package_id, "com.ph-r.vua.local.synthetic");
    assert_eq!(packages[0].version, "0.0.1");
    assert!(packages[0].dependencies.is_empty());
    fs::remove_dir_all(&base).ok();
}

#[test]
fn b6_remove_roundtrip_is_digest_bound_and_honest_in_preview() {
    let (backend, project, base) = installed_world("b6-remove");
    let package_ids = vec!["com.ph-r.vua.local.synthetic".to_owned()];

    let preview = backend.preview_remove(&project, &package_ids).unwrap();
    assert_eq!(preview.items.len(), 1);
    assert_eq!(
        preview.items[0].kind,
        vua_orchestrator::ChangeKindV1::Remove
    );
    assert!(!preview.destructive, "removing the only package breaks nothing");

    // The double-digest discipline: a stale confirmation is refused as a
    // typed RECOVERABLE conflict (026 A1 frozen word face: re-preview and
    // re-confirm, never a silent overwrite — honesty rule 3 at the port).
    let forged = format!("sha256-deadbeef{}", &preview.digest[14..]);
    let error = backend
        .apply_remove(&project, &package_ids, &forged)
        .unwrap_err();
    assert_eq!(error.code, "vua.vpm.preview_drift");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Conflict);
    assert!(
        error.recoverable,
        "digest drift is a recoverable conflict, never a terminal failure"
    );

    backend
        .apply_remove(&project, &package_ids, &preview.digest)
        .unwrap();
    let packages = backend.list_packages(&project).unwrap();
    assert!(packages.is_empty(), "the package is gone from the project");
    let manifest = fs::read_to_string(project.root.join("Packages/vpm-manifest.json")).unwrap();
    assert!(
        !manifest.contains("com.ph-r.vua.local.synthetic"),
        "the manifest no longer references the removed package"
    );
    fs::remove_dir_all(&base).ok();
}

/// The port removal receipt contract (026 A1 wiring, 2026-09-19): the
/// backend answers `{"removed": items}` — the item array the wire layer
/// lifts verbatim into the frozen audit receipt (`removedItems`). A result
/// without that array is a port-contract violation the wire refuses; this
/// test pins the shape the audit receipt's third part is sourced from.
#[test]
fn b6_apply_remove_receipt_carries_the_removed_items() {
    let (backend, project, base) = installed_world("b6-remove-receipt");
    let package_ids = vec!["com.ph-r.vua.local.synthetic".to_owned()];
    let preview = backend.preview_remove(&project, &package_ids).unwrap();

    let applied = backend
        .apply_remove(&project, &package_ids, &preview.digest)
        .unwrap();
    let removed = applied
        .get("removed")
        .and_then(|value| value.as_array())
        .expect("the port receipt carries the removed item array");
    assert_eq!(removed.len(), preview.items.len());
    let item = &removed[0];
    assert_eq!(item["packageId"], "com.ph-r.vua.local.synthetic");
    assert_eq!(item["kind"], "remove");
    assert!(
        item["version"].is_null(),
        "removal items carry no target version"
    );
    assert!(
        item["reason"].as_str().is_some_and(|reason| !reason.is_empty()),
        "removal items carry the machine reason"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn b6_removing_an_uninstalled_package_is_a_typed_validation_error() {
    let (backend, project, base) = installed_world("b6-remove-miss");
    let preview_result = backend.preview_remove(&project, &["com.example.absent".to_owned()]);
    let error = preview_result.expect_err("an absent package must be a typed error");
    assert_eq!(error.code, "vua.vpm.package_not_installed");
    assert_eq!(
        error.category,
        vua_orchestrator::ErrorCategory::Validation
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn b6_project_registry_reports_vcc_compatible_registrations() {
    let (backend, project, base) = installed_world("b6-registry");

    // Seed the VCC-compatible registry the way a manager does: register the
    // project through the library's own project-management API.
    let io = vrc_get_vpm::io::DefaultEnvironmentIo::new(
        base.join("isolated-vpm-environment").into_boxed_path(),
    );
    let project_io =
        vrc_get_vpm::io::DefaultProjectIo::new(project.root.clone().into_boxed_path());
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            let mut connection = vrc_get_vpm::environment::VccDatabaseConnection::connect(&io)
                .await
                .unwrap();
            let unity_project = vrc_get_vpm::UnityProject::load(project_io).await.unwrap();
            connection.add_project(&unity_project).await.unwrap();
            // The registry persists only on an explicit save.
            connection.save(&io).await.unwrap();
        });

    let registered = backend.project_registry().unwrap();
    assert_eq!(registered.len(), 1);
    assert_eq!(
        registered[0].path,
        project.root.to_string_lossy().into_owned()
    );
    assert_eq!(registered[0].name, "managed-project");
    fs::remove_dir_all(&base).ok();
}

// --- 025 P2 read faces: subscription world + per-package catalog ---

/// Writes a synthetic repository-cache file (the flat LocalCachedRepository
/// JSON shape vrc-get itself persists) with three versions of one synthetic
/// package: a yanked 0.9.0, a compatible 1.0.0, and a 2.0.0 whose minimum
/// Unity (2022.4) exceeds the 2022.3 test project. All data synthetic.
fn synthetic_repo_cache(cache_path: &std::path::Path) {
    fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
    // The LocalCachedRepository file wraps the repository document in a
    // `repo` key (the shape vrc-get itself persists).
    let cache = serde_json::json!({
        "repo": {
            "name": "Synthetic Repo",
            "id": "com.vua.test.repo.synthetic",
            "url": "https://example.invalid/vua/synthetic-repo.json",
            "packages": {
            "com.vua.test.catalog.synthetic": {
                "versions": {
                    "0.9.0": {
                        "name": "com.vua.test.catalog.synthetic",
                        "displayName": "Synthetic Catalog",
                        "version": "0.9.0",
                        "unity": "2022.3",
                        "vpmDependencies": {},
                        "vrc-get": { "yanked": true }
                    },
                    "1.0.0": {
                        "name": "com.vua.test.catalog.synthetic",
                        "displayName": "Synthetic Catalog",
                        "version": "1.0.0",
                        "unity": "2022.3",
                        "vpmDependencies": {}
                    },
                    "2.0.0": {
                        "name": "com.vua.test.catalog.synthetic",
                        "displayName": "Synthetic Catalog",
                        "version": "2.0.0",
                        "unity": "2022.4",
                        "vpmDependencies": {}
                    }
                }
            }
            }
        }
    });
    fs::write(cache_path, cache.to_string()).unwrap();
}

/// Writes a minimal VCC-shaped settings.json carrying exactly the given
/// userRepos entries (the subscription face is the world).
fn synthetic_settings(environment_root: &std::path::Path, user_repos: serde_json::Value) {
    fs::create_dir_all(environment_root).unwrap();
    fs::write(
        environment_root.join("settings.json"),
        serde_json::json!({ "userRepos": user_repos }).to_string(),
    )
    .unwrap();
}

fn p2_repo_world(label: &str) -> (VrcGetLibBackend, ProjectRef, std::path::PathBuf) {
    let base = unique_dir(label);
    let environment_root = base.join("isolated-vpm-environment");
    let cached_repo = environment_root.join("Repos").join("synthetic-repo.json");
    synthetic_repo_cache(&cached_repo);
    synthetic_settings(
        &environment_root,
        serde_json::json!([{
            "localPath": cached_repo.display().to_string(),
            "url": "https://example.invalid/vua/synthetic-repo.json"
        }]),
    );
    let project = minimal_vpm_project(&base.join("managed-project"));
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();
    (backend, project, base)
}

#[test]
fn p2_list_repos_projects_the_subscription_face_with_per_repo_cache_facts() {
    let base = unique_dir("p2-repos");
    let environment_root = base.join("isolated-vpm-environment");
    let cached_repo = environment_root.join("Repos").join("synthetic-repo.json");
    synthetic_repo_cache(&cached_repo);
    let never_refreshed = environment_root.join("Repos").join("never-refreshed.json");
    synthetic_settings(
        &environment_root,
        serde_json::json!([
            {
                "localPath": cached_repo.display().to_string(),
                "name": "Synthetic Repo",
                "id": "com.vua.test.repo.synthetic",
                "url": "https://example.invalid/vua/synthetic-repo.json"
            },
            {
                "localPath": never_refreshed.display().to_string(),
                "url": "https://example.invalid/vua/never-refreshed.json"
            },
            {
                "localPath": base.join("local-directory-repo").display().to_string()
            }
        ]),
    );
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    // The capability is declared exactly now that the faces are implemented.
    assert!(backend.catalog_capabilities().catalog);

    let rows = backend.list_repos().unwrap();
    assert_eq!(rows.len(), 3, "row order = subscription order, nothing dropped");
    let cached = &rows[0];
    assert_eq!(
        cached.repo_id.as_deref(),
        Some("com.vua.test.repo.synthetic")
    );
    assert_eq!(cached.name.as_deref(), Some("Synthetic Repo"));
    assert_eq!(
        cached.url.as_deref(),
        Some("https://example.invalid/vua/synthetic-repo.json")
    );
    assert!(cached.cached, "the refreshed cache file exists");

    let stale = &rows[1];
    assert!(!stale.cached, "subscribed-but-never-refreshed is its own honest listed state");
    assert_eq!(stale.name.as_deref(), None, "absent facts project as null");
    assert_eq!(stale.repo_id.as_deref(), None);

    let local_dir = &rows[2];
    assert_eq!(
        local_dir.url.as_deref(),
        None,
        "a local-directory repo carries no url (honest null)"
    );
    assert!(!local_dir.cached, "its repo.json was never established");

    // VccCliBackend stays declared-none with the default absence arm (zero
    // change to the five-bit closed set or its backend).
    let cli = backend_with(Arc::new(FakeProcessRunner::new()));
    assert!(!cli.catalog_capabilities().catalog);
    let error = cli.list_repos().unwrap_err();
    assert_eq!(error.code, "vua.vpm.capability_missing");
    fs::remove_dir_all(&base).ok();
}

#[test]
fn p2_list_repos_answers_an_honest_empty_subscription() {
    let base = unique_dir("p2-repos-empty");
    let environment_root = base.join("isolated-vpm-environment");
    synthetic_settings(&environment_root, serde_json::json!([]));
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();

    let rows = backend.list_repos().unwrap();
    assert!(
        rows.is_empty(),
        "zero subscriptions is a valid, honest answer"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn p2_package_catalog_reports_repo_versions_with_judgment_facts() {
    let (backend, project, base) = p2_repo_world("p2-catalog");

    let catalog = backend
        .package_catalog(&project, "com.vua.test.catalog.synthetic")
        .unwrap();
    assert_eq!(
        catalog.source,
        PackageSourceV01::Repo,
        "the package resolves from a repository cache"
    );
    assert_eq!(
        catalog.project_path,
        project.root.to_string_lossy().into_owned()
    );
    assert_eq!(catalog.package_id, "com.vua.test.catalog.synthetic");
    assert_eq!(catalog.display_name.as_deref(), Some("Synthetic Catalog"));
    assert!(!catalog.installed);
    assert_eq!(
        catalog.update_available, None,
        "not installed = judgment not executed (null), never \"no update\""
    );
    let versions: Vec<&str> = catalog
        .versions
        .iter()
        .map(|row| row.version.as_str())
        .collect();
    assert_eq!(versions, vec!["0.9.0", "1.0.0", "2.0.0"], "semver ascending");
    assert!(
        catalog.versions[0].yanked,
        "the repo-cache yank fact carries verbatim"
    );
    assert!(!catalog.versions[1].yanked);
    assert_eq!(
        catalog.versions[0].compatible, Some(true),
        "a 2022.3 minimum is satisfied by the 2022.3 project"
    );
    assert_eq!(catalog.versions[1].compatible, Some(true));
    assert_eq!(
        catalog.versions[2].compatible, Some(false),
        "2.0.0 requires a newer Unity — false, not null"
    );
    fs::remove_dir_all(&base).ok();
}

/// Synthetic repository cache for the 025 inline special-case pins: the
/// public VPM package identifiers the library's special cases key on,
/// with fully synthetic versions and unity constraints.
fn special_case_repo_cache(cache_path: &std::path::Path) {
    fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
    let cache = serde_json::json!({
        "repo": {
            "name": "Synthetic SpecialCase Repo",
            "id": "com.vua.test.repo.specialcase",
            "url": "https://example.invalid/vua/specialcase-repo.json",
            "packages": {
                "com.vrchat.avatars": {
                    "versions": {
                        "3.4.0": {
                            "name": "com.vrchat.avatars",
                            "version": "3.4.0",
                            "unity": "2019.4",
                            "vpmDependencies": {}
                        },
                        "3.5.0": {
                            "name": "com.vrchat.avatars",
                            "version": "3.5.0",
                            "unity": "2022.3",
                            "vpmDependencies": {}
                        }
                    }
                },
                "com.vrchat.core.vpm-resolver": {
                    "versions": {
                        "0.1.26": {
                            "name": "com.vrchat.core.vpm-resolver",
                            "version": "0.1.26",
                            "unity": "2019.4",
                            "vpmDependencies": {}
                        },
                        "0.1.27": {
                            "name": "com.vrchat.core.vpm-resolver",
                            "version": "0.1.27",
                            "unity": "2022.3",
                            "vpmDependencies": {}
                        }
                    }
                }
            }
        }
    });
    fs::write(cache_path, cache.to_string()).unwrap();
}

/// 025 inline thread (core stance on implementation-stance item 2: the
/// general-branch-only landing was rejected — the field must re-create the
/// FULL library `unity_compatible` semantics). Each divergence example
/// below is a case the general minimum-constraint branch alone would get
/// WRONG (it would answer true where the library answers false, or miss
/// the exact-match arm), so these pins prove the special cases are live:
/// VRCSDK 3.4 (2019-only SDK) and resolver ≤0.1.26 against a 2022 project
/// judge false; VRCSDK 3.5+ against a Unity 6000 project judges false
/// (the exact-major.minor arm exists precisely to prevent treating
/// VRCSDK-for-2022 as Unity-6000-compatible); the 2019 project keeps the
/// special-case positives; resolver 0.1.27 escapes the special case into
/// the general branch. Synthetic data only; the com.vrchat.* identifiers
/// are the public names the special cases key on.
#[test]
fn p2_package_catalog_compatible_recreates_the_full_library_special_cases() {
    let base = unique_dir("p2-specialcase");
    let environment_root = base.join("isolated-vpm-environment");
    let cached_repo = environment_root.join("Repos").join("specialcase-repo.json");
    special_case_repo_cache(&cached_repo);
    synthetic_settings(
        &environment_root,
        serde_json::json!([{
            "localPath": cached_repo.display().to_string(),
            "url": "https://example.invalid/vua/specialcase-repo.json"
        }]),
    );
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();

    let project_2022 = minimal_vpm_project(&base.join("proj-2022"));
    let project_6000 = minimal_vpm_project_at(&base.join("proj-6000"), "6000.0.23f1");
    let project_2019 = minimal_vpm_project_at(&base.join("proj-2019"), "2019.4.31f1");

    let compatible_of = |project: &ProjectRef, package: &str, version: &str| {
        let catalog = backend.package_catalog(project, package).unwrap();
        catalog
            .versions
            .iter()
            .find(|row| row.version == version)
            .unwrap_or_else(|| panic!("version {version} of {package} must be enumerated"))
            .compatible
    };

    // 2022.3 project: the 2019-only special cases judge false exactly where
    // the general branch would answer true.
    assert_eq!(
        compatible_of(&project_2022, "com.vrchat.avatars", "3.4.0"),
        Some(false),
        "VRCSDK 3.4 is 2019-only: false on a 2022 project (general branch would say true)"
    );
    assert_eq!(
        compatible_of(&project_2022, "com.vrchat.core.vpm-resolver", "0.1.26"),
        Some(false),
        "resolver ≤0.1.26 is 2019-only: false on a 2022 project (general branch would say true)"
    );
    assert_eq!(
        compatible_of(&project_2022, "com.vrchat.core.vpm-resolver", "0.1.27"),
        Some(true),
        "resolver 0.1.27 escapes the special case into the general branch"
    );
    assert_eq!(
        compatible_of(&project_2022, "com.vrchat.avatars", "3.5.0"),
        Some(true),
        "VRCSDK 3.5 with a 2022.3 constraint on a 2022.3 project: exact match holds"
    );

    // Unity 6000 project + VRCSDK 3.5+ (the divergence that motivated the
    // library's exact-match arm): library false, general branch would say
    // true (6000 >= 2022.3).
    assert_eq!(
        compatible_of(&project_6000, "com.vrchat.avatars", "3.5.0"),
        Some(false),
        "VRCSDK-for-2022 must NOT read compatible against a Unity 6000 project"
    );

    // 2019.4 project: the special-case positives hold...
    assert_eq!(
        compatible_of(&project_2019, "com.vrchat.avatars", "3.4.0"),
        Some(true),
        "VRCSDK 3.4 on its 2019 home: true"
    );
    assert_eq!(
        compatible_of(&project_2019, "com.vrchat.core.vpm-resolver", "0.1.26"),
        Some(true),
        "resolver 0.1.26 on its 2019 home: true"
    );
    // ...and the exact-match arm still rejects a 2022-constrained SDK.
    assert_eq!(
        compatible_of(&project_2019, "com.vrchat.avatars", "3.5.0"),
        Some(false),
        "VRCSDK 3.5 (2022.3 constraint) on a 2019.4 project: exact match fails"
    );

    fs::remove_dir_all(&base).ok();
}

/// 025 inline ruling 6 / packages-catalog v0.2 freeze batch: the backend
/// declares the v0.2 word face exactly when it implements the V02 method
/// (ORC-DEV-004: no implementation, no reservation), and the v0.2 result
/// carries the REQUIRED cacheSourced disclosure fact (informational, never
/// an error). In this offline synthetic world every catalog answer is
/// served through the cache-degradation path, so the fact reads true; the
/// v0.1 face keeps serving the field-less frozen shape (its type has no
/// such key, so a fabricated annotation is unrepresentable there).
#[test]
fn p2_package_catalog_v02_declares_and_discloses_the_cache_sourced_fact() {
    let (backend, project, base) = p2_repo_world("p2-catalog-v02");

    assert!(
        backend.catalog_v02(),
        "the backend declares v0.2 exactly when it implements package_catalog_v02"
    );

    let catalog = backend
        .package_catalog_v02(&project, "com.vua.test.catalog.synthetic")
        .unwrap();
    assert!(
        catalog.cache_sourced,
        "offline world = this result was served through the cache-degradation path"
    );
    // The v0.2 facts are the frozen v0.1 facts verbatim plus the disclosure.
    assert_eq!(catalog.package_id, "com.vua.test.catalog.synthetic");
    assert_eq!(catalog.project_path, project.root.to_string_lossy());
    assert_eq!(catalog.versions.len(), 3, "same enumerated facts");
    assert_eq!(catalog.versions[1].compatible, Some(true));
    fs::remove_dir_all(&base).ok();
}

#[test]
fn p2_package_catalog_judges_update_available_against_the_installed_version() {
    let (backend, project, base) = p2_repo_world("p2-update");
    let manifest_path = project.root.join("Packages/vpm-manifest.json");

    // Installed 0.9.0 (the yanked one): a strictly newer compatible version
    // (1.0.0) exists, so the frozen judgment answers true. Yanked candidates
    // and versions beyond the project's Unity are excluded by the selector.
    // Installed = the package folder under Packages/ PLUS the locked entry.
    let install = |version: &str| {
        let package_dir = project
            .root
            .join("Packages")
            .join("com.vua.test.catalog.synthetic");
        fs::create_dir_all(&package_dir).unwrap();
        fs::write(
            package_dir.join("package.json"),
            format!(
                r#"{{"name":"com.vua.test.catalog.synthetic","displayName":"Synthetic Catalog","version":"{version}","unity":"2022.3","vpmDependencies":{{}}}}"#
            ),
        )
        .unwrap();
        fs::write(
            &manifest_path,
            format!(
                r#"{{"dependencies":{{"com.vua.test.catalog.synthetic":{{"version":"{version}"}}}},"locked":{{"com.vua.test.catalog.synthetic":{{"version":"{version}","dependencies":{{}}}}}}}}"#
            ),
        )
        .unwrap();
    };
    install("0.9.0");
    let catalog = backend
        .package_catalog(&project, "com.vua.test.catalog.synthetic")
        .unwrap();
    assert!(catalog.installed);
    assert_eq!(
        catalog.update_available,
        Some(true),
        "a strictly newer compatible version exists"
    );

    // Installed 1.0.0 is already the latest compatible version: an honest
    // false conclusion, never null.
    install("1.0.0");
    let catalog = backend
        .package_catalog(&project, "com.vua.test.catalog.synthetic")
        .unwrap();
    assert_eq!(catalog.update_available, Some(false));
    fs::remove_dir_all(&base).ok();
}

#[test]
fn p2_package_catalog_local_source_is_honest_and_an_absent_package_is_typed() {
    // Register-only world: the package exists in the environment's local set
    // but is NOT installed in the project (no apply step).
    let base = unique_dir("p2-local");
    let environment_root = base.join("isolated-vpm-environment");
    let package_root = base.join("generated-package");
    fs::create_dir_all(package_root.join("Runtime")).unwrap();
    fs::write(package_root.join("Runtime/hello.txt"), "hello\n").unwrap();
    fs::write(
        package_root.join("package.json"),
        r#"{
  "name": "com.ph-r.vua.local.synthetic",
  "displayName": "Synthetic",
  "version": "0.0.1",
  "unity": "2022.3",
  "vpmDependencies": {}
}"#,
    )
    .unwrap();
    let project = minimal_vpm_project(&base.join("managed-project"));
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();
    backend.register_local_package(&package_root).unwrap();

    let catalog = backend
        .package_catalog(&project, "com.ph-r.vua.local.synthetic")
        .unwrap();
    assert_eq!(
        catalog.source,
        PackageSourceV01::Local,
        "registered in the environment without any repository"
    );
    assert!(
        catalog.versions.is_empty(),
        "a local-source package honestly carries no repo versions"
    );
    assert!(
        !catalog.installed,
        "registered in the environment is not installed in this project"
    );
    assert_eq!(catalog.update_available, None);
    assert_eq!(catalog.display_name.as_deref(), Some("Synthetic"));

    let error = backend
        .package_catalog(&project, "com.example.absent")
        .unwrap_err();
    assert_eq!(
        error.code, "vua.vpm.no_matching_package",
        "absent from both repo caches and the local set is the reused typed code"
    );
    fs::remove_dir_all(&base).ok();
}

// --- 027 F2 read face: per-repository installable-package inventory ---

/// Builds the F2 repo-catalog world: an isolated environment subscribing two
/// synthetic repositories — one refreshed (cache file in place, id+name in
/// the document) and one never refreshed — exercising the face's whole world
/// (loaded rows + not-loaded remainder incl. the two predefined caches).
/// All data synthetic; the backend runs offline (cache-degradation path).
fn f2_repo_world(label: &str) -> (VrcGetLibBackend, std::path::PathBuf) {
    let base = unique_dir(label);
    let environment_root = base.join("isolated-vpm-environment");
    let cached_repo = environment_root.join("Repos").join("synthetic-repo.json");
    synthetic_repo_cache(&cached_repo);
    let never_refreshed = environment_root.join("Repos").join("never-refreshed.json");
    fs::create_dir_all(&environment_root).unwrap();
    fs::write(
        environment_root.join("settings.json"),
        serde_json::json!({
            "userRepos": [
                {
                    "localPath": cached_repo.display().to_string(),
                    "name": "Synthetic Repo",
                    "id": "com.vua.test.repo.synthetic",
                    "url": "https://example.invalid/vua/synthetic-repo.json"
                },
                {
                    "localPath": never_refreshed.display().to_string(),
                    "name": "Never Refreshed",
                    "id": "com.vua.test.repo.never",
                    "url": "https://example.invalid/vua/never-refreshed.json"
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();
    (backend, base)
}

#[test]
fn f2_repo_catalog_projects_the_collection_world_per_repository() {
    let (backend, base) = f2_repo_world("f2-catalog");

    // The capability is declared exactly now that the face is implemented
    // (ORC-DEV-004: no implementation, no reservation) — this override is
    // what flips the served row packages.repoCatalogOps to available.
    assert!(backend.repo_catalog_capabilities().repo_catalog);

    let catalog = backend.repo_catalog(None, &[]).unwrap();
    assert!(
        catalog.cache_sourced,
        "offline world = this result was served through the cache-degradation path"
    );
    assert_eq!(
        catalog.repos.len(),
        4,
        "world = the loaded row + the not-loaded remainder (official/curated/never-refreshed)"
    );

    // Loaded rows first, in the collection's own enumeration order (one here);
    // identity comes from the cache document itself.
    let synthetic = &catalog.repos[0];
    assert_eq!(
        synthetic.repo_id.as_deref(),
        Some("com.vua.test.repo.synthetic")
    );
    assert_eq!(synthetic.name.as_deref(), Some("Synthetic Repo"));
    assert!(synthetic.cached, "the refreshed cache file is a collection hit");
    assert_eq!(synthetic.packages.len(), 1);
    let package = &synthetic.packages[0];
    assert_eq!(package.package_id, "com.vua.test.catalog.synthetic");
    assert_eq!(package.display_name.as_deref(), Some("Synthetic Catalog"));
    assert_eq!(
        package.description.as_deref(),
        None,
        "the manifest carries no description: honest null, never padded"
    );
    // latestVersion = the per-repo frozen judgment: yanked 0.9.0 excluded,
    // prerelease switch off, and NO project-Unity constraint (2.0.0 with
    // unity 2022.4 still qualifies — the compatible judgment stays the
    // packages-catalog face's project-bound fact).
    assert_eq!(package.latest_version.as_deref(), Some("2.0.0"));
    assert_eq!(
        package.version_count, 3,
        "cache inventory count, yanked included — a cache fact, not an availability promise"
    );

    // The not-loaded remainder in the library's own source-chain order: the
    // two predefined caches (no identity facts exist yet: honest null/null)
    // then the never-refreshed subscription (its settings-row identity), all
    // with EMPTY packages arrays — their own honest state, never hidden.
    for predefined in [&catalog.repos[1], &catalog.repos[2]] {
        assert_eq!(predefined.repo_id, None, "predefined cache not yet refreshed: no identity facts");
        assert_eq!(predefined.name, None);
        assert!(!predefined.cached);
        assert!(predefined.packages.is_empty());
    }
    let never = &catalog.repos[3];
    assert_eq!(never.repo_id.as_deref(), Some("com.vua.test.repo.never"));
    assert_eq!(never.name.as_deref(), Some("Never Refreshed"));
    assert!(
        !never.cached,
        "subscribed but never refreshed is its own honest listed state"
    );
    assert!(never.packages.is_empty(), "empty packages array, never hidden");
    fs::remove_dir_all(&base).ok();
}

#[test]
fn f2_repo_catalog_scopes_by_repo_id_and_answers_the_reused_repo_not_found() {
    let (backend, base) = f2_repo_world("f2-scope");

    let scoped = backend
        .repo_catalog(Some("com.vua.test.repo.synthetic"), &[])
        .unwrap();
    assert_eq!(
        scoped.repos.len(),
        1,
        "the scoped lens answers exactly the one repo row"
    );
    assert_eq!(
        scoped.repos[0].repo_id.as_deref(),
        Some("com.vua.test.repo.synthetic")
    );
    assert!(!scoped.repos[0].packages.is_empty());

    // A not-loaded row is reachable by its settings-row id — its only
    // existing identity fact.
    let never = backend
        .repo_catalog(Some("com.vua.test.repo.never"), &[])
        .unwrap();
    assert_eq!(never.repos.len(), 1);
    assert!(!never.repos[0].cached);
    assert!(never.repos[0].packages.is_empty());

    // An id outside the word face answers the REUSED A4 removeRepo same-fact
    // code verbatim — zero new codes, never a fabricated empty-shape success.
    let error = backend
        .repo_catalog(Some("com.vua.test.repo.unknown"), &[])
        .unwrap_err();
    assert_eq!(error.code, "vua.vpm.repo_not_found");
    assert_eq!(error.message_key, "errors.vpm.repoNotFound");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Validation);
    fs::remove_dir_all(&base).ok();
}

#[test]
fn f2_repo_catalog_package_ids_filter_is_a_lens_never_an_existence_assertion() {
    let (backend, base) = f2_repo_world("f2-filter");

    let hit = backend
        .repo_catalog(None, &["com.vua.test.catalog.synthetic".to_owned()])
        .unwrap();
    let synthetic = hit.repos.iter().find(|repo| repo.cached).unwrap();
    assert_eq!(synthetic.packages.len(), 1, "the requirement-set lens passes the match");

    // A filter matching nothing is an HONEST EMPTY answer — not an error;
    // vua.vpm.no_matching_package has no reach on this face (the single-
    // package existence assertion stays the packages-catalog face's fact).
    let miss = backend
        .repo_catalog(None, &["com.vua.test.catalog.missing".to_owned()])
        .unwrap();
    let synthetic = miss.repos.iter().find(|repo| repo.cached).unwrap();
    assert!(
        synthetic.packages.is_empty(),
        "the filter is a lens: empty here, never an error"
    );
    assert!(miss.cache_sourced);
    fs::remove_dir_all(&base).ok();
}

#[test]
fn f2_repo_catalog_loaded_predefined_cache_is_never_duplicated() {
    let base = unique_dir("f2-official");
    let environment_root = base.join("isolated-vpm-environment");
    // A refreshed official cache: the library loads Repos/vrc-official.json
    // and serves the document with the predefined subscription url stamped
    // on it — the face must answer it exactly ONCE (the loaded row), never
    // plus a second cached-false remainder row.
    let official_cache = environment_root.join("Repos").join("vrc-official.json");
    fs::create_dir_all(official_cache.parent().unwrap()).unwrap();
    let cache = serde_json::json!({
        "repo": {
            "name": "Official",
            "id": "com.vrchat.repos.official",
            "url": "https://packages.vrchat.com/official?download",
            "packages": {
                "com.vua.test.official.synthetic": {
                    "versions": {
                        "1.0.0": {
                            "name": "com.vua.test.official.synthetic",
                            "displayName": "Official Synthetic",
                            "version": "1.0.0",
                            "unity": "2022.3",
                            "vpmDependencies": {}
                        }
                    }
                }
            }
        }
    });
    fs::write(&official_cache, cache.to_string()).unwrap();
    synthetic_settings(&environment_root, serde_json::json!([]));
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();

    let catalog = backend.repo_catalog(None, &[]).unwrap();
    assert_eq!(
        catalog.repos.len(),
        2,
        "the loaded official row + the not-loaded curated row — nothing duplicated, nothing dropped"
    );
    let official = &catalog.repos[0];
    assert_eq!(
        official.repo_id.as_deref(),
        Some("com.vrchat.repos.official"),
        "identity from the loaded cache document"
    );
    assert!(official.cached);
    assert_eq!(
        official.packages[0].package_id,
        "com.vua.test.official.synthetic"
    );
    let curated = &catalog.repos[1];
    assert_eq!(curated.repo_id, None, "curated not refreshed: honest null identity, exactly once");
    assert!(!curated.cached);
    assert!(curated.packages.is_empty());

    // Scoping reaches the loaded predefined row by its document id.
    let scoped = backend
        .repo_catalog(Some("com.vrchat.repos.official"), &[])
        .unwrap();
    assert_eq!(scoped.repos.len(), 1);
    assert_eq!(
        scoped.repos[0].repo_id.as_deref(),
        Some("com.vrchat.repos.official")
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn f2_repo_catalog_answers_an_honest_empty_world_when_every_repo_is_ignored() {
    let base = unique_dir("f2-empty");
    let environment_root = base.join("isolated-vpm-environment");
    fs::create_dir_all(environment_root.join("vrc-get")).unwrap();
    fs::write(
        environment_root.join("vrc-get/settings.json"),
        serde_json::json!({
            "ignoreOfficialRepository": true,
            "ignoreCuratedRepository": true
        })
        .to_string(),
    )
    .unwrap();
    synthetic_settings(&environment_root, serde_json::json!([]));
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();

    let catalog = backend.repo_catalog(None, &[]).unwrap();
    assert!(
        catalog.repos.is_empty(),
        "zero repository caches is a legal, honest empty answer"
    );
    assert!(catalog.cache_sourced);
    fs::remove_dir_all(&base).ok();
}

#[test]
fn f2_repo_catalog_cli_backend_stays_declared_none_with_absence_arm() {
    let cli = backend_with(Arc::new(FakeProcessRunner::new()));
    assert!(
        !cli.repo_catalog_capabilities().repo_catalog,
        "the CLI has no repo-scale package listing (environment verification 3bd4f12 s1): honestly declared-none"
    );
    let error = cli.repo_catalog(None, &[]).unwrap_err();
    assert_eq!(
        error.code, "vua.vpm.capability_missing",
        "the trait default absence arm answers capability_missing"
    );
}

// --- 027 F3 read face: installed-set listing at the v0.2 word face ---

/// Writes a synthetic repository-cache file with the given package versions
/// (the flat LocalCachedRepository JSON shape vrc-get itself persists; each
/// entry: id -> { versions: version -> manifest }). All data synthetic.
fn f3_repo_cache(cache_path: &std::path::Path, packages: serde_json::Value) {
    fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
    let cache = serde_json::json!({
        "repo": {
            "name": "Synthetic F3 Repo",
            "id": "com.vua.test.repo.f3",
            "url": "https://example.invalid/vua/f3-repo.json",
            "packages": packages
        }
    });
    fs::write(cache_path, cache.to_string()).unwrap();
}

/// Builds a synthetic VCC-shaped project with physically installed packages:
/// each row = the folder manifest (`Packages/<id>/package.json` — the
/// version authority the library reads) + the matching locked entry in
/// `Packages/vpm-manifest.json` (the MAP shape vrc-get 0.0.16 persists).
/// All data synthetic.
fn f3_installed_project(
    root: &std::path::Path,
    editor_version: &str,
    packages: &[(&str, &str, Option<&str>)],
) -> ProjectRef {
    fs::create_dir_all(root.join("Packages")).unwrap();
    fs::create_dir_all(root.join("ProjectSettings")).unwrap();
    fs::write(
        root.join("Packages/manifest.json"),
        r#"{"dependencies":{}}"#,
    )
    .unwrap();
    let mut locked = serde_json::Map::new();
    for (id, version, _) in packages {
        locked.insert(
            (*id).to_owned(),
            serde_json::json!({ "version": version, "dependencies": {} }),
        );
    }
    fs::write(
        root.join("Packages/vpm-manifest.json"),
        serde_json::json!({ "dependencies": {}, "locked": locked }).to_string(),
    )
    .unwrap();
    for (id, version, unity) in packages {
        let dir = root.join("Packages").join(id);
        fs::create_dir_all(&dir).unwrap();
        let mut manifest = serde_json::json!({
            "name": id,
            "version": version,
            "vpmDependencies": {}
        });
        if let Some(unity) = unity {
            manifest["unity"] = serde_json::json!(unity);
        }
        fs::write(dir.join("package.json"), manifest.to_string()).unwrap();
    }
    fs::write(
        root.join("ProjectSettings/ProjectVersion.txt"),
        format!("m_EditorVersion: {editor_version}\n"),
    )
    .unwrap();
    ProjectRef {
        id: "vpm-f3-spike".to_owned(),
        root: root.to_owned(),
    }
}

#[test]
fn f3_query_v02_negotiation_declares_exactly_the_implemented_face() {
    // ORC-DEV-004 pairing, both directions: the library backend overrides
    // query_v02 exactly now that list_packages_v02 is implemented (this is
    // what makes the wire route answer the v0.2 family const); the CLI
    // backend does neither — its served face stays the frozen v0.1 family
    // and the port default absence arm answers capability_missing verbatim.
    let library =
        VrcGetLibBackend::with_environment_root(unique_dir("f3-neg"), true).unwrap();
    assert!(
        library.query_v02(),
        "the declaration is the implementation's honest face on the negotiation"
    );
    let cli = backend_with(Arc::new(FakeProcessRunner::new()));
    assert!(
        !cli.query_v02(),
        "no implementation, no reservation: the CLI keeps serving the frozen v0.1 family"
    );
    let project = minimal_vpm_project(&unique_dir("f3-neg-proj"));
    let error = cli.list_packages_v02(&project).unwrap_err();
    assert_eq!(error.code, "vua.vpm.capability_missing");
    assert_eq!(error.message_key, "errors.vpm.capabilityMissing");
}

#[test]
fn f3_list_packages_v02_projects_v01_facts_with_the_judgment_pair() {
    let base = unique_dir("f3-list");
    let environment_root = base.join("isolated-vpm-environment");
    // The F2 synthetic cache reused verbatim: 0.9.0 yanked, 1.0.0 (unity
    // 2022.3), 2.0.0 (minimum unity 2022.4) — against the 2022.3 project the
    // qualifying latest is exactly 1.0.0.
    f3_repo_cache(
        &environment_root.join("Repos").join("synthetic-repo.json"),
        serde_json::json!({
            "com.vua.test.catalog.synthetic": { "versions": {
                "0.9.0": { "name": "com.vua.test.catalog.synthetic", "version": "0.9.0", "unity": "2022.3", "vpmDependencies": {}, "vrc-get": { "yanked": true } },
                "1.0.0": { "name": "com.vua.test.catalog.synthetic", "version": "1.0.0", "unity": "2022.3", "vpmDependencies": {} },
                "2.0.0": { "name": "com.vua.test.catalog.synthetic", "version": "2.0.0", "unity": "2022.4", "vpmDependencies": {} }
            } }
        }),
    );
    fs::write(
        environment_root.join("settings.json"),
        serde_json::json!({ "userRepos": [{
            "localPath": environment_root.join("Repos").join("synthetic-repo.json").display().to_string(),
            "url": "https://example.invalid/vua/f3-repo.json"
        }] })
        .to_string(),
    )
    .unwrap();
    let project = f3_installed_project(
        &base.join("managed-project"),
        "2022.3.22f1",
        &[
            ("com.vua.test.catalog.synthetic", "1.0.0", Some("2022.3")),
            // An installed package with NEITHER a repository cache position
            // nor an environment local-package position: the judgment has no
            // candidate — the null PAIR, never a false fill.
            ("com.vua.test.orphan", "0.1.0", None),
        ],
    );
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();

    let listing = backend.list_packages_v02(&project).unwrap();
    assert!(
        listing.cache_sourced,
        "offline world = the judgment rode the cache-degradation path (informational, not a failure)"
    );
    assert_eq!(
        listing.packages.len(),
        2,
        "the installed-set fact root is the frozen v0.1 manifest+lock projection"
    );
    // Rows ascend by packageId — the v0.1 frozen row order, unchanged.
    assert_eq!(listing.packages[0].package_id, "com.vua.test.catalog.synthetic");
    assert_eq!(listing.packages[1].package_id, "com.vua.test.orphan");

    let synthetic = &listing.packages[0];
    assert_eq!(synthetic.version, "1.0.0", "the v0.1 version fact, verbatim");
    assert!(
        synthetic.dependencies.is_empty(),
        "the v0.1 dependencies fact, verbatim"
    );
    assert_eq!(
        synthetic.latest_version.as_deref(),
        Some("1.0.0"),
        "cross-repo qualifying latest: 0.9.0 yanked-excluded, 2.0.0 unity-2022.4-excluded under the 2022.3 project binding"
    );
    assert_eq!(
        synthetic.update_available,
        Some(false),
        "the PRECISE false: no strictly newer version matching the CURRENT filter exists (2.0.0 exists but does not qualify) — never a generalized no-update"
    );

    let orphan = &listing.packages[1];
    assert_eq!(
        orphan.latest_version, None,
        "no qualifying candidate: the judgment is NOT EXECUTED — null is never already-latest"
    );
    assert_eq!(
        orphan.update_available, None,
        "the null arm rides the latest arm in a PAIR — never filled with a default false (024 stance-2 line)"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn f3_list_packages_v02_prerelease_setting_drives_the_judgment() {
    let base = unique_dir("f3-pre");
    let environment_root = base.join("isolated-vpm-environment");
    f3_repo_cache(
        &environment_root.join("Repos").join("pre-repo.json"),
        serde_json::json!({
            "com.vua.test.pre.synthetic": { "versions": {
                "1.0.0": { "name": "com.vua.test.pre.synthetic", "version": "1.0.0", "unity": "2022.3", "vpmDependencies": {} },
                "2.0.0-beta.1": { "name": "com.vua.test.pre.synthetic", "version": "2.0.0-beta.1", "unity": "2022.3", "vpmDependencies": {} }
            } }
        }),
    );
    let cache_path = environment_root
        .join("Repos")
        .join("pre-repo.json")
        .display()
        .to_string();
    // Zero wire switch: the prerelease inclusion reads the SAME settings.json
    // (the exact camelCase key the library's vpm_settings serde reads:
    // `showPrereleasePackages`).
    let settings_with = |prerelease: serde_json::Value| {
        fs::write(
            environment_root.join("settings.json"),
            serde_json::json!({
                "userRepos": [{ "localPath": cache_path, "url": "https://example.invalid/vua/f3-pre.json" }],
                "showPrereleasePackages": prerelease
            })
            .to_string(),
        )
        .unwrap();
    };
    let switch_off_stable = f3_installed_project(
        &base.join("project-stable"),
        "2022.3.22f1",
        &[("com.vua.test.pre.synthetic", "1.0.0", Some("2022.3"))],
    );
    let switch_off_prerelease = f3_installed_project(
        &base.join("project-prerelease"),
        "2022.3.22f1",
        &[("com.vua.test.pre.synthetic", "2.0.0-beta.1", Some("2022.3"))],
    );
    let backend_of = |prerelease: serde_json::Value| {
        settings_with(prerelease);
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap()
    };

    // Switch OFF (the default), stable installed: the prerelease candidate is
    // setting-excluded, the qualifying latest stays 1.0.0.
    let listing = backend_of(serde_json::json!(false))
        .list_packages_v02(&switch_off_stable)
        .unwrap();
    assert_eq!(listing.packages[0].latest_version.as_deref(), Some("1.0.0"));
    assert_eq!(listing.packages[0].update_available, Some(false));

    // Switch ON: the same collection world now qualifies 2.0.0-beta.1 as the
    // cross-repo max — the judgment follows the setting, never a wire key.
    let listing = backend_of(serde_json::json!(true))
        .list_packages_v02(&switch_off_stable)
        .unwrap();
    assert_eq!(
        listing.packages[0].latest_version.as_deref(),
        Some("2.0.0-beta.1")
    );
    assert_eq!(listing.packages[0].update_available, Some(true));

    // Boundary 2 (the frozen false semantics): the INSTALLED version is
    // itself a prerelease with the switch off — the qualifying latest is
    // taken from the stable set; `false` reads exactly "no strictly newer
    // version matching the current filter exists" (the installed row is
    // itself newer than the stable latest), never "this package stopped
    // updating" and never a null.
    let listing = backend_of(serde_json::json!(false))
        .list_packages_v02(&switch_off_prerelease)
        .unwrap();
    assert_eq!(listing.packages[0].version, "2.0.0-beta.1");
    assert_eq!(listing.packages[0].latest_version.as_deref(), Some("1.0.0"));
    assert_eq!(listing.packages[0].update_available, Some(false));
    fs::remove_dir_all(&base).ok();
}

#[test]
fn f3_list_packages_v02_cross_repo_max_merges_the_collection() {
    let base = unique_dir("f3-cross");
    let environment_root = base.join("isolated-vpm-environment");
    // The same package id carried by TWO subscribed repositories at different
    // qualifying latests — this face answers the CROSS-REPO max (the
    // find_package_by_name semantics), deliberately NOT the F2 per-repo view
    // (which would answer 1.4.0 in repo A and 1.5.0 in repo B separately).
    // Two views, each face declaring its own, never mixed.
    f3_repo_cache(
        &environment_root.join("Repos").join("cross-a.json"),
        serde_json::json!({
            "com.vua.test.cross.common": { "versions": {
                "1.4.0": { "name": "com.vua.test.cross.common", "version": "1.4.0", "unity": "2022.3", "vpmDependencies": {} }
            } }
        }),
    );
    f3_repo_cache(
        &environment_root.join("Repos").join("cross-b.json"),
        serde_json::json!({
            "com.vua.test.cross.common": { "versions": {
                "1.5.0": { "name": "com.vua.test.cross.common", "version": "1.5.0", "unity": "2022.3", "vpmDependencies": {} }
            } }
        }),
    );
    fs::write(
        environment_root.join("settings.json"),
        serde_json::json!({ "userRepos": [
            { "localPath": environment_root.join("Repos").join("cross-a.json").display().to_string(), "url": "https://example.invalid/vua/cross-a.json" },
            { "localPath": environment_root.join("Repos").join("cross-b.json").display().to_string(), "url": "https://example.invalid/vua/cross-b.json" }
        ] })
        .to_string(),
    )
    .unwrap();
    let project = f3_installed_project(
        &base.join("managed-project"),
        "2022.3.22f1",
        &[("com.vua.test.cross.common", "1.4.0", Some("2022.3"))],
    );
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();

    let listing = backend.list_packages_v02(&project).unwrap();
    assert_eq!(
        listing.packages[0].latest_version.as_deref(),
        Some("1.5.0"),
        "the judgment merges the WHOLE repository set and takes the highest qualifying version"
    );
    assert_eq!(
        listing.packages[0].update_available,
        Some(true),
        "a strictly newer qualifying version exists across the collection"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn f3_list_packages_v02_local_collection_chain_rides_the_same_judgment() {
    let base = unique_dir("f3-local");
    let environment_root = base.join("isolated-vpm-environment");
    // No repository caches at all — but the environment's local-package set
    // (the collection's user_packages chain in find_package_by_name) carries
    // a newer version of the installed package. The verbatim library
    // semantics keep the local chain inside the SAME one-load judgment.
    fs::create_dir_all(&environment_root).unwrap();
    fs::write(
        environment_root.join("settings.json"),
        serde_json::json!({ "userRepos": [] }).to_string(),
    )
    .unwrap();
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();
    let local_package_dir = base.join("local-pkg-src");
    fs::create_dir_all(&local_package_dir).unwrap();
    fs::write(
        local_package_dir.join("package.json"),
        serde_json::json!({
            "name": "com.vua.test.local.pkg",
            "version": "1.1.0",
            "vpmDependencies": {}
        })
        .to_string(),
    )
    .unwrap();
    backend.register_local_package(&local_package_dir).unwrap();

    let project = f3_installed_project(
        &base.join("managed-project"),
        "2022.3.22f1",
        &[("com.vua.test.local.pkg", "1.0.0", None)],
    );
    let listing = backend.list_packages_v02(&project).unwrap();
    assert_eq!(
        listing.packages[0].latest_version.as_deref(),
        Some("1.1.0"),
        "the collection's local-package chain is a candidate source of the SAME judgment"
    );
    assert_eq!(listing.packages[0].update_available, Some(true));
    fs::remove_dir_all(&base).ok();
}

#[test]
fn f3_list_packages_v02_honest_empty_set_and_the_frozen_error_face() {
    let base = unique_dir("f3-empty");
    let environment_root = base.join("isolated-vpm-environment");
    fs::create_dir_all(&environment_root).unwrap();
    fs::write(
        environment_root.join("settings.json"),
        serde_json::json!({ "userRepos": [] }).to_string(),
    )
    .unwrap();
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();

    // An empty lock is an honest empty array at the v0.2 face too.
    let empty_project = minimal_vpm_project(&base.join("bare-project"));
    let listing = backend.list_packages_v02(&empty_project).unwrap();
    assert!(listing.packages.is_empty());
    assert!(listing.cache_sourced);

    // The error face is the frozen v0.1 face, zero new codes: a root without
    // ProjectVersion.txt fails the project load with the REUSED
    // project_load_failed code (the v0.1 list_packages same-fact arm).
    let error = backend
        .list_packages_v02(&ProjectRef {
            id: "broken".to_owned(),
            root: base.join("not-a-project"),
        })
        .unwrap_err();
    assert_eq!(error.code, "vua.vpm.project_load_failed");
    assert_eq!(error.message_key, "errors.vpm.projectLoadFailed");
    assert_eq!(
        error.category,
        vua_orchestrator::ErrorCategory::ExternalFailure
    );
    fs::remove_dir_all(&base).ok();
}


// --- F5 (packages-templates v0.1): the library enumeration face ---

/// Creates an empty synthetic template directory under one of the two
/// pinned roots of the environment root. All data synthetic, temp roots
/// only (never the user's real VCC/ALCOM home).
fn f5_template_dir(environment_root: &std::path::Path, root: &str, name: &str) {
    fs::create_dir_all(environment_root.join(root).join(name)).unwrap();
}

#[test]
fn f5_template_capabilities_declares_exactly_the_implemented_face() {
    // ORC-DEV-004 pairing, both directions: the library backend overrides
    // template_capabilities exactly now that list_templates is implemented
    // (this is what flips the served wire row packages.templatesOps to
    // available); the CLI backend does neither — its default declared-none
    // keeps the row honestly unavailable and the port absence arm answers
    // capability_missing verbatim, never reaching a backend method.
    let library =
        VrcGetLibBackend::with_environment_root(unique_dir("f5-neg"), true).unwrap();
    assert!(
        library.template_capabilities().list_templates,
        "the declaration is the implementation's honest face on the capability accessor"
    );
    let cli = backend_with(Arc::new(FakeProcessRunner::new()));
    assert!(
        !cli.template_capabilities().list_templates,
        "no implementation, no reservation: the CLI backend stays declared-none"
    );
    let error = cli.list_templates().unwrap_err();
    assert_eq!(error.code, "vua.vpm.capability_missing");
    assert_eq!(error.message_key, "errors.vpm.capabilityMissing");
    assert_eq!(error.category, vua_orchestrator::ErrorCategory::Unavailable);
}

#[test]
fn f5_list_templates_scans_both_roots_dedup_resolver_order() {
    // The frozen root-facts section, item by item: both pinned roots of the
    // library-path default resolution leg, VRCTemplates scanned in full
    // FIRST, Templates filling only the missing set (World under both roots
    // enumerates ONCE, resolved to VRCTemplates — enumeration never
    // diverges from what create would copy); directory entries only (the
    // plain file is not a template); rows id-ascending with name the frozen
    // same-value display projection of id.
    let base = unique_dir("f5-scan");
    let environment_root = base.join("isolated-vpm-environment");
    f5_template_dir(&environment_root, "VRCTemplates", "World");
    f5_template_dir(&environment_root, "VRCTemplates", "Base");
    fs::write(
        environment_root.join("VRCTemplates").join("readme.txt"),
        "synthetic: a plain file is NOT a template",
    )
    .unwrap();
    f5_template_dir(&environment_root, "Templates", "Avatar");
    f5_template_dir(&environment_root, "Templates", "World");

    let backend =
        VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();
    let templates = backend.list_templates().unwrap();
    let ids: Vec<&str> = templates.iter().map(|entry| entry.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["Avatar", "Base", "World"],
        "id-ascending; the both-roots name enumerates once at the resolver root; the file is excluded"
    );
    for entry in &templates {
        assert_eq!(
            entry.name, entry.id,
            "name is the frozen same-value display projection of id"
        );
    }

    fs::remove_dir_all(&base).ok();
}

#[test]
fn f5_list_templates_honest_empty_when_roots_missing_or_bare() {
    // A missing pair of roots — and a bare root holding zero directories —
    // is the honest zero-templates answer: a FACT, never an error (the R4
    // precedent; zero new error codes), so the wired route answers an empty
    // templates array successfully.
    let base = unique_dir("f5-empty");
    let environment_root = base.join("isolated-vpm-environment");
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();
    assert!(
        backend.list_templates().unwrap().is_empty(),
        "missing roots contribute nothing and stay a success"
    );

    fs::create_dir_all(environment_root.join("Templates")).unwrap();
    fs::write(
        environment_root.join("Templates").join("notes.txt"),
        "synthetic: still not a template",
    )
    .unwrap();
    assert!(
        backend.list_templates().unwrap().is_empty(),
        "only files under the bare root — zero directories is still the honest empty answer"
    );

    fs::remove_dir_all(&base).ok();
}

// --- 027 F4: repository lifecycle implementation verification ---

/// Preseeds the VUA-owned enable/disable state file (the storage ruling's
/// `<environment_root>/.vua/vpm-repo-state.json`) with the given disabled
/// repoIds. Tests that need a FRESH world simply never call this — the
/// absent file is the all-enabled honest empty state.
fn f4_write_state(environment_root: &std::path::Path, disabled: &[&str]) {
    let state_path = environment_root.join(".vua").join("vpm-repo-state.json");
    fs::create_dir_all(state_path.parent().unwrap()).unwrap();
    let document = serde_json::json!({
        "schemaVersion": 1,
        "disabledRepoIds": disabled,
    });
    fs::write(state_path, document.to_string()).unwrap();
}

/// Reads the raw VUA-owned state document for exact-content pins.
fn f4_read_state(environment_root: &std::path::Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(f4_state_path(environment_root)).unwrap()).unwrap()
}

/// The state file path predicate (absence pins).
fn f4_state_path(environment_root: &std::path::Path) -> std::path::PathBuf {
    environment_root.join(".vua").join("vpm-repo-state.json")
}

/// One synthetic HTTP origin serving a repository manifest with a fixed
/// ETag: a fetch without that etag answers 200 + ETag + body; a conditional
/// fetch carrying that exact etag answers 304 (the etag-conditional
/// two-arm law, exercised over loopback with synthetic data only).
fn f4_spawn_repo_server(body: String, etag: &'static str) -> std::net::SocketAddr {
    use std::io::{Read as _, Write as _};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                let read = match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                buffer.extend_from_slice(&chunk[..read]);
                if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let head = String::from_utf8_lossy(&buffer).to_lowercase();
            let response = if head.contains(&format!("if-none-match: {etag}")) {
                "HTTP/1.1 304 Not Modified\r\nConnection: close\r\n\r\n".to_owned()
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nETag: {etag}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
            };
            let _ = stream.write_all(response.as_bytes());
        }
    });
    addr
}

/// The capability declaration IS the served-row flip switch (the A3/A4/F2/F5
/// law): the library backend implements all three lifecycle methods
/// in-process, so the override declares the THREE INDEPENDENT bits (the A4
/// three-bit law — gate per method, never per face); `VccCliBackend`
/// implements none and stays declared-none with the trait-default absence
/// arms (ORC-DEV-004), including the repos_v02 negotiation default.
#[test]
fn f4_repo_lifecycle_capabilities_declare_exactly_the_three_bits() {
    let base = unique_dir("f4-caps");
    let backend =
        VrcGetLibBackend::with_environment_root(base.join("isolated-vpm-environment"), true)
            .unwrap();

    let caps = backend.repo_lifecycle_capabilities();
    assert!(
        caps.enable_repo && caps.disable_repo && caps.refresh_repo,
        "the library implements all three lifecycle methods in-process, so \
         the override declares all three INDEPENDENT bits"
    );
    assert!(
        backend.repos_v02(),
        "the v0.2 negotiation bit rides the implemented list_repos_v02"
    );

    // The CLI backend: declared-none everywhere, absence arms answer the
    // reused capability_missing (same code as the wire gate — two-layer
    // honesty), and the frozen v0.1 repos family keeps being served.
    let cli = backend_with(Arc::new(FakeProcessRunner::new()));
    assert_eq!(
        cli.repo_lifecycle_capabilities(),
        vua_orchestrator::RepoLifecycleCapabilities::NONE,
        "no implementation, no declaration"
    );
    assert!(!cli.repos_v02());
    assert_eq!(
        cli.enable_repo("com.example.absent").unwrap_err().code,
        "vua.vpm.capability_missing"
    );
    assert_eq!(
        cli.disable_repo("com.example.absent").unwrap_err().code,
        "vua.vpm.capability_missing"
    );
    assert_eq!(
        cli.refresh_repo("com.example.absent").unwrap_err().code,
        "vua.vpm.capability_missing"
    );
    assert_eq!(
        cli.list_repos_v02().unwrap_err().code,
        "vua.vpm.capability_missing"
    );
    fs::remove_dir_all(&base).ok();
}

/// The enable/disable round trip persists ONLY in the VUA-owned state file:
/// the toggle never writes settings.json (byte-pinned), never touches
/// userRepos[i], and the v0.2 enabled bit reads back truthfully at every
/// step. Absent file = all enabled (the honest empty state); the disabled
/// row STAYS subscribed and listed (disabling hides nothing from the
/// configuration view).
#[test]
fn f4_disable_enable_roundtrip_persists_in_the_vua_owned_state_file() {
    let base = unique_dir("f4-toggle");
    let environment_root = base.join("isolated-vpm-environment");
    synthetic_settings(
        &environment_root,
        serde_json::json!([{
            "localPath": environment_root.join("Repos").join("synthetic-repo.json").display().to_string(),
            "name": "Synthetic Repo",
            "id": "com.vua.test.repo.synthetic",
            "url": "https://example.invalid/vua/synthetic-repo.json"
        }]),
    );
    let settings_before = fs::read(environment_root.join("settings.json")).unwrap();
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    // Absent state file: the honest all-enabled empty state.
    let rows = backend.list_repos_v02().unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].enabled, "no state file = all enabled");

    // Disable: the state lands in .vua/vpm-repo-state.json and NOWHERE else.
    backend.disable_repo("com.vua.test.repo.synthetic").unwrap();
    assert_eq!(
        fs::read(environment_root.join("settings.json")).unwrap(),
        settings_before,
        "the toggle face NEVER writes the shared settings.json (byte pin)"
    );
    let state = f4_read_state(&environment_root);
    assert_eq!(
        state["disabledRepoIds"],
        serde_json::json!(["com.vua.test.repo.synthetic"]),
        "exactly the toggled repoId, keyed by id"
    );
    let rows = backend.list_repos_v02().unwrap();
    assert_eq!(rows.len(), 1, "the disabled row STAYS listed");
    assert!(!rows[0].enabled, "the v0.2 bit reads back false");

    // settings.json carries no VUA key anywhere (the storage ruling's point).
    let settings_json: serde_json::Value =
        serde_json::from_slice(&fs::read(environment_root.join("settings.json")).unwrap())
            .unwrap();
    assert!(
        settings_json.get("vua").is_none() && settings_json.get("disabledRepoIds").is_none(),
        "settings.json carries shared facts only"
    );

    // Re-enable: the set empties and the bit reads back true.
    backend.enable_repo("com.vua.test.repo.synthetic").unwrap();
    let state = f4_read_state(&environment_root);
    assert_eq!(
        state["disabledRepoIds"],
        serde_json::json!([]),
        "re-enabling removes the entry instead of carrying a residue"
    );
    assert!(backend.list_repos_v02().unwrap()[0].enabled);

    // Repeat toggles are honest no-op rewrites (no invented second entries).
    backend.disable_repo("com.vua.test.repo.synthetic").unwrap();
    backend.disable_repo("com.vua.test.repo.synthetic").unwrap();
    let state = f4_read_state(&environment_root);
    assert_eq!(state["disabledRepoIds"].as_array().unwrap().len(), 1);
    fs::remove_dir_all(&base).ok();
}

/// An unknown repoId answers the REUSED `vua.vpm.repo_not_found` on all
/// three lifecycle methods — never a silent success — and a refused toggle
/// writes nothing at all (no state file, no .vua directory).
#[test]
fn f4_unknown_repo_id_answers_reused_repo_not_found_and_writes_nothing() {
    let base = unique_dir("f4-not-found");
    let environment_root = base.join("isolated-vpm-environment");
    synthetic_settings(&environment_root, serde_json::json!([]));
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    let enable_error = backend.enable_repo("com.example.absent").unwrap_err();
    assert_eq!(enable_error.code, "vua.vpm.repo_not_found");
    assert_eq!(
        enable_error.category,
        vua_orchestrator::ErrorCategory::Validation
    );
    assert_eq!(
        backend.disable_repo("com.example.absent").unwrap_err().code,
        "vua.vpm.repo_not_found"
    );
    assert_eq!(
        backend.refresh_repo("com.example.absent").unwrap_err().code,
        "vua.vpm.repo_not_found"
    );
    assert!(
        !f4_state_path(&environment_root).exists() && !environment_root.join(".vua").exists(),
        "a refused toggle writes nothing"
    );
    fs::remove_dir_all(&base).ok();
}

/// The state write is an ATOMIC replace (batch 160 self reverse-review
/// half-write face, the state_file.rs ORC-STO-003 house law): the toggle
/// lands via a synced `.tmp` sibling renamed into place, so a successful
/// write leaves NO temp residue and a stale `.tmp` from a dead prior run
/// neither breaks the toggle nor survives it. The rename-replace path is
/// also what keeps the word face "absent = all enabled" from ever firing
/// spuriously — the state file is never absent and never half-written
/// between two toggles (power-loss aside, which a unit test cannot arrange;
/// what IS pinned here is the writer's observable contract).
#[test]
fn f4_state_write_leaves_no_temp_residue_and_replaces_in_place() {
    let base = unique_dir("f4-atomic-write");
    let environment_root = base.join("isolated-vpm-environment");
    synthetic_settings(
        &environment_root,
        serde_json::json!([
            {
                "localPath": environment_root.join("Repos").join("a.json").display().to_string(),
                "name": "Repo A",
                "id": "com.vua.test.repo.a",
                "url": "https://example.invalid/vua/a.json"
            }
        ]),
    );
    let vua_dir = environment_root.join(".vua");
    let temp_sibling = vua_dir.join("vpm-repo-state.tmp");
    fs::create_dir_all(&vua_dir).unwrap();
    fs::write(&temp_sibling, b"dead-prior-run leftover").unwrap();
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    backend.disable_repo("com.vua.test.repo.a").unwrap();
    let state = f4_read_state(&environment_root);
    assert_eq!(
        state["disabledRepoIds"],
        serde_json::json!(["com.vua.test.repo.a"]),
        "the replace-over-stale-sibling write lands exactly the toggled set"
    );
    assert!(
        !temp_sibling.exists(),
        "the atomic write consumes its .tmp sibling — no residue"
    );

    // The second toggle writes over an EXISTING state file (the
    // rename-replace path): the content stays exact, still no residue, and
    // the .vua directory carries exactly the one state document.
    backend.enable_repo("com.vua.test.repo.a").unwrap();
    let state = f4_read_state(&environment_root);
    assert_eq!(
        state["disabledRepoIds"],
        serde_json::json!([]),
        "re-enable empties the set in place"
    );
    let mut landed: Vec<String> = fs::read_dir(&vua_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    landed.sort();
    assert_eq!(
        landed,
        vec!["vpm-repo-state.json".to_owned()],
        "exactly the state file — no temp siblings, no stray entries"
    );
    fs::remove_dir_all(&base).ok();
}

/// The v0.2 row projection: the frozen v0.1 five keys project verbatim and
/// EXACTLY one REQUIRED fact joins them — `enabled`. The id-absent row is
/// ALWAYS enabled (id-absent rows sit outside the toggle faces' reach — the
/// id IS the row handle), while the disabled id-bearing row reads false.
#[test]
fn f4_id_absent_row_is_always_enabled_and_projects_the_v02_keys() {
    let base = unique_dir("f4-id-absent");
    let environment_root = base.join("isolated-vpm-environment");
    let manifest = base.join("local-dir-repo").join("repo.json");
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    fs::write(&manifest, r#"{"name":"local","packages":{}}"#).unwrap();
    synthetic_settings(
        &environment_root,
        serde_json::json!([
            {
                "localPath": environment_root.join("Repos").join("a.json").display().to_string(),
                "name": "Repo A",
                "id": "com.vua.test.repo.a",
                "url": "https://example.invalid/vua/a.json"
            },
            {
                "localPath": manifest.display().to_string(),
                "name": "Local Dir"
            }
        ]),
    );
    f4_write_state(&environment_root, &["com.vua.test.repo.a"]);
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    let rows = backend.list_repos_v02().unwrap();
    assert_eq!(rows.len(), 2, "row order = subscription order");
    assert!(!rows[0].enabled, "the disabled id-bearing row reads false");
    assert!(!rows[0].cached, "subscribed-but-never-refreshed");
    assert!(
        rows[1].enabled,
        "the id-absent row is ALWAYS enabled — outside the toggle faces' reach"
    );
    assert_eq!(rows[1].repo_id, None);
    assert_eq!(rows[1].url, None, "a local row carries no url (honest null)");
    assert!(rows[1].cached, "repo.json exists and parses — cached=true");

    // The row shape is the frozen closed set: the v0.1 five keys plus
    // enabled — exactly six keys, zero invented facts (the schema
    // additionalProperties:false mirror at the projection source).
    let row_json = serde_json::to_value(&rows[0]).unwrap();
    let mut keys: Vec<&str> = row_json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["cached", "enabled", "localPath", "name", "repoId", "url"]
    );
    fs::remove_dir_all(&base).ok();
}

/// The refresh two-arm law over a loopback origin with synthetic data: the
/// first refresh downloads (200 + ETag), writes the row's OWN cache file at
/// userRepos[i].localPath with the etag aboard, and answers
/// cacheUpdated=true; the conditional second refresh rides the stored etag,
/// the origin answers 304, and cacheUpdated=false — "already up to date" is
/// an outcome, never an error, and NOTHING was rewritten. Refresh never
/// touches the enable/disable state file (no .vua write at all) and never
/// writes settings.json.
#[test]
fn f4_refresh_writes_cache_then_304_reports_no_new_data() {
    let base = unique_dir("f4-refresh");
    let environment_root = base.join("isolated-vpm-environment");
    let cache_path = environment_root.join("Repos").join("refreshed.json");
    let addr = f4_spawn_repo_server(
        r#"{"name":"Refreshed Synthetic","id":"com.vua.test.repo.refresh","packages":{}}"#
            .to_owned(),
        "\"v1\"",
    );
    synthetic_settings(
        &environment_root,
        serde_json::json!([{
            "localPath": cache_path.display().to_string(),
            "name": "Refreshed Synthetic",
            "id": "com.vua.test.repo.refresh",
            "url": format!("http://{addr}/repo.json")
        }]),
    );
    let settings_before = fs::read(environment_root.join("settings.json")).unwrap();
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    // Arm one: no cache yet → full download → cache written → true.
    let outcome = backend.refresh_repo("com.vua.test.repo.refresh").unwrap();
    assert!(outcome.cache_updated, "a first refresh writes the cache");
    let raw: serde_json::Value =
        serde_json::from_slice(&fs::read(&cache_path).unwrap()).unwrap();
    assert_eq!(
        raw["repo"]["id"], "com.vua.test.repo.refresh",
        "the cache file carries the downloaded manifest (the library's own \
         LocalCachedRepository shape: the document nested under `repo`)"
    );
    assert_eq!(
        raw["vrc-get"]["etag"], "\"v1\"",
        "the etag rides the cache file itself (the library's own storage law)"
    );
    let bytes_after_first = fs::read(&cache_path).unwrap();

    // Arm two: the stored etag matches → 304 → false, zero writes.
    let outcome = backend.refresh_repo("com.vua.test.repo.refresh").unwrap();
    assert!(
        !outcome.cache_updated,
        "etag unchanged = the honest already-up-to-date outcome, never an error"
    );
    assert_eq!(
        fs::read(&cache_path).unwrap(),
        bytes_after_first,
        "the 304 arm rewrites nothing"
    );

    // Refresh is not a state writer: no .vua file, settings.json untouched.
    assert!(
        !f4_state_path(&environment_root).exists(),
        "refresh NEVER touches the VUA enable/disable state file"
    );
    assert_eq!(
        fs::read(environment_root.join("settings.json")).unwrap(),
        settings_before,
        "refresh NEVER writes the shared settings.json"
    );
    // The refreshed row reads back enabled (refresh carries no state change).
    assert!(backend.list_repos_v02().unwrap()[0].enabled);
    fs::remove_dir_all(&base).ok();
}

/// The refresh network segment failing answers the REUSED
/// `vua.vpm.repo_fetch_failed` (ExternalFailure) — never a silent success,
/// never an invented cache. Zero writes on the failed path.
#[test]
fn f4_refresh_fetch_failure_answers_reused_repo_fetch_failed() {
    let base = unique_dir("f4-refresh-fail");
    let environment_root = base.join("isolated-vpm-environment");
    let cache_path = environment_root.join("Repos").join("never.json");
    synthetic_settings(
        &environment_root,
        serde_json::json!([{
            "localPath": cache_path.display().to_string(),
            "name": "Unreachable",
            "id": "com.vua.test.repo.unreachable",
            "url": "http://127.0.0.1:1/repo.json"
        }]),
    );
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    let error = backend
        .refresh_repo("com.vua.test.repo.unreachable")
        .unwrap_err();
    assert_eq!(error.code, "vua.vpm.repo_fetch_failed");
    assert_eq!(
        error.category,
        vua_orchestrator::ErrorCategory::ExternalFailure
    );
    assert!(!cache_path.exists(), "a failed fetch writes no cache");
    assert!(!f4_state_path(&environment_root).exists());
    fs::remove_dir_all(&base).ok();
}

/// A url-less (local-directory) row has no remote to refresh — the library's
/// own update arm is a no-op for exactly this shape (repo_holder.rs: no
/// effective url → no fetch, no write). The honest projection of that arm
/// is cacheUpdated=false ("no new data"), never an error, never a write.
#[test]
fn f4_refresh_url_less_row_is_honest_no_new_data() {
    let base = unique_dir("f4-refresh-local");
    let environment_root = base.join("isolated-vpm-environment");
    let manifest = base.join("local-dir-repo").join("repo.json");
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    fs::write(
        &manifest,
        r#"{"name":"local","id":"com.vua.test.repo.local","packages":{}}"#,
    )
    .unwrap();
    synthetic_settings(
        &environment_root,
        serde_json::json!([{
            "localPath": manifest.display().to_string(),
            "name": "Local Dir",
            "id": "com.vua.test.repo.local"
        }]),
    );
    let manifest_before = fs::read(&manifest).unwrap();
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    let outcome = backend.refresh_repo("com.vua.test.repo.local").unwrap();
    assert!(
        !outcome.cache_updated,
        "a local row's manifest IS its cache — nothing remote to refresh"
    );
    assert_eq!(
        fs::read(&manifest).unwrap(),
        manifest_before,
        "the no-new-data arm on a url-less row rewrites nothing"
    );
    assert!(!f4_state_path(&environment_root).exists());
    fs::remove_dir_all(&base).ok();
}

/// The F4 frozen duty on the pre-existing add/remove faces: a newly added
/// subscription row is always enabled (adds reset stale state — by the
/// manifest id on the remote face, by the url backfill when the manifest
/// carries no id) and a removed row leaves no state residue. All resets ride
/// the VUA-owned state file; the settings.json the library writes never
/// carries a VUA key.
#[test]
fn f4_add_and_remove_faces_leave_no_state_residue() {
    let base = unique_dir("f4-residue");
    let environment_root = base.join("isolated-vpm-environment");
    let id_addr = f4_spawn_repo_server(
        r#"{"name":"Re-added","id":"com.vua.test.repo.readded","packages":{}}"#.to_owned(),
        "\"etag-id\"",
    );
    let url_id_addr = f4_spawn_repo_server(
        r#"{"name":"No Id In Manifest","packages":{}}"#.to_owned(),
        "\"etag-url\"",
    );
    synthetic_settings(
        &environment_root,
        serde_json::json!([{
            "localPath": environment_root.join("Repos").join("gone.json").display().to_string(),
            "name": "To Remove",
            "id": "com.vua.test.repo.gone",
            "url": "https://example.invalid/vua/gone.json"
        }]),
    );
    f4_write_state(
        &environment_root,
        &[
            "com.vua.test.repo.gone",
            "com.vua.test.repo.readded",
            "com.vua.test.repo.untouched",
            &format!("http://{url_id_addr}/repo.json"),
        ],
    );
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    // Remove leaves no residue: the removed row's id leaves the state set.
    backend.remove_repo("com.vua.test.repo.gone").unwrap();
    assert_eq!(
        f4_read_state(&environment_root)["disabledRepoIds"],
        serde_json::json!([
            "com.vua.test.repo.readded",
            "com.vua.test.repo.untouched",
            format!("http://{url_id_addr}/repo.json"),
        ]),
        "exactly the removed row's entry is pruned"
    );

    // Add resets stale state (remote face, manifest id): the re-added
    // subscription starts enabled even though a stale entry existed.
    backend
        .add_remote_repo(&format!("http://{id_addr}/repo.json"), "Re-added")
        .unwrap();
    assert_eq!(
        f4_read_state(&environment_root)["disabledRepoIds"],
        serde_json::json!([
            "com.vua.test.repo.untouched",
            format!("http://{url_id_addr}/repo.json"),
        ]),
        "the re-added manifest id's stale entry is reset"
    );

    // Add resets stale state (remote face, id-less manifest → the library's
    // own url backfill is the effective row id): the url-keyed stale entry
    // is reset too.
    backend
        .add_remote_repo(&format!("http://{url_id_addr}/repo.json"), "No Id")
        .unwrap();
    assert_eq!(
        f4_read_state(&environment_root)["disabledRepoIds"],
        serde_json::json!(["com.vua.test.repo.untouched"]),
        "the url-backfill id's stale entry is reset"
    );

    // Every subscription row reads back enabled; the shared settings.json
    // the library saved carries no VUA-owned key anywhere.
    for row in backend.list_repos_v02().unwrap() {
        assert!(row.enabled, "a freshly added subscription starts enabled");
    }
    let settings_json: serde_json::Value =
        serde_json::from_slice(&fs::read(environment_root.join("settings.json")).unwrap())
            .unwrap();
    assert!(settings_json.get("vua").is_none());
    assert!(settings_json.get("disabledRepoIds").is_none());
    fs::remove_dir_all(&base).ok();
}

/// A state-file write-back failure answers the REUSED
/// `vua.vpm.repo_write_failed` (ExternalFailure) — the frozen three-code
/// closed set transports the honest failure, zero new codes.
#[test]
fn f4_state_write_failure_answers_reused_repo_write_failed() {
    let base = unique_dir("f4-write-fail");
    let environment_root = base.join("isolated-vpm-environment");
    synthetic_settings(
        &environment_root,
        serde_json::json!([{
            "localPath": base.join("a.json").display().to_string(),
            "name": "Repo A",
            "id": "com.vua.test.repo.a",
            "url": "https://example.invalid/vua/a.json"
        }]),
    );
    // `.vua` exists as a FILE: creating the state directory beneath it fails.
    fs::write(environment_root.join(".vua"), "not a directory").unwrap();
    let backend =
        VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();

    let error = backend.disable_repo("com.vua.test.repo.a").unwrap_err();
    assert_eq!(error.code, "vua.vpm.repo_write_failed");
    assert_eq!(
        error.category,
        vua_orchestrator::ErrorCategory::ExternalFailure
    );
    fs::remove_dir_all(&base).ok();
}

// --- 027 F4 patch slice: the collection world consumes the disabled set ---
//
// 第 144 批补切片（第 143 批退回裁决的逐条兑现）：冻结词面「禁用＝该行
// 离开包集合世界」（packages-ops v0.6 协议本「启停语义（冻结词面事实）」
// 节）的效果面钉死——集合装载路径（collection_world：settings 内存克隆
// 过滤后入 PackageCollection::load/load_cache）让禁用行的包在三个消费面
// 集体缺席（repo-catalog 列表／latest 判定／A2 安装解析器），而 repos
// v0.2 订阅投影仍如实列出行（enabled=false）——两面对照钉死，绝不投影
// 虚假事实（诚实纪律 #1）。

/// The F4 collection-world builder: TWO subscription rows, each with an
/// in-place cache file — `com.vua.test.repo.off` (package
/// `com.vua.test.f4.off` with qualifying 1.0.0 and newer 2.0.0, both unity
/// 2022.3) and `com.vua.test.repo.on` (package `com.vua.test.f4.on` 1.0.0).
/// The project has both packages installed at 1.0.0. All data synthetic;
/// the backend runs offline (the load_cache leg).
fn f4_collection_world(label: &str) -> (VrcGetLibBackend, ProjectRef, PathBuf) {
    let base = unique_dir(label);
    let environment_root = base.join("isolated-vpm-environment");
    let off_cache = environment_root.join("Repos").join("off-repo.json");
    let on_cache = environment_root.join("Repos").join("on-repo.json");
    let off_packages = serde_json::json!({
        "com.vua.test.f4.off": { "versions": {
            "1.0.0": { "name": "com.vua.test.f4.off", "version": "1.0.0", "unity": "2022.3", "vpmDependencies": {} },
            "2.0.0": { "name": "com.vua.test.f4.off", "version": "2.0.0", "unity": "2022.3", "vpmDependencies": {} }
        } }
    });
    let on_packages = serde_json::json!({
        "com.vua.test.f4.on": { "versions": {
            "1.0.0": { "name": "com.vua.test.f4.on", "version": "1.0.0", "unity": "2022.3", "vpmDependencies": {} }
        } }
    });
    for (path, id, name, url, packages) in [
        (
            &off_cache,
            "com.vua.test.repo.off",
            "Will Disable",
            "https://example.invalid/vua/f4-off.json",
            &off_packages,
        ),
        (
            &on_cache,
            "com.vua.test.repo.on",
            "Stays Enabled",
            "https://example.invalid/vua/f4-on.json",
            &on_packages,
        ),
    ] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            serde_json::json!({
                "repo": { "name": name, "id": id, "url": url, "packages": packages }
            })
            .to_string(),
        )
        .unwrap();
    }
    fs::write(
        environment_root.join("settings.json"),
        serde_json::json!({ "userRepos": [
            {
                "localPath": off_cache.display().to_string(),
                "name": "Will Disable",
                "id": "com.vua.test.repo.off",
                "url": "https://example.invalid/vua/f4-off.json"
            },
            {
                "localPath": on_cache.display().to_string(),
                "name": "Stays Enabled",
                "id": "com.vua.test.repo.on",
                "url": "https://example.invalid/vua/f4-on.json"
            }
        ] })
        .to_string(),
    )
    .unwrap();
    let project = f3_installed_project(
        &base.join("managed-project"),
        "2022.3.22f1",
        &[
            ("com.vua.test.f4.off", "1.0.0", Some("2022.3")),
            ("com.vua.test.f4.on", "1.0.0", Some("2022.3")),
        ],
    );
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();
    (backend, project, base)
}

/// THE two-face contrast pin (the rejection's exact defect class, dead):
/// the same disabled repository answers `enabled = false` on the repos v0.2
/// subscription projection — row PRESENT, the configuration view hides
/// nothing — while its packages are absent from every collection-world
/// consumer: the repo-catalog listing (row present via the not-loaded layer
/// with the HONEST cache-hit fact, packages empty), the per-package catalog
/// (`no_matching_package` — the collection world truly has no such package),
/// the v0.2 latest judgment (the null pair — the judgment cannot see the
/// disabled row, and absence is never filled with "no update"), and the A2
/// install resolver (`no_matching_package`). The enabled control row keeps
/// enumerating, judging and resolving throughout. Offline world = the
/// load_cache leg of the collection load.
#[test]
fn f4_collection_world_disabled_row_stays_listed_while_its_packages_leave_every_consumer() {
    let (backend, project, base) = f4_collection_world("f4-world-contrast");
    let environment_root = base.join("isolated-vpm-environment");
    f4_write_state(&environment_root, &["com.vua.test.repo.off"]);

    // Face 1 — repo-catalog listing: the disabled row is NOT hidden. It
    // falls to the not-loaded layer with its OWN identity facts, the honest
    // cache-hit fact (true — leaving the collection world is not a cache
    // removal), and an EMPTY packages array. The enabled control row keeps
    // its package.
    let catalog = backend.repo_catalog(None, &[]).unwrap();
    assert!(catalog.cache_sourced, "offline world = the load_cache leg");
    let off = catalog
        .repos
        .iter()
        .find(|repo| repo.repo_id.as_deref() == Some("com.vua.test.repo.off"))
        .expect("the disabled row STAYS listed — hiding it would violate the word face's other half");
    assert!(off.cached, "cache-hit fact stays honest: filtered != never refreshed");
    assert!(
        off.packages.is_empty(),
        "the disabled row's packages left the enumeration"
    );
    let on = catalog
        .repos
        .iter()
        .find(|repo| repo.repo_id.as_deref() == Some("com.vua.test.repo.on"))
        .unwrap();
    assert!(on.cached);
    assert_eq!(on.packages.len(), 1, "the enabled row keeps enumerating");
    assert_eq!(on.packages[0].package_id, "com.vua.test.f4.on");

    // The scope lens reaches the disabled row by its subscription id (the
    // id is still subscribed — repo_not_found would be a false fact) and
    // answers the same honest row.
    let scoped = backend
        .repo_catalog(Some("com.vua.test.repo.off"), &[])
        .unwrap();
    assert_eq!(scoped.repos.len(), 1);
    assert!(scoped.repos[0].cached);
    assert!(scoped.repos[0].packages.is_empty());

    // Face 2 — the repos v0.2 subscription projection, THE OTHER SIDE of
    // the contrast: the same repo row present with enabled=false, the cache
    // fact unchanged. One disabled repository, two faces, both truthful.
    let rows = backend.list_repos_v02().unwrap();
    assert_eq!(rows.len(), 2, "disabling hides nothing from the configuration view");
    let off_row = rows
        .iter()
        .find(|row| row.repo_id.as_deref() == Some("com.vua.test.repo.off"))
        .unwrap();
    assert!(!off_row.enabled, "the v0.2 bit reads back the real state");
    assert!(
        off_row.cached,
        "the subscription face's cache fact is untouched by the filter"
    );
    assert!(
        rows.iter()
            .find(|row| row.repo_id.as_deref() == Some("com.vua.test.repo.on"))
            .unwrap()
            .enabled
    );

    // Face 3 — the per-package catalog: the disabled row's package answers
    // the REUSED no_matching_package (the collection world truthfully has
    // no such package), while the enabled control resolves.
    let off_catalog = backend
        .package_catalog(&project, "com.vua.test.f4.off")
        .unwrap_err();
    assert_eq!(off_catalog.code, "vua.vpm.no_matching_package");
    assert_eq!(
        off_catalog.category,
        vua_orchestrator::ErrorCategory::Dependency
    );
    let on_catalog = backend.package_catalog(&project, "com.vua.test.f4.on").unwrap();
    assert_eq!(on_catalog.versions.len(), 1);

    // Face 4 — the v0.2 latest judgment: the installed off package has a
    // qualifying 2.0.0 in its (disabled) repo, yet the judgment sees only
    // the filtered world — the null PAIR (judgment not executed), never a
    // false "no update" fill. The on package judges normally against its
    // enabled repo (latest = itself, no strict update).
    let listing = backend.list_packages_v02(&project).unwrap();
    let off_installed = listing
        .packages
        .iter()
        .find(|row| row.package_id == "com.vua.test.f4.off")
        .unwrap();
    assert_eq!(
        off_installed.latest_version, None,
        "the latest judgment cannot see the disabled row"
    );
    assert_eq!(
        off_installed.update_available, None,
        "no qualifying version in the filtered world = the null pair, never false"
    );
    let on_installed = listing
        .packages
        .iter()
        .find(|row| row.package_id == "com.vua.test.f4.on")
        .unwrap();
    assert_eq!(on_installed.latest_version.as_deref(), Some("1.0.0"));
    assert_eq!(on_installed.update_available, Some(false));

    // Face 5 — the A2 install resolver: the disabled row's package no
    // longer resolves (REUSED no_matching_package), the enabled one does.
    let off_request = PackageRequestV1 {
        package_id: "com.vua.test.f4.off".to_owned(),
        version: Some("2.0.0".to_owned()),
    };
    let preview_error = backend
        .preview_install(&project, std::slice::from_ref(&off_request))
        .unwrap_err();
    assert_eq!(preview_error.code, "vua.vpm.no_matching_package");
    let on_request = PackageRequestV1 {
        package_id: "com.vua.test.f4.on".to_owned(),
        version: None,
    };
    backend
        .preview_install(&project, std::slice::from_ref(&on_request))
        .unwrap();
    fs::remove_dir_all(&base).ok();
}

/// The round trip (disable → faces exclude → enable → faces restore →
/// disable again → faces exclude) on ONE backend instance: every read face
/// rebuilds its collection from a fresh settings load plus a fresh state
/// read (there is no cross-call collection cache in the implementation
/// face), so a toggle write face returning is IMMEDIATELY visible to the
/// next load — the cache-invalidation property pinned structurally: the
/// "changed the state, stale cache still serves" failure has no code path
/// to live in.
#[test]
fn f4_collection_world_reenable_restores_every_face_on_the_same_backend() {
    let (backend, project, base) = f4_collection_world("f4-world-roundtrip");
    let enabled_faces = |backend: &VrcGetLibBackend, project: &ProjectRef| {
        let catalog = backend.repo_catalog(None, &[]).unwrap();
        let off = catalog
            .repos
            .iter()
            .find(|repo| repo.repo_id.as_deref() == Some("com.vua.test.repo.off"))
            .unwrap();
        assert!(
            !off.packages.is_empty(),
            "enabled world: the off row is a loaded collection row with its package"
        );
        assert_eq!(off.packages[0].latest_version.as_deref(), Some("2.0.0"));
        assert!(backend.package_catalog(project, "com.vua.test.f4.off").is_ok());
        assert!(
            backend
                .list_repos_v02()
                .unwrap()
                .iter()
                .find(|row| row.repo_id.as_deref() == Some("com.vua.test.repo.off"))
                .unwrap()
                .enabled
        );
    };
    let disabled_faces = |backend: &VrcGetLibBackend, project: &ProjectRef| {
        let catalog = backend.repo_catalog(None, &[]).unwrap();
        let off = catalog
            .repos
            .iter()
            .find(|repo| repo.repo_id.as_deref() == Some("com.vua.test.repo.off"))
            .unwrap();
        assert!(off.cached, "the row stays listed, cache fact honest");
        assert!(off.packages.is_empty(), "disabled world: packages gone");
        assert_eq!(
            backend
                .package_catalog(project, "com.vua.test.f4.off")
                .unwrap_err()
                .code,
            "vua.vpm.no_matching_package"
        );
        assert!(
            !backend
                .list_repos_v02()
                .unwrap()
                .iter()
                .find(|row| row.repo_id.as_deref() == Some("com.vua.test.repo.off"))
                .unwrap()
                .enabled
        );
    };

    // Absent state file: the all-enabled honest empty state.
    enabled_faces(&backend, &project);

    // Disable — the very next read on the SAME instance serves the filtered
    // world. No re-instantiation, no cache warm-up: the load path re-reads
    // the state file per call.
    backend.disable_repo("com.vua.test.repo.off").unwrap();
    disabled_faces(&backend, &project);

    // Re-enable — every face restores (the row's cache file was never
    // touched by the filter: recovery needs no refresh).
    backend.enable_repo("com.vua.test.repo.off").unwrap();
    enabled_faces(&backend, &project);

    // Disable again — excluded again. Three toggles, six world states, one
    // instance: a stale in-process collection cache could not pass this.
    backend.disable_repo("com.vua.test.repo.off").unwrap();
    disabled_faces(&backend, &project);
    fs::remove_dir_all(&base).ok();
}

/// A corrupt VUA state file refuses the collection world with the REUSED
/// `backend_unavailable` (the shared load-leg constructor — the v0.2
/// projection face's same fact, same code, same wording): with the disable
/// state unreadable, no collection-world face may guess "all enabled" and
/// serve packages the user disabled. The v0.2 projection face answers the
/// same code on the same file (one fact, one code everywhere).
#[test]
fn f4_collection_world_corrupt_state_file_refuses_every_loader_never_guesses() {
    let (backend, project, base) = f4_collection_world("f4-world-corrupt");
    let environment_root = base.join("isolated-vpm-environment");
    f4_write_state(&environment_root, &["com.vua.test.repo.off"]);
    fs::write(f4_state_path(&environment_root), "not-json").unwrap();

    let catalog_error = backend.repo_catalog(None, &[]).unwrap_err();
    assert_eq!(catalog_error.code, "vua.vpm.backend_unavailable");
    assert_eq!(
        catalog_error.category,
        vua_orchestrator::ErrorCategory::Unavailable
    );
    assert_eq!(
        backend
            .package_catalog(&project, "com.vua.test.f4.off")
            .unwrap_err()
            .code,
        "vua.vpm.backend_unavailable"
    );
    let request = PackageRequestV1 {
        package_id: "com.vua.test.f4.off".to_owned(),
        version: None,
    };
    assert_eq!(
        backend
            .preview_install(&project, std::slice::from_ref(&request))
            .unwrap_err()
            .code,
        "vua.vpm.backend_unavailable"
    );
    assert_eq!(
        backend.list_packages_v02(&project).unwrap_err().code,
        "vua.vpm.backend_unavailable"
    );
    assert_eq!(
        backend.list_repos_v02().unwrap_err().code,
        "vua.vpm.backend_unavailable"
    );
    fs::remove_dir_all(&base).ok();
}

/// The filter rides the ONLINE load arm with the same force: with the two
/// predefined repositories ignored (the library's own experimental switch —
/// the network segment stays hermetic, loopback only, synthetic data), a
/// real `PackageCollection::load` against a loopback origin serves
/// `cache_sourced = false` and the disabled row's packages are equally
/// absent. The disabled row's cache file is untouched by the online load
/// (leaving the collection world is not a cache removal — the file the
/// re-enable recovery reads back is byte-identical).
#[test]
fn f4_collection_world_filter_rides_the_online_load_arm_too() {
    let base = unique_dir("f4-world-online");
    let environment_root = base.join("isolated-vpm-environment");
    fs::create_dir_all(environment_root.join("vrc-get")).unwrap();
    fs::write(
        environment_root.join("vrc-get/settings.json"),
        serde_json::json!({
            "ignoreOfficialRepository": true,
            "ignoreCuratedRepository": true
        })
        .to_string(),
    )
    .unwrap();
    let on_packages = serde_json::json!({
        "com.vua.test.f4.on": { "versions": {
            "1.0.0": { "name": "com.vua.test.f4.on", "version": "1.0.0", "unity": "2022.3", "vpmDependencies": {} }
        } }
    });
    let on_body = serde_json::json!({
        "name": "Stays Enabled",
        "id": "com.vua.test.repo.on",
        "packages": on_packages
    })
    .to_string();
    let on_addr = f4_spawn_repo_server(on_body, "\"vua-f4-online-etag\"");
    let on_url = format!("http://{on_addr}/repo.json");
    let on_cache = environment_root.join("Repos").join("on-repo.json");
    fs::create_dir_all(on_cache.parent().unwrap()).unwrap();
    fs::write(
        &on_cache,
        serde_json::json!({
            "repo": { "name": "Stays Enabled", "id": "com.vua.test.repo.on", "url": on_url, "packages": on_packages }
        })
        .to_string(),
    )
    .unwrap();
    let off_cache = environment_root.join("Repos").join("off-repo.json");
    let off_bytes_before = {
        fs::create_dir_all(off_cache.parent().unwrap()).unwrap();
        let bytes = serde_json::json!({
            "repo": {
                "name": "Will Disable",
                "id": "com.vua.test.repo.off",
                "url": "https://example.invalid/vua/f4-off.json",
                "packages": {
                    "com.vua.test.f4.off": { "versions": {
                        "2.0.0": { "name": "com.vua.test.f4.off", "version": "2.0.0", "unity": "2022.3", "vpmDependencies": {} }
                    } }
                }
            }
        })
        .to_string();
        fs::write(&off_cache, &bytes).unwrap();
        bytes
    };
    fs::write(
        environment_root.join("settings.json"),
        serde_json::json!({ "userRepos": [
            {
                "localPath": off_cache.display().to_string(),
                "name": "Will Disable",
                "id": "com.vua.test.repo.off",
                "url": "https://example.invalid/vua/f4-off.json"
            },
            {
                "localPath": on_cache.display().to_string(),
                "name": "Stays Enabled",
                "id": "com.vua.test.repo.on",
                "url": on_url
            }
        ] })
        .to_string(),
    )
    .unwrap();
    f4_write_state(&environment_root, &["com.vua.test.repo.off"]);
    let project = f3_installed_project(
        &base.join("managed-project"),
        "2022.3.22f1",
        &[("com.vua.test.f4.on", "1.0.0", Some("2022.3"))],
    );
    // ONLINE backend: the load arm fetches the loopback origin for the one
    // ENABLED row; the disabled row is not in the load's world at all.
    let backend = VrcGetLibBackend::with_environment_root(environment_root, false).unwrap();

    let catalog = backend.repo_catalog(None, &[]).unwrap();
    assert!(
        !catalog.cache_sourced,
        "the ONLINE load arm served this result (predefined ignored, enabled row loopback 200) — the filter provably rides the online arm, not just load_cache"
    );
    let off = catalog
        .repos
        .iter()
        .find(|repo| repo.repo_id.as_deref() == Some("com.vua.test.repo.off"))
        .expect("the disabled row stays listed on the online arm too");
    assert!(off.cached);
    assert!(off.packages.is_empty());
    let on = catalog
        .repos
        .iter()
        .find(|repo| repo.repo_id.as_deref() == Some("com.vua.test.repo.on"))
        .unwrap();
    assert_eq!(on.packages.len(), 1);
    assert!(
        !backend
            .list_repos_v02()
            .unwrap()
            .iter()
            .find(|row| row.repo_id.as_deref() == Some("com.vua.test.repo.off"))
            .unwrap()
            .enabled
    );
    assert_eq!(
        fs::read_to_string(&off_cache).unwrap(),
        off_bytes_before,
        "the online load never touched the disabled row's cache file — re-enable recovery reads the same bytes"
    );
    // The A2 resolver rides the same online-loaded collection: the disabled
    // row's package does not resolve, the enabled one does.
    let off_request = PackageRequestV1 {
        package_id: "com.vua.test.f4.off".to_owned(),
        version: None,
    };
    assert_eq!(
        backend
            .preview_install(&project, std::slice::from_ref(&off_request))
            .unwrap_err()
            .code,
        "vua.vpm.no_matching_package"
    );
    let on_request = PackageRequestV1 {
        package_id: "com.vua.test.f4.on".to_owned(),
        version: None,
    };
    backend
        .preview_install(&project, std::slice::from_ref(&on_request))
        .unwrap();
    fs::remove_dir_all(&base).ok();
}

// --- batch 146 (user ruling 2026-09-21: SDK import before real-machine
// --- acceptance): VrcGetLibBackend::resolve_project, the three honest arms ---

/// Writes a vpm-manifest.json that DECLARES one dependency (the creation
/// template's shape: declared but NOT vendored).
fn declare_dependency(project: &std::path::Path, package_id: &str, version: &str) {
    let manifest = format!(
        r#"{{"dependencies":{{"{package_id}":{{"version":"{version}"}}}},"locked":{{}}}}"#
    );
    fs::write(project.join("Packages/vpm-manifest.json"), manifest).unwrap();
}

#[test]
fn b3_batch146_resolve_installs_declared_local_package_and_reports_it() {
    let base = unique_dir("batch146-resolve-installs");
    let environment_root = base.join("isolated-vpm-environment");
    let package_root = base.join("generated-package");
    fs::create_dir_all(package_root.join("Runtime")).unwrap();
    fs::write(package_root.join("Runtime/hello.txt"), "hello\n").unwrap();
    fs::write(
        package_root.join("package.json"),
        r#"{
  "name": "com.ph-r.vua.local.synthetic",
  "displayName": "Synthetic",
  "version": "0.0.1",
  "unity": "2022.3",
  "vpmDependencies": {}
}"#,
    )
    .unwrap();
    let project_root = base.join("validation-project");
    let _project = minimal_vpm_project(&project_root);
    let backend = VrcGetLibBackend::with_environment_root(environment_root.clone(), true).unwrap();
    backend.register_local_package(&package_root).unwrap();
    declare_dependency(&project_root, "com.ph-r.vua.local.synthetic", "0.0.1");

    // First resolve: the declared dependency is fetched-and-installed (from
    // the registered local set here — no network needed), the receipt names
    // it with its version and source, the body lands in Packages/ and the
    // locked section is written back.
    let receipt = backend.resolve_project(&project_root).unwrap();
    assert!(receipt.failed.is_empty());
    assert!(receipt.already_satisfied.is_empty());
    assert_eq!(receipt.resolved.len(), 1);
    assert_eq!(receipt.resolved[0].id, "com.ph-r.vua.local.synthetic");
    assert_eq!(receipt.resolved[0].version, "0.0.1");
    assert_eq!(receipt.resolved[0].source_repo, "local");
    assert!(
        project_root
            .join("Packages/com.ph-r.vua.local.synthetic/package.json")
            .is_file(),
        "the resolved package body must be installed into Packages/"
    );
    let manifest =
        fs::read_to_string(project_root.join("Packages/vpm-manifest.json")).unwrap();
    assert!(
        manifest.contains("\"locked\"") && manifest.contains("com.ph-r.vua.local.synthetic"),
        "the locked section must be written back: {manifest}"
    );

    // Second resolve over the unchanged project: the idempotency arm — the
    // dependency is already satisfied, nothing is resolved again.
    let second = backend.resolve_project(&project_root).unwrap();
    assert!(second.resolved.is_empty());
    assert!(second.failed.is_empty());
    assert_eq!(second.already_satisfied, vec!["com.ph-r.vua.local.synthetic"]);
    fs::remove_dir_all(&base).ok();
}

#[test]
fn b3_batch146_unresolvable_dependency_answers_the_honest_failed_receipt() {
    let base = unique_dir("batch146-resolve-missing");
    let project_root = base.join("empty-environment-project");
    let _project = minimal_vpm_project(&project_root);
    // No repositories, no local packages: the declared SDK dependency cannot
    // be satisfied from ANY enabled source.
    let backend =
        VrcGetLibBackend::with_environment_root(base.join("isolated-vpm-environment"), true)
            .unwrap();
    declare_dependency(&project_root, "com.vrchat.base", "3.10.1");

    let receipt = backend.resolve_project(&project_root).unwrap();
    assert!(receipt.resolved.is_empty());
    assert!(receipt.already_satisfied.is_empty());
    assert_eq!(receipt.failed.len(), 1);
    assert_eq!(receipt.failed[0].id, "com.vrchat.base");
    assert_eq!(
        receipt.failed[0].reason_code, "vua.vpm.no_matching_package",
        "the failed arm reuses the standing code — zero new codes"
    );
    assert!(
        !project_root.join("Packages/com.vrchat.base").exists(),
        "nothing may be installed when nothing resolves"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn b3_batch146_capability_absence_keeps_the_cli_backend_honestly_false() {
    let base = unique_dir("batch146-cli-absent");
    let project_root = base.join("cli-project");
    let _project = minimal_vpm_project(&project_root);
    let cli = backend_with(Arc::new(FakeProcessRunner::new()));

    // The capability bit stays declared-none...
    assert!(!cli.capabilities().resolve_project);
    // ...and the trait-default absence arm answers the capability_missing
    // family — the CLI never pretends to resolve.
    let error = cli.resolve_project(&project_root).unwrap_err();
    assert_eq!(error.code, "vua.vpm.capability_missing");
    fs::remove_dir_all(&base).ok();
}

// ======================================================================
// 环境实现核对切片（wt-6，批 146）：resolve_project 的环境侧证据钉。核心
// 接线批 70f7476 的三臂钉之上，本组按操作者指派钉「幂等零网络零写入逐字
// 节＋禁用集语义与 collection_world 互证＋离线如实上浮＋环回夹具仓全解
// 析落地」。全部走环回源＋合成数据（零真实网络），离线臂零出网。
// ======================================================================

/// Builds a minimal STORED-method (no compression) zip archive in bytes:
/// one entry per (name, content), correct CRC-32, central directory and
/// EOCD — just enough for the library's async_zip reader to extract the
/// synthetic package, with zero extra dependencies in the test crate.
fn f6_zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut table = [0u32; 256];
        for (index, slot) in table.iter_mut().enumerate() {
            let mut crc = index as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    0xEDB8_8320 ^ (crc >> 1)
                } else {
                    crc >> 1
                };
            }
            *slot = crc;
        }
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in data {
            crc = table[((crc ^ byte as u32) & 0xFF) as usize] ^ (crc >> 8);
        }
        crc ^ 0xFFFF_FFFF
    }
    fn put_u16(out: &mut Vec<u8>, value: u16) {
        out.extend_from_slice(&value.to_le_bytes());
    }
    fn put_u32(out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    let mut local = Vec::new();
    let mut central = Vec::new();
    let mut offset = 0u32;
    for (name, content) in entries {
        let crc = crc32(content);
        let name_bytes = name.as_bytes();
        let header_offset = offset;
        put_u32(&mut local, 0x0403_4b50);
        put_u16(&mut local, 20); // version needed
        put_u16(&mut local, 0); // flags
        put_u16(&mut local, 0); // method: stored
        put_u16(&mut local, 0); // mod time
        put_u16(&mut local, 0x0021); // mod date (1980-01-01)
        put_u32(&mut local, crc);
        put_u32(&mut local, content.len() as u32);
        put_u32(&mut local, content.len() as u32);
        put_u16(&mut local, name_bytes.len() as u16);
        put_u16(&mut local, 0); // extra length
        local.extend_from_slice(name_bytes);
        local.extend_from_slice(content);
        put_u32(&mut central, 0x0201_4b50);
        put_u16(&mut central, 20); // version made by
        put_u16(&mut central, 20); // version needed
        put_u16(&mut central, 0); // flags
        put_u16(&mut central, 0); // method
        put_u16(&mut central, 0); // mod time
        put_u16(&mut central, 0x0021); // mod date
        put_u32(&mut central, crc);
        put_u32(&mut central, content.len() as u32);
        put_u32(&mut central, content.len() as u32);
        put_u16(&mut central, name_bytes.len() as u16);
        put_u16(&mut central, 0); // extra
        put_u16(&mut central, 0); // comment
        put_u16(&mut central, 0); // disk start
        put_u16(&mut central, 0); // internal attrs
        put_u32(&mut central, 0); // external attrs
        put_u32(&mut central, header_offset);
        central.extend_from_slice(name_bytes);
        offset += (30 + name_bytes.len() + content.len()) as u32;
    }
    let central_offset = offset;
    let central_size = central.len() as u32;
    let mut out = local;
    out.extend_from_slice(&central);
    put_u32(&mut out, 0x0605_4b50);
    put_u16(&mut out, 0); // disk
    put_u16(&mut out, 0); // cd disk
    put_u16(&mut out, entries.len() as u16);
    put_u16(&mut out, entries.len() as u16);
    put_u32(&mut out, central_size);
    put_u32(&mut out, central_offset);
    put_u16(&mut out, 0); // comment length
    out
}

const F6_README_NAME: &str = "README.md";
const F6_README_BODY: &str = "synthetic resolve package\n";

/// One path-aware loopback origin serving BOTH the repository manifest
/// (`/repo.json`) and the package zip (`/pkg.zip`), synthetic data only.
/// The `build` closure receives the bound loopback address so the manifest
/// body can point at this very origin for the zip. Counts accepted
/// connections so the zero-network pins are PROVEN, not assumed.
fn f6_spawn_resolve_server(
    build: impl FnOnce(std::net::SocketAddr) -> (String, Vec<u8>),
) -> (std::net::SocketAddr, Arc<std::sync::atomic::AtomicUsize>) {
    use std::io::{Read as _, Write as _};
    use std::sync::atomic::{AtomicUsize, Ordering};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (repo_body, zip_bytes) = build(addr);
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_thread = Arc::clone(&hits);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            hits_thread.fetch_add(1, Ordering::SeqCst);
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                let read = match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                buffer.extend_from_slice(&chunk[..read]);
                if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let head = String::from_utf8_lossy(&buffer);
            let path = head
                .split_whitespace()
                .nth(1)
                .unwrap_or("")
                .split('?')
                .next()
                .unwrap_or("")
                .to_owned();
            let response: Vec<u8> = match path.as_str() {
                "/repo.json" => format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    repo_body.len(),
                    repo_body
                )
                .into_bytes(),
                "/pkg.zip" => {
                    let mut http = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        zip_bytes.len()
                    )
                    .into_bytes();
                    http.extend_from_slice(&zip_bytes);
                    http
                }
                _ => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_vec(),
            };
            let _ = stream.write_all(&response);
        }
    });
    (addr, hits)
}

/// A VCC-shaped project with DECLARED but UNRESOLVED dependencies: the
/// vpm-manifest dependencies section carries the rows, `locked` is empty
/// and nothing is installed under `Packages/` — the exact shape a VCC
/// template copy (the material chain's supply step) leaves behind.
fn f6_declared_project(root: &std::path::Path, editor: &str, deps: &[(&str, &str)]) -> ProjectRef {
    fs::create_dir_all(root.join("Packages")).unwrap();
    fs::create_dir_all(root.join("ProjectSettings")).unwrap();
    fs::write(root.join("Packages/manifest.json"), r#"{"dependencies":{}}"#).unwrap();
    let mut dependencies = serde_json::Map::new();
    for (id, range) in deps {
        dependencies.insert((*id).to_owned(), serde_json::json!({ "version": range }));
    }
    fs::write(
        root.join("Packages/vpm-manifest.json"),
        serde_json::json!({ "dependencies": dependencies, "locked": {} }).to_string(),
    )
    .unwrap();
    fs::write(
        root.join("ProjectSettings/ProjectVersion.txt"),
        format!("m_EditorVersion: {editor}\n"),
    )
    .unwrap();
    ProjectRef {
        id: "vpm-f6-resolve".to_owned(),
        root: root.to_owned(),
    }
}

/// Deterministic snapshot of every file byte in the project tree (sorted by
/// path) — the zero-write pins compare before/after snapshots, not memories.
fn f6_tree_bytes(root: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    fn walk(dir: &std::path::Path, prefix: &str, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let rel = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if path.is_dir() {
                walk(&path, &format!("{rel}\\"), out);
            } else {
                out.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, "", &mut out);
    out.sort_by(|left, right| left.0.cmp(&right.0));
    out
}

/// The resolve world: predefined official/curated repos neutralized via the
/// library's own experimental switches (zero outbound network), one loopback
/// subscription row (cache file ABSENT — the online leg fetches and writes
/// it, the add_remote_repo row shape), and the declared-only project.
fn f6_resolve_world(
    label: &str,
    dep_range: &str,
) -> (
    VrcGetLibBackend,
    ProjectRef,
    PathBuf,
    Arc<std::sync::atomic::AtomicUsize>,
) {
    let base = unique_dir(label);
    let environment_root = base.join("isolated-vpm-environment");
    fs::create_dir_all(environment_root.join("vrc-get")).unwrap();
    fs::write(
        environment_root.join("vrc-get/settings.json"),
        serde_json::json!({
            "ignoreOfficialRepository": true,
            "ignoreCuratedRepository": true
        })
        .to_string(),
    )
    .unwrap();

    let package_json = serde_json::json!({
        "name": "com.vua.test.resolve.dep",
        "version": "1.0.0",
        "vpmDependencies": {}
    })
    .to_string();
    let zip = f6_zip_bytes(&[
        ("package.json", package_json.as_bytes()),
        (F6_README_NAME, F6_README_BODY.as_bytes()),
    ]);

    let (addr, hits) = f6_spawn_resolve_server(|addr| {
        let body = serde_json::json!({
            "name": "Synthetic Resolve Repo",
            "id": "com.vua.test.repo.resolve",
            "packages": {
                "com.vua.test.resolve.dep": { "versions": {
                    "1.0.0": {
                        "name": "com.vua.test.resolve.dep",
                        "version": "1.0.0",
                        "vpmDependencies": {},
                        "url": format!("http://{addr}/pkg.zip")
                    }
                } }
            }
        })
        .to_string();
        (body, zip)
    });
    let repo_url = format!("http://{addr}/repo.json");
    let cache_path = environment_root.join("Repos").join("resolve-repo.json");
    fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
    fs::write(
        environment_root.join("settings.json"),
        serde_json::json!({ "userRepos": [{
            "localPath": cache_path.display().to_string(),
            "name": "Synthetic Resolve Repo",
            "id": "com.vua.test.repo.resolve",
            "url": repo_url
        }] })
        .to_string(),
    )
    .unwrap();
    let project = f6_declared_project(
        &base.join("managed-project"),
        "2022.3.22f1",
        &[("com.vua.test.resolve.dep", dep_range)],
    );
    let backend = VrcGetLibBackend::with_environment_root(environment_root, false).unwrap();
    (backend, project, base, hits)
}

/// 主钉：声明依赖全解析成功（环回夹具仓——清单与包 zip 全走 127.0.0.1，
/// 真实网络零触碰）。resolved 收据（id/version/source_repo＝启用仓库行
/// id）＋Packages 实地落地＋vpm-manifest locked 段回写；恰两次环回连接
/// （清单刷新＋包 zip 下载）。
#[test]
fn f6_resolve_declared_dependencies_land_locked_and_receipted() {
    let (backend, project, base, hits) = f6_resolve_world("f6-resolve-main", "1.0.0");
    let manifest_path = project.root.join("Packages/vpm-manifest.json");

    let receipt = backend.resolve_project(&project.root).unwrap();
    assert_eq!(
        receipt.resolved.len(),
        1,
        "exactly the declared dependency lands"
    );
    let entry = &receipt.resolved[0];
    assert_eq!(entry.id, "com.vua.test.resolve.dep");
    assert_eq!(entry.version, "1.0.0");
    assert_eq!(
        entry.source_repo, "com.vua.test.repo.resolve",
        "source_repo is the enabled repository's row id"
    );
    assert!(receipt.already_satisfied.is_empty());
    assert!(
        receipt.failed.is_empty(),
        "a full resolve answers an empty failed set"
    );

    // Packages 实地落地：解包内容存在且 package.json 与收据同源。
    let installed = fs::read_to_string(
        project
            .root
            .join("Packages/com.vua.test.resolve.dep/package.json"),
    )
    .unwrap();
    let installed: serde_json::Value = serde_json::from_str(&installed).unwrap();
    assert_eq!(installed["name"], "com.vua.test.resolve.dep");
    assert_eq!(installed["version"], "1.0.0");
    assert!(
        project
            .root
            .join("Packages/com.vua.test.resolve.dep")
            .join(F6_README_NAME)
            .is_file(),
        "the zip content itself landed"
    );

    // locked 段回写：依赖在锁定表、版本与收据一致。
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    assert_eq!(
        manifest["locked"]["com.vua.test.resolve.dep"]["version"], "1.0.0",
        "the resolve writes the locked section back"
    );

    // 环回实证：恰两次连接（清单刷新＋包 zip 下载），零真实网络。
    assert_eq!(
        hits.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "exactly the manifest fetch and the package zip download rode the loopback"
    );
    fs::remove_dir_all(&base).ok();
}

/// 幂等钉：locked 已满足＝already_satisfied，零网络零写入——第二次
/// resolve 在快路径短路（端口面文档「touches nothing」律：在线 load 臂
/// 会刷新仓库缓存，快路径绝不装载集合），连接计数一动不动、工程树逐字节
/// 不动。
#[test]
fn f6_resolve_idempotent_second_call_is_already_satisfied_zero_network_zero_writes() {
    let (backend, project, base, hits) = f6_resolve_world("f6-resolve-idem", "1.0.0");
    backend.resolve_project(&project.root).unwrap();
    let before = f6_tree_bytes(&project.root);
    let hits_after_first = hits.load(std::sync::atomic::Ordering::SeqCst);

    let receipt = backend.resolve_project(&project.root).unwrap();
    assert!(receipt.resolved.is_empty(), "nothing re-lands");
    assert_eq!(
        receipt.already_satisfied,
        vec!["com.vua.test.resolve.dep".to_owned()],
        "the satisfied declared dependency answers already_satisfied"
    );
    assert!(receipt.failed.is_empty());
    assert_eq!(
        hits.load(std::sync::atomic::Ordering::SeqCst),
        hits_after_first,
        "zero network: the fast path never loads the collection, never opens a connection"
    );
    assert_eq!(
        f6_tree_bytes(&project.root),
        before,
        "zero writes: the project tree is byte-identical"
    );
    fs::remove_dir_all(&base).ok();
}

/// 版本不满足→failed 如实：声明依赖的版本范围在启用仓库无满足项→诚实
/// 不完整收据（failed 逐依赖携复用码 no_matching_package，零新码），
/// resolved 恰空、zip 下载从不发生（恰一次清单连接）、工程树逐字节不动
/// ——绝不返回半落地成功收据。
#[test]
fn f6_resolve_version_unsatisfiable_answers_the_honest_failed_receipt() {
    let (backend, project, base, hits) = f6_resolve_world("f6-resolve-ver", "2.0.0");
    let before = f6_tree_bytes(&project.root);

    let receipt = backend.resolve_project(&project.root).unwrap();
    assert!(receipt.resolved.is_empty(), "nothing resolves, nothing lands");
    assert!(receipt.already_satisfied.is_empty());
    assert_eq!(receipt.failed.len(), 1);
    assert_eq!(receipt.failed[0].id, "com.vua.test.resolve.dep");
    assert_eq!(
        receipt.failed[0].reason_code, "vua.vpm.no_matching_package",
        "the failed arm reuses the standing code — zero new codes"
    );
    assert_eq!(
        hits.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "only the manifest fetch ran — the zip download never started"
    );
    assert_eq!(
        f6_tree_bytes(&project.root),
        before,
        "a failed resolve writes nothing — never a half-landed state"
    );
    fs::remove_dir_all(&base).ok();
}

/// The void world: official/curated repos ignored via the library's own
/// experimental switches and ZERO user repo rows — every declared dependency
/// is unknown to the collection world, so one resolve fails wholesale and the
/// library's DependenciesNotFound carries ALL declared roots at once. Zero
/// network faces exist by construction (no server, no repo rows).
fn f6_resolve_void_world(label: &str) -> (VrcGetLibBackend, ProjectRef, PathBuf) {
    let base = unique_dir(label);
    let environment_root = base.join("isolated-vpm-environment");
    fs::create_dir_all(environment_root.join("vrc-get")).unwrap();
    fs::write(
        environment_root.join("vrc-get/settings.json"),
        serde_json::json!({
            "ignoreOfficialRepository": true,
            "ignoreCuratedRepository": true
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        environment_root.join("settings.json"),
        serde_json::json!({ "userRepos": [] }).to_string(),
    )
    .unwrap();
    // 声明序刻意非字母序——夹具本身演示「载体迭代序 ≠ id 序」的漂移面。
    let project = f6_declared_project(
        &base.join("managed-project"),
        "2022.3.22f1",
        &[
            ("com.vua.test.missing.eta", "1.0.0"),
            ("com.vua.test.missing.alpha", "1.0.0"),
            ("com.vua.test.missing.delta", "1.0.0"),
            ("com.vua.test.missing.beta", "1.0.0"),
            ("com.vua.test.missing.gamma", "1.0.0"),
        ],
    );
    let backend = VrcGetLibBackend::with_environment_root(environment_root, false).unwrap();
    (backend, project, base)
}

/// 失败集可复现钉（批 195，操作者裁决采纳兑现）：多依赖全部不可解→
/// DependenciesNotFound 臂逐依赖进 failed。库内载体 MissingDependencies 是
/// 每进程随机盐的 HashMap，未排序时同一工程＋同一环境两次运行的 failed 序
/// 逐字漂移（失败证据不可复现）。排序后：同一输入两次调用（两次独立慢路
/// 径、两个独立映射实例——failed 解析零写入故第二次绝不走 locked 快路径）
/// 的 failed 序列逐字一致。钉的是「可复现性」而非具体序：具体序不进断言，
/// 未来任何确定性序替换不受本钉阻碍；failed 集合恒等于声明集（按集合比较，
/// 与序无关）、全员复用 no_matching_package、工程树逐字节零写入。
#[test]
fn f6_resolve_missing_dependencies_failed_sequence_is_reproducible_across_calls() {
    let (backend, project, base) = f6_resolve_void_world("f6-resolve-reproducible");
    let before = f6_tree_bytes(&project.root);

    let first = backend.resolve_project(&project.root).unwrap();
    let second = backend.resolve_project(&project.root).unwrap();

    assert!(first.resolved.is_empty(), "nothing resolves in a void world");
    assert!(
        first.already_satisfied.is_empty(),
        "nothing was ever locked — no already-satisfied face"
    );
    assert_eq!(
        first.failed.len(),
        5,
        "every declared dependency fails, carried at once"
    );
    // 集合相等（显式排序后比较）——断言集合成员，不断言收据顺序。
    let mut first_ids: Vec<&str> = first.failed.iter().map(|f| f.id.as_str()).collect();
    first_ids.sort_unstable();
    assert_eq!(
        first_ids,
        [
            "com.vua.test.missing.alpha",
            "com.vua.test.missing.beta",
            "com.vua.test.missing.delta",
            "com.vua.test.missing.eta",
            "com.vua.test.missing.gamma",
        ],
        "the failed set is exactly the declared set"
    );
    assert!(
        first
            .failed
            .iter()
            .all(|f| f.reason_code == "vua.vpm.no_matching_package"),
        "every failed entry reuses the standing code — zero new codes"
    );
    // 钉本体：同一输入两次调用，failed 序列逐字一致（可复现，非具体序）。
    assert_eq!(
        first.failed, second.failed,
        "the same project twice names its failures in the SAME order — \
         failure evidence is reproducible"
    );
    assert_eq!(
        f6_tree_bytes(&project.root),
        before,
        "a failed resolve writes nothing — never a half-landed state"
    );
    fs::remove_dir_all(&base).ok();
}

/// 仓库全禁用→解析失败诚实（与 F4 collection_world 语义互证）：禁用行的
/// 包不参与解析→failed 收据如实；被禁行的源站零接触（恰零连接）；对照臂
/// （重新启用同一世界、零其他改动）→同包解析落地成功——失败确证来自
/// 禁用过滤（离开集合世界），不是仓库不可达。
#[test]
fn f6_resolve_all_repos_disabled_leaves_the_package_unresolvable() {
    let (backend, project, base, hits) = f6_resolve_world("f6-resolve-disable", "1.0.0");
    let environment_root = base.join("isolated-vpm-environment");
    let before = f6_tree_bytes(&project.root);

    f4_write_state(&environment_root, &["com.vua.test.repo.resolve"]);
    let receipt = backend.resolve_project(&project.root).unwrap();
    assert!(receipt.resolved.is_empty());
    assert_eq!(receipt.failed.len(), 1);
    assert_eq!(receipt.failed[0].id, "com.vua.test.resolve.dep");
    assert_eq!(
        receipt.failed[0].reason_code, "vua.vpm.no_matching_package",
        "the disabled row's packages left the collection world — unresolvable, honestly"
    );
    assert_eq!(
        hits.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "the filtered row's origin is never contacted — leaving the collection world is not a fetch failure"
    );
    assert_eq!(
        f6_tree_bytes(&project.root),
        before,
        "nothing lands from a world that cannot resolve"
    );

    // 对照臂：重新启用（禁用集清空）、零其他改动→同包解析落地成功。
    f4_write_state(&environment_root, &[]);
    let receipt = backend.resolve_project(&project.root).unwrap();
    assert_eq!(
        receipt.resolved.len(),
        1,
        "the same package resolves once enabled"
    );
    assert_eq!(receipt.resolved[0].id, "com.vua.test.resolve.dep");
    assert!(
        project
            .root
            .join("Packages/com.vua.test.resolve.dep/package.json")
            .is_file(),
        "the enabled world lands the package the disabled world could not see"
    );
    assert!(
        hits.load(std::sync::atomic::Ordering::SeqCst) >= 2,
        "the enabled arm rode the loopback origin (manifest + zip)"
    );
    fs::remove_dir_all(&base).ok();
}

/// 离线钉：离线后端＋缓存可解析＋下载不可行→repo_fetch_failed 如实上浮
/// （端口面文档「repo_fetch_failed for the network segment」；库自身
/// "Offline mode" 错误随 reason 携带），绝不静默半成功；工程树逐字节不动、
/// 无半落地包目录。零出网：安装臂无 http，example.invalid 从不被接触。
#[test]
fn f6_resolve_offline_answers_repo_fetch_failed_never_half_success() {
    let base = unique_dir("f6-resolve-offline");
    let environment_root = base.join("isolated-vpm-environment");
    // 预定义两仓照 F4 先例以库自有实验开关 neutralize（零出网）。
    fs::create_dir_all(environment_root.join("vrc-get")).unwrap();
    fs::write(
        environment_root.join("vrc-get/settings.json"),
        serde_json::json!({
            "ignoreOfficialRepository": true,
            "ignoreCuratedRepository": true
        })
        .to_string(),
    )
    .unwrap();
    // 直接种子仓库缓存文件（离线 load_cache 腿可解析），包 url 指向
    // example.invalid（离线后端安装臂无 http——该地址从不被请求）。
    let cache_path = environment_root.join("Repos").join("resolve-repo.json");
    fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
    fs::write(
        &cache_path,
        serde_json::json!({
            "repo": {
                "name": "Synthetic Resolve Repo",
                "id": "com.vua.test.repo.resolve",
                "url": "https://example.invalid/vua/resolve-repo.json",
                "packages": {
                    "com.vua.test.resolve.dep": { "versions": {
                        "1.0.0": {
                            "name": "com.vua.test.resolve.dep",
                            "version": "1.0.0",
                            "vpmDependencies": {},
                            "url": "https://example.invalid/vua/pkg.zip"
                        }
                    } }
                }
            }
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        environment_root.join("settings.json"),
        serde_json::json!({ "userRepos": [{
            "localPath": cache_path.display().to_string(),
            "name": "Synthetic Resolve Repo",
            "id": "com.vua.test.repo.resolve",
            "url": "https://example.invalid/vua/resolve-repo.json"
        }] })
        .to_string(),
    )
    .unwrap();
    let project = f6_declared_project(
        &base.join("managed-project"),
        "2022.3.22f1",
        &[("com.vua.test.resolve.dep", "1.0.0")],
    );
    let backend = VrcGetLibBackend::with_environment_root(environment_root, true).unwrap();
    let before = f6_tree_bytes(&project.root);

    let error = backend.resolve_project(&project.root).unwrap_err();
    assert_eq!(
        error.code, "vua.vpm.repo_fetch_failed",
        "offline: the download arm surfaces the network-segment failure honestly"
    );
    let reason = match error.params.as_ref().and_then(|params| params.get("reason")) {
        Some(vua_orchestrator::ParamValue::Text(text)) => text.clone(),
        other => panic!("the failure carries the honest detail: {other:?}"),
    };
    assert!(
        reason.to_lowercase().contains("offline"),
        "the library's own Offline mode error rides the reason: {reason}"
    );
    assert_eq!(
        f6_tree_bytes(&project.root),
        before,
        "never a silent half success: the project tree is byte-identical"
    );
    assert!(
        !project.root.join("Packages/com.vua.test.resolve.dep").exists(),
        "no package directory landed"
    );
    fs::remove_dir_all(&base).ok();
}

/// 能力位双向钉（操作者指派缺席臂＋库位覆写）：库后端独立位如实 true；
/// CLI 后端如实 false 且缺席臂答 capability_missing、message_key 同词位
/// （与核心批 b3_batch146_capability_absence 互证互补）。
#[test]
fn f6_resolve_capabilities_absence_arm_on_cli_backend() {
    let library = VrcGetLibBackend::with_environment_root(unique_dir("f6-caps-lib"), true).unwrap();
    assert!(
        library.capabilities().resolve_project,
        "the library implements resolve in-process, so its independent bit is true"
    );

    let cli = backend_with(Arc::new(FakeProcessRunner::new()));
    assert!(
        !cli.capabilities().resolve_project,
        "no implementation, no declaration"
    );
    let project = minimal_vpm_project(&unique_dir("f6-caps-proj"));
    let error = cli.resolve_project(&project.root).unwrap_err();
    assert_eq!(error.code, "vua.vpm.capability_missing");
    assert_eq!(error.message_key, "errors.vpm.capabilityMissing");
}

/// F4 集合世界第六消费者同律：VUA 启停状态文件不可读→backend_unavailable
/// 共享构造器如实拒绝（与五装载点同事实同码），绝不以「全启用」猜测。
#[test]
fn f6_resolve_corrupt_state_file_refuses_like_every_collection_consumer() {
    let (backend, project, base, _hits) = f6_resolve_world("f6-resolve-state", "1.0.0");
    let environment_root = base.join("isolated-vpm-environment");
    f4_write_state(&environment_root, &[]);
    let state_path = f4_state_path(&environment_root);
    fs::write(&state_path, "not-json").unwrap();

    let error = backend.resolve_project(&project.root).unwrap_err();
    assert_eq!(
        error.code, "vua.vpm.backend_unavailable",
        "no state means no honest collection world — refuse, never guess"
    );
    fs::remove_dir_all(&base).ok();
}
