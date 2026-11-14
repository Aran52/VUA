//! Handle-locked executable verification and zero-argument UAC launch helper.
//!
//! Holding an open file handle with `FILE_SHARE_READ` only (`0x0000_0001`) prevents any
//! concurrent writer, deleter, or renamer from modifying or swapping the target file
//! between SHA-256 / Authenticode verification and process creation (eliminating TOCTOU).

use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const INTEGRITY_ERROR: &str = "vua.deployment.installer_integrity_failed";
const FILE_SHARE_READ_ONLY: u32 = 0x0000_0001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElevatedLaunchOutcome {
    Completed { exit_code: i32 },
    ElevationDeclined,
    LaunchFailed,
    TimedOut,
}

#[derive(Debug)]
pub struct LockedExecutable {
    file: File,
    path: PathBuf,
}

impl LockedExecutable {
    /// Opens `path` read-only with `FILE_SHARE_READ` only on Windows so concurrent writes,
    /// deletes, and renames fail with `ERROR_SHARING_VIOLATION` until this guard is dropped.
    pub fn open_read_locked(path: &Path) -> io::Result<Self> {
        let lossy = path.to_string_lossy();
        if !path.is_absolute() || lossy.contains('"') || lossy.chars().any(char::is_control) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "executable path must be an absolute path free of quotes and control characters",
            ));
        }

        #[cfg(windows)]
        let file = {
            use std::os::windows::fs::OpenOptionsExt;
            OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ_ONLY)
                .open(path)?
        };

        #[cfg(not(windows))]
        let file = OpenOptions::new().read(true).open(path)?;

        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "locked executable target is not a regular file",
            ));
        }

        Ok(Self {
            file,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file(&self) -> &File {
        &self.file
    }

    /// Checks that the locked file is at least 64 bytes and starts with DOS/PE `MZ`.
    pub fn has_pe_header(&mut self) -> bool {
        let Ok(meta) = self.file.metadata() else {
            return false;
        };
        if !meta.is_file() || meta.len() < 64 {
            return false;
        }
        if self.file.seek(SeekFrom::Start(0)).is_err() {
            return false;
        }
        let mut magic = [0u8; 2];
        let ok = self.file.read_exact(&mut magic).is_ok() && magic == *b"MZ";
        let _ = self.file.seek(SeekFrom::Start(0));
        ok
    }

    /// Streams the locked file from offset 0 to compute SHA-256 while enforcing size bounds,
    /// `MZ` PE header (>= 64 bytes), and optional exact size / SHA-256 pins.
    pub fn verify_sha256_and_size(
        &mut self,
        max_bytes: u64,
        expected_size: Option<u64>,
        expected_sha256: Option<&str>,
        mut on_progress: impl FnMut(u64, u64) -> Result<(), &'static str>,
    ) -> Result<String, &'static str> {
        let size = self.file.metadata().map_err(|_| INTEGRITY_ERROR)?.len();
        if size == 0 || size > max_bytes || expected_size.is_some_and(|exp| exp != size) {
            return Err(INTEGRITY_ERROR);
        }
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|_| INTEGRITY_ERROR)?;

        on_progress(0, size)?;

        let mut sha256 = Sha256::new();
        let mut buffer = [0u8; 256 * 1024];
        let mut total = 0u64;
        let mut first_two = [0u8; 2];
        let mut last_report = Instant::now();

        loop {
            let n = self.file.read(&mut buffer).map_err(|_| INTEGRITY_ERROR)?;
            if n == 0 {
                break;
            }
            if total == 0 && n >= 2 {
                first_two.copy_from_slice(&buffer[..2]);
            } else if total == 0 && n == 1 {
                first_two[0] = buffer[0];
            } else if total == 1 {
                first_two[1] = buffer[0];
            }
            total += n as u64;
            if total > max_bytes || expected_size.is_some_and(|exp| total > exp) {
                return Err(INTEGRITY_ERROR);
            }
            sha256.update(&buffer[..n]);
            if last_report.elapsed() >= Duration::from_secs(1) {
                on_progress(total, size)?;
                last_report = Instant::now();
            }
        }

        let digest: String = sha256
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();

        if total != size
            || size < 64
            || first_two != *b"MZ"
            || expected_sha256.is_some_and(|exp| exp != digest)
        {
            return Err(INTEGRITY_ERROR);
        }

        on_progress(total, size)?;
        Ok(digest)
    }

    /// Verifies Authenticode signature + leaf certificate CN directly against the open handle.
    pub fn verify_authenticode_leaf_cn(&self, accepted_cns: &[&str]) -> bool {
        crate::deployment_trust::signature_leaf_cn_matches_handle(
            &self.path,
            &self.file,
            accepted_cns,
        )
    }

    /// Launches the locked executable via Windows UAC (`runas`) with zero command-line
    /// arguments (`lpParameters = null`), keeping the read-share lock open until exit or timeout.
    pub fn run_elevated_no_args(&self, timeout: Duration) -> ElevatedLaunchOutcome {
        let _keep_handle_open = &self.file;
        run_elevated_no_args_os(&self.path, timeout)
    }
}

#[cfg(not(windows))]
fn run_elevated_no_args_os(_path: &Path, _timeout: Duration) -> ElevatedLaunchOutcome {
    ElevatedLaunchOutcome::LaunchFailed
}

#[cfg(windows)]
fn run_elevated_no_args_os(path: &Path, timeout: Duration) -> ElevatedLaunchOutcome {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{GetExitCodeProcess, WaitForSingleObject},
        UI::{
            Shell::{
                ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
            },
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    };

    if timeout.is_zero() || timeout > Duration::from_secs(3600) || !path.is_absolute() {
        return ElevatedLaunchOutcome::LaunchFailed;
    }

    let encode_wide =
        |s: &std::ffi::OsStr| -> Vec<u16> { s.encode_wide().chain(Some(0)).collect() };
    let verb = encode_wide(std::ffi::OsStr::new("runas"));
    let exe = encode_wide(path.as_os_str());
    let dir = path.parent().map(|p| encode_wide(p.as_os_str()));

    // SAFETY: all UTF-16 buffers outlive ShellExecuteExW; zeroed optional fields are null.
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    info.lpVerb = verb.as_ptr();
    info.lpFile = exe.as_ptr();
    info.lpParameters = ptr::null();
    info.lpDirectory = dir.as_ref().map_or(ptr::null(), |d| d.as_ptr());
    info.nShow = SW_SHOWNORMAL;

    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        return match io::Error::last_os_error().raw_os_error() {
            Some(1223) => ElevatedLaunchOutcome::ElevationDeclined,
            _ => ElevatedLaunchOutcome::LaunchFailed,
        };
    }

    struct ProcessHandleGuard(HANDLE);
    impl Drop for ProcessHandleGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    let handle = ProcessHandleGuard(info.hProcess);
    if handle.0.is_null() {
        return ElevatedLaunchOutcome::LaunchFailed;
    }

    let timeout_ms = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
    let waited = unsafe { WaitForSingleObject(handle.0, timeout_ms) };
    match waited {
        WAIT_TIMEOUT => ElevatedLaunchOutcome::TimedOut,
        WAIT_OBJECT_0 => {
            let mut exit_code = 0u32;
            if unsafe { GetExitCodeProcess(handle.0, &mut exit_code) } == 0 {
                ElevatedLaunchOutcome::LaunchFailed
            } else {
                ElevatedLaunchOutcome::Completed {
                    exit_code: exit_code as i32,
                }
            }
        }
        _ => ElevatedLaunchOutcome::LaunchFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deployment_trust::EAC_TRUSTED_LEAF_SIGNERS;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vua-verified-launch-{label}-{}-{}",
            std::process::id(),
            TEST_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn open_read_locked_rejects_relative_missing_and_directory_targets() {
        assert!(LockedExecutable::open_read_locked(Path::new("relative.exe")).is_err());
        let dir = unique_temp_dir("invalid");
        assert!(LockedExecutable::open_read_locked(&dir).is_err());
        assert!(LockedExecutable::open_read_locked(&dir.join("missing.exe")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn verify_sha256_and_authenticode_refuse_unsigned_or_non_pe_files() {
        let dir = unique_temp_dir("pe-check");
        let non_pe = dir.join("not-pe.exe");
        std::fs::write(&non_pe, vec![b'X'; 128]).unwrap();
        let mut locked_non_pe = LockedExecutable::open_read_locked(&non_pe).unwrap();
        assert!(!locked_non_pe.has_pe_header());
        assert_eq!(
            locked_non_pe.verify_sha256_and_size(1024, Some(128), None, |_, _| Ok(())),
            Err(INTEGRITY_ERROR)
        );
        drop(locked_non_pe);

        let synthetic_pe = dir.join("synthetic.exe");
        let mut pe_bytes = vec![0u8; 128];
        pe_bytes[0] = b'M';
        pe_bytes[1] = b'Z';
        std::fs::write(&synthetic_pe, &pe_bytes).unwrap();

        let expected_digest: String = Sha256::digest(&pe_bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();

        let mut locked_pe = LockedExecutable::open_read_locked(&synthetic_pe).unwrap();
        assert!(locked_pe.has_pe_header());
        let actual = locked_pe
            .verify_sha256_and_size(1024, Some(128), Some(&expected_digest), |_, _| Ok(()))
            .unwrap();
        assert_eq!(actual, expected_digest);
        assert_eq!(
            locked_pe.verify_sha256_and_size(1024, Some(129), None, |_, _| Ok(())),
            Err(INTEGRITY_ERROR)
        );
        assert!(!locked_pe.verify_authenticode_leaf_cn(EAC_TRUSTED_LEAF_SIGNERS));
        drop(locked_pe);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_share_read_lock_blocks_write_rename_and_delete_while_allowing_execution() {
        let dir = unique_temp_dir("share-lock");
        let target = dir.join("locked-hostname.exe");
        let renamed = dir.join("swapped.exe");

        let system_hostname = Path::new(r"C:\Windows\System32\hostname.exe");
        if system_hostname.is_file() {
            std::fs::copy(system_hostname, &target).unwrap();
        } else {
            let mut pe = vec![0u8; 128];
            pe[0] = b'M';
            pe[1] = b'Z';
            std::fs::write(&target, pe).unwrap();
        }

        let locked = LockedExecutable::open_read_locked(&target).unwrap();

        // Concurrent readers are permitted.
        assert!(File::open(&target).is_ok());

        // Concurrent writers, renamers, and deleters are blocked by the kernel share mode.
        let write_err = OpenOptions::new().write(true).open(&target).unwrap_err();
        assert_eq!(write_err.raw_os_error(), Some(32));
        assert!(std::fs::rename(&target, &renamed).is_err());
        assert!(std::fs::remove_file(&target).is_err());

        // Process creation succeeds while the read-only lock is held.
        if system_hostname.is_file() {
            let out = std::process::Command::new(locked.path()).output().unwrap();
            assert!(out.status.success());
        }

        // Once the guard drops, rename and removal succeed immediately.
        drop(locked);
        std::fs::rename(&target, &renamed).unwrap();
        std::fs::remove_file(&renamed).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
