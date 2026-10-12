//! BDL recognition tests: BOOTH product page extraction per the accepted
//! spec (docs/research/booth-product-extraction). CI runs structurally
//! equivalent synthetic fixtures only; the golden anchors (corpus products
//! 4431242 / 5986971) run locally behind an ignored manual test.

use std::path::{Path, PathBuf};

use vua_orchestrator::{extract_product_page, ExtractedProduct, ExtractionError};

const FULL_PAGE: &str = r#"<html><body>
<main>
<div id="js-item"></div>
<div id="items" data-product-id="99990001"
     data-shop-tracking-product-category="3D Characters"
     data-product-category="123"
     data-product-price="1,500">
    <article>
    <div class="summary">
      <h2>Synthetic Sample Avatar</h2>
      <span class="bg-primary700 typography-12">Adult</span>
      <div id="js-item-wishlist-button"><span class="typography-14">128</span></div>
    </div>
    <div class="shop-info">
      <div class="shop-name">Synthetic Shop</div>
    </div>
    <div class="primary-image-area">
      <img class="market-item-detail-item-image" data-origin="https://img.synthetic.example/origin-1.png" src="https://img.synthetic.example/thumb-1.png">
      <img class="market-item-detail-item-image" src="https://img.synthetic.example/fallback-2.png" data-lazy="https://img.synthetic.example/lazy-2.png">
    </div>
    <div class="main-info-column">
      <script type="application/ld+json">{"@context":"https://schema.org","@type":"Product","name":"Synthetic Sample Avatar LD","brand":{"@type":"Brand","name":"Synthetic Shop","url":"https://shop.synthetic.example"},"offers":{"@type":"Offer","price":"1500","priceCurrency":"JPY","availability":"https://schema.org/InStock"}}</script>
      <div class="js-market-item-detail-description description">
        <p>Synthetic description body with a <a href="https://shop.synthetic.example/trial">trial page</a> link.</p>
        <img src="https://img.synthetic.example/body-unconfirmed.png">
      </div>
      <section class="shop__text"><p>Additional shop section note</p></section>
      <div id="variations">
        <ul>
          <li class="variation-item">
            <span class="variation-name">Variant A</span>
            <span class="variation-price">1,500 JPY</span>
            <input type="hidden" name="cart_item[variation_id]" value="555001">
            <input type="hidden" name="cart_item[variation_id]" value="555001">
          </li>
          <li class="variation-item sold-out">
            <span class="variation-name">Variant B</span>
            <span class="variation-price">2,000 JPY</span>
            <div data-product-variant="555002"></div>
          </li>
        </ul>
      </div>
      <video><source src="https://media.synthetic.example/clip.mp4" type="video/mp4"></video>
      <iframe src="https://www.youtube.com/embed/synthetic"></iframe>
      <iframe src="https://ads.synthetic.example/not-a-video"></iframe>
    </div>
  </article>
  <aside>
    <h2>Aside decoy title</h2>
    <p class="shop-name">Aside decoy shop</p>
    <ul>
      <li class="item-card" data-product-id="99990002">
        <img class="item-card__thumbnail" src="https://img.synthetic.example/other-product.png">
      </li>
    </ul>
  </aside>
</div>
</main>
</body></html>"#;

#[test]
fn empty_category_attribute_falls_back_to_public_breadcrumbs() {
    let page = FULL_PAGE.replace("data-shop-tracking-product-category=\"3D Characters\"", "data-shop-tracking-product-category=\" \"")
        .replace("<main>", "<main><ul id='js-item-category-breadcrumbs'><li>3D Models</li><li>3D Clothing</li></ul>");
    let result = extract_product_page(&page).unwrap();
    assert_eq!(result.source_category.as_deref(), Some("3D Clothing"));
}

#[test]
fn bdl_ext_001_full_page_extracts_every_spec_field() {
    let extracted: ExtractedProduct =
        extract_product_page(FULL_PAGE).expect("a well-formed page extracts");

    // Identity and category come from root attributes.
    assert_eq!(extracted.native_product_id, "99990001");
    assert_eq!(extracted.source_category.as_deref(), Some("3D Characters"));

    // Title: primary source is the h2; the aside decoy and the ld+json
    // variant are not chosen while the primary exists.
    assert_eq!(extracted.title_source, Some("h2"));
    assert_eq!(extracted.title.as_deref(), Some("Synthetic Sample Avatar"));
    assert_ne!(extracted.title.as_deref(), Some("Aside decoy title"));

    // The aside is not part of the product.
    assert_eq!(extracted.shop_name.as_deref(), Some("Synthetic Shop"));
    assert_eq!(extracted.shop_url.as_deref(), Some("https://shop.synthetic.example"));

    // Adult only via the explicit badge.
    assert!(extracted.adult);

    assert_eq!(extracted.likes.as_deref(), Some("128"));

    // Availability loses its schema.org namespace; price prefers the root
    // attribute, currency comes from the JSON-LD offer.
    assert_eq!(extracted.availability.as_deref(), Some("InStock"));
    assert_eq!(extracted.price_amount.as_deref(), Some("1,500"));
    assert_eq!(extracted.price_currency.as_deref(), Some("JPY"));

    // Images: gallery originals first, then attribute fallbacks; body and
    // aside images never leak in.
    assert_eq!(
        extracted.image_urls,
        vec![
            "https://img.synthetic.example/origin-1.png",
            "https://img.synthetic.example/fallback-2.png",
        ]
    );

    // Videos: explicit sources and platform embeds only.
    assert_eq!(
        extracted.video_urls,
        vec!["https://media.synthetic.example/clip.mp4", "https://www.youtube.com/embed/synthetic"]
    );

    // Subproducts: deduped by variation id, sold-out recorded not deleted.
    assert_eq!(extracted.subproducts.len(), 2);
    let a = &extracted.subproducts[0];
    assert_eq!(a.variation_id.as_deref(), Some("555001"));
    assert_eq!(a.name.as_deref(), Some("Variant A"));
    assert_eq!(a.price_amount.as_deref(), Some("1,500"));
    assert_eq!(a.price_currency.as_deref(), Some("JPY"));
    assert_eq!(a.availability, "available");
    let b = &extracted.subproducts[1];
    assert_eq!(b.variation_id.as_deref(), Some("555002"));
    assert_eq!(b.availability, "sold_out");

    // Body: description plus shop section, appended in DOM order; the
    // unconfirmed body image is excluded.
    let body = extracted.body_text.expect("body present");
    assert!(body.contains("Synthetic description body"));
    assert!(body.contains("Additional shop section note"));
    assert!(!extracted.image_urls.contains(&"https://img.synthetic.example/body-unconfirmed.png".to_string()));

    // Search projection: body, title, author, subproduct names, category —
    // normalized.
    let projection = extracted.search_text_normalized.as_deref().expect("projection");
    assert!(projection.contains("synthetic description body"));
    assert!(projection.contains("synthetic sample avatar"));
    assert!(projection.contains("variant a"));
    assert!(projection.contains("3d characters"));
}

#[test]
fn bdl_ext_002_title_and_category_fallbacks_apply_without_inference() {
    let fallback_page = r#"<html><body>
<div id="items" data-product-id="99990003">
  <ul id="js-item-category-breadcrumbs">
    <li>Top</li><li>3D Characters</li>
  </ul>
  <article>
    <div class="summary"><p class="shop-name">Fallback Shop</p></div>
    <div class="main-info-column">
      <script type="application/ld+json">{"@type":"Product","name":"Fallback Title From LD"}</script>
    </div>
  </article>
</div>
</body></html>"#;
    let extracted = extract_product_page(fallback_page).expect("page extracts");
    assert_eq!(extracted.title_source, Some("ld_json"));
    assert_eq!(extracted.title.as_deref(), Some("Fallback Title From LD"));
    assert_eq!(extracted.source_category.as_deref(), Some("3D Characters"),
        "breadcrumbs are the category fallback; the generic Top level is not chosen blindly");
    // Everything else the fixture omits lands in missing_fields.
    for expected in [
        "likes",
        "published_date",
        "single_price",
        "price_currency",
        "availability",
        "images",
        "videos",
        "subproducts",
        "body",
    ] {
        assert!(
            extracted.missing_fields.contains(&expected),
            "missing_fields must list {expected}: {:?}",
            extracted.missing_fields
        );
    }
}

#[test]
fn bdl_ext_003_sparse_page_reports_honest_gaps_without_severity() {
    let sparse = r#"<html><body><div id="items" data-product-id="99990004"></div></body></html>"#;
    let extracted = extract_product_page(sparse).expect("identity present");
    assert_eq!(extracted.native_product_id, "99990004");
    assert!(!extracted.adult, "absence of a badge is never an Adult guess");
    assert!(extracted.missing_fields.contains(&"title"));
    assert!(extracted.missing_fields.contains(&"category"));
    assert!(extracted.missing_fields.contains(&"body"));
}

#[test]
fn bdl_ext_004_structural_failures_are_typed_errors() {
    assert_eq!(
        extract_product_page("<html><body><p>not a product page</p></body></html>"),
        Err(ExtractionError::ProductRootMissing)
    );
    assert_eq!(
        extract_product_page(r#"<html><body><div id="items"><article></article></div></body></html>"#),
        Err(ExtractionError::ProductIdMissing)
    );
}

// --- golden anchors: real corpus archives, local evidence only ---

/// Golden acceptance for the recognition rewrite: runs the extractor on the
/// two real product archives recorded in the legacy corpus. Requires the
/// corpus root through VUA_BDL_CORPUS; never runs in CI.
///
///   VUA_BDL_CORPUS="C:\...\_local_bdb_crawl" cargo test -p vua-orchestrator \
///     --test booth_extraction manual_golden -- --ignored --nocapture
#[test]
#[ignore = "manual: needs the local legacy corpus through VUA_BDL_CORPUS"]
fn manual_golden_anchor_extraction() {
    let corpus = PathBuf::from(
        std::env::var("VUA_BDL_CORPUS").expect("VUA_BDL_CORPUS must point at the crawl workspace"),
    );
    for id in ["4431242", "5986971"] {
        let record: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(corpus.join("data/products").join(format!("{id}.json")))
                .expect("anchor record"),
        )
        .unwrap();
        let storage = record["html"]["storage_path"].as_str().unwrap();
        // storage_path is content-addressed and relative to the corpus
        // `data/` directory (e.g. `html/sha256/1a/…html`).
        let html_path = if Path::new(storage).is_absolute() {
            PathBuf::from(storage)
        } else {
            corpus.join("data").join(storage)
        };
        let html = std::fs::read_to_string(&html_path).expect("archived HTML");
        let extracted = extract_product_page(&html).expect("golden anchors must extract");

        assert_eq!(extracted.native_product_id, id, "identity from the archive");
        assert!(
            extracted
                .title
                .as_deref()
                .is_some_and(|title| !title.trim().is_empty()),
            "{id}: a title must resolve through h2 or the JSON-LD fallback"
        );
        println!(
            "[{id}] title_source={:?} images={} videos={} subproducts={} missing={:?}",
            extracted.title_source,
            extracted.image_urls.len(),
            extracted.video_urls.len(),
            extracted.subproducts.len(),
            extracted.missing_fields
        );
    }
}
