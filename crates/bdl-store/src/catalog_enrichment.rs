//! Admission for bounded enrichment of existing account-library products.
use super::*;

const ELIGIBLE: &str = "
    length(p.native_product_id) BETWEEN 1 AND 20
    AND EXISTS(SELECT 1 FROM product_library_memberships m WHERE m.product_id=p.product_id)
    AND NOT EXISTS(SELECT 1 FROM removed_library_cards r WHERE r.kind='product' AND r.target_id=p.product_id)
    AND (NULLIF(TRIM(p.source_category),'') IS NULL OR NULLIF(TRIM(p.shop_name),'') IS NULL
        OR NULLIF(TRIM(p.shop_url),'') IS NULL OR NULLIF(TRIM(p.description),'') IS NULL
        OR NULLIF(TRIM(p.source_published_at),'') IS NULL)
    AND NOT EXISTS(SELECT 1 FROM bdl_meta a WHERE a.key='enrich_attempted_at:' || p.product_id
        AND (julianday(a.value) IS NULL OR julianday(a.value) > julianday(?1)-7))";

impl BdlStore {
    /// Pure read. An attempt is recorded only by begin_product_enrichment.
    pub fn enrichment_queue(&self, limit: u32, now: &str) -> Result<Vec<String>, BdlStoreError> {
        if !(1..=60).contains(&limit) {
            return Err(BdlStoreError::InvalidEvent("invalid_limit"));
        }
        let connection = self.connection.lock().expect("SQLite connection poisoned");
        validate_clock(&connection, now)?;
        let mut query = connection.prepare(&format!(
            "SELECT p.product_id FROM products p WHERE {ELIGIBLE}
             ORDER BY CASE
                WHEN EXISTS(SELECT 1 FROM product_library_memberships m WHERE m.product_id=p.product_id AND m.library_type='bought') THEN 0
                WHEN EXISTS(SELECT 1 FROM product_library_memberships m WHERE m.product_id=p.product_id AND m.library_type='gifts') THEN 1
                ELSE 2 END,p.product_id LIMIT ?2"
        ))?;
        let rows = query
            .query_map(params![now, limit], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Recheck membership, missing facts and the retry interval atomically.
    pub fn begin_product_enrichment(
        &self,
        product_id: &str,
        now: &str,
    ) -> Result<bool, BdlStoreError> {
        let mut connection = self.connection.lock().expect("SQLite connection poisoned");
        validate_clock(&connection, now)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            &format!(
                "INSERT INTO bdl_meta(key,value)
             SELECT 'enrich_attempted_at:' || p.product_id,?1 FROM products p
             WHERE p.product_id=?2 AND {ELIGIBLE}
             ON CONFLICT(key) DO UPDATE SET value=excluded.value"
            ),
            params![now, product_id],
        )?;
        transaction.commit()?;
        Ok(changed == 1)
    }
}

fn validate_clock(connection: &Connection, now: &str) -> Result<(), BdlStoreError> {
    let valid: bool =
        connection.query_row("SELECT julianday(?1) IS NOT NULL", [now], |row| row.get(0))?;
    if valid {
        Ok(())
    } else {
        Err(BdlStoreError::InvalidEvent("invalid_time"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-11-10T00:00:00.000Z";

    fn product(id: &str, membership: Option<&str>) -> ProductObservation {
        ProductObservation {
            product_id: format!("booth:{id}"),
            native_product_id: id.into(),
            library_type: membership.map(str::to_owned),
            variant_name: None,
            source_url: format!("https://booth.pm/ja/items/{id}"),
            final_url: None,
            status: ProductObservationStatus::Complete,
            source_locale: None,
            source_category: None,
            title: Some(format!("Synthetic {id}")),
            description: None,
            age_restriction: None,
            adult: false,
            availability: None,
            price_amount: None,
            price_currency: None,
            shop_name: None,
            shop_url: None,
            image_urls: vec![],
            video_urls: vec![],
            subproducts: vec![],
            source_published_at: None,
            content_hash: format!("sha256:{}", "1".repeat(64)),
            observed_at: NOW.into(),
            run_id: None,
            processor_version: "synthetic-enrichment".into(),
            missing_fields: vec!["adult_badge".into()],
        }
    }

    #[test]
    fn only_owned_incomplete_visible_products_are_queued_by_membership_priority() {
        let store = BdlStore::open_in_memory().unwrap();
        for (id, membership) in [
            ("1", Some("free_downloads")),
            ("2", Some("gifts")),
            ("3", Some("bought")),
            ("4", None),
            ("5", Some("bought")),
            ("999999999999999999999", Some("bought")),
        ] {
            store
                .record_product_observation(&product(id, membership))
                .unwrap();
        }
        store
            .record_product_observation(&product("2", Some("bought")))
            .unwrap();
        store
            .remove_library_cards(
                &RemoveLibraryCards {
                    schema_version: "0.2".into(),
                    targets: vec![LibraryCardTarget {
                        kind: "product".into(),
                        id: "booth:5".into(),
                    }],
                },
                "remove-5",
                NOW,
            )
            .unwrap();
        let mut complete = product("6", Some("bought"));
        complete.source_category = Some("3D".into());
        complete.description = Some("Description".into());
        complete.shop_name = Some("Shop".into());
        complete.shop_url = Some("https://synthetic.booth.pm/".into());
        complete.source_published_at = Some(NOW.into());
        store.record_product_observation(&complete).unwrap();
        assert_eq!(
            store.enrichment_queue(60, NOW).unwrap(),
            vec!["booth:2", "booth:3", "booth:1"]
        );
        assert_eq!(store.enrichment_queue(1, NOW).unwrap(), vec!["booth:2"]);
        assert!(!store.begin_product_enrichment("booth:4", NOW).unwrap());
        assert!(!store.begin_product_enrichment("booth:5", NOW).unwrap());
        assert!(!store.begin_product_enrichment("booth:6", NOW).unwrap());
        assert!(!store
            .begin_product_enrichment("booth:999999999999999999999", NOW)
            .unwrap());
    }

    #[test]
    fn queries_are_read_only_and_attempts_have_an_exact_seven_day_boundary() {
        let store = BdlStore::open_in_memory().unwrap();
        store
            .record_product_observation(&product("1", Some("bought")))
            .unwrap();
        assert_eq!(
            store.enrichment_queue(60, NOW).unwrap(),
            store.enrichment_queue(60, NOW).unwrap()
        );
        assert!(store.begin_product_enrichment("booth:1", NOW).unwrap());
        assert!(!store
            .begin_product_enrichment("booth:1", "2026-11-09T00:00:00Z")
            .unwrap());
        assert!(store
            .enrichment_queue(60, "2026-11-16T23:59:59.999Z")
            .unwrap()
            .is_empty());
        assert!(!store
            .begin_product_enrichment("booth:1", "2026-11-16T23:59:59.999Z")
            .unwrap());
        assert_eq!(
            store.enrichment_queue(60, "2026-11-17T00:00:00Z").unwrap(),
            vec!["booth:1"]
        );
        assert!(store
            .begin_product_enrichment("booth:1", "2026-11-17T00:00:00Z")
            .unwrap());
    }

    #[test]
    fn admission_rechecks_removal_and_allows_only_one_concurrent_claim() {
        let store = std::sync::Arc::new(BdlStore::open_in_memory().unwrap());
        store
            .record_product_observation(&product("1", Some("bought")))
            .unwrap();
        assert_eq!(store.enrichment_queue(60, NOW).unwrap(), vec!["booth:1"]);
        let claims: Vec<_> = (0..8)
            .map(|_| {
                let store = store.clone();
                std::thread::spawn(move || store.begin_product_enrichment("booth:1", NOW).unwrap())
            })
            .collect();
        assert_eq!(
            claims
                .into_iter()
                .map(|claim| claim.join().unwrap())
                .filter(|claimed| *claimed)
                .count(),
            1
        );
        // A second product can disappear after enumeration, before admission.
        store
            .record_product_observation(&product("2", Some("gifts")))
            .unwrap();
        assert_eq!(store.enrichment_queue(60, NOW).unwrap(), vec!["booth:2"]);
        store
            .remove_library_cards(
                &RemoveLibraryCards {
                    schema_version: "0.2".into(),
                    targets: vec![LibraryCardTarget {
                        kind: "product".into(),
                        id: "booth:2".into(),
                    }],
                },
                "remove-2",
                NOW,
            )
            .unwrap();
        assert!(!store.begin_product_enrichment("booth:2", NOW).unwrap());
    }

    #[test]
    fn sparse_observations_preserve_existing_facts_and_normalized_search() {
        let store = BdlStore::open_in_memory().unwrap();
        let mut rich = product("1", Some("bought"));
        rich.source_category = Some("3D".into());
        rich.description = Some("First line\n\nSecond line".into());
        rich.shop_name = Some("Synthetic Shop".into());
        rich.shop_url = Some("https://synthetic.booth.pm/".into());
        rich.source_published_at = Some(NOW.into());
        rich.adult = true;
        rich.subproducts = vec![SubproductObservation {
            variation_id: Some("9".into()),
            name: Some("Coat".into()),
            price_amount: None,
            price_currency: None,
            availability: None,
        }];
        store.record_product_observation(&rich).unwrap();
        let mut sparse = product("1", None);
        sparse.title = None;
        store.record_product_observation(&sparse).unwrap();
        let detail = store.catalog_detail("booth:1").unwrap().unwrap().product;
        assert_eq!(detail.description, rich.description);
        assert_eq!(detail.shop_name, rich.shop_name);
        assert_eq!(detail.shop_url, rich.shop_url);
        assert!(detail.adult);
        assert_eq!(
            store.library_product_categories().unwrap()["booth:1"],
            rich.source_category
        );
        assert_eq!(
            store.library_product_dates().unwrap()["booth:1"].published_at,
            rich.source_published_at
        );
        let search: String = store
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT search_text_normalized FROM products WHERE product_id='booth:1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            search,
            "synthetic 1 first line second line synthetic shop coat"
        );
        assert!(store.enrichment_queue(60, NOW).unwrap().is_empty());
    }

    #[test]
    fn attempts_survive_reopening_and_invalid_clocks_do_not_claim() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bdl.sqlite");
        {
            let store = BdlStore::open(&path).unwrap();
            store
                .record_product_observation(&product("1", Some("bought")))
                .unwrap();
            assert!(store.begin_product_enrichment("booth:1", NOW).unwrap());
        }
        let store = BdlStore::open(&path).unwrap();
        assert!(store.enrichment_queue(60, NOW).unwrap().is_empty());
        assert!(store.enrichment_queue(0, NOW).is_err());
        assert!(store.enrichment_queue(61, NOW).is_err());
        assert!(store
            .begin_product_enrichment("booth:1", "invalid")
            .is_err());
    }
}
