//! Hand-picked Unity editor path verification — the environment half of the
//! U10 ruling (`docs/decisions/path-configuration`), ADR gate ① and ②.
//!
//! Gate ① (identity): the version string is read from the editor
//! executable's own version resource — **never from the directory name**.
//! The Hub-layout root name stays informational only; a directory named
//! `2022.3.22f1` that does not contain an editor with a matching identity
//! must refuse, not pass.
//!
//! Gate ② (version): the parsed version is classified by the core
//! `editor_targets` classifier — the single classification authority per
//! `docs/compatibility/unity-editor`. Only the exact global production
//! target classifies as `production_target`; every other verdict is a
//! compatibility-policy diagnosis (migration source / other version /
//! Tuanjie family) and never silently used as the production editor —
//! consumers render the guidance code, they do not promote off-target
//! verdicts.
//!
//! ADR gate ③ (trust presentation, first-use confirmation, decision
//! record) is a settings-surface concern (desktop) plus persistence
//! (ownership per proposal 021) and deliberately not modeled here.
//!
//! The module is a detection-domain fact primitive: it verifies one
//! user-picked path and returns what the machine observed. It carries no
//! wire face; the transport question is left to the proposal 021 seam
//! discussion (core route, desktop settings surface).

use std::path::{Path, PathBuf};
use vua_orchestrator::{classify_editor, parse_editor_version, EditorClass};

/// Stable refusal/diagnosis codes; consumers key on them and the set is
/// closed — extend only with a new explicit code, never by overloading.
pub mod codes {
    /// The picked path does not exist.
    pub const TARGET_MISSING: &str = "vua.editor_verify.target_missing";
    /// The path exists but no `Unity.exe` was found in the accepted layouts.
    pub const EXE_MISSING: &str = "vua.editor_verify.exe_missing";
    /// The executable exists but its version resource is missing/unreadable.
    pub const IDENTITY_UNREADABLE: &str = "vua.editor_verify.identity_unreadable";
    /// A version resource was read but no string in it parses as a complete
    /// Unity editor version — identity not established.
    pub const NOT_AN_EDITOR: &str = "vua.editor_verify.not_an_editor";
    /// Identity reading is not implemented on this platform.
    pub const UNSUPPORTED_PLATFORM: &str = "vua.editor_verify.unsupported_platform";
}

/// What the verification observed: an identified editor, or a refusal with
/// a stable code. A refusal is a normal finding (ADR: 失败即空态加引导),
/// never an error to hide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorPathVerdict {
    Verified(EditorPathIdentity),
    Refused(EditorPathRefusal),
}

/// Identity established by reading the executable itself (gate ①), then
/// classified per the support matrix (gate ②).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorPathIdentity {
    /// Hub-layout versioned root (`<root>/Editor/Unity.exe`); when the
    /// picked path does not follow that layout this is the executable's
    /// immediate directory. Informational only — identity comes from the
    /// version string, not this name.
    pub editor_root: String,
    /// The executable the identity read targeted.
    pub exe_path: String,
    /// Complete version string as parsed (`display` form).
    pub version: String,
    pub classification: EditorClass,
    /// Compatibility-policy guidance code from the core classifier
    /// (`vua.env_managers.editor_*`) — render, never promote off-target.
    pub guidance_code: &'static str,
    /// Unity China distribution build (`c<n>` suffix).
    pub china_distribution: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorPathRefusal {
    /// The executable the read targeted, when one was located.
    pub exe_path: Option<String>,
    /// Stable code from [`codes`].
    pub code: &'static str,
    /// Human-readable diagnosis; carries the raw resource strings when a
    /// resource was read but no version parsed (diagnosis without
    /// invention — the raw bytes are quoted, not interpreted).
    pub detail: String,
}

/// Source of the identity strings for one executable. Injectable so the
/// verdict logic is testable without a real Windows version resource; the
/// system implementation reads the PE version resource.
pub trait EditorIdentitySource {
    /// Returns the candidate identity strings (e.g. `FileVersion`,
    /// `ProductVersion`), in resource order. `Err` = the resource could not
    /// be read at all ([`codes::IDENTITY_UNREADABLE`]).
    fn read_identity_strings(&self, exe: &Path) -> Result<Vec<String>, String>;
}

/// Locates the editor executable for a picked path, accepting the three
/// layouts a folder picker produces:
///
/// 1. the executable itself (`<root>/Editor/Unity.exe` or any picked file);
/// 2. the versioned root (`<root>` containing `Editor/Unity.exe`);
/// 3. the `Editor` directory (`<root>/Editor` containing `Unity.exe`).
///
/// Returns `(editor_root, exe_path)`. The root name is informational; the
/// caller must still verify identity from the executable (gate ①).
fn locate_editor_executable(input: &Path) -> Result<(PathBuf, PathBuf), (&'static str, String)> {
    if input.is_file() {
        let exe = input.to_path_buf();
        let editor_dir = exe.parent().unwrap_or(input);
        let root = match editor_dir.file_name() {
            Some(name) if name == "Editor" => editor_dir.parent().unwrap_or(input),
            _ => editor_dir,
        };
        return Ok((root.to_path_buf(), exe));
    }
    if input.is_dir() {
        let in_editor = input.join("Editor").join("Unity.exe");
        if in_editor.is_file() {
            return Ok((input.to_path_buf(), in_editor));
        }
        let direct = input.join("Unity.exe");
        if direct.is_file() {
            let root = match input.file_name() {
                Some(name) if name == "Editor" => input.parent().unwrap_or(input),
                _ => input,
            };
            return Ok((root.to_path_buf(), direct));
        }
        return Err((
            codes::EXE_MISSING,
            format!(
                "no Unity.exe under {} (tried Editor/Unity.exe, Unity.exe)",
                input.display()
            ),
        ));
    }
    Err((
        codes::TARGET_MISSING,
        format!("{} does not exist", input.display()),
    ))
}

/// Version-string candidates from one raw identity string: the full
/// trimmed string, then the segment before the first whitespace,
/// underscore, or dash. Unity build metadata sometimes trails the version
/// (`2022.3.22f1_<hash>`); the candidates are tried in order and the first
/// complete parse wins. All raw strings stay in the refusal detail when
/// nothing parses.
fn version_candidates(raw: &str) -> Vec<&str> {
    let trimmed = raw.trim();
    let mut candidates = vec![trimmed];
    if let Some(cut) = trimmed.find([' ', '_', '-']) {
        candidates.push(&trimmed[..cut]);
    }
    candidates
}

/// Verifies one user-picked path against ADR gates ① and ② with the given
/// identity source. Deterministic for a given tree and executable.
pub fn verify_editor_path(input: &Path, reader: &dyn EditorIdentitySource) -> EditorPathVerdict {
    let (root, exe) = match locate_editor_executable(input) {
        Ok(found) => found,
        Err((code, detail)) => {
            return EditorPathVerdict::Refused(EditorPathRefusal {
                exe_path: None,
                code,
                detail,
            });
        }
    };
    let raw_strings = match reader.read_identity_strings(&exe) {
        Ok(strings) => strings,
        Err(error) => {
            return EditorPathVerdict::Refused(EditorPathRefusal {
                exe_path: Some(exe.to_string_lossy().into_owned()),
                code: codes::IDENTITY_UNREADABLE,
                detail: error,
            });
        }
    };
    let parsed = raw_strings
        .iter()
        .filter_map(|raw| {
            version_candidates(raw)
                .into_iter()
                .find_map(parse_editor_version)
        })
        .next();
    match parsed {
        Some(identity) => {
            let (classification, guidance_code) = classify_editor(&identity);
            EditorPathVerdict::Verified(EditorPathIdentity {
                editor_root: root.to_string_lossy().into_owned(),
                exe_path: exe.to_string_lossy().into_owned(),
                version: identity.display,
                classification,
                guidance_code,
                china_distribution: identity.china_suffix.is_some(),
            })
        }
        None => EditorPathVerdict::Refused(EditorPathRefusal {
            exe_path: Some(exe.to_string_lossy().into_owned()),
            code: codes::NOT_AN_EDITOR,
            detail: format!(
                "version resource readable but no complete Unity editor version in: {}",
                raw_strings.join(" | ")
            ),
        }),
    }
}

/// Convenience wiring the platform identity source. Non-Windows platforms
/// refuse with [`codes::UNSUPPORTED_PLATFORM`] — the product is
/// Windows-first and the primitive does not pretend to verify what it
/// cannot read.
#[cfg(windows)]
pub fn verify_editor_path_system(input: &Path) -> EditorPathVerdict {
    verify_editor_path(input, &WindowsVersionResourceSource)
}

#[cfg(not(windows))]
pub fn verify_editor_path_system(input: &Path) -> EditorPathVerdict {
    let _ = input;
    EditorPathVerdict::Refused(EditorPathRefusal {
        exe_path: None,
        code: codes::UNSUPPORTED_PLATFORM,
        detail: "editor identity reading is implemented for Windows only".to_string(),
    })
}

/// Windows system source: reads `FileVersion` / `ProductVersion` from the
/// executable's version resource via the version.dll APIs. The language
/// codepage pairs come from the resource's own `VarFileInfo\Translation`
/// table; the neutral `040904b0` form is the fallback when the table is
/// absent, matching what resource tooling commonly emits.
#[cfg(windows)]
pub struct WindowsVersionResourceSource;

#[cfg(windows)]
impl EditorIdentitySource for WindowsVersionResourceSource {
    fn read_identity_strings(&self, exe: &Path) -> Result<Vec<String>, String> {
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
        };

        fn to_wide(value: &str) -> Vec<u16> {
            value.encode_utf16().chain(std::iter::once(0)).collect()
        }

        fn read_string_file_info(
            block: *const std::ffi::c_void,
            language: u16,
            codepage: u16,
            field: &str,
        ) -> Option<String> {
            let path = format!(
                "\\StringFileInfo\\{:04x}{:04x}\\{}",
                language, codepage, field
            );
            let mut buffer: *mut std::ffi::c_void = std::ptr::null_mut();
            let mut length = 0u32;
            unsafe {
                if VerQueryValueW(block, to_wide(&path).as_ptr(), &mut buffer, &mut length) != 0
                    && !buffer.is_null()
                    && length > 0
                {
                    let wide = std::slice::from_raw_parts(buffer as *const u16, length as usize);
                    let text = String::from_utf16_lossy(wide);
                    return Some(text.trim_end_matches('\0').trim().to_string());
                }
            }
            None
        }

        let wide_path = to_wide(&exe.to_string_lossy());
        let mut handle = 0u32;
        let size = unsafe { GetFileVersionInfoSizeW(wide_path.as_ptr(), &mut handle) };
        if size == 0 {
            return Err(format!(
                "no version resource on {} (GetLastError via size=0)",
                exe.display()
            ));
        }
        let mut data = vec![0u8; size as usize];
        let read_ok = unsafe {
            GetFileVersionInfoW(
                wide_path.as_ptr(),
                handle,
                size,
                data.as_mut_ptr() as *mut std::ffi::c_void,
            )
        };
        if read_ok == 0 {
            return Err(format!("GetFileVersionInfoW failed for {}", exe.display()));
        }
        let block = data.as_ptr() as *const std::ffi::c_void;

        // Translation table first: exact per-language strings.
        let mut pairs: Vec<(u16, u16)> = Vec::new();
        let mut buffer: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut length = 0u32;
        unsafe {
            if VerQueryValueW(
                block,
                to_wide("\\VarFileInfo\\Translation").as_ptr(),
                &mut buffer,
                &mut length,
            ) != 0
                && !buffer.is_null()
                && length >= 4
            {
                let words = std::slice::from_raw_parts(buffer as *const u16, (length / 2) as usize);
                for pair in words.chunks(2) {
                    pairs.push((pair[0], pair[1]));
                }
            }
        }
        if pairs.is_empty() {
            // Neutral resource form used by common tooling.
            pairs.push((0x0409, 0x04b0));
        }

        let mut strings = Vec::new();
        for field in ["ProductVersion", "FileVersion"] {
            for (language, codepage) in &pairs {
                if let Some(value) = read_string_file_info(block, *language, *codepage, field) {
                    if !value.is_empty() && !strings.contains(&value) {
                        strings.push(value);
                    }
                }
            }
        }
        if strings.is_empty() {
            return Err(format!(
                "version resource present but no ProductVersion/FileVersion string on {}",
                exe.display()
            ));
        }
        Ok(strings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct FixedSource(Result<Vec<String>, String>);

    impl EditorIdentitySource for FixedSource {
        fn read_identity_strings(&self, _exe: &Path) -> Result<Vec<String>, String> {
            self.0.clone()
        }
    }

    fn unique_temp_root(tag: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let directory = std::env::temp_dir().join(format!(
            "vua-editor-verify-{}-{}-{}",
            tag,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create temp root");
        directory
    }

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, b"placeholder").expect("write placeholder");
    }

    fn verify_with(source: FixedSource, input: &Path) -> EditorPathVerdict {
        verify_editor_path(input, &source)
    }

    #[test]
    fn verifies_production_target_from_editor_root_layout() {
        let root = unique_temp_root("prod-root");
        touch(&root.join("Editor").join("Unity.exe"));
        let verdict = verify_with(FixedSource(Ok(vec!["2022.3.22f1".to_string()])), &root);
        match verdict {
            EditorPathVerdict::Verified(identity) => {
                assert_eq!(identity.version, "2022.3.22f1");
                assert_eq!(identity.classification, EditorClass::ProductionTarget);
                assert_eq!(
                    identity.guidance_code,
                    vua_orchestrator::editor_target_codes::EDITOR_PRODUCTION_TARGET
                );
                assert!(!identity.china_distribution);
                assert!(identity
                    .editor_root
                    .ends_with(&root.file_name().unwrap().to_string_lossy().to_string()));
                assert!(identity.exe_path.ends_with("Editor\\Unity.exe"));
            }
            other => panic!("expected verified, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn verifies_when_picked_path_is_the_editor_directory() {
        let root = unique_temp_root("editor-dir");
        let editor_directory = root.join("2022.3.6f1").join("Editor");
        touch(&editor_directory.join("Unity.exe"));
        let verdict = verify_with(
            FixedSource(Ok(vec!["2022.3.6f1".to_string()])),
            &editor_directory,
        );
        match verdict {
            EditorPathVerdict::Verified(identity) => {
                assert_eq!(identity.classification, EditorClass::MigrationSource);
                // Root is the versioned directory above `Editor`.
                assert!(identity.editor_root.ends_with("2022.3.6f1"));
            }
            other => panic!("expected verified, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn verifies_when_picked_path_is_the_executable() {
        let root = unique_temp_root("exe-input");
        let exe = root.join("Editor").join("Unity.exe");
        touch(&exe);
        let verdict = verify_with(FixedSource(Ok(vec!["2019.4.31f1".to_string()])), &exe);
        match verdict {
            EditorPathVerdict::Verified(identity) => {
                assert_eq!(identity.classification, EditorClass::MigrationSource);
            }
            other => panic!("expected verified, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn china_distribution_classifies_other_with_china_diagnosis() {
        let root = unique_temp_root("china");
        touch(&root.join("Editor").join("Unity.exe"));
        let verdict = verify_with(FixedSource(Ok(vec!["2022.3.22f1c1".to_string()])), &root);
        match verdict {
            EditorPathVerdict::Verified(identity) => {
                assert_eq!(identity.classification, EditorClass::OtherUnityVersion);
                assert!(identity.china_distribution);
                assert_eq!(
                    identity.guidance_code,
                    vua_orchestrator::editor_target_codes::EDITOR_CHINA_DISTRIBUTION
                );
            }
            other => panic!("expected verified, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn build_metadata_suffix_falls_back_to_version_segment() {
        let root = unique_temp_root("hash");
        touch(&root.join("Editor").join("Unity.exe"));
        let verdict = verify_with(
            FixedSource(Ok(vec![
                "2022.3.22f1_a1b2c3".to_string(),
                "2022.3.22f1".to_string(),
            ])),
            &root,
        );
        match verdict {
            EditorPathVerdict::Verified(identity) => {
                assert_eq!(identity.classification, EditorClass::ProductionTarget);
            }
            other => panic!("expected verified, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn directory_name_is_never_trusted_without_identity() {
        // The root directory claims the production target in its name, but
        // the executable inside is not an editor (gate ①: read the version
        // string, never the path name).
        let root = unique_temp_root("fake-name");
        touch(&root.join("2022.3.22f1").join("Editor").join("Unity.exe"));
        let verdict = verify_with(
            FixedSource(Ok(vec!["7.7.7x9".to_string()])),
            &root.join("2022.3.22f1"),
        );
        match verdict {
            EditorPathVerdict::Refused(refusal) => {
                assert_eq!(refusal.code, codes::NOT_AN_EDITOR);
                assert!(refusal.detail.contains("7.7.7x9"));
            }
            other => panic!("expected refusal, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unreadable_resource_refuses_identity_unreadable() {
        let root = unique_temp_root("unreadable");
        touch(&root.join("Editor").join("Unity.exe"));
        let verdict = verify_with(FixedSource(Err("resource missing".to_string())), &root);
        match verdict {
            EditorPathVerdict::Refused(refusal) => {
                assert_eq!(refusal.code, codes::IDENTITY_UNREADABLE);
                assert!(refusal.exe_path.is_some());
            }
            other => panic!("expected refusal, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_targets_refuse_with_closed_codes() {
        let root = unique_temp_root("missing");
        // Nonexistent path.
        let verdict = verify_with(FixedSource(Ok(Vec::new())), &root.join("nowhere"));
        match &verdict {
            EditorPathVerdict::Refused(refusal) => {
                assert_eq!(refusal.code, codes::TARGET_MISSING);
            }
            other => panic!("expected refusal, got {other:?}"),
        }
        // Directory without an editor layout.
        let verdict = verify_with(FixedSource(Ok(Vec::new())), &root);
        match &verdict {
            EditorPathVerdict::Refused(refusal) => {
                assert_eq!(refusal.code, codes::EXE_MISSING);
            }
            other => panic!("expected refusal, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn tuanjie_family_identified_for_diagnostics() {
        let root = unique_temp_root("tuanjie");
        touch(&root.join("Editor").join("Unity.exe"));
        let verdict = verify_with(FixedSource(Ok(vec!["2022.3.22t1".to_string()])), &root);
        match verdict {
            EditorPathVerdict::Verified(identity) => {
                assert_eq!(identity.classification, EditorClass::TuanjieFamily);
            }
            other => panic!("expected verified, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    /// Real-machine probe: cross-checks every Hub-layout editor under the
    /// default editors root — the identity read from each executable must
    /// parse to the same version its directory name claims. `#[ignore]`d
    /// so default `cargo test` never touches real installed software;
    /// run explicitly on a machine with editors installed
    /// (`cargo test -p vua-project-manager -- --ignored editor_verify`).
    #[test]
    #[ignore]
    fn editor_verify_real_hub_editors_identity_matches_directory_name() {
        use vua_orchestrator::EnvironmentRoots;

        let editors_root = EnvironmentRoots::default().unity_editors_root;
        let entries = match std::fs::read_dir(&editors_root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                eprintln!(
                    "no Unity Hub editors root at {} — nothing to probe",
                    editors_root.display()
                );
                return;
            }
            Err(error) => panic!("editors root unreadable: {error}"),
        };
        let mut probed = 0;
        for entry in entries.flatten() {
            let root = entry.path();
            if !root.join("Editor").join("Unity.exe").is_file() {
                continue;
            }
            probed += 1;
            let claimed = root.file_name().unwrap().to_string_lossy().into_owned();
            let verdict = verify_editor_path_system(&root);
            match verdict {
                EditorPathVerdict::Verified(identity) => {
                    assert_eq!(
                        identity.version, claimed,
                        "identity read from {root:?} contradicts directory name"
                    );
                }
                EditorPathVerdict::Refused(refusal) => panic!(
                    "real editor at {root:?} refused ({}: {})",
                    refusal.code, refusal.detail
                ),
            }
        }
        eprintln!(
            "probed {probed} real editor(s) under {}",
            editors_root.display()
        );
    }
}
