use serde_json::json;
use vua_orchestrator::{extract_product_json, extract_product_page, ExtractionError};

fn product() -> serde_json::Value {
    json!({
        "id": 99990001, "name": "Synthetic Avatar", "description": " First line\r\n second line\n\n\nhttps://synthetic.booth.pm/items/99990002\n ",
        "published_at": "2026-11-01T10:30:00+09:00", "price": "¥ 1,500",
        "is_adult": true, "is_end_of_sale": false, "is_sold_out": false,
        "category": {"id": 7, "name": "3D Clothing", "parent": {"name":"3D"}},
        "shop": {"name":"Synthetic Shop", "url":"https://synthetic.booth.pm/", "verified":true},
        "images":[{"original":"https://synthetic.example/image.png"}],
        "variations":[{"id":99991001,"name":"Full set","price":1500,"status":"addable_to_cart","is_empty_stock":false}],
        "unknown_future_field": {"ignored": true}
    })
}

#[test]
fn public_json_preserves_observed_facts_and_multiline_description() {
    let extracted = extract_product_json(&product().to_string()).unwrap();
    assert_eq!(extracted.native_product_id, "99990001");
    assert_eq!(extracted.title_source, Some("json"));
    assert_eq!(extracted.source_category.as_deref(), Some("3D Clothing"));
    assert_eq!(extracted.shop_name.as_deref(), Some("Synthetic Shop"));
    assert_eq!(
        extracted.published_date_raw.as_deref(),
        Some("2026-11-01T10:30:00+09:00")
    );
    assert_eq!(
        extracted.body_text.as_deref(),
        Some("First line\nsecond line\n\nhttps://synthetic.booth.pm/items/99990002")
    );
    assert_eq!(extracted.search_text_normalized.as_deref().unwrap(), "first line second line https://synthetic.booth.pm/items/99990002 synthetic avatar synthetic shop 3d clothing full set");
    assert!(extracted.adult);
    assert_eq!(extracted.availability.as_deref(), Some("InStock"));
    assert_eq!(extracted.price_amount.as_deref(), Some("1,500"));
    assert_eq!(extracted.price_currency.as_deref(), Some("JPY"));
    assert_eq!(
        extracted.subproducts[0].price_amount.as_deref(),
        Some("1500")
    );
    assert_eq!(
        extracted.subproducts[0].availability.as_deref(),
        Some("available")
    );
}

#[test]
fn sparse_or_changed_json_reports_gaps_without_guessing() {
    let extracted = extract_product_json(r#"{"id":"99990001","published_at":"2026-02-30T10:00:00+09:00","is_adult":"false","category":null,"price":500,"variations":[{"id":2,"name":"Mystery","status":"future_value","price":500}]}"#).unwrap();
    assert_eq!(extracted.published_date_raw, None);
    assert_eq!(extracted.price_amount, None);
    assert_eq!(extracted.price_currency, None);
    assert_eq!(extracted.subproducts[0].availability, None);
    assert_eq!(extracted.subproducts[0].price_amount, None);
    for name in [
        "title",
        "category",
        "shop_name",
        "shop_url",
        "body",
        "published_date",
        "adult_badge",
        "subproduct_availability",
        "subproduct_price",
    ] {
        assert!(extracted.missing_fields.contains(&name), "{name}");
    }
}

#[test]
fn structural_json_failures_are_distinct_from_missing_fields() {
    for input in [
        "not json",
        "[]",
        "{}",
        r#"{"id":0}"#,
        r#"{"id":true}"#,
        r#"{"id":"1/2"}"#,
        r#"{"id":-1}"#,
    ] {
        assert_eq!(
            extract_product_json(input),
            Err(ExtractionError::InvalidProductJson)
        );
    }
    assert!(extract_product_json(&" ".repeat(5_000_001)).is_err());
}

#[test]
fn prices_are_pairs_and_sold_out_variants_remain_present() {
    let mut data = product();
    data["price"] = json!("¥ 0 ~ ¥ 1,500");
    data["is_end_of_sale"] = json!(true);
    data["variations"][0]["is_empty_stock"] = json!(true);
    let duplicate = data["variations"][0].clone();
    data["variations"].as_array_mut().unwrap().push(duplicate);
    let extracted = extract_product_json(&data.to_string()).unwrap();
    assert_eq!(extracted.price_amount, None);
    assert_eq!(extracted.price_currency, None);
    assert_eq!(extracted.availability.as_deref(), Some("Discontinued"));
    assert_eq!(extracted.subproducts.len(), 1);
    assert_eq!(
        extracted.subproducts[0].availability.as_deref(),
        Some("sold_out")
    );
}

#[test]
fn html_description_keeps_breaks_without_splitting_inline_links() {
    let extracted = extract_product_page(r#"<div id="items" data-product-id="99990001"><article>
      <div class="summary"><h2> Title <span>with</span> spaces </h2></div>
      <div class="js-market-item-detail-description description"> First<br>second
third<br><br><br>fourth<p>Paragraph <b>with</b> inline words.</p><p>https://synthetic.booth.pm/items/<span>99990002</span></p><script>ignored()</script></div>
      <section class="shop__text"> Shop<br>note </section><section class="shop__text"> Last </section>
      </article></div>"#).unwrap();
    assert_eq!(extracted.body_text.as_deref(), Some("First\nsecond\nthird\n\nfourth\n\nParagraph with inline words.\n\nhttps://synthetic.booth.pm/items/99990002\n\nShop\nnote\n\nLast"));
    assert_eq!(extracted.title.as_deref(), Some("Title with spaces"));
    assert!(!extracted.search_text_normalized.unwrap().contains('\n'));
}
