use rusqlite::{params, Connection};
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use vua_bdl_store::{BdlStore, BDL_FORMAT_VERSION};

fn baseline(connection: &Connection) {
    connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    for migration in [
        include_str!("../../../schemas/bdl/v0.1/001_initial.sql"),
        include_str!("../../../schemas/bdl/v0.2/002_dependency_observations.sql"),
        include_str!("../../../schemas/bdl/v0.3/003_library_type.sql"),
        include_str!("../../../schemas/bdl/v0.3/004_variant_name.sql"),
        include_str!("../../../schemas/bdl/v0.4/005_product_downloadables.sql"),
        include_str!("../../../schemas/bdl/v0.5/006_managed_library_files.sql"),
        include_str!("../../../schemas/bdl/v0.6/007_archive_members.sql"),
        include_str!("../../../schemas/bdl/v0.7/008_library_entry_metadata.sql"),
        include_str!("../../../schemas/bdl/v0.8/009_removed_local_entries.sql"),
        include_str!("../../../schemas/bdl/v0.9/010_removed_library_cards.sql"),
        include_str!("../../../schemas/bdl/v0.10/011_description_links.sql"),
    ] {
        connection.execute_batch(migration).unwrap();
    }
    connection.pragma_update(None, "user_version", 11).unwrap();
    connection.execute("INSERT INTO products(product_id,native_product_id,source_url,status,title,description,adult,content_hash,observed_at,processor_version,library_type) VALUES ('booth:1','1','https://booth.pm/items/1','complete','Synthetic legacy product','Kept description',0,?1,'2026-10-19T00:00:00.000Z','synthetic/1','bought')",[format!("sha256:{}","a".repeat(64))]).unwrap();
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "vua-terms-store-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        assert!(
            self.0.starts_with(std::env::temp_dir())
                && self
                    .0
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("vua-terms-store-")
        );
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn migration_from_v010_keeps_products_human_evidence_and_description_links() {
    let root = Temp::new();
    let path = root.0.join("bdl.db");
    let connection = Connection::open(&path).unwrap();
    baseline(&connection);
    connection.execute("INSERT INTO term_observations(product_id,kind,source_kind,source_url,extracted_by,note,observed_at) VALUES ('booth:1','vn3','link','https://example.invalid/legacy','human','Preserve this human note','2026-10-19T00:00:00.000Z')",[]).unwrap();
    connection.execute("INSERT INTO product_description_links VALUES ('booth:1','[\"booth:2\"]','synthetic-hash','2026-10-19T00:00:00.000Z')",[]).unwrap();
    drop(connection);
    let store = BdlStore::open(&path).unwrap();
    assert_eq!(BDL_FORMAT_VERSION, "0.11");
    let component: Value = serde_json::from_str(include_str!(
        "../../../apps/desktop/distribution/amf-version.json"
    ))
    .unwrap();
    assert_eq!(component["bdlWrite"], BDL_FORMAT_VERSION);
    let readable = component["bdlRead"].as_array().unwrap();
    assert!(readable.iter().any(|v| v == "0.10"));
    assert!(readable.iter().any(|v| v == BDL_FORMAT_VERSION));
    let product = store.terms_product("booth:1").unwrap().unwrap();
    assert_eq!(product.description.as_deref(), Some("Kept description"));
    assert_eq!(
        store.product_terms("booth:1").unwrap().state,
        "not_observed"
    );
    let connection = Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        12
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT value FROM bdl_meta WHERE key='format_version'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "0.11"
    );
    assert_eq!(
        connection
            .query_row("SELECT note FROM term_observations", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "Preserve this human note"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT target_ids FROM product_description_links",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "[\"booth:2\"]"
    );
}

#[test]
fn v011_schema_vectors_exercise_actual_constraints() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/bdl/v0.11/vectors");
    for file in fs::read_dir(root).unwrap() {
        let path = file.unwrap().path();
        let vector: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let connection = Connection::open_in_memory().unwrap();
        baseline(&connection);
        connection
            .execute_batch(include_str!("../../../schemas/bdl/v0.11/012_vn3_terms.sql"))
            .unwrap();
        connection.execute("INSERT INTO terms_sources(source_id,source_kind,source_url,google_file_id,status) VALUES ('drive:synthetic','drive','https://drive.google.com/file/d/synthetic/view','synthetic','pending')",[]).unwrap();
        connection.execute("INSERT INTO vn3_documents(source_id,sha256,parser_version,status,generator_version,vn3_version,language,hashtags,warnings) VALUES ('drive:synthetic',?1,'vn3-parser/1','parsed','1.11','1.10','en','[]','[]')",["a".repeat(64)]).unwrap();
        if let Some(setup) = vector.get("setup").and_then(Value::as_str) {
            connection.execute_batch(setup).unwrap();
        }
        for case in vector["cases"].as_array().unwrap() {
            let table = case["table"].as_str().unwrap();
            assert!(["vn3_documents", "vn3_document_items", "terms_sources"].contains(&table));
            let values = case["values"].as_object().unwrap();
            let columns: Vec<_> = values.keys().map(String::as_str).collect();
            let bindings: Vec<rusqlite::types::Value> = values
                .values()
                .map(|v| match v {
                    Value::Null => rusqlite::types::Value::Null,
                    Value::String(s) => s.clone().into(),
                    _ => panic!("owned string/null fixture"),
                })
                .collect();
            let sql = format!(
                "INSERT INTO {table} ({}) VALUES ({})",
                columns.join(","),
                vec!["?"; columns.len()].join(",")
            );
            let result = connection.execute(&sql, rusqlite::params_from_iter(bindings));
            assert_eq!(
                result.is_ok(),
                vector["expect"] == "accept",
                "{}: {result:?}",
                path.display()
            );
        }
    }
}

#[test]
fn empty_source_snapshot_and_failed_refresh_do_not_claim_vn3_permission() {
    let root = Temp::new();
    let path = root.0.join("bdl.db");
    let connection = Connection::open(&path).unwrap();
    baseline(&connection);
    drop(connection);
    let store = BdlStore::open(&path).unwrap();
    store
        .record_terms_links("booth:1", &[], "2026-10-19T00:00:00.000Z")
        .unwrap();
    assert_eq!(store.product_terms("booth:1").unwrap().state, "no_vn3");
    use vua_bdl_store::terms::{TermsDocument, TermsLink};
    let link = TermsLink {
        source_id: "drive:synthetic".into(),
        source_kind: "drive".into(),
        source_url: "https://drive.google.com/file/d/synthetic/view".into(),
        google_file_id: Some("synthetic".into()),
    };
    store
        .record_terms_links("booth:1", &[link], "2026-10-19T00:00:00.000Z")
        .unwrap();
    let doc = TermsDocument::empty(
        "unrecognized",
        &"b".repeat(64),
        "vn3-parser/1",
        "unrecognized_version",
    );
    store
        .record_terms_result(
            "drive:synthetic",
            "unrecognized",
            "2026-10-19T00:00:00.000Z",
            None,
            Some(&doc),
        )
        .unwrap();
    assert!(store
        .cached_terms_document("drive:synthetic", &doc.sha256, "vn3-parser/1")
        .unwrap()
        .is_some());
    store
        .record_terms_result(
            "drive:synthetic",
            "failed",
            "2026-10-19T01:00:00.000Z",
            Some("timeout"),
            None,
        )
        .unwrap();
    let result = store.product_terms("booth:1").unwrap();
    assert_eq!(result.state, "failed");
    assert!(result.sources[0].document.is_none());
    assert_eq!(result.sources[0].problem.as_deref(), Some("timeout"));
    let connection = Connection::open(&path).unwrap();
    assert!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM term_observations WHERE extracted_by=?1",
                params!["vn3-parser/1"],
                |r| r.get::<_, i64>(0)
            )
            .unwrap()
            >= 2
    );
}

#[test]
fn legacy_capture_preserves_html_evidence_and_excess_sources_remain_incomplete() {
    let root = Temp::new();
    let path = root.0.join("bdl.db");
    let connection = Connection::open(&path).unwrap();
    baseline(&connection);
    drop(connection);
    let store = BdlStore::open(&path).unwrap();
    let at = "2026-10-19T00:00:00.000Z";
    store.record_terms_links("booth:1", &[], at).unwrap();
    let links: Vec<_> = (0..257)
        .map(|n| vua_bdl_store::terms::TermsLink {
            source_id: format!("drive:synthetic{n}"),
            source_kind: "drive".into(),
            source_url: format!("https://drive.google.com/file/d/synthetic{n}/view"),
            google_file_id: Some(format!("synthetic{n}")),
        })
        .collect();
    store
        .record_terms_links_if_missing("booth:1", &links, at)
        .unwrap();
    assert_eq!(store.product_terms("booth:1").unwrap().state, "no_vn3");
    store.record_terms_links("booth:1", &links, at).unwrap();
    let snapshot = store.product_terms("booth:1").unwrap();
    assert_eq!(snapshot.state, "unsupported");
    assert_eq!(snapshot.capture_problem.as_deref(), Some("source_limit"));
    assert_eq!(snapshot.sources.len(), 256);
    store.record_terms_links("booth:1", &[], at).unwrap();
    let snapshot = store.product_terms("booth:1").unwrap();
    assert_eq!(snapshot.state, "no_vn3");
    assert!(snapshot.capture_problem.is_none());
}
