//! Provider assembly-face editor selection (U10 implementation slice, core
//! half — proposal 021 stance 2 + the integration arbitration on the gate-3
//! layering).
//!
//! The decision is a pure function over two facts: the explicit injection
//! (`VUA_UNITY_EDITOR`, the shell-injected, verified manual pick) and the
//! Hub editors-root enumeration ([`crate::installed_unity_editors`]).
//! Resolution order per the ADR `docs/decisions/path-configuration` ruling 1
//! and proposal 021 stance 2:
//!
//! 1. **Explicit injection** — the manual path already carries its own
//!    verification (editor_verify, environment domain) and the gate-3
//!    first-use confirmation on the desktop half; the core consumes it via
//!    injection and never second-guesses it here (a broken Hub root does
//!    not unselect an explicit pick).
//! 2. **Production-target auto-selection** — among the detected editors,
//!    the ones classifying as the production target (2022.3.22f1) win;
//!    the enumeration is sorted newest first, so the first production
//!    target *is* the "Unity Hub default item" tie-break of the ADR. The
//!    selection layer only resolves and presents; **execution release
//!    waits for the gate-3 first-use confirmation** (desktop settings
//!    face). Until that mechanism is in place the honest "not set =
//!    unavailable" posture holds — the auto-selection never executes.
//! 3. **None** — honest absence with the observed reason.

use std::path::PathBuf;

use crate::editor_targets::{classify_editor, EditorClass};
use crate::environment::{EditorInstallObservation, InstalledUnityEditor};

/// The assembly-face editor decision ([`select_editor`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSelection {
    /// The explicit injection executes directly (the manual path carries
    /// its own verification + gate-3 confirmation on the desktop half).
    Explicit { path: PathBuf },
    /// A production-target editor was detected. Presentation +
    /// precheck-observation only: the transition period keeps job
    /// execution unavailable until the desktop gate-3 confirmation
    /// exists — auto-selection never crosses the first-use confirmation.
    AutoSelected { editor: InstalledUnityEditor },
    /// No usable editor decision; the reason is the observed fact.
    Unavailable { reason: EditorSelectionGap },
}

/// Why no editor was selected (the observation, never a guess).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSelectionGap {
    /// No explicit injection and the Hub root held no editors at all.
    NotDetected,
    /// Editors exist but none classifies as the production target; the
    /// observed versions travel for presentation, off-target editors are
    /// never silently used (compatibility policy).
    NoProductionTarget { observed_versions: Vec<String> },
    /// The editors-root observation itself failed.
    DetectionFailed { reason: String },
}

impl EditorSelection {
    /// The path the selection points at, when any. The explicit injection
    /// and the auto-selected candidate both resolve to a concrete editor;
    /// the unavailable gap resolves to nothing.
    pub fn path(&self) -> Option<&std::path::Path> {
        match self {
            EditorSelection::Explicit { path } => Some(path),
            EditorSelection::AutoSelected { editor } => Some(&editor.path),
            EditorSelection::Unavailable { .. } => None,
        }
    }

    /// Whether the selection may execute production work. Only the
    /// explicit injection releases execution: the auto-selected candidate
    /// waits for the gate-3 first-use confirmation (desktop half), and the
    /// provider keeps the honest unavailable posture for it.
    pub fn releases_execution(&self) -> bool {
        matches!(self, EditorSelection::Explicit { .. })
    }
}

/// Resolves the editor selection from the explicit injection and the Hub
/// enumeration. The explicit injection short-circuits: it never consults
/// the enumeration, so a broken or empty Hub root cannot unselect a
/// verified manual pick.
pub fn select_editor(
    explicit: Option<PathBuf>,
    observation: &EditorInstallObservation,
) -> EditorSelection {
    if let Some(path) = explicit {
        return EditorSelection::Explicit { path };
    }
    match observation {
        EditorInstallObservation::Detected(editors) => {
            // The enumeration is sorted newest first; the first editor that
            // classifies as the production target is the auto-selection
            // (production-target versions win; the enumeration order is the
            // Hub-default tie-break).
            editors
                .iter()
                .find(|editor| classifies_production_target(editor))
                .map(|editor| EditorSelection::AutoSelected {
                    editor: editor.clone(),
                })
                .unwrap_or_else(|| EditorSelection::Unavailable {
                    reason: EditorSelectionGap::NoProductionTarget {
                        observed_versions: editors
                            .iter()
                            .map(|editor| editor.parsed.display.clone())
                            .collect(),
                    },
                })
        }
        EditorInstallObservation::NotDetected => EditorSelection::Unavailable {
            reason: EditorSelectionGap::NotDetected,
        },
        EditorInstallObservation::DetectionFailed { reason } => EditorSelection::Unavailable {
            reason: EditorSelectionGap::DetectionFailed {
                reason: reason.clone(),
            },
        },
    }
}

fn classifies_production_target(editor: &InstalledUnityEditor) -> bool {
    matches!(
        classify_editor(&editor.parsed).0,
        EditorClass::ProductionTarget
    )
}

#[cfg(test)]
mod selection_tests {
    use super::*;
    use crate::editor_targets::{parse_editor_version, PRODUCTION_TARGET};
    use std::path::Path;

    fn editor(version: &str) -> InstalledUnityEditor {
        InstalledUnityEditor {
            parsed: parse_editor_version(version).expect("test version parses"),
            path: Path::new("C:/Hub").join(version),
        }
    }

    fn detected(versions: &[&str]) -> EditorInstallObservation {
        EditorInstallObservation::Detected(versions.iter().map(|version| editor(version)).collect())
    }

    #[test]
    fn explicit_injection_wins_over_any_enumeration() {
        // Even a failed observation must not unselect the verified manual
        // pick: the explicit path never consults the enumeration.
        let selection = select_editor(
            Some(PathBuf::from("C:/manual/2022.3.22f1/Editor/Unity.exe")),
            &EditorInstallObservation::DetectionFailed {
                reason: "root unreadable".into(),
            },
        );
        assert_eq!(
            selection,
            EditorSelection::Explicit {
                path: PathBuf::from("C:/manual/2022.3.22f1/Editor/Unity.exe"),
            }
        );
        assert!(selection.releases_execution());
    }

    #[test]
    fn the_sole_production_target_is_auto_selected() {
        let selection = select_editor(
            None,
            &detected(&["2019.4.31f1", PRODUCTION_TARGET, "2021.3.5f1"]),
        );
        match selection {
            EditorSelection::AutoSelected { editor } => {
                assert_eq!(editor.parsed.display, PRODUCTION_TARGET);
                assert_eq!(editor.path, Path::new("C:/Hub").join(PRODUCTION_TARGET));
            }
            other => panic!("expected an auto-selection, got {other:?}"),
        }
    }

    #[test]
    fn the_auto_selection_never_releases_execution_in_the_transition_period() {
        let selection = select_editor(None, &detected(&[PRODUCTION_TARGET]));
        assert!(!selection.releases_execution(),
            "auto-selection resolves + presents only: execution waits for the gate-3 first-use confirmation");
    }

    #[test]
    fn among_multiple_production_targets_the_newest_detected_wins() {
        // Two production-target installs: the enumeration is sorted newest
        // first, so the first entry is the Hub-default tie-break.
        let newest = editor(PRODUCTION_TARGET);
        let newest_path = newest.path.clone();
        let selection = select_editor(
            None,
            &EditorInstallObservation::Detected(vec![newest, editor(PRODUCTION_TARGET)]),
        );
        match selection {
            EditorSelection::AutoSelected { editor } => assert_eq!(editor.path, newest_path),
            other => panic!("expected an auto-selection, got {other:?}"),
        }
    }

    #[test]
    fn off_target_editors_never_auto_select() {
        // Migration sources and other versions are observed, never used.
        let selection = select_editor(None, &detected(&["2019.4.31f1", "2022.3.6f1"]));
        assert_eq!(
            selection,
            EditorSelection::Unavailable {
                reason: EditorSelectionGap::NoProductionTarget {
                    observed_versions: vec!["2019.4.31f1".into(), "2022.3.6f1".into()],
                },
            }
        );
    }

    #[test]
    fn the_china_distribution_of_the_target_version_is_not_the_production_target() {
        // 2022.3.22f1c1 classifies as the china-distribution diagnosis, not
        // the production target (unsupported-environment policy).
        let selection = select_editor(None, &detected(&["2022.3.22f1c1"]));
        assert!(matches!(
            selection,
            EditorSelection::Unavailable {
                reason: EditorSelectionGap::NoProductionTarget { .. }
            }
        ));
    }

    #[test]
    fn an_empty_hub_root_is_honest_absence() {
        let selection = select_editor(None, &EditorInstallObservation::NotDetected);
        assert_eq!(
            selection,
            EditorSelection::Unavailable {
                reason: EditorSelectionGap::NotDetected,
            }
        );
    }

    #[test]
    fn a_failed_observation_stays_a_failed_observation() {
        let selection = select_editor(
            None,
            &EditorInstallObservation::DetectionFailed {
                reason: "access denied".into(),
            },
        );
        assert_eq!(
            selection,
            EditorSelection::Unavailable {
                reason: EditorSelectionGap::DetectionFailed {
                    reason: "access denied".into(),
                },
            }
        );
    }

    #[test]
    fn the_selection_path_follows_the_resolution() {
        let explicit = select_editor(
            Some(PathBuf::from("C:/manual/2022.3.22f1/Editor/Unity.exe")),
            &EditorInstallObservation::NotDetected,
        );
        assert_eq!(
            explicit.path(),
            Some(std::path::Path::new(
                "C:/manual/2022.3.22f1/Editor/Unity.exe"
            ))
        );
        let auto = select_editor(None, &detected(&[PRODUCTION_TARGET]));
        assert_eq!(
            auto.path(),
            Some(Path::new("C:/Hub").join(PRODUCTION_TARGET).as_path())
        );
        assert_eq!(
            select_editor(None, &EditorInstallObservation::NotDetected).path(),
            None
        );
    }
}
