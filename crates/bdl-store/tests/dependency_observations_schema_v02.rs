//! Consumer test for the FROZEN `schemas/bdl/v0.2` (the product-dependency
//! observation face) — wt-4 batch 166 freeze batch of collab/proposals/030
//! (BOARD #46); drafted batch 164, frozen this batch.
//!
//! This file consumes the FROZEN schema files and the FROZEN vector files
//! over raw connections — it tests the schema/vector word face, NOT store
//! behavior. (Registering correction, wt-4 batch 168: the header used to
//! say "the store still runs format v0.1" — true at the freeze batch; the
//! store v0.2 landing is batch 168's slice, and the store's own behavior
//! tests live in dependency_observations_store_v02.rs.) What is pinned
//! here, per the「Schema＋正负例向量＋至少一端消费测试」freeze triad:
//!
//! - the v0.1 -> v0.2 migration chain preserves v0.1 rows verbatim (CHECK
//!   rebuild is a persistent-format obligation, data loss is a failure);
//! - the vector files match the registered freeze direction: exactly nine
//!   accept vectors (P1–P9) and eight reject vectors (N1–N8), and the frozen
//!   dep_kind closed set is pinned by N1 rejecting the five-value draft's
//!   `unity_or_sdk_version` member (version constraints ride version_hint);
//! - every accept-vector case row inserts; every reject-vector case row
//!   violates a constraint (closed sets, NOT NULL laws, the two confidence
//!   dimensions as two columns, the resolution-evidence hard law, FKs);
//! - the fresh readable authority (schema.sql) and the migrated chain
//!   (001 + 002) carry the same logical shape.
//!
//! Vector wording quotes the 030 §1 survey sample phrases (public page free
//! text, no paid content); synthetic negatives are marked in the vector
//! `basis` fields. Zero end-to-end claims: schema-file behavior only, never
//! store v0.2 behavior.

use rusqlite::Connection;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

const MIGRATION_001: &str = include_str!("../../../schemas/bdl/v0.1/001_initial.sql");
const MIGRATION_002: &str =
    include_str!("../../../schemas/bdl/v0.2/002_dependency_observations.sql");
const AUTHORITY_V02: &str = include_str!("../../../schemas/bdl/v0.2/schema.sql");

const PRODUCT_A: &str = "booth:6584744";

fn open_v01_with_product() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    conn.execute_batch(MIGRATION_001).unwrap();
    for (product_id, native_id) in [("booth:6584744", "6584744"), ("booth:3087170", "3087170")] {
        conn.execute(
            "INSERT INTO products (product_id, native_product_id, source_url, status,
                 content_hash, observed_at, processor_version)
             VALUES (?1, ?2, ?3, 'complete', 'sha256:' || hex(zeroblob(32)), ?4, 'obs-0.1')",
            rusqlite::params![
                product_id,
                native_id,
                format!("https://booth.pm/ja/items/{native_id}"),
                "2026-09-21T00:00:00.000Z"
            ],
        )
        .unwrap();
    }
    conn
}

fn migrate_to_v02() -> Connection {
    let conn = open_v01_with_product();
    conn.execute_batch(MIGRATION_002).unwrap();
    conn
}

fn fmt_version(conn: &Connection) -> String {
    conn.query_row(
        "SELECT value FROM bdl_meta WHERE key = 'format_version'",
        [],
        |row| row.get::<_, String>(0),
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// The frozen vector files (vectors/): the machine-readable word face.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct VectorFile {
    vector: String,
    name: String,
    expect: String, // "accept" | "reject"
    #[serde(default)]
    reject_law: Option<String>,
    cases: Vec<VectorCase>,
}

#[derive(Deserialize)]
struct VectorCase {
    table: String,
    values: serde_json::Map<String, Value>,
}

fn vectors_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/bdl/v0.2/vectors")
        .leak()
}

fn load_vectors() -> Vec<(String, VectorFile)> {
    let dir = vectors_dir();
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let bytes = fs::read(dir.join(&name)).unwrap();
            let parsed: VectorFile = serde_json::from_slice(&bytes).unwrap();
            (name, parsed)
        })
        .collect()
}

fn insert_case(conn: &Connection, case: &VectorCase) -> rusqlite::Result<usize> {
    let mut columns: Vec<String> = case.values.keys().cloned().collect();
    columns.sort();
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        case.table,
        columns.join(", "),
        vec!["?"; columns.len()].join(", ")
    );
    let bound: Vec<Box<dyn rusqlite::ToSql>> = columns
        .iter()
        .map(|c| match &case.values[c] {
            Value::String(s) => Box::new(s.clone()) as Box<dyn rusqlite::ToSql>,
            Value::Number(n) => {
                Box::new(n.as_i64().expect("vector integers are i64")) as Box<dyn rusqlite::ToSql>
            }
            Value::Null => Box::new(rusqlite::types::Null) as Box<dyn rusqlite::ToSql>,
            other => panic!("vector values are string/int/null only, got: {other}"),
        })
        .collect();
    let refs: Vec<&dyn rusqlite::ToSql> = bound.iter().map(|b| b.as_ref()).collect();
    conn.execute(&sql, refs.as_slice())
}

// ---------------------------------------------------------------------------
// Migration: v0.1 rows survive the CHECK rebuild verbatim; format moves on.
// ---------------------------------------------------------------------------

#[test]
fn migration_preserves_v01_rows_and_moves_the_format_version() {
    let conn = open_v01_with_product();
    conn.execute(
        "INSERT INTO compatibility_observations
             (product_id, raw_quote, source_span, confirmed_by_human, observed_at)
         VALUES (?1, '・liltoon 2.3.2~', 'body', 1, '2026-09-21T00:00:00.000Z')",
        rusqlite::params![PRODUCT_A],
    )
    .unwrap();

    conn.execute_batch(MIGRATION_002).unwrap();

    assert_eq!(fmt_version(&conn), "0.2");
    let (quote, span, confirmed, observed): (String, String, i64, String) = conn
        .query_row(
            "SELECT raw_quote, source_span, confirmed_by_human, observed_at
             FROM compatibility_observations WHERE product_id = ?1",
            rusqlite::params![PRODUCT_A],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(quote, "・liltoon 2.3.2~");
    assert_eq!(span, "body");
    assert_eq!(confirmed, 1);
    assert_eq!(observed, "2026-09-21T00:00:00.000Z");
}

// ---------------------------------------------------------------------------
// The vector set matches the registered freeze direction: 9 accept (P1–P9)
// + 8 reject (N1–N8); the dep_kind pin rides N1's first case.
// ---------------------------------------------------------------------------

#[test]
fn vector_files_match_the_registered_freeze_direction() {
    let vectors = load_vectors();
    assert_eq!(vectors.len(), 17, "nine positives + eight negatives");

    let accepts: Vec<&VectorFile> = vectors
        .iter()
        .map(|(_, v)| v)
        .filter(|v| v.expect == "accept")
        .collect();
    let rejects: Vec<&VectorFile> = vectors
        .iter()
        .map(|(_, v)| v)
        .filter(|v| v.expect == "reject")
        .collect();
    assert_eq!(accepts.len(), 9, "P1–P9");
    assert_eq!(rejects.len(), 8, "N1–N8");
    for (name, v) in &vectors {
        // The frozen naming convention: the file name is the vector's name.
        assert_eq!(
            name,
            &format!("{}.json", v.name),
            "file name matches the frozen vector name"
        );
        assert!(!v.cases.is_empty(), "{name} carries cases");
        if v.expect == "reject" {
            assert!(v.reject_law.is_some(), "{name} names its law");
        }
    }

    // The freeze ruling's contested member: the five-value draft's
    // 'unity_or_sdk_version' is a REJECT case (N1), i.e. NOT a dep_kind.
    let n1 = vectors
        .iter()
        .map(|(_, v)| v)
        .find(|v| v.vector == "N1")
        .expect("N1 present");
    let kinds: Vec<String> = n1
        .cases
        .iter()
        .map(|c| c.values["dep_kind"].as_str().unwrap().to_string())
        .collect();
    assert!(
        kinds.contains(&"unity_or_sdk_version".to_string()),
        "N1 pins the five-value member as rejected; got {kinds:?}"
    );
}

// ---------------------------------------------------------------------------
// Every accept vector inserts; every reject vector violates its law.
// ---------------------------------------------------------------------------

#[test]
fn accept_vectors_insert_and_reject_vectors_violate() {
    for (name, vector) in load_vectors() {
        let conn = migrate_to_v02();
        match vector.expect.as_str() {
            "accept" => {
                for case in &vector.cases {
                    insert_case(&conn, case)
                        .unwrap_or_else(|e| panic!("{name} case must be accepted: {e}"));
                }
            }
            "reject" => {
                for case in &vector.cases {
                    let err = insert_case(&conn, case)
                        .expect_err(&format!("{name} case must be rejected"));
                    assert!(
                        matches!(err, rusqlite::Error::SqliteFailure(f, _)
                            if f.code == rusqlite::ErrorCode::ConstraintViolation),
                        "{name}: expected a constraint violation, got: {err}"
                    );
                }
            }
            other => panic!("{name}: unknown expect '{other}'"),
        }

        if vector.vector == "P2" {
            // Rows that omit confirmed_by_human default to 0 — the unconfirmed
            // by default law (clue, never suggestion) — while P4's explicit
            // confirmation is the only 1.
            let zeros: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM dependency_observations WHERE confirmed_by_human = 0",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(zeros >= 1, "omitted confirmed_by_human defaults to 0");
        }
        if vector.vector == "P9" {
            let ones: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM compatibility_observations
                     WHERE source_span IN ('title', 'description_link')",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(ones, 2, "both new compat spans landed (P9)");
        }
    }

    // Whole-set cross-check on one connection: every P row present, the only
    // confirmed resolution is P4's, and the P4 evidence carries the frozen
    // element shape {linkText, linkUrl, span, note}.
    let conn = migrate_to_v02();
    for (_, vector) in load_vectors() {
        if vector.expect != "accept" {
            continue;
        }
        for case in &vector.cases {
            insert_case(&conn, case).unwrap();
        }
    }
    let (total, confirmed): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(confirmed_by_human), 0) FROM dependency_observations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(total, 8, "P1–P8 land eight dependency rows");
    assert_eq!(confirmed, 1, "only P4's explicit confirmation is set");
    let evidence: String = conn
        .query_row(
            "SELECT resolution_evidence FROM dependency_observations
             WHERE confirmed_by_human = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let parsed: Vec<Value> = serde_json::from_str(&evidence).unwrap();
    assert!(!parsed.is_empty(), "evidence array non-empty");
    let element = &parsed[0];
    for key in ["linkText", "linkUrl", "span", "note"] {
        assert!(
            element.get(key).is_some(),
            "evidence element carries frozen key '{key}'"
        );
    }
}

// ---------------------------------------------------------------------------
// The fresh readable authority and the migrated chain carry the same shape.
// ---------------------------------------------------------------------------

fn logical_shape(conn: &Connection) -> std::collections::BTreeMap<String, Shape> {
    let mut shape = std::collections::BTreeMap::new();
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .unwrap();
        stmt.query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    for table in tables {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        // Normalization note: STRICT tables (the executable discipline — 001/
        // 002) make every PRIMARY KEY column implicitly NOT NULL, while the
        // readable authority (schema.sql, the v0.1 two-form contract) omits
        // STRICT. A key column is an identity column either way, so the pk
        // columns' notnull bit is normalized to 1; everything else must be
        // literally equal.
        let columns: Vec<(String, String, i64, Option<String>, i64)> = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                let col_type: String = row.get(2)?;
                let mut notnull: i64 = row.get(3)?;
                let default: Option<String> = row.get(4)?;
                let pk: i64 = row.get(5)?;
                if pk > 0 {
                    notnull = 1;
                }
                Ok((name, col_type, notnull, default, pk))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        let mut stmt = conn
            .prepare(&format!("PRAGMA index_list({table})"))
            .unwrap();
        let mut indexes: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .filter(|name| !name.starts_with("sqlite_"))
            .collect();
        indexes.sort();
        shape.insert(table, Shape { columns, indexes });
    }
    shape
}

#[derive(Debug, PartialEq, Eq)]
struct Shape {
    columns: Vec<(String, String, i64, Option<String>, i64)>,
    indexes: Vec<String>,
}

#[test]
fn fresh_authority_equals_the_migrated_chain() {
    let fresh = Connection::open_in_memory().unwrap();
    fresh.execute_batch(AUTHORITY_V02).unwrap();

    let migrated = migrate_to_v02();

    assert_eq!(fmt_version(&fresh), "0.2");
    assert_eq!(fmt_version(&migrated), "0.2");

    let fresh_shape = logical_shape(&fresh);
    let migrated_shape = logical_shape(&migrated);
    assert_eq!(
        fresh_shape, migrated_shape,
        "fresh authority and 001+002 chain must agree"
    );
    assert_eq!(
        fresh_shape.len(),
        10,
        "bdl_meta + nine v0.1 tables + dependency_observations"
    );
}
