use vua_orchestrator::{
    BridgeError, FileSystemProjectStore, ProcessOutcome, ProcessRunner, ProcessSpec, ProjectRef,
    StdProcessRunner, UnityBridge, UnityCommand, UnityResult,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_BRIDGE_TIMEOUT: Duration = Duration::from_secs(1200);
/// The supervised interactive creator session waits for the player's Finish or Cancel, which
/// manual prefab steps can outgrow; the cap only ends a runaway process.
pub const FT_PATCH_SESSION_LIMIT: Duration = Duration::from_secs(8 * 60 * 60);
const OUTPUT_LIMIT: usize = 256 * 1024;

impl UnityBridge for UnityBatchBridge {
    fn execute(
        &self,
        project: &ProjectRef,
        command: &UnityCommand,
    ) -> Result<UnityResult, BridgeError> {
        UnityBatchBridge::execute(self, project, command)
    }
}

/// Executes Unity Bridge commands through the shared process runner port:
/// fixed executable, typed arguments, mandatory timeout and bounded output
/// (ORC-ADP-005, O6 "Unity 都通过该端口执行").
pub struct UnityBatchBridge {
    runner: Arc<dyn ProcessRunner>,
    executable: PathBuf,
    timeout: Duration,
    amf_production: bool,
}

impl UnityBatchBridge {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            runner: Arc::new(StdProcessRunner),
            executable: executable.into(),
            timeout: DEFAULT_BRIDGE_TIMEOUT,
            amf_production: false,
        }
    }

    pub fn with_runner(executable: impl Into<PathBuf>, runner: Arc<dyn ProcessRunner>) -> Self {
        Self {
            runner,
            executable: executable.into(),
            timeout: DEFAULT_BRIDGE_TIMEOUT,
            amf_production: false,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn for_amf_production(executable: impl Into<PathBuf>) -> Self {
        Self { amf_production: true, ..Self::new(executable) }
    }
}

impl UnityBatchBridge {
    pub fn execute(
        &self,
        project: &ProjectRef,
        command: &UnityCommand,
    ) -> Result<UnityResult, BridgeError> {
        self.execute_watched(project, command, &|| false)
    }

    /// Runs like [`Self::execute`] and calls `poll` repeatedly while Unity runs. When `poll`
    /// returns true, the Unity process tree is ended and the result is [`BridgeError::Cancelled`].
    /// The work project and its files stay as Unity left them.
    pub fn execute_watched(
        &self,
        project: &ProjectRef,
        command: &UnityCommand,
        poll: &(dyn Fn() -> bool + Send + Sync),
    ) -> Result<UnityResult, BridgeError> {
        let request =
            FileSystemProjectStore::write_bridge_command(project, &command.command_id, command)?;
        let result_path = project
            .root
            .join(".vua/bridge")
            .join(format!("{}.result.json", command.command_id));
        if result_path.exists() {
            fs::remove_file(&result_path)?;
        }
        let mut args = Self::invocation_args(&project.root, &request, &result_path);
        let ft_session =
            self.amf_production && command.operation == vua_orchestrator::UnityOperation::AmfFtPatch;
        if self.amf_production {
            if ft_session {
                // Only this versioned operation owns an interactive, supervised Editor session.
                args.retain(|arg| arg != "-batchmode" && arg != "-quit");
            }
            let index = args.iter().position(|a| a == "-executeMethod").expect("fixed Bridge arguments");
            args[index + 1] = "Vua.Editor.Bridge.AmfProductionEntryPoint.Run".into();
            let logs = project.root.join(".vua/logs");
            fs::create_dir_all(&logs)?;
            args.extend(["-logFile".into(), logs.join(format!("{}.log",command.command_id)).to_string_lossy().into_owned()]);
        }
        let spec = ProcessSpec {
            executable: self.executable.clone(),
            args,
            working_dir: None,
            timeout: if ft_session { FT_PATCH_SESSION_LIMIT } else { self.timeout },
            output_limit: OUTPUT_LIMIT,
            // R2-2: Unity 子进程同样剥离凭据基线。
            removals: vua_orchestrator::CREDENTIAL_ENV_REMOVALS
                .iter()
                .map(|key| key.to_string())
                .collect(),
            ..Default::default()
        };
        let outcome = self.runner.run_cancellable(&spec, poll).map_err(|error| match error {
            vua_orchestrator::ProcessError::Io(io) => BridgeError::Io(io),
        })?;
        if ft_session && !outcome.process_tree_clean {
            return Err(BridgeError::ProcessTreeUnclean);
        }
        Self::collect(&outcome, &result_path)
    }

    /// Maps a process outcome plus the expected result file into a bridge
    /// result. Shared with tests through the runner port.
    fn collect(outcome: &ProcessOutcome, result_path: &Path) -> Result<UnityResult, BridgeError> {
        if outcome.cancelled {
            return Err(BridgeError::Cancelled);
        }
        if outcome.timed_out {
            return Err(BridgeError::TimedOut);
        }
        if !outcome.success() && !result_path.exists() {
            return Err(BridgeError::UnityFailed(outcome.exit_code));
        }
        if !result_path.exists() {
            return Err(BridgeError::MissingResult);
        }
        let bytes = fs::read(result_path)?;
        serde_json::from_slice(&bytes).map_err(BridgeError::InvalidResult)
    }

    pub fn invocation_args(project: &Path, request: &Path, result: &Path) -> Vec<String> {
        vec![
            "-batchmode".into(),
            "-quit".into(),
            "-projectPath".into(),
            project.to_string_lossy().into_owned(),
            "-executeMethod".into(),
            "Vua.Editor.Bridge.BridgeEntryPoint.Run".into(),
            "-vuaRequest".into(),
            request.to_string_lossy().into_owned(),
            "-vuaResult".into(),
            result.to_string_lossy().into_owned(),
        ]
    }
}

#[cfg(test)]
mod face_tracking_tests {
    use super::*;
    use std::sync::Mutex;
    use vua_orchestrator::{ProcessError, UnityOperation, UnityPayload};

    struct Capture(Mutex<Vec<ProcessSpec>>, bool);
    impl ProcessRunner for Capture {
        fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutcome, ProcessError> {
            self.0.lock().unwrap().push(spec.clone());
            Ok(ProcessOutcome { exit_code: Some(0), timed_out: false, cancelled: false,
                process_tree_clean: self.1, stdout: String::new(), stderr: String::new(), truncated: false })
        }
    }
    #[test]
    fn only_the_versioned_patch_uses_an_interactive_supervised_session_and_requires_a_receipt() {
        let root = std::env::temp_dir().join(format!("vua-ft-args-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let capture = Arc::new(Capture(Mutex::new(Vec::new()), true));
        let mut bridge = UnityBatchBridge::with_runner("synthetic-unity", capture.clone());
        bridge.amf_production = true;
        let project = ProjectRef { id: "ft-test".into(), root: root.clone() };
        for operation in [UnityOperation::AmfInspect, UnityOperation::AmfFtPatch] {
            let command = UnityCommand { schema_version: 5, command_id: format!("ft-{operation:?}"),
                operation, project_id: project.id.clone(), dry_run: operation == UnityOperation::AmfInspect,
                expected_project_fingerprint: Some("synthetic".into()), payload: UnityPayload::default() };
            assert!(matches!(bridge.execute(&project, &command), Err(BridgeError::MissingResult)));
        }
        let calls = capture.0.lock().unwrap();
        assert!(calls[0].args.contains(&"-batchmode".into()) && calls[0].args.contains(&"-quit".into()));
        assert_eq!(calls[0].timeout, Duration::from_secs(1200));
        assert!(!calls[1].args.contains(&"-batchmode".into()) && !calls[1].args.contains(&"-quit".into()));
        assert!(calls[1].args.contains(&"Vua.Editor.Bridge.AmfProductionEntryPoint.Run".into()));
        // The session waits for the player's Finish or Cancel; only a runaway process hits the cap.
        assert_eq!(calls[1].timeout, FT_PATCH_SESSION_LIMIT);
        assert!(calls[1].removals.contains(&"GH_TOKEN".into()));
        drop(calls);
        let mut unclean = UnityBatchBridge::with_runner("synthetic-unity", Arc::new(Capture(Mutex::new(Vec::new()), false)));
        unclean.amf_production = true;
        let command = UnityCommand { schema_version: 5, command_id: "ft-unclean".into(),
            operation: UnityOperation::AmfFtPatch, project_id: project.id.clone(), dry_run: false,
            expected_project_fingerprint: Some("synthetic".into()), payload: UnityPayload::default() };
        assert!(matches!(unclean.execute(&project, &command), Err(BridgeError::ProcessTreeUnclean)));
        fs::remove_dir_all(root).unwrap();
    }
}
