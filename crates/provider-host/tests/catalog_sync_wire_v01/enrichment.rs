use super::*;
use vua_bdl_store::{ProductObservation, ProductObservationStatus};

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../schemas/catalog-enrichment/v0.1/cases.json"
    ))
    .unwrap()
}
fn request(name: &str) -> Value {
    vectors()["requests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .unwrap()["request"]
        .clone()
}

#[test]
fn enrichment_schemas_and_shared_vectors_agree() {
    let requests: Value = serde_json::from_str(include_str!(
        "../../../../schemas/catalog-enrichment/v0.1/request.schema.json"
    ))
    .unwrap();
    let responses: Value = serde_json::from_str(include_str!(
        "../../../../schemas/catalog-enrichment/v0.1/response.schema.json"
    ))
    .unwrap();
    let requests = jsonschema::validator_for(&requests).unwrap();
    let responses = jsonschema::validator_for(&responses).unwrap();
    for case in vectors()["requests"].as_array().unwrap() {
        assert_eq!(
            requests.is_valid(&case["request"]),
            case.get("schemaValid")
                .unwrap_or(&case["valid"])
                .as_bool()
                .unwrap(),
            "{}",
            case["name"]
        );
    }
    for case in vectors()["responses"].as_array().unwrap() {
        assert_eq!(
            responses.is_valid(&case["value"]),
            case["valid"].as_bool().unwrap(),
            "{}",
            case["name"]
        );
    }
}

#[test]
fn application_vectors_reach_the_real_parser_and_reject_bad_identity_without_writes() {
    for case in vectors()["requests"].as_array().unwrap() {
        let world = make_world("enrichment-vector");
        let frames = run_frames(&world, &[case["request"].clone()]);
        let payload = &frames[0]["payload"];
        let succeeds = case["valid"] == true && case.get("expectedError").is_none();
        assert_eq!(payload["ok"], succeeds, "{}: {payload}", case["name"]);
        if !succeeds {
            assert_eq!(
                payload["error"]["code"],
                case.get("expectedError")
                    .cloned()
                    .unwrap_or(json!("vua.catalog.invalid_params")),
                "{}",
                case["name"]
            );
            assert!(world
                .bdl
                .catalog_detail("booth:99990001")
                .unwrap()
                .is_none());
        }
        if case["name"] == "product-json" {
            let detail = world
                .bdl
                .catalog_detail("booth:99990001")
                .unwrap()
                .unwrap()
                .product;
            assert_eq!(detail.description.as_deref(), Some("First line\n\nhttps://synthetic.booth.pm/items/99990002\nhttps://vn3.org/synthetic-terms"));
            assert_eq!(
                world.bdl.library_product_dates().unwrap()["booth:99990001"]
                    .published_at
                    .as_deref(),
                Some("2026-11-01T10:30:00+09:00")
            );
            assert_eq!(
                world.bdl.library_description_links().unwrap()["booth:99990001"],
                vec!["booth:99990002"]
            );
            assert!(world
                .bdl
                .product_library_memberships("booth:99990001")
                .unwrap()
                .is_empty());
        }
    }
}

#[test]
fn existing_membership_is_required_and_attempt_receipts_survive_provider_restart() {
    let world = make_world("enrichment-owned");
    let queue = request("queue");
    let begin = request("begin");
    let frames = run_frames(&world, &[queue.clone(), begin.clone()]);
    assert_eq!(frames[0]["payload"]["value"]["productIds"], json!([]));
    assert_eq!(frames[1]["payload"]["value"]["claimed"], false);
    world
        .bdl
        .record_product_observation(&ProductObservation {
            product_id: "booth:99990001".into(),
            native_product_id: "99990001".into(),
            library_type: Some("bought".into()),
            variant_name: None,
            source_url: "https://booth.pm/en/items/99990001".into(),
            final_url: None,
            status: ProductObservationStatus::Complete,
            source_locale: None,
            source_category: None,
            title: Some("Synthetic".into()),
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
            observed_at: "2026-11-10T00:00:00Z".into(),
            run_id: None,
            processor_version: "synthetic-enrichment".into(),
            missing_fields: vec!["adult_badge".into()],
        })
        .unwrap();
    let frames = run_frames(
        &world,
        &[
            queue.clone(),
            begin.clone(),
            begin.clone(),
            queue.clone(),
            request("product-json"),
        ],
    );
    assert_eq!(
        frames[0]["payload"]["value"]["productIds"],
        json!(["booth:99990001"])
    );
    assert_eq!(frames[1]["payload"]["value"]["claimed"], true);
    assert_eq!(frames[2]["payload"]["value"]["claimed"], false);
    assert_eq!(frames[3]["payload"]["value"]["productIds"], json!([]));
    assert_eq!(frames[4]["payload"]["ok"], true);
    let restarted = run_frames(&world, &[begin, queue]);
    assert_eq!(restarted[0]["payload"]["value"]["claimed"], false);
    assert_eq!(restarted[1]["payload"]["value"]["productIds"], json!([]));
    assert_eq!(
        world
            .bdl
            .product_library_memberships("booth:99990001")
            .unwrap(),
        vec!["bought"]
    );
}
