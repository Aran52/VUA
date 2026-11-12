//! The bdl-queries v0.5 dependencies read face IMPLEMENTATION: the real
//! query executor reading the AMF-private BDL library (the v0.2 store
//! surface: `dependency_observations` + `products`) behind the core
//! `DependenciesQueriesPort`. Placement follows the
//! `OnDiskProjectDraftExporter` precedent — the implementing adapter lives
//! beside the port trait in the core crate and is wired at the provider
//! assembly point; until an instance overrides the defaulted capability
//! accessor, the served row and both routes answer honestly unavailable
//! (ORC-DEV-004: no implementation, no reservation).
//!
//! The rule tables are the FROZEN v0.5 read-time law
//! (docs/protocols/bdl-queries-v0.5):
//!
//! - **Matching rule v1**: ASCII case-fold EXACT over the stored
//!   `dep_name` (non-ASCII nominals reduce to verbatim equality —
//!   `to_ascii_lowercase` leaves them untouched). No substring, no fuzzy,
//!   no equivalence: an input that is not literally present (under case
//!   fold) returns the HONEST EMPTY SET — a success fact (`total: 0`),
//!   never an error, never a guess. Storage is never rewritten: matching
//!   is not normalization.
//! - **Advisory rule v1 (the double gate)**: an advisory exists if and only
//!   if the layout is a deliberate declaration (`extraction_method ∈
//!   {explicit_heading, one_line, bullet}`) AND the resolution is
//!   human-confirmed (`confirmed_by_human = 1`; an unconfirmed resolution
//!   is a clue and never turns into a suggestion). `installSource`
//!   derives from the RESOLUTION TARGET's source host — a booth.pm host =
//!   `booth_page`, any other host = `external_page`; `vpm`/`unknown` stay
//!   in the frozen enum but v1 never emits them (claiming vpm without a
//!   VPM-repo reconciliation fact would be a guess). `confidence` over the
//!   layout dimension only: `strong` = explicit_heading/one_line, `weak` =
//!   bullet.
//! - **Clues-not-conclusions (the two-face contrast)**: lookup surfaces
//!   `resolvedProductId` ONLY for confirmed resolutions (null = no
//!   resolution or an unconfirmed one — no distinction revealed);
//!   `listByProduct` lists every observation as-is with `confirmed`
//!   labeled (unconfirmed clues included, tombstoned declaring products
//!   included). Both faces read the one library — the contrast is the
//!   honesty.
//!
//! Ordering and pagination are the frozen determinism law: lookup rows
//! sort by productId ascending then observation identity ascending, total
//! computed before pagination; listByProduct rows sort by observation
//! identity (insertion order). Failure mapping: a store read that cannot
//! be served answers the family's registered honest-absence code (zero
//! new codes — the family carries no internal-error channel); the
//! executor never fabricates a row to keep an answer shaped.

use std::collections::HashMap;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use vua_bdl_store::{
    availability_status, BdlStore, BdlStoreError, DependencyProductRow, StoredDependencyObservation,
};

use crate::contracts::{AppErrorV1, ErrorCategory};
use crate::dependencies_queries::{
    AdvisoryConfidence, DependenciesListByProductResultV05, DependenciesLookupParams,
    DependenciesLookupResultV05, DependenciesQueriesCapabilities, DependenciesQueriesPort,
    DependencyMatchV05, DependencyObservationV05, DependencyProductStatus, InstallAdvisoryV05,
    InstallSource, ResolutionEvidenceV05, ResolutionV05,
};

/// The real dependencies read-model executor over one opened BDL library.
/// Read-only: both port methods issue no write, no network, no tasked face.
pub struct BdlDependencyQueries {
    bdl: Arc<BdlStore>,
}

impl BdlDependencyQueries {
    /// Serves the dependencies face over the given library (the same store
    /// the warehouse/downloads faces read — one library, one source of
    /// truth).
    pub fn new(bdl: Arc<BdlStore>) -> Self {
        Self { bdl }
    }

    /// The declaring/resolution product row with a per-call cache (lookup
    /// fans out over rows of few products; the store query is the same
    /// honest read either way).
    fn product_row(
        &self,
        product_id: &str,
        cache: &mut HashMap<String, Option<DependencyProductRow>>,
    ) -> Result<Option<DependencyProductRow>, AppErrorV1> {
        if let Some(row) = cache.get(product_id) {
            return Ok(row.clone());
        }
        let row = self
            .bdl
            .dependency_product_row(product_id)
            .map_err(|error| store_failure(&error))?;
        cache.insert(product_id.to_owned(), row.clone());
        Ok(row)
    }

    /// Advisory rule v1 installSource: the RESOLUTION TARGET's source host
    /// — a booth.pm host = booth_page, any other host (or no parsable
    /// host) = external_page. `vpm`/`unknown` are never emitted.
    fn install_source(
        &self,
        target_product_id: &str,
        cache: &mut HashMap<String, Option<DependencyProductRow>>,
    ) -> Result<InstallSource, AppErrorV1> {
        let row = self.product_row(target_product_id, cache)?;
        let host = row
            .as_ref()
            .and_then(|product| product.source_url.as_deref())
            .and_then(url_host);
        Ok(match host {
            Some(host) if host == "booth.pm" || host.ends_with(".booth.pm") => {
                InstallSource::BoothPage
            }
            _ => InstallSource::ExternalPage,
        })
    }

    /// One lookup match row (the suggestion face): evidence body verbatim,
    /// declaring product's presentation facts, and the confirmed-only
    /// resolution/advisory gates.
    fn as_match(
        &self,
        row: &StoredDependencyObservation,
        cache: &mut HashMap<String, Option<DependencyProductRow>>,
    ) -> Result<DependencyMatchV05, AppErrorV1> {
        let declaring = self.product_row(&row.product_id, cache)?;
        // The resolution gate: resolvedProductId + advisory surface ONLY
        // for human-confirmed resolutions. null = no resolution, or an
        // unconfirmed one — no distinction is revealed (the admission law).
        let (resolved_product_id, advisory) = match (
            row.confirmed_by_human,
            row.resolved_ref_product_id.as_deref(),
        ) {
            (true, Some(target)) => {
                // Advisory gate 1 (the deliberate-declaration layout):
                // prose/title/link forms are incidental mentions, below
                // the suggestion line, even when the resolution is
                // confirmed.
                let advisory = if deliberate_declaration(&row.extraction_method) {
                    Some(InstallAdvisoryV05 {
                        install_source: self.install_source(target, cache)?,
                        confidence: confidence(&row.extraction_method),
                    })
                } else {
                    None
                };
                (Some(target.to_owned()), advisory)
            }
            _ => (None, None),
        };
        Ok(DependencyMatchV05 {
            product_id: row.product_id.clone(),
            product_title: declaring.as_ref().and_then(|product| product.title.clone()),
            availability_raw: declaring
                .as_ref()
                .and_then(|product| product.availability.clone()),
            availability_status: availability_status(
                declaring
                    .as_ref()
                    .and_then(|product| product.availability.as_deref()),
            ),
            dep_kind: closed_set_word(&row.dep_kind)?,
            dep_name: row.dep_name.clone(),
            version_hint: row.version_hint.clone(),
            raw_quote: row.raw_quote.clone(),
            source_span: closed_set_word(&row.source_span)?,
            extraction_method: closed_set_word(&row.extraction_method)?,
            resolved_product_id,
            advisory,
        })
    }

    /// One listByProduct observation row (the clue face): carried WHOLE —
    /// the confirmation workflow's extra keys (extractedBy/observedAt) and
    /// the resolution with its honest confirmation label; no advisory here
    /// (the suggestion derivation is lookup's job).
    fn as_observation(
        row: &StoredDependencyObservation,
    ) -> Result<DependencyObservationV05, AppErrorV1> {
        let resolution = row.resolved_ref_product_id.as_ref().map(|target| {
            let evidence = row
                .resolution_evidence
                .as_ref()
                .map(|items| {
                    items
                        .iter()
                        .map(|item| {
                            Ok(ResolutionEvidenceV05 {
                                link_text: item.link_text.clone(),
                                link_url: item.link_url.clone(),
                                span: closed_set_word(&item.span)?,
                                note: item.note.clone(),
                            })
                        })
                        .collect::<Result<Vec<_>, AppErrorV1>>()
                })
                // The store CHECK demands evidence with resolution; the
                // honest projection of the impossible shape is the empty
                // set, never a fabricated item.
                .unwrap_or_else(|| Ok(Vec::new()))?;
            Ok::<ResolutionV05, AppErrorV1>(ResolutionV05 {
                product_id: target.clone(),
                confirmed: row.confirmed_by_human,
                evidence,
            })
        });
        let resolution = resolution.transpose()?;
        Ok(DependencyObservationV05 {
            dep_kind: closed_set_word(&row.dep_kind)?,
            dep_name: row.dep_name.clone(),
            version_hint: row.version_hint.clone(),
            raw_quote: row.raw_quote.clone(),
            source_span: closed_set_word(&row.source_span)?,
            extraction_method: closed_set_word(&row.extraction_method)?,
            extracted_by: row.extracted_by.clone(),
            observed_at: row.observed_at.clone(),
            resolution,
        })
    }
}

impl DependenciesQueriesPort for BdlDependencyQueries {
    /// The implementation FLIP (the recipe-export loop-3 precedent): the
    /// executor exists, so the served capability row and both route gates
    /// turn available.
    fn dependencies_capabilities(&self) -> DependenciesQueriesCapabilities {
        DependenciesQueriesCapabilities {
            dependencies_queries: true,
        }
    }

    /// The suggestion face: dependency reverse lookup under matching rule
    /// v1 over the whole library, then the advisory rule v1 derivation.
    fn dependencies_lookup(
        &self,
        params: &DependenciesLookupParams,
    ) -> Result<DependenciesLookupResultV05, AppErrorV1> {
        let rows = self
            .bdl
            .dependency_observations_all()
            .map_err(|error| store_failure(&error))?;
        // Matching rule v1: ASCII case-fold EXACT (non-ASCII words are
        // untouched by the fold and compare verbatim); the stored nominal
        // is never rewritten.
        let needle = params.name.to_ascii_lowercase();
        let mut matched: Vec<&StoredDependencyObservation> = rows
            .iter()
            .filter(|row| row.dep_name.to_ascii_lowercase() == needle)
            .filter(|row| match params.dep_kind {
                Some(expected) => {
                    closed_set_name(&expected).as_deref() == Some(row.dep_kind.as_str())
                }
                None => true,
            })
            .collect();
        // The frozen determinism law: productId ascending, then observation
        // identity ascending; total BEFORE pagination.
        matched.sort_by(|left, right| {
            left.product_id
                .cmp(&right.product_id)
                .then_with(|| left.observation_id.cmp(&right.observation_id))
        });
        let total = matched.len() as i64;
        let mut cache = HashMap::new();
        let matches = matched
            .iter()
            .skip(usize::try_from(params.offset).unwrap_or(usize::MAX))
            .take(usize::try_from(params.limit).unwrap_or(0))
            .map(|row| self.as_match(row, &mut cache))
            .collect::<Result<Vec<_>, AppErrorV1>>()?;
        Ok(DependenciesLookupResultV05 { total, matches })
    }

    /// The clue face: all dependency observations of one product,
    /// unfiltered, tombstoned declaring products included (their rows
    /// answer with `productStatus: missing` and the observations stay
    /// readable). `Ok(None)` = the unknown productId — the route maps it
    /// onto catalog.detail's not-found, never a fabricated empty answer.
    fn dependencies_list_by_product(
        &self,
        product_id: &str,
    ) -> Result<Option<DependenciesListByProductResultV05>, AppErrorV1> {
        let product = self
            .bdl
            .dependency_product_row(product_id)
            .map_err(|error| store_failure(&error))?;
        let Some(product) = product else {
            return Ok(None);
        };
        let rows = self
            .bdl
            .dependency_observations(product_id)
            .map_err(|error| store_failure(&error))?;
        let observations = rows
            .iter()
            .map(Self::as_observation)
            .collect::<Result<Vec<_>, AppErrorV1>>()?;
        Ok(Some(DependenciesListByProductResultV05 {
            product_id: product.product_id.clone(),
            // The tombstone honesty face; the CHECK closed set is
            // complete|missing, so the residual arm IS the complete case.
            // Constraint authority: the EXECUTABLE migration chain
            // (schemas/bdl/v0.1/001_initial.sql products DDL carries
            // `CHECK (status IN ('complete','missing'))`; the runtime
            // database is born from 001+002). The v0.2 restatement
            // (schemas/bdl/v0.2/schema.sql) shows this column's closed
            // set as a comment only — the test pin
            // product_status_maps_both_legal_words_and_the_closed_set_is_
            // database_enforced holds the executable CHECK against silent
            // loss in a future table rebuild (the 002
            // compatibility_observations rebuild precedent).
            product_status: if product.status == "missing" {
                DependencyProductStatus::Missing
            } else {
                DependencyProductStatus::Complete
            },
            observations,
        }))
    }
}

/// The single failure mapping: a BDL store read that cannot be served
/// answers the family's registered honest-absence code (zero new codes —
/// the family carries no internal channel); the executor never fabricates
/// a result to keep an answer shaped.
fn store_failure(_error: &BdlStoreError) -> AppErrorV1 {
    AppErrorV1::new(
        "vua.catalog.unavailable",
        ErrorCategory::Unavailable,
        "errors.catalog.unavailable",
        "corr-dependencies-queries-executor",
    )
}

/// Advisory gate 1: the deliberate-declaration layouts
/// (`explicit_heading`/`one_line`/`bullet`); prose/title/link are
/// incidental mentions. The words are the frozen rule table's law, applied
/// at read time over the verbatim stored column — the store keeps no
/// duplicate Rust closed set, and this is not one either: it is the rule.
fn deliberate_declaration(extraction_method: &str) -> bool {
    matches!(
        extraction_method,
        "explicit_heading" | "one_line" | "bullet"
    )
}

/// The confidence tiers over the layout dimension only: strong =
/// explicit_heading/one_line (a dedicated heading or one-line statement),
/// weak = bullet (a listed line — real but compressed).
fn confidence(extraction_method: &str) -> AdvisoryConfidence {
    if extraction_method == "bullet" {
        AdvisoryConfidence::Weak
    } else {
        AdvisoryConfidence::Strong
    }
}

/// Parses a stored closed-set word back into its frozen enum. The v0.2
/// CHECK constraints are the single authority for these sets, so a foreign
/// word would be store-face drift, never a guessed member; it surfaces as
/// the family's service code (zero new codes).
fn closed_set_word<T: DeserializeOwned>(word: &str) -> Result<T, AppErrorV1> {
    serde_json::from_value::<T>(Value::String(word.to_owned())).map_err(|_| {
        store_failure(&BdlStoreError::CorruptValue {
            field: "closed_set_word",
            value: word.to_owned(),
        })
    })
}

/// The closed-set enum's frozen wire word (serde snake_case) for matching
/// a params enum against the stored verbatim column.
fn closed_set_name<T: Serialize>(value: &T) -> Option<String> {
    match serde_json::to_value(value) {
        Ok(Value::String(word)) => Some(word),
        _ => None,
    }
}

/// Minimal URL host extraction (no new dependency): scheme-stripped
/// authority, userinfo/port removed, ASCII-lowercased.
fn url_host(url: &str) -> Option<String> {
    let rest = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(authority);
    let host = authority
        .split_once(':')
        .map(|(host, _)| host)
        .unwrap_or(authority);
    if host.is_empty() {
        None
    } else {
        Some(host.to_ascii_lowercase())
    }
}
