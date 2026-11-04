//! Coordinates opaque, AMF-owned backups. Distribution never opens user databases.
use crate::{
    layout::{Active, Installation},
    paths::{digest, owned_path},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const PROFILE: &str = "updates-v02/data-profile.json";
const INTENT: &str = "updates-v02/data-intent.json";
const ABI: &str = "amf-data/0.1";
fn identity(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Profile {
    schema_version: String,
    base: Active,
    profile: PathBuf,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Intent {
    schema_version: String,
    before: Active,
    next: Active,
    profile: PathBuf,
    token: String,
    binding: String,
    receipt: Option<String>,
    state: String,
}

fn safe_profile(profile: &Path) -> Result<()> {
    if !profile.is_absolute()
        || !profile.is_dir()
        || profile
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(Error("invalid_data_profile".into()));
    }
    for ancestor in profile.ancestors() {
        let metadata = fs::symlink_metadata(ancestor)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(Error("data_path_is_link".into()));
            }
        }
        if metadata.file_type().is_symlink() {
            return Err(Error("data_path_is_link".into()));
        }
    }
    Ok(())
}
fn closed_command(mut command: Command) -> Result<Value> {
    for (name, _) in std::env::vars_os() {
        if name
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("VUA_")
        {
            command.env_remove(name);
        }
    }
    command.stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn()?;
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if started.elapsed() > Duration::from_secs(130) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error("data_backup_timeout".into()));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let result = child.wait_with_output()?;
    if !result.status.success() || result.stdout.len() > 65536 {
        return Err(Error("amf_data_recovery_failed".into()));
    }
    serde_json::from_slice(&result.stdout).map_err(|_| Error("amf_data_recovery_failed".into()))
}
impl Installation {
    pub fn register_data_profile(&self, profile: &Path) -> Result<()> {
        safe_profile(profile)?;
        let _operation = self.lock("operation.lock")?;
        self.save_update(
            PROFILE,
            &Profile {
                schema_version: "0.1".into(),
                base: self.active()?,
                profile: profile.into(),
            },
        )
    }
    fn data_profile(&self, before: &Active) -> Result<Option<PathBuf>> {
        if !owned_path(&self.root, PROFILE)?.exists() {
            return Ok(None);
        }
        let value: Profile = self.update_json(PROFILE)?;
        if value.schema_version != "0.1" {
            return Err(Error("invalid_data_profile".into()));
        }
        if &value.base != before {
            return Ok(None);
        }
        safe_profile(&value.profile)?;
        Ok(Some(value.profile))
    }
    fn recovery_executable(&self, next: &Active) -> Result<PathBuf> {
        let target = self.manifest(&next.amf_manifest)?;
        let root = self.amf_root(&target)?;
        let file = target
            .amf
            .files
            .iter()
            .find(|file| file.path == "provider/vua-amf-provider.exe")
            .ok_or_else(|| Error("amf_data_recovery_unavailable".into()))?;
        let executable = owned_path(&root, &file.path)?;
        if !crate::transfer::verified_file(&executable, file.bytes, &file.sha256) {
            return Err(Error("installed_files_need_repair".into()));
        }
        Ok(executable)
    }
    fn data_capability(&self, next: &Active) -> Result<bool> {
        let target = self.manifest(&next.amf_manifest)?;
        let Some(file) = target
            .amf
            .files
            .iter()
            .find(|file| file.path == "ui/component.json")
        else {
            return Ok(false);
        };
        if file.bytes > 65536 {
            return Err(Error("invalid_amf_data_identity".into()));
        }
        let path = owned_path(&self.amf_root(&target)?, &file.path)?;
        if !crate::transfer::verified_file(&path, file.bytes, &file.sha256) {
            return Err(Error("installed_files_need_repair".into()));
        }
        let metadata: Value = serde_json::from_slice(&fs::read(path)?)?;
        if metadata["dataRecoveryAbi"] != ABI {
            return Ok(false);
        }
        if metadata["schemaVersion"] != "0.1"
            || metadata["component"] != "amf"
            || metadata["version"] != target.amf.version
            || metadata["build"] != target.amf.build
            || metadata["bdlWrite"] != target.compatibility.bdl_write
        {
            return Err(Error("invalid_amf_data_identity".into()));
        }
        let mut command = Command::new(self.recovery_executable(next)?);
        command.arg("--component-identity");
        let identity = closed_command(command)?;
        if identity["dataRecoveryAbi"] != ABI
            || identity["version"] != target.amf.version
            || identity["build"] != target.amf.build
            || identity["bdlWrite"] != target.compatibility.bdl_write
        {
            return Err(Error("invalid_amf_data_identity".into()));
        }
        Ok(true)
    }
    pub(crate) fn admit_data_recovery(&self, before: &Active, next: &Active) -> Result<()> {
        let (_, old) = self.validate_pair(before)?;
        let (_, target) = self.validate_pair(next)?;
        if before.amf_manifest != next.amf_manifest
            && self.data_profile(before)?.is_some()
            && self.data_capability(next)?
        {
            self.require_data_host(next)?;
            self.require_data_launcher()?;
        }
        if old.compatibility.bdl_write != target.compatibility.bdl_write {
            if self.data_profile(before)?.is_none() || !self.data_capability(next)? {
                return Err(Error("data_migration_requires_installer".into()));
            }
            self.require_data_launcher()?;
        }
        Ok(())
    }
    fn require_data_host(&self, next: &Active) -> Result<()> {
        let host = self.manifest(&next.host_manifest)?;
        let file = host
            .host
            .files
            .iter()
            .find(|file| file.path == "dist/renderer/update-runtime.json")
            .ok_or_else(|| Error("installer_required".into()))?;
        let path = owned_path(&self.host_root(&host)?, &file.path)?;
        if file.bytes > 65536 || !crate::transfer::verified_file(&path, file.bytes, &file.sha256) {
            return Err(Error("installed_files_need_repair".into()));
        }
        let value: Value = serde_json::from_slice(&fs::read(path)?)?;
        if value["schemaVersion"] != "0.1"
            || value["dataTrialAbi"] != ABI
            || value["build"] != host.host.build
        {
            return Err(Error("installer_required".into()));
        }
        Ok(())
    }
    pub(crate) fn prepare_data_recovery(&self, before: &Active, next: &Active) -> Result<()> {
        if before.amf_manifest == next.amf_manifest {
            return Ok(());
        }
        let Some(profile) = self.data_profile(before)? else {
            return self.admit_data_recovery(before, next);
        };
        if !self.data_capability(next)? {
            return self.admit_data_recovery(before, next);
        }
        // The root must recover this sidecar after an interrupted selection, even for same-writer changes.
        self.require_data_launcher()?;
        let binding = digest(&serde_json::to_vec(&(before, next))?);
        let token = digest(
            format!(
                "{binding}:{}:{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| Error("invalid_system_time".into()))?
                    .as_nanos()
            )
            .as_bytes(),
        );
        let mut intent = Intent {
            schema_version: "0.1".into(),
            before: before.clone(),
            next: next.clone(),
            profile,
            token,
            binding,
            state: "snapshotting".into(),
            receipt: None,
        };
        self.save_update(INTENT, &intent)?;
        intent.receipt = Some(self.data_operation(&intent, "snapshot", "prepared")?);
        intent.state = "prepared".into();
        self.save_update(INTENT, &intent)?;
        self.data_operation(&intent, "migrate", "prepared")
            .map(|_| ())
    }
    fn data_intent(&self) -> Result<Option<Intent>> {
        if !owned_path(&self.root, INTENT)?.exists() {
            return Ok(None);
        }
        let value: Intent = self.update_json(INTENT)?;
        if ["prepared", "restored", "committed"].contains(&value.state.as_str())
            && value
                .receipt
                .as_deref()
                .is_none_or(|receipt| !identity(receipt))
        {
            return Err(Error("invalid_data_intent".into()));
        }
        if value.schema_version != "0.1"
            || ![
                "snapshotting",
                "prepared",
                "restored",
                "committed",
                "aborted",
            ]
            .contains(&value.state.as_str())
            || value.binding != digest(&serde_json::to_vec(&(&value.before, &value.next))?)
            || !identity(&value.token)
        {
            return Err(Error("invalid_data_intent".into()));
        }
        self.validate_pair(&value.before)?;
        self.validate_pair(&value.next)?;
        safe_profile(&value.profile)?;
        Ok(Some(value))
    }
    fn data_operation(&self, intent: &Intent, operation: &str, expected: &str) -> Result<String> {
        let mut command = Command::new(self.recovery_executable(&intent.next)?);
        command
            .arg(format!("--data-{operation}"))
            .arg("--profile")
            .arg(&intent.profile)
            .arg("--token")
            .arg(&intent.token)
            .arg("--binding")
            .arg(&intent.binding);
        if operation != "snapshot" {
            command.arg("--receipt").arg(
                intent
                    .receipt
                    .as_ref()
                    .ok_or_else(|| Error("invalid_data_intent".into()))?,
            );
        }
        let result = closed_command(command)?;
        if result["schemaVersion"] != "0.1"
            || result["dataRecoveryAbi"] != ABI
            || result["token"] != intent.token
            || result["binding"] != intent.binding
            || result["state"] != expected
        {
            return Err(Error("amf_data_recovery_failed".into()));
        }
        let receipt = result["receipt"]
            .as_str()
            .filter(|value| {
                value.len() == 64
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            .ok_or_else(|| Error("invalid_data_intent".into()))?;
        if intent
            .receipt
            .as_ref()
            .is_some_and(|expected| expected != receipt)
        {
            return Err(Error("data_backup_receipt_mismatch".into()));
        }
        Ok(receipt.into())
    }
    pub(crate) fn recover_data_recovery(&self, before: &Active, next: &Active) -> Result<bool> {
        let Some(mut intent) = self.data_intent()? else {
            return Ok(false);
        };
        if &intent.before != before || &intent.next != next {
            return Ok(false);
        }
        if intent.state == "snapshotting" {
            if &self.active()? != before {
                return Err(Error("invalid_data_intent".into()));
            }
            intent.state = "aborted".into();
            self.save_update(INTENT, &intent)?;
            return Ok(false);
        }
        if intent.state == "committed" || intent.state == "aborted" {
            return Ok(false);
        }
        self.data_operation(&intent, "restore", "restored")?;
        intent.state = "restored".into();
        self.save_update(INTENT, &intent)?;
        Ok(true)
    }
    pub(crate) fn commit_data_recovery(&self, before: &Active, next: &Active) -> Result<()> {
        let Some(mut intent) = self.data_intent()? else {
            return Ok(());
        };
        if &intent.before != before || &intent.next != next || intent.state == "committed" {
            return Ok(());
        }
        if intent.state != "prepared" {
            return Err(Error("invalid_data_intent".into()));
        }
        self.data_operation(&intent, "commit", "committed")?;
        intent.state = "committed".into();
        self.save_update(INTENT, &intent)
    }
    pub fn data_trial_status(&self, profile: &Path) -> Result<Value> {
        let Some(intent) = self.data_intent()? else {
            return Ok(json!({"pending":false}));
        };
        let Some(journal) = self.activation_journal()? else {
            return Err(Error("invalid_data_intent".into()));
        };
        let pending = intent.before == journal.before
            && intent.next == journal.next
            && journal.next == self.active()?
            && ["verifying", "healthy"].contains(&journal.state.as_str())
            && intent.state == "prepared";
        if pending && intent.profile != profile {
            return Err(Error("data_profile_mismatch".into()));
        }
        Ok(json!({"pending":pending}))
    }
}
