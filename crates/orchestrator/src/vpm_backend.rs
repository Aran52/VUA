//! VPM backend port (E-VPM-DUAL, ADR-0006): the dual-backend contract
//! consumed by the assembly engine and the material executor. The
//! concrete vrc-get-library and VCC CLI implementations live in
//! vua-project-manager.

use crate::contracts::{AppErrorV1, ErrorCategory, ParamValue};
use crate::ProjectRef;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod error_codes {
    pub const NO_MATCHING_PACKAGE: &str = "vua.vpm.no_matching_package";
    pub const PREVIEW_FAILED: &str = "vua.vpm.preview_failed";
    pub const APPLY_FAILED: &str = "vua.vpm.apply_failed";
    pub const PREVIEW_DRIFT: &str = "vua.vpm.preview_drift";
    pub const CAPABILITY_MISSING: &str = "vua.vpm.capability_missing";
    pub const TEMPLATE_MISSING: &str = "vua.vpm.template_missing";
    pub const BACKEND_UNAVAILABLE: &str = "vua.vpm.backend_unavailable";
    pub const LOCAL_PACKAGE_INVALID: &str = "vua.vpm.local_package_invalid";
    pub const LOCAL_PACKAGE_REGISTER_FAILED: &str = "vua.vpm.local_package_register_failed";
    pub const PACKAGE_NOT_INSTALLED: &str = "vua.vpm.package_not_installed";
    pub const PROJECT_LOAD_FAILED: &str = "vua.vpm.project_load_failed";
    /// A4 repository add/remove face (proposal 026 freeze batch): the add
    /// guard refused the row (duplicate url/name, official/curated guard,
    /// malformed shape).
    pub const REPO_INVALID: &str = "vua.vpm.repo_invalid";
    /// A4: remove_repo named a repoId absent from the subscription list.
    pub const REPO_NOT_FOUND: &str = "vua.vpm.repo_not_found";
    /// A4: the remote-repository manifest fetch failed (the add-remote
    /// network segment; the local-add and remove faces never answer this).
    pub const REPO_FETCH_FAILED: &str = "vua.vpm.repo_fetch_failed";
    /// A4: the isolated-environment settings write-back failed (save /
    /// atomic replace / backup maintenance).
    pub const REPO_WRITE_FAILED: &str = "vua.vpm.repo_write_failed";
}

/// Which optional capabilities a backend actually provides (honest gating,
/// ORC-ADP-007): the frontend only renders entry points for true capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpmCapabilities {
    /// A5 (proposal 026 freeze batch): this EXISTING five-bit-closed-set
    /// member is the creation gate — no new accessor is frozen for A5 (the
    /// bit predates the batch and both in-repo backends already declare it
    /// honestly: the library backend true, the CLI backend true). The wire
    /// gate reads it before submit; a false bit answers the generic
    /// capability_missing and never reaches a task.
    pub create_project: bool,
    pub preview_install: bool,
    /// B6: read the installed package set of one project.
    pub list_packages: bool,
    /// B6: digest-bound removal preview + apply on one project.
    pub remove_packages: bool,
    /// B6: enumerate the manager's registered project paths.
    pub project_registry: bool,
    /// Batch 146 (user ruling 2026-09-21: SDK import before real-machine
    /// acceptance): resolve the SDK dependencies a project's
    /// `Packages/vpm-manifest.json` DECLARES — the creation template only
    /// declares them (`com.vrchat.base` / `com.vrchat.avatars`), the package
    /// bodies are not vendored by the copy. This is the预留 bit the struct
    /// comment reserved ("resolve joins this struct when a backend grows the
    /// matching method" — ORC-DEV-004: no implementation, no reservation):
    /// the method lands in the same batch, so the reservation closes now.
    /// Default declared-none: the trait-default `resolve_project` answers the
    /// capability_missing family; the VrcGetLib backend overrides the bit to
    /// true, the VCC CLI backend stays honestly false (no resolve command —
    /// the CLI's creation cannot complete the declared-dependency face).
    pub resolve_project: bool,
}

/// One entry of an install preview (ORC-WF-002: the plan must cover every
/// change the backend will make -- transitive deps, removals, conflicts).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeItemV1 {
    pub kind: ChangeKindV1,
    pub package_id: String,
    /// Target version for installs; None for removals.
    pub version: Option<String>,
    /// Machine reason, e.g. `transitive_dependency`, `conflict`.
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKindV1 {
    Install,
    Remove,
}

/// The complete, digest-stamped change set of one atomic request batch
/// (ORC-WF-003: the user confirms exactly this content; the apply step
/// re-requests the preview and refuses to run on digest drift).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePreviewV1 {
    pub items: Vec<ChangeItemV1>,
    pub conflicts: Vec<String>,
    pub remove_legacy_files: Vec<String>,
    pub remove_legacy_folders: Vec<String>,
    /// True when the change set contains conflicts or legacy removals --
    /// the confirm UI must warn (ADR-0006).
    pub destructive: bool,
    /// FNV-1a over the canonical item list; confirmed digest is bound to it.
    pub digest: String,
}

/// One package requested as part of an atomic VPM change calculation.
/// Multiple requests must be previewed together so dependency interactions,
/// upgrades and removals are represented by one confirmation digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageRequestV1 {
    pub package_id: String,
    pub version: Option<String>,
}

/// One installed package of a project, as resolved from its VPM manifest
/// and lock by the backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackageV1 {
    pub package_id: String,
    pub version: String,
    /// Direct dependencies the installed package declares.
    pub dependencies: Vec<String>,
}

/// One project registered in the manager's project registry (the
/// VCC-compatible database VUA shares with VCC/ALCOM/vrc-get).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredProjectV1 {
    pub path: String,
    pub name: String,
}

/// P2 read-family capability declaration (proposal 025 freeze batch,
/// 2026-09-17). Declared as a separate defaulted trait accessor instead of
/// a new `VpmCapabilities` field so the five-bit closed set stays stable
/// and backends that do not implement the catalog faces keep compiling
/// unchanged (ORC-DEV-004: no implementation, no reservation — the default
/// is declared-none; a backend overrides it exactly when it implements
/// `list_repos` / `package_catalog`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogCapabilities {
    /// Covers both P2 read faces (`packages.listRepos` /
    /// `packages.packageCatalog`): the repo subscription list and the
    /// per-package catalog facts share one backend capability.
    pub catalog: bool,
}

impl CatalogCapabilities {
    pub const NONE: Self = Self { catalog: false };
}

/// A3 write-face capability declaration (proposal 026 freeze batch,
/// 2026-09-19). Same shape law as `CatalogCapabilities` (the 025
/// precedent): a separate defaulted trait accessor instead of a new
/// `VpmCapabilities` field, so the five-bit closed set stays stable and
/// backends without the local-package registration face keep compiling
/// unchanged (ORC-DEV-004: no implementation, no reservation — the
/// default is declared-none; a backend overrides it exactly when it
/// implements `register_local_package`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterCapabilities {
    /// Covers the A3 write face (`packages.registerLocalPackage`):
    /// registering a generated local package in the backend's isolated
    /// environment.
    pub register_local_package: bool,
}

impl RegisterCapabilities {
    pub const NONE: Self = Self {
        register_local_package: false,
    };
}

/// A4 write-face capability declaration (proposal 026 freeze batch,
/// 2026-09-19). Same shape law as `CatalogCapabilities` / `RegisterCapabilities`
/// (the 025 accessor precedent): a separate defaulted trait accessor instead
/// of a new `VpmCapabilities` field, so the five-bit closed set stays stable
/// and backends without the repo add/remove face keep compiling unchanged
/// (ORC-DEV-004: no implementation, no reservation — the default is
/// declared-none; a backend overrides it exactly when it implements the
/// `add_remote_repo` / `add_local_repo` / `remove_repo` methods). Three
/// INDEPENDENT bits on purpose: a backend may serve a subset of the face
/// (e.g. local-add only), and the honest gate is per method, never per face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoWriteCapabilities {
    /// Covers `packages.addRemoteRepo`: subscribing a remote repository
    /// (the manifest-fetch network segment included).
    pub add_remote_repo: bool,
    /// Covers `packages.addLocalRepo`: subscribing a local directory
    /// repository.
    pub add_local_repo: bool,
    /// Covers `packages.removeRepo`: removing one subscription row by id.
    pub remove_repo: bool,
}

impl RepoWriteCapabilities {
    pub const NONE: Self = Self {
        add_remote_repo: false,
        add_local_repo: false,
        remove_repo: false,
    };
}

/// F2 read-face capability declaration (proposal 027 freeze batch,
/// 2026-09-20). Same shape law as `CatalogCapabilities` / `RegisterCapabilities`
/// / `RepoWriteCapabilities` (the 025 accessor precedent): a separate
/// defaulted trait accessor instead of a new `VpmCapabilities` field, so the
/// five-bit closed set stays stable and backends without the repo-catalog
/// read face keep compiling unchanged (ORC-DEV-004: no implementation, no
/// reservation — the default is declared-none; a backend overrides it
/// exactly when it implements `repo_catalog`). The library backend has the
/// repo-scale listing (PackageCollection::get_remote); the CLI backend has
/// no repo-scale package listing and stays honestly false.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoCatalogCapabilities {
    /// Covers the F2 read face (`packages.repoCatalog`): the per-repository
    /// installable-package inventory over the backend collection's cache.
    pub repo_catalog: bool,
}

impl RepoCatalogCapabilities {
    pub const NONE: Self = Self {
        repo_catalog: false,
    };
}

/// P2: one repository subscription row (proposal 025 freeze batch). The
/// subscription face is the world (the user's configuration fact), so the
/// row projects the settings userRepos entry verbatim: every
/// identifier/location fact is an Option and `None` is projected as an
/// honest absence — never padded, never guessed. `cached` is the REQUIRED
/// per-repo cache-hit fact: false = subscribed but never refreshed, its own
/// honest state (never hidden, never rendered as an empty catalog).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfoV01 {
    pub repo_id: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
    pub local_path: Option<String>,
    pub cached: bool,
}

/// P2: the resolved origin of a catalog package (proposal 025 freeze
/// batch). The desktop three-state presentation composes this with the
/// separate `installed` fact of `PackageCatalogV01`; the word face never
/// merges origin and installation into one word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageSourceV01 {
    /// Resolved from a repository cache.
    Repo,
    /// User-local package (no repository).
    Local,
}

/// P2: one repository-cache version row (proposal 025 freeze batch).
/// `yanked` is the repo-cache-carried yank fact; `compatible` is evaluated
/// against the selected project's Unity version and is `None` exactly when
/// that version is unknown (absence is not incompatibility).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogVersionV01 {
    pub version: String,
    pub yanked: bool,
    pub compatible: Option<bool>,
}

/// P2: per-package catalog facts for one package in one registered
/// project's context (proposal 025 freeze batch). `update_available` is
/// the frozen judgment CONCLUSION (a strictly newer compatible version
/// exists vs this project's installed version); `None` = judgment not
/// executed (package not installed here, or project Unity version
/// unknown) — absence is never "no update". `versions` is empty for a
/// local-source package (honest empty, not an error).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageCatalogV01 {
    pub project_path: String,
    pub package_id: String,
    pub display_name: Option<String>,
    pub source: PackageSourceV01,
    pub installed: bool,
    pub update_available: Option<bool>,
    pub versions: Vec<CatalogVersionV01>,
}

/// P2 v0.2 increment (proposal 025 inline ruling, 2026-09-17): the catalog
/// facts of `PackageCatalogV01` plus the REQUIRED informational
/// `cache_sourced` disclosure fact. true = THIS result was served through
/// the cache-degradation path (offline → load_cache, or an online load
/// failed and degraded — the ORC-ADP-006 isomorphic precedent); false =
/// served from an online-refreshed load. Cache sourcing is not an error:
/// consumers render it as an informational "cached data" annotation, never
/// a failure. The `packages-repos` family has NO such field: `list_repos`
/// is a zero-network face, where the fact would be a permanent constant —
/// a constant informational field is not a fact and gets no wire key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageCatalogV02 {
    pub project_path: String,
    pub package_id: String,
    pub display_name: Option<String>,
    pub source: PackageSourceV01,
    pub installed: bool,
    pub update_available: Option<bool>,
    pub versions: Vec<CatalogVersionV01>,
    pub cache_sourced: bool,
}

/// F2 (proposal 027 freeze batch): one installable package row of ONE
/// repository's cache inventory — the library's ACTUAL field ceiling
/// (environment verification 3bd4f12 s1(b)). `author` is deliberately
/// absent: the library's manifest deserialization carries no author field
/// and drops undeclared keys, and v0.1 rules the honest-absence option
/// (iii) over a second parsing surface (one fact source). `latest_version`
/// is the frozen per-repo judgment (this repo's newest version neither
/// yanked nor excluded by the user's prerelease setting, NO project Unity
/// constraint); null = no version qualifies under the current setting —
/// absence is not "no packages". There is deliberately NO compatible fact
/// on this face: without a project context the judgment cannot execute and
/// a constant null is not a fact (per-version compatible stays the
/// packages-catalog face's project-bound frozen fact).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoCatalogPackageV01 {
    pub package_id: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub latest_version: Option<String>,
    /// The repository cache's own inventory count of version entries
    /// (yanked included) — the cache fact as counted, not an availability
    /// promise.
    pub version_count: u64,
}

/// F2 (proposal 027 freeze batch): one repository's inventory row. The
/// world of the face is the backend collection's repository set (predefined
/// official/curated unless ignored + subscribed user repos) — NOT the
/// subscription face (the packages-repos family keeps that word), and
/// never a cross-repository merge (a package in several repos appears under
/// each; the cross-repo latest judgment stays the packages-catalog family's
/// fact). `cached` is the REQUIRED per-repo cache-hit fact: false =
/// subscribed but never refreshed — its own honest state rendered with an
/// EMPTY packages array, never hidden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoCatalogRepoV01 {
    pub repo_id: Option<String>,
    pub name: Option<String>,
    pub cached: bool,
    pub packages: Vec<RepoCatalogPackageV01>,
}

/// F2 (proposal 027 freeze batch): the per-repository installable-package
/// inventory (wire method `packages.repoCatalog`, family
/// `vua.packages-repo-catalog/v0.1`). `cache_sourced` is the REQUIRED
/// informational degradation disclosure born with this v0.1 (the
/// packages-catalog v0.2 precedent adopted at birth per the 027 core
/// ruling): true = served through the cache-degradation path (offline ->
/// load_cache, or an online load failed and degraded — the ORC-ADP-006
/// isomorphic precedent); false = served from an online-refreshed load.
/// Cache sourcing is not an error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoCatalogV01 {
    /// Collection's own enumeration order projected verbatim (no invented
    /// sort keys); EMPTY = honest zero-repository-caches answer.
    pub repos: Vec<RepoCatalogRepoV01>,
    pub cache_sourced: bool,
}

/// F5 (proposal 027 freeze batch, 2026-09-20): one available template entry
/// of the template-enumeration read face (wire method
/// `packages.listTemplates`, family `vua.packages-templates/v0.1`). The
/// enumeration's fact source is the two pinned directory roots of the
/// library-path default resolution leg — `<environment_root>/VRCTemplates`
/// first, then `<environment_root>/Templates` (the create_from_template
/// resolution order, environment verification 027 s4: vrc-get-vpm 0.0.16
/// ships NO template enumeration API, so the scan IS the enumeration) —
/// with the duplicate-name rule VRCTemplates-first (an id present under
/// both roots enumerates ONCE, resolved to the root the creation
/// resolution order would pick: enumeration never diverges from what
/// create would actually copy). `id` is the template directory name; `name`
/// is its frozen same-value display projection (no independent display-name
/// fact source exists in v0.1 — the projection states the identity
/// verbatim, consumers never fabricate a friendlier label); there is
/// deliberately NO description/metadata field: the template directory's
/// metadata-file shape is unverified (W25 real-machine item) and v0.1 has
/// no producer for it (ORC-DEV-004: no implementation, no reservation —
/// the P1 displayName precedent; a row carrying description/sourceRoot is
/// INVALID by schema, negative vectors pin it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateEntryV01 {
    pub id: String,
    pub name: String,
}

/// F5 (proposal 027 freeze batch): capability declaration for the
/// template-enumeration read face. Same shape law as `CatalogCapabilities`
/// / `RegisterCapabilities` / `RepoWriteCapabilities` / `RepoCatalogCapabilities`
/// (the 025 accessor precedent): a separate defaulted trait accessor
/// instead of a new `VpmCapabilities` field, so the five-bit closed set
/// stays stable and backends without the template-enumeration face keep
/// compiling unchanged (ORC-DEV-004: no implementation, no reservation —
/// the default is declared-none; a backend overrides it exactly when it
/// implements `list_templates`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateCapabilities {
    /// Covers the F5 read face (`packages.listTemplates`): the available
    /// template entries under the two pinned directory roots.
    pub list_templates: bool,
}

impl TemplateCapabilities {
    pub const NONE: Self = Self {
        list_templates: false,
    };
}

/// F4 write-face capability declaration (proposal 027 freeze batch,
/// 2026-09-20). Same shape law as `CatalogCapabilities` / `RegisterCapabilities`
/// / `RepoWriteCapabilities` / `RepoCatalogCapabilities` / `TemplateCapabilities`
/// (the 025 accessor precedent): a separate defaulted trait accessor instead
/// of a new `VpmCapabilities` field, so the five-bit closed set stays stable
/// and backends without the repository-lifecycle face keep compiling
/// unchanged (ORC-DEV-004: no implementation, no reservation — the default
/// is declared-none; a backend overrides it exactly when it implements the
/// `enable_repo` / `disable_repo` / `refresh_repo` methods). Three
/// INDEPENDENT bits on purpose (the A4 `RepoWriteCapabilities` law): a
/// backend may serve a subset of the face, and the honest gate is per
/// method, never per face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoLifecycleCapabilities {
    /// Covers `packages.enableRepo`: re-activating one disabled subscription
    /// row (VUA-owned state — the W25 evidence record ruling (c): VCC
    /// carries no enable/disable state anywhere, so the state is VUA's own).
    pub enable_repo: bool,
    /// Covers `packages.disableRepo`: excluding one subscription row from
    /// the package-collection world (subscribed and listed, never resolved).
    pub disable_repo: bool,
    /// Covers `packages.refreshRepo`: the etag-conditional cache refresh of
    /// one subscription row's own cache file (the network segment is
    /// inherent to the face).
    pub refresh_repo: bool,
}

impl RepoLifecycleCapabilities {
    pub const NONE: Self = Self {
        enable_repo: false,
        disable_repo: false,
        refresh_repo: false,
    };
}

/// F4 (proposal 027 freeze batch, packages-ops v0.6): the refresh-outcome
/// fact of one `refresh_repo` call. `cache_updated` is the library's own
/// two-arm outcome: true = the etag-conditional fetch wrote a new cache
/// file (the subscription's own `userRepos[i].localPath`); false = the etag
/// was unchanged ("already up to date"). BOTH arms are success — "no new
/// data" is a refresh outcome, never an error (the wire receipt carries it
/// as the REQUIRED `cacheUpdated` fact).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoRefreshOutcomeV01 {
    pub cache_updated: bool,
}

/// F4 (proposal 027 freeze batch, packages-repos v0.2): one subscription row
/// at the v0.2 word face — the frozen v0.1 five-key projection (repoId /
/// name / url / localPath / cached, every Option projected as an honest
/// absence, row order the configuration fact) plus EXACTLY one new REQUIRED
/// fact: `enabled`, the VUA-owned enable/disable state bit read back from
/// the backend's own storage (NEVER a settings.json key — the W25 evidence
/// record ruling (c): VCC carries no enable counterpart, so there is
/// nothing to share and nothing another writer could strip). true = the row
/// is active in the package-collection world; false = disabled (subscribed
/// and listed, its packages excluded from enumeration and resolution). A row
/// whose `repo_id` is `None` projects `enabled: true` ALWAYS: id-absent rows
/// are outside the toggle faces' reach (the `remove_repo` same boundary —
/// the id IS the row handle), so true is its honest permanent fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfoV02 {
    pub repo_id: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
    pub local_path: Option<String>,
    pub cached: bool,
    pub enabled: bool,
}

/// F3 (proposal 027 freeze batch, packages-query v0.2): one installed-package
/// row at the v0.2 word face — the frozen v0.1 three-key projection
/// (packageId / version / dependencies; packageId-ascending order stays the
/// frozen presentation fact, zero movement) plus EXACTLY two REQUIRED
/// judgment facts. `latest_version` is the latest-version fact found by the
/// frozen selector over the collection's WHOLE repository set (the
/// cross-repository max — deliberately NOT the per-repo view, which stays
/// the packages-repo-catalog family's declared fact; the two views are
/// different facts and are never conflated); null = no qualifying latest
/// under the current setting (the package sits in no repository cache — a
/// local-source package, or every candidate is yanked/excluded) — absence
/// is never "no update". `update_available` is the frozen judgment
/// CONCLUSION (a strictly newer selector-qualifying version exists vs the
/// installed version); null = judgment not executed (no qualifying latest,
/// or the project's Unity version is unknown) — null is never "already
/// latest" (the 024 stance-2 false-assertion line). The selector reuses the
/// packages-catalog frozen semantics verbatim (latest_for(project Unity
/// version, show_prerelease setting), zero wire switch): when the installed
/// version is itself a prerelease and the setting is off, the qualifying
/// latest comes from the stable set — `false` means exactly "no strictly
/// newer version matching the CURRENT filter exists", never a generalized
/// "no update".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackageV02 {
    pub package_id: String,
    pub version: String,
    /// Direct dependencies the installed package declares (the frozen v0.1
    /// fact, unchanged).
    pub dependencies: Vec<String>,
    pub latest_version: Option<String>,
    pub update_available: Option<bool>,
}

/// F3 (proposal 027 freeze batch, packages-query v0.2): the installed-set
/// listing at the v0.2 word face — the v0.2 rows plus the REQUIRED
/// informational `cache_sourced` disclosure (the packages-catalog v0.2
/// precedent adopted for the judgment face): true = this listing's judgment
/// rode the cache-degradation path (offline -> load_cache, or an online
/// load failed and degraded — the ORC-ADP-006 isomorphic precedent); false
/// = served from an online-refreshed load. The whole table's judgments ride
/// ONE collection load (the environment-verification cost law: per-row
/// collection reloads never serve this face). projectPath is an
/// envelope-assembly fact: the route stamps it, the backend facts stay
/// verbatim (the P1 discipline).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledListingV02 {
    pub packages: Vec<InstalledPackageV02>,
    pub cache_sourced: bool,
}

/// Batch 146: the schema-version constant of the resolve receipt family.
/// The receipt is an in-process supply-step fact (NOT a wire face — the
/// desktop gateway never carries it), so the version travels as the named
/// family constant here, the same identification law as the standing
/// `*_SCHEMA_VERSION` consts; the struct keeps the minimal closed field set
/// the batch pinned (`resolved` / `already_satisfied` / `failed`).
pub const RESOLVE_RECEIPT_SCHEMA_VERSION: &str = "vua.vpm-resolve-receipt/v0.1";

/// Batch 146: one dependency the resolve installed — the package id, the
/// locked version written back to `Packages/vpm-manifest.json`, and the
/// repository the package came from (the subscription row's id when present,
/// else its name; a local-path source is recorded verbatim as `local`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedPackageV01 {
    pub id: String,
    pub version: String,
    pub source_repo: String,
}

/// Batch 146: one dependency resolve could not satisfy. `reason_code` reuses
/// the standing `vua.vpm.*` codes (zero new codes — the freeze transports
/// honest faces, it mints no code); a failed entry means the receipt's
/// `resolved` set is INCOMPLETE for the declared dependency set, never a
/// partial success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveFailureV01 {
    pub id: String,
    pub reason_code: String,
}

/// Batch 146: the outcome of one `resolve_project` call. Three arms, and
/// exactly the minimal closed field set the batch pinned:
/// - `resolved`: packages fetched and installed into `Packages/` with their
///   locked versions written back to the manifest (the network segment is
///   inherent — declared dependencies are not vendored by the template copy);
/// - `already_satisfied`: dependency ids whose locked/declared requirement
///   was already met on disk (the idempotency arm: a second resolve over an
///   unchanged project answers this vec and touches nothing);
/// - `failed`: dependencies that could not be resolved (see
///   [`ResolveFailureV01`] — the honest incomplete face).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveReceiptV01 {
    pub resolved: Vec<ResolvedPackageV01>,
    pub already_satisfied: Vec<String>,
    pub failed: Vec<ResolveFailureV01>,
}

impl ResolveReceiptV01 {
    pub const SCHEMA_VERSION: &'static str = RESOLVE_RECEIPT_SCHEMA_VERSION;
    /// The honest empty receipt: nothing resolved, nothing pending, nothing
    /// failed — the shape a backend without declared dependencies answers.
    pub const EMPTY: Self = Self {
        resolved: Vec::new(),
        already_satisfied: Vec::new(),
        failed: Vec::new(),
    };
}

/// One VPM backend implementation.
pub trait VpmBackend: Send + Sync {
    /// Stable backend name, e.g. `vrc-get-lib`, `vcc-cli`.
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> VpmCapabilities;
    /// Read-only preview of everything an install would change.
    fn preview_install(
        &self,
        project: &ProjectRef,
        packages: &[PackageRequestV1],
    ) -> Result<ChangePreviewV1, AppErrorV1>;
    /// Plans against the same baseline that project creation will produce.
    /// Most fakes and existing-project backends can use the default directly;
    /// template-aware backends override it for a not-yet-created target.
    fn preview_install_for_plan(
        &self,
        project: &ProjectRef,
        packages: &[PackageRequestV1],
        _template: Option<&str>,
    ) -> Result<ChangePreviewV1, AppErrorV1> {
        self.preview_install(project, packages)
    }
    /// Applies a previously previewed install. Re-requests the preview and
    /// refuses to run when its digest differs from the confirmed one
    /// (ORC-WF-003/004: same binding discipline as assembly plans).
    fn apply_install(
        &self,
        project: &ProjectRef,
        packages: &[PackageRequestV1],
        confirmed_digest: &str,
    ) -> Result<serde_json::Value, AppErrorV1>;
    /// Registers a generated local package in this backend's isolated
    /// environment.
    fn register_local_package(&self, _package_root: &Path) -> Result<(), AppErrorV1> {
        Err(unsupported("register_local_package"))
    }
    /// B6: read the installed package set of one project (manifest + lock).
    fn list_packages(&self, _project: &ProjectRef) -> Result<Vec<InstalledPackageV1>, AppErrorV1> {
        Err(unsupported("list_packages"))
    }
    /// B6: read-only removal preview — what disappears, what breaks.
    fn preview_remove(
        &self,
        _project: &ProjectRef,
        _package_ids: &[String],
    ) -> Result<ChangePreviewV1, AppErrorV1> {
        Err(unsupported("preview_remove"))
    }
    /// B6: apply a confirmed removal under the same digest discipline as
    /// installs.
    fn apply_remove(
        &self,
        _project: &ProjectRef,
        _package_ids: &[String],
        _confirmed_digest: &str,
    ) -> Result<serde_json::Value, AppErrorV1> {
        Err(unsupported("preview_remove"))
    }
    /// B6: enumerate the manager's registered project paths.
    fn project_registry(&self) -> Result<Vec<RegisteredProjectV1>, AppErrorV1> {
        Err(unsupported("project_registry"))
    }
    /// P2 (proposal 025 freeze batch): capability declaration for the
    /// catalog read faces. The default is declared-none; a backend
    /// overrides it exactly when it implements `list_repos` and
    /// `package_catalog`.
    fn catalog_capabilities(&self) -> CatalogCapabilities {
        CatalogCapabilities::NONE
    }
    /// A3 (proposal 026 freeze batch): capability declaration for the
    /// local-package registration write face. The default is
    /// declared-none; a backend overrides it exactly when it implements
    /// `register_local_package` (the 025 accessor law — the VrcGetLib
    /// override lands with the environment implementation-verification
    /// slice, the same honest-absence discipline: the served wire row
    /// stays unavailable until the override flips it).
    fn register_capabilities(&self) -> RegisterCapabilities {
        RegisterCapabilities::NONE
    }
    /// A4 (proposal 026 freeze batch): capability declaration for the
    /// repository add/remove write face. The default is declared-none; a
    /// backend overrides it exactly when it implements the three repo
    /// write methods (the 025 accessor law — the VrcGetLib override lands
    /// with the environment implementation-verification slice, the same
    /// honest-absence discipline: the served wire row stays unavailable
    /// until the override flips it).
    fn repo_write_capabilities(&self) -> RepoWriteCapabilities {
        RepoWriteCapabilities::NONE
    }
    /// F2 (proposal 027 freeze batch): capability declaration for the
    /// repo-catalog read face. The default is declared-none; a backend
    /// overrides it exactly when it implements `repo_catalog` (the 025
    /// accessor law — the VrcGetLib override lands with the environment
    /// implementation-verification slice, the same honest-absence
    /// discipline: the served wire row stays unavailable until the
    /// override flips it; the CLI backend has no repo-scale listing and
    /// stays honestly false).
    fn repo_catalog_capabilities(&self) -> RepoCatalogCapabilities {
        RepoCatalogCapabilities::NONE
    }
    /// F4 (proposal 027 freeze batch): capability declaration for the
    /// repository-lifecycle write face. The default is declared-none; a
    /// backend overrides it exactly when it implements the `enable_repo` /
    /// `disable_repo` / `refresh_repo` methods (the 025 accessor law — the
    /// VrcGetLib override lands with the environment implementation-
    /// verification slice, the same honest-absence discipline: the served
    /// wire row stays unavailable until the override flips it; the CLI
    /// backend has no lifecycle face and stays honestly false).
    fn repo_lifecycle_capabilities(&self) -> RepoLifecycleCapabilities {
        RepoLifecycleCapabilities::NONE
    }
    /// A4 (proposal 026 freeze batch): subscribe one REMOTE repository in
    /// this backend's isolated environment. The backend fetches the remote
    /// manifest (the network segment is inherent to the face — a preview
    /// cannot verify reachability without doing the same network work, so
    /// the face has NO preview arm), then adds the subscription row. The
    /// user-supplied `name` is required (the subscription list presents it;
    /// the read face `RepoInfoV01.name` Option projects EXISTING rows
    /// verbatim, it does not imply new rows may go nameless). Guard
    /// refusals (duplicate url/name, official/curated guard, malformed
    /// shape) answer `vua.vpm.repo_invalid`; fetch failures answer
    /// `vua.vpm.repo_fetch_failed`; settings write-back failures answer
    /// `vua.vpm.repo_write_failed`.
    fn add_remote_repo(&self, _url: &str, _name: &str) -> Result<(), AppErrorV1> {
        Err(unsupported("add_remote_repo"))
    }
    /// A4 (proposal 026 freeze batch): subscribe one LOCAL directory
    /// repository in this backend's isolated environment. No network
    /// segment. The same required-`name` and guard disciplines as
    /// `add_remote_repo`; guard refusals answer `vua.vpm.repo_invalid`,
    /// settings write-back failures answer `vua.vpm.repo_write_failed`.
    fn add_local_repo(&self, _path: &Path, _name: &str) -> Result<(), AppErrorV1> {
        Err(unsupported("add_local_repo"))
    }
    /// A4 (proposal 026 freeze batch): remove ONE subscription row by its
    /// repository id. Id-addressed on purpose (an index drifts under
    /// concurrent writers; an id is the row's stable handle). Rows whose
    /// id is absent (the read face projects `RepoInfoV01.repo_id` as an
    /// honest Option) are OUTSIDE this word face's remove reach — the
    /// honest boundary is declared in the protocol document, not papered
    /// over with index or url addressing. An unknown repoId answers
    /// `vua.vpm.repo_not_found`; settings write-back failures answer
    /// `vua.vpm.repo_write_failed`.
    fn remove_repo(&self, _repo_id: &str) -> Result<(), AppErrorV1> {
        Err(unsupported("remove_repo"))
    }
    /// F4 (proposal 027 freeze batch): re-activate one disabled subscription
    /// row by its repository id. Id-addressed on purpose (the `remove_repo`
    /// same handle law). The state is VUA-OWNED semantics — the W25
    /// read-only evidence record (proposal 027 s6) concluded ruling (c):
    /// VCC carries no enable/disable state anywhere, so the toggle shares
    /// nothing and the state lives in VUA's own storage under the
    /// environment root (NEVER a userRepos[i] key — vrc-get's own save
    /// strips unknown element keys — and NEVER a settings.json top-level
    /// key — the VCC/ALCOM writer tolerance for unknown top-level keys is
    /// unverified; the shared file carries shared facts only). An unknown
    /// repoId answers the REUSED `vua.vpm.repo_not_found`; the state-file
    /// write-back failing answers the REUSED `vua.vpm.repo_write_failed`
    /// (zero new codes — the freeze transports honest faces, it mints no
    /// code).
    fn enable_repo(&self, _repo_id: &str) -> Result<(), AppErrorV1> {
        Err(unsupported("enable_repo"))
    }
    /// F4 (proposal 027 freeze batch): exclude one subscription row from
    /// the backend's package-collection world by its repository id. The
    /// disabled row STAYS subscribed and listed (the packages-repos v0.2
    /// `enabled` bit projects the state; disabling hides nothing from the
    /// configuration view) — enumeration and resolution faces
    /// (repo-catalog listing, latest-version judgment, install resolver)
    /// never see its packages. A newly added subscription row is always
    /// enabled (the add faces reset any stale state entry — a fresh
    /// subscription starts fresh); removing a row leaves no state residue.
    /// Error faces: the `enable_repo` reused-code law verbatim.
    fn disable_repo(&self, _repo_id: &str) -> Result<(), AppErrorV1> {
        Err(unsupported("disable_repo"))
    }
    /// F4 (proposal 027 freeze batch): the etag-conditional cache refresh of
    /// one subscription row's OWN cache file (`userRepos[i].localPath` — the
    /// exact write vrc-get itself performs on refresh, same-origin with
    /// VCC/vrc-get; the official/curated predefined caches have no repoId in
    /// the subscription world and are unreachable). The network fetch is
    /// inherent to the face (no preview arm — the A4 add-remote law). SUCCESS
    /// ANSWERS [`RepoRefreshOutcomeV01`] — the library's own two-arm outcome:
    /// cache written, or etag unchanged ("already up to date"); BOTH arms are
    /// success and the fact travels to the wire receipt as the REQUIRED
    /// `cacheUpdated` fact — "no new data" is a refresh outcome, never an
    /// error. Error faces: the `enable_repo` reused-code law plus the REUSED
    /// `vua.vpm.repo_fetch_failed` for the network segment.
    fn refresh_repo(&self, _repo_id: &str) -> Result<RepoRefreshOutcomeV01, AppErrorV1> {
        Err(unsupported("refresh_repo"))
    }
    /// P2 (proposal 025 freeze batch): the repository subscription list —
    /// the subscription face is the world (settings userRepos projected
    /// verbatim, array order preserved), each row carrying the REQUIRED
    /// per-repo cache-hit fact. Read-only: the add/remove write faces are
    /// frozen as proposal 026 A4 (`add_remote_repo` / `add_local_repo` /
    /// `remove_repo` + `repo_write_capabilities`); the enable/disable and
    /// manual-refresh write faces are frozen as proposal 027 F4
    /// (`enable_repo` / `disable_repo` / `refresh_repo` +
    /// `repo_lifecycle_capabilities`, packages-ops v0.6 — the W25 evidence
    /// record ruling (c) settled the enable state as VUA-owned semantics);
    /// their read-back state bit rides the v0.2 word face
    /// (`repos_v02` / `list_repos_v02`).
    fn list_repos(&self) -> Result<Vec<RepoInfoV01>, AppErrorV1> {
        Err(unsupported("list_repos"))
    }
    /// F4 (proposal 027 freeze batch, packages-repos v0.2): declaration that
    /// this backend serves the subscription-list read face at the v0.2 word
    /// face (rows carry the REQUIRED VUA-owned `enabled` state bit). The
    /// default is false — the frozen v0.1 word face keeps being served; a
    /// backend overrides this exactly when it implements `list_repos_v02`
    /// (ORC-DEV-004: no implementation, no reservation; the additive
    /// dual-version negotiation law is the `catalog_v02` / `query_v02`
    /// precedent).
    fn repos_v02(&self) -> bool {
        false
    }
    /// F4 (proposal 027 freeze batch, packages-repos v0.2): the subscription
    /// list at the v0.2 word face — the frozen v0.1 five-key projection plus
    /// the REQUIRED `enabled` state bit per row ([`RepoInfoV02`]; id-absent
    /// rows project `enabled: true` always — they are outside the toggle
    /// faces' reach). Backends keep serving v0.1 through `list_repos` until
    /// they adopt this; the wire route negotiates the family version by
    /// `repos_v02` (the stamped family const tells the consumer which word
    /// face answered, never a guess).
    fn list_repos_v02(&self) -> Result<Vec<RepoInfoV02>, AppErrorV1> {
        Err(unsupported("list_repos_v02"))
    }
    /// P2 (proposal 025 freeze batch): per-package catalog facts for one
    /// package in one registered project's context. On-demand granularity
    /// only — no full-catalog projection, no pagination. An unknown
    /// package (in neither repository caches nor the local set) answers
    /// `vua.vpm.no_matching_package` (reused code, same fact); the
    /// compatible judgment binds to this project's Unity version.
    fn package_catalog(
        &self,
        _project: &ProjectRef,
        _package_id: &str,
    ) -> Result<PackageCatalogV01, AppErrorV1> {
        Err(unsupported("package_catalog"))
    }
    /// P2 v0.2 increment (proposal 025 inline ruling): declaration that
    /// this backend serves the catalog face at the v0.2 word face (the
    /// result carries the `cacheSourced` disclosure). The default is
    /// false — the frozen v0.1 word face stays served; a backend overrides
    /// this exactly when it implements `package_catalog_v02`
    /// (ORC-DEV-004: no implementation, no reservation).
    fn catalog_v02(&self) -> bool {
        false
    }
    /// P2 v0.2 increment (proposal 025 inline ruling): the catalog facts
    /// at the v0.2 word face — same shape as `package_catalog` plus the
    /// REQUIRED `cache_sourced` disclosure fact. Backends keep serving
    /// v0.1 through `package_catalog` until they adopt this; the wire
    /// route negotiates the family version by `catalog_v02`.
    fn package_catalog_v02(
        &self,
        _project: &ProjectRef,
        _package_id: &str,
    ) -> Result<PackageCatalogV02, AppErrorV1> {
        Err(unsupported("package_catalog_v02"))
    }
    /// F2 (proposal 027 freeze batch): the per-repository installable-package
    /// inventory over the backend collection's cache — package discovery
    /// facts (packageId / displayName / latestVersion / versionCount /
    /// description within the library's field ceiling; `author` deliberately
    /// absent, environment verification 3bd4f12 s1(b)). The world is the
    /// COLLECTION's repository set (predefined official/curated unless
    /// ignored + subscribed user repos), grouped per repository, never a
    /// cross-repository merge; rows project the collection's own enumeration
    /// order verbatim. `repo_id` scopes the answer to one repository row
    /// (None = all); an unknown id answers the REUSED
    /// `vua.vpm.repo_not_found` (the A4 removeRepo same-fact precedent).
    /// `package_ids` is the batch requirement-set filter (user ruling 4:
    /// Recipe automation is this face's first consumer; empty slice = no
    /// filter) — a filter that matches nothing is an HONEST EMPTY answer
    /// (empty packages arrays / empty repos), never an error:
    /// `no_matching_package` has no reach on this face. Read-only:
    /// enable/disable and manual refresh are packages-ops (F4) write faces
    /// and do not exist here.
    fn repo_catalog(
        &self,
        _repo_id: Option<&str>,
        _package_ids: &[String],
    ) -> Result<RepoCatalogV01, AppErrorV1> {
        Err(unsupported("repo_catalog"))
    }
    /// F3 (proposal 027 freeze batch, packages-query v0.2): declaration that
    /// this backend serves the installed-set read face at the v0.2 word face
    /// (rows carry the latestVersion/updateAvailable judgment facts; the
    /// listing carries the REQUIRED cacheSourced disclosure). The default is
    /// false — the frozen v0.1 word face keeps being served; a backend
    /// overrides this exactly when it implements `list_packages_v02`
    /// (ORC-DEV-004: no implementation, no reservation; the additive
    /// dual-version negotiation law is the `catalog_v02` precedent).
    fn query_v02(&self) -> bool {
        false
    }
    /// F3 (proposal 027 freeze batch, packages-query v0.2): the installed
    /// set at the v0.2 word face — the frozen v0.1 manifest+lock projection
    /// facts plus the per-row judgment facts ([`InstalledPackageV02`]) and
    /// the REQUIRED `cache_sourced` disclosure. Backends keep serving v0.1
    /// through `list_packages` until they adopt this; the wire route
    /// negotiates the family version by `query_v02` (the stamped family
    /// const tells the consumer which word face answered, never a guess).
    /// The judgment reuses the packages-catalog frozen selector semantics
    /// over ONE collection load for the whole table (never per-row
    /// reloads); the error face is the frozen v0.1 face, zero new codes.
    fn list_packages_v02(&self, _project: &ProjectRef) -> Result<InstalledListingV02, AppErrorV1> {
        Err(unsupported("list_packages_v02"))
    }
    /// F5 (proposal 027 freeze batch): capability declaration for the
    /// template-enumeration read face. The default is declared-none; a
    /// backend overrides it exactly when it implements `list_templates`
    /// (the 025 accessor law — the VrcGetLib override lands with the
    /// environment implementation-verification slice, the same
    /// honest-absence discipline: the served wire row stays unavailable
    /// until the override flips it).
    fn template_capabilities(&self) -> TemplateCapabilities {
        TemplateCapabilities::NONE
    }
    /// F5 (proposal 027 freeze batch): the available template entries under
    /// the two pinned directory roots of the library-path default
    /// resolution leg (`<environment_root>/VRCTemplates` first, then
    /// `<environment_root>/Templates` — the create_from_template
    /// resolution order; the explicit-path leg is a per-create argument
    /// shape, NOT a directory root, and has no reach on this face). The
    /// enumeration is a LOCAL DIRECTORY SCAN (vrc-get-vpm 0.0.16 ships no
    /// template enumeration API — environment verification 027 s4), a
    /// zero-network face: NO cacheSourced disclosure exists here (a
    /// constant informational field is not a fact, the packages-repos
    /// v0.1 law). A name present under both roots enumerates ONCE,
    /// resolved to the root the creation resolution order would pick —
    /// enumeration never diverges from what create would copy. Rows are
    /// id-ascending (the frozen presentation fact); an EMPTY vec is the
    /// honest zero-templates answer (a missing root or an empty pair of
    /// roots is a fact, never an error — the R4 precedent). Zero new
    /// error codes: the error face is the standing envelope set.
    fn list_templates(&self) -> Result<Vec<TemplateEntryV01>, AppErrorV1> {
        Err(unsupported("list_templates"))
    }
    /// A5 (proposal 026 freeze batch, packages-ops v0.5): creates a project
    /// from a template. REQUIRED method (no default body): a backend without
    /// the creation capability declares it honestly through
    /// `capabilities().create_project == false` and the wire gate answers
    /// `capability_missing` BEFORE submit — capability absence never reaches
    /// a task. THE PORT HAS NO CREATE-PREVIEW COUNTERPART (the A5 word face
    /// is the second no-preview-pair member, rooted here): a brand-new
    /// project directory has no pre-existing state to diff, so no digest
    /// binds and the user's explicit form submission IS the confirmation.
    /// `template: None` = the backend's default template resolution (the
    /// library path: the Avatar template, VRCTemplates/<t> ->
    /// Templates/<t> -> explicit-path order — a frozen word-face FACT, not
    /// a picker: the first face has zero new read faces). SUCCESS ANSWERS
    /// `ProjectRef` — the one packages-ops face with an actual-result
    /// payload — AND REGISTERS THE NEW PROJECT IN VUA's in-store project
    /// storage (the `FileSystemProjectStore::initialize` tail call on both
    /// backends' success paths): a created project IS a registered project.
    /// Refusal faces are backend-honest and DIVERGE by backend (the wire
    /// word face folds them all into `execution_failed` carrying the
    /// original code in detail; the protocol document declares the
    /// difference): the library path answers `vua.vpm.template_missing`
    /// for all four refusal keys (projectExists / projectNameInvalid /
    /// templateMissing / templateCopyFailed — the i18n message key and the
    /// port code are two layers); the CLI path answers
    /// `vua.vpm.apply_failed` (timeout / non-zero exit, carrying the
    /// exitCode) and `vua.vpm.backend_unavailable` (runner-spawn failure).
    fn create_project(
        &self,
        parent: &Path,
        name: &str,
        template: Option<&str>,
    ) -> Result<ProjectRef, AppErrorV1>;
    /// Batch 146 (user ruling 2026-09-21: SDK import before real-machine
    /// acceptance): resolve the dependencies the project's
    /// `Packages/vpm-manifest.json` DECLARES — read the dependencies, resolve
    /// them from the ENABLED repositories (the disabled-set semantics are the
    /// F4 collection-world law: a disabled subscription row never serves
    /// resolution), install the package bodies into `Packages/` and write the
    /// locked section back. Idempotent: a project whose locked/declared
    /// requirements are already satisfied answers them via
    /// `already_satisfied` and touches nothing. The network segment is
    /// inherent to the face (fetching the package bodies), so there is NO
    /// preview arm — the A4 add-remote law. Zero new error codes: the error
    /// face reuses the standing family (`repo_fetch_failed` for the network
    /// segment, `repo_not_found` / `backend_unavailable` for the collection
    /// and state legs, `no_matching_package` for a dependency no enabled
    /// repository satisfies, `apply_failed` for the install/manifest-write
    /// leg). The trait default is the DECLARED-NONE absence arm: a backend
    /// that has not grown this face answers the capability_missing family
    /// (the `unsupported` arm — the same law as `register_local_package`),
    /// and `capabilities().resolve_project == false` states that honestly;
    /// the VrcGetLib backend overrides both, the VCC CLI backend stays
    /// honestly absent.
    fn resolve_project(&self, _project_root: &Path) -> Result<ResolveReceiptV01, AppErrorV1> {
        Err(unsupported("resolve_project"))
    }
}

fn unsupported(capability: &str) -> AppErrorV1 {
    AppErrorV1::new(
        error_codes::CAPABILITY_MISSING,
        ErrorCategory::Unavailable,
        "errors.vpm.capabilityMissing",
        "corr-vpm-backend",
    )
    .with_param("capability", ParamValue::Text(capability.to_owned()))
}
