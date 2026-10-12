//! Bounded extraction of a user-requested public BOOTH search; no catalog writes.
use serde::Deserialize;
use serde_json::{json, Value};
use scraper::{Html, Selector};
use crate::library_download::LibraryDownloadError;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Search { schema_version: String, query: String, source_url: String, html: String }

fn public_search_url(query: &str) -> String {
    // Mirrors encodeURIComponent: encode UTF-8 bytes, preserving only its unreserved set.
    let mut url = "https://booth.pm/ja/search/".to_owned();
    for byte in query.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) { url.push(byte as char); }
        else { url.push_str(&format!("%{byte:02X}")); }
    }
    url
}

pub fn search(params: Value) -> Result<Value, LibraryDownloadError> {
    let p: Search = serde_json::from_value(params)?;
    if p.schema_version != "0.1" || p.query.trim() != p.query || p.query.is_empty() || p.query.chars().count() > 200
        || p.query.chars().any(|c| c.is_control()) || p.html.len() > 5_000_000
        || p.source_url != public_search_url(&p.query) {
        return Err(LibraryDownloadError("invalid_params"));
    }
    let doc = Html::parse_document(&p.html);
    let cards = Selector::parse("li.item-card[data-product-id]").expect("static selector");
    let title = Selector::parse(".item-card__title, .item-card__name").expect("static selector");
    let shop = Selector::parse(".item-card__shop-name, .item-card__shop").expect("static selector");
    let shell = Selector::parse(".market-items, .pager, .search-result, #js-search-result").expect("static selector");
    if doc.select(&cards).next().is_none() && doc.select(&shell).next().is_none() {
        return Err(LibraryDownloadError("search_unavailable"));
    }
    let text = |element: scraper::ElementRef<'_>| element.text().collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
    let mut items = Vec::new();
    let mut ids = std::collections::HashSet::new();
    let mut truncated = doc.select(&Selector::parse("a[rel='next']").expect("static selector")).next().is_some();
    for card in doc.select(&cards) {
        let id = card.value().attr("data-product-id").unwrap_or("");
        if id.is_empty() || !id.bytes().all(|c| c.is_ascii_digit()) || !ids.insert(id.to_owned()) { continue; }
        let name = card.select(&title).next().map(text).filter(|s| !s.is_empty())
            .or_else(|| card.value().attr("data-product-name").filter(|s| !s.is_empty()).map(str::to_owned));
        let Some(name) = name.filter(|s| s.chars().count() <= 1000) else { truncated = true; continue; };
        if items.len() == 24 { truncated = true; break; }
        let shop_name = card.select(&shop).next().map(text).filter(|s| !s.is_empty() && s.chars().count() <= 500);
        let category = card.value().attr("data-product-category").map(str::trim).filter(|s| !s.is_empty() && s.chars().count() <= 500);
        items.push(json!({"productId":format!("booth:{id}"),"title":name,"shopName":shop_name,"sourceCategory":category}));
    }
    Ok(json!({"schemaVersion":"0.1","query":p.query,"items":items,"truncated":truncated}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_candidates_are_not_limited_to_a_signed_in_library() {
        let result = search(json!({"schemaVersion":"0.1","query":"Example","sourceUrl":"https://booth.pm/ja/search/Example",
          "html":"<ul class='market-items'><li class='item-card' data-product-id='90' data-product-category='3D衣装'><h2 class='item-card__title'>Example</h2></li><li class='item-card' data-product-id='90' data-product-name='Duplicate'></li></ul>"})).unwrap();
        assert_eq!(result["items"].as_array().unwrap().len(), 1);
        assert_eq!(result["items"][0]["productId"], "booth:90");
        assert_eq!(result["items"][0]["sourceCategory"], "3D衣装");
        assert!(search(json!({"schemaVersion":"0.1","query":"Example","sourceUrl":"https://accounts.booth.pm/library","html":""})).is_err());
        assert!(search(json!({"schemaVersion":"0.1","query":"Example","sourceUrl":"https://booth.pm/ja/search/Example","html":"<form>Sign in</form>"})).is_err());
    }
    #[test]
    fn empty_search_and_pagination_are_explicit() {
        let result = search(json!({"schemaVersion":"0.1","query":"none","sourceUrl":"https://booth.pm/ja/search/none","html":"<div class='market-items'></div><a rel='next'>2</a>"})).unwrap();
        assert_eq!(result["items"], json!([])); assert_eq!(result["truncated"], true);
    }
}
