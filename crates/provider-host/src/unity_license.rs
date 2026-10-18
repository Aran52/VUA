//! Private license metadata and a VUA-owned Editor check. License files are never opened.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use vua_orchestrator::{
    amf_production::ProductionEditor, unity_license::*, AppErrorV1, Clock, EnvironmentEngine,
    EnvironmentPresence, EnvironmentRoots, ErrorCategory, ProcessRunner, ProcessSpec,
    SubmitRequest, SystemClock, TaskContext, TaskExit,
};

type HubFinding = Arc<dyn Fn() -> vua_orchestrator::EnvironmentCheckItemV1 + Send + Sync>;
type Processes = Arc<dyn Fn() -> Option<Vec<String>> + Send + Sync>;
type Editors = Arc<dyn Fn() -> Result<Vec<ProductionEditor>, AppErrorV1> + Send + Sync>;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct LicensePaths {
    pub local_app_data: Option<PathBuf>,
    pub program_data: Option<PathBuf>,
}

pub trait LicenseProbe: Send + Sync {
    fn observe(&self) -> LicenseObservation;
    fn verify_editor(&self, path: &str) -> Result<ProductionEditor, AppErrorV1>;
    fn ensure_client(&self, cancelled: &(dyn Fn() -> bool + Send + Sync)) -> bool;
}

pub struct NativeLicenseProbe {
    paths: LicensePaths,
    hub: HubFinding,
    processes: Processes,
    editors: Editors,
}

fn error(code: &str) -> AppErrorV1 {
    AppErrorV1::new(
        format!("vua.unity_license.{code}"),
        if code == "unavailable" {
            ErrorCategory::Unavailable
        } else {
            ErrorCategory::Validation
        },
        "errors.unityLicense.failed",
        "unity-license",
    )
    .with_recoverable(true)
}

impl NativeLicenseProbe {
    /// Construction and observation do not run tools; Hub discovery reads files and registry.
    pub fn system() -> Self {
        let engine = EnvironmentEngine::new(
            Arc::new(vua_orchestrator::StdProcessRunner),
            Arc::new(SystemClock),
            EnvironmentRoots::default(),
            Arc::new(vua_project_manager::VccSettingsFileReader),
        );
        Self {
            paths: LicensePaths {
                local_app_data: std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
                program_data: Some(
                    std::env::var_os("ProgramData")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from("C:/ProgramData")),
                ),
            },
            hub: Arc::new(move || engine.inspect_unity_hub()),
            processes: Arc::new(crate::unity_hub::running_processes),
            editors: Arc::new(|| Ok(Vec::new())),
        }
    }
    pub(crate) fn with_editors(mut self, editors: Editors) -> Self {
        self.editors = editors;
        self
    }
}

fn file_metadata(path: Option<PathBuf>) -> (Option<bool>, Option<String>, bool) {
    let Some(path) = path else {
        return (None, None, false);
    };
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => (
            Some(true),
            metadata.modified().ok().map(vua_orchestrator::rfc3339),
            metadata.len() > 0,
        ),
        Ok(_) => (None, None, false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Some(false), None, false),
        Err(_) => (None, None, false),
    }
}

impl LicenseProbe for NativeLicenseProbe {
    fn observe(&self) -> LicenseObservation {
        let paths = [
            self.paths.local_app_data.as_ref().map(|p| p.join("Unity/licenses/UnityEntitlementLicense.xml")),
            self.paths.local_app_data.as_ref().map(|p| p.join("Packages/UnityTechnologies.UnityHub_2vrhnee42bhxm/LocalCache/Local/Unity/licenses/UnityEntitlementLicense.xml")),
            self.paths.program_data.as_ref().map(|p| p.join("Unity/Unity_lic.ulf")),
        ];
        let kinds = [
            LicenseKind::ClassicEntitlement,
            LicenseKind::MsixEntitlement,
            LicenseKind::LegacyUlf,
        ];
        let metadata = paths.map(file_metadata);
        let files = std::array::from_fn(|i| LicenseFile {
            kind: kinds[i],
            present: metadata[i].0,
            modified_at: metadata[i].1.clone(),
        });
        let finding = (self.hub)();
        let names = (self.processes)();
        let running = |name: &str| {
            names
                .as_ref()
                .map(|names| names.iter().any(|p| p.eq_ignore_ascii_case(name)))
        };
        let cli = file_metadata(
            self.paths
                .local_app_data
                .as_ref()
                .map(|p| p.join("Unity/bin/unity.exe")),
        )
        .0;
        LicenseObservation {
            complete: finding.presence != EnvironmentPresence::DetectionFailed
                && files
                    .iter()
                    .all(|file| file.present != Some(true) || file.modified_at.is_some()),
            files,
            nonempty: metadata.map(|m| m.2),
            hub_type: if finding.presence != EnvironmentPresence::Detected {
                HubType::None
            } else if finding.facts["via"] == "package" {
                HubType::Msix
            } else {
                HubType::Classic
            },
            hub_running: running("Unity Hub.exe")
                .zip(running("UnityHub.exe"))
                .map(|(a, b)| a || b),
            licensing_client_running: running("Unity.Licensing.Client.exe"),
            hub_cli_present: cli,
        }
    }
    fn verify_editor(&self, path: &str) -> Result<ProductionEditor, AppErrorV1> {
        let target = fs::canonicalize(path).map_err(|_| error("editor_unavailable"))?;
        let discovered = (self.editors)()
            .map_err(|_| error("editor_unavailable"))?
            .into_iter()
            .any(|editor| {
                fs::canonicalize(editor.path).ok().is_some_and(|p| {
                    p.to_string_lossy()
                        .eq_ignore_ascii_case(&target.to_string_lossy())
                })
            });
        if !discovered {
            return Err(error("editor_unavailable"));
        }
        match vua_project_manager::verify_editor_path_system(&target) {
            vua_project_manager::EditorPathVerdict::Verified(identity)
                if matches!(identity.version.as_str(), "2022.3.22f1" | "2022.3.22f1c1") =>
            {
                Ok(ProductionEditor {
                    path: identity.exe_path,
                    version: identity.version,
                })
            }
            _ => Err(error("editor_unavailable")),
        }
    }
    fn ensure_client(&self, cancelled: &(dyn Fn() -> bool + Send + Sync)) -> bool {
        if cancelled() {
            return false;
        }
        crate::unity_hub::ensure_licensing_client(
            crate::unity_hub::hub_launch(&(self.hub)().facts),
            cancelled,
        )
    }
}

pub(crate) struct UnityLicenseService {
    pub(crate) probe: Arc<dyn LicenseProbe>,
    pub(crate) root: PathBuf,
    pub(crate) amf: Option<Arc<crate::amf_production_routes::AmfProductionServices>>,
    pub(crate) runner: Arc<dyn ProcessRunner>,
    pub(crate) clock: Arc<dyn Clock>,
}

impl UnityLicenseService {
    pub(crate) fn snapshot(&self) -> UnityLicenseSnapshot {
        snapshot(self.probe.observe(), self.last_check())
    }
    fn last_check(&self) -> Option<CheckedLicense> {
        let path = self.root.join("last-check.json");
        let metadata = fs::symlink_metadata(&path).ok()?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 4096 {
            return None;
        }
        let checked: CheckedLicense = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
        valid_checked_license(&checked).then_some(checked)
    }
    fn save(&self, checked: &CheckedLicense) -> Result<(), AppErrorV1> {
        owned_directory(&self.root)?;
        let path = self.root.join("last-check.json");
        reject_link(&path)?;
        let pending = self.root.join("last-check.pending");
        reject_link(&pending)?;
        let mut file = fs::File::create(&pending).map_err(|_| error("store_failed"))?;
        file.write_all(&serde_json::to_vec(checked).map_err(|_| error("store_failed"))?)
            .map_err(|_| error("store_failed"))?;
        file.sync_all().map_err(|_| error("store_failed"))?;
        drop(file);
        fs::rename(pending, path).map_err(|_| error("store_failed"))
    }
    pub(crate) fn submit(
        self: &Arc<Self>,
        editor_path: &str,
        correlation: &str,
        command_id: &str,
    ) -> Result<Value, AppErrorV1> {
        let amf = self.amf.as_ref().ok_or_else(|| error("unavailable"))?;
        self.probe.verify_editor(editor_path)?;
        let (worker, path) = (self.clone(), editor_path.to_owned());
        let fingerprint: String = Sha256::digest(editor_path.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let accepted = amf.submit_exclusive(
            SubmitRequest {
                correlation_id: Some(correlation.into()),
                timeout: None,
                job: Box::new(move |ctx| worker.check(&path, ctx)),
            },
            "environment.checkUnityLicense",
            command_id,
            &fingerprint,
        )?;
        Ok(json!({"schemaVersion":"0.1","taskId":accepted.task_id}))
    }
    fn check(&self, editor_path: &str, ctx: &TaskContext) -> Result<TaskExit, AppErrorV1> {
        let editor = self.probe.verify_editor(editor_path)?;
        let project = self.project(&editor.version)?;
        ctx.emit_progress(
            json!({"operation":"environment.checkUnityLicense","stage":"licensing_client"}),
        );
        if self.probe.observe().licensing_client_running != Some(true) {
            self.probe.ensure_client(&|| ctx.check_cancel());
        }
        let (state, reason, cancelled) = if ctx.check_cancel() {
            (
                LicenseCheckState::Unknown,
                Some("The license check was cancelled."),
                true,
            )
        } else {
            // Hub readiness may take time. Reverify the executable immediately before launch.
            let verified = self.probe.verify_editor(editor_path)?;
            if verified.version != editor.version || verified.path != editor.path {
                return Err(error("editor_unavailable"));
            }
            let log = project.join(".vua").join(format!(
                "check-{}.log",
                crate::provider_host::uuid_v7_identity()
            ));
            let spec = ProcessSpec {
                executable: PathBuf::from(&editor.path),
                args: vec![
                    "-batchmode".into(),
                    "-nographics".into(),
                    "-quit".into(),
                    "-projectPath".into(),
                    project.to_string_lossy().into_owned(),
                    "-logFile".into(),
                    log.to_string_lossy().into_owned(),
                ],
                working_dir: Some(project.clone()),
                timeout: Duration::from_secs(300),
                output_limit: 0,
                ..Default::default()
            };
            ctx.emit_progress(
                json!({"operation":"environment.checkUnityLicense","stage":"editor"}),
            );
            match self.runner.run_cancellable(&spec, &|| ctx.check_cancel()) {
                Err(_) => (
                    LicenseCheckState::Unknown,
                    Some("Unity could not be started."),
                    false,
                ),
                Ok(outcome) if outcome.cancelled || ctx.check_cancel() => (
                    LicenseCheckState::Unknown,
                    Some("The license check was cancelled."),
                    true,
                ),
                Ok(outcome) if outcome.timed_out => (
                    LicenseCheckState::Unknown,
                    Some("Unity did not finish within five minutes."),
                    false,
                ),
                Ok(outcome) => match read_log(&log) {
                    Err(()) => (
                        LicenseCheckState::Unknown,
                        Some("Unity's check log could not be read completely."),
                        false,
                    ),
                    Ok(Some(_)) => match self.probe.observe().licensing_client_running {
                        Some(true) => (LicenseCheckState::NotActivated, None, false),
                        Some(false) => (LicenseCheckState::NeedsHubClient, None, false),
                        None => (
                            LicenseCheckState::Unknown,
                            Some("The licensing client could not be observed."),
                            false,
                        ),
                    },
                    Ok(None) if outcome.exit_code == Some(0) => {
                        (LicenseCheckState::Active, None, false)
                    }
                    Ok(None) => (
                        LicenseCheckState::Unknown,
                        Some("Unity exited without completing the license check."),
                        false,
                    ),
                },
            }
        };
        let checked = CheckedLicense {
            state,
            editor_version: editor.version,
            checked_at: self.clock.now_rfc3339(),
        };
        self.save(&checked)?;
        if cancelled {
            return Ok(TaskExit::Cancelled);
        }
        if state != LicenseCheckState::Active {
            ctx.warn();
        }
        let mut result = serde_json::to_value(checked).expect("closed license record");
        result["schemaVersion"] = json!("0.1");
        result["reason"] = json!(reason);
        Ok(TaskExit::Done(result))
    }
    fn project(&self, version: &str) -> Result<PathBuf, AppErrorV1> {
        owned_directory(&self.root)?;
        let project = self.root.join("project");
        let marker = project.join(".vua-license-check");
        const OWNER: &[u8] = b"VUA Unity license check v0.1\n";
        reject_link(&project)?;
        if project.exists() {
            reject_link(&marker)?;
            if fs::read(&marker).ok().as_deref() != Some(OWNER) {
                return Err(error("project_unavailable"));
            }
        } else {
            fs::create_dir(&project).map_err(|_| error("project_unavailable"))?;
            fs::write(&marker, OWNER).map_err(|_| error("project_unavailable"))?;
        }
        for folder in ["Assets", "Packages", "ProjectSettings", ".vua"] {
            owned_directory(&project.join(folder))?;
        }
        if fs::read_dir(project.join("Assets"))
            .map_err(|_| error("project_unavailable"))?
            .next()
            .is_some()
        {
            return Err(error("project_unavailable"));
        }
        for entry in
            fs::read_dir(project.join("Packages")).map_err(|_| error("project_unavailable"))?
        {
            let entry = entry.map_err(|_| error("project_unavailable"))?;
            reject_link(&entry.path())?;
            if !["manifest.json", "packages-lock.json"]
                .iter()
                .any(|name| entry.file_name() == *name)
            {
                return Err(error("project_unavailable"));
            }
        }
        let manifest = project.join("Packages/manifest.json");
        const MANIFEST: &[u8] = b"{\"dependencies\":{}}\n";
        reject_link(&manifest)?;
        if manifest.exists() {
            let existing = fs::read(&manifest)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
            if existing != Some(json!({"dependencies":{}})) {
                return Err(error("project_unavailable"));
            }
        } else {
            fs::write(manifest, MANIFEST).map_err(|_| error("project_unavailable"))?;
        }
        let settings = project.join("ProjectSettings/ProjectVersion.txt");
        reject_link(&settings)?;
        fs::write(settings, format!("m_EditorVersion: {version}\n"))
            .map_err(|_| error("project_unavailable"))?;
        Ok(project)
    }
}

fn reject_link(path: &Path) -> Result<(), AppErrorV1> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(error("project_unavailable")),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(error("project_unavailable")),
    }
}
fn owned_directory(path: &Path) -> Result<(), AppErrorV1> {
    reject_link(path)?;
    fs::create_dir_all(path).map_err(|_| error("project_unavailable"))
}
fn read_log(path: &Path) -> Result<Option<LicenseLogCategory>, ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 8 * 1024 * 1024
    {
        return Err(());
    }
    let file = fs::File::open(path).map_err(|_| ())?;
    for line in BufReader::new(file).lines() {
        if let Some(category) = classify_license_log(&line.map_err(|_| ())?) {
            return Ok(Some(category));
        }
    }
    Ok(None)
}
