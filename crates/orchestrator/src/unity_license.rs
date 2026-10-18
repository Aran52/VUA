//! Metadata-only license hints and the separate result of an explicit Editor check.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseKind {
    ClassicEntitlement,
    MsixEntitlement,
    LegacyUlf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LicenseFile {
    pub kind: LicenseKind,
    /// Null means metadata could not be observed; false means the file was absent.
    pub present: Option<bool>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HubType {
    Classic,
    Msix,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseHint {
    LikelyActive,
    NeedsHubClient,
    NotFound,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseNextAction {
    None,
    StartHub,
    SignInAndActivate,
    RunCheck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseCheckState {
    Active,
    NotActivated,
    NeedsHubClient,
    Unknown,
}

/// The complete persisted record. It contains no paths, account fields or log text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckedLicense {
    pub state: LicenseCheckState,
    pub editor_version: String,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnityLicenseSnapshot {
    pub schema_version: String,
    pub files: [LicenseFile; 3],
    pub hub_type: HubType,
    pub hub_running: Option<bool>,
    pub licensing_client_running: Option<bool>,
    pub hub_cli_present: Option<bool>,
    pub last_check: Option<CheckedLicense>,
    pub state: LicenseHint,
    pub next_action: LicenseNextAction,
}

/// File sizes are used only to reject empty-file hints, and never leave the provider.
pub struct LicenseObservation {
    pub files: [LicenseFile; 3],
    pub nonempty: [bool; 3],
    pub complete: bool,
    pub hub_type: HubType,
    pub hub_running: Option<bool>,
    pub licensing_client_running: Option<bool>,
    pub hub_cli_present: Option<bool>,
}

pub fn snapshot(
    observed: LicenseObservation,
    last_check: Option<CheckedLicense>,
) -> UnityLicenseSnapshot {
    let any_file = observed.files.iter().any(|file| file.present == Some(true));
    let complete = observed.complete
        && observed.files.iter().all(|file| file.present.is_some())
        && observed.hub_running.is_some()
        && observed.licensing_client_running.is_some()
        && observed.hub_cli_present.is_some();
    let direct = observed.nonempty[0] || observed.nonempty[2];
    let msix = observed.nonempty[1];
    let state = if !complete {
        LicenseHint::Unknown
    } else if !any_file {
        LicenseHint::NotFound
    } else if !direct && msix && observed.licensing_client_running == Some(false) {
        LicenseHint::NeedsHubClient
    } else if direct || msix {
        LicenseHint::LikelyActive
    } else {
        LicenseHint::Unknown
    };
    // A stored check is historical, for its recorded Editor. Changed or unreadable metadata
    // cannot use it to suppress the next check, and it never makes the hint `active`.
    let unchanged = last_check.as_ref().is_some_and(|checked| {
        let checked_at = crate::rfc3339_millis(&checked.checked_at);
        observed
            .files
            .iter()
            .filter(|file| file.present == Some(true))
            .all(|file| {
                file.modified_at
                    .as_deref()
                    .and_then(crate::rfc3339_millis)
                    .zip(checked_at)
                    .is_some_and(|(modified, checked)| modified <= checked)
            })
    });
    let next_action = match state {
        LicenseHint::NotFound => LicenseNextAction::SignInAndActivate,
        LicenseHint::NeedsHubClient => LicenseNextAction::StartHub,
        LicenseHint::Unknown => LicenseNextAction::RunCheck,
        LicenseHint::LikelyActive => match last_check
            .as_ref()
            .filter(|_| unchanged)
            .map(|checked| checked.state)
        {
            Some(LicenseCheckState::Active) => LicenseNextAction::None,
            Some(LicenseCheckState::NotActivated) => LicenseNextAction::SignInAndActivate,
            _ => LicenseNextAction::RunCheck,
        },
    };
    UnityLicenseSnapshot {
        schema_version: "0.1".into(),
        files: observed.files,
        hub_type: observed.hub_type,
        hub_running: observed.hub_running,
        licensing_client_running: observed.licensing_client_running,
        hub_cli_present: observed.hub_cli_present,
        last_check,
        state,
        next_action,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseLogCategory {
    NoUlf,
    Inactive,
    NoValidEditorLicense,
}

/// Only the closed category is retained; the matching line never crosses the boundary.
pub fn classify_license_log(text: &str) -> Option<LicenseLogCategory> {
    if text.contains("No ULF license found") {
        Some(LicenseLogCategory::NoUlf)
    } else if text.contains("License is not active") {
        Some(LicenseLogCategory::Inactive)
    } else if text.contains("No valid Unity Editor license found") {
        Some(LicenseLogCategory::NoValidEditorLicense)
    } else {
        None
    }
}

pub fn valid_checked_license(checked: &CheckedLicense) -> bool {
    matches!(
        checked.editor_version.as_str(),
        "2022.3.22f1" | "2022.3.22f1c1"
    ) && crate::rfc3339_millis(&checked.checked_at).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation(present: [bool; 3], client: bool) -> LicenseObservation {
        let kinds = [
            LicenseKind::ClassicEntitlement,
            LicenseKind::MsixEntitlement,
            LicenseKind::LegacyUlf,
        ];
        LicenseObservation {
            files: std::array::from_fn(|i| LicenseFile {
                kind: kinds[i],
                present: Some(present[i]),
                modified_at: present[i].then(|| "2026-10-19T00:00:00.000Z".into()),
            }),
            nonempty: present,
            complete: true,
            hub_type: HubType::Msix,
            hub_running: Some(client),
            licensing_client_running: Some(client),
            hub_cli_present: Some(false),
        }
    }
    #[test]
    fn hints_and_next_actions_follow_metadata_without_claiming_active() {
        for (files, client, expected, action) in [
            (
                [false; 3],
                false,
                LicenseHint::NotFound,
                LicenseNextAction::SignInAndActivate,
            ),
            (
                [true, false, false],
                false,
                LicenseHint::LikelyActive,
                LicenseNextAction::RunCheck,
            ),
            (
                [false, false, true],
                false,
                LicenseHint::LikelyActive,
                LicenseNextAction::RunCheck,
            ),
            (
                [false, true, false],
                false,
                LicenseHint::NeedsHubClient,
                LicenseNextAction::StartHub,
            ),
            (
                [false, true, false],
                true,
                LicenseHint::LikelyActive,
                LicenseNextAction::RunCheck,
            ),
        ] {
            let view = snapshot(observation(files, client), None);
            assert_eq!((view.state, view.next_action), (expected, action));
            assert_ne!(serde_json::to_value(view).unwrap()["state"], "active");
        }
        let mut empty = observation([true, false, false], true);
        empty.nonempty = [false; 3];
        assert_eq!(snapshot(empty, None).state, LicenseHint::Unknown);
        let mut failed = observation([true, false, false], true);
        failed.files[1].present = None;
        assert_eq!(
            snapshot(failed, None).next_action,
            LicenseNextAction::RunCheck
        );
    }
    #[test]
    fn historical_checks_do_not_override_current_missing_clients_or_changed_files() {
        let mut checked = CheckedLicense {
            state: LicenseCheckState::Active,
            editor_version: "2022.3.22f1".into(),
            checked_at: "2026-10-19T01:00:00.000Z".into(),
        };
        let view = snapshot(
            observation([false, true, false], true),
            Some(checked.clone()),
        );
        assert_eq!(
            (view.state, view.next_action),
            (LicenseHint::LikelyActive, LicenseNextAction::None)
        );
        assert_eq!(
            snapshot(
                observation([false, true, false], false),
                Some(checked.clone())
            )
            .next_action,
            LicenseNextAction::StartHub
        );
        checked.checked_at = "2026-10-18T00:00:00.000Z".into();
        assert_eq!(
            snapshot(
                observation([true, false, false], true),
                Some(checked.clone())
            )
            .next_action,
            LicenseNextAction::RunCheck
        );
        checked.checked_at = "2026-10-19T01:00:00.000Z".into();
        checked.state = LicenseCheckState::NotActivated;
        assert_eq!(
            snapshot(observation([true, false, false], true), Some(checked)).next_action,
            LicenseNextAction::SignInAndActivate
        );
    }
    #[test]
    fn classification_retains_only_the_three_existing_license_categories() {
        for (text, expected) in [
            ("No ULF license found", LicenseLogCategory::NoUlf),
            ("License is not active", LicenseLogCategory::Inactive),
            (
                "No valid Unity Editor license found",
                LicenseLogCategory::NoValidEditorLicense,
            ),
        ] {
            assert_eq!(
                classify_license_log(&format!("private path/account: {text}")),
                Some(expected)
            );
        }
        assert_eq!(classify_license_log("Package Manager failed"), None);
    }
}
