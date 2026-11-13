//! Library presentation reconciliation. A source hint never proves byte equality.
use crate::artifact_inspection::hex_lower;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use vua_bdl_store::{
    bdl_queries::CatalogProductSummary, CopyRole, LibraryCopyEvidence, WarehouseEntryCard,
};

pub(crate) const OPERATION: &str = "library.reconcileSources";

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Stamp {
    location: String,
    expected: String,
    length: u64,
    modified: SystemTime,
    created: Option<SystemTime>,
}
impl Stamp {
    fn read(copy: &LibraryCopyEvidence) -> Option<Self> {
        let path = Path::new(&copy.copy.stored_path);
        let meta = std::fs::symlink_metadata(path).ok()?;
        if !meta.file_type().is_file() {
            return None;
        }
        Some(Self {
            location: hex_lower(&Sha256::digest(path.to_string_lossy().as_bytes())),
            expected: copy.copy.artifact_sha256.clone(),
            length: meta.len(),
            modified: meta.modified().ok()?,
            created: meta.created().ok(),
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Proof {
    copy_id: String,
    stamp: Stamp,
    outcome: String,
}

/// Only background verification writes this cache. Queries compare metadata;
/// they never hash large files or launch work. Durable proofs can be reused after restart.
#[derive(Default)]
pub(crate) struct ContentChecks(HashMap<String, Proof>);
impl ContentChecks {
    pub(crate) fn insert(&mut self, proof: Proof) {
        self.0.insert(proof.copy_id.clone(), proof);
    }
    pub(crate) fn restore(
        store: &vua_orchestrator::SqliteTaskStore,
    ) -> Result<Self, vua_orchestrator::SqliteStoreError> {
        let mut checks = Self::default();
        let mut events = Vec::new();
        for task in store.idempotent_tasks(OPERATION)? {
            events.extend(store.events_after(&task.task_id, 0)?);
        }
        events.sort_by(|a, b| {
            (&a.occurred_at, &a.task_id, a.revision).cmp(&(&b.occurred_at, &b.task_id, b.revision))
        });
        for event in events {
            if event.payload["operation"] != OPERATION {
                continue;
            }
            if let Ok(proof) = serde_json::from_value::<Proof>(event.payload["proof"].clone()) {
                checks.insert(proof);
            }
        }
        Ok(checks)
    }
    pub(crate) fn check(&self, copy: &LibraryCopyEvidence) -> Option<&'static str> {
        let stamp = Stamp::read(copy)?;
        let proof = self.0.get(&copy.copy.copy_id)?;
        if proof.stamp != stamp {
            return None;
        }
        match proof.outcome.as_str() {
            "present" => Some("present"),
            "changed" => Some("changed"),
            "unreadable" => Some("unreadable"),
            _ => None,
        }
    }
    fn previously_verified(&self, copy: &LibraryCopyEvidence) -> bool {
        self.0.get(&copy.copy.copy_id).is_some_and(|proof| {
            proof.outcome == "present"
                && proof.stamp.expected == copy.copy.artifact_sha256
                && proof.stamp.location
                    == hex_lower(&Sha256::digest(copy.copy.stored_path.as_bytes()))
        })
    }
}

pub(crate) fn verify(
    copy: &LibraryCopyEvidence,
    ctx: &vua_orchestrator::TaskContext,
) -> Result<Option<Proof>, ()> {
    let Some(stamp) = Stamp::read(copy) else {
        return Ok(None);
    };
    let digest = (|| -> std::io::Result<Option<[u8; 32]>> {
        let mut file = std::fs::File::open(&copy.copy.stored_path)?;
        let mut hash = Sha256::new();
        let mut buffer = vec![0u8; 1024 * 1024];
        loop {
            if ctx.check_cancel() {
                return Ok(None);
            }
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hash.update(&buffer[..read]);
        }
        Ok(Some(hash.finalize().into()))
    })();
    let outcome = match digest {
        Ok(None) => return Err(()),
        Ok(Some(hash)) if format!("sha256:{}", hex_lower(&hash)) == stamp.expected => "present",
        Ok(_) => "changed",
        Err(_) => "unreadable",
    };
    // No proof for bytes/path metadata that changed during verification.
    if Stamp::read(copy).as_ref() != Some(&stamp) {
        return Ok(None);
    }
    Ok(Some(Proof {
        copy_id: copy.copy.copy_id.clone(),
        stamp,
        outcome: outcome.into(),
    }))
}

pub(crate) fn proof_matches(proof: &Proof) -> bool {
    proof.outcome == "present"
}

pub(crate) fn verification_job(
    candidates: Vec<LibraryCopyEvidence>,
    root: PathBuf,
    bdl: Arc<vua_bdl_store::BdlStore>,
    copy_writes: Arc<Mutex<()>>,
    checks: Arc<Mutex<ContentChecks>>,
    trigger: String,
) -> vua_orchestrator::TaskJob {
    Box::new(move |ctx| {
        let total = candidates.len();
        let mut checked = 0;
        let mut matched = 0;
        ctx.emit_progress(
            json!({"operation":OPERATION,"checked":checked,"matched":matched,"total":total}),
        );
        for planned in candidates {
            if ctx.check_cancel() {
                return Ok(vua_orchestrator::TaskExit::Cancelled);
            }
            let still_current = || -> Result<bool, vua_orchestrator::AppErrorV1> {
                Ok(bdl
                    .entry_copies(&planned.copy.warehouse_item_id)
                    .map_err(|_| {
                        vua_orchestrator::AppErrorV1::new(
                            "vua.library.store_failed",
                            vua_orchestrator::ErrorCategory::Internal,
                            "errors.library.reconciliationFailed",
                            &trigger,
                        )
                    })?
                    .into_iter()
                    .any(|c| {
                        c.copy_id == planned.copy.copy_id
                            && c.artifact_sha256 == planned.copy.artifact_sha256
                            && c.stored_path == planned.copy.stored_path
                            && c.role == planned.copy.role
                    }))
            };
            let current = {
                let _guard = copy_writes.lock().expect("library copy writes poisoned");
                still_current()? && crate::library_view::presence(&root, &planned) == "present"
            };
            // Hash outside the write lock so listing, deletion previews and
            // cancellation stay responsive. Check drift again before publishing.
            let mut proof = if current {
                match verify(&planned, ctx) {
                    Ok(proof) => proof,
                    Err(()) => return Ok(vua_orchestrator::TaskExit::Cancelled),
                }
            } else {
                None
            };
            if ctx.check_cancel() {
                return Ok(vua_orchestrator::TaskExit::Cancelled);
            }
            let _guard = copy_writes.lock().expect("library copy writes poisoned");
            if proof
                .as_ref()
                .is_some_and(|p| Stamp::read(&planned).as_ref() != Some(&p.stamp))
                || !still_current()?
            {
                proof = None;
            }
            checked += 1;
            if proof.as_ref().is_some_and(proof_matches) {
                matched += 1;
            } else {
                ctx.warn();
            }
            ctx.emit_progress(json!({"operation":OPERATION,"checked":checked,"matched":matched,"total":total,"proof":proof}));
            if ctx.check_cancel() {
                return Ok(vua_orchestrator::TaskExit::Cancelled);
            }
            if let Some(proof) = proof {
                checks
                    .lock()
                    .expect("library checks poisoned")
                    .insert(proof);
            }
        }
        Ok(vua_orchestrator::TaskExit::Done(
            json!({"operation":OPERATION,"checked":checked,"matched":matched,"total":total}),
        ))
    })
}

pub(crate) struct Grouping {
    pub assignments: HashMap<String, HashSet<String>>,
    pub reference_hashes: HashMap<String, HashSet<String>>,
}

pub(crate) fn group(
    entries: &[WarehouseEntryCard],
    products: &[CatalogProductSummary],
    copies: &[LibraryCopyEvidence],
    managed_products: &HashMap<i64, String>,
    presences: &mut HashMap<String, &'static str>,
    checks: &ContentChecks,
    explicit_sources: &HashMap<String, String>,
) -> Grouping {
    let product_ids: HashSet<_> = products.iter().map(|p| p.product_id.as_str()).collect();
    let downloaded: HashSet<_> = entries
        .iter()
        .filter(|e| e.kind == "downloaded_material")
        .map(|e| e.warehouse_item_id.as_str())
        .collect();
    let mut assignments: HashMap<String, HashSet<String>> = HashMap::new();
    let mut reference_hashes: HashMap<String, HashSet<String>> = HashMap::new();
    let mut hash_products: HashMap<&str, HashSet<String>> = HashMap::new();
    // Only inspected account downloads establish reference bytes. A user mapping
    // or a matching title cannot manufacture an official fingerprint.
    for copy in copies {
        let managed_product = copy
            .downloadable_id
            .or(copy.archive_downloadable_id)
            .and_then(|id| managed_products.get(&id))
            .filter(|id| product_ids.contains(id.as_str()));
        let ids: HashSet<_> = if let Some(id) = managed_product {
            HashSet::from([id.clone()])
        } else if downloaded.contains(copy.copy.warehouse_item_id.as_str()) {
            copy.product_ids
                .iter()
                .filter(|id| product_ids.contains(id.as_str()))
                .cloned()
                .collect()
        } else {
            continue;
        };
        if ids.is_empty() {
            continue;
        }
        assignments.insert(copy.copy.copy_id.clone(), ids.clone());
        if copy.copy.role == CopyRole::Original {
            if let Some(id) =
                managed_product.filter(|_| copy.downloadable_id.is_some() || copy.archive_current)
            {
                let id = id.clone();
                reference_hashes
                    .entry(id.clone())
                    .or_default()
                    .insert(copy.copy.artifact_sha256.clone());
                hash_products
                    .entry(&copy.copy.artifact_sha256)
                    .or_default()
                    .insert(id);
            }
        }
    }
    for copy in copies.iter().filter(|c| c.copy.role == CopyRole::Original) {
        if assignments.contains_key(&copy.copy.copy_id) {
            continue;
        }
        let Some(ids) = hash_products.get(copy.copy.artifact_sha256.as_str()) else {
            continue;
        };
        let ids: HashSet<_> = ids
            .iter()
            .filter(|id| {
                explicit_sources
                    .get(&copy.copy.warehouse_item_id)
                    .is_none_or(|source| source == *id)
            })
            .cloned()
            .collect();
        if ids.is_empty() {
            continue;
        }
        let state = presences[&copy.copy.copy_id];
        // Explicit deletion does not erase a previously verified content identity.
        // Reappearing bytes must pass the metadata/proof check again.
        let result = if state == "missing" && checks.previously_verified(copy) {
            "missing"
        } else if state == "present" {
            let Some(result) = checks.check(copy) else {
                continue;
            };
            result
        } else {
            continue;
        };
        presences.insert(copy.copy.copy_id.clone(), result);
        if result == "present" || result == "missing" {
            assignments.insert(copy.copy.copy_id.clone(), ids);
        }
    }
    let mut copies_of_entry: HashMap<&str, Vec<&LibraryCopyEvidence>> = HashMap::new();
    for copy in copies {
        copies_of_entry
            .entry(copy.copy.warehouse_item_id.as_str())
            .or_default()
            .push(copy);
    }
    // A derivative is grouped only when every original in its entry has matched
    // the same product. A partial match must not swallow unrelated local files.
    for entry in entries
        .iter()
        .filter(|e| !downloaded.contains(e.warehouse_item_id.as_str()))
    {
        let Some(own) = copies_of_entry.get(entry.warehouse_item_id.as_str()) else {
            continue;
        };
        let originals: Vec<_> = own
            .iter()
            .filter(|c| c.copy.role == CopyRole::Original)
            .collect();
        let Some(first) = originals
            .first()
            .and_then(|c| assignments.get(&c.copy.copy_id))
        else {
            continue;
        };
        let common: HashSet<_> = first
            .iter()
            .filter(|id| {
                originals.iter().all(|c| {
                    assignments
                        .get(&c.copy.copy_id)
                        .is_some_and(|ids| ids.contains(*id))
                })
            })
            .cloned()
            .collect();
        if common.is_empty() {
            continue;
        }
        for copy in own.iter().filter(|c| c.copy.role == CopyRole::GeneratedVpm) {
            assignments.insert(copy.copy.copy_id.clone(), common.clone());
        }
    }
    Grouping {
        assignments,
        reference_hashes,
    }
}

fn normalized_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The products one library read matches its local entries against. Every entry
/// asks, so the products are indexed once.
pub(crate) struct SourceIndex<'a> {
    by_id: HashMap<&'a str, &'a CatalogProductSummary>,
    /// BOOTH ids by number, for numbers long enough to mean something inside a name.
    by_number: HashMap<&'a str, Vec<&'a str>>,
    by_title: HashMap<String, Vec<&'a str>>,
    booth_ids: Vec<&'a str>,
    other_ids: Vec<&'a str>,
}

impl<'a> SourceIndex<'a> {
    pub(crate) fn new(products: &'a [CatalogProductSummary]) -> Self {
        let mut index = Self {
            by_id: HashMap::with_capacity(products.len()),
            by_number: HashMap::new(),
            by_title: HashMap::new(),
            booth_ids: Vec::new(),
            other_ids: Vec::new(),
        };
        for product in products {
            let id = product.product_id.as_str();
            index.by_id.insert(id, product);
            match id.strip_prefix("booth:") {
                Some(number) => {
                    index.booth_ids.push(id);
                    if number.len() >= 5 {
                        index.by_number.entry(number).or_default().push(id);
                    }
                }
                None => index.other_ids.push(id),
            }
            if let Some(title) = &product.title {
                index
                    .by_title
                    .entry(normalized_name(title))
                    .or_default()
                    .push(id);
            }
        }
        index
    }

    pub(crate) fn product(&self, id: &str) -> Option<&'a CatalogProductSummary> {
        self.by_id.get(id).copied()
    }

    /// Products a name states outright: their whole id, or their number as a whole run of digits.
    fn named_by(&self, names: &[&str]) -> HashSet<&'a str> {
        let mut named = HashSet::new();
        for name in names {
            for digits in name.split(|c: char| !c.is_ascii_digit()) {
                if let Some(ids) = self.by_number.get(digits) {
                    named.extend(ids.iter().copied());
                }
            }
            if name.contains("booth:") {
                named.extend(
                    self.booth_ids
                        .iter()
                        .copied()
                        .filter(|id| name.contains(id)),
                );
            }
            named.extend(
                self.other_ids
                    .iter()
                    .copied()
                    .filter(|id| name.contains(id)),
            );
        }
        named
    }
}

/// Suggestions are deterministic, local, and unambiguous. They are read-model
/// hints, not persisted user confirmations or compatibility claims.
pub(crate) fn source_hint(
    entry: &WarehouseEntryCard,
    copies: &[&LibraryCopyEvidence],
    products: &SourceIndex,
    grouping: &Grouping,
    explicit_source: Option<&str>,
) -> Option<(&'static str, String, &'static str)> {
    let mapped: HashSet<_> = copies
        .iter()
        .filter(|c| c.copy.role == CopyRole::Original)
        .flat_map(|c| &c.product_ids)
        .filter(|id| products.by_id.contains_key(id.as_str()))
        .cloned()
        .collect();
    let (basis, id) = if let Some(id) = explicit_source.filter(|id| products.by_id.contains_key(id))
    {
        ("mapping", id.to_owned())
    } else if !mapped.is_empty() {
        if mapped.len() != 1 {
            return None;
        }
        ("mapping", mapped.into_iter().next()?)
    } else {
        let names: Vec<_> = std::iter::once(entry.display_name.as_str())
            .chain(
                copies
                    .iter()
                    .filter(|c| c.copy.role == CopyRole::Original)
                    .map(|c| c.copy.relative_path.as_str()),
            )
            .collect();
        let named = products.named_by(&names);
        if named.len() > 1 {
            return None;
        }
        if let Some(id) = named.into_iter().next() {
            ("product_id", id.to_owned())
        } else {
            let names: HashSet<_> = names
                .iter()
                .flat_map(|name| {
                    let path = Path::new(name);
                    [
                        normalized_name(name),
                        normalized_name(path.file_stem().and_then(|s| s.to_str()).unwrap_or(name)),
                    ]
                })
                .collect();
            let matches: Vec<_> = names
                .iter()
                .filter_map(|name| products.by_title.get(name))
                .flatten()
                .collect();
            if matches.len() != 1 {
                return None;
            }
            ("name", (*matches[0]).to_owned())
        }
    };
    let content = if grouping.reference_hashes.get(&id).is_some_and(|hashes| {
        copies
            .iter()
            .any(|c| c.copy.role == CopyRole::Original && !hashes.contains(&c.copy.artifact_sha256))
    }) {
        "different"
    } else {
        "unverified"
    };
    Some((basis, id, content))
}

pub(crate) fn hint_value(
    product: &CatalogProductSummary,
    sources: &[String],
    basis: &str,
    content: &str,
) -> Value {
    json!({"product":product,"sources":sources,"basis":basis,"content":content})
}

#[cfg(test)]
mod source_index_tests {
    use super::*;
    use vua_bdl_store::bdl_queries::AvailabilityStatus;

    fn product(id: &str) -> CatalogProductSummary {
        CatalogProductSummary {
            product_id: id.to_owned(),
            title: None,
            library_type: None,
            imported_artifacts: 0,
            shop_name: None,
            variant_name: None,
            price: None,
            image_url: None,
            image_urls: Vec::new(),
            availability_raw: None,
            availability_status: AvailabilityStatus::Unknown,
            entity_count: 0,
            entity_types: Vec::new(),
        }
    }

    #[test]
    fn a_name_states_a_product_by_its_whole_id_or_its_whole_number() {
        let products = [
            product("booth:123456"),
            product("booth:1234"),
            product("other:abc"),
        ];
        let index = SourceIndex::new(&products);
        let named = |name| {
            let mut ids: Vec<_> = index.named_by(&[name]).into_iter().collect();
            ids.sort_unstable();
            ids
        };
        assert_eq!(named("pack_123456_v2"), ["booth:123456"]);
        assert_eq!(named("from booth:1234 page"), ["booth:1234"]);
        assert_eq!(named("see other:abc"), ["other:abc"]);
        assert!(
            named("1234567").is_empty(),
            "a longer run of digits is another number"
        );
        assert!(
            named("version 1234").is_empty(),
            "short numbers need the whole id"
        );
        assert_eq!(
            index.product("booth:1234").map(|p| p.product_id.as_str()),
            Some("booth:1234")
        );
        assert!(index.product("booth:9").is_none());
    }
}
