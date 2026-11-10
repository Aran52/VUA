//! Optional public BOOTH JSON representation. Unknown upstream fields are ignored.
use super::{
    normalize_description, normalize_search_text, ExtractedProduct, ExtractedSubproduct,
    ExtractionError,
};
use serde_json::Value;
use std::collections::HashSet;

pub fn extract_product_json(body: &str) -> Result<ExtractedProduct, ExtractionError> {
    if body.len() > 5_000_000 {
        return Err(ExtractionError::InvalidProductJson);
    }
    let value: Value =
        serde_json::from_str(body).map_err(|_| ExtractionError::InvalidProductJson)?;
    if !value.is_object() {
        return Err(ExtractionError::InvalidProductJson);
    }
    let native_product_id = identity(&value["id"]).ok_or(ExtractionError::InvalidProductJson)?;
    let mut missing = Vec::new();
    let title = text(&value["name"]);
    let source_category = text(&value["category"]["name"]);
    let shop_name = text(&value["shop"]["name"]);
    let shop_url = https_url(&value["shop"]["url"]);
    let body_text = value["description"]
        .as_str()
        .map(normalize_description)
        .filter(|s| !s.is_empty());
    let published_date_raw = text(&value["published_at"]).filter(|s| valid_published_date(s));
    let adult = value["is_adult"].as_bool();
    let availability = match (
        value["is_end_of_sale"].as_bool(),
        value["is_sold_out"].as_bool(),
    ) {
        (Some(true), _) => Some("Discontinued".to_owned()),
        (Some(false), Some(true)) => Some("OutOfStock".to_owned()),
        (Some(false), Some(false)) => Some("InStock".to_owned()),
        _ => None,
    };
    let likes = value["wish_lists_count"].as_u64().map(|n| n.to_string());
    // BOOTH's explicit yen-formatted display is the currency evidence. A price
    // range is not a single price and a bare numeric value has no currency.
    let price_display = value["price"].as_str().unwrap_or("").trim();
    let yen_price = price_display.strip_prefix('¥').map(str::trim);
    let single_price = yen_price.filter(|s| amount(s)).map(str::to_owned);
    let price_currency = single_price.as_ref().map(|_| "JPY".to_owned());
    let mut image_urls = Vec::new();
    for image in value["images"].as_array().into_iter().flatten() {
        if let Some(url) = https_url(&image["original"]) {
            if !image_urls.contains(&url) {
                image_urls.push(url);
            }
        }
    }
    let mut subproducts = Vec::new();
    let mut seen = HashSet::new();
    for variant in value["variations"].as_array().into_iter().flatten() {
        let Some(id) = identity(&variant["id"]) else {
            missing.push("subproduct_id");
            continue;
        };
        if !seen.insert(id.clone()) {
            continue;
        }
        let availability = if variant["is_empty_stock"].as_bool() == Some(true) {
            Some("sold_out".to_owned())
        } else if variant["status"].as_str() == Some("addable_to_cart") {
            Some("available".to_owned())
        } else {
            None
        };
        if availability.is_none() {
            missing.push("subproduct_availability");
        }
        let price_amount = yen_price
            .and_then(|_| variant["price"].as_u64())
            .map(|n| n.to_string());
        if price_amount.is_none() {
            missing.push("subproduct_price");
        }
        subproducts.push(ExtractedSubproduct {
            variation_id: Some(id),
            name: text(&variant["name"]),
            price_currency: price_amount.as_ref().map(|_| "JPY".to_owned()),
            price_amount,
            availability,
        });
    }
    for (field, absent) in [
        ("title", title.is_none()),
        ("category", source_category.is_none()),
        ("shop_name", shop_name.is_none()),
        ("shop_url", shop_url.is_none()),
        ("body", body_text.is_none()),
        ("published_date", published_date_raw.is_none()),
        ("adult_badge", adult.is_none()),
        ("availability", availability.is_none()),
        ("likes", likes.is_none()),
        ("single_price", single_price.is_none()),
        ("price_currency", price_currency.is_none()),
        ("images", image_urls.is_empty()),
        ("subproducts", subproducts.is_empty()),
        ("videos", true),
    ] {
        if absent {
            missing.push(field);
        }
    }
    missing.sort_unstable();
    missing.dedup();
    let mut projection: Vec<&str> = [
        body_text.as_deref(),
        title.as_deref(),
        shop_name.as_deref(),
        source_category.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect();
    projection.extend(subproducts.iter().filter_map(|v| v.name.as_deref()));
    let search_text = (!projection.is_empty()).then(|| projection.join(" "));
    let search_text_normalized = search_text.as_deref().map(normalize_search_text);
    Ok(ExtractedProduct {
        native_product_id,
        source_category,
        title_source: title.as_ref().map(|_| "json"),
        title,
        shop_name,
        shop_url,
        adult: adult.unwrap_or(false),
        likes,
        published_date_raw,
        availability,
        price_amount: single_price,
        price_currency,
        image_urls,
        video_urls: Vec::new(),
        subproducts,
        body_text,
        search_text,
        search_text_normalized,
        missing_fields: missing,
    })
}

fn text(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn identity(value: &Value) -> Option<String> {
    let id = value
        .as_u64()
        .filter(|id| *id > 0)
        .map(|id| id.to_string())
        .or_else(|| text(value))?;
    (!id.is_empty()
        && id.len() <= 20
        && id.bytes().all(|b| b.is_ascii_digit())
        && id.parse::<u64>().ok().is_some_and(|id| id > 0))
    .then_some(id)
}

fn https_url(value: &Value) -> Option<String> {
    let url = text(value)?;
    let host = url.strip_prefix("https://")?.split('/').next()?;
    (!host.is_empty() && !host.contains(['@', '?', '#']) && !url.chars().any(char::is_whitespace))
        .then_some(url)
}

fn amount(value: &str) -> bool {
    let groups: Vec<_> = value.split(',').collect();
    groups
        .iter()
        .all(|group| !group.is_empty() && group.bytes().all(|b| b.is_ascii_digit()))
        && (groups.len() == 1 || groups[0].len() <= 3 && groups[1..].iter().all(|g| g.len() == 3))
}

fn valid_published_date(value: &str) -> bool {
    crate::rfc3339_offset_millis(value).is_some()
}
