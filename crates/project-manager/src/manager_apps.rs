//! Manager application executable discovery (`v0.1`) and detached breakaway launch (`v0.2`).
//! Config files are deliberately not installation evidence, and external projects are never
//! passed as CLI write targets (Ruling U3).

use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use vua_orchestrator::{
    Clock, EnvironmentEngine, EnvironmentPresence, EnvironmentRoots, RegistryHive, SystemClock,
};

const MAX_COMMAND_LEDGER: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagerAppId {
    UnityHub,
    Vcc,
    Alcom,
}

impl ManagerAppId {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "unity_hub" => Some(Self::UnityHub),
            "vcc" => Some(Self::Vcc),
            "alcom" => Some(Self::Alcom),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnityHub => "unity_hub",
            Self::Vcc => "vcc",
            Self::Alcom => "alcom",
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerApps {
    pub schema_version: &'static str,
    pub captured_at: String,
    pub apps: Vec<ManagerApp>,
}

#[derive(Serialize)]
pub struct ManagerApp {
    pub component: &'static str,
    pub presence: &'static str,
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerAppLaunch {
    pub schema_version: &'static str,
    pub manager_id: &'static str,
    pub launched_at: String,
}

pub trait ManagerProcessSpawner: Send + Sync {
    fn spawn_detached(&self, executable: &Path) -> Result<(), &'static str>;
}

pub struct WindowsManagerProcessSpawner;

impl ManagerProcessSpawner for WindowsManagerProcessSpawner {
    fn spawn_detached(&self, executable: &Path) -> Result<(), &'static str> {
        if !valid_pe_executable(executable).unwrap_or(false) {
            return Err("not_found");
        }
        let parent = executable.parent().ok_or("launch_failed")?;
        let mut command = std::process::Command::new(executable);
        vua_orchestrator::strip_sensitive_env(&mut command);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use windows_sys::Win32::System::Threading::CREATE_BREAKAWAY_FROM_JOB;
            command.creation_flags(CREATE_BREAKAWAY_FROM_JOB);
        }
        command
            .current_dir(parent)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let mut child = command.spawn().map_err(|_| "launch_failed")?;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }
}

type ExecutableResolver = dyn Fn(&EnvironmentRoots, ManagerAppId) -> Option<PathBuf> + Send + Sync;

pub struct ManagerAppLauncher {
    roots: EnvironmentRoots,
    resolver: Arc<ExecutableResolver>,
    spawner: Arc<dyn ManagerProcessSpawner>,
    clock: Arc<dyn Clock>,
    launch_gate: Mutex<()>,
    ledger: Mutex<BTreeMap<String, ManagerAppLaunch>>,
}

impl ManagerAppLauncher {
    pub fn windows(roots: EnvironmentRoots) -> Self {
        Self {
            roots,
            resolver: Arc::new(resolve_executable_path),
            spawner: Arc::new(WindowsManagerProcessSpawner),
            clock: Arc::new(SystemClock),
            launch_gate: Mutex::new(()),
            ledger: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn with_dependencies(
        roots: EnvironmentRoots,
        resolver: Arc<ExecutableResolver>,
        spawner: Arc<dyn ManagerProcessSpawner>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            roots,
            resolver,
            spawner,
            clock,
            launch_gate: Mutex::new(()),
            ledger: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn open(
        &self,
        manager_id: ManagerAppId,
        command_id: &str,
    ) -> Result<ManagerAppLaunch, &'static str> {
        if !valid_command_id(command_id) {
            return Err("invalid_command");
        }
        let _gate = self.launch_gate.lock().map_err(|_| "unavailable")?;
        {
            let ledger = self.ledger.lock().map_err(|_| "unavailable")?;
            if let Some(existing) = ledger.get(command_id) {
                if existing.manager_id == manager_id.as_str() {
                    return Ok(existing.clone());
                }
                return Err("command_conflict");
            }
            if ledger.len() >= MAX_COMMAND_LEDGER {
                return Err("command_conflict");
            }
        }

        let executable = (self.resolver)(&self.roots, manager_id).ok_or("not_found")?;
        if !valid_pe_executable(&executable).unwrap_or(false) {
            return Err("not_found");
        }
        self.spawner.spawn_detached(&executable)?;

        let receipt = ManagerAppLaunch {
            schema_version: "vua.manager-apps/v0.2",
            manager_id: manager_id.as_str(),
            launched_at: self.clock.now_rfc3339(),
        };
        let mut ledger = self.ledger.lock().map_err(|_| "unavailable")?;
        if let Some(existing) = ledger.get(command_id) {
            if existing.manager_id == manager_id.as_str() {
                return Ok(existing.clone());
            }
            return Err("command_conflict");
        }
        if ledger.len() < MAX_COMMAND_LEDGER {
            ledger.insert(command_id.to_owned(), receipt.clone());
        }
        Ok(receipt)
    }
}

pub fn valid_command_id(command_id: &str) -> bool {
    !command_id.is_empty()
        && command_id.len() <= 128
        && command_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(&c))
}

fn valid_pe_executable(path: &Path) -> std::io::Result<bool> {
    let mut file = std::fs::File::open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() < 64 {
        return Ok(false);
    }
    let mut header = [0u8; 2];
    file.read_exact(&mut header)?;
    Ok(header == *b"MZ")
}

pub fn resolve_executable_path(
    roots: &EnvironmentRoots,
    manager_id: ManagerAppId,
) -> Option<PathBuf> {
    inspect(roots)
        .apps
        .into_iter()
        .find(|app| app.component == manager_id.as_str() && app.presence == "found")
        .and_then(|app| app.path)
        .map(PathBuf::from)
}

pub fn inspect(roots: &EnvironmentRoots) -> ManagerApps {
    let engine = EnvironmentEngine::new(
        std::sync::Arc::new(vua_orchestrator::StdProcessRunner),
        std::sync::Arc::new(SystemClock),
        roots.clone(),
        std::sync::Arc::new(crate::VccSettingsFileReader),
    );
    let hub = engine
        .inspect_deployment_components()
        .into_iter()
        .find(|f| f.id == "unity_hub");
    let hub_path = hub
        .as_ref()
        .and_then(|f| f.facts["exe"].as_str())
        .map(PathBuf::from);
    let hub_presence = match hub.as_ref().map(|f| f.presence) {
        Some(EnvironmentPresence::Detected) if hub_path.as_ref().is_some_and(|p| p.is_file()) => {
            "found"
        }
        Some(EnvironmentPresence::NotDetected) => "not_found",
        _ => "unknown",
    };
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let programs = std::env::var_os("ProgramFiles").map(PathBuf::from);
    let mut apps = vec![ManagerApp {
        component: "unity_hub",
        presence: hub_presence,
        path: hub_path.map(|p| p.to_string_lossy().into_owned()),
    }];
    for (component, product, binaries) in [
        (
            "vcc",
            "VRChat Creator Companion",
            &["CreatorCompanion.exe"][..],
        ),
        ("alcom", "ALCOM", &["ALCOM.exe", "vrc-get-gui.exe"][..]),
    ] {
        let mut candidates = Vec::new();
        for root in local
            .iter()
            .map(|p| p.join("Programs"))
            .chain(programs.iter().cloned())
        {
            for binary in binaries {
                candidates.push(root.join(product).join(binary));
            }
        }
        for binary in binaries {
            for hive in [RegistryHive::CurrentUser, RegistryHive::LocalMachine] {
                let key = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{binary}");
                if let Some(path) = roots.registry.get_string(hive, &key, "") {
                    candidates.push(PathBuf::from(path.trim_matches('"')));
                }
                for product_key in [product.to_owned(), format!("{product}_is1")] {
                    for prefix in [
                        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
                        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
                    ] {
                        let key = format!(r"{prefix}\{product_key}");
                        if let Some(root) = roots.registry.get_string(hive, &key, "InstallLocation")
                        {
                            candidates.push(PathBuf::from(root.trim_matches('"')).join(binary));
                        }
                    }
                }
            }
        }
        apps.push(discover(component, &candidates));
    }
    ManagerApps {
        schema_version: "vua.manager-apps/v0.1",
        captured_at: SystemClock.now_rfc3339(),
        apps,
    }
}

fn discover(component: &'static str, candidates: &[PathBuf]) -> ManagerApp {
    let mut unreadable = false;
    let path = candidates.iter().find_map(|p| match std::fs::metadata(p) {
        Ok(metadata) if metadata.is_file() => Some(p.to_string_lossy().into_owned()),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            unreadable = true;
            None
        }
        _ => None,
    });
    ManagerApp {
        component,
        presence: if path.is_some() {
            "found"
        } else if unreadable {
            "unknown"
        } else {
            "not_found"
        },
        path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vua_orchestrator::FakeRegistrySource;

    struct RecordingSpawner {
        calls: Mutex<Vec<PathBuf>>,
    }

    impl ManagerProcessSpawner for RecordingSpawner {
        fn spawn_detached(&self, executable: &Path) -> Result<(), &'static str> {
            self.calls.lock().unwrap().push(executable.to_path_buf());
            Ok(())
        }
    }

    struct FixedClock;
    impl Clock for FixedClock {
        fn now_rfc3339(&self) -> String {
            "2026-11-14T03:00:00Z".to_owned()
        }
    }

    fn fake_roots(root: &Path) -> EnvironmentRoots {
        EnvironmentRoots {
            local_low: root.to_path_buf(),
            registry: Arc::new(FakeRegistrySource::new()),
            ..EnvironmentRoots::default()
        }
    }

    #[test]
    fn configuration_and_empty_folders_do_not_count_as_apps() {
        let root = std::env::temp_dir().join(format!("vua-manager-app-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let exe = root.join("ALCOM.exe");
        std::fs::write(root.join("settings.json"), "{}").unwrap();
        assert_eq!(
            discover("alcom", std::slice::from_ref(&exe)).presence,
            "not_found"
        );
        std::fs::write(&exe, "fixture").unwrap();
        assert_eq!(discover("alcom", &[exe]).presence, "found");
        std::fs::remove_file(root.join("ALCOM.exe")).unwrap();
        std::fs::remove_file(root.join("settings.json")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn open_validates_pe_header_and_enforces_command_idempotency() {
        let root = std::env::temp_dir().join(format!("vua-manager-launch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let bad_exe = root.join("not-pe.exe");
        std::fs::write(&bad_exe, b"not an executable").unwrap();

        let good_exe = root.join("ALCOM.exe");
        let mut pe_bytes = vec![0u8; 128];
        pe_bytes[0] = b'M';
        pe_bytes[1] = b'Z';
        std::fs::write(&good_exe, &pe_bytes).unwrap();

        let spawner = Arc::new(RecordingSpawner {
            calls: Mutex::new(Vec::new()),
        });
        let bad_clone = bad_exe.clone();
        let good_clone = good_exe.clone();
        let launcher = ManagerAppLauncher::with_dependencies(
            fake_roots(&root),
            Arc::new(move |_, id| match id {
                ManagerAppId::Alcom => Some(good_clone.clone()),
                ManagerAppId::Vcc => Some(bad_clone.clone()),
                ManagerAppId::UnityHub => None,
            }),
            spawner.clone(),
            Arc::new(FixedClock),
        );

        // Missing or non-PE executables are rejected before spawning.
        assert_eq!(
            launcher.open(ManagerAppId::UnityHub, "cmd-missing"),
            Err("not_found")
        );
        assert_eq!(
            launcher.open(ManagerAppId::Vcc, "cmd-bad-pe"),
            Err("not_found")
        );
        assert!(spawner.calls.lock().unwrap().is_empty());

        // Valid PE spawns once and replays idempotently on duplicate commandId.
        let receipt = launcher.open(ManagerAppId::Alcom, "cmd-alcom-1").unwrap();
        assert_eq!(receipt.schema_version, "vua.manager-apps/v0.2");
        assert_eq!(receipt.manager_id, "alcom");
        assert_eq!(spawner.calls.lock().unwrap().len(), 1);

        let replay = launcher.open(ManagerAppId::Alcom, "cmd-alcom-1").unwrap();
        assert_eq!(replay, receipt);
        assert_eq!(spawner.calls.lock().unwrap().len(), 1);

        // Reusing commandId for another manager fails with command_conflict.
        assert_eq!(
            launcher.open(ManagerAppId::Vcc, "cmd-alcom-1"),
            Err("command_conflict")
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
