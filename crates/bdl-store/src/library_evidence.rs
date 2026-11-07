//! Local evidence read faces. Current presence and task interpretation belong to AMF.
use super::*;

const LIBRARY_POSITION_PREFIX: &str = "library_position:";

/// When a product first entered an account library, and BOOTH's own publish date when recorded.
#[derive(Clone, Debug, Default)]
pub struct LibraryProductDates {
    pub first_seen_at: Option<String>,
    pub published_at: Option<String>,
}

pub struct LibraryCopyEvidence {
    pub copy: StoredArtifactCopy,
    pub size_bytes: u64,
    pub product_ids: Vec<String>,
    pub downloadable_id: Option<i64>,
    pub archive_downloadable_id: Option<i64>,
    pub archive_current: bool,
    pub archive_expansion_state: Option<String>,
    pub superseded: bool,
}

impl BdlStore {
    /// Last complete description snapshot. Empty replaces older link evidence;
    /// account-library observations do not overwrite this independent snapshot.
    pub fn record_description_links(&self, product_id: &str, targets: &[String], content_hash: &str, observed_at: &str) -> Result<(), BdlStoreError> {
        let targets = serde_json::to_string(targets)?;
        let connection = self.connection.lock().expect("SQLite connection poisoned");
        connection.execute("INSERT INTO product_description_links(product_id,target_ids,content_hash,observed_at) VALUES(?1,?2,?3,?4)
            ON CONFLICT(product_id) DO UPDATE SET target_ids=excluded.target_ids,content_hash=excluded.content_hash,observed_at=excluded.observed_at",
            rusqlite::params![product_id, targets, content_hash, observed_at])?;
        Ok(())
    }

    pub fn library_description_links(&self) -> Result<std::collections::HashMap<String, Vec<String>>, BdlStoreError> {
        let connection = self.connection.lock().expect("SQLite connection poisoned");
        let mut statement = connection.prepare("SELECT product_id,target_ids FROM product_description_links")?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
        let mut result = std::collections::HashMap::new();
        for row in rows { let (id, targets) = row?; result.insert(id, serde_json::from_str(&targets)?); }
        Ok(result)
    }

    /// Observed product-page variant choices, never account entitlement counts.
    pub fn library_variant_counts(&self) -> Result<std::collections::HashMap<String, usize>, BdlStoreError> {
        let connection = self.connection.lock().expect("SQLite connection poisoned");
        let mut statement = connection.prepare("SELECT product_id,COALESCE(json_array_length(subproducts),0) FROM products")?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)? as usize)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn library_product_dates(&self) -> Result<std::collections::HashMap<String, LibraryProductDates>, BdlStoreError> {
        let connection = self.connection.lock().expect("SQLite connection poisoned");
        let mut statement = connection.prepare(
            "SELECT p.product_id,(SELECT MIN(m.first_seen_at) FROM product_library_memberships m WHERE m.product_id=p.product_id),p.source_published_at FROM products p",
        )?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, LibraryProductDates { first_seen_at: row.get(1)?, published_at: row.get(2)? })))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Original BOOTH category observations, without widening the frozen catalog face.
    pub fn library_product_categories(&self) -> Result<std::collections::HashMap<String, Option<String>>, BdlStoreError> {
        let conn = self.connection.lock().expect("SQLite connection poisoned");
        let mut statement = conn.prepare("SELECT product_id, source_category FROM products ORDER BY product_id")?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn library_product_summaries(
        &self,
    ) -> Result<Vec<crate::bdl_queries::CatalogProductSummary>, BdlStoreError> {
        // This private domain read has no transport page limit; library.list owns
        // filtering/pagination after combining products and physical entries.
        Ok(self
            .catalog_list(&crate::bdl_queries::CatalogListParams {
                limit: i64::MAX,
                ..Default::default()
            })?
            .entries)
    }

    /// Where products sit in one of BOOTH's account lists, newest purchase first. These live beside
    /// the format version, so builds that do not read them keep opening this store.
    pub fn record_library_positions(&self, library_type: &str, positions: &[(usize, String)]) -> Result<(), BdlStoreError> {
        let mut connection = self.connection.lock().expect("SQLite connection poisoned");
        let transaction = connection.transaction()?;
        for (position, product_id) in positions {
            transaction.execute(
                "INSERT INTO bdl_meta(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![format!("{LIBRARY_POSITION_PREFIX}{library_type}:{product_id}"), position.to_string()],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Every recorded place: library type, product and position.
    pub fn library_positions(&self) -> Result<Vec<(String, String, i64)>, BdlStoreError> {
        let connection = self.connection.lock().expect("SQLite connection poisoned");
        let mut statement = connection.prepare("SELECT key, value FROM bdl_meta WHERE key LIKE 'library_position:%'")?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
        let mut positions = Vec::new();
        for row in rows {
            let (key, value) = row?;
            let Some((kind, product)) = key.strip_prefix(LIBRARY_POSITION_PREFIX).and_then(|rest| rest.split_once(':')) else { continue };
            let Ok(position) = value.parse() else { continue };
            positions.push((kind.to_owned(), product.to_owned(), position));
        }
        Ok(positions)
    }

    pub fn product_library_memberships(
        &self,
        product_id: &str,
    ) -> Result<Vec<String>, BdlStoreError> {
        let connection = self.connection.lock().expect("SQLite connection poisoned");
        let mut statement = connection.prepare("SELECT library_type FROM product_library_memberships WHERE product_id=?1 ORDER BY library_type")?;
        let rows = statement
            .query_map([product_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Every product's memberships at once, for reads that cover the whole library.
    pub fn library_memberships_all(
        &self,
    ) -> Result<std::collections::HashMap<String, Vec<String>>, BdlStoreError> {
        let connection = self.connection.lock().expect("SQLite connection poisoned");
        let mut statement = connection.prepare(
            "SELECT product_id,library_type FROM product_library_memberships ORDER BY product_id,library_type",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut memberships = std::collections::HashMap::<String, Vec<String>>::new();
        for row in rows {
            let (product, library_type) = row?;
            memberships.entry(product).or_default().push(library_type);
        }
        Ok(memberships)
    }

    /// The product each downloadable was first seen under, for reads that cover the whole library.
    pub fn downloadable_owners(
        &self,
    ) -> Result<std::collections::HashMap<i64, String>, BdlStoreError> {
        let connection = self.connection.lock().expect("SQLite connection poisoned");
        let mut statement = connection.prepare(
            "SELECT downloadable_id,product_id FROM product_downloadables ORDER BY first_seen_at,downloadable_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut owners = std::collections::HashMap::new();
        for row in rows {
            let (downloadable, product) = row?;
            owners.entry(downloadable).or_insert(product);
        }
        Ok(owners)
    }

    pub fn library_copy_evidence(&self) -> Result<Vec<LibraryCopyEvidence>, BdlStoreError> {
        let connection = self.connection.lock().expect("SQLite connection poisoned");
        let mut statement = connection.prepare(
            "SELECT c.copy_id,c.warehouse_item_id,c.artifact_sha256,c.relative_path,c.stored_path,c.role,c.created_at,a.size_bytes,
                    (SELECT json_group_array(m.product_id) FROM artifact_mappings m WHERE m.artifact_sha256=c.artifact_sha256),
                    (SELECT downloadable_id FROM managed_library_files f WHERE f.copy_id=c.copy_id),
                    EXISTS(SELECT 1 FROM superseded_generated_copies s WHERE s.copy_id=c.copy_id),
                    (SELECT f.downloadable_id FROM archive_members m JOIN managed_library_files f ON f.copy_id=m.parent_copy_id WHERE m.copy_id=c.copy_id),
                    EXISTS(SELECT 1 FROM archive_members m JOIN artifact_copies p ON p.copy_id=m.parent_copy_id WHERE m.copy_id=c.copy_id AND m.parent_sha256=p.artifact_sha256),
                    (SELECT state FROM archive_expansions e WHERE e.parent_copy_id=c.copy_id AND e.parent_sha256=c.artifact_sha256)
             FROM artifact_copies c JOIN local_artifacts a ON a.artifact_sha256=c.artifact_sha256 ORDER BY c.created_at,c.copy_id")?;
        let mut copies = Vec::new();
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let role: String = row.get(5)?;
            let size: i64 = row.get(7)?;
            let product_ids: String = row.get(8)?;
            copies.push(LibraryCopyEvidence {
                copy: StoredArtifactCopy {
                    copy_id: row.get(0)?,
                    warehouse_item_id: row.get(1)?,
                    artifact_sha256: row.get(2)?,
                    relative_path: row.get(3)?,
                    stored_path: row.get(4)?,
                    role: match role.as_str() {
                        "original" => CopyRole::Original,
                        "generated_vpm" => CopyRole::GeneratedVpm,
                        _ => {
                            return Err(BdlStoreError::CorruptValue {
                                field: "copy_role",
                                value: role,
                            })
                        }
                    },
                    created_at: row.get(6)?,
                },
                size_bytes: u64::try_from(size).map_err(|_| BdlStoreError::CorruptValue {
                    field: "size_bytes",
                    value: size.to_string(),
                })?,
                product_ids: serde_json::from_str(&product_ids)?,
                downloadable_id: row.get(9)?,
                superseded: row.get(10)?,
                archive_downloadable_id: row.get(11)?,
                archive_current: row.get(12)?,
                archive_expansion_state: row.get(13)?,
            });
        }
        // Generated packages belong to the same managed entry as its originals.
        // Derive their catalog grouping without creating provenance observations.
        let mut entry_products: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for copy in &copies {
            if copy.copy.role == CopyRole::Original {
                entry_products
                    .entry(copy.copy.warehouse_item_id.clone())
                    .or_default()
                    .extend(copy.product_ids.clone());
            }
        }
        for copy in &mut copies {
            if copy.copy.role == CopyRole::GeneratedVpm {
                if let Some(products) = entry_products.get(&copy.copy.warehouse_item_id) {
                    copy.product_ids.extend(products.clone());
                }
                copy.product_ids.sort();
                copy.product_ids.dedup();
            }
        }
        Ok(copies)
    }
}
