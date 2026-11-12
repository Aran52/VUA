//! The recipe-export v0.1 core port face (proposal 029 B-face wiring loop 2,
//! core batch 2026-09-22) and the REAL export executor (implementation loop
//! 3, same day). The frozen word-row `recipe.exportProjectDraft`
//! (`schemas/recipe-export/v0.1/`, freeze batch the same day) routes through
//! the [`ProjectDraftExportPort`] trait — the synchronous read-only Query
//! that derives a RECIPE DRAFT (never a Recipe) from one registered Unity
//! project. This module lands the port face, the typed draft document, and
//! (loop 3) the on-disk executor [`OnDiskProjectDraftExporter`] over the
//! standing proposal-013 inspection facts: VPM manifest declared
//! dependencies + locked pins, the observed editor version, the VUA-native
//! identity tri-state.
//!
//! Structure (three rulings of the freeze batch, 029 inline core section):
//!
//! 1. **Synchronous Query, no tasked face** (ruling 3): the scan touches
//!    local project files only — no Bridge, no network, nothing to cancel,
//!    nothing to recover. The port method returns the document directly;
//!    no nine-state task is minted (the packages-ops preview/query
//!    precedent).
//! 2. **Declared-none capability default** (the 025 `catalog_capabilities`
//!    accessor law, ORC-DEV-004: no implementation, no reservation): the
//!    defaulted `export_capabilities` accessor answers declared-none, so a
//!    wired-but-unimplemented port keeps the served capability row and the
//!    route honestly unavailable (`vua.recipe_export.unavailable`) until
//!    the implementing adapter overrides the accessor (the environment
//!    implementation-verification slice flips it — the F5
//!    `template_capabilities` law verbatim).
//! 3. **The draft is its own type** (ruling 2): [`ProjectDraftDocumentV01`]
//!    carries NO recipeId, NO title, NO assets/instances/relations/
//!    wardrobeGroups and NO locked block — `deny_unknown_fields` makes a
//!    draft claiming the Recipe face unrepresentable at the type level, and
//!    the family const is stamped by the wire route at envelope assembly
//!    (the P1 discipline: the route stamps the consts, the port facts stay
//!    verbatim), so the typed document carries exactly the six fact keys.
//!
//! The port's typed refusals travel verbatim through the wire route (the
//! read-face pass-through discipline — no read-face fold exists). The
//! reused registration refusal `vua.project.project_not_found` (024
//! packages-query ruling: same fact, same code) is answered by the ROUTE's
//! registration calibration over the same 013 aggregate
//! `project.inspectProject` uses — an off-aggregate path never reaches the
//! port. Zero new error codes: the face's closed set is
//! `vua.recipe_export.unavailable` / `vua.recipe_export.invalid_params` /
//! the reused `vua.project.project_not_found`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::contracts::AppErrorV1;
use crate::editor_targets::classify_version_string;
use crate::time::{Clock, SystemClock};

/// Capability declaration for the project-draft export read face (proposal
/// 029 B-face loop 2). Same shape law as the packages-side
/// `TemplateCapabilities` / `RepoLifecycleCapabilities` accessors: a
/// separate defaulted trait accessor instead of a monolithic bit set, so
/// ports without the export face keep answering honestly unavailable
/// (ORC-DEV-004: no implementation, no reservation — the default is
/// declared-none; an adapter overrides it exactly when it implements
/// `export_project_draft`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDraftExportCapabilities {
    /// Covers the single `recipe.exportProjectDraft` read face: deriving a
    /// Recipe project draft from one registered Unity project.
    pub export_project_draft: bool,
}

impl ProjectDraftExportCapabilities {
    pub const NONE: Self = Self {
        export_project_draft: false,
    };
}

impl Default for ProjectDraftExportCapabilities {
    fn default() -> Self {
        Self::NONE
    }
}

/// The VUA-native identity tri-state (`.vua/project.json`), verbatim from
/// the 013 inspection aggregate (v0.2 additive finding). An applicability
/// note only — never a gate in v0.1: whether an absent identity triggers a
/// desktop difference prompt is presentation, ruling still open (proposal
/// 029 open item 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VuaIdentityStatusV01 {
    Absent,
    Present,
    Unreadable,
}

/// The draft's missing-dimension closed set — the honesty core of the
/// export face. Nine dimensions are CONSTANT members of every v0.1 draft
/// (the relation-face five — assets, instances, relations, wardrobeGroups,
/// targetAvatar — plus the semantic four — assetRoles, assetLabels,
/// sourceRefs, titleSemantics: design intent, provenance and title
/// semantics are never asserted by an export, honesty rule 1); the
/// conditional tenth (`environmentUnityVersion`) joins exactly when the
/// editor version was unreadable — the route-level wire schema pins the
/// nine constant contains and the bidirectional iff against a null
/// `unityVersionConstraint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MissingDimensionV01 {
    Assets,
    Instances,
    Relations,
    WardrobeGroups,
    TargetAvatar,
    AssetRoles,
    AssetLabels,
    SourceRefs,
    TitleSemantics,
    EnvironmentUnityVersion,
}

/// The draft origin face: the registered project identity echoed verbatim
/// from the 013 inspection aggregate. `project_name` is a source fact only
/// — the draft has NO title field (ruling 2); `None` serializes as the
/// honest `null` the frozen schema requires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DraftOriginV01 {
    /// The registered project path (proposal-013 identity), echoed
    /// verbatim.
    pub project_path: String,
    /// The project name as the 013 aggregate observed it, verbatim; null
    /// when no name fact is readable.
    pub project_name: Option<String>,
    pub vua_identity_status: VuaIdentityStatusV01,
}

/// The draft environment face: the observed editor version, recorded
/// verbatim (proposal 029 boundary 2 — no version migration); `None`
/// serializes as the honest `null`, which the wire schema iff-binds to the
/// `environmentUnityVersion` missing marker. No capabilities field: no
/// producer fact exists in v0.1 (ORC-DEV-004).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DraftEnvironmentV01 {
    pub unity_version_constraint: Option<String>,
}

/// One declared-dependency row of the draft. Rows are the manifest's
/// DECLARED dependencies (locked-only entries are transitive resolution
/// facts, not user intents — out of this face's vocabulary), sorted by
/// `package_id` ascending (the frozen deterministic presentation fact —
/// the packages-query precedent). `locked_version` carries the exact
/// locked pin for the same id when one exists; it is ABSENT from the
/// serialization when `None` (the frozen schema forbids a null member —
/// absence, not null, is the honest no-pin shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DraftDependencyV01 {
    /// The VPM package identifier (the manifest key), verbatim.
    pub package_id: String,
    /// The declared version constraint, verbatim from the manifest
    /// declaration.
    pub version_constraint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_version: Option<String>,
}

/// The Recipe project DRAFT document (the frozen v0.1 closed set MINUS the
/// family const, which the wire route stamps at envelope assembly — the
/// repoCatalog P1 discipline). A draft-seed document, NEVER a Recipe: no
/// recipeId (minted only by the save chain when the user explicitly
/// confirms), no title, no relation face, no locked block — and
/// `deny_unknown_fields` keeps every one of those unrepresentable here
/// (the draft/Recipe boundary is a type fact, ruling 2). `draft_id` is a
/// uuidv7 DRAFT INSTANCE identity minted per export call, never a Recipe
/// identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProjectDraftDocumentV01 {
    /// uuidv7 draft instance identity, minted per export call.
    pub draft_id: String,
    /// The instant the export scan completed (RFC 3339, VUA-side fact).
    pub exported_at: String,
    pub origin: DraftOriginV01,
    pub environment: DraftEnvironmentV01,
    /// The manifest's declared dependencies, packageId ascending; an EMPTY
    /// vec is the valid, honest answer (manifest absent or zero declared
    /// dependencies) — absence is never padded with guesses.
    pub dependencies: Vec<DraftDependencyV01>,
    /// The missing-dimension list: nine constant members plus the
    /// conditional tenth (see [`MissingDimensionV01`]).
    pub missing: Vec<MissingDimensionV01>,
}

/// The project-draft export port (proposal 029 B-face loop 2): the core
/// cross-domain contract the wire route serves
/// `recipe.exportProjectDraft` through. The real implementing adapter is
/// [`OnDiskProjectDraftExporter`] (loop 3); until an implementation is wired
/// the declared-none default keeps every answer honestly unavailable.
/// Errors travel verbatim through the route (the read-face pass-through
/// discipline); zero new error codes — the face's closed set is
/// `vua.recipe_export.unavailable` / `vua.recipe_export.invalid_params` /
/// the reused `vua.project.project_not_found`.
pub trait ProjectDraftExportPort: Send + Sync {
    /// Capability declaration for the export face. Default declared-none
    /// (ORC-DEV-004; the 025 accessor law): the served capability row and
    /// the route stay honestly unavailable until an implementing adapter
    /// overrides this exactly when it implements `export_project_draft`.
    fn export_capabilities(&self) -> ProjectDraftExportCapabilities {
        ProjectDraftExportCapabilities::NONE
    }

    /// Derives the Recipe project draft for ONE registered Unity project
    /// (the proposal-013 registered-path identity). Synchronous read-only:
    /// local project files only, no Bridge, no network, no mutation, no
    /// tasked face — the port returns the document directly. A failed
    /// on-disk observation is NOT an error of this face: an absent manifest
    /// yields the honest empty dependencies vec and an unreadable
    /// ProjectVersion.txt yields a null constraint plus the
    /// environmentUnityVersion marker — honesty rules 1/2, never padded,
    /// never guessed. The default body answers the face's honest-absence
    /// code so a declared-but-unimplemented adapter CAN exist at the type
    /// level (the F5 structural law — the route gate answers first, the
    /// default body is the second honest layer).
    fn export_project_draft(
        &self,
        _project_path: &str,
    ) -> Result<ProjectDraftDocumentV01, AppErrorV1> {
        Err(AppErrorV1::new(
            "vua.recipe_export.unavailable",
            crate::contracts::ErrorCategory::Unavailable,
            "errors.recipeExport.unavailable",
            "corr-recipe-export-port",
        ))
    }
}

/// The REAL export executor (proposal 029 B-face loop 3, core batch
/// 2026-09-22): the on-disk reader that derives a Recipe project draft from
/// one registered Unity project by observing exactly the facts the
/// proposal-013 inspection aggregate observes — nothing else, nothing
/// invented. The route's registration calibration (provider-host, the SAME
/// 013 aggregate `project.inspectProject` uses) has already answered
/// `vua.project.project_not_found` for off-aggregate paths before this port
/// is ever called, so the executor reads without re-judging registration.
///
/// Facts observed (the 013 aggregate's read discipline mirrored in core —
/// the aggregate imports core, never the reverse, so the reads are mirrored
/// here rather than called: zero new project-manager read faces):
///
/// - `Packages/vpm-manifest.json` — the DECLARED `dependencies` map (rows of
///   the draft; string-valued entries only, the aggregate's
///   document-read rule) joined with the same-id pin from the `locked` map
///   (a locked-only entry is a transitive resolution fact and produces no
///   row). Rows sort by `packageId` ascending — the frozen deterministic
///   presentation fact. An ABSENT manifest, an unreadable one, a corrupted
///   (non-parsing) one and a non-object one all project the honest EMPTY
///   rows vec — the 013 aggregate itself carries exactly this projection for
///   those findings (a parse failure there is an honest warning finding,
///   never an invented package list; its diagnostics channel documents the
///   cause, while the draft's closed seven-key set has no diagnostics member
///   and the face's error closed set reserves no code for on-disk
///   observations: 观察失败不设错误码，诚实律 1/2). No row is ever guessed.
/// - `ProjectSettings/ProjectVersion.txt` — the observed editor version,
///   verbatim, under the same completeness gate the aggregate applies (the
///   `m_EditorVersion:` line must carry a classifiable complete version); a
///   missing, unreadable or incomplete version line projects
///   `unityVersionConstraint: null` PLUS the `environmentUnityVersion`
///   missing marker — the frozen bidirectional iff (honesty rule 2: a failed
///   observation rides as a marked fact, never as a fabricated value and
///   never as an invented error).
/// - `.vua/project.json` — the VUA-native identity tri-state
///   (absent/present/unreadable) under the same verdict boundaries as the
///   aggregate's identity finding: a missing file is ABSENT; an existing
///   file that fails to parse or carries an unknown identity version is
///   UNREADABLE — itself evidence, never silently reported as absent.
/// - the path's final component — the project-name source fact. `None` (the
///   honest null) when the path spells no final component; the aggregate
///   keeps a whole-path echo for stale-list visibility, which is a display
///   concern the draft's schema gloss does not carry ("null when no name
///   fact is readable" — a path without a final component has no name
///   fact). The draft has NO title field either way (ruling 2).
///
/// The nine structural/semantic missing dimensions are constant members of
/// every draft (the zero-bridge ruling: the relation face is never scanned;
/// design intent — roles, labels, title semantics — is never asserted by an
/// export; option B = the user completes by hand through the confirmation
/// flow). The executor is therefore a TOTAL function over its input: every
/// on-disk observation failure degrades into the honest fact the frozen
/// word face reserves for it, and the `Err` arm exists for the port
/// contract (typed refusals of other adapters, the wire-verified
/// pass-through discipline) — this executor never produces one.
pub struct OnDiskProjectDraftExporter {
    clock: Arc<dyn Clock>,
}

impl OnDiskProjectDraftExporter {
    /// Production constructor: the real system clock stamps `exportedAt`.
    pub fn new() -> Self {
        Self {
            clock: Arc::new(SystemClock),
        }
    }

    /// Injectable-clock constructor (the ORC-TST-001 replaceable-source
    /// discipline): tests pin `exportedAt` with a fixed clock instead of
    /// depending on the real one.
    pub fn with_clock(clock: Arc<dyn Clock>) -> Self {
        Self { clock }
    }

    /// Reads the observed editor version from
    /// `ProjectSettings/ProjectVersion.txt` under the aggregate's
    /// completeness gate: the file must read, carry an `m_EditorVersion:`
    /// line, and the trimmed remainder must classify as a complete version —
    /// any miss is the honest `None` (null constraint + missing marker).
    fn observed_unity_version(project_dir: &Path) -> Option<String> {
        let text = std::fs::read_to_string(
            project_dir
                .join("ProjectSettings")
                .join("ProjectVersion.txt"),
        )
        .ok()?;
        let raw = text
            .lines()
            .find_map(|line| line.strip_prefix("m_EditorVersion:"))
            .map(str::trim)
            .unwrap_or_default();
        if raw.is_empty() {
            return None;
        }
        // Same gate as the 013 aggregate's unity_version fact: an
        // incomplete/unclassifiable version line is NOT an observed version.
        classify_version_string(raw).map(|_| raw.to_owned())
    }

    /// Reads the VUA-native identity tri-state from `.vua/project.json`
    /// under the aggregate's verdict boundaries: NotFound is ABSENT; every
    /// other read failure, a parse failure or an unknown identity version is
    /// UNREADABLE; only a parseable document at the current version is
    /// PRESENT. `marked_at` participates in the parse so the tri-state
    /// boundary matches the aggregate's identity finding exactly (a document
    /// missing a required member is unreadable evidence, not a present
    /// identity); the note and marking time are not draft facts.
    fn observed_vua_identity(project_dir: &Path) -> VuaIdentityStatusV01 {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct IdentityProbe {
            #[allow(dead_code)]
            marked_at: String,
            identity_version: u32,
        }
        const IDENTITY_SCHEMA_VERSION: u32 = 1;
        match std::fs::read_to_string(project_dir.join(".vua").join("project.json")) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                VuaIdentityStatusV01::Absent
            }
            Err(_) => VuaIdentityStatusV01::Unreadable,
            Ok(raw) => match serde_json::from_str::<IdentityProbe>(&raw) {
                Ok(probe) if probe.identity_version == IDENTITY_SCHEMA_VERSION => {
                    VuaIdentityStatusV01::Present
                }
                _ => VuaIdentityStatusV01::Unreadable,
            },
        }
    }

    /// Reads the manifest's `dependencies` (declared rows) and `locked`
    /// (same-id pins) maps as documents — the aggregate's rule: string-valued
    /// entries only, a missing or wrong-shaped field is an empty map, never a
    /// guessed entry. Sorted maps make the projection deterministic.
    fn observed_manifest(
        project_dir: &Path,
    ) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
        let manifest_path = project_dir.join("Packages").join("vpm-manifest.json");
        // Only a FILE is a manifest (the aggregate's manifest_present gate);
        // an absent manifest is the honest empty rows projection.
        if !manifest_path.is_file() {
            return (BTreeMap::new(), BTreeMap::new());
        }
        let Ok(text) = std::fs::read_to_string(&manifest_path) else {
            // Unreadable: an honest warning finding in the aggregate's
            // diagnostics channel; here the honest projection is no rows —
            // never an invented package list, never an invented error.
            return (BTreeMap::new(), BTreeMap::new());
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            // Corrupted JSON: same honest projection (the aggregate marks
            // manifest_schema_ok=false and keeps the maps empty).
            return (BTreeMap::new(), BTreeMap::new());
        };
        let Some(object) = value.as_object() else {
            // Valid JSON but not an object: the aggregate's
            // manifest_schema_unexpected finding; the maps stay empty.
            return (BTreeMap::new(), BTreeMap::new());
        };
        let string_map = |field: &str| -> BTreeMap<String, String> {
            object
                .get(field)
                .and_then(|field| field.as_object())
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|(id, version)| {
                            version
                                .as_str()
                                .map(|version| (id.clone(), version.to_owned()))
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        (string_map("dependencies"), string_map("locked"))
    }
}

impl Default for OnDiskProjectDraftExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectDraftExportPort for OnDiskProjectDraftExporter {
    /// The loop-3 override FLIP (the F5 `template_capabilities` law): the
    /// implementation exists, so the served capability row and the route
    /// gate turn available — until this override the declared-none default
    /// kept every answer honestly unavailable.
    fn export_capabilities(&self) -> ProjectDraftExportCapabilities {
        ProjectDraftExportCapabilities {
            export_project_draft: true,
        }
    }

    fn export_project_draft(
        &self,
        project_path: &str,
    ) -> Result<ProjectDraftDocumentV01, AppErrorV1> {
        let project_dir = Path::new(project_path);
        let (declared, locked) = Self::observed_manifest(project_dir);
        // Rows = the DECLARED set (locked-only entries are transitive
        // resolution facts, not user intents — no rows for them); packageId
        // ascending is the frozen deterministic presentation fact and the
        // BTreeMap iteration already yields it.
        let dependencies: Vec<DraftDependencyV01> = declared
            .into_iter()
            .map(|(package_id, version_constraint)| DraftDependencyV01 {
                locked_version: locked.get(&package_id).cloned(),
                package_id,
                version_constraint,
            })
            .collect();
        let unity_version_constraint = Self::observed_unity_version(project_dir);
        let mut missing = vec![
            MissingDimensionV01::Assets,
            MissingDimensionV01::Instances,
            MissingDimensionV01::Relations,
            MissingDimensionV01::WardrobeGroups,
            MissingDimensionV01::TargetAvatar,
            MissingDimensionV01::AssetRoles,
            MissingDimensionV01::AssetLabels,
            MissingDimensionV01::SourceRefs,
            MissingDimensionV01::TitleSemantics,
        ];
        if unity_version_constraint.is_none() {
            // The conditional tenth member — iff-bound to the null
            // constraint by the frozen schema's two implications.
            missing.push(MissingDimensionV01::EnvironmentUnityVersion);
        }
        Ok(ProjectDraftDocumentV01 {
            dependencies,
            draft_id: draft_identity_v7(),
            environment: DraftEnvironmentV01 {
                unity_version_constraint,
            },
            exported_at: self.clock.now_rfc3339(),
            missing,
            origin: DraftOriginV01 {
                project_name: project_dir
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned()),
                project_path: project_path.to_owned(),
                vua_identity_status: Self::observed_vua_identity(project_dir),
            },
        })
    }
}

/// Generates a uuid-v7-shaped DRAFT INSTANCE identity (unix-ts-ms ordering +
/// in-process counter randomness; single-process uniqueness is what the
/// per-call mint needs). The version nibble is `7` and the variant bits are
/// `10xx` so every identity satisfies the frozen `uuidV7` pattern shared by
/// the recipe v0.3 suite, the production-use-case v0.2 word list and the
/// recipe-export v0.1 draft face (the provider-host mint carries the same
/// shape law; core mirrors it because the dependency direction forbids the
/// core from calling the transport adapter).
fn draft_identity_v7() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id() as u64;
    let unix_ts_ms = ms & 0x0000_ffff_ffff_ffff;
    let ver_rand_a = 0x7000u32 | (((counter << 1) as u32) & 0x0fff);
    let var_hi = 0x8000u16 | (((pid << 4) as u16) & 0x3fff);
    let var_lo = (counter & 0xffff_ffff) | 0x0000_0001_0000_0000;
    format!(
        "{:08x}-{:04x}-7{:03x}-{:04x}-{:012x}",
        (unix_ts_ms >> 16) as u32,
        (unix_ts_ms & 0xffff) as u16,
        ver_rand_a & 0x0fff,
        var_hi,
        var_lo & 0xffff_ffff_ffff,
    )
}
