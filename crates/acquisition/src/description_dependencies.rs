//! BOOTH description links are unconfirmed relationship clues, not compatibility verdicts.
use scraper::{Html, Selector};
use std::collections::HashSet;
use vua_bdl_store::{
    BdlStore, BdlStoreError, DependencyResolutionEvidence, NewDependencyObservation,
};

/// The source product must already be recorded. Only the supplied author-description
/// snapshot is inspected; no page fetching or automatic dependency acquisition occurs.
pub fn record_description_dependencies(
    store: &BdlStore,
    html: &str,
    source_product_id: &str,
    page_hash: &str,
    observed_at: &str,
) -> Result<(), BdlStoreError> {
    let document = Html::parse_document(html);
    let body_selector = Selector::parse("#items article .js-market-item-detail-description.description").expect("static description selector");
    let links = Selector::parse("a[href]").expect("static link selector");
    let bodies: Vec<_> = document.select(&body_selector).collect();
    // A missing description container is unknown, not an observed empty body.
    if bodies.is_empty() { return Ok(()); }
    let mut seen = HashSet::new();
    let mut named_links = HashSet::new();
    let mut targets = Vec::new();
    for link in bodies.iter().flat_map(|body| body.select(&links)) {
        let href = link.value().attr("href").expect("selected link has href");
        let Some(target_product_id) = booth_product_id(href) else {
            continue;
        };
        if target_product_id == source_product_id {
            continue;
        }
        if seen.insert(target_product_id.clone()) { targets.push(target_product_id.clone()); }
        let text = link.text().collect::<Vec<_>>().join(" ");
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        // Image-only links still participate in reverse link filtering.
        // Retain the older named-dependency vocabulary unchanged.
        if text.is_empty() || !named_links.insert(target_product_id.clone()) { continue; }
        let lowered = text.to_lowercase();
        let dep_kind = if lowered.contains("shader")
            || lowered.contains("シェーダー")
            || lowered.contains("liltoon")
            || lowered.contains("poiyomi")
        {
            "shader"
        } else {
            "other"
        };
        let in_catalog = store.catalog_detail(&target_product_id)?.is_some();
        let evidence = if in_catalog {
            vec![DependencyResolutionEvidence {
                link_text: text.clone(),
                link_url: href.to_owned(),
                span: "description_link".to_owned(),
                note: None,
            }]
        } else {
            Vec::new()
        };
        store.record_dependency_observation(&NewDependencyObservation {
            product_id: source_product_id.to_owned(),
            dep_kind: dep_kind.to_owned(),
            dep_name: text,
            raw_quote: href.to_owned(),
            source_span: "description_link".to_owned(),
            version_hint: None,
            resolved_ref_product_id: in_catalog.then_some(target_product_id),
            resolution_evidence: evidence,
            extraction_method: "link".to_owned(),
            extracted_by: "catalog-sync/0.2-description-links-v2".to_owned(),
            observed_at: observed_at.to_owned(),
            processor_version: "catalog-sync/0.2-description-links-v2".to_owned(),
            content_hash: Some(page_hash.to_owned()),
            run_id: None,
        })?;
    }
    store.record_description_links(source_product_id, &targets, page_hash, observed_at)?;
    Ok(())
}

pub(crate) fn booth_product_id(href: &str) -> Option<String> {
    let href = href.trim();
    let (host, path) = if href.starts_with('/') && !href.starts_with("//") { ("booth.pm", href.strip_prefix('/')?) }
        else { href.strip_prefix("https://").or_else(|| href.strip_prefix("http://")).or_else(|| href.strip_prefix("//"))?.split_once('/')? };
    let host = host.to_ascii_lowercase();
    if host.contains(['@', ':']) || !(host == "booth.pm" || host.ends_with(".booth.pm")) {
        return None;
    }
    let path = path.split(['?', '#']).next()?;
    let mut segments = path.split('/');
    let first = segments.next()?;
    if first != "items" && (!["ja", "en", "ko", "zh-cn", "zh-tw"].contains(&first) || segments.next()? != "items") {
        return None;
    }
    let native_id = segments.next()?;
    if native_id.is_empty()
        || !native_id.bytes().all(|byte| byte.is_ascii_digit())
        || native_id.parse::<u64>().ok()? == 0
        || segments.any(|part| !part.is_empty())
    {
        return None;
    }
    Some(format!("booth:{native_id}"))
}

/// Older builds recorded description links as immutable dependency evidence.
/// Use only the latest captured link-page for sources without a new snapshot;
/// names, manually confirmed dependencies and title links never qualify.
pub(crate) fn products_linking_to(store: &BdlStore, target: &str) -> Result<HashSet<String>, BdlStoreError> {
    let mut snapshots = store.library_description_links()?;
    let mut legacy = std::collections::HashMap::new();
    for row in store.dependency_observations_all()? {
        if row.source_span == "description_link" && row.extraction_method == "link" && row.extracted_by == "catalog-sync/0.2-description-links-v2" {
            let entry = legacy.entry(row.product_id.clone()).or_insert_with(|| (row.content_hash.clone(), Vec::new()));
            if entry.0 != row.content_hash { *entry = (row.content_hash, Vec::new()); }
            if let Some(id) = booth_product_id(&row.raw_quote) { entry.1.push(id); }
        }
    }
    for (source, (_, ids)) in legacy { snapshots.entry(source).or_insert(ids); }
    Ok(snapshots.into_iter().filter(|(source, ids)| source != target && ids.iter().any(|id| id == target)).map(|(source, _)| source).collect())
}
