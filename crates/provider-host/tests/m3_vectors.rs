//! M3/T1 fixed-vector contract tests: every vector under
//! `schemas/amf-production/v0.2/vectors/` is executed against the real
//! provider host (real executor, instant fake Bridge, synthetic fixture
//! tree). The F side (packages/contracts + mock provider) consumes the SAME
//! files — any drift between schema, vectors and either implementation
//! fails here first. "三处同批" discipline.

#![allow(clippy::result_large_err)]

use std::fs;
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use flate2::write::GzEncoder;
use tar::{Builder, Header};

use vua_orchestrator::{
    AppErrorV1, BuildRecordStore, ChangePreviewV1, FileSystemSnapshotStore, FixedClock,
    PackageRequestV1, ProjectRef, ResultStatus, UnityBridge, UnityCommand, UnityResult, VpmBackend,
    VpmCapabilities,
};
use vua_provider_host::ProductionConfig;
use vua_unity_bridge::{LocalPackageIdentityStore, MaterialExecutor};

// --- fakes ---

struct NoBridge;
impl UnityBridge for NoBridge {
    fn execute(
        &self,
        _project: &ProjectRef,
        command: &UnityCommand,
    ) -> Result<UnityResult, vua_orchestrator::BridgeError> {
        Ok(UnityResult {
            schema_version: 1,
            command_id: command.command_id.clone(),
            status: ResultStatus::Succeeded,
            changed_paths: vec![],
            diagnostics: vec![],
            data: serde_json::json!({"projectFingerprint": "fp-vector"}),
            steps: Vec::new(),
            replayed: None,
            snapshot_id: None,
            restored_from: None,
            project_fingerprint_before: None,
        })
    }
}

fn capability_missing(capability: &str) -> AppErrorV1 {
    AppErrorV1::new(
        "vua.vpm.capability_missing",
        vua_orchestrator::ErrorCategory::Unavailable,
        "errors.vpm.capabilityMissing",
        "corr-vectors",
    )
    .with_param(
        "capability",
        vua_orchestrator::ParamValue::Text(capability.to_owned()),
    )
}

struct NoVpm;
impl VpmBackend for NoVpm {
    fn name(&self) -> &'static str {
        "none"
    }
    fn capabilities(&self) -> VpmCapabilities {
        VpmCapabilities {
            create_project: false,
            preview_install: false,
            list_packages: false,
            remove_packages: false,
            project_registry: false,
            resolve_project: false,
        }
    }
    fn preview_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
    ) -> Result<ChangePreviewV1, AppErrorV1> {
        Err(capability_missing("preview_install"))
    }
    fn apply_install(
        &self,
        _project: &ProjectRef,
        _packages: &[PackageRequestV1],
        _confirmed_digest: &str,
    ) -> Result<serde_json::Value, AppErrorV1> {
        Err(capability_missing("apply_install"))
    }
    fn create_project(
        &self,
        _parent: &Path,
        _name: &str,
        _template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1> {
        Err(capability_missing("create_project"))
    }
}

// --- frame plumbing ---

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vua-m3vec-{label}-{nanos}"))
}

struct Paced {
    inner: std::io::Cursor<Vec<u8>>,
}

impl Read for Paced {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::thread::sleep(Duration::from_millis(60));
        self.inner.read(buf)
    }
}

impl BufRead for Paced {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.inner.fill_buf()
    }
    fn consume(&mut self, amount: usize) {
        self.inner.consume(amount);
    }
}

fn frames_input(frames: Vec<serde_json::Value>) -> Paced {
    let mut bytes = Vec::new();
    for frame in frames {
        bytes.extend_from_slice(frame.to_string().as_bytes());
        bytes.push(b'\n');
    }
    Paced {
        inner: std::io::Cursor::new(bytes),
    }
}

fn frame(id: &str, payload: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "frameVersion": "0.1",
        "frameId": id,
        "kind": "request",
        "payload": payload,
    })
}

fn request(
    request_id: &str,
    method: &str,
    command_id: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "contractVersion": "0.1",
        "requestId": request_id,
        "correlationId": "corr-m3-vectors",
        "kind": "command",
        "method": method,
        "commandId": command_id,
        "params": params,
    })
}

fn parse_frames(bytes: &[u8]) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn production_config(base: &Path) -> ProductionConfig {
    ProductionConfig {
        executor: Arc::new(MaterialExecutor::new(
            Arc::new(NoBridge),
            FileSystemSnapshotStore,
            Arc::new(NoVpm),
            BuildRecordStore::new(base.join("records")),
            Arc::new(FixedClock::new(&["2026-09-06T00:00:00Z"])),
            base.join("temp"),
            "2022.3.22f1",
            LocalPackageIdentityStore::new(base.join("identities.json")),
        )),
        records: Arc::new(BuildRecordStore::new(base.join("records"))),
    }
}

// --- fixture world ---

struct World {
    base: PathBuf,
    database: PathBuf,
    source: String,
    project_root: String,
    artifact_output_root: String,
}

fn make_world(label: &str) -> World {
    let base = unique_dir(label);
    let source = base.join("source");
    fs::create_dir_all(&source).unwrap();
    let package = source.join("pack.unitypackage");
    let file = fs::File::create(&package).unwrap();
    let mut builder = Builder::new(GzEncoder::new(file, flate2::Compression::default()));
    let mut header = Header::new_gnu();
    header.set_size(8);
    header.set_cksum();
    builder
        .append_data(&mut header, "Assets/Asset.prefab", &b"fixture!"[..])
        .unwrap();
    builder.finish().unwrap();

    let project_root = base.join("target");
    for dir in ["Assets", "Packages", "ProjectSettings"] {
        fs::create_dir_all(project_root.join(dir)).unwrap();
    }
    fs::write(project_root.join("vpm-manifest.json"), "{}").unwrap();
    fs::create_dir_all(project_root.join("ProjectSettings")).unwrap();
    fs::write(
        project_root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.22f1\n",
    )
    .unwrap();
    let artifact_output_root = base.join("artifacts");
    fs::create_dir_all(&artifact_output_root).unwrap();

    World {
        database: base.join("provider.db"),
        source: source.to_string_lossy().into_owned(),
        project_root: project_root.to_string_lossy().into_owned(),
        artifact_output_root: artifact_output_root.to_string_lossy().into_owned(),
        base,
    }
}

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/amf-production/v0.2/vectors")
}

fn substitute(
    value: &serde_json::Value,
    context: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    match value {
        serde_json::Value::String(text) => {
            if let Some(key) = text.strip_prefix('$') {
                if let Some(replacement) = context.get(key) {
                    return replacement.clone();
                }
            }
            value.clone()
        }
        serde_json::Value::Object(object) => {
            let mut mapped = serde_json::Map::new();
            for (key, inner) in object {
                mapped.insert(key.clone(), substitute(inner, context));
            }
            serde_json::Value::Object(mapped)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(|item| substitute(item, context)).collect())
        }
        other => other.clone(),
    }
}

fn check_entry(value: &serde_json::Value, checks: &serde_json::Value) -> Vec<String> {
    let mut failures = Vec::new();
    if let Some(checks_object) = checks.as_object() {
        for (pointer, expectation) in checks_object {
            // Dotted paths ("a.b.c") convert to JSON-pointer form.
            let pointer_path = format!("/{}", pointer.replace('.', "/"));
            let Some(target) = value.pointer(&pointer_path) else {
                failures.push(format!("{pointer}: missing"));
                continue;
            };
            if let Some(prefix) = expectation["startsWith"].as_str() {
                let matches = target
                    .as_str()
                    .map(|s| s.starts_with(prefix))
                    .unwrap_or(false);
                if !matches {
                    failures.push(format!("{pointer}: {target} does not start with {prefix}"));
                }
            }
            if let Some(expected) = expectation["equals"].as_str() {
                if target.as_str() != Some(expected) {
                    failures.push(format!("{pointer}: {target} != {expected:?}"));
                }
            }
            if let Some(expected) = expectation["equals"].as_array() {
                if target.as_array() != Some(expected) {
                    failures.push(format!("{pointer}: {target} != {expected:?}"));
                }
            }
            if expectation["isArray"].as_bool() == Some(true) && target.as_array().is_none() {
                failures.push(format!("{pointer}: expected an array"));
            }
        }
    }
    failures
}

fn run_step(
    database: &Path,
    method: &str,
    command_id: &str,
    params: serde_json::Value,
) -> (bool, Option<String>, serde_json::Value) {
    let mut output_buffer = Vec::new();
    vua_provider_host::run_provider_host_with(
        frames_input(vec![frame(
            &format!("f-{command_id}"),
            request(&format!("req-{command_id}"), method, command_id, params),
        )]),
        &mut output_buffer,
        database,
        Some(production_config(database.parent().expect("db parent"))),
    )
    .unwrap();
    let frames = parse_frames(&output_buffer);
    let frame = frames
        .iter()
        .find(|frame| frame["kind"] == "response")
        .expect("a response frame");
    let ok = frame["payload"]["ok"].as_bool().unwrap_or(false);
    let error_code = frame["payload"]["error"]["code"]
        .as_str()
        .map(str::to_owned);
    let value = frame["payload"]["value"].clone();
    (ok, error_code, value)
}

#[test]
fn m3_t1_fixed_vectors_run_green_against_the_provider_host() {
    let dir = vectors_dir();
    let mut vector_files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().map(|ext| ext == "json").unwrap_or(false))
        .collect();
    vector_files.sort();
    assert!(
        vector_files.len() >= 8,
        "the vector set must stay populated (found {:?})",
        vector_files
    );

    let world = make_world("vectors");
    let mut context: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    context.insert("sourceFolder".into(), world.source.clone().into());
    context.insert("projectRoot".into(), world.project_root.clone().into());
    context.insert(
        "artifactOutputRoot".into(),
        world.artifact_output_root.clone().into(),
    );
    context.insert("projectId".into(), "project".into());

    for vector_path in vector_files {
        let vector: serde_json::Value =
            serde_json::from_slice(&fs::read(&vector_path).unwrap()).unwrap();
        let name = vector["name"].as_str().unwrap().to_owned();
        let steps = vector["steps"].as_array().expect("vector steps").clone();
        for (index, step) in steps.iter().enumerate() {
            let method = step["method"].as_str().unwrap().to_owned();
            let command_id = format!("{name}-{index}");
            let params = substitute(&step["params"], &context);
            let (ok, error_code, value) = run_step(&world.database, &method, &command_id, params);

            let expect_ok = step["expect"]["ok"].as_bool().unwrap_or(true);
            assert_eq!(ok, expect_ok, "{name} step {index}: payload {value}");
            if let Some(expected_code) = step["expect"]["errorCode"].as_str() {
                let code = error_code.as_deref().unwrap_or_default();
                assert_eq!(code, expected_code, "{name} step {index}");
            }

            // Harvest identities into the context for later steps.
            for key in ["inspectionId", "planId", "taskId", "buildRecordId"] {
                if let Some(id) = value[key].as_str() {
                    context.insert(key.to_owned(), id.to_owned().into());
                }
            }
            if let Some(revision) = value["revision"].as_u64() {
                context.insert("revision".into(), revision.into());
            }

            // A confirm step waits for its worker to reach a terminal state
            // so later steps (getBuildRecord) see the receipt.
            if step["waitTerminal"].as_str() == Some("$taskId") {
                let task_id = context["taskId"].as_str().expect("taskId").to_owned();
                let store = vua_orchestrator::SqliteTaskStore::open(&world.database).unwrap();
                let deadline = Instant::now() + Duration::from_secs(15);
                loop {
                    let task = store.task(&task_id).unwrap().expect("task persists");
                    if task.state.is_terminal() {
                        break;
                    }
                    assert!(Instant::now() < deadline, "{name}: confirm never completed");
                    std::thread::sleep(Duration::from_millis(20));
                }
            }

            let failures = check_entry(&value, &step["expect"]["checks"]);
            assert!(failures.is_empty(), "{name} step {index}: {failures:?}");
        }
    }
    fs::remove_dir_all(&world.base).ok();
}
