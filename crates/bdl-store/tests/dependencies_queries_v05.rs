//! Consumer test for the FROZEN `schemas/bdl-queries/v0.5` dependency query
//! vocabulary (collab/proposals/030 §5.7 case A; data seat, batch 168) —
//! `dependencies.lookup` + `dependencies.listByProduct`.
//!
//! Contract-first split, v0.4 precedent: the data seat freezes the domain
//! vocabulary ahead of the wire — the envelope version constant
//! (`BDL_QUERIES_SCHEMA_VERSION`) and the provider-host routing rise with
//! the core wiring batch, the TS face is the desktop role's, and the store
//! v0.2 landing (dependency_observations behind user_version=2) is the
//! production seat's slice. This file therefore touches NO bdl-store code:
//! it consumes the frozen schema files and example vectors, and drives the
//! versioned rule tables (matching rule v1, advisory rule v1 — the rule
//! bodies belong to this consuming face per the frozen BDL v0.2 protocol)
//! as a REFERENCE derivation over the frozen `schemas/bdl/v0.2` migration
//! chain. What is pinned here:
//!
//! - the operation closed set is exactly the eight queries (six v0.4 + two
//!   new), and the word faces equal the FROZEN BDL v0.2 closed sets
//!   (dep_kind / source_span / extraction_method) on BOTH frozen
//!   authorities — the readable schema.sql AND the executable 002 migration
//!   chain whose CHECKs are the hard law a runtime database actually
//!   carries (data-seat batch-193 self reverse-review: the batch-190/191
//!   core-seat lesson is that a restatement can under-report the constraint,
//!   so the anchor pins both documents; productStatus likewise pins the
//!   products.status CHECK that lives only in 001_initial.sql);
//! - matching rule v1: case-insensitive EXACT over dep_name (ASCII fold),
//!   no substring, no fuzzy, package-form miss = honest empty set;
//! - the clues-not-conclusions gates on the wire: resolvedProductId
//!   surfaces ONLY for human-confirmed resolutions and advisory is null
//!   unless the versioned rule admits the row, while listByProduct lists
//!   the same clue as `confirmed:false`;
//! - the two-face contrast is mechanical: lookup rows carry no
//!   extractedBy/observedAt/resolution, observation rows carry them and no
//!   advisory; paths never appear on either face.
//!
//! Vector wording quotes the 030 §1 survey sample phrases (public page
//! free text, no paid content). Zero end-to-end claims: schema/rule-table
//! behavior over the frozen files only, never store v0.2 behavior, never a
//! live wire.

use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::PathBuf;

const MIGRATION_001: &str = include_str!("../../../schemas/bdl/v0.1/001_initial.sql");
const MIGRATION_002: &str =
    include_str!("../../../schemas/bdl/v0.2/002_dependency_observations.sql");
const AUTHORITY_V02: &str = include_str!("../../../schemas/bdl/v0.2/schema.sql");

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/bdl-queries/v0.5")
}

fn read_json(relative: &str) -> Value {
    let bytes = std::fs::read(schema_dir().join(relative)).expect("schema/vector must exist");
    serde_json::from_slice(&bytes).expect("schema/vector must be valid JSON")
}

fn query_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_json("query.schema.json")).unwrap()
}

fn result_validator() -> jsonschema::Validator {
    jsonschema::validator_for(&read_json("result.schema.json")).unwrap()
}

const LOOKUP_REQUEST: &str = "examples/dependencies-lookup.request.json";
const LOOKUP_RESULT: &str = "examples/dependencies-lookup.result.json";
const LIST_BY_PRODUCT_REQUEST: &str = "examples/dependencies-listbyproduct.request.json";
const LIST_BY_PRODUCT_RESULT: &str = "examples/dependencies-listbyproduct.result.json";
const NEGATIVES: &[&str] = &[
    "examples/invalid-dependencies-lookup-empty-name.json",
    "examples/invalid-dependencies-lookup-foreign-dep-kind.json",
    "examples/invalid-dependencies-lookup-fuzzy-param.json",
    "examples/invalid-dependencies-listbyproduct-params.json",
    "examples/invalid-schema-version.json",
];

// ---------------------------------------------------------------------------
// Frozen BDL v0.2 migration chain + synthetic seed (products + observations).
// ---------------------------------------------------------------------------

fn open_v02() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    conn.execute_batch(MIGRATION_001).unwrap();
    conn.execute_batch(MIGRATION_002).unwrap();
    // One tombstoned product (status 'missing'): the 030 §1 survey proved
    // page states change over time (sample 4 shows a page gone non-public);
    // this synthetic row pins the read-side law — observations of a dead
    // source page stay listed, and productStatus says missing. No claim is
    // made about any real page's current state.
    for (product_id, native_id, title, availability, source_url, status) in [
        (
            "booth:6584744",
            "6584744",
            Some("オリジナル3Dモデル ~ネコチヤン~"),
            Some("https://schema.org/InStock"),
            "https://booth.pm/ja/items/6584744",
            "complete",
        ),
        (
            "booth:8179865",
            "8179865",
            Some("【liltoon】機能盛り沢山！リアルなでさわシェーダー…【JP/EN】"),
            None,
            "https://booth.pm/ja/items/8179865",
            "missing",
        ),
        (
            "booth:3087170",
            "3087170",
            Some("【無料】lilToon"),
            Some("https://schema.org/InStock"),
            "https://lilxyzw.booth.pm/items/3087170",
            "complete",
        ),
        (
            "booth:4993931",
            "4993931",
            Some("Lapwing"),
            Some("https://schema.org/InStock"),
            "https://booth.pm/ja/items/4993931",
            "complete",
        ),
        (
            "booth:7547699",
            "7547699",
            Some("Midnight Hug"),
            None,
            "https://example.org/non-booth-tool-page",
            "complete",
        ),
    ] {
        conn.execute(
            "INSERT INTO products (product_id, native_product_id, source_url, status,
                 title, availability, content_hash, observed_at, processor_version)
             VALUES (?1, ?2, ?3, ?6, ?4, ?5, 'sha256:' || hex(zeroblob(32)),
                 '2026-09-22T00:00:00.000Z', 'dep-0.1')",
            rusqlite::params![
                product_id,
                native_id,
                source_url,
                title,
                availability,
                status
            ],
        )
        .unwrap();
    }
    conn
}

struct ObsSeed {
    product_id: &'static str,
    dep_kind: &'static str,
    dep_name: &'static str,
    raw_quote: &'static str,
    source_span: &'static str,
    version_hint: Option<&'static str>,
    resolved: Option<&'static str>,
    confirmed: i64,
    extraction_method: &'static str,
}

fn seed_observations(conn: &Connection, seeds: &[ObsSeed]) {
    for seed in seeds {
        let evidence = seed.resolved.map(|target| {
            json!([{
                "linkText": seed.dep_name,
                "linkUrl": format!("https://link.example/{target}"),
                "span": "description_link",
                "note": Value::Null,
            }])
            .to_string()
        });
        conn.execute(
            "INSERT INTO dependency_observations (product_id, dep_kind, dep_name, raw_quote,
                 source_span, version_hint, resolved_ref_product_id, resolution_evidence,
                 confirmed_by_human, extraction_method, extracted_by, observed_at,
                 processor_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'dep-pipeline-0.1',
                 '2026-09-22T00:00:00.000Z', 'dep-0.1')",
            rusqlite::params![
                seed.product_id,
                seed.dep_kind,
                seed.dep_name,
                seed.raw_quote,
                seed.source_span,
                seed.version_hint,
                seed.resolved,
                evidence,
                seed.confirmed,
                seed.extraction_method,
            ],
        )
        .unwrap();
    }
}

/// Deliberately out of productId order; the lookup order contract is pinned
/// below. Row 4 is the substring trap, row 5 an engine pin stored as
/// `other` + version_hint, row 6/7 exercise the non-booth host and the
/// weak-confidence layout.
fn standard_seeds() -> Vec<ObsSeed> {
    vec![
        ObsSeed {
            product_id: "booth:8179865",
            dep_kind: "shader",
            dep_name: "lilToon",
            raw_quote: "本ギミックはlilToonのカスタムパラメータとして作動します。",
            source_span: "body",
            version_hint: None,
            resolved: None,
            confirmed: 0,
            extraction_method: "prose",
        },
        ObsSeed {
            product_id: "booth:6584744",
            dep_kind: "shader",
            dep_name: "liltoon",
            raw_quote: "・liltoon 2.3.2~",
            source_span: "body",
            version_hint: Some("2.3.2~"),
            resolved: Some("booth:3087170"),
            confirmed: 1,
            extraction_method: "explicit_heading",
        },
        ObsSeed {
            product_id: "booth:6584744",
            dep_kind: "avatar_base",
            dep_name: "Lapwing",
            raw_quote: "◎Liltoon",
            source_span: "description_link",
            version_hint: None,
            resolved: Some("booth:4993931"),
            confirmed: 0,
            extraction_method: "link",
        },
        ObsSeed {
            product_id: "booth:6584744",
            dep_kind: "shader",
            dep_name: "lilToon Shader",
            raw_quote: "Shader: Liltoon",
            source_span: "body",
            version_hint: None,
            resolved: None,
            confirmed: 0,
            extraction_method: "one_line",
        },
        ObsSeed {
            product_id: "booth:6584744",
            dep_kind: "other",
            dep_name: "Unity",
            raw_quote: "- Unity 2022.3.22f1",
            source_span: "body",
            version_hint: Some("2022.3.22f1"),
            resolved: None,
            confirmed: 0,
            extraction_method: "bullet",
        },
        ObsSeed {
            product_id: "booth:6584744",
            dep_kind: "tool_package",
            dep_name: "SomeTool",
            raw_quote: "Requires SomeTool.",
            source_span: "body",
            version_hint: None,
            resolved: Some("booth:7547699"),
            confirmed: 1,
            extraction_method: "one_line",
        },
    ]
}

// ---------------------------------------------------------------------------
// The versioned rule tables (matching rule v1 + advisory rule v1), as the
// consuming face's reference derivation over the frozen schema.
// ---------------------------------------------------------------------------

/// availability rule v1 (v0.2 protocol): recognized schema.org words map to
/// the stable enum; anything else (or nothing) is honest `unknown`.
fn availability_status(raw: Option<&str>) -> &'static str {
    match raw {
        Some(word) if word.ends_with("InStock") || word.ends_with("InStore") => "available",
        Some(word) if word.ends_with("SoldOut") || word.ends_with("Discontinued") => "unavailable",
        _ => "unknown",
    }
}

/// advisory rule v1: a match carries a suggestion ONLY when the layout is a
/// deliberate declaration (explicit_heading / one_line / bullet) AND the
/// install source is provable — which, with no VPM-repo fact in the store,
/// means a CONFIRMED resolution target (clues never turn into suggestions).
/// `vpm`/`unknown` stay in the frozen enum but are never emitted by v1.
fn advisory_rule_v1(
    conn: &Connection,
    extraction_method: &str,
    resolved: Option<&str>,
    confirmed: bool,
) -> Option<Value> {
    let confidence = match extraction_method {
        "explicit_heading" | "one_line" => "strong",
        "bullet" => "weak",
        _ => return None,
    };
    let target = if confirmed { resolved? } else { return None };
    let source_url: String = conn
        .query_row(
            "SELECT source_url FROM products WHERE product_id = ?1",
            [target],
            |row| row.get(0),
        )
        .ok()?;
    let install_source = if source_url.contains("booth.pm") {
        "booth_page"
    } else {
        "external_page"
    };
    Some(json!({
        "installSource": install_source,
        "confidence": confidence,
    }))
}

/// lookup under matching rule v1: case-insensitive (ASCII fold) EXACT over
/// dep_name — no substring, no fuzzy. Deterministic order: productId asc,
/// then observation_id asc. total is computed before limit/offset.
fn lookup_matches(
    conn: &Connection,
    name: &str,
    dep_kind: Option<&str>,
    limit: i64,
    offset: i64,
) -> (i64, Vec<Value>) {
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM dependency_observations
             WHERE lower(dep_name) = lower(?1) AND (?2 IS NULL OR dep_kind = ?2)",
            rusqlite::params![name, dep_kind],
            |row| row.get(0),
        )
        .unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT d.observation_id, d.product_id, d.dep_kind, d.dep_name, d.version_hint,
                    d.raw_quote, d.source_span, d.extraction_method,
                    d.resolved_ref_product_id, d.confirmed_by_human,
                    p.title, p.availability
             FROM dependency_observations d
             JOIN products p ON p.product_id = d.product_id
             WHERE lower(d.dep_name) = lower(?1) AND (?2 IS NULL OR d.dep_kind = ?2)
             ORDER BY d.product_id ASC, d.observation_id ASC
             LIMIT ?3 OFFSET ?4",
        )
        .unwrap();
    let matches = stmt
        .query_map(rusqlite::params![name, dep_kind, limit, offset], |row| {
            let extraction_method: String = row.get(7)?;
            let resolved: Option<String> = row.get(8)?;
            let confirmed: i64 = row.get(9)?;
            let title: Option<String> = row.get(10)?;
            let availability: Option<String> = row.get(11)?;
            Ok((
                extraction_method,
                resolved,
                confirmed,
                title,
                availability,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .unwrap()
        .map(|result| result.unwrap())
        .map(
            |(
                extraction_method,
                resolved,
                confirmed,
                title,
                availability,
                product_id,
                dep_kind,
                dep_name,
                version_hint,
                raw_quote,
                source_span,
            )| {
                let advisory = advisory_rule_v1(
                    conn,
                    &extraction_method,
                    resolved.as_deref(),
                    confirmed == 1,
                );
                let resolved_wire = if confirmed == 1 { resolved } else { None };
                json!({
                    "productId": product_id,
                    "productTitle": title,
                    "availabilityRaw": availability,
                    "availabilityStatus": availability_status(availability.as_deref()),
                    "depKind": dep_kind,
                    "depName": dep_name,
                    "versionHint": version_hint,
                    "rawQuote": raw_quote,
                    "sourceSpan": source_span,
                    "extractionMethod": extraction_method,
                    "resolvedProductId": resolved_wire,
                    "advisory": advisory,
                })
            },
        )
        .collect::<Vec<_>>();
    (total, matches)
}

fn lookup_envelope(total: i64, matches: &[Value]) -> Value {
    json!({
        "schemaVersion": "0.5",
        "operation": "dependencies.lookup",
        "result": { "total": total, "matches": matches },
    })
}

/// listByProduct assembly: the UNFILTERED observation face — resolution is
/// carried WITH its labeled confirmation state.
fn list_by_product(conn: &Connection, product_id: &str) -> Value {
    let status: String = conn
        .query_row(
            "SELECT status FROM products WHERE product_id = ?1",
            [product_id],
            |row| row.get(0),
        )
        .unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT dep_kind, dep_name, version_hint, raw_quote, source_span,
                    extraction_method, extracted_by, observed_at,
                    resolved_ref_product_id, confirmed_by_human, resolution_evidence
             FROM dependency_observations
             WHERE product_id = ?1
             ORDER BY observation_id ASC",
        )
        .unwrap();
    let observations = stmt
        .query_map([product_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, Option<String>>(10)?,
            ))
        })
        .unwrap()
        .map(|result| result.unwrap())
        .map(
            |(
                dep_kind,
                dep_name,
                version_hint,
                raw_quote,
                source_span,
                extraction_method,
                extracted_by,
                observed_at,
                resolved,
                confirmed,
                evidence_json,
            )| {
                let resolution = resolved.map(|target| {
                    json!({
                        "productId": target,
                        "confirmed": confirmed == 1,
                        "evidence": serde_json::from_str::<Value>(
                            evidence_json.as_deref().unwrap_or("[]"),
                        )
                        .unwrap(),
                    })
                });
                json!({
                    "depKind": dep_kind,
                    "depName": dep_name,
                    "versionHint": version_hint,
                    "rawQuote": raw_quote,
                    "sourceSpan": source_span,
                    "extractionMethod": extraction_method,
                    "extractedBy": extracted_by,
                    "observedAt": observed_at,
                    "resolution": resolution,
                })
            },
        )
        .collect::<Vec<_>>();
    json!({
        "schemaVersion": "0.5",
        "operation": "dependencies.listByProduct",
        "result": {
            "productId": product_id,
            "productStatus": status,
            "observations": observations,
        },
    })
}

// ---------------------------------------------------------------------------
// Vectors vs schemas.
// ---------------------------------------------------------------------------

#[test]
fn request_vectors_validate_against_the_query_schema() {
    let validator = query_validator();
    for name in [LOOKUP_REQUEST, LIST_BY_PRODUCT_REQUEST] {
        let vector = read_json(name);
        let errors: Vec<String> = validator
            .iter_errors(&vector)
            .map(|error| format!("{}: {error}", error.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{name} must validate: {errors:?}");
    }
}

#[test]
fn result_vectors_validate_against_the_result_schema() {
    let validator = result_validator();
    for name in [LOOKUP_RESULT, LIST_BY_PRODUCT_RESULT] {
        let vector = read_json(name);
        let errors: Vec<String> = validator
            .iter_errors(&vector)
            .map(|error| format!("{}: {error}", error.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{name} must validate: {errors:?}");
    }
}

#[test]
fn negative_vectors_are_rejected_by_the_query_schema() {
    let validator = query_validator();
    for name in NEGATIVES {
        let vector = read_json(name);
        assert!(
            !validator.is_valid(&vector),
            "{name} is a negative vector and must not validate"
        );
    }
}

#[test]
fn operation_closed_set_is_exactly_the_eight_queries() {
    let schema = read_json("query.schema.json");
    let operations: Vec<String> = schema["properties"]["operation"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        operations,
        vec![
            "catalog.list",
            "catalog.detail",
            "catalog.status",
            "warehouse.listEntries",
            "warehouse.entryDetail",
            "downloads.listCompleted",
            "dependencies.lookup",
            "dependencies.listByProduct",
        ]
    );
}

// ---------------------------------------------------------------------------
// Word faces equal the FROZEN BDL v0.2 closed sets.
// ---------------------------------------------------------------------------

/// Extracts every `column IN (...)` CHECK list for `column` from the frozen
/// readable authority.
fn sql_check_lists(sql: &str, column: &str) -> Vec<Vec<String>> {
    let marker = format!("{column} IN");
    let mut lists = Vec::new();
    let mut from = 0;
    while let Some(pos) = sql[from..].find(&marker) {
        let start = from + pos + marker.len();
        let after = sql[start..].trim_start();
        if let Some(rest) = after.strip_prefix('(') {
            let end = rest.find(')').expect("CHECK list must close");
            lists.push(
                rest[..end]
                    .split(',')
                    .map(|token| token.trim().trim_matches('\'').to_owned())
                    .filter(|token| !token.is_empty())
                    .collect::<Vec<_>>(),
            );
        }
        from = start;
    }
    assert!(
        !lists.is_empty(),
        "{column}: at least one CHECK list expected"
    );
    lists
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

#[test]
fn word_faces_equal_the_frozen_bdl_v02_authority() {
    let query = read_json("query.schema.json");
    let result = read_json("result.schema.json");

    let dep_kind_enum: Vec<String> = query["$defs"]["dependenciesLookupParams"]["properties"]
        ["depKind"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|value| value.as_str())
        .map(str::to_owned)
        .collect();
    let source_span_enum: Vec<String> = result["$defs"]["dependencyMatch"]["properties"]
        ["sourceSpan"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();
    let extraction_method_enum: Vec<String> = result["$defs"]["dependencyMatch"]["properties"]
        ["extractionMethod"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();

    // Every CHECK list in the authority (dep_kind once; source_span twice —
    // the compat rebuild and the dependency table) must equal the query
    // face's enum: the vocabulary rides the frozen sets, no drift.
    assert_eq!(
        sorted(
            sql_check_lists(AUTHORITY_V02, "dep_kind")
                .into_iter()
                .flatten()
                .collect()
        ),
        sorted(dep_kind_enum.clone()),
        "dep_kind enum must equal the frozen four-value set"
    );
    for list in sql_check_lists(AUTHORITY_V02, "source_span") {
        assert_eq!(
            sorted(list),
            sorted(source_span_enum.clone()),
            "source_span enum must equal the frozen five-value set"
        );
    }
    assert_eq!(
        sorted(
            sql_check_lists(AUTHORITY_V02, "extraction_method")
                .into_iter()
                .flatten()
                .collect()
        ),
        sorted(extraction_method_enum.clone()),
        "extraction_method enum must equal the frozen six-value set"
    );

    // The SAME three word faces against the EXECUTABLE migration chain —
    // the CHECKs that actually guard a runtime database (open_v02 executes
    // exactly 001+002). The readable restatement and the executable chain
    // must carry identical closed sets; drift in either authority now turns
    // this anchor red instead of passing silently while the runtime law
    // moves (core seats batches 180/191: the chain, not a restatement, is
    // the constraint authority — and table rebuilds can change constraints,
    // the 002 compat rebuild being the standing precedent).
    assert_eq!(
        sorted(
            sql_check_lists(MIGRATION_002, "dep_kind")
                .into_iter()
                .flatten()
                .collect()
        ),
        sorted(dep_kind_enum.clone()),
        "dep_kind enum must equal the executable chain's four-value set"
    );
    for list in sql_check_lists(MIGRATION_002, "source_span") {
        assert_eq!(
            sorted(list),
            sorted(source_span_enum.clone()),
            "source_span enum must equal the executable chain's five-value set"
        );
    }
    assert_eq!(
        sorted(
            sql_check_lists(MIGRATION_002, "extraction_method")
                .into_iter()
                .flatten()
                .collect()
        ),
        sorted(extraction_method_enum),
        "extraction_method enum must equal the executable chain's six-value set"
    );

    // productStatus rides the products.status closed set — whose CHECK
    // lives ONLY in the executable 001 chain (the v0.2 restatement carries
    // the column as comment-only; core seat batch-180 candidate refuted and
    // pinned there). The wire enum equals the hard law verbatim, so a
    // future rebuild that drops or edits the constraint cannot drift past
    // this anchor silently.
    let product_status_enum: Vec<String> = result["$defs"]["dependenciesListByProductResult"]
        ["properties"]["productStatus"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();
    for list in sql_check_lists(MIGRATION_001, "status") {
        assert_eq!(
            sorted(list),
            sorted(product_status_enum.clone()),
            "productStatus enum must equal the executable status CHECK (001 chain)"
        );
    }
}

// ---------------------------------------------------------------------------
// The two-face contrast is mechanical, and paths never appear.
// ---------------------------------------------------------------------------

fn collect_property_keys(value: &Value, out: &mut Vec<String>) {
    if let Some(object) = value.as_object() {
        for (key, child) in object {
            if key == "properties" {
                if let Some(properties) = child.as_object() {
                    for (property_key, property_value) in properties {
                        out.push(property_key.clone());
                        collect_property_keys(property_value, out);
                    }
                }
            } else {
                collect_property_keys(child, out);
            }
        }
    } else if let Some(array) = value.as_array() {
        for item in array {
            collect_property_keys(item, out);
        }
    }
}

#[test]
fn two_face_contrast_is_mechanical_and_paths_never_appear() {
    let result = read_json("result.schema.json");
    let defs = &result["$defs"];

    let match_keys: Vec<String> = defs["dependencyMatch"]["properties"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    for key in [
        "productId",
        "productTitle",
        "availabilityRaw",
        "availabilityStatus",
        "depKind",
        "depName",
        "versionHint",
        "rawQuote",
        "sourceSpan",
        "extractionMethod",
        "resolvedProductId",
        "advisory",
    ] {
        assert!(
            match_keys.contains(&key.to_owned()),
            "lookup match carries '{key}'"
        );
    }
    for omitted in ["extractedBy", "observedAt", "resolution"] {
        assert!(
            !match_keys.contains(&omitted.to_owned()),
            "lookup match must NOT carry '{omitted}' (admission rule)"
        );
    }

    let observation_keys: Vec<String> = defs["dependencyObservation"]["properties"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    for key in ["extractedBy", "observedAt", "resolution"] {
        assert!(
            observation_keys.contains(&key.to_owned()),
            "listByProduct observation carries '{key}' (the clue face)"
        );
    }
    assert!(
        !observation_keys.contains(&"advisory".to_owned()),
        "the unfiltered face carries no advisory — suggestions are lookup's job"
    );

    // The frozen resolution_evidence element shape: exactly the four keys.
    let evidence_keys: Vec<String> = defs["resolutionEvidence"]["properties"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        sorted(evidence_keys),
        vec![
            "linkText".to_owned(),
            "linkUrl".to_owned(),
            "note".to_owned(),
            "span".to_owned()
        ]
    );

    let product_status: Vec<String> = defs["dependenciesListByProductResult"]["properties"]
        ["productStatus"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(product_status, vec!["complete", "missing"]);

    // The house rule: no path-typed key anywhere on the two new faces.
    let mut keys = Vec::new();
    collect_property_keys(&defs["dependencyMatch"], &mut keys);
    collect_property_keys(&defs["dependencyObservation"], &mut keys);
    collect_property_keys(&defs["resolution"], &mut keys);
    collect_property_keys(&defs["dependenciesLookupResult"], &mut keys);
    collect_property_keys(&defs["dependenciesListByProductResult"], &mut keys);
    for key in &keys {
        assert!(
            !key.to_lowercase().contains("path"),
            "no path field may appear on the dependency faces, found '{key}'"
        );
    }
}

// ---------------------------------------------------------------------------
// Matching rule v1 over the frozen migration chain.
// ---------------------------------------------------------------------------

#[test]
fn lookup_rule_v1_matches_exact_case_folded_and_orders_deterministically() {
    let conn = open_v02();
    seed_observations(&conn, &standard_seeds());

    // Case-insensitive EXACT: input case never matters, substring never
    // matches ('lilToon Shader' stays out), and the two stored spellings
    // ('liltoon' / 'lilToon') both answer 'lilToon'.
    let (total, matches) = lookup_matches(&conn, "LILTOON", None, 200, 0);
    assert_eq!(total, 2, "exactly the two exact rows match");
    assert_eq!(matches.len(), 2);
    assert_eq!(
        matches[0]["productId"], "booth:6584744",
        "productId ascending first"
    );
    assert_eq!(matches[0]["depName"], "liltoon");
    assert_eq!(matches[1]["productId"], "booth:8179865");
    assert_eq!(matches[1]["depName"], "lilToon");

    // Deterministic order holds regardless of insertion order: productId
    // asc, then observation_id asc. Seed the same rows in a fresh store
    // with row order shuffled and compare identities.
    let ordered_ids: Vec<String> = matches
        .iter()
        .map(|m| {
            format!(
                "{}|{}",
                m["productId"].as_str().unwrap(),
                m["depName"].as_str().unwrap()
            )
        })
        .collect();
    assert_eq!(
        ordered_ids,
        vec![
            "booth:6584744|liltoon".to_owned(),
            "booth:8179865|lilToon".to_owned()
        ]
    );

    // depKind filter rides the frozen four-value set; non-suggestion rows
    // still list (the evidence face is unfiltered by the gates).
    let (filter_total, filter_matches) =
        lookup_matches(&conn, "Lapwing", Some("avatar_base"), 200, 0);
    assert_eq!(filter_total, 1);
    assert_eq!(filter_matches[0]["depKind"], "avatar_base");
    assert_eq!(
        filter_matches[0]["resolvedProductId"],
        Value::Null,
        "an unconfirmed resolution never surfaces on the lookup wire"
    );
    assert_eq!(filter_matches[0]["advisory"], Value::Null);
    let (foreign_total, _) = lookup_matches(&conn, "Unity", Some("avatar_base"), 200, 0);
    assert_eq!(
        foreign_total, 0,
        "depKind filter excludes the 'other' engine pin"
    );

    // Pagination: total is computed before limit/offset.
    let (total, page) = lookup_matches(&conn, "LILTOON", None, 1, 0);
    assert_eq!(total, 2, "total counts before pagination");
    assert_eq!(page.len(), 1);

    // Package-form input that is not literally present: honest empty set —
    // nominal-to-packageId identity is never guessed.
    let (empty_total, empty_matches) = lookup_matches(&conn, "com.lilxyzw.liltoon", None, 200, 0);
    assert_eq!(empty_total, 0, "no equivalence guessing");
    assert!(empty_matches.is_empty());
    let empty_errors: Vec<String> = result_validator()
        .iter_errors(&lookup_envelope(empty_total, &empty_matches))
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        empty_errors.is_empty(),
        "the honest empty envelope must validate: {empty_errors:?}"
    );
}

#[test]
fn suggestion_gates_and_the_clue_face_hold_over_the_frozen_schema() {
    let conn = open_v02();
    seed_observations(&conn, &standard_seeds());

    let (_, matches) = lookup_matches(&conn, "LILTOON", None, 200, 0);
    // Confirmed resolution + deliberate layout: the full suggestion rides.
    let strong = &matches[0];
    assert_eq!(strong["resolvedProductId"], "booth:3087170");
    assert_eq!(strong["advisory"]["installSource"], "booth_page");
    assert_eq!(strong["advisory"]["confidence"], "strong");
    // Prose layout: evidence rides verbatim, but no suggestion and no
    // resolution — the row stays in the library without entering the
    // suggestion face.
    let prose = &matches[1];
    assert_eq!(prose["advisory"], Value::Null);
    assert_eq!(prose["resolvedProductId"], Value::Null);

    // Non-booth resolution host maps to external_page (rule v1 emits only
    // booth_page/external_page — never vpm/unknown).
    let (_, some_tool) = lookup_matches(&conn, "SomeTool", None, 200, 0);
    assert_eq!(some_tool[0]["advisory"]["installSource"], "external_page");
    assert_eq!(some_tool[0]["advisory"]["confidence"], "strong");

    // Engine pin stored as other + version_hint: no resolution, so no
    // provable install source — advisory stays null (宁缺勿错).
    let (_, unity) = lookup_matches(&conn, "Unity", None, 200, 0);
    assert_eq!(unity[0]["depKind"], "other");
    assert_eq!(unity[0]["versionHint"], "2022.3.22f1");
    assert_eq!(unity[0]["advisory"], Value::Null);

    // The whole suggestion face assembles into a validating envelope.
    let (total, all) = lookup_matches(&conn, "liltoon", None, 200, 0);
    let errors: Vec<String> = result_validator()
        .iter_errors(&lookup_envelope(total, &all))
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "assembled lookup must validate: {errors:?}"
    );

    // The clue face: the SAME store lists the unconfirmed mislink as a
    // labeled clue (confirmed:false) — the two-face contrast.
    let by_product = list_by_product(&conn, "booth:6584744");
    let observations = by_product["result"]["observations"].as_array().unwrap();
    assert_eq!(observations.len(), 5, "all observations list, unfiltered");
    let clue = observations
        .iter()
        .find(|o| o["depName"] == "Lapwing")
        .expect("the mislink clue lists");
    assert_eq!(clue["resolution"]["productId"], "booth:4993931");
    assert_eq!(
        clue["resolution"]["confirmed"], false,
        "the clue is labeled, never a conclusion"
    );
    assert_eq!(clue["resolution"]["evidence"].as_array().unwrap().len(), 1);
    let evidence_keys: Vec<&str> = clue["resolution"]["evidence"][0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    for key in ["linkText", "linkUrl", "span", "note"] {
        assert!(evidence_keys.contains(&key), "evidence carries '{key}'");
    }
    assert!(
        clue.get("advisory").is_none(),
        "no advisory on the clue face"
    );

    let by_product_errors: Vec<String> = result_validator()
        .iter_errors(&by_product)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        by_product_errors.is_empty(),
        "assembled listByProduct must validate: {by_product_errors:?}"
    );

    // Tombstone honesty: a dead source page lists as productStatus
    // 'missing' and its observations stay readable.
    let dead = list_by_product(&conn, "booth:8179865");
    assert_eq!(dead["result"]["productStatus"], "missing");
    assert_eq!(dead["result"]["observations"].as_array().unwrap().len(), 1);
    let dead_errors: Vec<String> = result_validator()
        .iter_errors(&dead)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    assert!(
        dead_errors.is_empty(),
        "tombstone envelope must validate: {dead_errors:?}"
    );
}
