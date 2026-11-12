//! The bdl-queries v0.5 dependencies read face: the core cross-domain
//! contract the wire routes `dependencies.lookup` /
//! `dependencies.listByProduct` through (proposal 030 §5.7 case A; the
//! vocabulary frozen by the wt-5 data batch 168 —
//! `schemas/bdl-queries/v0.5` + docs/protocols/bdl-queries-v0.5).
//!
//! Port-face placement follows the `ProjectDraftExportPort` /
//! `VpmBackend` law: the port trait and its typed facts live in the core,
//! the implementing adapter lands in an upstream crate (the real query
//! executor reading the BDL library is a LATER data/production-domain
//! implementation ring — until an adapter overrides the defaulted
//! capability accessor the served row and both routes stay honestly
//! unavailable; ORC-DEV-004: no implementation, no reservation).
//!
//! The typed facts mirror the FROZEN v0.5 word face exactly (the queries
//! family isomorphism — a field here that the schema does not know is a
//! contract break caught by the consumer tests): the wire closed sets ride
//! the frozen BDL v0.2 four/five/six-value enums, `availabilityStatus`
//! reuses the family's existing `vua_bdl_store::AvailabilityStatus` (zero
//! duplication of a stable enum), and the params closed sets carry
//! `from_value` parsers isomorphic with `CatalogListParams` (unknown keys
//! and out-of-vocabulary values are contract errors, never silently empty
//! answers). Matching rule v1 / advisory rule v1 are READ-TIME rules of
//! the implementing executor — this face carries zero matching logic, it
//! is the typed carriage only.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::contracts::{AppErrorV1, ErrorCategory};

/// The frozen BDL v0.2 `dep_kind` four-value closed set (no fifth member;
/// engine/SDK pins are stored as `other` with the pin in `versionHint`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    Shader,
    ToolPackage,
    AvatarBase,
    Other,
}

/// The frozen BDL v0.2 `source_span` five-value closed set (where on the
/// page the quote was lifted from).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSpan {
    Body,
    SubproductName,
    Image,
    Title,
    DescriptionLink,
}

/// The frozen BDL v0.2 `extraction_method` six-value closed set (the
/// layout fact of the declaration).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMethod {
    ExplicitHeading,
    Bullet,
    OneLine,
    Prose,
    Title,
    Link,
}

/// The tombstone honesty face (BDL v0.2 products.status projection):
/// `missing` = the source page died (404/410 kept, never deleted) — the
/// observations stay listed and readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyProductStatus {
    Complete,
    Missing,
}

/// The frozen `installSource` enum. Rule v1 emits ONLY `booth_page` /
/// `external_page` (derived from the confirmed resolution target's source
/// host); `vpm` and `unknown` stay in the frozen closed set but v1 never
/// emits them — claiming vpm without a VPM-repo reconciliation fact would
/// be a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallSource {
    Vpm,
    BoothPage,
    ExternalPage,
    Unknown,
}

/// The advisory confidence tiers over the layout dimension only: `strong`
/// = explicit_heading/one_line (the author declares in a dedicated heading
/// or one-line statement), `weak` = bullet (a listed line, real but
/// compressed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryConfidence {
    Strong,
    Weak,
}

/// The frozen evidence four-key shape (BDL v0.2 verbatim): one resolvable
/// link clue the declaration carried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolutionEvidenceV05 {
    pub link_text: String,
    pub link_url: String,
    pub span: SourceSpan,
    pub note: Option<String>,
}

/// The resolution fact of one observation: the linked product plus its
/// HONEST confirmation state. A `confirmed: false` resolution is a labeled
/// clue, never a suggestion (the confirmation flip is the production
/// build slice's explicit write face — it never happens automatically).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolutionV05 {
    pub product_id: String,
    pub confirmed: bool,
    pub evidence: Vec<ResolutionEvidenceV05>,
}

/// One dependency observation, carried WHOLE (the clue face row): the
/// evidence keys (`depKind`/`depName`/`versionHint`/`rawQuote`/
/// `sourceSpan`/`extractionMethod`) plus `extractedBy` (the extractor
/// identity — the confirmation workflow must know who extracted) plus
/// `observedAt` (the confirmer judges freshness) plus the resolution.
/// NO advisory here — the suggestion derivation is lookup's job; this
/// face lists observations as they are, low-confidence rows included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyObservationV05 {
    pub dep_kind: DependencyKind,
    pub dep_name: String,
    pub version_hint: Option<String>,
    pub raw_quote: String,
    pub source_span: SourceSpan,
    pub extraction_method: ExtractionMethod,
    pub extracted_by: String,
    pub observed_at: String,
    pub resolution: Option<ResolutionV05>,
}

/// `dependencies.listByProduct` result: one product, all its dependency
/// observations — the unfiltered clue face.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependenciesListByProductResultV05 {
    pub product_id: String,
    pub product_status: DependencyProductStatus,
    pub observations: Vec<DependencyObservationV05>,
}

/// The advisory (the SUGGESTION with evidence, never a fact claim): an
/// observation carries one if and only if the layout is a deliberate
/// declaration AND the install source is provable (a human-confirmed
/// resolution — unconfirmed resolutions are clues and never turn into
/// suggestions).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallAdvisoryV05 {
    pub install_source: InstallSource,
    pub confidence: AdvisoryConfidence,
}

/// One lookup match (the suggestion-face row): the evidence body plus the
/// declaring product's identity/presentation facts plus — ONLY for
/// human-confirmed resolutions — the resolved product id and the advisory.
/// Deliberate omissions (the admission rule): no `extractedBy`, no
/// `observedAt`, no paths, no tombstone filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyMatchV05 {
    pub product_id: String,
    pub product_title: Option<String>,
    pub availability_raw: Option<String>,
    pub availability_status: vua_bdl_store::AvailabilityStatus,
    pub dep_kind: DependencyKind,
    pub dep_name: String,
    pub version_hint: Option<String>,
    pub raw_quote: String,
    pub source_span: SourceSpan,
    pub extraction_method: ExtractionMethod,
    pub resolved_product_id: Option<String>,
    pub advisory: Option<InstallAdvisoryV05>,
}

/// `dependencies.lookup` result: `{ total, matches }` — total computed
/// before pagination (the catalog.list law). `total: 0 + matches: []` =
/// "no matching nominal under the current rule table", NOT "no such
/// dependency" — the empty state is a SUCCESS fact, never a not-found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependenciesLookupResultV05 {
    pub total: i64,
    pub matches: Vec<DependencyMatchV05>,
}

/// `dependencies.lookup` closed param set `{ name, depKind, limit, offset }`:
/// `name` required non-empty (the dependency nominal as the caller holds
/// it — never normalized into storage); `depKind` rides the frozen BDL
/// v0.2 four-value set (null/absent = no filtering); limit 1–200 default
/// 50 / offset default 0 per the catalog.list pagination law. Unknown
/// keys and out-of-vocabulary values are contract errors (the negative
/// vectors pin it, including a `fuzzy` key: this vocabulary deliberately
/// carries no fuzzy or equivalence switch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependenciesLookupParams {
    pub name: String,
    pub dep_kind: Option<DependencyKind>,
    pub limit: i64,
    pub offset: i64,
}

/// The params closed-set parse errors (the `CatalogParamsError` isomorph).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependenciesParamsError {
    UnknownKey(String),
    InvalidValue { key: String, reason: String },
}

impl DependenciesLookupParams {
    pub fn from_value(value: &Value) -> Result<Self, DependenciesParamsError> {
        const KEYS: [&str; 4] = ["name", "depKind", "limit", "offset"];
        let object = value
            .as_object()
            .ok_or_else(|| DependenciesParamsError::InvalidValue {
                key: "params".into(),
                reason: "expected an object".into(),
            })?;
        for key in object.keys() {
            if !KEYS.contains(&key.as_str()) {
                return Err(DependenciesParamsError::UnknownKey(key.clone()));
            }
        }
        let name = match object.get("name") {
            Some(Value::String(raw)) if !raw.is_empty() => raw.clone(),
            Some(Value::String(_)) => {
                return Err(DependenciesParamsError::InvalidValue {
                    key: "name".into(),
                    reason: "minLength 1".into(),
                });
            }
            None | Some(Value::Null) => {
                return Err(DependenciesParamsError::InvalidValue {
                    key: "name".into(),
                    reason: "required".into(),
                });
            }
            Some(_) => {
                return Err(DependenciesParamsError::InvalidValue {
                    key: "name".into(),
                    reason: "expected a string".into(),
                });
            }
        };
        let dep_kind = match object.get("depKind") {
            None | Some(Value::Null) => None,
            Some(Value::String(raw)) => {
                let parsed = serde_json::from_value::<DependencyKind>(Value::String(raw.clone()))
                    .map_err(|_| DependenciesParamsError::InvalidValue {
                    key: "depKind".into(),
                    reason: format!("{raw} is outside the frozen BDL v0.2 closed set"),
                })?;
                Some(parsed)
            }
            Some(_) => {
                return Err(DependenciesParamsError::InvalidValue {
                    key: "depKind".into(),
                    reason: "expected a string or null".into(),
                });
            }
        };
        let limit = match object.get("limit") {
            None => 50,
            Some(Value::Null) => 50,
            Some(Value::Number(raw)) => {
                let raw = raw.as_i64().ok_or(DependenciesParamsError::InvalidValue {
                    key: "limit".into(),
                    reason: "expected an integer".into(),
                })?;
                if !(1..=200).contains(&raw) {
                    return Err(DependenciesParamsError::InvalidValue {
                        key: "limit".into(),
                        reason: "out of range 1–200".into(),
                    });
                }
                raw
            }
            Some(_) => {
                return Err(DependenciesParamsError::InvalidValue {
                    key: "limit".into(),
                    reason: "expected an integer".into(),
                });
            }
        };
        let offset = match object.get("offset") {
            None => 0,
            Some(Value::Null) => 0,
            Some(Value::Number(raw)) => {
                let raw = raw.as_i64().ok_or(DependenciesParamsError::InvalidValue {
                    key: "offset".into(),
                    reason: "expected an integer".into(),
                })?;
                if raw < 0 {
                    return Err(DependenciesParamsError::InvalidValue {
                        key: "offset".into(),
                        reason: "out of range (minimum 0)".into(),
                    });
                }
                raw
            }
            Some(_) => {
                return Err(DependenciesParamsError::InvalidValue {
                    key: "offset".into(),
                    reason: "expected an integer".into(),
                });
            }
        };
        Ok(Self {
            name,
            dep_kind,
            limit,
            offset,
        })
    }
}

/// `dependencies.listByProduct` closed param set: exactly `{ productId }`,
/// the catalog.detail namespaced pattern. No name/filter keys exist — a
/// client-supplied filter is a contract error, never a silently empty
/// answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependenciesListByProductParams {
    pub product_id: String,
}

impl DependenciesListByProductParams {
    pub fn from_value(value: &Value) -> Result<Self, DependenciesParamsError> {
        let object = value
            .as_object()
            .ok_or_else(|| DependenciesParamsError::InvalidValue {
                key: "params".into(),
                reason: "expected an object".into(),
            })?;
        for key in object.keys() {
            if key != "productId" {
                return Err(DependenciesParamsError::UnknownKey(key.clone()));
            }
        }
        let product_id = match object.get("productId") {
            Some(Value::String(raw)) => raw.clone(),
            _ => {
                return Err(DependenciesParamsError::InvalidValue {
                    key: "productId".into(),
                    reason: "required, a namespaced string".into(),
                });
            }
        };
        if product_id.is_empty() {
            return Err(DependenciesParamsError::InvalidValue {
                key: "productId".into(),
                reason: "minLength 1".into(),
            });
        }
        Self::validate_product_id(&product_id)?;
        Ok(Self { product_id })
    }

    /// The frozen `^booth:[0-9]+$` pattern (the catalog.detail namespaced
    /// corpus identity).
    fn validate_product_id(product_id: &str) -> Result<(), DependenciesParamsError> {
        let digits = product_id.strip_prefix("booth:").ok_or_else(|| {
            DependenciesParamsError::InvalidValue {
                key: "productId".into(),
                reason: "must match ^booth:[0-9]+$".into(),
            }
        })?;
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(DependenciesParamsError::InvalidValue {
                key: "productId".into(),
                reason: "must match ^booth:[0-9]+$".into(),
            });
        }
        Ok(())
    }
}

/// Capability declaration for the dependencies read face. Same shape law
/// as the `ProjectDraftExportCapabilities` accessor (the 025
/// `catalog_capabilities` law): a separate defaulted trait accessor, the
/// default declared-none (ORC-DEV-004: no implementation, no reservation)
/// — an adapter overrides it exactly when it implements the two read
/// methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependenciesQueriesCapabilities {
    /// Covers BOTH `dependencies.lookup` and `dependencies.listByProduct`:
    /// the frozen v0.5 face is one design unit (the clues-not-conclusions
    /// law needs the two-face contrast to hold), so one bit serves the one
    /// row and both route gates (the one-row-serves-n precedent).
    pub dependencies_queries: bool,
}

impl DependenciesQueriesCapabilities {
    pub const NONE: Self = Self {
        dependencies_queries: false,
    };
}

impl Default for DependenciesQueriesCapabilities {
    fn default() -> Self {
        Self::NONE
    }
}

/// The dependencies read-model port (proposal 030 §5.7 case A; the v0.5
/// freeze). Both methods are synchronous read-only: the BDL library only,
/// no network, no mutation, no tasked face (the family carries zero write
/// operations — the `confirmed_by_human` flip belongs to the production
/// build slice's explicit write face). Errors travel verbatim through the
/// route (the read-face pass-through discipline); ZERO new error codes —
/// the face reuses the bdl-queries family's existing codes
/// (`vua.catalog.unavailable` honest absence / `vua.catalog.invalid_params`
/// shape verdict / `vua.catalog.product_not_found` absence semantics the
/// protocol text itself aligns with catalog.detail).
pub trait DependenciesQueriesPort: Send + Sync {
    /// Capability declaration for the dependencies face. Default
    /// declared-none (the `export_capabilities` accessor law): the served
    /// capability row and both routes stay honestly unavailable until an
    /// implementing adapter overrides this exactly when it implements the
    /// read methods.
    fn dependencies_capabilities(&self) -> DependenciesQueriesCapabilities {
        DependenciesQueriesCapabilities::NONE
    }

    /// The suggestion face: dependency reverse lookup under the versioned
    /// matching rule table (v1 = case-insensitive EXACT over `dep_name`,
    /// ASCII case fold only — no substring, no fuzzy, no equivalence; the
    /// rules live in the IMPLEMENTING executor, never in this face). The
    /// default body answers the family's honest-absence code so a
    /// declared-but-unimplemented adapter CAN exist at the type level
    /// (the F5 structural law — the route gate answers first, the default
    /// body is the second honest layer).
    fn dependencies_lookup(
        &self,
        _params: &DependenciesLookupParams,
    ) -> Result<DependenciesLookupResultV05, AppErrorV1> {
        Err(AppErrorV1::new(
            "vua.catalog.unavailable",
            ErrorCategory::Unavailable,
            "errors.catalog.unavailable",
            "corr-dependencies-queries-port",
        ))
    }

    /// The clue face: all dependency observations of one product,
    /// unfiltered, tombstoned products included (the confirmation
    /// workflow must still see a dead page's declarations). `Ok(None)` =
    /// unknown productId — the route maps it onto the application-face
    /// not-found (the catalog.detail absence semantics), never a
    /// fabricated empty answer.
    fn dependencies_list_by_product(
        &self,
        _product_id: &str,
    ) -> Result<Option<DependenciesListByProductResultV05>, AppErrorV1> {
        Err(AppErrorV1::new(
            "vua.catalog.unavailable",
            ErrorCategory::Unavailable,
            "errors.catalog.unavailable",
            "corr-dependencies-queries-port",
        ))
    }
}
