//! BDL recognition: BOOTH product page extraction.
//!
//! A pure, read-only function of an archived product page. Every rule and
//! fallback chain comes verbatim from the extraction spec
//! (`docs/research/booth-product-extraction`): primary source, fallback
//! source, and the no-inference principle. Fields the page does not carry
//! are reported as `missing_fields` entries — never guessed, never defaulted
//! (the spec's standing nulls are honored here, e.g. `source_updated_at`
//! does not exist at all).
//!
//! Trust model: the output is a fact only together with the provenance of
//! the HTML it came from (`content_hash` + `observed_at` + processor
//! version, attached by the caller).

use scraper::{ElementRef, Html, Selector};
use serde::Serialize;
use serde_json::Value;
use std::fmt;

/// Extraction failures are structural: the page is not a product page at
/// all. Missing *fields* are findings on a successful extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractionError {
    ProductRootMissing,
    ProductIdMissing,
}

impl fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProductRootMissing => write!(f, "no #items product root found"),
            Self::ProductIdMissing => write!(f, "#items carries no data-product-id"),
        }
    }
}

impl std::error::Error for ExtractionError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedSubproduct {
    pub variation_id: Option<String>,
    pub name: Option<String>,
    pub price_amount: Option<String>,
    pub price_currency: Option<String>,
    /// `available` or `sold_out` (a sold-out option is recorded, never removed).
    pub availability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedProduct {
    pub native_product_id: String,
    pub source_category: Option<String>,
    /// Which title source answered: `h2` (primary) or `ld_json` (fallback).
    pub title_source: Option<&'static str>,
    pub title: Option<String>,
    pub shop_name: Option<String>,
    pub shop_url: Option<String>,
    /// True only with BOOTH's explicit Adult badge — never inferred.
    pub adult: bool,
    pub likes: Option<String>,
    pub published_date_raw: Option<String>,
    pub availability: Option<String>,
    pub price_amount: Option<String>,
    pub price_currency: Option<String>,
    pub image_urls: Vec<String>,
    pub video_urls: Vec<String>,
    pub subproducts: Vec<ExtractedSubproduct>,
    pub body_text: Option<String>,
    pub search_text: Option<String>,
    pub search_text_normalized: Option<String>,
    /// Spec data names the page did not carry (honest gaps, never guesses).
    pub missing_fields: Vec<&'static str>,
}

/// Extracts one product page per the spec. The `aside` (related products
/// and shops) and the `#js-item` mount point are structurally excluded.
pub fn extract_product_page(html: &str) -> Result<ExtractedProduct, ExtractionError> {
    let document = Html::parse_document(html);
    let items = Selector::parse("#items").expect("static selector");
    let root = document.select(&items).next().ok_or(ExtractionError::ProductRootMissing)?;
    let article = Selector::parse("article").expect("static selector");
    let root_article = root.select(&article).next();

    let native_product_id = root
        .value()
        .attr("data-product-id")
        .map(str::to_owned)
        .ok_or(ExtractionError::ProductIdMissing)?;
    let mut missing = Vec::new();

    // Category: root attribute first, breadcrumbs only as fallback, never
    // inferred. The breadcrumbs live at document level.
    let source_category = root
        .value()
        .attr("data-shop-tracking-product-category")
        .map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned)
        .or_else(|| {
            let crumbs = Selector::parse("#js-item-category-breadcrumbs li").expect("static selector");
            let last = document
                .select(&crumbs)
                .map(text_of)
                .rfind(|text| !text.is_empty());
            if last.is_some() {
                last
            } else {
                missing.push("category");
                None
            }
        });

    // Title: primary h2, JSON-LD Product.name as fallback (the
    // data-product-name attribute was observed truncated).
    let summary = Selector::parse("article .summary").expect("static selector");
    let summary = root_article.as_ref().and_then(|article| article.select(&summary).next());
    let title_h2 = Selector::parse("h2").expect("static selector");
    let title_h2 = summary.as_ref().and_then(|summary| {
        summary.select(&title_h2).next().map(text_of).filter(|text| !text.is_empty())
    });
    let (title, title_source) = if let Some(title) = title_h2 {
        (Some(title), Some("h2"))
    } else if let Some(name) = json_ld_products(&document)
        .iter()
        .find_map(|product| string_field(product, &["name"]))
    {
        (Some(name), Some("ld_json"))
    } else {
        missing.push("title");
        (None, None)
    };

    let shop_name_selector = Selector::parse("article .shop-name").expect("static selector");
    let shop_name = root_article.as_ref().and_then(|article| {
        article.select(&shop_name_selector).next().map(text_of).filter(|text| !text.is_empty())
    });
    if shop_name.is_none() {
        missing.push("shop_name");
    }

    // Adult: only the explicit badge with the literal text counts.
    let adult_badge = Selector::parse("[class~='bg-primary700']").expect("static selector");
    let adult = summary
        .as_ref()
        .map(|summary| {
            summary
                .select(&adult_badge)
                .any(|badge| text_of(badge).trim_eq_ignore_case("adult"))
        })
        .unwrap_or(false);

    let likes_selector = Selector::parse("#js-item-wishlist-button").expect("static selector");
    let likes = document
        .select(&likes_selector)
        .next()
        .map(text_of)
        .filter(|text| !text.is_empty());
    if likes.is_none() {
        missing.push("likes");
    }

    let published = Selector::parse("#js-item-published-date").expect("static selector");
    let published_date_raw = document
        .select(&published)
        .next()
        .map(text_of)
        .filter(|text| !text.is_empty());
    if published_date_raw.is_none() {
        missing.push("published_date");
    }

    // Single price: root attribute or JSON-LD offer; complex-option products
    // legitimately have neither (the variations carry the prices).
    let json_ld = json_ld_products(&document);
    let offer = json_ld.iter().find_map(offer_of);
    let price_amount = root
        .value()
        .attr("data-product-price")
        .map(str::to_owned)
        .or_else(|| offer.as_ref().and_then(|offer| string_field(offer, &["price"])));
    if price_amount.is_none() {
        missing.push("single_price");
    }
    let price_currency = offer.as_ref().and_then(|offer| string_field(offer, &["priceCurrency"]));
    if price_currency.is_none() {
        missing.push("price_currency");
    }
    let availability = offer.as_ref().and_then(|offer| string_field(offer, &["availability"]))
        .map(|value| {
            value
                .rsplit('/')
                .next()
                .unwrap_or(&value)
                .to_owned()
        });
    if availability.is_none() {
        missing.push("availability");
    }
    let shop_url = json_ld.iter().find_map(brand_url);

    // Images: gallery originals first (data-origin), src/data-lazy as
    // fallbacks. Body images stay out unless confirmed to belong to the
    // product — the static archive cannot confirm that.
    let gallery = Selector::parse(".primary-image-area .market-item-detail-item-image")
        .expect("static selector");
    let mut image_urls = Vec::new();
    if let Some(article) = root_article.as_ref() {
        for image in article.select(&gallery) {
            let value = image
                .value()
                .attr("data-origin")
                .or_else(|| image.value().attr("src"))
                .or_else(|| image.value().attr("data-lazy"));
            if let Some(url) = value {
                if !image_urls.iter().any(|existing| existing == url) {
                    image_urls.push(url.to_owned());
                }
            }
        }
    }
    if image_urls.is_empty() {
        missing.push("images");
    }

    // Videos: explicit <video> sources plus recognizable platform embeds;
    // ordinary download links and trial pages never qualify.
    let mut video_urls = Vec::new();
    let video = Selector::parse("video").expect("static selector");
    let source = Selector::parse("source").expect("static selector");
    for element in root.select(&video) {
        if let Some(url) = element.value().attr("src") {
            push_unique(&mut video_urls, url);
        }
        for child in element.select(&source) {
            if let Some(url) = child.value().attr("src") {
                push_unique(&mut video_urls, url);
            }
        }
    }
    let iframe = Selector::parse("iframe").expect("static selector");
    for frame in root.select(&iframe) {
        if let Some(url) = frame.value().attr("src") {
            if is_embedded_video_url(url) {
                push_unique(&mut video_urls, url);
            }
        }
    }
    if video_urls.is_empty() {
        missing.push("videos");
    }

    // Subproducts: one entry per sold option, deduped by variation ID
    // (cart and gift controls may repeat the same ID).
    let mut subproducts = Vec::new();
    let mut seen_variations = Vec::new();
    if let Some(article) = root_article.as_ref() {
        let variation_item = Selector::parse("#variations li.variation-item").expect("static selector");
        let variation_name = Selector::parse(".variation-name").expect("static selector");
        let variation_price = Selector::parse(".variation-price").expect("static selector");
        let variant_attr = Selector::parse("[data-product-variant]").expect("static selector");
        let variant_input =
            Selector::parse("input[name='cart_item[variation_id]']").expect("static selector");
        for item in article.select(&variation_item) {
            let mut variation_id = item
                .select(&variant_attr)
                .next()
                .and_then(|element| element.value().attr("data-product-variant"))
                .map(str::to_owned);
            if variation_id.is_none() {
                variation_id = item.select(&variant_input).next().and_then(|input| {
                    input.value().attr("value").map(str::to_owned)
                });
            }
            if let Some(id) = &variation_id {
                if seen_variations.iter().any(|seen| seen == id) {
                    continue;
                }
                seen_variations.push(id.clone());
            }
            let price_text = item.select(&variation_price).next().map(text_of);
            let (price_amount, price_currency) = match &price_text {
                Some(text) => split_price(text),
                None => (None, None),
            };
            // The sold-out marker sits on the li itself, not a descendant.
            let sold_out = item.value().classes().any(|class| class == "sold-out");
            subproducts.push(ExtractedSubproduct {
                variation_id,
                name: item.select(&variation_name).next().map(text_of).filter(|text| !text.is_empty()),
                price_amount,
                price_currency,
                availability: if sold_out {
                    "sold_out".to_owned()
                } else {
                    "available".to_owned()
                },
            });
        }
    }
    if subproducts.is_empty() {
        missing.push("subproducts");
    }

    // Body: visible description text, then the shop sections appended in
    // DOM order. Links stay inside the text — they are not extracted.
    let body_selector =
        Selector::parse(".js-market-item-detail-description.description").expect("static selector");
    let shop_text = Selector::parse("article section.shop__text").expect("static selector");
    let mut body_parts = Vec::new();
    if let Some(article) = root_article.as_ref() {
        if let Some(description) = article.select(&body_selector).next() {
            let text = text_of(description);
            if !text.is_empty() {
                body_parts.push(text);
            }
        }
        for section in article.select(&shop_text) {
            let text = text_of(section);
            if !text.is_empty() {
                body_parts.push(text);
            }
        }
    }
    let body_text = if body_parts.is_empty() {
        missing.push("body");
        None
    } else {
        Some(body_parts.join(" "))
    };

    // Search projection: body, title, author, subproduct names, source
    // category — one string, links included in the body.
    let mut projection_parts: Vec<String> = Vec::new();
    if let Some(body) = &body_text {
        projection_parts.push(body.clone());
    }
    if let Some(title) = &title {
        projection_parts.push(title.clone());
    }
    if let Some(shop) = &shop_name {
        projection_parts.push(shop.clone());
    }
    for subproduct in &subproducts {
        if let Some(name) = &subproduct.name {
            projection_parts.push(name.clone());
        }
    }
    if let Some(category) = &source_category {
        projection_parts.push(category.clone());
    }
    let search_text = if projection_parts.is_empty() {
        None
    } else {
        Some(projection_parts.join(" "))
    };
    let search_text_normalized =
        search_text.as_ref().map(|text| normalize_search_text(text));

    Ok(ExtractedProduct {
        native_product_id,
        source_category,
        title_source,
        title,
        shop_name,
        shop_url,
        adult,
        likes,
        published_date_raw,
        availability,
        price_amount,
        price_currency,
        image_urls,
        video_urls,
        subproducts,
        body_text,
        search_text,
        search_text_normalized,
        missing_fields: missing,
    })
}

// --- helpers ---

fn text_of(element: ElementRef) -> String {
    let joined = element
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    joined
}

trait TrimEqIgnoreCase {
    fn trim_eq_ignore_case(&self, other: &str) -> bool;
}

impl TrimEqIgnoreCase for String {
    fn trim_eq_ignore_case(&self, other: &str) -> bool {
        self.trim().eq_ignore_ascii_case(other)
    }
}

fn push_unique(urls: &mut Vec<String>, url: &str) {
    if !urls.iter().any(|existing| existing == url) {
        urls.push(url.to_owned());
    }
}

fn is_embedded_video_url(url: &str) -> bool {
    let lowered = url.to_ascii_lowercase();
    lowered.contains("youtube.com") || lowered.contains("youtu.be") || lowered.contains("vimeo.com")
}

/// Splits a displayed variation price (`"1,500 JPY"`) into amount and
/// currency when a trailing currency token is present.
fn split_price(displayed: &str) -> (Option<String>, Option<String>) {
    let trimmed = displayed.trim();
    let mut parts = trimmed.split_whitespace();
    let head = parts.next().unwrap_or("");
    let tail = parts.next();
    match tail {
        Some(currency) if currency.len() == 3 && currency.chars().all(|c| c.is_ascii_uppercase()) => {
            (Some(head.to_owned()), Some(currency.to_owned()))
        }
        _ => {
            if trimmed.is_empty() {
                (None, None)
            } else {
                (Some(trimmed.to_owned()), None)
            }
        }
    }
}

fn normalize_search_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Parses every JSON-LD block and returns the Product objects (tolerant of
/// arrays, wrapped roots, and malformed blocks — a broken block is skipped,
/// not an error).
fn json_ld_products(document: &Html) -> Vec<Value> {
    let script = Selector::parse("script[type='application/ld+json']").expect("static selector");
    let mut products = Vec::new();
    for block in document.select(&script) {
        let Ok(value) = serde_json::from_str::<Value>(&text_of(block)) else {
            continue;
        };
        collect_product_values(&value, &mut products);
    }
    products
}

fn collect_product_values(value: &Value, products: &mut Vec<Value>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_product_values(item, products);
            }
        }
        Value::Object(object) => {
            let is_product = match object.get("@type") {
                Some(Value::String(kind)) => kind == "Product",
                Some(Value::Array(kinds)) => kinds
                    .iter()
                    .any(|kind| kind.as_str() == Some("Product")),
                _ => false,
            };
            if is_product {
                products.push(value.clone());
            }
            for child in object.values() {
                collect_product_values(child, products);
            }
        }
        _ => {}
    }
}

fn string_field(object: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        match object.get(*key) {
            Some(Value::String(text)) if !text.is_empty() => return Some(text.clone()),
            Some(Value::Number(number)) => return Some(number.to_string()),
            _ => {}
        }
    }
    None
}

fn offer_of(product: &Value) -> Option<Value> {
    match product.get("offers") {
        Some(offer @ Value::Object(_)) => Some(offer.clone()),
        Some(Value::Array(items)) => items.first().cloned(),
        _ => None,
    }
}

fn brand_url(product: &Value) -> Option<String> {
    match product.get("brand") {
        Some(brand @ Value::Object(_)) => string_field(brand, &["url"]),
        Some(Value::String(url)) if !url.is_empty() => Some(url.clone()),
        _ => None,
    }
}
